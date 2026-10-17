use egui::ScrollArea;

use super::song_card::{CardState, song_card};
use crate::model::SongView;
use crate::theme::Theme;

/// What the right-click menu of a song can ask for. Copying the title is done by the menu itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SongMenu {
    Play,
    PlayNext,
    AddToQueue,
    /// Index into the playlists passed to `song_list`.
    AddToPlaylist(usize),
    ShowInFolder,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ListEvent {
    pub clicked: Option<usize>,
    /// Double-clicked: the caller should start playing this song.
    pub activated: Option<usize>,
    /// A right-click menu entry was chosen for this row.
    pub menu: Option<(usize, SongMenu)>,
}

/// Virtualized list: only the visible rows are laid out, so thousands of songs stay cheap.
pub fn song_list(
    ui: &mut egui::Ui,
    theme: &Theme,
    songs: &[SongView],
    selected: Option<usize>,
    playing: Option<usize>,
    playlists: &[String],
) -> ListEvent {
    let mut event = ListEvent::default();
    ui.spacing_mut().item_spacing.y = theme.shape.card_gap;

    ScrollArea::vertical().auto_shrink(false).show_rows(
        ui,
        theme.shape.card_height,
        songs.len(),
        |ui, range| {
            for index in range {
                let state = CardState {
                    selected: selected == Some(index),
                    playing: playing == Some(index),
                };
                let response = song_card(ui, theme, &songs[index], state);
                if response.clicked() {
                    event.clicked = Some(index);
                }
                if response.double_clicked() {
                    event.activated = Some(index);
                }
                // Select on right-click too, so the menu acts on the row the user sees marked.
                if response.secondary_clicked() {
                    event.clicked = Some(index);
                }
                if let Some(choice) = song_menu(&response, &songs[index], playlists) {
                    event.menu = Some((index, choice));
                }
            }
        },
    );

    event
}

fn song_menu(response: &egui::Response, song: &SongView, playlists: &[String]) -> Option<SongMenu> {
    let mut choice = None;
    response.context_menu(|ui| {
        if ui.button("Play").clicked() {
            choice = Some(SongMenu::Play);
        }
        if ui.button("Play next").clicked() {
            choice = Some(SongMenu::PlayNext);
        }
        if ui.button("Add to queue").clicked() {
            choice = Some(SongMenu::AddToQueue);
        }
        ui.add_enabled_ui(!playlists.is_empty(), |ui| {
            ui.menu_button("Add to playlist", |ui| {
                for (index, name) in playlists.iter().enumerate() {
                    if ui.button(name).clicked() {
                        choice = Some(SongMenu::AddToPlaylist(index));
                    }
                }
            });
        });
        ui.separator();
        if ui.button("Show in folder").clicked() {
            choice = Some(SongMenu::ShowInFolder);
        }
        if ui.button("Copy title").clicked() {
            ui.ctx().copy_text(song.title.clone());
        }
        // Any choice closes the menu; the submenu does not close it on its own.
        if choice.is_some() {
            ui.close();
        }
    });
    choice
}
