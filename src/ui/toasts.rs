//! Short-lived notifications in the bottom-right corner.

use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, RichText, vec2};

use super::theme;

const LIFETIME: Duration = Duration::from_secs(4);
const MAX_VISIBLE: usize = 4;

#[derive(Clone, Copy)]
enum Kind {
    Info,
    Success,
    Error,
}

struct Toast {
    title: String,
    body: String,
    kind: Kind,
    created: Instant,
}

#[derive(Default)]
pub struct Toasts {
    items: Vec<Toast>,
}

impl Toasts {
    pub fn info(&mut self, title: &str, body: &str) {
        self.push(Kind::Info, title, body);
    }

    pub fn success(&mut self, title: &str, body: &str) {
        self.push(Kind::Success, title, body);
    }

    pub fn error(&mut self, title: &str, body: &str) {
        self.push(Kind::Error, title, body);
    }

    fn push(&mut self, kind: Kind, title: &str, body: &str) {
        self.items.push(Toast {
            title: title.to_owned(),
            body: body.to_owned(),
            kind,
            created: Instant::now(),
        });
        if self.items.len() > MAX_VISIBLE {
            self.items.remove(0);
        }
    }

    pub fn show(&mut self, ctx: &egui::Context) {
        // Errors stay twice as long; they usually need reading.
        self.items.retain(|t| {
            let life = if matches!(t.kind, Kind::Error) {
                LIFETIME * 2
            } else {
                LIFETIME
            };
            t.created.elapsed() < life
        });
        if self.items.is_empty() {
            return;
        }
        ctx.request_repaint_after(Duration::from_millis(250));

        egui::Area::new(egui::Id::unique("toasts"))
            .anchor(egui::Align2::RIGHT_BOTTOM, vec2(-16.0, -42.0))
            .order(egui::Order::Tooltip)
            .interactable(false)
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 8.0;
                for t in self.items.iter().rev() {
                    let accent = match t.kind {
                        Kind::Info => theme::TEXT_DIM,
                        Kind::Success => theme::ACCENT,
                        Kind::Error => theme::RED,
                    };
                    egui::Frame::new()
                        .fill(theme::SURFACE)
                        .stroke(egui::Stroke::new(1.0, theme::BORDER))
                        .corner_radius(8)
                        .inner_margin(egui::Margin::symmetric(14, 10))
                        .shadow(egui::Shadow {
                            offset: [0, 4],
                            blur: 14,
                            spread: 0,
                            color: Color32::from_black_alpha(90),
                        })
                        .show(ui, |ui| {
                            ui.set_max_width(340.0);
                            ui.horizontal(|ui| {
                                let (r, _) =
                                    ui.allocate_exact_size(vec2(3.0, 16.0), egui::Sense::hover());
                                ui.painter().rect_filled(r, 1.5, accent);
                                ui.add_space(4.0);
                                ui.vertical(|ui| {
                                    ui.label(RichText::new(&t.title).color(theme::TEXT).strong());
                                    if !t.body.is_empty() {
                                        ui.add(
                                            egui::Label::new(
                                                RichText::new(&t.body)
                                                    .small()
                                                    .color(theme::TEXT_DIM),
                                            )
                                            .wrap(),
                                        );
                                    }
                                });
                            });
                        });
                }
            });
    }
}
