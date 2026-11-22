//! Keyboard input through the XDG RemoteDesktop portal (Linux).
//!
//! XTest keystrokes sent through XWayland make GNOME ask "Allow Remote Interaction"
//! for every new connection. Here we ask the portal once with
//! `PersistMode::ExplicitlyRevoked` and keep the returned restore token on disk,
//! so later launches reuse the permission without a prompt.

use std::path::PathBuf;
use std::sync::Mutex;

use ashpd::desktop::remote_desktop::{
    DeviceType, KeyState, NotifyKeyboardKeycodeOptions, RemoteDesktop,
    SelectDevicesOptions, StartOptions,
};
use ashpd::desktop::{CreateSessionOptions, PersistMode, Session};
use ashpd::zbus::block_on;

/// Linux input event codes.
const KEY_LEFTCTRL: i32 = 29;
const KEY_C: i32 = 46;
const KEY_V: i32 = 47;

struct Portal {
    remote: RemoteDesktop,
    session: Session<RemoteDesktop>,
}

static PORTAL: Mutex<Option<Portal>> = Mutex::new(None);

fn token_path() -> Result<PathBuf, String> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .ok_or("cannot find the config directory")?;
    Ok(base.join("keymint").join("remote-desktop-token"))
}

fn connect() -> Result<Portal, String> {
    let path = token_path()?;
    let saved = std::fs::read_to_string(&path).ok().map(|t| t.trim().to_string()).filter(|t| !t.is_empty());
    block_on(async {
        let remote = RemoteDesktop::new().await.map_err(|e| e.to_string())?;
        let session = remote.create_session(CreateSessionOptions::default()).await.map_err(|e| e.to_string())?;
        let options = SelectDevicesOptions::default()
            .set_devices(DeviceType::KEYBOARD)
            .set_persist_mode(PersistMode::ExplicitlyRevoked)
            .set_restore_token(saved.as_deref());
        remote.select_devices(&session, options).await.map_err(|e| e.to_string())?;
        let started = remote
            .start(&session, None, StartOptions::default())
            .await
            .map_err(|e| e.to_string())?
            .response()
            .map_err(|e| e.to_string())?;
        if let Some(token) = started.restore_token() {
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir).map_err(|e| format!("cannot save the permission: {e}"))?;
            }
            std::fs::write(&path, token).map_err(|e| format!("cannot save the permission: {e}"))?;
        }
        Ok(Portal { remote, session })
    })
}

fn send(portal: &Portal, keys: &[(i32, KeyState)]) -> Result<(), String> {
    block_on(async {
        for (code, state) in keys {
            portal
                .remote
                .notify_keyboard_keycode(&portal.session, *code, *state, NotifyKeyboardKeycodeOptions::default())
                .await
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    })
}

/// Press Ctrl+`letter` ('c' or 'v'). Reconnects once if the session died.
pub fn press_shortcut(letter: char) -> Result<(), String> {
    let key = match letter {
        'c' => KEY_C,
        'v' => KEY_V,
        other => return Err(format!("unsupported shortcut key '{other}'")),
    };
    let mut guard = PORTAL.lock().map_err(|_| "portal lock is poisoned".to_string())?;
    let keys = [
        (KEY_LEFTCTRL, KeyState::Pressed),
        (key, KeyState::Pressed),
        (key, KeyState::Released),
        (KEY_LEFTCTRL, KeyState::Released),
    ];
    if let Some(portal) = guard.as_ref() {
        if send(portal, &keys).is_ok() {
            return Ok(());
        }
        // Session is gone or the permission was revoked: start over.
        *guard = None;
    }
    let portal = connect()?;
    let result = send(&portal, &keys);
    *guard = Some(portal);
    result
}
