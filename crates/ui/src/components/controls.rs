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

/// Height of a row that holds a button. egui does not re-center what was placed before a taller
/// widget joins the row, so a row that mixes text and buttons is fixed at this height first,
/// or the text sits above the buttons' text.
pub fn fix_row_height(ui: &mut egui::Ui) -> f32 {
    let spacing = ui.spacing();
    let button_height =
        ui.text_style_height(&egui::TextStyle::Button) + 2.0 * spacing.button_padding.y;
    let height = button_height.max(spacing.interact_size.y);
    ui.set_min_height(height);
    height
}

/// Label and combo box placed in the current horizontal row, `width` wide in total (label included).
/// Several can share one row; nesting a `horizontal` per drop-down would misalign the later ones.
pub fn dropdown_inline(
    ui: &mut egui::Ui,
    id_salt: &str,
    label: &str,
    selected: &mut usize,
    options: &[String],
    width: Option<f32>,
) -> bool {
    let mut changed = false;
    fix_row_height(ui);
    // An empty label would still add a widget and a gap.
    let label_width = if label.is_empty() {
        0.0
    } else {
        ui.label(label).rect.width()
    };
    let mut combo = egui::ComboBox::from_id_salt(id_salt);
    if let Some(width) = width {
        // What is left of the width after the label and the gap, so the pair fills exactly `width`.
        combo = combo.width((width - label_width - ui.spacing().item_spacing.x).max(0.0));
    }
    combo
        .selected_text(options.get(*selected).map_or("", String::as_str))
        .show_ui(ui, |ui| {
            for (index, option) in options.iter().enumerate() {
                changed |= ui.selectable_value(selected, index, option).changed();
            }
        });
    changed
}

/// Thin horizontal bar for volume.
/// Returns the new value (0.0 to 1.0) continuously while the user presses or drags on it.
pub fn slider_bar(
    ui: &mut egui::Ui,
    theme: &Theme,
    width: f32,
    value: f32,
) -> (Response, Option<f32>) {
    bar(ui, theme, width, value, false)
}

/// Same bar for seeking. Dragging only moves the handle; the value is returned once, when the
/// user lets go (or clicks), because every seek restarts the decoder and a stream of them is audible.
pub fn seek_bar(
    ui: &mut egui::Ui,
    theme: &Theme,
    width: f32,
    value: f32,
) -> (Response, Option<f32>) {
    bar(ui, theme, width, value, true)
}

fn bar(
    ui: &mut egui::Ui,
    theme: &Theme,
    width: f32,
    value: f32,
    on_release: bool,
) -> (Response, Option<f32>) {
    let (rect, response) = ui.allocate_exact_size(
        vec2(width, theme.shape.padding * 1.5),
        Sense::click_and_drag(),
    );
    let at_pointer = response
        .interact_pointer_pos()
        .map(|pos| ((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0));
    // The last dragged position, since the release frame may carry no pointer position.
    let drag_key = response.id.with("dragged");
    if on_release && response.dragged() {
        ui.data_mut(|d| d.insert_temp(drag_key, at_pointer));
    }
    let shown = match at_pointer {
        Some(pointer) if on_release && response.dragged() => pointer,
        _ => value,
    };

    let c = &theme.colors;
    let track_height = theme.shape.spacing / 2.0;
    let track = Rect::from_center_size(rect.center(), vec2(rect.width(), track_height));
    let radius = track_height / 2.0;
    let value = shown.clamp(0.0, 1.0);

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

    let new_value = if !on_release {
        if response.is_pointer_button_down_on() || response.clicked() {
            at_pointer
        } else {
            None
        }
    } else if response.drag_stopped() {
        ui.data_mut(|d| d.remove_temp::<Option<f32>>(drag_key))
            .flatten()
            .or(at_pointer)
    } else if response.clicked() {
        at_pointer
    } else {
        None
    };
    (response, new_value)
}
