#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// The engine is used by the interface added in the next change.
#[allow(dead_code)]
mod engine;
#[allow(dead_code)]
mod format;
#[allow(dead_code)]
mod settings;

use eframe::egui;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "warn,riptide=info".into()),
        )
        .init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Riptide")
            .with_app_id("riptide")
            .with_inner_size([1120.0, 700.0])
            .with_min_inner_size([760.0, 460.0]),
        ..Default::default()
    };
    eframe::run_ui_native("Riptide", options, |ui, _frame| {
        egui::CentralPanel::default_margins().show(ui, |ui| {
            ui.heading("Riptide");
        });
    })
    .map_err(|e| anyhow::anyhow!("{e}"))
}
