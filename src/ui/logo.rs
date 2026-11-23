//! The app icon, rendered in code so there is no binary asset to keep in sync:
//! a teal tile with three waves, the "riptide".

use eframe::egui::IconData;

const SIZE: u32 = 256;
const TEAL: [f32; 3] = [0x38 as f32, 0xb2 as f32, 0xa6 as f32];
const INK: [f32; 3] = [0x0b as f32, 0x1f as f32, 0x1d as f32];

/// Signed distance to a rounded square centered in the canvas (negative inside).
fn tile_distance(x: f32, y: f32) -> f32 {
    let half = SIZE as f32 / 2.0;
    let radius = SIZE as f32 * 0.22;
    let inner = half - radius - 4.0;
    let dx = ((x - half).abs() - inner).max(0.0);
    let dy = ((y - half).abs() - inner).max(0.0);
    (dx * dx + dy * dy).sqrt() - radius
}

/// Distance to the nearest of three sine waves, each slightly stronger than the last.
fn wave_distance(x: f32, y: f32) -> f32 {
    let s = SIZE as f32;
    let mut best = f32::MAX;
    for (i, row) in [0.36, 0.52, 0.68].into_iter().enumerate() {
        let amplitude = s * (0.035 + 0.012 * i as f32);
        let phase = i as f32 * 0.9;
        let wave_y = s * row + amplitude * (x / s * std::f32::consts::TAU * 1.5 + phase).sin();
        best = best.min((y - wave_y).abs());
    }
    best
}

pub fn icon() -> IconData {
    let mut rgba = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    let stroke = SIZE as f32 * 0.032;
    let margin = SIZE as f32 * 0.18;
    for py in 0..SIZE {
        for px in 0..SIZE {
            let (x, y) = (px as f32 + 0.5, py as f32 + 0.5);
            let tile = (0.5 - tile_distance(x, y)).clamp(0.0, 1.0);
            let inside_x = x > margin && x < SIZE as f32 - margin;
            let wave = if inside_x {
                (stroke - wave_distance(x, y) + 0.5).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let mix = |c: usize| TEAL[c] + (INK[c] - TEAL[c]) * wave;
            rgba.extend([
                mix(0) as u8,
                mix(1) as u8,
                mix(2) as u8,
                (tile * 255.0) as u8,
            ]);
        }
    }
    IconData {
        rgba,
        width: SIZE,
        height: SIZE,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn icon_has_transparent_corners_and_opaque_center() {
        let icon = super::icon();
        let px = |x: u32, y: u32| icon.rgba[((y * icon.width + x) * 4 + 3) as usize];
        assert_eq!(px(0, 0), 0);
        assert_eq!(px(icon.width / 2, icon.height / 2), 255);
    }
}
