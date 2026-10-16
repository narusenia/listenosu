use egui::{Color32, ColorImage, Rect, TextureHandle, TextureOptions, pos2};

use crate::theme::Theme;

/// Blurred, dimmed full-window background taken from the playing song's image.
/// The blur is a box downscale; the GPU's linear filtering smooths it when drawn large.
#[derive(Default)]
pub struct Backdrop {
    current: Option<TextureHandle>,
    previous: Option<TextureHandle>,
    /// Crossfade progress of `current` over `previous`, 0.0 to 1.0.
    fade: f32,
}

impl Backdrop {
    /// `rgba` is unmultiplied RGBA8, `width * height * 4` bytes.
    pub fn set_image(
        &mut self,
        ctx: &egui::Context,
        rgba: &[u8],
        width: usize,
        height: usize,
        blur: f32,
    ) {
        let (pixels, w, h) = downscale(rgba, width, height, blur.max(1.0) as usize);
        let image = ColorImage::from_rgba_unmultiplied([w, h], &pixels);
        self.previous = self.current.take();
        self.current = Some(ctx.load_texture("backdrop", image, TextureOptions::LINEAR));
        self.fade = 0.0;
    }

    pub fn clear(&mut self) {
        self.current = None;
        self.previous = None;
    }

    pub fn paint(&mut self, ui: &egui::Ui, rect: Rect, theme: &Theme) {
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, theme.colors.base);

        let seconds = theme.motion.backdrop_crossfade_seconds;
        if self.fade < 1.0 {
            self.fade = if seconds > 0.0 {
                (self.fade + ui.input(|i| i.stable_dt) / seconds).min(1.0)
            } else {
                1.0
            };
            ui.ctx().request_repaint();
        }

        for (texture, alpha) in [(&self.previous, 1.0), (&self.current, self.fade)] {
            if let Some(texture) = texture {
                let size = texture.size_vec2();
                painter.image(
                    texture.id(),
                    rect,
                    cover_uv(size.x / size.y, rect.aspect_ratio()),
                    Color32::WHITE.gamma_multiply(alpha),
                );
            }
        }
        if self.current.is_some() {
            painter.rect_filled(
                rect,
                0.0,
                Color32::from_black_alpha((theme.backdrop.dim * 255.0) as u8),
            );
        }
    }
}

/// UV rectangle that makes an image of `image_aspect` cover an area of `area_aspect`.
fn cover_uv(image_aspect: f32, area_aspect: f32) -> Rect {
    if image_aspect > area_aspect {
        let visible = area_aspect / image_aspect;
        Rect::from_min_max(
            pos2((1.0 - visible) / 2.0, 0.0),
            pos2((1.0 + visible) / 2.0, 1.0),
        )
    } else {
        let visible = image_aspect / area_aspect;
        Rect::from_min_max(
            pos2(0.0, (1.0 - visible) / 2.0),
            pos2(1.0, (1.0 + visible) / 2.0),
        )
    }
}

/// Averages `factor` x `factor` blocks of an RGBA8 image. Edge blocks may be smaller.
fn downscale(rgba: &[u8], width: usize, height: usize, factor: usize) -> (Vec<u8>, usize, usize) {
    let factor = factor.max(1);
    let (w, h) = (width.div_ceil(factor), height.div_ceil(factor));
    let mut out = Vec::with_capacity(w * h * 4);
    for by in 0..h {
        for bx in 0..w {
            let mut sum = [0u32; 4];
            let mut count = 0;
            for y in by * factor..((by + 1) * factor).min(height) {
                for x in bx * factor..((bx + 1) * factor).min(width) {
                    let i = (y * width + x) * 4;
                    for (channel, total) in sum.iter_mut().enumerate() {
                        *total += rgba[i + channel] as u32;
                    }
                    count += 1;
                }
            }
            out.extend(sum.iter().map(|total| (total / count) as u8));
        }
    }
    (out, w, h)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downscale_keeps_flat_color_and_rounds_size_up() {
        let rgba = [10u8, 20, 30, 255].repeat(5 * 3);
        let (out, w, h) = downscale(&rgba, 5, 3, 2);
        assert_eq!((w, h), (3, 2));
        assert!(out.chunks(4).all(|px| px == [10, 20, 30, 255]));
    }

    #[test]
    fn downscale_averages_a_block() {
        let rgba = [0, 0, 0, 255, 100, 100, 100, 255];
        let (out, w, h) = downscale(&rgba, 2, 1, 2);
        assert_eq!((w, h), (1, 1));
        assert_eq!(out, [50, 50, 50, 255]);
    }

    #[test]
    fn cover_uv_crops_the_longer_side() {
        let wide = cover_uv(2.0, 1.0);
        assert_eq!((wide.min.x, wide.max.x), (0.25, 0.75));
        let tall = cover_uv(0.5, 1.0);
        assert_eq!((tall.min.y, tall.max.y), (0.25, 0.75));
    }
}
