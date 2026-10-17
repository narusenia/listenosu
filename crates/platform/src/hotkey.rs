use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

/// One system-wide hotkey. It is off until `set` is given a binding.
pub struct Hotkey {
    manager: GlobalHotKeyManager,
    registered: Option<HotKey>,
}

impl Hotkey {
    /// `on_press` runs on the UI thread's message loop, so it must only forward the event.
    pub fn new(on_press: impl Fn() + Send + Sync + 'static) -> Result<Self, String> {
        let manager = GlobalHotKeyManager::new().map_err(|e| e.to_string())?;
        GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
            if event.state == HotKeyState::Pressed {
                on_press();
            }
        }));
        Ok(Self {
            manager,
            registered: None,
        })
    }

    /// Replaces the current hotkey. `binding` looks like `Ctrl+Shift+P`; None turns it off.
    /// On error nothing stays registered (another app may own the combination).
    pub fn set(&mut self, binding: Option<&str>) -> Result<(), String> {
        if let Some(old) = self.registered.take() {
            let _ = self.manager.unregister(old);
        }
        let Some(binding) = binding else {
            return Ok(());
        };
        let hotkey: HotKey = binding.parse().map_err(|e| format!("{e}"))?;
        self.manager.register(hotkey).map_err(|e| e.to_string())?;
        self.registered = Some(hotkey);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use global_hotkey::hotkey::HotKey;

    /// The settings screen writes bindings in this form.
    #[test]
    fn the_settings_format_parses() {
        for binding in ["Ctrl+Shift+P", "F5", "Alt+ArrowUp", "Ctrl+Num1"] {
            assert!(binding.parse::<HotKey>().is_ok(), "{binding}");
        }
    }
}
