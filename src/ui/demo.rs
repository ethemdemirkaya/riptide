//! Offscreen renders of the interface with sample data.
//!
//! These double as visual checks during development and as the source of the
//! screenshots in the README. They need a GPU (or a software Vulkan/DX12 adapter),
//! so they are ignored by default:
//!
//! ```sh
//! cargo test screenshots -- --ignored
//! ```
//!
//! Images are written to `docs/screenshots/`.

use std::path::PathBuf;
use std::time::Instant;

use eframe::egui;
use egui_kittest::Harness;

use super::{App, DetailsTab};
use crate::engine::{FileView, PeerView, PendingView, Status, TorrentView};
use crate::i18n::Lang;
use crate::settings::{Paths, Settings};

const GB: u64 = 1024 * 1024 * 1024;
const MB: u64 = 1024 * 1024;

#[allow(clippy::too_many_arguments)]
fn torrent(
    id: usize,
    name: &str,
    status: Status,
    progress: f32,
    total: u64,
    down: u64,
    up: u64,
    peers: u32,
) -> TorrentView {
    let done = (total as f64 * progress as f64) as u64;
    TorrentView {
        id,
        info_hash: format!(
            "{:040x}",
            0x5eed_u128.wrapping_mul(id as u128 + 7) * 0x9e37_79b9_7f4a_7c15
        ),
        name: name.to_owned(),
        status,
        error: (status == Status::Error).then(|| "No space left on device".to_owned()),
        progress,
        total_bytes: total,
        done_bytes: done,
        uploaded_bytes: done / 3,
        down_speed: down,
        up_speed: up,
        eta_secs: (status == Status::Downloading && down > 0).then(|| (total - done) / down),
        peers,
        seeds: peers / 2,
        output_folder: PathBuf::from(r"C:\Users\you\Downloads").join(name),
    }
}

impl App {
    fn demo(cc: &eframe::CreationContext<'_>, lang: Lang) -> Self {
        super::theme::apply(&cc.egui_ctx);
        let tmp = std::env::temp_dir().join("riptide-demo");
        let paths = Paths {
            config_file: tmp.join("settings.json"),
            session_dir: tmp.join("session"),
        };
        let mut app = Self::with_engine(
            None,
            None,
            paths,
            Settings {
                language: lang,
                download_dir: PathBuf::from(r"C:\Users\you\Downloads"),
                download_limit_kib: 0,
                upload_limit_kib: 2048,
                ..Settings::default()
            },
        );
        app.torrents = vec![
            torrent(
                1,
                "debian-13.2.0-amd64-netinst.iso",
                Status::Seeding,
                1.0,
                754 * MB,
                0,
                412 * 1024,
                6,
            ),
            torrent(
                2,
                "Big Buck Bunny (2008) 1080p",
                Status::Downloading,
                0.62,
                3 * GB + 400 * MB,
                6_400 * 1024,
                220 * 1024,
                38,
            ),
            torrent(
                3,
                "ubuntu-26.04-desktop-amd64.iso",
                Status::Downloading,
                0.18,
                6 * GB,
                11_800 * 1024,
                96 * 1024,
                112,
            ),
            torrent(
                4,
                "Sintel (2010) 4K Open Movie",
                Status::Paused,
                0.41,
                9 * GB,
                0,
                0,
                0,
            ),
            torrent(
                5,
                "archlinux-2026.11.01-x86_64.iso",
                Status::Done,
                1.0,
                GB + 300 * MB,
                0,
                0,
                0,
            ),
            torrent(
                6,
                "LibreOffice 26.2 Collection",
                Status::Checking,
                0.73,
                2 * GB,
                0,
                0,
                0,
            ),
            torrent(
                7,
                "enwiki-20261101-pages-articles.xml.bz2",
                Status::Error,
                0.05,
                24 * GB,
                0,
                0,
                0,
            ),
        ];
        app.pending = vec![PendingView {
            key: 1,
            name: "Tears of Steel (2012) Remastered".to_owned(),
            since: Instant::now(),
        }];
        app.speed = (18_200 * 1024, 728 * 1024);
        app
    }
}

fn output(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("docs")
        .join("screenshots");
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(format!("{name}.png"))
}

fn render(name: &str, lang: Lang, setup: impl FnOnce(&mut App)) {
    let mut setup = Some(setup);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1180.0, 720.0))
        .with_pixels_per_point(1.5)
        .with_max_steps(8)
        .build_eframe(|cc| {
            let mut app = App::demo(cc, lang);
            if let Some(f) = setup.take() {
                f(&mut app);
            }
            app
        });
    harness.run_steps(4);
    let image = harness.render().expect("render failed");
    image.save(output(name)).unwrap();
}

#[test]
#[ignore = "needs a GPU adapter"]
fn screenshots() {
    render("main", Lang::En, |app| {
        app.selected = [2].into();
        app.anchor = Some(2);
    });
    render("main-tr", Lang::Tr, |app| {
        app.selected = [3].into();
    });
    render("files", Lang::En, |app| {
        app.selected = [2].into();
        app.details_tab = DetailsTab::Files;
        app.detail.id = Some(2);
        app.detail.refreshed = Some(Instant::now());
        app.detail.files = vec![
            FileView {
                index: 0,
                path: "bbb_sunflower_1080p_60fps.mp4".into(),
                size: 3 * GB,
                done: 2 * GB,
                selected: true,
            },
            FileView {
                index: 1,
                path: "Subtitles/en.srt".into(),
                size: 48 * 1024,
                done: 48 * 1024,
                selected: true,
            },
            FileView {
                index: 2,
                path: "Subtitles/tr.srt".into(),
                size: 51 * 1024,
                done: 0,
                selected: false,
            },
            FileView {
                index: 3,
                path: "poster.jpg".into(),
                size: 2 * MB,
                done: 2 * MB,
                selected: true,
            },
        ];
    });
    render("peers", Lang::En, |app| {
        app.selected = [3].into();
        app.details_tab = DetailsTab::Peers;
        app.detail.id = Some(3);
        app.detail.refreshed = Some(Instant::now());
        app.detail.peers = (0..6)
            .map(|i| PeerView {
                addr: format!("198.51.100.{}:{}", 20 + i * 13, 51413 + i),
                client: [
                    "qBittorrent 5.1",
                    "Transmission 4.1",
                    "Riptide 0.1.0",
                    "rqbit 9.1",
                    "",
                    "Deluge 2.2",
                ][i as usize]
                    .to_owned(),
                downloaded: (40 - i as u64 * 6) * MB,
                uploaded: i as u64 * 3 * MB,
                utp: i % 2 == 0,
            })
            .collect();
    });
    render("empty", Lang::En, |app| {
        app.torrents.clear();
        app.pending.clear();
        app.speed = (0, 0);
    });
}
