use egui::{Frame, Margin, RichText};

use super::header;
use crate::fonts;
use crate::theme::Theme;

/// Returns true when Back was clicked.
pub fn about_screen(ui: &mut egui::Ui, theme: &Theme, version: &str) -> bool {
    let mut back = false;
    Frame::NONE
        .inner_margin(Margin::same(theme.shape.padding as i8))
        .show(ui, |ui| {
            back = header(ui, theme, "About");
            ui.label(
                RichText::new("listenosu")
                    .font(fonts::bold(theme.text.title))
                    .color(theme.colors.text),
            );
            ui.label(RichText::new(format!("Version {version}")).color(theme.colors.text_dim));
            ui.add_space(theme.shape.padding);

            // The project rules require saying it is unofficial wherever the name shows.
            ui.label(
                RichText::new("Unofficial. Not affiliated with or endorsed by ppy or osu!.")
                    .color(theme.colors.accent),
            );
            ui.label(
                "A music player for the songs in your osu!lazer library. \
                 It only reads the library and never writes to it.",
            );
            ui.label("The osu! logo and the Torus font are not included.");

            ui.add_space(theme.shape.padding);
            ui.label(theme.heading("Licenses"));
            ui.label("listenosu: MIT License");
            ui.label("Nunito: SIL Open Font License 1.1");
            ui.label("Noto Sans JP: SIL Open Font License 1.1");
        });
    back
}
