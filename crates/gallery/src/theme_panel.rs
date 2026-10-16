use eframe::egui;
use ui::Theme;

/// Live theme editor; `status` carries the message of the last action.
pub fn show(ui: &mut egui::Ui, theme: &mut Theme, status: &mut String) {
    ui.heading("Theme");
    ui.horizontal(|ui| {
        if ui.button("Reset").clicked() {
            *theme = Theme::lazer();
        }
        if ui.button("Copy TOML").clicked() {
            match theme.to_toml() {
                Ok(text) => {
                    ui.ctx().copy_text(text);
                    *status = "Copied theme as TOML".to_owned();
                }
                Err(err) => *status = format!("Failed: {err}"),
            }
        }
    });
    if !status.is_empty() {
        ui.label(&*status);
    }
    ui.separator();

    egui::ScrollArea::vertical().show(ui, |ui| {
        let colors = &mut theme.colors;
        ui.strong("Colors");
        for (label, color) in [
            ("base", &mut colors.base),
            ("chrome", &mut colors.chrome),
            ("surface", &mut colors.surface),
            ("surface_hover", &mut colors.surface_hover),
            ("surface_selected", &mut colors.surface_selected),
            ("primary", &mut colors.primary),
            ("primary_light", &mut colors.primary_light),
            ("accent", &mut colors.accent),
            ("text", &mut colors.text),
            ("text_dim", &mut colors.text_dim),
            ("text_disabled", &mut colors.text_disabled),
            ("border", &mut colors.border),
            ("warning", &mut colors.warning),
            ("error", &mut colors.error),
        ] {
            ui.horizontal(|ui| {
                ui.color_edit_button_srgba(color);
                ui.label(label);
            });
        }

        ui.separator();
        ui.strong("Shape");
        let shape = &mut theme.shape;
        ui.add(egui::Slider::new(&mut shape.radius_card, 0..=24).text("radius_card"));
        ui.add(egui::Slider::new(&mut shape.radius_button, 0..=24).text("radius_button"));
        ui.add(egui::Slider::new(&mut shape.radius_panel, 0..=32).text("radius_panel"));
        ui.add(egui::Slider::new(&mut shape.spacing, 0.0..=24.0).text("spacing"));
        ui.add(egui::Slider::new(&mut shape.padding, 0.0..=32.0).text("padding"));
        ui.add(egui::Slider::new(&mut shape.card_height, 32.0..=96.0).text("card_height"));
        ui.add(egui::Slider::new(&mut shape.card_gap, 0.0..=24.0).text("card_gap"));
        ui.add(egui::Slider::new(&mut shape.playbar_height, 48.0..=120.0).text("playbar_height"));
        ui.add(egui::Slider::new(&mut shape.list_width, 280.0..=640.0).text("list_width"));

        ui.separator();
        ui.strong("Text");
        let text = &mut theme.text;
        ui.add(egui::Slider::new(&mut text.title, 16.0..=48.0).text("title"));
        ui.add(egui::Slider::new(&mut text.heading, 12.0..=36.0).text("heading"));
        ui.add(egui::Slider::new(&mut text.body, 10.0..=24.0).text("body"));
        ui.add(egui::Slider::new(&mut text.small, 8.0..=18.0).text("small"));

        ui.separator();
        ui.strong("Backdrop");
        let backdrop = &mut theme.backdrop;
        ui.add(egui::Slider::new(&mut backdrop.blur, 1.0..=64.0).text("blur"));
        ui.add(egui::Slider::new(&mut backdrop.dim, 0.0..=1.0).text("dim"));
        ui.add(egui::Slider::new(&mut backdrop.scrim, 0.0..=1.0).text("scrim"));

        ui.separator();
        ui.strong("Motion");
        let motion = &mut theme.motion;
        ui.add(egui::Slider::new(&mut motion.hover_seconds, 0.0..=0.6).text("hover_seconds"));
        ui.add(
            egui::Slider::new(&mut motion.backdrop_crossfade_seconds, 0.0..=2.0)
                .text("backdrop_crossfade_seconds"),
        );

        ui.separator();
        ui.strong("Spectrum");
        let spectrum = &mut theme.spectrum;
        ui.add(egui::Slider::new(&mut spectrum.bars, 8..=128).text("bars"));
        ui.add(egui::Slider::new(&mut spectrum.gap, 0.0..=8.0).text("gap"));
        ui.add(egui::Slider::new(&mut spectrum.height, 24.0..=160.0).text("height"));
        ui.add(egui::Slider::new(&mut spectrum.mini_height, 8.0..=48.0).text("mini_height"));
        ui.add(egui::Slider::new(&mut spectrum.radius, 0..=8).text("radius"));
        ui.add(
            egui::Slider::new(&mut spectrum.decay_per_second, 0.2..=6.0).text("decay_per_second"),
        );
        for (label, color) in [
            ("color_low", &mut spectrum.color_low),
            ("color_high", &mut spectrum.color_high),
        ] {
            ui.horizontal(|ui| {
                ui.color_edit_button_srgba(color);
                ui.label(label);
            });
        }
    });
}
