//! Thin layer over librqbit that the UI talks to.
//!
//! The UI thread never blocks on the network: every potentially slow operation
//! (adding a torrent, resolving a magnet, deleting files) is spawned onto a tokio
//! runtime and reports back through [`Event`]s. Read-only views of the session are
//! cheap snapshots taken once per frame.

use std::collections::HashSet;
use std::net::{Ipv6Addr, SocketAddr};
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use anyhow::Context;
use librqbit::api::TorrentIdOrHash;
use librqbit::{
    AddTorrent, AddTorrentOptions, AddTorrentResponse, ListenerMode, ListenerOptions, Magnet,
    ManagedTorrent, Session, SessionOptions, SessionPersistenceConfig, TorrentStatsState,
};

use crate::settings::Settings;

pub type TorrentId = usize;
type Notify = Arc<dyn Fn() + Send + Sync>;

const MIB: f64 = 1024.0 * 1024.0;

/// What a torrent is currently doing, from the user's point of view.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Status {
    /// Verifying existing data on disk.
    Checking,
    Downloading,
    /// Finished and uploading to others.
    Seeding,
    /// Stopped by the user before finishing.
    Paused,
    /// Finished and stopped.
    Done,
    Error,
}

impl Status {
    pub fn is_active(self) -> bool {
        matches!(self, Self::Checking | Self::Downloading | Self::Seeding)
    }
}

/// A snapshot of one torrent, cheap to clone and render.
#[derive(Clone, Debug)]
pub struct TorrentView {
    pub id: TorrentId,
    pub info_hash: String,
    pub name: String,
    pub status: Status,
    pub error: Option<String>,
    /// 0.0..=1.0
    pub progress: f32,
    pub total_bytes: u64,
    pub done_bytes: u64,
    pub uploaded_bytes: u64,
    pub down_speed: u64,
    pub up_speed: u64,
    pub eta_secs: Option<u64>,
    pub peers: u32,
    pub seeds: u32,
    pub output_folder: PathBuf,
}

impl TorrentView {
    pub fn magnet_link(&self) -> String {
        format!(
            "magnet:?xt=urn:btih:{}&dn={}",
            self.info_hash,
            crate::format::url_encode(&self.name)
        )
    }
}

/// A torrent that is still being resolved (e.g. a magnet waiting for metadata).
#[derive(Clone, Debug)]
pub struct PendingView {
    pub key: u64,
    pub name: String,
    pub since: Instant,
}

#[derive(Clone, Debug)]
pub struct FileView {
    pub index: usize,
    pub path: PathBuf,
    pub size: u64,
    pub done: u64,
    pub selected: bool,
}

#[derive(Clone, Debug)]
pub struct PeerView {
    pub addr: String,
    pub client: String,
    pub downloaded: u64,
    pub uploaded: u64,
    pub utp: bool,
}

/// Where a torrent comes from.
#[derive(Clone, Debug)]
pub enum Source {
    /// A magnet link or http(s) URL to a .torrent file.
    Url(String),
    /// Raw contents of a .torrent file.
    File { name: String, bytes: Vec<u8> },
}

impl Source {
    pub fn from_path(path: &Path) -> anyhow::Result<Self> {
        let bytes =
            std::fs::read(path).with_context(|| format!("could not read {}", path.display()))?;
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        Ok(Self::File { name, bytes })
    }

    /// A best-effort name to show before metadata is known.
    pub fn display_name(&self) -> String {
        match self {
            Self::Url(url) => Magnet::parse(url)
                .ok()
                .and_then(|m| m.name)
                .unwrap_or_else(|| url.chars().take(60).collect()),
            Self::File { name, .. } => name.clone(),
        }
    }

    fn trackers(&self) -> Vec<String> {
        match self {
            Self::Url(url) => Magnet::parse(url).map(|m| m.trackers).unwrap_or_default(),
            Self::File { .. } => Vec::new(),
        }
    }

    fn to_add(&self) -> AddTorrent<'static> {
        match self {
            Self::Url(url) => AddTorrent::from_url(url.clone()),
            Self::File { bytes, .. } => AddTorrent::from_bytes(bytes.clone()),
        }
    }
}

