use egui::{
    Align, Align2, Color32, CornerRadius, FontId, Frame, Id, Layout, Margin, Order, RichText,
    Sense, vec2,
};

use super::icons::{Icon, icon_button};
use crate::theme::Theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoticeKind {
    Info,
    Warning,
    Error,
}

impl NoticeKind {
    fn color(self, theme: &Theme) -> Color32 {
        match self {
            Self::Info => theme.colors.primary_light,
            Self::Warning => theme.colors.warning,
            Self::Error => theme.colors.error,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BannerEvent {
    pub action: bool,
    pub dismiss: bool,
}

/// Full-width notice with an optional action button,
/// e.g. "lazer's database is not supported yet" or "update available".
pub fn banner(
    ui: &mut egui::Ui,
    theme: &Theme,
    kind: NoticeKind,
    text: &str,
    action_label: Option<&str>,
) -> BannerEvent {
    let mut event = BannerEvent::default();
    let gap = theme.shape.spacing;
    let font = FontId::proportional(theme.text.body);
    let icon_size = theme.text.heading * 1.4;

    Frame::new()
        .fill(theme.colors.surface)
        .corner_radius(CornerRadius::same(theme.shape.radius_card))
        .inner_margin(Margin::same(theme.shape.padding as i8))
        .show(ui, |ui| {
            let width = ui.available_width();
            // Lay the message out first: the row height must come from the text, because a
            // centered layout would otherwise stretch to the full height of a scroll area.
            let action_width = action_label.map_or(0.0, |label| {
                let galley =
                    ui.painter()
                        .layout_no_wrap(label.to_owned(), font.clone(), Color32::WHITE);
                galley.size().x + theme.shape.padding * 2.0 + gap
            });
            let stripe_width = theme.shape.spacing / 2.0;
            let text_width = (width - icon_size - action_width - stripe_width - gap * 2.0).max(0.0);
            let galley = ui
                .painter()
                .layout(text.to_owned(), font, theme.colors.text, text_width);
            let row = vec2(width, galley.size().y.max(icon_size));

            ui.allocate_ui_with_layout(row, Layout::right_to_left(Align::Center), |ui| {
                event.dismiss = icon_button(ui, theme, Icon::Close, icon_size).clicked();
                if let Some(label) = action_label {
                    event.action = ui.button(label).clicked();
                }
                // Whatever the buttons leave over: the accent stripe and the message.
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    stripe(ui, theme, kind, galley.size().y);
                    let (rect, _) = ui.allocate_exact_size(galley.size(), Sense::hover());
                    ui.painter().galley(rect.min, galley, theme.colors.text);
                });
            });
        });
    event
}

fn stripe(ui: &mut egui::Ui, theme: &Theme, kind: NoticeKind, height: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(theme.shape.spacing / 2.0, height), Sense::hover());
    ui.painter().rect_filled(
        rect,
        CornerRadius::same(theme.shape.radius_button),
        kind.color(theme),
    );
}

struct Toast {
    kind: NoticeKind,
    text: String,
    remaining: f32,
}

/// Short-lived messages stacked above the playbar.
#[derive(Default)]
pub struct Toasts {
    items: Vec<Toast>,
}

impl Toasts {
    pub fn push(&mut self, kind: NoticeKind, text: impl Into<String>, seconds: f32) {
        self.items.push(Toast {
            kind,
            text: text.into(),
            remaining: seconds,
        });
    }

    /// Advances the timers and drops expired toasts.
    pub fn tick(&mut self, dt: f32) {
        for toast in &mut self.items {
            toast.remaining -= dt;
        }
        self.items.retain(|toast| toast.remaining > 0.0);
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn show(&mut self, ctx: &egui::Context, theme: &Theme) {
        self.tick(ctx.input(|i| i.stable_dt));
        if self.items.is_empty() {
            return;
        }

        let lift = theme.shape.playbar_height + theme.shape.padding * 2.0;
        egui::Area::new(Id::unique("toasts"))
            .order(Order::Foreground)
            .anchor(Align2::CENTER_BOTTOM, vec2(0.0, -lift))
            .show(ctx, |ui| {
                for toast in &self.items {
                    Frame::new()
                        .fill(theme.colors.surface_selected)
                        .corner_radius(CornerRadius::same(theme.shape.radius_card))
                        .inner_margin(Margin::same(theme.shape.padding as i8))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                stripe(ui, theme, toast.kind, theme.text.body * 1.4);
                                ui.label(RichText::new(&toast.text).color(theme.colors.text));
                            });
                        });
                }
            });
        ctx.request_repaint();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toasts_expire_after_their_duration() {
        let mut toasts = Toasts::default();
        toasts.push(NoticeKind::Info, "short", 1.0);
        toasts.push(NoticeKind::Error, "long", 3.0);
        toasts.tick(0.5);
        assert_eq!(toasts.len(), 2);
        toasts.tick(0.6);
        assert_eq!(toasts.len(), 1);
        toasts.tick(5.0);
        assert!(toasts.is_empty());
    }
}
