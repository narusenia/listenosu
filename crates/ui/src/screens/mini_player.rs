use egui::{Align, Frame, Label, Layout, Margin, RichText, UiBuilder, pos2};

use crate::backdrop::Backdrop;
use crate::components::controls::slider_bar;
use crate::components::icons::{Icon, icon_button};
use crate::components::playbar::PlaybarEvent;
use crate::components::spectrum::spectrum;
use crate::fonts;
use crate::model::{PlayerView, SongView, mmss};
use crate::theme::Theme;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MiniEvent {
    /// Same buttons as the main playbar; shuffle, repeat and volume are not shown here.
    pub playbar: PlaybarEvent,
    /// Switch back to the main screen.
    pub expand: bool,
}

/// Small window: backdrop, title and artist, transport buttons and a seek bar over a thin spectrum.
pub fn mini_player(
    ui: &mut egui::Ui,
    theme: &Theme,
    backdrop: &mut Backdrop,
    song: Option<&SongView>,
    player: &PlayerView,
    levels: &[f32],
) -> MiniEvent {
    let mut event = MiniEvent::default();
    let c = &theme.colors;
    let area = ui.available_rect_before_wrap();
    backdrop.paint(ui, area, theme);

    // Drawn first so the controls sit on top of it.
    let strip = egui::Rect::from_min_max(
        pos2(area.left(), area.bottom() - theme.spectrum.mini_height),
        area.max,
    );
    spectrum(
        &mut ui.new_child(UiBuilder::new().max_rect(strip)),
        theme,
        levels,
        theme.spectrum.mini_height,
    );

    Frame::NONE
        .inner_margin(Margin::same(theme.shape.padding as i8))
        .show(ui, |ui| {
            // Full button first, right-aligned; the title takes whatever width is left.
            ui.horizontal(|ui| {
                ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                    event.expand = icon_button(ui, theme, Icon::Full, theme.text.body * 1.6)
                        .on_hover_text("Full window")
                        .clicked();
                    ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
                        let Some(song) = song else { return };
                        let mut truncated = |text: RichText| ui.add(Label::new(text).truncate());
                        truncated(
                            RichText::new(&song.title)
                                .font(fonts::bold(theme.text.heading))
                                .color(c.text),
                        );
                        truncated(
                            RichText::new(&song.artist)
                                .size(theme.text.small)
                                .color(c.text_dim),
                        );
                    });
                });
            });

            let button = theme.shape.playbar_height * 0.45;
            ui.horizontal(|ui| {
                let pb = &mut event.playbar;
                pb.prev = icon_button(ui, theme, Icon::Prev, button).clicked();
                let play = if player.playing {
                    Icon::Pause
                } else {
                    Icon::Play
                };
                pb.toggle_play = icon_button(ui, theme, play, button).clicked();
                pb.next = icon_button(ui, theme, Icon::Next, button).clicked();

                let time = |ui: &mut egui::Ui, secs: f32| {
                    ui.add_sized(
                        [theme.time_width(), theme.text.body],
                        Label::new(
                            RichText::new(mmss(secs))
                                .size(theme.text.small)
                                .color(c.text_dim),
                        ),
                    );
                };
                time(ui, player.position_secs);
                let seek_width =
                    ui.available_width() - theme.time_width() - theme.shape.spacing * 2.0;
                let progress = if player.length_secs > 0.0 {
                    player.position_secs / player.length_secs
                } else {
                    0.0
                };
                let (_, seek) = slider_bar(ui, theme, seek_width.max(0.0), progress);
                pb.seek_secs = seek.map(|fraction| fraction * player.length_secs);
                time(ui, player.length_secs);
            });
        });

    event
}
