//! IPC surface. These functions only translate between the webview and
//! `keymint-core` / OS integration; there is no business logic here.

use std::sync::Arc;

use keymint_core::ai::{prepare_chat, ChatMessage, Progress, ProgressSink, ProviderStatus};
use keymint_core::config::{Config, ProviderKind};
use keymint_core::history::HistoryEntry;
use keymint_core::registry::{
    Action, CommandContext, CommandInput, CommandManifest, CommandOutput, Registry, SlashResult,
};
use keymint_core::Error;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_opener::OpenerExt as _;

#[derive(Clone, serde::Serialize)]
struct ProgressEvent {
    run_id: u64,
    event: Progress,
}

use crate::platform::{self, PlatformInfo};
use crate::selection::{self, ReplaceOutcome};
use crate::state::AppState;
use crate::{shortcuts, window};

type CmdResult<T> = Result<T, Error>;

const SEARCH_LIMIT: usize = 50;

#[tauri::command]
pub fn search(state: State<'_, AppState>, query: String) -> Vec<CommandManifest> {
    let usage = state.usage.lock().expect("usage lock");
    state.registry().search(&query, &usage, SEARCH_LIMIT)
}

/// `/` queries: commands by slash name, plus any text typed after the name.
#[tauri::command]
pub fn search_slash(state: State<'_, AppState>, query: String) -> SlashResult {
    state.registry().search_slash(&query)
}

/// Every command, for the shortcuts editor.
#[tauri::command]
pub fn list_commands(state: State<'_, AppState>) -> Vec<CommandManifest> {
    state.registry().manifests().to_vec()
}

#[tauri::command]
pub async fn execute(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    input: CommandInput,
    run_id: u64,
) -> CmdResult<CommandOutput> {
    let registry = state.registry();
    let command = registry.get(&id)?;
    let title = command.manifest().title.clone();
    let input_text = input.text.clone().unwrap_or_default();

    // Stream what the AI side is doing to the palette, tagged so stale runs can be ignored.
    let emitter = app.clone();
    let progress: ProgressSink = Arc::new(move |event| {
        let _ = emitter.emit_to(window::PALETTE, "command:progress", ProgressEvent { run_id, event });
    });
    let config = state.config();
    let history_limit = config.history_limit as usize;
    let ctx = CommandContext { config, ai: state.ai.clone(), progress };
    let output = command.run(&ctx, input).await?;

    if let CommandOutput::Text { text } = &output {
        let snapshot = {
            let mut history = state.history.lock().expect("history lock");
            history.add(&id, &title, &input_text, text, history_limit);
            history.clone()
        };
        if let Err(e) = snapshot.save(&state.history_path) {
            eprintln!("keymint: cannot save history: {e}");
        }
    }

    let snapshot = {
        let mut usage = state.usage.lock().expect("usage lock");
        usage.record(&id);
        usage.clone()
    };
    if let Err(e) = snapshot.save(&state.usage_path) {
        eprintln!("keymint: cannot save usage history: {e}");
    }

    if let CommandOutput::Action { action } = &output {
        perform(&app, action)?;
    }
    Ok(output)
}

fn perform(app: &AppHandle, action: &Action) -> CmdResult<()> {
    match action {
        Action::OpenSettings => window::open_settings(app),
        // The palette itself switches to its history view.
        Action::ShowHistory | Action::OpenChat { .. } => {}
        Action::Quit => app.exit(0),
        Action::OpenUrl { url } => app
            .opener()
            .open_url(url, None::<&str>)
            .map_err(|e| Error::Process(format!("cannot open {url}: {e}")))?,
    }
    Ok(())
}

/// One turn of the Ask AI chat. The conversation lives in the palette; this
/// sends it to the default provider and streams progress like any command.
#[tauri::command]
pub async fn chat(
    app: AppHandle,
    state: State<'_, AppState>,
    messages: Vec<ChatMessage>,
    run_id: u64,
) -> CmdResult<String> {
    let messages = prepare_chat(messages)?;
    let question = messages.last().map(|m| m.content.clone()).unwrap_or_default();

    let emitter = app.clone();
    let progress: ProgressSink = Arc::new(move |event| {
        let _ = emitter.emit_to(window::PALETTE, "command:progress", ProgressEvent { run_id, event });
    });
    let config = state.config();
    let answer = state.ai.complete(&config, config.chat_system_prompt.clone(), messages, &progress).await?;

    let snapshot = {
        let mut history = state.history.lock().expect("history lock");
        history.add("ai.ask", "Ask AI", &question, &answer, config.history_limit as usize);
        history.clone()
    };
    if let Err(e) = snapshot.save(&state.history_path) {
        eprintln!("keymint: cannot save history: {e}");
    }
    Ok(answer)
}

#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> Config {
    state.config()
}

