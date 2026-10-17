use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::media::Command;

const ICON_SIZE: u32 = 32;
/// The theme's primary color; the tray has no `Theme`, so the one value is copied here.
const ICON_RGB: [u8; 3] = [0x59, 0x33, 0xCC];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TrayEvent {
    /// Bring the window back.
    Show,
    Quit,
    Command(Command),
}

/// A tray icon with a small menu. Events arrive on the UI thread's message loop, so the handler
/// must only forward them (send a viewport command, push to a channel) and return.
pub struct Tray {
    _icon: TrayIcon,
}

impl Tray {
    pub fn new(handler: impl Fn(TrayEvent) + Send + Sync + 'static) -> Result<Self, String> {
        let show = MenuItem::new("Show", true, None);
        let toggle = MenuItem::new("Play / Pause", true, None);
        let next = MenuItem::new("Next", true, None);
        let previous = MenuItem::new("Previous", true, None);
        let quit = MenuItem::new("Quit", true, None);
        let menu = Menu::new();
        menu.append_items(&[
            &show,
            &PredefinedMenuItem::separator(),
            &toggle,
            &next,
            &previous,
            &PredefinedMenuItem::separator(),
            &quit,
        ])
        .map_err(|e| e.to_string())?;

        let handler = std::sync::Arc::new(handler);
        let on_click = handler.clone();
        TrayIconEvent::set_event_handler(Some(move |event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                on_click(TrayEvent::Show);
            }
        }));
        let ids = [
            (show.id().clone(), TrayEvent::Show),
            (toggle.id().clone(), TrayEvent::Command(Command::Toggle)),
            (next.id().clone(), TrayEvent::Command(Command::Next)),
            (previous.id().clone(), TrayEvent::Command(Command::Previous)),
            (quit.id().clone(), TrayEvent::Quit),
        ];
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            if let Some((_, tray_event)) = ids.iter().find(|(id, _)| *id == event.id) {
                handler(*tray_event);
            }
        }));

        let icon = TrayIconBuilder::new()
            .with_icon(icon()?)
            .with_tooltip("listenosu!")
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self { _icon: icon })
    }
}

/// A filled circle in the primary color, with soft edges.
fn icon() -> Result<Icon, String> {
    let n = ICON_SIZE as f32;
    let mut rgba = Vec::with_capacity((ICON_SIZE * ICON_SIZE * 4) as usize);
    for y in 0..ICON_SIZE {
        for x in 0..ICON_SIZE {
            let (dx, dy) = (x as f32 + 0.5 - n / 2.0, y as f32 + 0.5 - n / 2.0);
            let edge = (n / 2.0 - 1.0 - (dx * dx + dy * dy).sqrt()).clamp(0.0, 1.0);
            rgba.extend_from_slice(&[ICON_RGB[0], ICON_RGB[1], ICON_RGB[2], (edge * 255.0) as u8]);
        }
    }
    Icon::from_rgba(rgba, ICON_SIZE, ICON_SIZE).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_icon_builds() {
        assert!(super::icon().is_ok());
    }
}
