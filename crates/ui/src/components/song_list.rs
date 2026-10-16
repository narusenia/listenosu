use egui::ScrollArea;

use super::song_card::{CardState, song_card};
use crate::model::SongView;
use crate::theme::Theme;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ListEvent {
    pub clicked: Option<usize>,
    /// Double-clicked: the caller should start playing this song.
    pub activated: Option<usize>,
}

/// Virtualized list: only the visible rows are laid out, so thousands of songs stay cheap.
pub fn song_list(
    ui: &mut egui::Ui,
    theme: &Theme,
    songs: &[SongView],
    selected: Option<usize>,
    playing: Option<usize>,
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
            }
        },
    );

    event
}