/// Torrent metadata resolved before adding, so the user can choose files.
#[derive(Clone)]
pub struct Preview {
    pub name: String,
    pub info_hash: String,
    pub files: Vec<PreviewFile>,
    /// Sub-folder librqbit would create for this torrent (empty for single-file torrents).
    sub_folder: PathBuf,
    torrent_bytes: bytes::Bytes,
    peers: Vec<SocketAddr>,
    trackers: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct PreviewFile {
    pub index: usize,
    pub path: PathBuf,
    pub size: u64,
}

pub struct AddRequest {
    pub source: Source,
    /// Resolved metadata, if the user waited for it.
    pub preview: Option<Arc<Preview>>,
    /// Folder to save into; `None` uses the default download folder.
    pub folder: Option<PathBuf>,
    /// File indices to download; `None` downloads everything.
    pub only_files: Option<Vec<usize>>,
    pub paused: bool,
}

/// Results of background work, drained by the UI every frame.
pub enum Event {
    Added {
        name: String,
    },
    AlreadyAdded {
        name: String,
    },
    Failed {
        context: String,
        error: String,
    },
    Preview {
        request: u64,
        result: Result<Arc<Preview>, String>,
    },
}

pub struct Engine {
    rt: tokio::runtime::Runtime,
    session: Arc<Session>,
    default_dir: PathBuf,
    pending: Arc<Mutex<Vec<PendingView>>>,
    next_key: AtomicU64,
    events_tx: mpsc::Sender<Event>,
    events_rx: mpsc::Receiver<Event>,
    notify: Notify,
}

impl Engine {
    /// Starts the BitTorrent session. Torrents from the previous run are restored
    /// from `session_dir` and continue where they left off.
    pub fn start(
        settings: &Settings,
        session_dir: PathBuf,
        notify: Notify,
    ) -> anyhow::Result<Self> {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("riptide-engine")
            .build()?;

        std::fs::create_dir_all(&settings.download_dir).ok();
        std::fs::create_dir_all(&session_dir)?;

        let opts = SessionOptions {
            fastresume: true,
            persistence: Some(SessionPersistenceConfig::Json {
                folder: Some(session_dir),
            }),
            listen: Some(ListenerOptions {
                mode: ListenerMode::TcpAndUtp,
                listen_addr: (Ipv6Addr::UNSPECIFIED, settings.listen_port).into(),
                enable_upnp_port_forwarding: settings.upnp,
                ..Default::default()
            }),
            ratelimits: librqbit::limits::LimitsConfig {
                download_bps: kib_to_bps(settings.download_limit_kib),
                upload_bps: kib_to_bps(settings.upload_limit_kib),
            },
            client_name_and_version: Some(format!("Riptide {}", env!("CARGO_PKG_VERSION"))),
            ..Default::default()
        };

        let download_dir = settings.download_dir.clone();
        let session = rt
            .block_on(Session::new_with_opts(download_dir.clone(), opts))
            .context("failed to start the torrent session")?;

        let (events_tx, events_rx) = mpsc::channel();
        Ok(Self {
            rt,
            session,
            default_dir: download_dir,
            pending: Default::default(),
            next_key: AtomicU64::new(1),
            events_tx,
            events_rx,
            notify,
        })
    }

    pub fn listen_port(&self) -> Option<u16> {
        self.session.announce_port()
    }

    /// Returns the next finished background result, if any.
    pub fn poll_event(&self) -> Option<Event> {
        self.events_rx.try_recv().ok()
    }

    pub fn torrents(&self) -> Vec<TorrentView> {
        let mut list = self
            .session
            .with_torrents(|iter| iter.map(|(id, t)| view_of(id, t)).collect::<Vec<_>>());
        list.sort_by_key(|t| t.id);
        list
    }

    pub fn pending(&self) -> Vec<PendingView> {
        self.pending.lock().unwrap().clone()
    }

    /// Total transfer rates across all torrents, in bytes per second.
    pub fn total_speed(&self) -> (u64, u64) {
        let s = self.session.stats_snapshot();
        (
            (s.download_speed.mbps * MIB) as u64,
            (s.upload_speed.mbps * MIB) as u64,
        )
    }

