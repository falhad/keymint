use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Session {
    Wayland,
    X11,
    /// macOS or Windows.
    Native,
}

pub fn session() -> Session {
    if cfg!(target_os = "linux") {
        let wayland = std::env::var("XDG_SESSION_TYPE").map(|v| v.eq_ignore_ascii_case("wayland")).unwrap_or(false)
            || std::env::var_os("WAYLAND_DISPLAY").is_some();
        if wayland {
            Session::Wayland
        } else {
            Session::X11
        }
    } else {
        Session::Native
    }
}

pub fn is_wayland() -> bool {
    session() == Session::Wayland
}

pub fn os() -> &'static str {
    if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "linux"
    }
}

pub fn which(program: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|dir| dir.join(program).is_file()))
        .unwrap_or(false)
}

#[cfg(target_os = "macos")]
pub fn accessibility_trusted() -> bool {
    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrusted() -> bool;
    }
    // SAFETY: plain C call with no arguments or side effects.
    unsafe { AXIsProcessTrusted() }
}

#[cfg(not(target_os = "macos"))]
pub fn accessibility_trusted() -> bool {
    true
}

/// What this machine can do, so the UI can label buttons honestly.
#[derive(Debug, Clone, Serialize)]
pub struct PlatformInfo {
    pub os: &'static str,
    pub session: Session,
    /// A global hotkey can be registered by the app itself.
    pub global_shortcut: bool,
    /// Selected text is captured automatically when the palette opens.
    pub auto_capture: bool,
    /// "Replace Selected Text" can paste into the other app by itself.
    pub auto_replace: bool,
    /// Human-readable limitations and setup hints.
    pub notes: Vec<String>,
}

pub fn info(shortcut_errors: &[String]) -> PlatformInfo {
    let session = session();
    let mut notes: Vec<String> = Vec::new();
    let (global_shortcut, auto_capture, auto_replace) = match session {
        Session::Wayland => {
            notes.push(
                "Wayland does not let apps register global shortcuts. Bind a keyboard shortcut in your \
                 desktop settings to the command `keymint --toggle` (see the README)."
                    .into(),
            );
            if !(which("wtype") || which("ydotool")) {
                notes.push(
                    "Replace pastes into X11/XWayland apps (such as JetBrains IDEs). For native Wayland apps \
                     the result is copied and you press Ctrl+V; install `wtype` or `ydotool` to automate that."
                        .into(),
                );
            }
            if !which("wl-paste") {
                notes.push("Install `wl-clipboard` to improve selected-text capture on Wayland.".into());
            }
            (false, true, true)
        }
        _ => (true, true, true),
    };
    if !accessibility_trusted() {
        notes.push(
            "Grant Keymint Accessibility access (System Settings → Privacy & Security → Accessibility) \
             to capture and replace selected text."
                .into(),
        );
    }
    notes.extend(shortcut_errors.iter().cloned());
    PlatformInfo {
        os: os(),
        session,
        global_shortcut,
        auto_capture: auto_capture && accessibility_trusted(),
        auto_replace: auto_replace && accessibility_trusted(),
        notes,
    }
}
