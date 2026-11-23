//! Colors, spacing and the egui style built from them.
//!
//! One accent color, neutral graphite surfaces, and status colors that are
//! only used where they carry meaning (progress bars and status labels).

use std::sync::Arc;

use eframe::egui::{
    self, Color32, CornerRadius, FontFamily, FontId, Margin, Stroke, TextStyle, Visuals,
};

use crate::engine::Status;

pub const BG: Color32 = Color32::from_rgb(0x12, 0x13, 0x15);
pub const SIDEBAR: Color32 = Color32::from_rgb(0x17, 0x18, 0x1b);
pub const SURFACE: Color32 = Color32::from_rgb(0x1c, 0x1e, 0x21);
pub const SURFACE_HOVER: Color32 = Color32::from_rgb(0x22, 0x24, 0x28);
pub const SURFACE_ACTIVE: Color32 = Color32::from_rgb(0x26, 0x2a, 0x2e);
pub const INPUT_BG: Color32 = Color32::from_rgb(0x0e, 0x0f, 0x11);
pub const BORDER: Color32 = Color32::from_rgb(0x2a, 0x2c, 0x31);
pub const BORDER_STRONG: Color32 = Color32::from_rgb(0x3a, 0x3d, 0x44);

pub const TEXT: Color32 = Color32::from_rgb(0xe6, 0xe7, 0xe9);
pub const TEXT_DIM: Color32 = Color32::from_rgb(0x9a, 0x9e, 0xa6);
pub const TEXT_FAINT: Color32 = Color32::from_rgb(0x66, 0x6a, 0x73);

pub const ACCENT: Color32 = Color32::from_rgb(0x38, 0xb2, 0xa6);
pub const ACCENT_HOVER: Color32 = Color32::from_rgb(0x45, 0xc4, 0xb7);
pub const ACCENT_TEXT: Color32 = Color32::from_rgb(0x0b, 0x1f, 0x1d);
pub const SELECTION: Color32 = Color32::from_rgb(0x1a, 0x33, 0x31);

pub const BLUE: Color32 = Color32::from_rgb(0x6f, 0x9c, 0xf0);
pub const AMBER: Color32 = Color32::from_rgb(0xd8, 0xa6, 0x4a);
pub const RED: Color32 = Color32::from_rgb(0xe0, 0x65, 0x5f);
pub const GREY: Color32 = Color32::from_rgb(0x70, 0x74, 0x7c);

pub const RADIUS: u8 = 6;
pub const ROW_HEIGHT: f32 = 50.0;

pub fn status_color(status: Status) -> Color32 {
    match status {
        Status::Downloading => ACCENT,
        Status::Seeding => BLUE,
        Status::Checking => AMBER,
        Status::Paused => GREY,
        Status::Done => Color32::from_rgb(0x5f, 0x8f, 0x89),
        Status::Error => RED,
    }
}

/// Inter Medium, for headings and emphasis.
pub fn medium(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("medium".into()))
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "Inter".to_owned(),
        Arc::new(egui::FontData::from_static(include_bytes!(
            "../../assets/fonts/Inter-Regular.ttf"
        ))),
    );
    fonts.font_data.insert(
        "Inter-Medium".to_owned(),
        Arc::new(egui::FontData::from_static(include_bytes!(
            "../../assets/fonts/Inter-Medium.ttf"
        ))),
    );
    let fallback = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let mut proportional = vec!["Inter".to_owned()];
    proportional.extend(fallback.iter().cloned());
    let mut medium = vec!["Inter-Medium".to_owned()];
    medium.extend(fallback);
    fonts
        .families
        .insert(FontFamily::Proportional, proportional);
    fonts
        .families
        .insert(FontFamily::Name("medium".into()), medium);
    ctx.set_fonts(fonts);
}

pub fn apply(ctx: &egui::Context) {
    install_fonts(ctx);
    ctx.set_theme(egui::ThemePreference::Dark);
    let mut style = (*ctx.global_style()).clone();

    style.text_styles = [
        (TextStyle::Heading, medium(17.0)),
        (TextStyle::Body, FontId::new(13.5, FontFamily::Proportional)),
        (
            TextStyle::Button,
            FontId::new(13.5, FontFamily::Proportional),
        ),
        (
            TextStyle::Small,
            FontId::new(11.5, FontFamily::Proportional),
        ),
        (
            TextStyle::Monospace,
            FontId::new(12.5, FontFamily::Monospace),
        ),
    ]
    .into();

    let s = &mut style.spacing;
    s.item_spacing = egui::vec2(8.0, 6.0);
    s.button_padding = egui::vec2(12.0, 6.0);
    s.interact_size.y = 30.0;
    s.window_margin = Margin::same(20);
    s.menu_margin = Margin::same(6);
    s.scroll.bar_width = 8.0;
    s.scroll.floating = true;

    let radius = CornerRadius::same(RADIUS);
    let mut v = Visuals::dark();
    v.override_text_color = None;
    v.panel_fill = BG;
    v.window_fill = SURFACE;
    v.window_stroke = Stroke::new(1.0, BORDER);
    v.window_corner_radius = CornerRadius::same(10);
    v.window_shadow = egui::Shadow {
        offset: [0, 8],
        blur: 28,
        spread: 0,
        color: Color32::from_black_alpha(120),
    };
    v.popup_shadow = egui::Shadow {
        offset: [0, 4],
        blur: 16,
        spread: 0,
        color: Color32::from_black_alpha(100),
    };
    v.menu_corner_radius = radius;
    v.extreme_bg_color = INPUT_BG;
    v.faint_bg_color = SURFACE;
    v.code_bg_color = SURFACE;
    v.hyperlink_color = ACCENT_HOVER;
    v.selection.bg_fill = SELECTION;
    v.selection.stroke = Stroke::new(1.0, ACCENT);
    v.text_cursor.stroke = Stroke::new(2.0, ACCENT);

    let w = &mut v.widgets;
    w.noninteractive.bg_fill = SURFACE;
    w.noninteractive.weak_bg_fill = SURFACE;
    w.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    w.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_DIM);
    w.noninteractive.corner_radius = radius;

    w.inactive.bg_fill = SURFACE;
    w.inactive.weak_bg_fill = SURFACE;
    w.inactive.bg_stroke = Stroke::new(1.0, BORDER);
    w.inactive.fg_stroke = Stroke::new(1.0, TEXT);
    w.inactive.corner_radius = radius;

    w.hovered.bg_fill = SURFACE_HOVER;
    w.hovered.weak_bg_fill = SURFACE_HOVER;
    w.hovered.bg_stroke = Stroke::new(1.0, BORDER_STRONG);
    w.hovered.fg_stroke = Stroke::new(1.0, TEXT);
    w.hovered.corner_radius = radius;
    w.hovered.expansion = 0.0;

    w.active.bg_fill = SURFACE_ACTIVE;
    w.active.weak_bg_fill = SURFACE_ACTIVE;
    w.active.bg_stroke = Stroke::new(1.0, BORDER_STRONG);
    w.active.fg_stroke = Stroke::new(1.0, TEXT);
    w.active.corner_radius = radius;
    w.active.expansion = 0.0;

    w.open = w.active;

    style.visuals = v;
    ctx.set_style_of(egui::Theme::Dark, style);
}