    pub fn files(&self, id: TorrentId) -> Vec<FileView> {
        let Some(handle) = self.session.get(TorrentIdOrHash::Id(id)) else {
            return Vec::new();
        };
        let stats = handle.stats();
        let only = handle
            .only_files()
            .map(|v| v.into_iter().collect::<HashSet<_>>());
        handle
            .with_metadata(|m| {
                m.file_infos
                    .iter()
                    .enumerate()
                    .filter(|(_, f)| !f.attrs.padding)
                    .map(|(index, f)| FileView {
                        index,
                        path: f.relative_filename.clone(),
                        size: f.len,
                        done: stats.file_progress.get(index).copied().unwrap_or(0),
                        selected: only.as_ref().is_none_or(|o| o.contains(&index)),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn peers(&self, id: TorrentId) -> Vec<PeerView> {
        let Some(live) = self
            .session
            .get(TorrentIdOrHash::Id(id))
            .and_then(|h| h.live())
        else {
            return Vec::new();
        };
        let mut peers: Vec<PeerView> = live
            .per_peer_stats_snapshot(Default::default())
            .peers
            .into_iter()
            .map(|(addr, p)| PeerView {
                addr,
                client: p.client_name.unwrap_or_default(),
                downloaded: p.counters.fetched_bytes,
                uploaded: p.counters.uploaded_bytes,
                utp: p.conn_kind.is_some_and(|k| format!("{k:?}") == "Utp"),
            })
            .collect();
        peers.sort_by_key(|p| std::cmp::Reverse(p.downloaded));
        peers
    }

    /// Resolves metadata without starting the download. The answer arrives as
    /// [`Event::Preview`] tagged with the returned request number.
    pub fn request_preview(&self, source: Source) -> u64 {
        let request = self.next_key.fetch_add(1, Ordering::Relaxed);
        let session = self.session.clone();
        let tx = self.events_tx.clone();
        let notify = self.notify.clone();
        let default_dir = self.default_dir.clone();
        self.rt.spawn(async move {
            let result = resolve(&session, &default_dir, &source)
                .await
                .map(Arc::new)
                .map_err(|e| format!("{e:#}"));
            let _ = tx.send(Event::Preview { request, result });
            notify();
        });
        request
    }

    pub fn add(&self, req: AddRequest) {
        let key = self.next_key.fetch_add(1, Ordering::Relaxed);
        let name = req
            .preview
            .as_ref()
            .map(|p| p.name.clone())
            .unwrap_or_else(|| req.source.display_name());
        self.pending.lock().unwrap().push(PendingView {
            key,
            name: name.clone(),
            since: Instant::now(),
        });

        let session = self.session.clone();
        let tx = self.events_tx.clone();
        let notify = self.notify.clone();
        let pending = self.pending.clone();
        let default_dir = self.default_dir.clone();
        self.rt.spawn(async move {
            let event = match add_torrent(&session, &default_dir, req).await {
                Ok((true, name)) => Event::Added { name },
                Ok((false, name)) => Event::AlreadyAdded { name },
                Err(e) => Event::Failed {
                    context: name,
                    error: format!("{e:#}"),
                },
            };
            pending.lock().unwrap().retain(|p| p.key != key);
            let _ = tx.send(event);
            notify();
        });
    }

    pub fn pause(&self, id: TorrentId) {
        self.with_handle(id, "pause", |session, handle| async move {
            if handle.is_paused() {
                return Ok(());
            }
            session.pause(&handle).await
        });
    }

    pub fn resume(&self, id: TorrentId) {
        self.with_handle(id, "resume", |session, handle| async move {
            if !handle.is_paused() && !handle.with_state(|s| s.name() == "error") {
                return Ok(());
            }
            session.unpause(&handle).await
        });
    }

    pub fn remove(&self, id: TorrentId, delete_files: bool) {
        let session = self.session.clone();
        self.spawn_action("remove", async move {
            session.delete(TorrentIdOrHash::Id(id), delete_files).await
        });
    }

    pub fn set_selected_files(&self, id: TorrentId, files: HashSet<usize>) {
        self.with_handle(id, "update files", |session, handle| async move {
            session.update_only_files(&handle, &files).await
        });
    }

    #[allow(dead_code)] // used by the settings dialog
    /// Applies global speed limits immediately. `0` means unlimited.
    pub fn set_limits(&self, download_kib: u32, upload_kib: u32) {
        let limits = &self.session.ratelimits;
        limits.set_download_bps(kib_to_bps(download_kib));
        limits.set_upload_bps(kib_to_bps(upload_kib));
    }

    /// Stops all torrents and flushes resume data. Torrents keep their
    /// running/paused state for the next launch.
    pub fn shutdown(self) {
        let session = self.session.clone();
        self.rt.block_on(async move {
            let _ = tokio::time::timeout(Duration::from_secs(3), session.stop()).await;
        });
        self.rt.shutdown_timeout(Duration::from_secs(1));
    }

    fn with_handle<F, Fut>(&self, id: TorrentId, what: &'static str, f: F)
    where
        F: FnOnce(Arc<Session>, Arc<ManagedTorrent>) -> Fut,
        Fut: Future<Output = anyhow::Result<()>> + Send + 'static,
    {
        let Some(handle) = self.session.get(TorrentIdOrHash::Id(id)) else {
            return;
        };
        self.spawn_action(what, f(self.session.clone(), handle));
    }

    fn spawn_action(
        &self,
        what: &'static str,
        fut: impl Future<Output = anyhow::Result<()>> + Send + 'static,
    ) {
        let tx = self.events_tx.clone();
        let notify = self.notify.clone();
        self.rt.spawn(async move {
            if let Err(e) = fut.await {
                let _ = tx.send(Event::Failed {
                    context: what.to_owned(),
                    error: format!("{e:#}"),
                });
            }
            notify();
        });
    }
}

fn kib_to_bps(kib: u32) -> Option<NonZeroU32> {
    NonZeroU32::new(kib.saturating_mul(1024))
}

async fn resolve(
    session: &Arc<Session>,
    default_dir: &Path,
    source: &Source,
) -> anyhow::Result<Preview> {
    let resp = session
        .add_torrent(
            source.to_add(),
            Some(AddTorrentOptions {
                list_only: true,
                ..Default::default()
            }),
        )
        .await?;
    let AddTorrentResponse::ListOnly(list) = resp else {
        anyhow::bail!("unexpected response while reading torrent metadata");
    };

    let name = list
        .info
        .name()
        .map(|n| n.into_owned())
        .unwrap_or_else(|| source.display_name());
    let mut files = Vec::new();
    for (index, f) in list.info.iter_file_details().enumerate() {
        if f.attrs().padding {
            continue;
        }
        files.push(PreviewFile {
            index,
            path: f.filename.to_pathbuf(),
            size: f.len,
        });
    }
    Ok(Preview {
        name,
        info_hash: list.info_hash.as_string(),
        files,
        sub_folder: list
            .output_folder
            .strip_prefix(default_dir)
            .map(Path::to_path_buf)
            .unwrap_or_default(),
        torrent_bytes: list.torrent_bytes,
        peers: list.seen_peers,
        trackers: source.trackers(),
    })
}

/// Returns `(newly_added, name)`.
async fn add_torrent(
    session: &Arc<Session>,
    default_dir: &Path,
    req: AddRequest,
) -> anyhow::Result<(bool, String)> {
    // Always resolve first: the output folder for multi-file torrents depends on
    // the torrent name, which a magnet link only reveals once metadata arrives.
    let preview = match req.preview {
        Some(p) => p,
        None => Arc::new(resolve(session, default_dir, &req.source).await?),
    };
    let folder = req.folder.unwrap_or_else(|| default_dir.to_path_buf());
    let output_folder = folder.join(&preview.sub_folder);

    let opts = AddTorrentOptions {
        paused: req.paused,
        only_files: req.only_files,
        // Reuse data that is already on disk instead of refusing to start.
        overwrite: true,
        output_folder: Some(output_folder.to_string_lossy().into_owned()),
        initial_peers: Some(preview.peers.clone()).filter(|p| !p.is_empty()),
        trackers: Some(preview.trackers.clone()).filter(|t| !t.is_empty()),
        ..Default::default()
    };
    let resp = session
        .add_torrent(
            AddTorrent::from_bytes(preview.torrent_bytes.clone()),
            Some(opts),
        )
        .await?;
    Ok(match resp {
        AddTorrentResponse::Added(..) => (true, preview.name.clone()),
        AddTorrentResponse::AlreadyManaged(..) => (false, preview.name.clone()),
        AddTorrentResponse::ListOnly(_) => anyhow::bail!("torrent was not added"),
    })
}

fn view_of(id: TorrentId, handle: &Arc<ManagedTorrent>) -> TorrentView {
    let stats = handle.stats();
    let live = stats.live.as_ref();

    let down_speed = live.map_or(0, |l| (l.download_speed.mbps * MIB) as u64);
    let up_speed = live.map_or(0, |l| (l.upload_speed.mbps * MIB) as u64);
    let (peers, seeds) = live.map_or((0, 0), |l| {
        let p = &l.snapshot.peer_stats;
        (p.live, p.live_seeders)
    });

    let status = match stats.state {
        TorrentStatsState::Initializing { .. } => Status::Checking,
        TorrentStatsState::Error => Status::Error,
        TorrentStatsState::Paused if stats.finished => Status::Done,
        TorrentStatsState::Paused => Status::Paused,
        TorrentStatsState::Live if stats.finished => Status::Seeding,
        TorrentStatsState::Live => Status::Downloading,
    };

    let remaining = stats.total_bytes.saturating_sub(stats.progress_bytes);
    let eta_secs =
        (status == Status::Downloading && down_speed > 0).then(|| remaining / down_speed.max(1));

    TorrentView {
        id,
        info_hash: handle.info_hash().as_string(),
        name: handle
            .name()
            .unwrap_or_else(|| "Unnamed torrent".to_owned()),
        status,
        error: stats.error.clone(),
        progress: if stats.total_bytes == 0 {
            0.0
        } else {
            (stats.progress_bytes as f64 / stats.total_bytes as f64) as f32
        },
        total_bytes: stats.total_bytes,
        done_bytes: stats.progress_bytes,
        uploaded_bytes: stats.uploaded_bytes,
        down_speed,
        up_speed,
        eta_secs,
        peers,
        seeds,
        output_folder: handle.output_folder().to_path_buf(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sintel (Blender Foundation, CC BY 3.0): the standard WebTorrent test torrent.
    const SINTEL: &str = "magnet:?xt=urn:btih:08ada5a7a6183aae1e09d831df6748d566095a10&dn=Sintel&tr=udp%3A%2F%2Fexplodie.org%3A6969&tr=udp%3A%2F%2Ftracker.opentrackr.org%3A1337&tr=wss%3A%2F%2Ftracker.webtorrent.dev&ws=https%3A%2F%2Fwebtorrent.io%2Ftorrents%2F";

    fn wait_for<T>(engine: &Engine, secs: u64, mut f: impl FnMut(&Engine) -> Option<T>) -> T {
        let deadline = Instant::now() + Duration::from_secs(secs);
        loop {
            if let Some(v) = f(engine) {
                return v;
            }
            assert!(Instant::now() < deadline, "timed out");
            std::thread::sleep(Duration::from_millis(250));
        }
    }

    /// Downloads one small file, restarts the engine, and checks the torrent
    /// comes back finished. Needs internet access.
    #[test]
    #[ignore = "needs network"]
    fn downloads_and_resumes_after_restart() {
        let root = std::env::temp_dir().join(format!("riptide-e2e-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let settings = Settings {
            download_dir: root.join("downloads"),
            listen_port: 0,
            upnp: false,
            ..Settings::default()
        };
        let session_dir = root.join("session");
        let start = || Engine::start(&settings, session_dir.clone(), Arc::new(|| {})).unwrap();

        let engine = start();
        let request = engine.request_preview(Source::Url(SINTEL.to_owned()));
        let preview = wait_for(&engine, 120, |e| match e.poll_event() {
            Some(Event::Preview { request: r, result }) if r == request => Some(result.unwrap()),
            _ => None,
        });
        assert_eq!(preview.name, "Sintel");
        let subtitle = preview
            .files
            .iter()
            .find(|f| f.path.to_string_lossy().ends_with(".srt"))
            .expect("subtitle file");
        println!(
            "downloading {} ({} bytes)",
            subtitle.path.display(),
            subtitle.size
        );

        engine.add(AddRequest {
            source: Source::Url(SINTEL.to_owned()),
            preview: Some(preview.clone()),
            folder: None,
            only_files: Some(vec![subtitle.index]),
            paused: false,
        });
        wait_for(&engine, 300, |e| {
            let t = e.torrents().into_iter().next()?;
            matches!(t.status, Status::Seeding | Status::Done).then_some(())
        });
        let path = settings.download_dir.join("Sintel").join(&subtitle.path);
        assert_eq!(std::fs::metadata(&path).unwrap().len(), subtitle.size);
        engine.shutdown();

        // Second launch: the torrent is restored from the session folder.
        let engine = start();
        let restored = wait_for(&engine, 60, |e| {
            let t = e.torrents().into_iter().next()?;
            (t.status != Status::Checking).then_some(t)
        });
        assert_eq!(restored.name, "Sintel");
        assert!(
            restored.progress >= 0.999,
            "progress was {}",
            restored.progress
        );
        engine.shutdown();
        let _ = std::fs::remove_dir_all(&root);
    }
}
