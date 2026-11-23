//! The app icon (see `assets/logo/riptide-icon.svg` for the source artwork).

use eframe::egui::{self, IconData};

const ICON_256: &[u8] = include_bytes!("../../assets/logo/icon-256.png");
const ICON_64: &[u8] = include_bytes!("../../assets/logo/icon-64.png");

fn decode(png: &[u8]) -> image::RgbaImage {
    image::load_from_memory_with_format(png, image::ImageFormat::Png)
        .expect("bundled icon is a valid PNG")
        .into_rgba8()
}

/// Window and taskbar icon.
pub fn icon() -> IconData {
    let img = decode(ICON_256);
    IconData {
        width: img.width(),
        height: img.height(),
        rgba: img.into_raw(),
    }
}

/// A small version for the sidebar, uploaded once as a texture.
pub fn texture(ctx: &egui::Context) -> egui::TextureHandle {
    let img = decode(ICON_64);
    let image = egui::ColorImage::from_rgba_unmultiplied(
        [img.width() as usize, img.height() as usize],
        img.as_raw(),
    );
    ctx.load_texture("logo", image, egui::TextureOptions::LINEAR)
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
