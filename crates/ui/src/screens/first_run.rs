use egui::{Align2, CornerRadius, Frame, Margin, RichText, vec2};

use crate::theme::Theme;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FirstRunEvent {
    pub choose_folder: bool,
    /// Look for the lazer folder again (after installing or moving it).
    pub retry: bool,
}

/// Shown when no lazer data folder was found at the default locations.
pub fn first_run_screen(ui: &mut egui::Ui, theme: &Theme, searched: &[String]) -> FirstRunEvent {
    let mut event = FirstRunEvent::default();
    let rect = ui.available_rect_before_wrap();
    ui.painter().rect_filled(rect, 0.0, theme.colors.base);

    let padding = theme.shape.padding * 2.0;
    let card_width = (rect.width() - theme.shape.padding * 2.0).min(theme.shape.list_width);
    egui::Area::new(egui::Id::unique("first_run"))
        .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
        .show(ui.ctx(), |ui| {
            Frame::new()
                .fill(theme.colors.surface)
                .corner_radius(CornerRadius::same(theme.shape.radius_panel))
                .inner_margin(Margin::same(padding as i8))
                .show(ui, |ui| {
                    ui.set_width(card_width - padding * 2.0);
                    ui.label(theme.heading("osu!lazer was not found"));
                    ui.label(
                        RichText::new(
                            "listenosu! plays the songs in your osu!lazer library. \
                             Choose the folder that contains client.realm and the files folder.",
                        )
                        .color(theme.colors.text_dim),
                    );
                    if !searched.is_empty() {
                        ui.add_space(theme.shape.spacing);
                        ui.label(RichText::new("Looked in:").size(theme.text.small));
                        for path in searched {
                            ui.label(
                                RichText::new(path)
                                    .size(theme.text.small)
                                    .color(theme.colors.text_dim),
                            );
                        }
                    }
                    ui.add_space(theme.shape.padding);
                    ui.horizontal(|ui| {
                        event.choose_folder = ui.button("Choose folder…").clicked();
                        event.retry = ui.button("Try again").clicked();
                    });
                });
        });
    event
}
