//! The torrent list: a sortable header and custom-painted rows.

use eframe::egui::{
    self, Align2, Color32, FontId, Rect, RichText, Sense, Stroke, TextStyle, Ui, pos2, vec2,
};

use super::icons::Icon;
use super::{App, SortKey, theme, widgets};
use crate::engine::{Status, TorrentId, TorrentView};
use crate::format;
use crate::i18n::Strings;

const PAD: f32 = 16.0;

/// Right-hand numeric columns: (key, width).
const COLUMNS: [(SortKey, f32); 5] = [
    (SortKey::Size, 84.0),
    (SortKey::Down, 92.0),
    (SortKey::Up, 92.0),
    (SortKey::Eta, 76.0),
    (SortKey::Peers, 64.0),
];

fn column_label(key: SortKey, s: &Strings) -> &'static str {
    match key {
        SortKey::Added | SortKey::Name => s.name,
        SortKey::Size => s.size,
        SortKey::Down => s.down,
        SortKey::Up => s.up,
        SortKey::Eta => s.eta,
        SortKey::Peers => s.peers,
    }
}

pub fn status_label(status: Status, s: &Strings) -> &'static str {
    match status {
        Status::Checking => s.checking,
        Status::Downloading => s.downloading,
        Status::Seeding => s.seeding,
        Status::Paused => s.paused,
        Status::Done => s.completed,
        Status::Error => s.error,
    }
}

/// Splits a row into the name area and the numeric column rects.
fn layout(row: Rect) -> (Rect, Vec<Rect>) {
    let fixed: f32 = COLUMNS.iter().map(|(_, w)| w).sum();
    // Hide numeric columns from the right when the window gets narrow.
    let min_name = 220.0;
    let mut visible = COLUMNS.len();
    while visible > 0
        && row.width() - PAD * 2.0 - COLUMNS[..visible].iter().map(|c| c.1).sum::<f32>() < min_name
    {
        visible -= 1;
    }
    let used: f32 = COLUMNS[..visible].iter().map(|c| c.1).sum();
    let _ = fixed;
    let name = Rect::from_min_max(
        pos2(row.left() + PAD, row.top()),
        pos2(row.right() - PAD - used - 12.0, row.bottom()),
    );
    let mut x = row.right() - PAD - used;
    let cols = COLUMNS[..visible]
        .iter()
        .map(|(_, w)| {
            let r = Rect::from_min_max(pos2(x, row.top()), pos2(x + w, row.bottom()));
            x += w;
            r
        })
        .collect();
    (name, cols)
}

impl App {
    pub(super) fn list(&mut self, ui: &mut Ui) {
        let s = self.s();
        let visible: Vec<TorrentView> = self.visible().into_iter().cloned().collect();
        let pending = self.pending.clone();

        if visible.is_empty() && pending.is_empty() {
            self.empty_state(ui);
            return;
        }
        self.header(ui);

        let rows = pending.len() + visible.len();
        egui::ScrollArea::vertical().auto_shrink(false).show_rows(
            ui,
            theme::ROW_HEIGHT,
            rows,
            |ui, range| {
                ui.spacing_mut().item_spacing.y = 0.0;
                for i in range {
                    if let Some(p) = pending.get(i) {
                        let (rect, _) = ui.allocate_exact_size(
                            vec2(ui.available_width(), theme::ROW_HEIGHT),
                            Sense::hover(),
                        );
                        self.pending_row(ui, rect, &p.name, p.since.elapsed().as_secs_f32(), s);
                    } else {
                        let t = &visible[i - pending.len()];
                        self.torrent_row(ui, t, &visible);
                    }
                }
            },
        );
    }

    fn header(&mut self, ui: &mut Ui) {
        let s = self.s();
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::hover());
        let painter = ui.painter().clone();
        painter.line_segment(
            [rect.left_bottom(), rect.right_bottom()],
            Stroke::new(1.0, theme::BORDER),
        );
        let (name_rect, cols) = layout(rect);
        let font = TextStyle::Small.resolve(ui.style());

        let mut cells: Vec<(SortKey, Rect, Align2)> =
            vec![(SortKey::Name, name_rect, Align2::LEFT_CENTER)];
        for ((key, _), r) in COLUMNS.iter().zip(cols) {
            cells.push((*key, r, Align2::RIGHT_CENTER));
        }

