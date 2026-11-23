//! Bottom panel with details for the selected torrent.

use std::collections::HashSet;

use eframe::egui::{self, Align2, RichText, Sense, Stroke, TextStyle, Ui, vec2};

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
        let s = self.s();
        let files = self.detail.files.clone();
        let mut toggled: Option<usize> = None;
        let can_toggle = files.len() > 1;

        egui::ScrollArea::vertical().auto_shrink(false).show_rows(
            ui,
            30.0,
            files.len(),
            |ui, range| {
                ui.spacing_mut().item_spacing.y = 0.0;
                for f in &files[range] {
                    let (rect, resp) =
                        ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::hover());
                    if resp.hovered() {
                        ui.painter().rect_filled(rect, 4.0, theme::SURFACE);
                    }
                    let mut child = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(rect.shrink2(vec2(6.0, 0.0)))
                            .layout(egui::Layout::left_to_right(egui::Align::Center)),
                    );
                    let mut checked = f.selected;
                    if can_toggle {
                        if widgets::checkbox(&mut child, &mut checked, "").changed() {
                            toggled = Some(f.index);
                        }
                    } else {
                        child.add_space(2.0);
                    }

                    let right_w = 260.0;
                    let name_w = (child.available_width() - right_w).max(60.0);
                    widgets::path_label(&mut child, &f.path, f.selected, name_w);

                    let progress = if f.size == 0 {
                        1.0
                    } else {
                        f.done as f32 / f.size as f32
                    };
                    let painter = child.painter().clone();
                    let r = child.available_rect_before_wrap();
                    let size_text = format::bytes(f.size);
                    painter.text(
                        r.right_center(),
                        Align2::RIGHT_CENTER,
                        size_text,
                        TextStyle::Small.resolve(child.style()),
                        theme::TEXT_DIM,
                    );
                    let pct_x = r.right() - 80.0;
                    painter.text(
                        egui::pos2(pct_x, r.center().y),
                        Align2::RIGHT_CENTER,
                        format!("{:.0}%", progress * 100.0),
                        TextStyle::Small.resolve(child.style()),
                        theme::TEXT_DIM,
                    );
                    let bar = egui::Rect::from_min_max(
                        egui::pos2(r.left() + 12.0, r.center().y - 2.0),
                        egui::pos2(pct_x - 44.0, r.center().y + 2.0),
                    );
                    let color = if !f.selected {
                        theme::GREY
                    } else if progress >= 1.0 {
                        theme::BLUE
                    } else {
                        theme::ACCENT
                    };
                    if bar.width() > 20.0 {
                        widgets::paint_progress(&painter, bar, progress, color);
                    }
                }
            },
        );

        if let (Some(index), Some(engine)) = (toggled, &self.engine) {
            let mut selected: HashSet<usize> = files
                .iter()
                .filter(|f| f.selected)
                .map(|f| f.index)
                .collect();
            if !selected.remove(&index) {
                selected.insert(index);
            }
            // Never deselect everything: a torrent with nothing to download is just confusing.
            if !selected.is_empty() {
                engine.set_selected_files(id, selected.clone());
                for f in &mut self.detail.files {
                    f.selected = selected.contains(&f.index);
                }
            }
        }
        let _ = s;
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
