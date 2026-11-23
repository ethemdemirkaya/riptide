//! User preferences, persisted as JSON in the platform config directory.

use std::path::{Path, PathBuf};

use anyhow::Context;
use directories::{ProjectDirs, UserDirs};
use serde::{Deserialize, Serialize};

use crate::i18n::Lang;

/// Platform-specific folders used by the app.
pub struct Paths {
    pub config_file: PathBuf,
    /// Where librqbit keeps its session state (torrent list, fast-resume data, DHT).
    pub session_dir: PathBuf,
}

impl Paths {
    /// Uses `RIPTIDE_HOME` when set (portable installs, testing), otherwise the
    /// platform's standard config and data folders.
    pub fn resolve() -> anyhow::Result<Self> {
        if let Some(home) = std::env::var_os("RIPTIDE_HOME") {
            let home = PathBuf::from(home);
            return Ok(Self {
                config_file: home.join("settings.json"),
                session_dir: home.join("session"),
            });
        }
        let dirs = ProjectDirs::from("dev", "riptide", "Riptide")
            .context("could not determine a home directory")?;
        Ok(Self {
            config_file: dirs.config_dir().join("settings.json"),
            session_dir: dirs.data_dir().join("session"),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Default folder new torrents are saved into.
    pub download_dir: PathBuf,
    /// TCP/uTP port for incoming peer connections.
    pub listen_port: u16,
    /// Ask the router to forward `listen_port` via UPnP.
    pub upnp: bool,
    /// Global download limit in KiB/s, 0 for unlimited.
    pub download_limit_kib: u32,
    /// Global upload limit in KiB/s, 0 for unlimited.
    pub upload_limit_kib: u32,
    /// Start downloading as soon as a torrent is added.
    pub start_on_add: bool,
    /// Maximum connected peers per torrent.
    pub max_peers: usize,
    /// Announce every torrent to a list of large public trackers too.
    pub extra_trackers: bool,
    /// Seconds to wait for a peer to accept a connection. Not shown in the UI.
    pub connect_timeout_secs: u64,
    pub language: Lang,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            download_dir: default_download_dir(),
            // Outside the classic 6881-6889 range, which some ISPs throttle.
            listen_port: 42_000 + (std::process::id() % 1000) as u16,
            upnp: true,
            download_limit_kib: 0,
            upload_limit_kib: 0,
            start_on_add: true,
            max_peers: 250,
            extra_trackers: true,
            connect_timeout_secs: 4,
            language: Lang::detect(),
        }
    }
}

impl Settings {
    /// Loads settings, falling back to defaults when the file is missing or unreadable.
    pub fn load(path: &Path) -> Self {
        let Ok(raw) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        serde_json::from_str(&raw).unwrap_or_else(|err| {
            tracing::warn!("ignoring malformed settings file: {err}");
            Self::default()
        })
    }

    /// Writes settings atomically (write to a temp file, then rename).
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(self)?)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }
}

fn default_download_dir() -> PathBuf {
    UserDirs::new()
        .and_then(|d| d.download_dir().map(Path::to_path_buf))
        .or_else(|| UserDirs::new().map(|d| d.home_dir().join("Downloads")))
        .unwrap_or_else(|| PathBuf::from("Downloads"))
        .join("Riptide")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrips_through_disk() {
        let dir = std::env::temp_dir().join(format!("riptide-test-{}", std::process::id()));
        let path = dir.join("settings.json");
        let settings = Settings {
            download_limit_kib: 512,
            ..Settings::default()
        };
        settings.save(&path).unwrap();
        assert_eq!(Settings::load(&path), settings);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn tolerates_missing_fields() {
        let parsed: Settings = serde_json::from_str(r#"{"upnp": false}"#).unwrap();
        assert!(!parsed.upnp);
        assert!(parsed.start_on_add);
    }
}
