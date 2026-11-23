#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;

fn main() -> anyhow::Result<()> {
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
