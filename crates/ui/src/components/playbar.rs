use egui::{Frame, Margin, RichText};

use super::controls::slider_bar;
use super::icons::{Icon, icon_button};
use crate::model::{PlayerView, Repeat, mmss};
use crate::theme::Theme;

/// What the user asked for this frame; the caller applies it to the player.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PlaybarEvent {
    pub toggle_play: bool,
    pub prev: bool,
    pub next: bool,
    pub toggle_shuffle: bool,
    pub cycle_repeat: bool,
    pub seek_secs: Option<f32>,
    pub volume: Option<f32>,
    /// Switch to the mini player.
    pub toggle_mini: bool,
}

pub fn playbar(ui: &mut egui::Ui, theme: &Theme, player: &PlayerView) -> PlaybarEvent {
    let mut event = PlaybarEvent::default();
    let c = &theme.colors;
    let button_size = theme.shape.playbar_height * 0.6;

    Frame::new()
        .fill(c.chrome)
        .inner_margin(Margin::symmetric(theme.shape.padding as i8, 0))
        .show(ui, |ui| {
            // A Frame only fills the width its contents used, so claim the whole row.
            ui.set_width(ui.available_width());
            ui.set_height(theme.shape.playbar_height);
            ui.horizontal_centered(|ui| {
                event.prev = icon_button(ui, theme, Icon::Prev, button_size).clicked();
                let play = if player.playing {
                    Icon::Pause
                } else {
                    Icon::Play
                };
                event.toggle_play = icon_button(ui, theme, play, button_size).clicked();
                event.next = icon_button(ui, theme, Icon::Next, button_size).clicked();

                event.toggle_shuffle = ui.selectable_label(player.shuffle, "Shuffle").clicked();
                let repeat_label = match player.repeat {
                    Repeat::Off => "Repeat: off",
                    Repeat::All => "Repeat: all",
                    Repeat::One => "Repeat: one",
                };
                event.cycle_repeat = ui
                    .selectable_label(player.repeat != Repeat::Off, repeat_label)
                    .clicked();

                let time =
                    |text: String| RichText::new(text).size(theme.text.small).color(c.text_dim);
                // Fixed width, so the seek bar width below adds up exactly.
                let fixed = |ui: &mut egui::Ui, text: String| {
                    ui.add_sized(
                        [theme.time_width(), theme.text.body],
                        egui::Label::new(time(text)),
                    )
                };
                fixed(ui, mmss(player.position_secs));
                let seek_width = ui.available_width()
                    - theme.volume_width()
                    - theme.time_width()
                    - theme.button_width()
                    - theme.shape.spacing * 4.0;
                let progress = if player.length_secs > 0.0 {
                    player.position_secs / player.length_secs
                } else {
                    0.0
                };
                let (_, seek) = slider_bar(ui, theme, seek_width.max(0.0), progress);
                event.seek_secs = seek.map(|fraction| fraction * player.length_secs);
                fixed(ui, mmss(player.length_secs));

                let (_, volume) = slider_bar(ui, theme, theme.volume_width(), player.volume);
                event.volume = volume;

                event.toggle_mini = ui
                    .add_sized(
                        [theme.button_width(), theme.text.body * 1.6],
                        egui::Button::new("Mini"),
                    )
                    .clicked();
            });
        });

    event
}
