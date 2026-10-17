use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::media::Command;

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
    /// `icon` is straight RGBA pixels, `width` x `height`.
    pub fn new(
        icon: (Vec<u8>, u32, u32),
        handler: impl Fn(TrayEvent) + Send + Sync + 'static,
    ) -> Result<Self, String> {
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
            .with_icon(Icon::from_rgba(icon.0, icon.1, icon.2).map_err(|e| e.to_string())?)
            .with_tooltip("listenosu!")
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self { _icon: icon })
    }
}
