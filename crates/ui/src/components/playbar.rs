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
}

const VOLUME_WIDTH: f32 = 96.0;
const TIME_WIDTH: f32 = 40.0;

pub fn playbar(ui: &mut egui::Ui, theme: &Theme, player: &PlayerView) -> PlaybarEvent {
    let mut event = PlaybarEvent::default();
    let c = &theme.colors;
    let button_size = theme.shape.playbar_height * 0.6;

    Frame::new()
        .fill(c.chrome)
        .inner_margin(Margin::symmetric(theme.shape.padding as i8, 0))
        .show(ui, |ui| {
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
                ui.label(time(mmss(player.position_secs)));
                let seek_width =
                    ui.available_width() - VOLUME_WIDTH - TIME_WIDTH - theme.shape.spacing * 3.0;
                let progress = if player.length_secs > 0.0 {
                    player.position_secs / player.length_secs
                } else {
                    0.0
                };
                let (_, seek) = slider_bar(ui, theme, seek_width.max(0.0), progress);
                event.seek_secs = seek.map(|fraction| fraction * player.length_secs);
                ui.label(time(mmss(player.length_secs)));

                let (_, volume) = slider_bar(ui, theme, VOLUME_WIDTH, player.volume);
                event.volume = volume;
            });
        });

    event
}
