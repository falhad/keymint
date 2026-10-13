use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::platform;
use crate::state::AppState;
use crate::window::{self, Trigger};

/// (Re)register the palette shortcut and every per-command shortcut.
/// Problems are collected for Settings instead of aborting startup.
pub fn register_all(app: &AppHandle) {
    let state = app.state::<AppState>();
    let gs = app.global_shortcut();
    let mut errors = Vec::new();

    if let Err(e) = gs.unregister_all() {
        errors.push(format!("Cannot clear old shortcuts: {e}"));
    }

    if platform::is_wayland() {
        // Registration would appear to succeed but never fire on Wayland.
        *state.shortcut_errors.lock().expect("lock") = errors;
        return;
    }

    let cfg = state.config();
    let mut bindings = vec![(cfg.shortcut.clone(), Trigger::Toggle)];
    bindings.extend(cfg.command_shortcuts.iter().map(|(id, accel)| (accel.clone(), Trigger::Run(id.clone()))));

    for (accel, trig) in bindings {
        if accel.trim().is_empty() {
            continue;
        }
        let result = gs.on_shortcut(accel.as_str(), move |app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                window::trigger(app, trig.clone());
            }
        });
        if let Err(e) = result {
            errors.push(format!("Shortcut \"{accel}\" could not be registered: {e}"));
        }
    }
    *state.shortcut_errors.lock().expect("lock") = errors;
}
