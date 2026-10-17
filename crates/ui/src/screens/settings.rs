use crate::i18n::t;
use egui::{Frame, Margin, RichText, ScrollArea};

use super::header;
use crate::components::controls::dropdown_inline;
use crate::components::settings::{KeyBindChange, keybind, setting_row, toggle};
use crate::theme::Theme;

/// Language names are shown in their own language, except the first entry, which follows the OS.
pub fn languages() -> [String; 3] {
    [
        t("settings.language_system").to_owned(),
        "English".to_owned(),
        "日本語".to_owned(),
    ]
}

#[derive(Clone, Debug, PartialEq)]
pub struct SettingsState {
    pub data_folder: String,
    /// Index into `languages()`.
    pub language: usize,
    /// Name of the chosen theme; one of the names given to `settings_screen`.
    pub theme: String,
    /// Show romanized title and artist instead of the original language.
    pub romanized: bool,
    /// Show songs with the same title and artist (speed variants, re-imports) as one row.
    pub group_same: bool,
    pub crossfade_secs: f32,
    pub tray_on_close: bool,
    pub mini_on_top: bool,
    pub spectrum: bool,
    /// Backdrop blur strength and darkening (0.0 to 1.0); the same values as `Theme::backdrop`.
    /// Spectrum bar count, fall speed (fraction of the range per second) and height; the same
    /// values as `Theme::spectrum`.
    pub spectrum_bars: usize,
    pub spectrum_decay: f32,
    pub spectrum_height: f32,
    pub backdrop_blur: f32,
    pub backdrop_dim: f32,
    pub hotkeys: bool,
    pub hotkey_play_pause: Option<String>,
    pub hotkey_next: Option<String>,
    pub hotkey_previous: Option<String>,
    pub discord: bool,
    /// Application ID from the Discord Developer Portal; presence stays off while it is empty.
    pub discord_app_id: String,
    /// Send the beatmap cover (an image URL on osu!'s server) and a link button to Discord.
    pub discord_cover: bool,
    pub discord_button: bool,
    pub check_updates: bool,
    /// The hotkey row (0 = play / pause, 1 = next, 2 = previous) waiting for a key press.
    pub capturing: Option<usize>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SettingsEvent {
    /// The caller opens the folder dialog and writes the result to `data_folder`.
    pub choose_folder: bool,
    pub back: bool,
}

pub fn settings_screen(
    ui: &mut egui::Ui,
    theme: &Theme,
    theme_names: &[String],
    state: &mut SettingsState,
) -> SettingsEvent {
    let mut event = SettingsEvent::default();
    Frame::NONE
        .inner_margin(Margin::same(theme.shape.padding as i8))
        .show(ui, |ui| {
            event.back = header(ui, theme, t("settings.title"));
            ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                section(ui, theme, t("settings.library"));
                setting_row(ui, theme, t("settings.data_folder"), |ui| {
                    event.choose_folder = ui.button(t("settings.browse")).clicked();
                    ui.label(RichText::new(&state.data_folder).color(theme.colors.text_dim));
                });

                section(ui, theme, t("settings.display"));
                setting_row(ui, theme, t("settings.language"), |ui| {
                    let options = languages();
                    dropdown_inline(ui, "language", "", &mut state.language, &options, None);
                });
                setting_row(ui, theme, t("settings.theme"), |ui| {
                    let current = theme_names.iter().position(|n| *n == state.theme);
                    let mut picked = current.unwrap_or(0);
                    dropdown_inline(ui, "theme", "", &mut picked, theme_names, None);
                    if current != Some(picked)
                        && let Some(name) = theme_names.get(picked)
                    {
                        state.theme = name.clone();
                    }
                });
                setting_row(ui, theme, t("settings.romanized"), |ui| {
                    toggle(ui, theme, &mut state.romanized);
                });
                setting_row(ui, theme, t("settings.group_songs"), |ui| {
                    toggle(ui, theme, &mut state.group_same);
                });
                setting_row(ui, theme, t("settings.spectrum"), |ui| {
                    toggle(ui, theme, &mut state.spectrum);
                });
                ui.add_enabled_ui(state.spectrum, |ui| {
                    setting_row(ui, theme, t("settings.spectrum_bars"), |ui| {
                        ui.add(egui::Slider::new(&mut state.spectrum_bars, 16..=128).step_by(8.0));
                    });
                    setting_row(ui, theme, t("settings.spectrum_decay"), |ui| {
                        ui.add(
                            egui::Slider::new(&mut state.spectrum_decay, 0.4..=4.0).step_by(0.2),
                        );
                    });
                    setting_row(ui, theme, t("settings.spectrum_height"), |ui| {
                        ui.add(
                            egui::Slider::new(&mut state.spectrum_height, 24.0..=160.0)
                                .step_by(8.0),
                        );
                    });
                });
                setting_row(ui, theme, t("settings.backdrop_blur"), |ui| {
                    ui.add(egui::Slider::new(&mut state.backdrop_blur, 4.0..=48.0).step_by(1.0));
                });
                setting_row(ui, theme, t("settings.backdrop_dim"), |ui| {
                    ui.add(egui::Slider::new(&mut state.backdrop_dim, 0.0..=1.0).step_by(0.05));
                });

                section(ui, theme, t("settings.playback"));
                setting_row(ui, theme, t("settings.crossfade"), |ui| {
                    ui.add(egui::Slider::new(&mut state.crossfade_secs, 0.0..=10.0).step_by(0.5));
                });

                section(ui, theme, t("settings.window"));
                setting_row(ui, theme, t("settings.tray_on_close"), |ui| {
                    toggle(ui, theme, &mut state.tray_on_close);
                });
                setting_row(ui, theme, t("settings.mini_on_top"), |ui| {
                    toggle(ui, theme, &mut state.mini_on_top);
                });

                section(ui, theme, t("settings.integrations"));
                setting_row(ui, theme, t("settings.hotkeys"), |ui| {
                    toggle(ui, theme, &mut state.hotkeys);
                });
                ui.add_enabled_ui(state.hotkeys, |ui| {
                    let rows = [
                        (
                            t("settings.hotkey_play_pause"),
                            &mut state.hotkey_play_pause,
                        ),
                        (t("settings.hotkey_next"), &mut state.hotkey_next),
                        (t("settings.hotkey_previous"), &mut state.hotkey_previous),
                    ];
                    for (slot, (label, binding)) in rows.into_iter().enumerate() {
                        setting_row(ui, theme, label, |ui| {
                            let mut capturing = state.capturing == Some(slot);
                            match keybind(ui, theme, binding.as_deref(), &mut capturing) {
                                Some(KeyBindChange::Set(set)) => *binding = Some(set),
                                Some(KeyBindChange::Clear) => *binding = None,
                                None => {}
                            }
                            if capturing {
                                state.capturing = Some(slot);
                            } else if state.capturing == Some(slot) {
                                state.capturing = None;
                            }
                        });
                    }
                });
                setting_row(ui, theme, t("settings.discord"), |ui| {
                    toggle(ui, theme, &mut state.discord);
                });
                ui.add_enabled_ui(state.discord, |ui| {
                    setting_row(ui, theme, t("settings.discord_app_id"), |ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut state.discord_app_id)
                                .desired_width(theme.shape.list_width / 2.0)
                                .hint_text(t("settings.application_id")),
                        );
                    });
                    setting_row(ui, theme, t("settings.discord_cover"), |ui| {
                        toggle(ui, theme, &mut state.discord_cover);
                    });
                    setting_row(ui, theme, t("settings.discord_button"), |ui| {
                        toggle(ui, theme, &mut state.discord_button);
                    });
                });

                section(ui, theme, t("settings.updates"));
                setting_row(ui, theme, t("settings.check_updates"), |ui| {
                    toggle(ui, theme, &mut state.check_updates);
                });
            });
        });
    event
}

fn section(ui: &mut egui::Ui, theme: &Theme, title: &str) {
    ui.add_space(theme.shape.padding);
    ui.label(
        RichText::new(title)
            .size(theme.text.small)
            .color(theme.colors.accent),
    );
}
