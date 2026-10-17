use egui::{Color32, FontId, Response, Sense, Vec2};

use crate::fonts;
use crate::theme::Theme;

/// Lucide glyphs (the bundled icon font, version pinned in assets/fonts/lucide).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Play,
    Pause,
    Prev,
    Next,
    Up,
    Down,
    Close,
    Shuffle,
    Repeat,
    RepeatOne,
    /// Switch to the mini player.
    Mini,
    /// Switch back from the mini player.
    Full,
    Settings,
    About,
    Minimize,
    Maximize,
    /// Keep the window on top (shown while it is).
    Pin,
    PinOff,
}

impl Icon {
    pub fn glyph(self) -> char {
        match self {
            Icon::Play => '\u{e13c}',
            Icon::Pause => '\u{e12e}',
            Icon::Prev => '\u{e15f}',
            Icon::Next => '\u{e160}',
            Icon::Up => '\u{e070}',
            Icon::Down => '\u{e06d}',
            Icon::Close => '\u{e1b2}',
            Icon::Shuffle => '\u{e15e}',
            Icon::Repeat => '\u{e146}',
            Icon::RepeatOne => '\u{e1fd}',
            Icon::Mini => '\u{e3af}',
            Icon::Full => '\u{e113}',
            Icon::Settings => '\u{e154}',
            Icon::About => '\u{e0f9}',
            Icon::Minimize => '\u{e11c}',
            Icon::Maximize => '\u{e167}',
            Icon::Pin => '\u{e259}',
            Icon::PinOff => '\u{e2b6}',
        }
    }
}

/// How large a glyph is drawn inside a button of the given size.
const GLYPH_RATIO: f32 = 0.55;

/// Paints `icon` centered in `rect`.
pub fn paint_icon(ui: &egui::Ui, icon: Icon, rect: egui::Rect, size: f32, color: Color32) {
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        icon.glyph(),
        FontId::new(size, fonts::icon_family()),
        color,
    );
}

/// Round icon button.
pub fn icon_button(ui: &mut egui::Ui, theme: &Theme, icon: Icon, size: f32) -> Response {
    icon_toggle(ui, theme, icon, size, false)
}

/// Round icon button that is tinted with the accent color while `active` (shuffle, repeat).
pub fn icon_toggle(
    ui: &mut egui::Ui,
    theme: &Theme,
    icon: Icon,
    size: f32,
    active: bool,
) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let hover = ui.ctx().animate_bool_with_time(
        response.id,
        response.hovered(),
        theme.motion.hover_seconds,
    );
    let c = &theme.colors;
    ui.painter().circle_filled(
        rect.center(),
        size / 2.0,
        Color32::TRANSPARENT.lerp_to_gamma(c.surface_hover, hover),
    );
    let color = if active { c.accent } else { c.text };
    paint_icon(ui, icon, rect, size * GLYPH_RATIO, color);
    response
}
