//! The app icon, bundled in the binary.

use egui::{ColorImage, Context, TextureHandle, TextureOptions};

const LARGE: &[u8] = include_bytes!("../../../assets/icons/icon-256.png");
const SMALL: &[u8] = include_bytes!("../../../assets/icons/icon-32.png");

pub struct Rgba {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// 256 px: the window icon and the About screen.
pub fn large() -> Rgba {
    decode(LARGE)
}

/// 32 px: the tray.
pub fn small() -> Rgba {
    decode(SMALL)
}

/// The large icon as a texture, loaded once per context.
pub fn texture(ctx: &Context) -> TextureHandle {
    let id = egui::Id::unique("app-icon");
    // Not inside `data_mut`: loading a texture takes the same context lock.
    if let Some(texture) = ctx.data(|d| d.get_temp::<TextureHandle>(id)) {
        return texture;
    }
    let icon = large();
    let size = [icon.width as usize, icon.height as usize];
    let image = ColorImage::from_rgba_unmultiplied(size, &icon.rgba);
    let texture = ctx.load_texture("app-icon", image, TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(id, texture.clone()));
    texture
}

fn decode(png: &[u8]) -> Rgba {
    let image = image::load_from_memory(png)
        .expect("the bundled icon is a valid PNG")
        .to_rgba8();
    Rgba {
        width: image.width(),
        height: image.height(),
        rgba: image.into_raw(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bundled_icons_decode_at_their_sizes() {
        assert_eq!((large().width, large().height), (256, 256));
        assert_eq!((small().width, small().height), (32, 32));
    }
}
