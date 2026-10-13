//! Reading the selection of another app and writing text back into it.
//!
//! - macOS / Windows / X11: simulate the copy / paste keystroke and borrow the
//!   clipboard, restoring it afterwards.
//! - Wayland: apps cannot inject keystrokes or watch other apps, so we read the
//!   PRIMARY selection (what is highlighted) and paste with `wtype`/`ydotool`
//!   when installed; otherwise the text is left on the clipboard.

mod wayland;

use std::thread::sleep;
use std::time::{Duration, Instant};

use arboard::Clipboard;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use serde::Serialize;

use crate::platform;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ReplaceOutcome {
    /// The text was pasted into the previously focused app.
    Pasted,
    /// Pasting is not possible here; the text is on the clipboard.
    Copied,
}

const MODIFIER: Key = if cfg!(target_os = "macos") { Key::Meta } else { Key::Control };

/// Put text on the clipboard.
pub fn copy_text(text: &str) -> Result<(), String> {
    Clipboard::new()
        .and_then(|mut c| c.set_text(text.to_string()))
        .map_err(|e| format!("Cannot access the clipboard: {e}"))
}

/// Selected text in the app that currently has focus, if any. Blocking.
pub fn capture() -> Option<String> {
    let text = if platform::is_wayland() { wayland::read_primary() } else { capture_by_copy() };
    text.filter(|t| !t.trim().is_empty())
}

fn press_shortcut(letter: char) -> Result<(), String> {
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
    enigo.key(MODIFIER, Direction::Press).map_err(|e| e.to_string())?;
    let click = enigo.key(Key::Unicode(letter), Direction::Click);
    // Always release the modifier, even if the click failed.
    let release = enigo.key(MODIFIER, Direction::Release);
    click.and(release).map_err(|e| e.to_string())
}

/// Snapshot of the clipboard so we can put it back after borrowing it.
struct ClipboardBackup {
    text: Option<String>,
    image: Option<arboard::ImageData<'static>>,
}

impl ClipboardBackup {
    fn take(cb: &mut Clipboard) -> Self {
        Self { text: cb.get_text().ok(), image: cb.get_image().ok().map(|i| i.to_owned_img()) }
    }

    fn restore(self, cb: &mut Clipboard) {
        let result = match (self.text, self.image) {
            (Some(t), _) => cb.set_text(t),
            (None, Some(i)) => cb.set_image(i),
            (None, None) => cb.clear(),
        };
        if let Err(e) = result {
            eprintln!("keymint: could not restore the clipboard: {e}");
        }
    }
}

fn capture_by_copy() -> Option<String> {
    let mut cb = Clipboard::new().ok()?;
    let backup = ClipboardBackup::take(&mut cb);

    // A sentinel tells us whether the copy really happened, even if the
    // selection equals what was already on the clipboard.
    let sentinel = format!("\u{200b}keymint-{:?}", Instant::now());
    cb.set_text(sentinel.clone()).ok()?;

    // Let the user release the shortcut keys first so they do not combine with ours.
    sleep(Duration::from_millis(120));
    let selected = match press_shortcut('c') {
        Ok(()) => {
            let deadline = Instant::now() + Duration::from_millis(400);
            loop {
                match cb.get_text() {
                    Ok(t) if t != sentinel => break Some(t),
                    _ if Instant::now() >= deadline => break None,
                    _ => sleep(Duration::from_millis(20)),
                }
            }
        }
        Err(e) => {
            eprintln!("keymint: cannot simulate copy: {e}");
            None
        }
    };
    backup.restore(&mut cb);
    selected
}

/// Replace the selection in the previously focused app. The caller must have
/// hidden the palette so focus is back in the target app. Blocking.
pub fn replace(text: &str) -> Result<ReplaceOutcome, String> {
    if platform::is_wayland() {
        // Best effort: the frontend normally already placed the text on the clipboard.
        let _ = copy_text(text);
        if wayland::paste() {
            return Ok(ReplaceOutcome::Pasted);
        }
        // Last resort: synthetic keys reach X11/XWayland windows (e.g. JetBrains IDEs) even on Wayland.
        // We cannot tell whether a native Wayland window received them; the text stays on the clipboard either way.
        return Ok(match press_shortcut('v') {
            Ok(()) => ReplaceOutcome::Pasted,
            Err(_) => ReplaceOutcome::Copied,
        });
    }

    if !platform::accessibility_trusted() {
        copy_text(text)?;
        return Ok(ReplaceOutcome::Copied);
    }

    let mut cb = Clipboard::new().map_err(|e| format!("Cannot access the clipboard: {e}"))?;
    let backup = ClipboardBackup::take(&mut cb);
    cb.set_text(text.to_string()).map_err(|e| e.to_string())?;
    sleep(Duration::from_millis(60));
    let pasted = press_shortcut('v');
    // The target app reads the clipboard asynchronously; wait before restoring.
    sleep(Duration::from_millis(250));
    match pasted {
        Ok(()) => {
            backup.restore(&mut cb);
            Ok(ReplaceOutcome::Pasted)
        }
        // Leave our text on the clipboard so the user can paste it manually.
        Err(_) => Ok(ReplaceOutcome::Copied),
    }
}
