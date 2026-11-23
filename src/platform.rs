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

/// Registers Riptide as an *available* handler without changing defaults.
/// Called on startup so the app appears in Windows' "Open with" list.
pub fn register_capabilities() {
    #[cfg(target_os = "windows")]
    if let Ok(exe) = std::env::current_exe()
        && let Err(e) = imp::register_capabilities(&exe)
    {
        tracing::warn!("could not register file associations: {e}");
    }
}

#[cfg(target_os = "windows")]
fn open_uri(uri: &str) -> std::io::Result<()> {
    Command::new("explorer").arg(uri).spawn().map(drop)
}

/// Whether magnet links currently open this executable.
pub fn is_registered() -> bool {
    std::env::current_exe().is_ok_and(|exe| imp::is_registered(&exe))
}

#[cfg(target_os = "windows")]
mod imp {
    //! Windows keeps associations in three places:
    //! - ProgIDs under `HKCU\Software\Classes` describe how to open something;
    //! - `OpenWithProgids` / `RegisteredApplications` make the app *eligible*
    //!   (it shows up in "Open with" and in Settings > Default apps);
    //! - `UserChoice` records what the user actually picked. Apps cannot write
    //!   it, so "make default" ends by opening the Default apps page.

    use std::path::Path;

    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    const TORRENT_PROGID: &str = "Riptide.Torrent";
    const MAGNET_PROGID: &str = "Riptide.Magnet";
    const CAPABILITIES: &str = r"Software\Riptide\Capabilities";

    fn set(root: &RegKey, path: &str, name: &str, value: &str) -> std::io::Result<()> {
        let (key, _) = root.create_subkey(path)?;
        key.set_value(name, &value)
    }

    fn open_command(exe: &Path) -> String {
        format!("\"{}\" \"%1\"", exe.display())
    }

    /// Makes Riptide eligible for .torrent files and magnet links without
    /// taking over anything the user already chose. Safe to run on every launch;
    /// it also keeps the executable path current if the app moved.
    pub fn register_capabilities(exe: &Path) -> std::io::Result<()> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (classes, _) = hkcu.create_subkey(r"Software\Classes")?;
        let open = open_command(exe);
        let icon = format!("\"{}\",0", exe.display());

        for (progid, label) in [
            (TORRENT_PROGID, "Torrent file"),
            (MAGNET_PROGID, "Magnet link"),
        ] {
            set(&classes, progid, "", label)?;
            set(&classes, &format!(r"{progid}\DefaultIcon"), "", &icon)?;
            set(
                &classes,
                &format!(r"{progid}\shell\open\command"),
                "",
                &open,
            )?;
        }
        set(&classes, MAGNET_PROGID, "URL Protocol", "")?;
        set(&classes, r".torrent\OpenWithProgids", TORRENT_PROGID, "")?;

        let app = r"Applications\riptide.exe";
        set(&classes, app, "FriendlyAppName", "Riptide")?;
        set(&classes, &format!(r"{app}\SupportedTypes"), ".torrent", "")?;
        set(&classes, &format!(r"{app}\shell\open\command"), "", &open)?;

        set(&hkcu, CAPABILITIES, "ApplicationName", "Riptide")?;
        set(&hkcu, CAPABILITIES, "ApplicationIcon", &icon)?;
        set(
            &hkcu,
            CAPABILITIES,
            "ApplicationDescription",
            "A fast, quiet BitTorrent client",
        )?;
        set(
            &hkcu,
            &format!(r"{CAPABILITIES}\FileAssociations"),
            ".torrent",
            TORRENT_PROGID,
        )?;
        set(
            &hkcu,
            &format!(r"{CAPABILITIES}\URLAssociations"),
            "magnet",
            MAGNET_PROGID,
        )?;
        set(
            &hkcu,
            r"Software\RegisteredApplications",
            "Riptide",
            CAPABILITIES,
        )?;
        notify_shell();
        Ok(())
    }

    pub fn register(exe: &Path) -> anyhow::Result<()> {
        register_capabilities(exe)?;
        // Fallbacks used when the user never picked anything (no UserChoice yet).
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (classes, _) = hkcu.create_subkey(r"Software\Classes")?;
        set(&classes, "magnet", "", "URL:Magnet link")?;
        set(&classes, "magnet", "URL Protocol", "")?;
        set(
            &classes,
            r"magnet\shell\open\command",
            "",
            &open_command(exe),
        )?;
        set(&classes, ".torrent", "", TORRENT_PROGID)?;
        notify_shell();
        // Let the user confirm in Windows' own Default apps page, filtered to Riptide.
        super::open_uri("ms-settings:defaultapps?registeredAppUser=Riptide")?;
        Ok(())
    }

    pub fn is_registered(exe: &Path) -> bool {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let choice =
            r"Software\Microsoft\Windows\Shell\Associations\UrlAssociations\magnet\UserChoice";
        if let Ok(key) = hkcu.open_subkey(choice) {
            return key
                .get_value::<String, _>("ProgId")
                .is_ok_and(|p| p == MAGNET_PROGID);
        }
        hkcu.open_subkey(r"Software\Classes\magnet\shell\open\command")
            .and_then(|k| k.get_value::<String, _>(""))
            .is_ok_and(|cmd| {
                cmd.to_lowercase()
                    .contains(&exe.display().to_string().to_lowercase())
            })
    }

    /// Tells Explorer that associations changed so "Open with" refreshes.
    fn notify_shell() {
        use windows_sys::Win32::UI::Shell::{SHCNE_ASSOCCHANGED, SHCNF_IDLIST, SHChangeNotify};
        // SAFETY: SHCNE_ASSOCCHANGED takes no item pointers.
        unsafe {
            SHChangeNotify(
                SHCNE_ASSOCCHANGED as i32,
                SHCNF_IDLIST,
                std::ptr::null(),
                std::ptr::null(),
            )
        };
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
