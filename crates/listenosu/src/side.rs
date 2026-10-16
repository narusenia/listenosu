//! The area under the song info on the main screen: queue and playlists.

use eframe::egui;
use ui::Theme;
use ui::components::notice::{NoticeKind, banner};
use ui::components::playlist_editor::{PlaylistEditor, PlaylistEvent, PlaylistView};
use ui::components::song_rows::{RowEvent, song_rows};
use ui::model::SongView;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Tab {
    #[default]
    Queue,
    Playlists,
}

/// What the user asked for; the app applies it.
pub enum Action {
    DismissBanner,
    ClearQueue,
    AddToQueue,
    QueueRows(RowEvent),
    AddToPlaylist(usize),
    Playlist(PlaylistEvent),
}

pub struct SideInput<'a> {
    pub queue_current: Option<usize>,
    /// A song is selected in the list, so it can be added somewhere.
    pub can_add: bool,
    pub problem: Option<&'a str>,
}

#[derive(Default)]
pub struct Side {
    tab: Tab,
    editor: PlaylistEditor,
    pub selected_playlist: Option<usize>,
    pub queue_views: Vec<SongView>,
    pub playlist_views: Vec<PlaylistView>,
}

impl Side {
    pub fn show(&mut self, ui: &mut egui::Ui, theme: &Theme, input: &SideInput) -> Vec<Action> {
        let mut actions = Vec::new();
        if let Some(text) = input.problem
            && banner(ui, theme, NoticeKind::Warning, text, None).dismiss
        {
            actions.push(Action::DismissBanner);
        }
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.tab, Tab::Queue, "Queue");
            ui.selectable_value(&mut self.tab, Tab::Playlists, "Playlists");
        });
        // Leave the bottom to the spectrum, which the main screen pins there.
        let size = egui::vec2(
            ui.available_width(),
            (ui.available_height() - theme.spectrum.height).max(0.0),
        );
        ui.allocate_ui(size, |ui| match self.tab {
            Tab::Queue => self.queue(ui, theme, input, &mut actions),
            Tab::Playlists => self.playlists(ui, theme, input, &mut actions),
        });
        actions
    }

    fn queue(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        input: &SideInput,
        out: &mut Vec<Action>,
    ) {
        ui.horizontal(|ui| {
            ui.label(theme.heading(format!("Up next ({})", self.queue_views.len())));
            if ui.button("Clear").clicked() {
                out.push(Action::ClearQueue);
            }
            if ui
                .add_enabled(input.can_add, egui::Button::new("Add selected song"))
                .clicked()
            {
                out.push(Action::AddToQueue);
            }
        });
        out.push(Action::QueueRows(song_rows(
            ui,
            theme,
            &self.queue_views,
            input.queue_current,
        )));
    }

    fn playlists(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        input: &SideInput,
        out: &mut Vec<Action>,
    ) {
        if let Some(selected) = self.selected_playlist
            && ui
                .add_enabled(
                    input.can_add,
                    egui::Button::new("Add selected song to this playlist"),
                )
                .clicked()
        {
            out.push(Action::AddToPlaylist(selected));
        }
        let event = self
            .editor
            .show(ui, theme, &self.playlist_views, self.selected_playlist);
        out.push(Action::Playlist(event));
    }
}
