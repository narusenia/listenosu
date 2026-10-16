use eframe::egui;
use ui::Theme;
use ui::components::notice::{NoticeKind, Toasts, banner};
use ui::components::playlist_editor::{PlaylistEditor, PlaylistView};
use ui::components::settings::{KeyBindChange, keybind, setting_row, toggle};
use ui::components::song_rows::{RowEvent, song_rows};
use ui::model::SongView;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Section {
    #[default]
    Components,
    Queue,
    Playlists,
    Notices,
}

impl Section {
    pub const ALL: [Self; 4] = [
        Self::Components,
        Self::Queue,
        Self::Playlists,
        Self::Notices,
    ];

    /// Section named on the command line, e.g. `cargo run -p gallery -- notices`.
    pub fn from_arg(arg: &str) -> Option<Self> {
        match arg {
            "components" => Some(Self::Components),
            "queue" => Some(Self::Queue),
            "playlists" => Some(Self::Playlists),
            "notices" => Some(Self::Notices),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Components => "Components",
            Self::Queue => "Play queue",
            Self::Playlists => "Playlists",
            Self::Notices => "Notices & settings",
        }
    }
}

struct SettingsSample {
    tray_on_close: bool,
    discord: bool,
    hotkeys: bool,
    crossfade_secs: f32,
    hotkey: Option<String>,
    capturing: bool,
    folder: String,
}

pub struct Extras {
    queue: Vec<SongView>,
    queue_current: Option<usize>,
    playlists: Vec<PlaylistView>,
    selected_playlist: Option<usize>,
    editor: PlaylistEditor,
    pub toasts: Toasts,
    db_banner: bool,
    update_banner: bool,
    settings: SettingsSample,
}

impl Extras {
    pub fn new(songs: &[SongView]) -> Self {
        let playlist = |name: &str, range: std::ops::Range<usize>| PlaylistView {
            name: name.to_owned(),
            songs: songs[range].to_vec(),
        };
        Self {
            queue: songs[..40].to_vec(),
            queue_current: Some(2),
            playlists: vec![
                playlist("Work BGM", 0..12),
                playlist("Night drive", 20..28),
                playlist("Empty", 0..0),
            ],
            selected_playlist: Some(0),
            editor: PlaylistEditor::default(),
            toasts: Toasts::default(),
            db_banner: true,
            update_banner: true,
            settings: SettingsSample {
                tray_on_close: true,
                discord: false,
                hotkeys: false,
                crossfade_secs: 0.0,
                hotkey: None,
                capturing: false,
                folder: r"C:\Users\you\AppData\Roaming\osu".to_owned(),
            },
        }
    }

    pub fn queue(&mut self, ui: &mut egui::Ui, theme: &Theme) {
        ui.horizontal(|ui| {
            ui.label(theme.heading(format!("Up next ({})", self.queue.len())));
            if ui.button("Clear").clicked() {
                self.queue.clear();
                self.queue_current = None;
            }
        });
        let event = song_rows(ui, theme, &self.queue, self.queue_current);
        apply_rows(&mut self.queue, &mut self.queue_current, event);
    }

    pub fn playlists(&mut self, ui: &mut egui::Ui, theme: &Theme) {
        let event = self
            .editor
            .show(ui, theme, &self.playlists, self.selected_playlist);

        if let Some(index) = event.select {
            self.selected_playlist = Some(index);
        }
        if let Some(name) = event.create {
            self.playlists.push(PlaylistView {
                name,
                songs: Vec::new(),
            });
            self.selected_playlist = Some(self.playlists.len() - 1);
        }
        let Some(selected) = self.selected_playlist else {
            return;
        };
        if let Some(name) = event.rename {
            self.playlists[selected].name = name;
        }
        if event.delete {
            self.playlists.remove(selected);
            self.selected_playlist = None;
            return;
        }
        let mut no_current = None;
        apply_rows(
            &mut self.playlists[selected].songs,
            &mut no_current,
            event.rows,
        );
    }

