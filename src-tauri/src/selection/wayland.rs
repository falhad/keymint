//! Wayland-specific selection helpers. On non-Linux targets everything here is a no-op.

use std::io::Read;
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

use crate::platform::which;

/// Run a program and return its stdout if it exits successfully within `timeout`.
fn run_with_timeout(mut cmd: Command, timeout: Duration) -> Option<String> {
    let mut child = cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().ok()?;
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return None;
                }
                let mut out = String::new();
                child.stdout.take()?.read_to_string(&mut out).ok()?;
                return Some(out);
            }
            Ok(None) if Instant::now() < deadline => sleep(Duration::from_millis(15)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
}

/// The highlighted text (PRIMARY selection).
#[cfg(target_os = "linux")]
pub fn read_primary() -> Option<String> {
    use arboard::{Clipboard, GetExtLinux, LinuxClipboardKind};

    let via_arboard = Clipboard::new()
        .ok()
        .and_then(|mut cb| cb.get().clipboard(LinuxClipboardKind::Primary).text().ok());
    if via_arboard.as_deref().is_some_and(|t| !t.trim().is_empty()) {
        return via_arboard;
    }
    if which("wl-paste") {
        let mut cmd = Command::new("wl-paste");
        cmd.args(["--primary", "--no-newline"]);
        return run_with_timeout(cmd, Duration::from_millis(600));
    }
    None
}

#[cfg(not(target_os = "linux"))]
pub fn read_primary() -> Option<String> {
    None
}

/// Press Ctrl+V in the focused app using whichever tool is installed.
pub fn paste() -> bool {
    // Give the compositor time to return focus to the previous window.
    sleep(Duration::from_millis(250));
    let timeout = Duration::from_secs(2);
    if which("wtype") {
        let mut cmd = Command::new("wtype");
        cmd.args(["-M", "ctrl", "-k", "v", "-m", "ctrl"]);
        return run_with_timeout(cmd, timeout).is_some();
    }
    if which("ydotool") {
        // Linux input codes: 29 = left ctrl, 47 = v.
        let mut cmd = Command::new("ydotool");
        cmd.args(["key", "29:1", "47:1", "47:0", "29:0"]);
        return run_with_timeout(cmd, timeout).is_some();
    }
    false
}
