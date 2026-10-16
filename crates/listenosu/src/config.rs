//! The app's own settings and the state it restores on the next launch.

use std::path::PathBuf;

use library::store::{AppDirs, read_json, write_json};
use player::Repeat;
use serde::{Deserialize, Serialize};
use ui::screens::settings::SettingsState;

/// Outer position and inner size of a window, in points.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Geometry {
    pub position: [f32; 2],
    pub size: [f32; 2],
}

/// The queue and position of the last session, restored paused.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LastSession {
    pub audio_hashes: Vec<String>,
    pub index: usize,
    pub position_secs: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub data_folder: String,
    pub language: usize,
    pub romanized: bool,
    pub group_same: bool,
    pub crossfade_secs: f32,
    pub tray_on_close: bool,
    pub mini_on_top: bool,
    pub spectrum: bool,
    pub hotkeys: bool,
    pub hotkey_play_pause: Option<String>,
    pub discord: bool,
    pub check_updates: bool,

    /// Whether the queue / playlists panel is expanded.
    pub side_open: bool,
    pub volume: f32,
    pub shuffle: bool,
    /// 0 off, 1 all, 2 one.
    pub repeat: u8,
    pub sort: usize,
    pub mini: bool,
    pub full_window: Option<Geometry>,
    pub mini_window: Option<Geometry>,
    pub last_session: Option<LastSession>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            data_folder: String::new(),
            language: 0,
            romanized: false,
            group_same: true,
            side_open: false,
            crossfade_secs: 0.0,
            tray_on_close: true,
            mini_on_top: false,
            spectrum: true,
            hotkeys: false,
            hotkey_play_pause: None,
            discord: false,
            check_updates: true,
            volume: 0.7,
            shuffle: false,
            repeat: 0,
            sort: 2,
            mini: false,
            full_window: None,
            mini_window: None,
            last_session: None,
        }
    }
}

impl Config {
    pub fn path(dirs: &AppDirs) -> PathBuf {
        dirs.config.join("settings.json")
    }

    /// A missing or unreadable file gives the defaults.
    pub fn load(dirs: &AppDirs) -> Self {
        read_json(&Self::path(dirs)).unwrap_or_default()
    }

    pub fn save(&self, dirs: &AppDirs) {
        // Losing a save only costs the last few changes, so a failed write is not worth interrupting playback.
        let _ = write_json(&Self::path(dirs), self);
    }

    pub fn repeat_mode(&self) -> Repeat {
        match self.repeat {
            1 => Repeat::All,
            2 => Repeat::One,
            _ => Repeat::Off,
        }
    }

    pub fn set_repeat_mode(&mut self, repeat: Repeat) {
        self.repeat = match repeat {
            Repeat::Off => 0,
            Repeat::All => 1,
            Repeat::One => 2,
        };
    }

    pub fn settings_state(&self) -> SettingsState {
        SettingsState {
            data_folder: self.data_folder.clone(),
            language: self.language,
            romanized: self.romanized,
            group_same: self.group_same,
            crossfade_secs: self.crossfade_secs,
            tray_on_close: self.tray_on_close,
            mini_on_top: self.mini_on_top,
            spectrum: self.spectrum,
            hotkeys: self.hotkeys,
            hotkey_play_pause: self.hotkey_play_pause.clone(),
            discord: self.discord,
            check_updates: self.check_updates,
            capturing: false,
        }
    }

    pub fn apply_settings(&mut self, s: &SettingsState) {
        self.data_folder = s.data_folder.clone();
        self.language = s.language;
        self.romanized = s.romanized;
        self.group_same = s.group_same;
        self.crossfade_secs = s.crossfade_secs;
        self.tray_on_close = s.tray_on_close;
        self.mini_on_top = s.mini_on_top;
        self.spectrum = s.spectrum;
        self.hotkeys = s.hotkeys;
        self.hotkey_play_pause = s.hotkey_play_pause.clone();
        self.discord = s.discord;
        self.check_updates = s.check_updates;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_fields_fall_back_to_defaults_so_old_files_keep_loading() {
        let config: Config = serde_json::from_str(r#"{"volume": 0.2}"#).unwrap();
        assert_eq!(config.volume, 0.2);
        assert!(config.spectrum && config.tray_on_close);
    }

    #[test]
    fn settings_round_trip_through_the_settings_screen_state() {
        let mut config = Config::default();
        let mut state = config.settings_state();
        state.romanized = true;
        state.crossfade_secs = 4.5;
        config.apply_settings(&state);
        assert_eq!(config.settings_state(), state);
    }
}
