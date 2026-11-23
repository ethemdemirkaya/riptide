//! Reusable widgets in the app's visual language.

use eframe::egui::{
    self, Color32, CornerRadius, Response, Sense, Stroke, StrokeKind, Ui, Vec2, WidgetText, vec2,
};

use super::icons::{Icon, icon_rect};
use super::theme;

/// A square, borderless button showing only an icon, with a tooltip.
pub fn icon_button(ui: &mut Ui, icon: Icon, tooltip: &str, enabled: bool) -> Response {
    let size = Vec2::splat(30.0);
    let (rect, resp) = ui.allocate_exact_size(
        size,
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        if enabled && resp.hovered() {
            let fill = if resp.is_pointer_button_down_on() {
                theme::SURFACE_ACTIVE
            } else {
                theme::SURFACE_HOVER
            };
            painter.rect_filled(rect, theme::RADIUS, fill);
        }
        let color = if !enabled {
            theme::TEXT_FAINT
        } else if resp.hovered() {
            theme::TEXT
        } else {
            theme::TEXT_DIM
        };
        icon.paint(painter, icon_rect(rect.center(), 16.0), color);
    }
    resp.on_hover_text(tooltip)
}

/// A button with an icon and a label. `primary` uses the accent fill.
pub fn button(ui: &mut Ui, icon: Option<Icon>, label: &str, primary: bool) -> Response {
    button_enabled(ui, icon, label, primary, true)
}

pub fn button_enabled(
    ui: &mut Ui,
    icon: Option<Icon>,
    label: &str,
    primary: bool,
    enabled: bool,
) -> Response {
    let font = egui::TextStyle::Button.resolve(ui.style());
    let text_color = match (primary, enabled) {
        (_, false) => theme::TEXT_FAINT,
        (true, true) => theme::ACCENT_TEXT,
        (false, true) => theme::TEXT,
    };
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font, text_color);
    let icon_w = if icon.is_some() { 14.0 + 7.0 } else { 0.0 };
    let pad = vec2(14.0, 0.0);
    let size = vec2(galley.size().x + icon_w + pad.x * 2.0, 32.0);
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(size, sense);

    if ui.is_rect_visible(rect) {
        let hovered = enabled && resp.hovered();
        let pressed = enabled && resp.is_pointer_button_down_on();
        let (fill, stroke) = if primary {
            let fill = match (enabled, hovered) {
                (false, _) => theme::SURFACE,
                (true, true) => theme::ACCENT_HOVER,
                (true, false) => theme::ACCENT,
            };
            (fill, Stroke::NONE)
        } else {
            let fill = if pressed {
                theme::SURFACE_ACTIVE
            } else if hovered {
                theme::SURFACE_HOVER
            } else {
                theme::SURFACE
            };
            let border = if hovered {
                theme::BORDER_STRONG
            } else {
                theme::BORDER
            };
            (fill, Stroke::new(1.0, border))
        };
        let painter = ui.painter();
        painter.rect(rect, theme::RADIUS, fill, stroke, StrokeKind::Inside);

        let mut x = rect.left() + pad.x;
        if let Some(icon) = icon {
            icon.paint(
                painter,
                icon_rect(egui::pos2(x + 7.0, rect.center().y), 14.0),
                text_color,
            );
            x += icon_w;
        }
        painter.galley(
            egui::pos2(x, rect.center().y - galley.size().y / 2.0),
            galley,
            text_color,
        );
    }
    if enabled {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        resp
    }
}

pub fn paint_progress(painter: &egui::Painter, rect: egui::Rect, progress: f32, color: Color32) {
    let radius = CornerRadius::same(2);
    painter.rect_filled(rect, radius, theme::BORDER);
    let progress = progress.clamp(0.0, 1.0);
    if progress > 0.0 {
        let mut fill = rect;
        fill.set_width((rect.width() * progress).max(4.0));
        painter.rect_filled(fill, radius, color);
    }
}

/// Small colored dot followed by a label, used for statuses.
pub fn status_dot(ui: &mut Ui, color: Color32, label: &str) {
    let font = egui::TextStyle::Body.resolve(ui.style());
    let mut job = egui::text::LayoutJob::default();
    job.append("● ", 0.0, egui::TextFormat::simple(font.clone(), color));
    job.append(label, 0.0, egui::TextFormat::simple(font, theme::TEXT));
    ui.label(job);
}

/// A relative path with the folder part dimmed so the file name stands out.
pub fn path_label(ui: &mut Ui, path: &std::path::Path, enabled: bool, width: f32) {
    let font = egui::TextStyle::Body.resolve(ui.style());
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let dir = path
        .parent()
        .map(|p| p.to_string_lossy().into_owned())
        .filter(|p| !p.is_empty());
    let (dir_color, name_color) = if enabled {
        (theme::TEXT_FAINT, theme::TEXT)
    } else {
        (theme::TEXT_FAINT, theme::TEXT_FAINT)
    };
    let mut job = egui::text::LayoutJob::default();
    if let Some(dir) = dir {
        job.append(
            &format!("{dir}{}", std::path::MAIN_SEPARATOR),
            0.0,
            egui::TextFormat::simple(font.clone(), dir_color),
        );
    }
    job.append(&name, 0.0, egui::TextFormat::simple(font, name_color));
    job.wrap = egui::text::TextWrapping::truncate_at_width(width.max(20.0));
    ui.allocate_ui_with_layout(
        vec2(width, 20.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_width(width);
            ui.add(egui::Label::new(job).selectable(false))
                .on_hover_text(path.display().to_string());
        },
    );
}

/// A labelled value pair for detail views.
pub fn field(ui: &mut Ui, label: &str, value: impl Into<WidgetText>) {
    ui.label(egui::RichText::new(label).color(theme::TEXT_FAINT).small());
    ui.add(egui::Label::new(value).selectable(true));
}

/// Section title used in dialogs and settings.
pub fn section_title(ui: &mut Ui, text: &str) {
    ui.label(egui::RichText::new(text).color(theme::TEXT_DIM).strong());
}

/// A checkbox drawn to match the theme.
pub fn checkbox(ui: &mut Ui, checked: &mut bool, label: &str) -> Response {
    let font = egui::TextStyle::Body.resolve(ui.style());
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font, theme::TEXT);
    let box_size = 16.0;
    let gap = if label.is_empty() { 0.0 } else { 8.0 };
    let size = vec2(
        box_size + gap + galley.size().x,
        box_size.max(galley.size().y),
    );
    let (rect, mut resp) = ui.allocate_exact_size(size, Sense::click());
    if resp.clicked() {
        *checked = !*checked;
        resp.mark_changed();
    }
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let b = egui::Rect::from_min_size(
            egui::pos2(rect.left(), rect.center().y - box_size / 2.0),
            Vec2::splat(box_size),
        );
        if *checked {
            painter.rect_filled(b, 4.0, theme::ACCENT);
            let c = b.center();
            painter.add(egui::Shape::line(
                vec![
                    c + vec2(-4.0, 0.0),
                    c + vec2(-1.0, 3.0),
                    c + vec2(4.5, -3.0),
                ],
                Stroke::new(1.8, theme::ACCENT_TEXT),
            ));
        } else {
            let border = if resp.hovered() {
                theme::TEXT_DIM
            } else {
                theme::BORDER_STRONG
            };
            painter.rect_stroke(b, 4.0, Stroke::new(1.2, border), StrokeKind::Inside);
        }
        painter.galley(
            egui::pos2(b.right() + gap, rect.center().y - galley.size().y / 2.0),
            galley,
            theme::TEXT,
        );
    }
    resp
}
