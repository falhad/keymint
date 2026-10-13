# Keymint

A fast, lightweight, open-source command palette for macOS, Windows and Linux,
inspired by Raycast. Press a shortcut, type, hit Enter.

* Floating palette with fuzzy search and full keyboard navigation
* **AI actions on selected text**: rewrite in ASD-STE100, make friendly, fix
  grammar, summarize, translate, and your own prompts
* Providers: OpenAI, Anthropic, Ollama (local), OpenRouter, and a local 9router (`http://localhost:20128/v1`). API keys live in the
  OS keychain
* Result view with **Copy**, **Retry** and **Replace Selected Text**
* **Ask AI**: chat with your provider right in the palette (follow-ups, selected text as context)
* **`/` commands**: type `/` to list commands, `/ste some text`, `/fix`, `/ask why is the sky blue`
* Live progress while the AI works: request sent → model thinking → reply streaming in
* **History** (`Ctrl+H` or the *Show History* command) to copy or paste recent results again
* Custom prompts, web links and programs, each with an optional global shortcut
* Dark / light / system theme, launch at login, system tray
* Built on a **command registry**: new action groups plug in as providers
  (see [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md))

Stack: Tauri 2, Rust core, React + TypeScript + Tailwind CSS 4.

## Install

Keymint is not published as a release yet. Build it from source:

```bash
git clone https://github.com/falhad/keymint.git
cd keymint
pnpm install
pnpm tauri build
```

See [Building and packaging](#building-and-packaging) for the output per OS
and [Development](#development) for the system dependencies.

## Using it

| Keys | Action |
|---|---|
| `Ctrl+Space` (`Alt+Space` on macOS) | Open / close the palette (configurable) |
| `↑` `↓`, `Enter` | Move, run |
| `Esc` | Clear search → close; in results: back |
| `Enter` / `Ctrl+Enter` / `Ctrl+R` / `Ctrl+E` | Copy / Replace selection / Retry / Edit input |
| `/` then `Tab` | List slash commands; `Tab` completes the name, text after it is the command's input |
| In Ask AI: `Enter` / `Ctrl+N` / `Esc` | Send / new chat / back (or cancel while waiting) |
| `Ctrl+H` | Open history (`Enter` copy, `Ctrl+Enter` paste, `Del` remove) |

Select text in any app, open Keymint, pick an AI command. The first time,
open **Settings → Providers**, choose a provider and save an API key (or pick
Ollama and no key). The settings window is in the tray menu or the
*Open Settings* command.

Command line (also how you bind shortcuts on Wayland):

```
keymint --toggle           open or close the palette
keymint --run <command-id> open the palette and run a command, e.g. ai.fix-grammar
keymint --settings
```

The command ids are listed in Settings → Shortcuts.

Data lives in the per-user config directory (`~/.config/dev.keymint.app` on
Linux, `~/Library/Application Support/dev.keymint.app` on macOS,
`%APPDATA%\dev.keymint.app` on Windows): `config.json` (settings, no secrets)
`usage.json` (ranking history) and `history.json` (recent results; size and clearing in Settings → General).

## Platform notes

**macOS** — grant Accessibility access (System Settings → Privacy & Security →
Accessibility) so Keymint can copy and paste on your behalf. Keymint runs as a
menu-bar app without a Dock icon.

**Windows** — nothing special. Some elevated apps ignore simulated keystrokes.

**Linux X11** — works like Windows.

**Linux Wayland** — the protocol forbids apps from registering global shortcuts
and injecting keystrokes, so:

1. Bind a shortcut in your desktop settings to `keymint --toggle`
   (GNOME: Settings → Keyboard → Custom Shortcuts; KDE: System Settings →
   Shortcuts → Custom Shortcuts). Do the same with `keymint --run <id>` per command.
2. Selected text is read from the PRIMARY selection (the highlighted text).
   Install `wl-clipboard` for best results. GNOME does not expose it to
   background apps in all cases; if nothing is captured, paste your text into the
   palette's input box.
3. *Replace Selected Text* tries `wtype`/`ydotool` if installed, then synthetic
   keys, which reach X11/XWayland apps (e.g. JetBrains IDEs). For native Wayland
   apps the result is left on the clipboard and you press `Ctrl+V`.
   Turn on **Settings → General → Paste AI results over the selected text
   automatically** to skip the result screen.
4. API keys need a Secret Service provider (GNOME Keyring or KWallet), which
   desktops ship by default.

## Development

Prerequisites: [Rust](https://rustup.rs) (stable), Node 20+ and
[pnpm](https://pnpm.io), plus the Tauri system dependencies:

```bash
# Debian / Ubuntu
sudo apt install build-essential pkg-config libwebkit2gtk-4.1-dev libgtk-3-dev \
  libayatana-appindicator3-dev librsvg2-dev libxdo-dev libssl-dev wl-clipboard
# Fedora
sudo dnf install webkit2gtk4.1-devel gtk3-devel libappindicator-gtk3-devel \
  librsvg2-devel libxdo-devel openssl-devel wl-clipboard
# Arch
sudo pacman -S webkit2gtk-4.1 gtk3 libayatana-appindicator librsvg xdotool openssl wl-clipboard
```

macOS: `xcode-select --install`. Windows: Visual Studio Build Tools
(C++ workload) and WebView2 (preinstalled on Windows 11).

```bash
pnpm install
pnpm tauri dev          # run the app with hot reload
cargo test --workspace  # Rust tests (core crate needs no system libraries)
cargo clippy --workspace --all-targets -- -D warnings
pnpm typecheck          # frontend types
```

`cargo test -p keymint-core` runs the registry, search, config, AI request /
response and shell-safety tests without any GUI libraries.

## Building and packaging

```bash
pnpm tauri build
```

Installers are written to `target/release/bundle/`. Build on the OS you are
targeting.

| OS | Output | Notes |
|---|---|---|
| Linux | `.deb`, `.rpm`, `.AppImage` | `pnpm tauri build --bundles deb` (or `rpm`, `appimage`) |
| macOS | `.app`, `.dmg` | Universal: `rustup target add aarch64-apple-darwin x86_64-apple-darwin` then `pnpm tauri build --target universal-apple-darwin`. Distribution needs an Apple Developer ID for signing and notarization (`APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`). |
| Windows | `.msi`, `.exe` (NSIS) | `pnpm tauri build --bundles nsis`. Optional code signing via `bundle.windows.certificateThumbprint`. |

The `.deb` recommends `wl-clipboard` and `wtype`. The app icon is generated
from `assets/icon.svg` with `pnpm tauri icon assets/icon.svg`.

## Project layout

```
crates/keymint-core/   registry, commands, AI providers, config (no OS deps)
src-tauri/             window, tray, shortcuts, selection, keychain, IPC
src/                   React UI: palette/ and settings/ entry points
docs/ARCHITECTURE.md   design and how to add commands
```

## Contributing

Issues and pull requests are welcome at
<https://github.com/falhad/keymint>. Before opening a PR, run
`cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`
and `pnpm typecheck`.

## License

MIT
