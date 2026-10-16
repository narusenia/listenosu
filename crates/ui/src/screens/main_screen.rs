use egui::{CentralPanel, Frame, Margin, Panel};

use crate::backdrop::Backdrop;
use crate::components::controls::{dropdown, search_bar};
use crate::components::info_panel::info_panel;
use crate::components::playbar::{PlaybarEvent, playbar};
use crate::components::song_list::{ListEvent, song_list};
use crate::components::spectrum::spectrum;
use crate::model::{PlayerView, SongView};
use crate::theme::Theme;

/// Controls the screen owns; the caller reads `query`, `view` and `sort` when `list_changed`.
#[derive(Clone, Debug, Default)]
pub struct MainState {
    pub query: String,
    pub view: usize,
    pub sort: usize,
    pub selected: Option<usize>,
}

pub struct MainInput<'a> {
    /// The filtered, sorted list as shown.
    pub songs: &'a [SongView],
    /// Row of the playing song in `songs`, if it is shown.
    pub playing: Option<usize>,
    /// The playing song for the info panel, even when the search hides it from the list.
    pub now_playing: Option<&'a SongView>,
    pub player: &'a PlayerView,
    /// Smoothed spectrum bar levels, 0.0 to 1.0.
    pub spectrum: &'a [f32],
    pub view_options: &'a [String],
    pub sort_options: &'a [String],
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MainEvent {
    /// Search text, view or sort changed: the caller rebuilds the list.
    pub list_changed: bool,
    pub list: ListEvent,
    pub playbar: PlaybarEvent,
}

/// lazer-style song select: backdrop everywhere, song info top left, card list on the right,
/// spectrum above the playbar. `extra` draws below the song info (the gallery's tabs).
pub fn main_screen(
    ui: &mut egui::Ui,
    theme: &Theme,
    backdrop: &mut Backdrop,
    state: &mut MainState,
    input: &MainInput,
    extra: impl FnOnce(&mut egui::Ui),
) -> MainEvent {
    let mut event = MainEvent::default();
    // Painted first, so the panels below draw on top without a fill of their own.
    backdrop.paint(ui, ui.available_rect_before_wrap(), theme);

    Panel::bottom("main_playbar")
        .frame(Frame::NONE)
        .show(ui, |ui| event.playbar = playbar(ui, theme, input.player));

    Panel::right("main_list")
        .frame(Frame::NONE.inner_margin(Margin::same(theme.shape.padding as i8)))
        .default_size(theme.shape.list_width)
        .show(ui, |ui| {
            let mut changed = search_bar(ui, theme, &mut state.query, input.songs.len()).changed();
            changed |= dropdown(ui, "view", "View", &mut state.view, input.view_options);
            changed |= dropdown(ui, "sort", "Sort", &mut state.sort, input.sort_options);
            if changed {
                state.selected = None;
                event.list_changed = true;
            }

            event.list = song_list(ui, theme, input.songs, state.selected, input.playing);
            if event.list.clicked.is_some() {
                state.selected = event.list.clicked;
            }
        });

    CentralPanel::default().frame(Frame::NONE).show(ui, |ui| {
        Frame::NONE
            .inner_margin(Margin::same(theme.shape.padding as i8))
            .show(ui, |ui| {
                if let Some(song) = input.now_playing {
                    info_panel(ui, theme, song);
                }
                extra(ui);
            });
        // Pin the spectrum to the bottom of the free space.
        ui.add_space((ui.available_height() - theme.spectrum.height).max(0.0));
        spectrum(ui, theme, input.spectrum, theme.spectrum.height);
    });

    event
}
