use crate::i18n::t;
use egui::RichText;

use super::song_rows::{RowEvent, song_rows};
use crate::fonts;
use crate::model::SongView;
use crate::theme::Theme;

pub struct PlaylistView {
    pub name: String,
    pub songs: Vec<SongView>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlaylistEvent {
    pub select: Option<usize>,
    pub create: Option<String>,
    pub rename: Option<String>,
    pub delete: bool,
    /// Row actions on the selected playlist's songs.
    pub rows: RowEvent,
}

/// Text buffers and the delete confirmation live here; the playlists themselves stay with the caller.
#[derive(Default)]
pub struct PlaylistEditor {
    new_name: String,
    rename: Option<String>,
    confirm_delete: bool,
}

impl PlaylistEditor {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        playlists: &[PlaylistView],
        selected: Option<usize>,
    ) -> PlaylistEvent {
        let mut event = PlaylistEvent::default();

        ui.label(theme.heading(t("playlists.title")));
        for (index, playlist) in playlists.iter().enumerate() {
            let label = format!("{}  ({})", playlist.name, playlist.songs.len());
            if ui
                .selectable_label(selected == Some(index), label)
                .clicked()
            {
                event.select = Some(index);
                self.rename = None;
                self.confirm_delete = false;
            }
        }

        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.new_name)
                    .hint_text(t("playlists.new"))
                    .desired_width(ui.available_width() - theme.shape.padding * 6.0),
            );
            let name = self.new_name.trim();
            let taken = playlists.iter().any(|p| p.name == name);
            let can_add = !name.is_empty() && !taken;
            if ui
                .add_enabled(can_add, egui::Button::new(t("common.add")))
                .clicked()
            {
                event.create = Some(name.to_owned());
                self.new_name.clear();
            }
        });

        let Some(playlist) = selected.and_then(|i| playlists.get(i)) else {
            return event;
        };
        ui.separator();

        ui.horizontal(|ui| {
            super::controls::fix_row_height(ui);
            if let Some(name) = &mut self.rename {
                ui.add(
                    egui::TextEdit::singleline(name)
                        .desired_width(ui.available_width() - theme.shape.padding * 12.0),
                );
                let trimmed = name.trim();
                let valid = !trimmed.is_empty()
                    && (trimmed == playlist.name || !playlists.iter().any(|p| p.name == trimmed));
                if ui
                    .add_enabled(valid, egui::Button::new(t("common.ok")))
                    .clicked()
                {
                    event.rename = Some(trimmed.to_owned());
                    self.rename = None;
                } else if ui.button(t("common.cancel")).clicked() {
                    self.rename = None;
                }
            } else {
                ui.label(
                    RichText::new(&playlist.name)
                        .font(fonts::bold(theme.text.body))
                        .color(theme.colors.text),
                );
                if ui.button(t("playlists.rename")).clicked() {
                    self.rename = Some(playlist.name.clone());
                }
                if self.confirm_delete {
                    if ui
                        .button(
                            RichText::new(t("playlists.confirm_delete")).color(theme.colors.error),
                        )
                        .clicked()
                    {
                        event.delete = true;
                        self.confirm_delete = false;
                    } else if ui.button(t("common.cancel")).clicked() {
                        self.confirm_delete = false;
                    }
                } else if ui.button(t("playlists.delete")).clicked() {
                    self.confirm_delete = true;
                }
            }
        });

        event.rows = song_rows(ui, theme, &playlist.songs, None);
        event
    }
}
