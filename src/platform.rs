//! Small OS integrations.

use std::path::Path;
use std::process::Command;

/// Opens a folder in the system file manager.
pub fn open_path(path: &Path) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    let mut cmd = Command::new("explorer");
    #[cfg(target_os = "macos")]
    let mut cmd = Command::new("open");
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut cmd = Command::new("xdg-open");

    cmd.arg(path).spawn().map(|_| ())
}

/// Makes this executable the handler for `magnet:` links and `.torrent` files
/// for the current user. No administrator rights are needed.
pub fn register_handler() -> anyhow::Result<()> {
    let exe = std::env::current_exe()?;
    imp::register(&exe)
}

/// Whether magnet links currently open this executable.
pub fn is_registered() -> bool {
    std::env::current_exe().is_ok_and(|exe| imp::is_registered(&exe))
}

#[cfg(target_os = "windows")]
mod imp {
    use std::os::windows::process::CommandExt;
    use std::path::Path;
    use std::process::Command;

    use anyhow::{Context, bail};

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const CLASSES: &str = r"HKCU\Software\Classes";

    fn reg(args: &[&str]) -> anyhow::Result<String> {
        let out = Command::new("reg")
            .args(args)
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .context("could not run reg.exe")?;
        if !out.status.success() {
            bail!("{}", String::from_utf8_lossy(&out.stderr).trim());
        }
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    }

    fn set(key: &str, name: Option<&str>, value: &str) -> anyhow::Result<()> {
        let key = format!(r"{CLASSES}\{key}");
        let mut args = vec!["add", key.as_str()];
        match name {
            Some(n) => args.extend(["/v", n]),
            None => args.push("/ve"),
        }
        args.extend(["/d", value, "/f"]);
        reg(&args).map(drop)
    }

    pub fn register(exe: &Path) -> anyhow::Result<()> {
        let exe = exe.display();
        let open = format!("\"{exe}\" \"%1\"");
        let icon = format!("\"{exe}\",0");

        set("magnet", None, "URL:Magnet link")?;
        set("magnet", Some("URL Protocol"), "")?;
        set(r"magnet\DefaultIcon", None, &icon)?;
        set(r"magnet\shell\open\command", None, &open)?;

        set("Riptide.Torrent", None, "Torrent file")?;
        set(r"Riptide.Torrent\DefaultIcon", None, &icon)?;
        set(r"Riptide.Torrent\shell\open\command", None, &open)?;
        set(".torrent", None, "Riptide.Torrent")?;
        Ok(())
    }

    pub fn is_registered(exe: &Path) -> bool {
        let key = format!(r"{CLASSES}\magnet\shell\open\command");
        reg(&["query", &key, "/ve"]).is_ok_and(|out| {
            out.to_lowercase()
                .contains(&exe.display().to_string().to_lowercase())
        })
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
mod imp {
    use std::path::{Path, PathBuf};
    use std::process::Command;

    use anyhow::Context;

    const DESKTOP: &str = "riptide.desktop";
    const MIME: [&str; 2] = ["x-scheme-handler/magnet", "application/x-bittorrent"];

    fn applications_dir() -> anyhow::Result<PathBuf> {
        let dirs = directories::BaseDirs::new().context("no home directory")?;
        Ok(dirs.data_dir().join("applications"))
    }

    pub fn register(exe: &Path) -> anyhow::Result<()> {
        let dir = applications_dir()?;
        std::fs::create_dir_all(&dir)?;
        let entry = format!(
            "[Desktop Entry]\nType=Application\nName=Riptide\nComment=BitTorrent client\n\
             Exec=\"{}\" %u\nTerminal=false\nCategories=Network;FileTransfer;P2P;\n\
             MimeType={};\n",
            exe.display(),
            MIME.join(";")
        );
        std::fs::write(dir.join(DESKTOP), entry)?;
        for mime in MIME {
            Command::new("xdg-mime")
                .args(["default", DESKTOP, mime])
                .status()
                .context("could not run xdg-mime")?;
        }
        Ok(())
    }

    pub fn is_registered(_exe: &Path) -> bool {
        Command::new("xdg-mime")
            .args(["query", "default", MIME[0]])
            .output()
            .is_ok_and(|o| String::from_utf8_lossy(&o.stdout).trim() == DESKTOP)
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use std::path::Path;

    pub fn register(_exe: &Path) -> anyhow::Result<()> {
        anyhow::bail!("on macOS, choose Riptide as the default app from Finder's Get Info window")
    }

    pub fn is_registered(_exe: &Path) -> bool {
        false
    }
}
