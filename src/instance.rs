//! Single-instance handling.
//!
//! The first launch listens on a loopback port. A second launch (for example a
//! magnet link clicked in the browser) connects to it, hands over its arguments
//! and exits, so the torrent opens in the window that is already running.

use std::hash::{Hash, Hasher};
use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

const HELLO: &str = "riptide-instance-v1";

pub enum Instance {
    /// We are the only instance; keep the listener alive.
    Primary(TcpListener),
    /// Another instance received our arguments; this process should exit.
    Forwarded,
    /// The port is taken by something else. Run without single-instance support.
    Standalone,
}

/// The port is derived from the config location, so separate portable installs
/// (`RIPTIDE_HOME`) don't hand torrents to each other.
fn port_for(config: &Path) -> u16 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    config.hash(&mut h);
    40_000 + (h.finish() % 20_000) as u16
}

pub fn acquire(config: &Path, args: &[String]) -> Instance {
    let port = port_for(config);
    match TcpListener::bind((Ipv4Addr::LOCALHOST, port)) {
        Ok(listener) => Instance::Primary(listener),
        Err(_) => match forward(port, args) {
            Ok(()) => Instance::Forwarded,
            Err(e) => {
                tracing::warn!("single-instance port {port} unavailable: {e}");
                Instance::Standalone
            }
        },
    }
}

fn forward(port: u16, args: &[String]) -> std::io::Result<()> {
    let mut stream =
        TcpStream::connect_timeout(&(Ipv4Addr::LOCALHOST, port).into(), Duration::from_secs(2))?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    writeln!(stream, "{HELLO}")?;
    let mut reply = String::new();
    BufReader::new(stream.try_clone()?).read_line(&mut reply)?;
    if reply.trim() != HELLO {
        return Err(std::io::Error::other("port is used by another program"));
    }
    for arg in args {
        writeln!(stream, "{}", arg.replace('\n', " "))?;
    }
    stream.flush()
}

/// Accepts hand-overs from later launches on a background thread. Each message
/// is the argument list of one launch (possibly empty, meaning "show yourself").
pub fn listen(
    listener: TcpListener,
    notify: impl Fn() + Send + 'static,
) -> mpsc::Receiver<Vec<String>> {
    let (tx, rx) = mpsc::channel();
    std::thread::Builder::new()
        .name("riptide-instance".into())
        .spawn(move || {
            for stream in listener.incoming().flatten() {
                if let Some(args) = receive(stream) {
                    if tx.send(args).is_err() {
                        break;
                    }
                    notify();
                }
            }
        })
        .ok();
    rx
}

fn receive(mut stream: TcpStream) -> Option<Vec<String>> {
    stream.set_read_timeout(Some(Duration::from_secs(2))).ok()?;
    let mut reader = BufReader::new(stream.try_clone().ok()?);
    let mut hello = String::new();
    reader.read_line(&mut hello).ok()?;
    if hello.trim() != HELLO {
        return None;
    }
    writeln!(stream, "{HELLO}").ok()?;
    Some(
        reader
            .lines()
            .map_while(Result::ok)
            .filter(|l| !l.trim().is_empty())
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_launch_hands_over_arguments() {
        let config = std::env::temp_dir().join(format!("riptide-instance-{}", std::process::id()));
        let Instance::Primary(listener) = acquire(&config, &[]) else {
            panic!("first launch should own the port");
        };
        let rx = listen(listener, || {});

        let args = vec![
            "magnet:?xt=urn:btih:abc".to_owned(),
            "C:\\a b.torrent".to_owned(),
        ];
        assert!(matches!(acquire(&config, &args), Instance::Forwarded));
        assert_eq!(rx.recv_timeout(Duration::from_secs(5)).unwrap(), args);
    }
}
