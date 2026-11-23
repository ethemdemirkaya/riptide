//! Preferences dialog. Edits a draft and applies it on save.

use eframe::egui::{self, RichText, Ui, vec2};

use super::icons::Icon;
use super::{App, theme, widgets};
use crate::i18n::Lang;
use crate::settings::Settings;

pub struct SettingsView {
    draft: Settings,
}

impl SettingsView {
    pub fn new(current: &Settings) -> Self {
        Self {
            draft: current.clone(),
        }
    }
}

impl App {
    pub(super) fn settings_modal(&mut self, ctx: &egui::Context) {
        let Some(view) = &mut self.settings_view else {
            return;
        };
        // Labels follow the draft language so switching previews immediately.
        let s = view.draft.language.strings();
        let mut close = false;
        let mut save = false;

        let modal = egui::Modal::new(egui::Id::unique("settings")).show(ctx, |ui| {
            ui.set_width(520.0);
            ui.heading(s.settings);
            ui.add_space(18.0);

            let d = &mut view.draft;

            section(ui, s.downloads, |ui| {
                row(ui, s.default_folder, |ui| {
                    ui.add(
                        egui::Label::new(
                            RichText::new(d.download_dir.display().to_string())
                                .color(theme::TEXT_DIM),
                        )
                        .truncate(),
                    );
                    let button = widgets::icon_button(ui, Icon::Folder, s.change, true);
                    if button.clicked()
                        && let Some(dir) = rfd::FileDialog::new()
                            .set_directory(&d.download_dir)
                            .pick_folder()
                    {
                        d.download_dir = dir;
                    }
                    button
                });
                ui.add_space(4.0);
                widgets::checkbox(ui, &mut d.start_on_add, s.start_now);
            });

            section(ui, s.speed_limits, |ui| {
                row(ui, s.download_limit, |ui| {
                    limit_input(ui, &mut d.download_limit_kib, s.unlimited)
                });
                row(ui, s.upload_limit, |ui| {
                    limit_input(ui, &mut d.upload_limit_kib, s.unlimited)
                });
            });

            section(ui, s.network, |ui| {
                row(ui, s.port, |ui| {
                    ui.add(
                        egui::DragValue::new(&mut d.listen_port)
                            .range(1024..=65535)
                            .speed(1.0),
                    )
                });
                ui.add_space(4.0);
                widgets::checkbox(ui, &mut d.upnp, s.upnp);
                ui.add_space(2.0);
                ui.label(
                    RichText::new(s.restart_note)
                        .small()
                        .color(theme::TEXT_FAINT),
                );
            });

            section(ui, s.interface, |ui| {
                row(ui, s.language, |ui| {
                    egui::ComboBox::from_id_salt("lang")
                        .selected_text(d.language.native_name())
                        .width(160.0)
                        .show_ui(ui, |ui| {
                            for lang in Lang::ALL {
                                ui.selectable_value(&mut d.language, lang, lang.native_name());
                            }
                        })
                        .response
                });
            });

            ui.add_space(10.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if widgets::button(ui, None, s.save, true).clicked() {
                    save = true;
                }
                if widgets::button(ui, None, s.cancel, false).clicked() {
                    close = true;
                }
            });
        });

        if modal.should_close() {
            close = true;
        }
        if save {
            let draft = view.draft.clone();
            if let Some(engine) = &self.engine {
                engine.set_limits(draft.download_limit_kib, draft.upload_limit_kib);
            }
            self.settings = draft;
            self.save_settings();
            close = true;
        }
        if close {
            self.settings_view = None;
        }
    }
}

fn section(ui: &mut Ui, title: &str, body: impl FnOnce(&mut Ui)) {
    widgets::section_title(ui, title);
    ui.add_space(6.0);
    egui::Frame::new()
        .fill(theme::BG)
        .stroke(egui::Stroke::new(1.0, theme::BORDER))
        .corner_radius(8)
        .inner_margin(egui::Margin::same(12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            body(ui);
        });
    ui.add_space(16.0);
}

/// A label and its control. The control is linked to the label for screen readers.
fn row(ui: &mut Ui, label: &str, value: impl FnOnce(&mut Ui) -> egui::Response) {
    ui.horizontal(|ui| {
        let label = ui
            .allocate_ui_with_layout(
                vec2(150.0, 28.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.set_width(150.0);
                    ui.label(RichText::new(label).color(theme::TEXT_DIM))
                },
            )
            .inner;
        value(ui).labelled_by(label.id);
    });
}

fn limit_input(ui: &mut Ui, value: &mut u32, unlimited: &'static str) -> egui::Response {
    ui.add(
        egui::DragValue::new(value)
            .range(0..=1_000_000)
            .speed(16.0)
            .custom_formatter(move |v, _| {
                if v <= 0.0 {
                    unlimited.to_owned()
                } else {
                    format!("{v:.0} KB/s")
                }
            })
            .custom_parser(|text| {
                let digits: String = text.chars().filter(char::is_ascii_digit).collect();
                Some(digits.parse::<f64>().unwrap_or(0.0))
            }),
    )
}
