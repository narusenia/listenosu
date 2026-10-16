use egui::{Color32, Rect, Response, Sense, TextEdit, vec2};

use crate::theme::Theme;

/// Search field with a match counter underneath.
pub fn search_bar(ui: &mut egui::Ui, theme: &Theme, text: &mut String, matches: usize) -> Response {
    let margin = vec2(theme.shape.padding, theme.shape.padding * 0.75);
    let response = ui.add(
        TextEdit::singleline(text)
            .hint_text("Search")
            .desired_width(ui.available_width())
            .margin(margin),
    );
    ui.label(
        egui::RichText::new(format!("{matches} match"))
            .size(theme.text.small)
            .color(theme.colors.accent),
    );
    response
}

/// Labeled drop-down. Returns true when the selection changed.
pub fn dropdown(
    ui: &mut egui::Ui,
    id_salt: &str,
    label: &str,
    selected: &mut usize,
    options: &[String],
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(label);
        egui::ComboBox::from_id_salt(id_salt)
            .selected_text(options.get(*selected).map_or("", String::as_str))
            .show_ui(ui, |ui| {
                for (index, option) in options.iter().enumerate() {
                    changed |= ui.selectable_value(selected, index, option).changed();
                }
            });
    });
    changed
}

/// Thin horizontal bar for seeking and volume.
/// Returns the new value (0.0 to 1.0) while the user presses or drags on it.
pub fn slider_bar(
    ui: &mut egui::Ui,
    theme: &Theme,
    width: f32,
    value: f32,
) -> (Response, Option<f32>) {
    let (rect, response) = ui.allocate_exact_size(
        vec2(width, theme.shape.padding * 1.5),
        Sense::click_and_drag(),
    );
    let c = &theme.colors;
    let track_height = theme.shape.spacing / 2.0;
    let track = Rect::from_center_size(rect.center(), vec2(rect.width(), track_height));
    let radius = track_height / 2.0;
    let value = value.clamp(0.0, 1.0);

    let painter = ui.painter();
    painter.rect_filled(track, radius, c.surface_hover);
    let filled = Rect::from_min_size(track.min, vec2(track.width() * value, track.height()));
    painter.rect_filled(filled, radius, c.primary_light);

    let active = response.hovered() || response.dragged();
    let grow = ui
        .ctx()
        .animate_bool_with_time(response.id, active, theme.motion.hover_seconds);
    if grow > 0.0 {
        let center = egui::pos2(filled.right(), rect.center().y);
        let handle: Color32 = c.text.gamma_multiply(grow);
        ui.painter()
            .circle_filled(center, track_height * 1.5, handle);
    }

    let new_value = if response.is_pointer_button_down_on() || response.clicked() {
        response
            .interact_pointer_pos()
            .map(|pos| ((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0))
    } else {
        None
    };
    (response, new_value)
}
