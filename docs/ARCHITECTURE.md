# Keymint architecture

```
┌────────────── webview (React + TS + Tailwind) ──────────────┐
│ palette window            settings window (created lazily)  │
│   thin views: search, input, result    draft + Save/Revert  │
└───────────────▲─────────────────────────────▲───────────────┘
                │ Tauri IPC (src-tauri/src/ipc.rs)            │
┌───────────────┴─────────────────────────────┴───────────────┐
│ src-tauri  (OS integration only)                             │
│   window · tray · global shortcuts · CLI flags               │
│   selection capture/replace · keychain · autostart           │
├──────────────────────────────────────────────────────────────┤
│ crates/keymint-core  (all logic, no UI or OS dependencies)   │
│   registry · commands · ai · config · secrets traits         │
└──────────────────────────────────────────────────────────────┘
```

The rule: **if it can be tested without a window, it lives in `keymint-core`.**
The shell never decides *what* a command does; it only performs the side
effects a command asks for (`Action`) and talks to the OS.

## The command registry

Everything the user can run is a `Command` (`registry/manifest.rs`):

```rust
#[async_trait]
pub trait Command: Send + Sync {
    fn manifest(&self) -> &CommandManifest;                  // static metadata for the UI
    async fn run(&self, ctx: &CommandContext, input: CommandInput) -> Result<CommandOutput>;
}
```

* `CommandManifest` — namespaced `id` (`ai.fix-grammar`, `shell.git-status`),
  title, subtitle, keywords, icon, category, `input` kind
  (`none` / `selection` / `text`) and whether the result may replace the selection.
* `CommandContext` — services handed to a command (config snapshot, AI service).
  Commands never read globals, so they are easy to test.
* `CommandOutput` — `Text` (shown with Copy / Retry / Replace), `Done`, or
  `Action` (open settings, open URL, quit). Actions are plain data, so the core
  never touches a window system.

Commands come from **providers** (`CommandProvider`). The registry is rebuilt
from all providers whenever the config changes; duplicate ids keep the first
registration, so nothing can shadow a built-in.

| Provider | Commands | Source |
|---|---|---|
| `builtin` | settings, quit, text transforms | code |
| `ai` | ASD-STE100, friendly, grammar, summarize, translate, `prompt.*` | code + config |
| `shell` | `shell.*` programs | config |
| `url` | `url.*` web links | config |

Search is fuzzy (`nucleo-matcher`) over title, keywords, category and subtitle,
boosted by usage (`frecency.rs`, stored in `usage.json`).

### Adding a command group (Git, Jira, workflows, plugins…)

1. Create `crates/keymint-core/src/commands/git.rs` with one or more
   `impl Command` types and a `GitProvider: CommandProvider`.
2. Add `Box::new(git::GitProvider)` to `default_providers()` in
   `commands/mod.rs`.
3. Done — it is searchable, rankable, shortcut-able (`keymint --run git.status`)
   and appears in Settings → Shortcuts. No UI or shell change is needed.

A provider may read its own section from `Config` (see how `shell` and `url`
do it). A future plugin loader is just another `CommandProvider` whose
`commands()` returns commands backed by an external process or script; the
registry does not care where a command comes from.

## AI layer

`ai::AiProvider::complete(&ChatRequest) -> String` with implementations for
OpenAI-compatible APIs (OpenAI, OpenRouter), Anthropic Messages and Ollama.
Request building and response parsing are pure functions with unit tests.
`AiService` resolves the default provider, model and key, and runs the call.
Streaming can be added by extending the trait; commands do not depend on it.

## Security model

* API keys exist only in the OS keychain (`secrets::SecretStore`, implemented
  with `keyring`). The frontend can set or delete a key and learn whether one
  exists; it can never read it back.
* Shell commands run `program + argv` without a shell, with a timeout and an
  output cap. Selected text cannot inject extra commands. Only commands the
  user created in Settings can run.
* URL commands accept only `http(s)://` templates, and the query is URL-encoded.
* The webview has only `core:default` permissions; privileged work happens in
  Rust commands. A CSP is set in `tauri.conf.json`.
* A corrupt `config.json` is moved to `config.json.bak`, never overwritten.

## Selection capture and replace

| Platform | Capture | Replace |
|---|---|---|
| macOS, Windows, Linux X11 | simulate Cmd/Ctrl+C, read clipboard, restore it | set clipboard, simulate Cmd/Ctrl+V, restore |
| Linux Wayland | read the PRIMARY selection (`arboard`, then `wl-paste --primary`) | set clipboard, then `wtype` or `ydotool` if installed; otherwise the user presses Ctrl+V |

Capture runs **before** the palette is shown, because afterwards the other
app's selection is out of reach. `platform::info()` tells the UI what works so
buttons are labelled honestly.

## Startup and memory

Only the small palette webview exists at startup (hidden). The Settings window
is created on first use and destroyed on close. Release builds use LTO,
`opt-level = "s"`, `panic = "abort"` and stripped symbols.