    pub fn notices(&mut self, ui: &mut egui::Ui, theme: &Theme) {
        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            ui.label(theme.heading("Banners"));
            if self.db_banner {
                let event = banner(
                    ui,
                    theme,
                    NoticeKind::Warning,
                    "This version of osu!lazer's database is not supported yet. Playing from the last known library.",
                    Some("Check for updates"),
                );
                if event.action {
                    self.toasts.push(NoticeKind::Info, "Checking for updates…", theme.motion.toast_seconds);
                }
                self.db_banner &= !event.dismiss;
            }
            if self.update_banner {
                let event = banner(ui, theme, NoticeKind::Info, "A new version is available.", Some("Download"));
                self.update_banner &= !event.dismiss;
            }
            banner(ui, theme, NoticeKind::Error, "The helper process stopped unexpectedly.", None);
            if (!self.db_banner || !self.update_banner)
                && ui.button("Restore dismissed banners").clicked()
            {
                self.db_banner = true;
                self.update_banner = true;
            }

            ui.add_space(theme.shape.padding);
            ui.label(theme.heading("Toasts"));
            ui.horizontal(|ui| {
                for (label, kind) in [
                    ("Info", NoticeKind::Info),
                    ("Warning", NoticeKind::Warning),
                    ("Error", NoticeKind::Error),
                ] {
                    if ui.button(label).clicked() {
                        self.toasts.push(kind, format!("{label} toast"), theme.motion.toast_seconds);
                    }
                }
            });

            ui.add_space(theme.shape.padding);
            ui.label(theme.heading("Settings controls"));
            let s = &mut self.settings;
            setting_row(ui, theme, "Hide to the tray when the window is closed", |ui| {
                toggle(ui, theme, &mut s.tray_on_close);
            });
            setting_row(ui, theme, "Discord Rich Presence", |ui| {
                toggle(ui, theme, &mut s.discord);
            });
            setting_row(ui, theme, "Global hotkeys", |ui| {
                toggle(ui, theme, &mut s.hotkeys);
            });
            setting_row(ui, theme, "Crossfade (seconds)", |ui| {
                ui.add(egui::Slider::new(&mut s.crossfade_secs, 0.0..=10.0).step_by(0.5));
            });
            setting_row(ui, theme, "Play / pause hotkey", |ui| {
                match keybind(ui, theme, s.hotkey.as_deref(), &mut s.capturing) {
                    Some(KeyBindChange::Set(binding)) => s.hotkey = Some(binding),
                    Some(KeyBindChange::Clear) => s.hotkey = None,
                    None => {}
                }
            });
            setting_row(ui, theme, "osu!lazer data folder", |ui| {
                if ui.button("Browse…").clicked() {
                    self.toasts.push(
                        NoticeKind::Info,
                        "The folder dialog is not part of the gallery.",
                        theme.motion.toast_seconds,
                    );
                }
                ui.label(egui::RichText::new(&s.folder).color(theme.colors.text_dim));
            });
        });
    }
}

/// Applies row actions to a list and keeps `current` pointing at the same song.
fn apply_rows(list: &mut Vec<SongView>, current: &mut Option<usize>, event: RowEvent) {
    if let Some(index) = event.play {
        *current = Some(index);
    }
    if let Some(index) = event.remove {
        list.remove(index);
        *current = match *current {
            Some(c) if c == index => None,
            Some(c) if c > index => Some(c - 1),
            other => other,
        };
    }
    for (index, other) in [
        (event.move_up, event.move_up.and_then(|i| i.checked_sub(1))),
        (event.move_down, event.move_down.map(|i| i + 1)),
    ] {
        let (Some(index), Some(other)) = (index, other) else {
            continue;
        };
        if other >= list.len() {
            continue;
        }
        list.swap(index, other);
        *current = match *current {
            Some(c) if c == index => Some(other),
            Some(c) if c == other => Some(index),
            same => same,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn songs(n: usize) -> Vec<SongView> {
        (0..n)
            .map(|i| SongView {
                title: i.to_string(),
                artist: String::new(),
                bpm: 120.0,
                length_secs: 100,
                missing: false,
            })
            .collect()
    }

    fn titles(list: &[SongView]) -> Vec<&str> {
        list.iter().map(|s| s.title.as_str()).collect()
    }

    #[test]
    fn removing_before_current_shifts_it_and_removing_current_clears_it() {
        let mut list = songs(4);
        let mut current = Some(2);
        let remove = |index| RowEvent {
            remove: Some(index),
            ..RowEvent::default()
        };
        apply_rows(&mut list, &mut current, remove(0));
        assert_eq!((titles(&list), current), (vec!["1", "2", "3"], Some(1)));
        apply_rows(&mut list, &mut current, remove(1));
        assert_eq!((titles(&list), current), (vec!["1", "3"], None));
    }

    #[test]
    fn moving_keeps_current_on_the_same_song() {
        let mut list = songs(3);
        let mut current = Some(1);
        let up = RowEvent {
            move_up: Some(1),
            ..RowEvent::default()
        };
        apply_rows(&mut list, &mut current, up);
        assert_eq!((titles(&list), current), (vec!["1", "0", "2"], Some(0)));
        let down = RowEvent {
            move_down: Some(2),
            ..RowEvent::default()
        };
        apply_rows(&mut list, &mut current, down);
        assert_eq!(titles(&list), vec!["1", "0", "2"]);
    }
}
