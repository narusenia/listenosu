use crate::i18n::t;
use std::sync::Arc;

use egui::text::{LayoutJob, TextWrapping};
use egui::{Color32, CornerRadius, FontId, Galley, Rect, Sense, Stroke, StrokeKind, vec2};

use crate::fonts;
use crate::model::SongView;
use crate::theme::Theme;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CardState {
    pub selected: bool,
    pub playing: bool,
}

/// One row of the song list. Returns the click response of the whole card.
pub fn song_card(
    ui: &mut egui::Ui,
    theme: &Theme,
    song: &SongView,
    state: CardState,
) -> egui::Response {
    let size = vec2(ui.available_width(), theme.shape.card_height);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    if !ui.is_rect_visible(rect) {
        return response;
    }

    let c = &theme.colors;
    let hover = ui.ctx().animate_bool_with_time(
        response.id,
        response.hovered(),
        theme.motion.hover_seconds,
    );
    let fill = if state.selected {
        c.surface_selected
    } else {
        c.surface.lerp_to_gamma(c.surface_hover, hover)
    };
    let radius = CornerRadius::same(theme.shape.radius_card);
    let painter = ui.painter();
    painter.rect_filled(rect, radius, fill);
    if state.selected {
        painter.rect_stroke(
            rect,
            radius,
            Stroke::new(theme.shape.spacing / 4.0, c.text),
            StrokeKind::Inside,
        );
    }

    let pad = theme.shape.padding;
    let bar_width = theme.shape.spacing / 2.0;
    if state.playing {
        let bar = Rect::from_min_max(
            rect.left_top() + vec2(0.0, pad),
            rect.left_bottom() + vec2(bar_width, -pad),
        );
        painter.rect_filled(bar, CornerRadius::same(theme.shape.radius_button), c.accent);
    }

    let (title_color, dim_color) = if song.missing {
        (c.text_disabled, c.text_disabled)
    } else {
        (c.text, c.text_dim)
    };
    let small = FontId::proportional(theme.text.small);
    let meta_text = if song.missing {
        t("common.file_not_found").to_owned()
    } else {
        format!(
            "{:.0} BPM · {}:{:02}",
            song.bpm,
            song.length_secs / 60,
            song.length_secs % 60
        )
    };
    let meta_color = if song.missing { c.warning } else { dim_color };
    let meta = galley(
        painter,
        &meta_text,
        small.clone(),
        meta_color,
        f32::INFINITY,
    );

    let left = rect.left() + pad + bar_width;
    let text_width = rect.right() - pad - meta.size().x - theme.shape.spacing - left;
    let title = galley(
        painter,
        &song.title,
        fonts::bold(theme.text.body),
        title_color,
        text_width,
    );
    let artist = galley(painter, &song.artist, small, dim_color, text_width);

    let top = rect.top() + (rect.height() - title.size().y - artist.size().y) / 2.0;
    let artist_top = top + title.size().y;
    painter.galley(egui::pos2(left, top), title, title_color);
    painter.galley(egui::pos2(left, artist_top), artist, dim_color);
    painter.galley(
        egui::pos2(
            rect.right() - pad - meta.size().x,
            rect.center().y - meta.size().y / 2.0,
        ),
        meta,
        meta_color,
    );

    response
}

/// Lays out a single line, truncated with an ellipsis at `max_width`.
fn galley(
    painter: &egui::Painter,
    text: &str,
    font: FontId,
    color: Color32,
    max_width: f32,
) -> Arc<Galley> {
    let mut job = LayoutJob::simple_singleline(text.to_owned(), font, color);
    job.wrap = TextWrapping::truncate_at_width(max_width);
    painter.layout_job(job)
}
