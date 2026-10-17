use crate::i18n::t;
use egui::{
    Align, CornerRadius, Event, Frame, Key, Layout, Margin, Modifiers, Response, Sense, vec2,
};

use crate::theme::Theme;

/// One settings line: label on the left, the control on the right.
pub fn setting_row(
    ui: &mut egui::Ui,
    theme: &Theme,
    label: &str,
    controls: impl FnOnce(&mut egui::Ui),
) {
    Frame::new()
        .fill(theme.colors.surface)
        .corner_radius(CornerRadius::same(theme.shape.radius_card))
        .inner_margin(Margin::symmetric(
            theme.shape.padding as i8,
            theme.shape.spacing as i8,
        ))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let height = super::controls::fix_row_height(ui);
                // The toggle is taller than a button.
                ui.set_min_height(height.max(theme.shape.spacing * 3.0));
                ui.label(label);
                ui.with_layout(Layout::right_to_left(Align::Center), controls);
            });
        });
}

/// lazer-style switch.
pub fn toggle(ui: &mut egui::Ui, theme: &Theme, on: &mut bool) -> Response {
    let size = vec2(theme.shape.spacing * 5.0, theme.shape.spacing * 3.0);
    let (rect, mut response) = ui.allocate_exact_size(size, Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }

    let c = &theme.colors;
    let t = ui
        .ctx()
        .animate_bool_with_time(response.id, *on, theme.motion.hover_seconds);
    let radius = rect.height() / 2.0;
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        radius,
        c.surface_hover.lerp_to_gamma(c.primary_light, t),
    );
    let knob_x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), t);
    painter.circle_filled(
        egui::pos2(knob_x, rect.center().y),
        radius - theme.shape.spacing / 4.0,
        c.text,
    );
    response
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyBindChange {
    Set(String),
    Clear,
}

/// Button that records the next key combination the user presses.
/// Escape cancels, Backspace / Delete or the Clear button clears the binding.
pub fn keybind(
    ui: &mut egui::Ui,
    theme: &Theme,
    current: Option<&str>,
    capturing: &mut bool,
) -> Option<KeyBindChange> {
    let text = if *capturing {
        t("common.press_a_key")
    } else {
        current.unwrap_or(t("common.unassigned"))
    };
    let color = if *capturing {
        theme.colors.accent
    } else if current.is_some() {
        theme.colors.text
    } else {
        theme.colors.text_disabled
    };
    if ui.button(egui::RichText::new(text).color(color)).clicked() {
        *capturing = !*capturing;
        return None;
    }
    if !*capturing {
        if current.is_some() && ui.button(t("common.clear")).clicked() {
            return Some(KeyBindChange::Clear);
        }
        return None;
    }

    let pressed = ui.input(|input| {
        input.events.iter().find_map(|event| match event {
            // A modifier pressed on its own is the start of a combination, not the key.
            Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } if !is_modifier(*key) => Some((*key, *modifiers)),
            _ => None,
        })
    });
    let (key, modifiers) = pressed?;
    match key {
        Key::Escape => {
            *capturing = false;
            None
        }
        Key::Backspace | Key::Delete => {
            *capturing = false;
            Some(KeyBindChange::Clear)
        }
        _ => format_binding(modifiers, key).map(|binding| {
            *capturing = false;
            KeyBindChange::Set(binding)
        }),
    }
}

fn is_modifier(key: Key) -> bool {
    matches!(
        key,
        Key::ShiftLeft
            | Key::ShiftRight
            | Key::ControlLeft
            | Key::ControlRight
            | Key::AltLeft
            | Key::AltRight
            | Key::SuperLeft
            | Key::SuperRight
    )
}

/// Text like `Ctrl+Shift+P`. A global hotkey needs a modifier so it cannot hijack normal typing;
/// function keys are the exception. Returns `None` for a bare letter or digit.
pub fn format_binding(modifiers: Modifiers, key: Key) -> Option<String> {
    let name = key.name();
    let is_function_key =
        name.len() > 1 && name.starts_with('F') && name[1..].parse::<u8>().is_ok();
    let has_modifier = modifiers.ctrl || modifiers.alt || modifiers.shift || modifiers.mac_cmd;
    if !has_modifier && !is_function_key {
        return None;
    }

    let mut parts = Vec::new();
    for (active, label) in [
        (modifiers.ctrl, "Ctrl"),
        (modifiers.mac_cmd, "Cmd"),
        (modifiers.alt, "Alt"),
        (modifiers.shift, "Shift"),
    ] {
        if active {
            parts.push(label);
        }
    }
    parts.push(name);
    Some(parts.join("+"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modifier_keys_alone_are_not_the_key_of_a_binding() {
        assert!(is_modifier(Key::ControlLeft));
        assert!(!is_modifier(Key::P));
    }

    #[test]
    fn binding_needs_a_modifier_except_function_keys() {
        assert_eq!(format_binding(Modifiers::NONE, Key::P), None);
        assert_eq!(
            format_binding(Modifiers::NONE, Key::F5),
            Some("F5".to_owned())
        );
        let ctrl_shift = Modifiers {
            ctrl: true,
            shift: true,
            ..Modifiers::NONE
        };
        assert_eq!(
            format_binding(ctrl_shift, Key::P),
            Some("Ctrl+Shift+P".to_owned())
        );
    }
}
