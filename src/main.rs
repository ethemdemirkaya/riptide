#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod engine;
mod format;
mod i18n;
mod instance;
mod platform;
mod settings;
mod trackers;
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
    let args: Vec<String> = std::env::args().skip(1).collect();

    // If Riptide is already running, give it our arguments and get out of the way.
    let listener = match instance::acquire(&paths.config_file, &args) {
        instance::Instance::Forwarded => return Ok(()),
        instance::Instance::Primary(listener) => Some(listener),
        instance::Instance::Standalone => None,
    };
    // Show up in "Open with" for .torrent files (never changes the user's defaults).
    std::thread::spawn(platform::register_capabilities);

    let initial: Vec<engine::Source> = args
        .iter()
        .filter_map(|a| engine::Source::from_arg(a))
        .collect();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Riptide")
            .with_app_id("riptide")
            .with_inner_size([1120.0, 700.0])
            .with_min_inner_size([760.0, 460.0])
            .with_drag_and_drop(true)
            .with_icon(ui::logo::icon()),
        ..Default::default()
    };

    eframe::run_native(
        "Riptide",
        options,
        Box::new(move |cc| Ok(Box::new(ui::App::new(cc, paths, initial, listener)))),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))
}
