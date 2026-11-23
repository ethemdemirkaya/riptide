//! Bottom panel with details for the selected torrent.

use std::collections::HashSet;

use eframe::egui::{self, RichText, Sense, Stroke, TextStyle, Ui, vec2};

use super::file_tree::TreeFile;
use super::icons::Icon;
use super::list::status_label;
use super::{App, DetailsTab, theme, widgets};
use crate::engine::{Status, TorrentId};
use crate::format;

impl App {
    pub(super) fn details(&mut self, ui: &mut Ui, id: TorrentId) {
        let s = self.s();
        let Some(t) = self.torrents.iter().find(|t| t.id == id).cloned() else {
            return;
        };

        // Tab bar.
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 20.0;
            for (tab, label) in [
                (DetailsTab::General, s.general),
                (DetailsTab::Files, s.files),
                (DetailsTab::Peers, s.peers),
            ] {
                if tab_button(ui, label, self.details_tab == tab) && self.details_tab != tab {
                    self.details_tab = tab;
                    // Load the new tab's data right away instead of on the next tick.
                    self.detail.refreshed = None;
                    self.refresh(true);
                }
            }
        });
        ui.add_space(10.0);

        match self.details_tab {
            DetailsTab::General => self.general_tab(ui, &t),
            DetailsTab::Files => self.files_tab(ui, id),
            DetailsTab::Peers => self.peers_tab(ui),
        }
    }

    fn general_tab(&mut self, ui: &mut Ui, t: &crate::engine::TorrentView) {
        let s = self.s();
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                ui.label(RichText::new(&t.name).size(15.0).color(theme::TEXT));
                ui.add_space(12.0);

                let col_width = ((ui.available_width() - 32.0) / 4.0).max(120.0);
                egui::Grid::new("general")
                    .num_columns(4)
                    .min_col_width(col_width)
                    .spacing(vec2(16.0, 14.0))
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.label(RichText::new(s.status).color(theme::TEXT_FAINT).small());
                            widgets::status_dot(
                                ui,
                                theme::status_color(t.status),
                                status_label(t.status, s),
                            );
                        });
                        ui.vertical(|ui| {
                            widgets::field(
                                ui,
                                s.total_size,
                                format!(
                                    "{} / {}",
                                    format::bytes(t.done_bytes),
                                    format::bytes(t.total_bytes)
                                ),
                            )
                        });
                        ui.vertical(|ui| {
                            widgets::field(ui, s.uploaded, format::bytes(t.uploaded_bytes))
                        });
                        ui.vertical(|ui| {
                            widgets::field(
                                ui,
                                s.ratio,
                                format::ratio(t.uploaded_bytes, t.done_bytes),
                            )
                        });
                        ui.end_row();

                        ui.vertical(|ui| widgets::field(ui, s.down, display_speed(t.down_speed)));
                        ui.vertical(|ui| widgets::field(ui, s.up, display_speed(t.up_speed)));
                        ui.vertical(|ui| {
                            let eta = t
                                .eta_secs
                                .map(format::eta)
                                .unwrap_or_else(|| "—".to_owned());
                            widgets::field(ui, s.eta, eta)
                        });
                        ui.vertical(|ui| {
                            widgets::field(ui, s.peers, (s.peers_summary)(t.peers, t.seeds))
                        });
                        ui.end_row();
                    });

                ui.add_space(14.0);
                ui.label(RichText::new(s.save_path).color(theme::TEXT_FAINT).small());
                ui.horizontal(|ui| {
                    ui.add(
                        egui::Label::new(t.output_folder.display().to_string())
                            .selectable(true)
                            .truncate(),
                    );
                    if widgets::icon_button(ui, Icon::Folder, s.open_folder, true).clicked() {
                        self.open_folder(t);
                    }
                });
                ui.add_space(8.0);
                ui.label(RichText::new(s.info_hash).color(theme::TEXT_FAINT).small());
                ui.horizontal(|ui| {
                    ui.add(
                        egui::Label::new(
                            RichText::new(&t.info_hash)
                                .monospace()
                                .color(theme::TEXT_DIM),
                        )
                        .selectable(true),
                    );
                    if widgets::icon_button(ui, Icon::Copy, s.copy_magnet, true).clicked() {
                        ui.ctx().copy_text(t.magnet_link());
                        self.toasts.info(s.copied, "");
                    }
                });
                if let Some(error) = &t.error {
                    ui.add_space(10.0);
                    ui.label(RichText::new(error).color(theme::RED));
                }
            });
    }

    fn files_tab(&mut self, ui: &mut Ui, id: TorrentId) {
        let files = self.detail.files.clone();
        let tree_files: Vec<TreeFile<'_>> = files
            .iter()
            .map(|f| TreeFile {
                path: &f.path,
                size: f.size,
                progress: Some(if f.size == 0 {
                    1.0
                } else {
                    f.done as f32 / f.size as f32
                }),
                selected: f.selected,
            })
            .collect();
        let selectable = files.len() > 1;
        let height = ui.available_height();
        let Some(toggle) = self.detail.tree.show(ui, &tree_files, selectable, height) else {
            return;
        };

        let mut selected: HashSet<usize> = files
            .iter()
            .filter(|f| f.selected)
            .map(|f| f.index)
            .collect();
        for pos in toggle.positions {
            let index = files[pos].index;
            if toggle.selected {
                selected.insert(index);
            } else {
                selected.remove(&index);
            }
        }
        // Never deselect everything: a torrent with nothing to download is just confusing.
        if selected.is_empty() {
            return;
        }
        if let Some(engine) = &self.engine {
            engine.set_selected_files(id, selected.clone());
        }
        for f in &mut self.detail.files {
            f.selected = selected.contains(&f.index);
        }
    }

    fn peers_tab(&mut self, ui: &mut Ui) {
        let s = self.s();
        let peers = &self.detail.peers;
        let status = self
            .focused()
            .and_then(|id| self.torrents.iter().find(|t| t.id == id))
            .map(|t| t.status);
        if peers.is_empty() {
            let text = if matches!(status, Some(Status::Paused | Status::Done)) {
                status_label(status.unwrap_or(Status::Paused), s)
            } else {
                s.no_peers
            };
            ui.label(RichText::new(text).color(theme::TEXT_FAINT));
            return;
        }

        let cols = [s.address, s.client, s.downloaded, s.uploaded];
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                egui::Grid::new("peers")
                    .num_columns(4)
                    .striped(false)
                    .spacing(vec2(28.0, 8.0))
                    .show(ui, |ui| {
                        for c in cols {
                            ui.label(
                                RichText::new(c.to_uppercase())
                                    .small()
                                    .color(theme::TEXT_FAINT),
                            );
                        }
                        ui.end_row();
                        for p in peers {
                            ui.label(RichText::new(&p.addr).monospace().color(theme::TEXT_DIM));
                            let client = if p.client.is_empty() {
                                "—"
                            } else {
                                &p.client
                            };
                            let mut label = client.to_owned();
                            if p.utp {
                                label.push_str("  · µTP");
                            }
                            ui.label(label);
                            ui.label(format::bytes(p.downloaded));
                            ui.label(
                                RichText::new(format::bytes(p.uploaded)).color(theme::TEXT_DIM),
                            );
                            ui.end_row();
                        }
                    });
            });
    }
}

fn display_speed(speed: u64) -> String {
    if speed == 0 {
        "—".to_owned()
    } else {
        format::speed(speed)
    }
}

/// Underlined text tab. Returns true when clicked.
fn tab_button(ui: &mut Ui, label: &str, active: bool) -> bool {
    let font = TextStyle::Body.resolve(ui.style());
    let color = if active { theme::TEXT } else { theme::TEXT_DIM };
    let galley = ui.painter().layout_no_wrap(label.to_owned(), font, color);
    let (rect, resp) = ui.allocate_exact_size(vec2(galley.size().x, 26.0), Sense::click());
    let hovered = resp.hovered();
    let color = if active || hovered {
        theme::TEXT
    } else {
        theme::TEXT_DIM
    };
    ui.painter()
        .galley(egui::pos2(rect.left(), rect.top() + 2.0), galley, color);
    if active {
        ui.painter().line_segment(
            [rect.left_bottom(), rect.right_bottom()],
            Stroke::new(2.0, theme::ACCENT),
        );
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
}
