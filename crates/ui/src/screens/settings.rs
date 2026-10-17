use egui::{Frame, Margin, RichText, ScrollArea};

use super::header;
use crate::components::controls::dropdown_inline;
use crate::components::settings::{KeyBindChange, keybind, setting_row, toggle};
use crate::theme::Theme;

pub const LANGUAGES: [&str; 3] = ["System", "English", "日本語"];

#[derive(Clone, Debug, PartialEq)]
pub struct SettingsState {
    pub data_folder: String,
    /// Index into `LANGUAGES`.
    pub language: usize,
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
    pub discord: bool,
    /// Application ID from the Discord Developer Portal; presence stays off while it is empty.
    pub discord_app_id: String,
    pub check_updates: bool,
    /// The play / pause hotkey button is waiting for a key press.
    pub capturing: bool,
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
    state: &mut SettingsState,
) -> SettingsEvent {
    let mut event = SettingsEvent::default();
    Frame::NONE
        .inner_margin(Margin::same(theme.shape.padding as i8))
        .show(ui, |ui| {
            event.back = header(ui, theme, "Settings");
            ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                section(ui, theme, "Library");
                setting_row(ui, theme, "osu!lazer data folder", |ui| {
                    event.choose_folder = ui.button("Browse…").clicked();
                    ui.label(RichText::new(&state.data_folder).color(theme.colors.text_dim));
                });

                section(ui, theme, "Display");
                setting_row(ui, theme, "Language", |ui| {
                    let options = LANGUAGES.map(String::from);
                    dropdown_inline(ui, "language", "", &mut state.language, &options, None);
                });
                setting_row(ui, theme, "Show romanized titles and artists", |ui| {
                    toggle(ui, theme, &mut state.romanized);
                });
                setting_row(
                    ui,
                    theme,
                    "Group songs with the same title and artist",
                    |ui| {
                        toggle(ui, theme, &mut state.group_same);
                    },
                );
                setting_row(ui, theme, "Audio spectrum", |ui| {
                    toggle(ui, theme, &mut state.spectrum);
                });
                ui.add_enabled_ui(state.spectrum, |ui| {
                    setting_row(ui, theme, "Spectrum bars", |ui| {
                        ui.add(egui::Slider::new(&mut state.spectrum_bars, 16..=128).step_by(8.0));
                    });
                    setting_row(ui, theme, "Spectrum fall speed", |ui| {
                        ui.add(
                            egui::Slider::new(&mut state.spectrum_decay, 0.4..=4.0).step_by(0.2),
                        );
                    });
                    setting_row(ui, theme, "Spectrum height", |ui| {
                        ui.add(
                            egui::Slider::new(&mut state.spectrum_height, 24.0..=160.0)
                                .step_by(8.0),
                        );
                    });
                });
                setting_row(ui, theme, "Backdrop blur", |ui| {
                    ui.add(egui::Slider::new(&mut state.backdrop_blur, 4.0..=48.0).step_by(1.0));
                });
                setting_row(ui, theme, "Backdrop dimming", |ui| {
                    ui.add(egui::Slider::new(&mut state.backdrop_dim, 0.0..=1.0).step_by(0.05));
                });

                section(ui, theme, "Playback");
                setting_row(ui, theme, "Crossfade (seconds, 0 = off)", |ui| {
                    ui.add(egui::Slider::new(&mut state.crossfade_secs, 0.0..=10.0).step_by(0.5));
                });

                section(ui, theme, "Window");
                setting_row(
                    ui,
                    theme,
                    "Hide to the tray when the window is closed",
                    |ui| {
                        toggle(ui, theme, &mut state.tray_on_close);
                    },
                );
                setting_row(ui, theme, "Keep the mini player on top", |ui| {
                    toggle(ui, theme, &mut state.mini_on_top);
                });

                section(ui, theme, "Integrations");
                setting_row(ui, theme, "Global hotkeys", |ui| {
                    toggle(ui, theme, &mut state.hotkeys);
                });
                ui.add_enabled_ui(state.hotkeys, |ui| {
                    setting_row(ui, theme, "Play / pause hotkey", |ui| {
                        let current = state.hotkey_play_pause.as_deref();
                        match keybind(ui, theme, current, &mut state.capturing) {
                            Some(KeyBindChange::Set(binding)) => {
                                state.hotkey_play_pause = Some(binding)
                            }
                            Some(KeyBindChange::Clear) => state.hotkey_play_pause = None,
                            None => {}
                        }
                    });
                });
                setting_row(ui, theme, "Discord Rich Presence", |ui| {
                    toggle(ui, theme, &mut state.discord);
                });
                ui.add_enabled_ui(state.discord, |ui| {
                    setting_row(ui, theme, "Discord Application ID", |ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut state.discord_app_id)
                                .desired_width(theme.shape.list_width / 2.0)
                                .hint_text("Application ID"),
                        );
                    });
                });

                section(ui, theme, "Updates");
                setting_row(ui, theme, "Check for updates at startup", |ui| {
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