#[tauri::command]
pub fn save_config(app: AppHandle, state: State<'_, AppState>, config: Config) -> CmdResult<Config> {
    let mut cfg = config;
    cfg.normalize();
    cfg.validate()?;

    // Drop shortcuts that point at commands that no longer exist.
    let known = Registry::build(&state.providers, &cfg);
    cfg.command_shortcuts.retain(|id, _| known.manifest(id).is_some());

    cfg.save(&state.config_path)?;
    state.set_config(cfg.clone());
    {
        let mut history = state.history.lock().expect("history lock");
        history.truncate(cfg.history_limit as usize);
        if let Err(e) = history.save(&state.history_path) {
            eprintln!("keymint: cannot save history: {e}");
        }
    }
    state.warnings.lock().expect("lock").clear();

    apply_autostart(&app, &cfg);
    shortcuts::register_all(&app);
    if let Err(e) = app.emit("config:changed", &cfg) {
        eprintln!("keymint: cannot broadcast config change: {e}");
    }
    Ok(cfg)
}

fn apply_autostart(app: &AppHandle, cfg: &Config) {
    let manager = app.autolaunch();
    let enabled = manager.is_enabled().unwrap_or(false);
    let result = match (cfg.launch_at_startup, enabled) {
        (true, false) => manager.enable(),
        (false, true) => manager.disable(),
        _ => Ok(()),
    };
    if let Err(e) = result {
        eprintln!("keymint: cannot update launch-at-startup: {e}");
    }
}

pub fn sync_autostart(app: &AppHandle, cfg: &Config) {
    apply_autostart(app, cfg);
}

#[tauri::command]
pub async fn provider_status(state: State<'_, AppState>) -> CmdResult<Vec<ProviderStatus>> {
    let ai = state.ai.clone();
    tauri::async_runtime::spawn_blocking(move || ai.status())
        .await
        .map_err(|e| Error::Secret(e.to_string()))
}

#[tauri::command]
pub async fn set_api_key(state: State<'_, AppState>, provider: ProviderKind, key: String) -> CmdResult<()> {
    let key = key.trim().to_string();
    if key.is_empty() {
        return Err(Error::InvalidInput("The API key is empty".into()));
    }
    let ai = state.ai.clone();
    tauri::async_runtime::spawn_blocking(move || ai.secrets().set(provider, &key))
        .await
        .map_err(|e| Error::Secret(e.to_string()))?
}

#[tauri::command]
pub async fn delete_api_key(state: State<'_, AppState>, provider: ProviderKind) -> CmdResult<()> {
    let ai = state.ai.clone();
    tauri::async_runtime::spawn_blocking(move || ai.secrets().delete(provider))
        .await
        .map_err(|e| Error::Secret(e.to_string()))?
}

/// Round-trip a tiny request to check the saved key, URL and model.
#[tauri::command]
pub async fn test_provider(state: State<'_, AppState>, provider: ProviderKind) -> CmdResult<String> {
    let cfg = state.config();
    state
        .ai
        .complete_with(
            &cfg,
            provider,
            "Reply with the single word OK.".into(),
            vec![ChatMessage::user("ping")],
            &keymint_core::ai::no_progress(),
        )
        .await
}

#[tauri::command]
pub async fn list_models(state: State<'_, AppState>, provider: ProviderKind) -> CmdResult<Vec<String>> {
    state.ai.list_models(&state.config(), provider).await
}

#[tauri::command]
pub fn platform_info(state: State<'_, AppState>) -> PlatformInfo {
    let mut problems = state.warnings.lock().expect("lock").clone();
    problems.extend(state.shortcut_errors.lock().expect("lock").iter().cloned());
    platform::info(&problems)
}

#[tauri::command]
pub async fn copy_text(text: String) -> CmdResult<()> {
    tauri::async_runtime::spawn_blocking(move || selection::copy_text(&text))
        .await
        .map_err(|e| Error::Process(e.to_string()))?
        .map_err(Error::Process)
}

#[tauri::command]
pub async fn replace_selection(app: AppHandle, text: String) -> CmdResult<ReplaceOutcome> {
    // Focus must return to the original app before we paste.
    window::hide_palette(&app);
    tauri::async_runtime::spawn_blocking(move || {
        std::thread::sleep(std::time::Duration::from_millis(150));
        selection::replace(&text)
    })
    .await
    .map_err(|e| Error::Process(e.to_string()))?
    .map_err(Error::Process)
}

#[tauri::command]
pub fn list_history(state: State<'_, AppState>) -> Vec<HistoryEntry> {
    state.history.lock().expect("history lock").entries().to_vec()
}

#[tauri::command]
pub fn delete_history_entry(state: State<'_, AppState>, id: u64) -> CmdResult<()> {
    let mut history = state.history.lock().expect("history lock");
    history.remove(id);
    history.save(&state.history_path)
}

#[tauri::command]
pub fn clear_history(state: State<'_, AppState>) -> CmdResult<()> {
    let mut history = state.history.lock().expect("history lock");
    history.clear();
    history.save(&state.history_path)
}

#[tauri::command]
pub fn hide_palette(app: AppHandle) {
    window::hide_palette(&app);
}

#[tauri::command]
pub fn open_settings(app: AppHandle) {
    window::open_settings(&app);
}
