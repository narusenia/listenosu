use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

use crate::media::Command;

/// System-wide hotkeys. None is registered until `set` is given bindings.
pub struct Hotkey {
    manager: GlobalHotKeyManager,
    registered: Vec<HotKey>,
    commands: Arc<Mutex<HashMap<u32, Command>>>,
}

impl Hotkey {
    /// `on_press` runs on the UI thread's message loop, so it must only forward the event.
    pub fn new(on_press: impl Fn(Command) + Send + Sync + 'static) -> Result<Self, String> {
        let manager = GlobalHotKeyManager::new().map_err(|e| e.to_string())?;
        let commands: Arc<Mutex<HashMap<u32, Command>>> = Arc::default();
        let lookup = commands.clone();
        GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
            if event.state != HotKeyState::Pressed {
                return;
            }
            let command = lookup
                .lock()
                .ok()
                .and_then(|map| map.get(&event.id).copied());
            if let Some(command) = command {
                on_press(command);
            }
        }));
        Ok(Self {
            manager,
            registered: Vec::new(),
            commands,
        })
    }

    /// Replaces every hotkey. A binding looks like `Ctrl+Shift+P`; None skips that command.
    /// A binding that fails (another app may own the combination) stays unregistered and the
    /// others still register; the first error is returned.
    pub fn set(&mut self, bindings: &[(Option<&str>, Command)]) -> Result<(), String> {
        for old in self.registered.drain(..) {
            let _ = self.manager.unregister(old);
        }
        let mut map = self.commands.lock().map_err(|e| e.to_string())?;
        map.clear();
        let mut first_error = None;
        for (binding, command) in bindings {
            let Some(binding) = binding else { continue };
            let result = binding
                .parse::<HotKey>()
                .map_err(|e| format!("{e}"))
                .and_then(|hotkey| {
                    self.manager.register(hotkey).map_err(|e| e.to_string())?;
                    Ok(hotkey)
                });
            match result {
                Ok(hotkey) => {
                    map.insert(hotkey.id(), *command);
                    self.registered.push(hotkey);
                }
                Err(e) => {
                    first_error.get_or_insert(e);
                }
            }
        }
        first_error.map_or(Ok(()), Err)
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
