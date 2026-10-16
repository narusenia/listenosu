use egui::{CornerRadius, Rect, Sense, vec2};

use crate::theme::Theme;

/// Smooths raw bar levels: bars rise instantly and fall at a fixed rate.
#[derive(Clone, Debug, Default)]
pub struct SpectrumSmoother {
    levels: Vec<f32>,
}

impl SpectrumSmoother {
    pub fn update(&mut self, input: &[f32], dt: f32, decay_per_second: f32) -> &[f32] {
        self.levels.resize(input.len(), 0.0);
        for (level, &new) in self.levels.iter_mut().zip(input) {
            *level = new.max(*level - decay_per_second * dt).clamp(0.0, 1.0);
        }
        &self.levels
    }
}

/// Bar-style spectrum drawn from the bottom of the allocated area.
/// `levels` are 0.0 to 1.0, low frequencies first.
pub fn spectrum(ui: &mut egui::Ui, theme: &Theme, levels: &[f32], height: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    if levels.is_empty() || !ui.is_rect_visible(rect) {
        return;
    }

    let s = &theme.spectrum;
    let count = levels.len() as f32;
    let bar_width = ((rect.width() - s.gap * (count - 1.0)) / count).max(1.0);
    let last = (levels.len() - 1).max(1) as f32;

    for (index, level) in levels.iter().enumerate() {
        let x = rect.left() + index as f32 * (bar_width + s.gap);
        let bar_height = (level.clamp(0.0, 1.0) * rect.height()).max(1.0);
        let bar = Rect::from_min_size(
            egui::pos2(x, rect.bottom() - bar_height),
            vec2(bar_width, bar_height),
        );
        let color = s.color_low.lerp_to_gamma(s.color_high, index as f32 / last);
        ui.painter()
            .rect_filled(bar, CornerRadius::same(s.radius), color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rises_instantly_and_falls_at_the_decay_rate() {
        let mut smoother = SpectrumSmoother::default();
        assert_eq!(smoother.update(&[1.0, 0.5], 0.1, 2.0), [1.0, 0.5]);
        let levels = smoother.update(&[0.0, 0.0], 0.1, 2.0);
        assert!((levels[0] - 0.8).abs() < 1e-6);
        assert!((levels[1] - 0.3).abs() < 1e-6);
    }

    #[test]
    fn never_goes_below_zero() {
        let mut smoother = SpectrumSmoother::default();
        smoother.update(&[0.1], 0.0, 1.0);
        assert_eq!(smoother.update(&[0.0], 10.0, 1.0), [0.0]);
    }
}
