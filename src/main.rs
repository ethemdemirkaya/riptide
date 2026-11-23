#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod engine;
mod format;
mod i18n;
mod platform;
mod settings;
mod ui;

use eframe::egui;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "warn,riptide=info".into()),
        )
        .init();

    let paths = settings::Paths::resolve()?;

    // Torrents passed on the command line (file associations, magnet handler).
    let initial: Vec<engine::Source> = std::env::args()
        .skip(1)
        .filter_map(|arg| {
            if ui::is_link(&arg) {
                Some(engine::Source::Url(arg))
            } else {
                engine::Source::from_path(std::path::Path::new(&arg)).ok()
            }
        })
        .collect();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Riptide")
            .with_app_id("riptide")
            .with_inner_size([1120.0, 700.0])
            .with_min_inner_size([760.0, 460.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };

    eframe::run_native(
        "Riptide",
        options,
        Box::new(move |cc| Ok(Box::new(ui::App::new(cc, paths, initial)))),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))
}
