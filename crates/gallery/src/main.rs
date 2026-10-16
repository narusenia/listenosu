use eframe::egui;
use ui::Theme;

struct Gallery {
    theme: Theme,
    applied: Option<Theme>,
    status: String,
}

impl Gallery {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        ui::fonts::install(&cc.egui_ctx);
        Self {
            theme: Theme::lazer(),
            applied: None,
            status: String::new(),
        }
    }

    fn sync_theme(&mut self, ctx: &egui::Context) {
        if self.applied.as_ref() != Some(&self.theme) {
            self.theme.apply(ctx);
            self.applied = Some(self.theme.clone());
        }
    }

    fn theme_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Theme");
        ui.horizontal(|ui| {
            if ui.button("Reset").clicked() {
                self.theme = Theme::lazer();
            }
            if ui.button("Copy TOML").clicked() {
                match self.theme.to_toml() {
                    Ok(text) => {
                        ui.ctx().copy_text(text);
                        self.status = "Copied theme as TOML".to_owned();
                    }
                    Err(err) => self.status = format!("Failed: {err}"),
                }
            }
        });
        if !self.status.is_empty() {
            ui.label(&self.status);
        }
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            let colors = &mut self.theme.colors;
            ui.strong("Colors");
            for (label, color) in [
                ("base", &mut colors.base),
                ("surface", &mut colors.surface),
                ("surface_hover", &mut colors.surface_hover),
                ("surface_selected", &mut colors.surface_selected),
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
            let shape = &mut self.theme.shape;
            ui.add(egui::Slider::new(&mut shape.radius_card, 0..=24).text("radius_card"));
            ui.add(egui::Slider::new(&mut shape.radius_button, 0..=24).text("radius_button"));
            ui.add(egui::Slider::new(&mut shape.radius_panel, 0..=32).text("radius_panel"));
            ui.add(egui::Slider::new(&mut shape.spacing, 0.0..=24.0).text("spacing"));
            ui.add(egui::Slider::new(&mut shape.padding, 0.0..=32.0).text("padding"));
            ui.add(egui::Slider::new(&mut shape.card_height, 32.0..=96.0).text("card_height"));
            ui.add(egui::Slider::new(&mut shape.card_gap, 0.0..=24.0).text("card_gap"));
            ui.add(
                egui::Slider::new(&mut shape.playbar_height, 48.0..=120.0).text("playbar_height"),
            );
            ui.add(egui::Slider::new(&mut shape.list_width, 280.0..=640.0).text("list_width"));

            ui.separator();
            ui.strong("Text");
            let text = &mut self.theme.text;
            ui.add(egui::Slider::new(&mut text.title, 16.0..=48.0).text("title"));
            ui.add(egui::Slider::new(&mut text.heading, 12.0..=36.0).text("heading"));
            ui.add(egui::Slider::new(&mut text.body, 10.0..=24.0).text("body"));
            ui.add(egui::Slider::new(&mut text.small, 8.0..=18.0).text("small"));

            ui.separator();
            ui.strong("Backdrop");
            let backdrop = &mut self.theme.backdrop;
            ui.add(egui::Slider::new(&mut backdrop.blur, 1.0..=64.0).text("blur"));
            ui.add(egui::Slider::new(&mut backdrop.dim, 0.0..=1.0).text("dim"));

            ui.separator();
            ui.strong("Motion");
            let motion = &mut self.theme.motion;
            ui.add(egui::Slider::new(&mut motion.hover_seconds, 0.0..=0.6).text("hover_seconds"));
            ui.add(
                egui::Slider::new(&mut motion.backdrop_crossfade_seconds, 0.0..=2.0)
                    .text("backdrop_crossfade_seconds"),
            );

            ui.separator();
            ui.strong("Spectrum");
            let spectrum = &mut self.theme.spectrum;
            ui.add(egui::Slider::new(&mut spectrum.bars, 8..=128).text("bars"));
            ui.add(egui::Slider::new(&mut spectrum.gap, 0.0..=8.0).text("gap"));
            ui.add(egui::Slider::new(&mut spectrum.height, 24.0..=160.0).text("height"));
            ui.add(egui::Slider::new(&mut spectrum.mini_height, 8.0..=48.0).text("mini_height"));
            ui.add(egui::Slider::new(&mut spectrum.radius, 0..=8).text("radius"));
            ui.add(
                egui::Slider::new(&mut spectrum.decay_per_second, 0.2..=6.0)
                    .text("decay_per_second"),
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

    fn preview(&self, ui: &mut egui::Ui) {
        ui.heading("listenosu gallery");
        ui.label("Regular: The quick brown fox jumps over the lazy dog. 0123456789");
        ui.label("日本語の表示確認：作業用BGMを流し聴きする。ひらがな・カタカナ・漢字。");
        ui.label(
            egui::RichText::new("Bold: The quick brown fox 0123456789")
                .font(ui::fonts::bold(self.theme.text.body)),
        );
        ui.label(
            egui::RichText::new("太字の表示確認：作業用BGM")
                .font(ui::fonts::bold(self.theme.text.body)),
        );
        ui.label(
            egui::RichText::new("Song Title 曲名のサンプル")
                .font(ui::fonts::bold(self.theme.text.title))
                .color(self.theme.colors.text),
        );
        ui.label(
            egui::RichText::new("Artist アーティスト名")
                .size(self.theme.text.heading)
                .color(self.theme.colors.text_dim),
        );
        ui.small("Small: 180 BPM · 3:45");
    }
}

impl eframe::App for Gallery {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.sync_theme(ui.ctx());

        egui::Panel::left("theme_panel")
            .default_size(300.0)
            .show(ui, |ui| self.theme_panel(ui));
        egui::CentralPanel::default().show(ui, |ui| self.preview(ui));
    }
}

fn main() -> eframe::Result {
    eframe::run_native(
        "listenosu gallery",
        eframe::NativeOptions::default(),
        Box::new(|cc| Ok(Box::new(Gallery::new(cc)))),
    )
}