        for (key, r, align) in cells {
            let resp = ui.interact(r, ui.scope_id().with(("sort", key as u8)), Sense::click());
            let active = self.sort == key;
            let color = if active || resp.hovered() {
                theme::TEXT
            } else {
                theme::TEXT_FAINT
            };
            let mut label = column_label(key, s).to_uppercase();
            if active {
                label.push_str(if self.sort_desc { " ↓" } else { " ↑" });
            }
            let anchor = if align == Align2::LEFT_CENTER {
                r.left_center()
            } else {
                r.right_center()
            };
            painter.text(anchor, align, label, font.clone(), color);
            if resp
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
            {
                if active {
                    // Third click on the same column returns to the default order.
                    if self.sort_desc == default_desc(key) {
                        self.sort_desc = !self.sort_desc;
                    } else {
                        self.sort = SortKey::Added;
                        self.sort_desc = true;
                    }
                } else {
                    self.sort = key;
                    self.sort_desc = default_desc(key);
                }
            }
        }
    }

    fn torrent_row(&mut self, ui: &mut Ui, t: &TorrentView, visible: &[TorrentView]) {
        let s = self.s();
        let width = ui.available_width();
        let (rect, resp) = ui.allocate_exact_size(vec2(width, theme::ROW_HEIGHT), Sense::click());
        let selected = self.selected.contains(&t.id);

        if resp.clicked() || (resp.secondary_clicked() && !selected) {
            self.click_row(ui, t.id, visible, resp.secondary_clicked());
        }
        if resp.double_clicked() {
            self.open_folder(t);
        }

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            if selected {
                painter.rect_filled(rect, 0.0, theme::SELECTION);
                painter.rect_filled(
                    Rect::from_min_size(rect.min, vec2(2.0, rect.height())),
                    0.0,
                    theme::ACCENT,
                );
            } else if resp.hovered() {
                painter.rect_filled(rect, 0.0, theme::SURFACE);
            }
            painter.line_segment(
                [rect.left_bottom(), rect.right_bottom()],
                Stroke::new(1.0, Color32::from_rgb(0x1d, 0x1f, 0x22)),
            );

            let (name_rect, cols) = layout(rect);
            let body = TextStyle::Body.resolve(ui.style());
            let small = TextStyle::Small.resolve(ui.style());

            // Line 1: name, elided to fit.
            let name_color = if t.status == Status::Paused || t.status == Status::Done {
                theme::TEXT_DIM
            } else {
                theme::TEXT
            };
            let galley = elided(ui, &t.name, body.clone(), name_color, name_rect.width());
            painter.galley(pos2(name_rect.left(), rect.top() + 8.0), galley, name_color);

            // Line 2: status, progress bar, percentage.
            let color = theme::status_color(t.status);
            let y = rect.top() + 34.0;
            let status = match (&t.error, t.status) {
                (Some(_), Status::Error) => s.error.to_owned(),
                _ => status_label(t.status, s).to_owned(),
            };
            let status_rect = painter.text(
                pos2(name_rect.left(), y),
                Align2::LEFT_CENTER,
                &status,
                small.clone(),
                color,
            );
            let pct = format!("{:.1}%", t.progress * 100.0);
            let pct_rect = painter.text(
                pos2(name_rect.right(), y),
                Align2::RIGHT_CENTER,
                &pct,
                small.clone(),
                theme::TEXT_DIM,
            );
            let bar = Rect::from_min_max(
                pos2(status_rect.right() + 12.0, y - 2.0),
                pos2(pct_rect.left() - 10.0, y + 2.0),
            );
            if bar.width() > 30.0 {
                widgets::paint_progress(painter, bar, t.progress, color);
            }

            // Numeric columns.
            let eta = t.eta_secs.map(format::eta).unwrap_or_default();
            let connected = matches!(t.status, Status::Downloading | Status::Seeding);
            let peers = if connected {
                t.peers.to_string()
            } else {
                String::new()
            };
            let values = [
                (format::bytes(t.total_bytes), theme::TEXT_DIM),
                (format::speed(t.down_speed), theme::TEXT),
                (format::speed(t.up_speed), theme::TEXT_DIM),
                (eta, theme::TEXT_DIM),
                (peers, theme::TEXT_DIM),
            ];
            for (r, (text, color)) in cols.iter().zip(values) {
                painter.text(
                    r.right_center(),
                    Align2::RIGHT_CENTER,
                    text,
                    body.clone(),
                    color,
                );
            }
        }

        if let Some(error) = &t.error {
            let resp = resp.clone();
            resp.on_hover_text(RichText::new(error).color(theme::RED));
        }

        resp.context_menu(|ui| self.row_menu(ui, t));
    }

    fn click_row(&mut self, ui: &Ui, id: TorrentId, visible: &[TorrentView], secondary: bool) {
        let mods = ui.input(|i| i.modifiers);
        if secondary {
            self.selected = [id].into();
            self.anchor = Some(id);
        } else if mods.shift && self.anchor.is_some() {
            let pos = |x: TorrentId| visible.iter().position(|t| t.id == x);
            if let (Some(a), Some(b)) = (self.anchor.and_then(pos), pos(id)) {
                let (lo, hi) = (a.min(b), a.max(b));
                self.selected = visible[lo..=hi].iter().map(|t| t.id).collect();
            }
        } else if mods.command {
            if !self.selected.remove(&id) {
                self.selected.insert(id);
            }
            self.anchor = Some(id);
        } else {
            self.selected = [id].into();
            self.anchor = Some(id);
        }
    }

    fn row_menu(&mut self, ui: &mut Ui, t: &TorrentView) {
        let s = self.s();
        ui.set_min_width(200.0);
        let multi = self.selected.len() > 1;
        if (t.status.is_active() || multi) && menu_item(ui, Icon::Pause, s.pause) {
            self.pause_selected();
            ui.close();
        }
        if (!t.status.is_active() || multi) && menu_item(ui, Icon::Play, s.resume) {
            self.resume_selected();
            ui.close();
        }
        ui.separator();
        if menu_item(ui, Icon::Folder, s.open_folder) {
            self.open_folder(t);
            ui.close();
        }
        if menu_item(ui, Icon::Link, s.copy_magnet) {
            let links: Vec<String> = self
                .selected_views()
                .map(TorrentView::magnet_link)
                .collect();
            ui.ctx().copy_text(links.join("\n"));
            self.toasts.info(s.copied, "");
            ui.close();
        }
        ui.separator();
        if menu_item(ui, Icon::Trash, s.remove) {
            self.ask_remove(false);
            ui.close();
        }
        if menu_item(ui, Icon::Trash, s.remove_with_files) {
            self.ask_remove(true);
            ui.close();
        }
    }

    fn pending_row(&self, ui: &Ui, rect: Rect, name: &str, elapsed: f32, s: &Strings) {
        let painter = ui.painter();
        painter.line_segment(
            [rect.left_bottom(), rect.right_bottom()],
            Stroke::new(1.0, Color32::from_rgb(0x1d, 0x1f, 0x22)),
        );
        let (name_rect, _) = layout(rect);
        let body = TextStyle::Body.resolve(ui.style());
        let small = TextStyle::Small.resolve(ui.style());
        let galley = elided(ui, name, body, theme::TEXT_DIM, name_rect.width());
        painter.galley(
            pos2(name_rect.left(), rect.top() + 8.0),
            galley,
            theme::TEXT_DIM,
        );

        let y = rect.top() + 34.0;
        let label = painter.text(
            pos2(name_rect.left(), y),
            Align2::LEFT_CENTER,
            s.fetching_metadata,
            small,
            theme::AMBER,
        );
        // Indeterminate bar: a short segment sliding back and forth.
        let track = Rect::from_min_max(
            pos2(label.right() + 12.0, y - 2.0),
            pos2(name_rect.right(), y + 2.0),
        );
        if track.width() > 40.0 {
            painter.rect_filled(track, 2.0, theme::BORDER);
            let seg = track.width() * 0.25;
            let t = (elapsed * 0.8).fract();
            let phase = if t < 0.5 { t * 2.0 } else { 2.0 - t * 2.0 };
            let x = track.left() + (track.width() - seg) * phase;
            painter.rect_filled(
                Rect::from_min_max(pos2(x, track.top()), pos2(x + seg, track.bottom())),
                2.0,
                theme::AMBER,
            );
        }
    }

    fn empty_state(&mut self, ui: &mut Ui) {
        let s = self.s();
        let filtered = !self.torrents.is_empty();
        ui.add_space((ui.available_height() * 0.3).max(40.0));
        ui.vertical_centered(|ui| {
            if filtered {
                ui.label(RichText::new(s.no_match).color(theme::TEXT_FAINT));
                return;
            }
            let (r, _) = ui.allocate_exact_size(vec2(36.0, 36.0), Sense::hover());
            Icon::ArrowDown.paint(ui.painter(), r, theme::TEXT_FAINT);
            ui.add_space(12.0);
            ui.label(RichText::new(s.empty_title).size(16.0).color(theme::TEXT));
            ui.add_space(6.0);
            ui.label(RichText::new(s.empty_hint).color(theme::TEXT_DIM));
            ui.add_space(18.0);
            if widgets::button(ui, Some(Icon::Plus), s.add_torrent, true).clicked() {
                self.add_dialog = Some(super::AddDialog::empty(&self.settings));
            }
        });
    }
}

fn default_desc(key: SortKey) -> bool {
    !matches!(key, SortKey::Name | SortKey::Eta)
}

fn menu_item(ui: &mut Ui, icon: Icon, label: &str) -> bool {
    let (rect, resp) =
        ui.allocate_exact_size(vec2(ui.available_width().max(200.0), 30.0), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect, 4.0, theme::SURFACE_HOVER);
    }
    let icon_rect = Rect::from_center_size(rect.left_center() + vec2(18.0, 0.0), vec2(14.0, 14.0));
    icon.paint(ui.painter(), icon_rect, theme::TEXT_DIM);
    ui.painter().text(
        rect.left_center() + vec2(36.0, 0.0),
        Align2::LEFT_CENTER,
        label,
        TextStyle::Body.resolve(ui.style()),
        theme::TEXT,
    );
    resp.clicked()
}

/// Lays out `text` on one line, truncating with an ellipsis to fit `width`.
fn elided(
    ui: &Ui,
    text: &str,
    font: FontId,
    color: Color32,
    width: f32,
) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple_singleline(text.to_owned(), font, color);
    job.wrap = egui::text::TextWrapping::truncate_at_width(width.max(10.0));
    ui.fonts_mut(|f| f.layout_job(job))
}
