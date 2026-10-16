use egui::RichText;

use crate::fonts;
use crate::model::SongView;
use crate::theme::Theme;

/// Title, artist and BPM / length of the current song (top left of the main screen).
pub fn info_panel(ui: &mut egui::Ui, theme: &Theme, song: &SongView) {
    let c = &theme.colors;
    ui.label(
        RichText::new(&song.title)
            .font(fonts::bold(theme.text.title))
            .color(c.text),
    );
    ui.label(
        RichText::new(&song.artist)
            .size(theme.text.heading)
            .color(c.text_dim),
    );
    ui.label(
        RichText::new(format!(
            "{:.0} BPM · {}",
            song.bpm,
            crate::model::mmss(song.length_secs as f32)
        ))
        .size(theme.text.body)
        .color(c.text_dim),
    );
}
