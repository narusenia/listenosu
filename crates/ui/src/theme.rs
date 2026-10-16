use egui::{Color32, CornerRadius, FontFamily, FontId, Stroke, TextStyle, vec2};
use serde::{Deserialize, Serialize};

/// Every value that affects the look of the UI lives here.
/// UI code must not hardcode colors or sizes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Theme {
    pub name: String,
    pub colors: Colors,
    pub shape: Shape,
    pub text: TextSizes,
    pub backdrop: Backdrop,
    pub motion: Motion,
    pub spectrum: Spectrum,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Colors {
    /// Window background.
    pub base: Color32,
    /// Top and bottom bars, darker than the window background.
    pub chrome: Color32,
    pub surface: Color32,
    pub surface_hover: Color32,
    pub surface_selected: Color32,
    /// Buttons and other primary controls.
    pub primary: Color32,
    /// Lighter primary for switches and selection indicators.
    pub primary_light: Color32,
    /// Highlights such as the playing song.
    pub accent: Color32,
    pub text: Color32,
    pub text_dim: Color32,
    pub text_disabled: Color32,
    pub border: Color32,
    pub warning: Color32,
    pub error: Color32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Shape {
    pub radius_card: u8,
    pub radius_button: u8,
    pub radius_panel: u8,
    pub spacing: f32,
    pub padding: f32,
    pub card_height: f32,
    pub card_gap: f32,
    pub playbar_height: f32,
    pub list_width: f32,
    pub mini_size: [f32; 2],
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TextSizes {
    pub title: f32,
    pub heading: f32,
    pub body: f32,
    pub small: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Backdrop {
    /// Blur strength of the backdrop image (the inverse of the downscale factor; larger is blurrier).
    pub blur: f32,
    /// How much the backdrop image is darkened (0.0 to 1.0).
    pub dim: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Motion {
    pub hover_seconds: f32,
    pub backdrop_crossfade_seconds: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Spectrum {
    pub bars: usize,
    pub gap: f32,
    pub height: f32,
    pub mini_height: f32,
    pub radius: u8,
    /// How far the bars fall per second, as a fraction of the 0.0 to 1.0 amplitude range.
    pub decay_per_second: f32,
    pub color_low: Color32,
    pub color_high: Color32,
}

impl Default for Theme {
    fn default() -> Self {
        Self::lazer()
    }
}

impl Theme {
    /// Built-in theme styled after osu!lazer (neutral dark background, purple controls, pink highlights).
    pub fn lazer() -> Self {
        Self {
            name: "lazer".to_owned(),
            // Sampled from osu!lazer screenshots: a neutral dark with a slight purple tint.
            colors: Colors {
                base: Color32::from_rgb(0x24, 0x22, 0x2a),
                chrome: Color32::from_rgb(0x19, 0x19, 0x19),
                surface: Color32::from_rgb(0x2c, 0x2a, 0x33),
                surface_hover: Color32::from_rgb(0x38, 0x35, 0x41),
                surface_selected: Color32::from_rgb(0x45, 0x3d, 0x5c),
                primary: Color32::from_rgb(0x59, 0x33, 0xcc),
                primary_light: Color32::from_rgb(0x8c, 0x66, 0xff),
                accent: Color32::from_rgb(0xff, 0x66, 0xaa),
                text: Color32::from_rgb(0xff, 0xff, 0xff),
                text_dim: Color32::from_rgb(0xb5, 0xb0, 0xc0),
                text_disabled: Color32::from_rgb(0x6b, 0x67, 0x75),
                border: Color32::from_rgb(0x3a, 0x37, 0x43),
                warning: Color32::from_rgb(0xff, 0xcc, 0x22),
                error: Color32::from_rgb(0xff, 0x55, 0x66),
            },
            shape: Shape {
                radius_card: 10,
                radius_button: 8,
                radius_panel: 14,
                spacing: 8.0,
                padding: 12.0,
                card_height: 56.0,
                card_gap: 6.0,
                playbar_height: 72.0,
                list_width: 440.0,
                mini_size: [360.0, 120.0],
            },
            text: TextSizes {
                title: 28.0,
                heading: 20.0,
                body: 15.0,
                small: 12.0,
            },
            backdrop: Backdrop {
                blur: 24.0,
                dim: 0.65,
            },
            motion: Motion {
                hover_seconds: 0.12,
                backdrop_crossfade_seconds: 0.6,
            },
            spectrum: Spectrum {
                bars: 64,
                gap: 3.0,
                height: 72.0,
                mini_height: 24.0,
                radius: 2,
                decay_per_second: 1.6,
                color_low: Color32::from_rgba_unmultiplied(0xff, 0x66, 0xaa, 0x90),
                color_high: Color32::from_rgba_unmultiplied(0x8c, 0x66, 0xff, 0x90),
            },
        }
    }

    pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }

    pub fn from_toml(source: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(source)
    }

    pub fn apply(&self, ctx: &egui::Context) {
        ctx.set_theme(egui::Theme::Dark);
        ctx.all_styles_mut(|style| self.apply_to_style(style));
    }

    fn apply_to_style(&self, style: &mut egui::Style) {
        let c = &self.colors;
        let s = &self.shape;

        let visuals = &mut style.visuals;
        *visuals = egui::Visuals::dark();
        visuals.panel_fill = c.base;
        visuals.window_fill = c.surface;
        visuals.extreme_bg_color = c.base;
        visuals.faint_bg_color = c.surface;
        visuals.window_stroke = Stroke::new(1.0, c.border);
        visuals.window_corner_radius = CornerRadius::same(s.radius_panel);
        visuals.menu_corner_radius = CornerRadius::same(s.radius_button);
        visuals.hyperlink_color = c.primary_light;
        visuals.warn_fg_color = c.warning;
        visuals.error_fg_color = c.error;
        visuals.selection.bg_fill = c.primary;
        // egui also uses this stroke color for the text of selected items.
        visuals.selection.stroke = Stroke::new(1.0, c.text);

        let radius = CornerRadius::same(s.radius_button);
        let widgets = &mut visuals.widgets;
        widgets.noninteractive.bg_fill = c.surface;
        widgets.noninteractive.weak_bg_fill = c.surface;
        widgets.noninteractive.bg_stroke = Stroke::new(1.0, c.border);
        widgets.noninteractive.fg_stroke = Stroke::new(1.0, c.text_dim);
        for (state, fill) in [
            (&mut widgets.inactive, c.surface),
            (&mut widgets.hovered, c.surface_hover),
            (&mut widgets.active, c.surface_selected),
            (&mut widgets.open, c.surface_selected),
        ] {
            state.bg_fill = fill;
            state.weak_bg_fill = fill;
            state.bg_stroke = Stroke::NONE;
            state.fg_stroke = Stroke::new(1.0, c.text);
        }
        for state in [
            &mut widgets.noninteractive,
            &mut widgets.inactive,
            &mut widgets.hovered,
            &mut widgets.active,
            &mut widgets.open,
        ] {
            state.corner_radius = radius;
        }

        style.spacing.item_spacing = vec2(s.spacing, s.spacing);
        style.spacing.button_padding = vec2(s.padding, s.spacing * 0.5);

        let t = &self.text;
        let proportional = |size: f32| FontId::new(size, FontFamily::Proportional);
        style
            .text_styles
            .insert(TextStyle::Heading, proportional(t.heading));
        style
            .text_styles
            .insert(TextStyle::Body, proportional(t.body));
        style
            .text_styles
            .insert(TextStyle::Button, proportional(t.body));
        style
            .text_styles
            .insert(TextStyle::Small, proportional(t.small));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toml_round_trip() {
        let theme = Theme::lazer();
        let text = theme.to_toml().unwrap();
        assert_eq!(Theme::from_toml(&text).unwrap(), theme);
    }

    #[test]
    fn partial_theme_falls_back_to_default() {
        let theme = Theme::from_toml("name = \"custom\"").unwrap();
        assert_eq!(theme.name, "custom");
        assert_eq!(theme.colors, Theme::lazer().colors);
    }
}
