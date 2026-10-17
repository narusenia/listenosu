use crate::i18n::t;
use egui::{Frame, Margin, RichText};

use super::header;
use crate::fonts;
use crate::theme::Theme;

/// Returns true when Back was clicked.
pub fn about_screen(
    ui: &mut egui::Ui,
    theme: &Theme,
    version: &str,
    icon: &egui::TextureHandle,
) -> bool {
    let mut back = false;
    Frame::NONE
        .inner_margin(Margin::same(theme.shape.padding as i8))
        .show(ui, |ui| {
            back = header(ui, theme, t("about.title"));
            ui.horizontal(|ui| {
                ui.add(
                    egui::Image::new(icon).fit_to_exact_size(egui::Vec2::splat(theme.icon_size())),
                );
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("listenosu!")
                            .font(fonts::bold(theme.text.title))
                            .color(theme.colors.text),
                    );
                    ui.label(
                        RichText::new(crate::i18n::tr("about.version", &[("version", version)]))
                            .color(theme.colors.text_dim),
                    );
                });
            });
            ui.add_space(theme.shape.padding);

            // The project rules require saying it is unofficial wherever the name shows.
            ui.label(RichText::new(t("about.unofficial")).color(theme.colors.accent));
            ui.label(
                "A music player for the songs in your osu!lazer library. \
                 It only reads the library and never writes to it.",
            );
            ui.label(t("about.no_logo"));

            ui.add_space(theme.shape.padding);
            ui.label(theme.heading(t("about.licenses")));
            ui.label(t("about.license_app"));
            ui.label(t("about.license_nunito"));
            ui.label(t("about.license_noto"));
            ui.label(t("about.license_lucide"));
            ui.label(t("about.license_palettes"));
        });
    back
}
