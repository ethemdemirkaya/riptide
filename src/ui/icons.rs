//! Small line icons drawn with the painter, so they stay crisp at any scale
//! and match the text color without bundling an icon font.

use eframe::egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Plus,
    Play,
    Pause,
    Trash,
    Folder,
    Settings,
    Search,
    ArrowDown,
    ArrowUp,
    File,
    Close,
    Copy,
    Link,
}

impl Icon {
    /// Paints the icon centered in `rect`. Icons are designed on a 16×16 grid.
    pub fn paint(self, painter: &Painter, rect: Rect, color: Color32) {
        let size = rect.width().min(rect.height());
        let s = size / 16.0;
        let o = rect.center() - vec2(8.0, 8.0) * s;
        let p = |x: f32, y: f32| -> Pos2 { o + vec2(x, y) * s };
        let stroke = Stroke::new(1.5 * s.max(0.9), color);
        let line = |a: Pos2, b: Pos2| painter.line_segment([a, b], stroke);
        let path = |pts: Vec<Pos2>, closed: bool| {
            let shape = if closed {
                Shape::closed_line(pts, stroke)
            } else {
                Shape::line(pts, stroke)
            };
            painter.add(shape);
        };

        match self {
            Icon::Plus => {
                line(p(8.0, 3.0), p(8.0, 13.0));
                line(p(3.0, 8.0), p(13.0, 8.0));
            }
            Icon::Play => {
                painter.add(Shape::convex_polygon(
                    vec![p(5.0, 3.0), p(13.0, 8.0), p(5.0, 13.0)],
                    color,
                    Stroke::NONE,
                ));
            }
            Icon::Pause => {
                for x in [4.5, 9.5] {
                    painter.rect_filled(
                        Rect::from_min_size(p(x, 3.5), vec2(2.5, 9.0) * s),
                        1.0,
                        color,
                    );
                }
            }
            Icon::Trash => {
                line(p(2.5, 4.5), p(13.5, 4.5));
                path(
                    vec![p(6.0, 4.5), p(6.0, 2.5), p(10.0, 2.5), p(10.0, 4.5)],
                    false,
                );
                path(
                    vec![p(4.0, 4.5), p(4.8, 13.5), p(11.2, 13.5), p(12.0, 4.5)],
                    false,
                );
                line(p(6.8, 7.0), p(7.0, 11.0));
                line(p(9.2, 7.0), p(9.0, 11.0));
            }
            Icon::Folder => {
                path(
                    vec![
                        p(2.0, 4.0),
                        p(6.0, 4.0),
                        p(7.5, 5.5),
                        p(14.0, 5.5),
                        p(14.0, 12.5),
                        p(2.0, 12.5),
                    ],
                    true,
                );
            }
            Icon::Settings => {
                // Three sliders: reads as "preferences" at small sizes better than a gear.
                for (y, knob) in [(4.0, 10.5), (8.0, 5.5), (12.0, 9.0)] {
                    line(p(2.5, y), p(13.5, y));
                    painter.circle(p(knob, y), 1.8 * s, Color32::TRANSPARENT, stroke);
                    painter.circle_filled(p(knob, y), 1.2 * s, color);
                }
            }
            Icon::Search => {
                painter.circle_stroke(p(7.0, 7.0), 4.2 * s, stroke);
                line(p(10.2, 10.2), p(13.5, 13.5));
            }
            Icon::ArrowDown => {
                line(p(8.0, 2.5), p(8.0, 13.0));
                path(vec![p(4.0, 9.0), p(8.0, 13.0), p(12.0, 9.0)], false);
            }
            Icon::ArrowUp => {
                line(p(8.0, 3.0), p(8.0, 13.5));
                path(vec![p(4.0, 7.0), p(8.0, 3.0), p(12.0, 7.0)], false);
            }
            Icon::File => {
                path(
                    vec![
                        p(3.5, 2.0),
                        p(9.5, 2.0),
                        p(12.5, 5.0),
                        p(12.5, 14.0),
                        p(3.5, 14.0),
                    ],
                    true,
                );
                path(vec![p(9.5, 2.0), p(9.5, 5.0), p(12.5, 5.0)], false);
            }
            Icon::Close => {
                line(p(4.0, 4.0), p(12.0, 12.0));
                line(p(12.0, 4.0), p(4.0, 12.0));
            }
            Icon::Copy => {
                painter.rect_stroke(
                    Rect::from_min_max(p(5.5, 5.5), p(13.5, 13.5)),
                    1.5 * s,
                    stroke,
                    eframe::egui::StrokeKind::Middle,
                );
                path(
                    vec![
                        p(3.0, 10.5),
                        p(2.5, 10.5),
                        p(2.5, 2.5),
                        p(10.5, 2.5),
                        p(10.5, 3.0),
                    ],
                    false,
                );
            }
            Icon::Link => {
                // A horseshoe magnet.
                let mut pts = Vec::new();
                for i in 0..=12 {
                    let a = std::f32::consts::PI * (i as f32 / 12.0);
                    pts.push(p(8.0 - 4.5 * a.cos(), 8.5 + 4.5 * a.sin()));
                }
                let mut shape = vec![p(3.5, 2.5)];
                shape.extend(pts);
                shape.push(p(12.5, 2.5));
                path(shape, false);
                line(p(2.5, 5.5), p(5.0, 5.5));
                line(p(11.0, 5.5), p(13.5, 5.5));
            }
        }
    }
}

/// Convenience for laying out an icon at a fixed size inside a larger rect.
pub fn icon_rect(center: Pos2, size: f32) -> Rect {
    Rect::from_center_size(center, vec2(size, size))
}
