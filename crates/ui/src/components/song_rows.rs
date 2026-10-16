use egui::{ScrollArea, vec2};

use super::icons::{Icon, icon_button};
use super::song_card::{CardState, song_card};
use crate::model::SongView;
use crate::theme::Theme;

/// What the user did to the rows this frame; the caller edits its own list.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RowEvent {
    /// Double-clicked: start playing this row.
    pub play: Option<usize>,
    pub remove: Option<usize>,
    pub move_up: Option<usize>,
    pub move_down: Option<usize>,
}

/// Reorderable, removable song rows, shared by the play queue and the playlist editor.
pub fn song_rows(
    ui: &mut egui::Ui,
    theme: &Theme,
    songs: &[SongView],
    current: Option<usize>,
) -> RowEvent {
    let mut event = RowEvent::default();
    let card_height = theme.shape.card_height;
    let button = card_height * 0.45;
    let gap = theme.shape.spacing;
    let buttons_width = 3.0 * button + 2.0 * gap;
    ui.spacing_mut().item_spacing.y = theme.shape.card_gap;

    ScrollArea::vertical().auto_shrink(false).show_rows(
        ui,
        card_height,
        songs.len(),
        |ui, range| {
            for index in range {
                ui.push_id(index, |ui| {
                    ui.horizontal(|ui| {
                        let card_width = (ui.available_width() - buttons_width - gap).max(0.0);
                        ui.allocate_ui(vec2(card_width, card_height), |ui| {
                            let state = CardState {
                                selected: false,
                                playing: current == Some(index),
                            };
                            if song_card(ui, theme, &songs[index], state).double_clicked() {
                                event.play = Some(index);
                            }
                        });
                        ui.vertical(|ui| {
                            ui.add_space((card_height - button) / 2.0);
                            ui.horizontal(|ui| {
                                ui.add_enabled_ui(index > 0, |ui| {
                                    if icon_button(ui, theme, Icon::Up, button).clicked() {
                                        event.move_up = Some(index);
                                    }
                                });
                                ui.add_enabled_ui(index + 1 < songs.len(), |ui| {
                                    if icon_button(ui, theme, Icon::Down, button).clicked() {
                                        event.move_down = Some(index);
                                    }
                                });
                                if icon_button(ui, theme, Icon::Close, button).clicked() {
                                    event.remove = Some(index);
                                }
                            });
                        });
                    });
                });
            }
        },
    );

    event
}
