use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::selection;
use crate::state::AppState;

pub const PALETTE: &str = "palette";
pub const SETTINGS: &str = "settings";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Trigger {
    /// Show the palette, or hide it if already visible.
    Toggle,
    Show,
    /// Show the palette and start this command right away.
    Run(String),
    Settings,
}

#[derive(Clone, Serialize)]
struct OpenPayload {
    selection: Option<String>,
    run: Option<String>,
}

/// Single entry point for global shortcuts, the tray and `keymint --…` calls.
pub fn trigger(app: &AppHandle, trigger: Trigger) {
    if trigger == Trigger::Settings {
        open_settings(app);
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let palette = app.get_webview_window(PALETTE);
        let visible = palette.as_ref().and_then(|w| w.is_visible().ok()).unwrap_or(false);
        if visible && trigger == Trigger::Toggle {
            hide_palette(&app);
            return;
        }
        // Capture before showing: once the palette has focus the other app's selection is out of reach.
        let selection = if visible {
            None
        } else {
            tauri::async_runtime::spawn_blocking(selection::capture).await.ok().flatten()
        };
        show_palette(&app);
        let run = if let Trigger::Run(id) = trigger { Some(id) } else { None };
        if let Err(e) = app.emit_to(PALETTE, "palette:open", OpenPayload { selection, run }) {
            eprintln!("keymint: cannot notify the palette: {e}");
        }
    });
}

pub fn show_palette(app: &AppHandle) {
    let Some(w) = app.get_webview_window(PALETTE) else { return };
    if let Some(state) = app.try_state::<AppState>() {
        state.mark_shown();
    }
    let _ = w.center();
    let _ = w.show();
    let _ = w.set_focus();
}

pub fn hide_palette(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(PALETTE) {
        let _ = w.hide();
    }
}

pub fn open_settings(app: &AppHandle) {
    hide_palette(app);
    if let Some(w) = app.get_webview_window(SETTINGS) {
        let _ = w.show();
        let _ = w.set_focus();
        return;
    }
    // Created on demand so the idle app only keeps the small palette webview alive.
    let built = WebviewWindowBuilder::new(app, SETTINGS, WebviewUrl::App("settings.html".into()))
        .title("Keymint Settings")
        .inner_size(860.0, 640.0)
        .min_inner_size(680.0, 480.0)
        .center()
        .build();
    if let Err(e) = built {
        eprintln!("keymint: cannot open settings: {e}");
    }
}
