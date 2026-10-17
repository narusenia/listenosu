use egui::{Color32, ColorImage, Mesh, Rect, TextureHandle, TextureOptions, pos2};

use crate::theme::Theme;

/// Blurred, dimmed full-window background taken from the playing song's image.
/// The blur is a downscale followed by three box-blur passes (close to a Gaussian); the GPU's
/// linear filtering smooths the small result when it is drawn large.
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
        let factor = (blur / BLUR_RADIUS as f32).max(1.0) as usize;
        let (mut pixels, w, h) = downscale(rgba, width, height, factor);
        blur_rgba(&mut pixels, w, h);
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
            painter.rect_filled(rect, 0.0, tint(theme, theme.backdrop.dim));
            let edge = tint(theme, theme.backdrop.scrim);
            let reach = rect.height() * SCRIM_REACH;
            let (top, bottom) = (
                rect.with_max_y(rect.top() + reach),
                rect.with_min_y(rect.bottom() - reach),
            );
            painter.add(vertical_gradient(top, edge, Color32::TRANSPARENT));
            painter.add(vertical_gradient(bottom, Color32::TRANSPARENT, edge));
        }
    }
}

/// Box-blur radius in downscaled pixels; the downscale factor follows from the theme's blur.
const BLUR_RADIUS: usize = 4;
/// Share of the window height the top and bottom scrims fade over.
const SCRIM_REACH: f32 = 0.4;

fn vertical_gradient(rect: Rect, top: Color32, bottom: Color32) -> Mesh {
    let mut mesh = Mesh::default();
    mesh.colored_vertex(rect.left_top(), top);
    mesh.colored_vertex(rect.right_top(), top);
    mesh.colored_vertex(rect.right_bottom(), bottom);
    mesh.colored_vertex(rect.left_bottom(), bottom);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    mesh
}

/// Three box-blur passes in each direction; edges repeat the border pixels.
fn blur_rgba(pixels: &mut [u8], width: usize, height: usize) {
    for _ in 0..3 {
        blur_pass(pixels, width, height, true);
        blur_pass(pixels, width, height, false);
    }
}

fn blur_pass(pixels: &mut [u8], width: usize, height: usize, horizontal: bool) {
    let (len, lines, line_stride, step) = if horizontal {
        (width, height, width * 4, 4)
    } else {
        (height, width, 4, width * 4)
    };
    let mut out = vec![0u8; len * 4];
    for line in 0..lines {
        let base = line * line_stride;
        for i in 0..len {
            let (lo, hi) = (
                i.saturating_sub(BLUR_RADIUS),
                (i + BLUR_RADIUS).min(len - 1),
            );
            for channel in 0..4 {
                let sum: usize = (lo..=hi)
                    .map(|j| pixels[base + j * step + channel] as usize)
                    .sum();
                out[i * 4 + channel] = (sum / (hi - lo + 1)) as u8;
            }
        }
        for (i, px) in out.chunks(4).enumerate() {
            pixels[base + i * step..base + i * step + 4].copy_from_slice(px);
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

/// The backdrop's tint color at `strength` (0.0 to 1.0) opacity.
fn tint(theme: &Theme, strength: f32) -> Color32 {
    theme.backdrop.tint.gamma_multiply(strength)
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
    fn blur_keeps_flat_color_and_spreads_a_bright_pixel() {
        let mut flat = [10u8, 20, 30, 255].repeat(9 * 9);
        blur_rgba(&mut flat, 9, 9);
        assert!(flat.chunks(4).all(|px| px == [10, 20, 30, 255]));

        let mut dot = [0u8, 0, 0, 255].repeat(9 * 9);
        dot[(4 * 9 + 4) * 4] = 255;
        blur_rgba(&mut dot, 9, 9);
        assert!(dot[(4 * 9 + 1) * 4] > 0, "neighbours pick up some light");
        assert!(dot[(4 * 9 + 4) * 4] < 255);
    }

    #[test]
    fn cover_uv_crops_the_longer_side() {
        let wide = cover_uv(2.0, 1.0);
        assert_eq!((wide.min.x, wide.max.x), (0.25, 0.75));
        let tall = cover_uv(0.5, 1.0);
        assert_eq!((tall.min.y, tall.max.y), (0.25, 0.75));
    }
}
