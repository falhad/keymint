# Contributing to Keymint

Thanks for helping out. Bug reports, ideas and pull requests are welcome.

## Reporting bugs and ideas

Open an [issue](https://github.com/falhad/keymint/issues). For bugs include:

* your OS and version, and on Linux whether you use X11 or Wayland
* how you installed Keymint (`.deb`, `.rpm`, AppImage, from source)
* the steps to reproduce, and what you expected to happen

Never paste API keys or private text into an issue.

## Setting up

Install the prerequisites and system libraries listed in the
[Development section of the README](README.md#development), then:

```bash
pnpm install
pnpm tauri dev
```

## Where things live

The rule is: **if it can be tested without a window, it lives in
`crates/keymint-core`.** `src-tauri` only talks to the OS and `src/` is the
React UI. See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the design and
how to add a command.

## Before you open a pull request

Run all three and make sure they pass:

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
pnpm typecheck
```

`cargo test -p keymint-core` runs without any GUI libraries.

## Guidelines

* Keep the change small and focused. Unrelated refactors and reformatting go in
  their own pull request.
* Match the style and patterns of the surrounding code.
* Add or update tests for behavior you change.
* Handle errors explicitly. No silent catches.
* Don't add dependencies without discussing them in an issue first.
* Never commit secrets. API keys belong in the OS keychain, not in `config.json`.
* Commit messages are short and imperative: "Add X", "Fix Y".

## Pull requests

1. Fork the repo and create a branch from `main`.
2. Make your change and run the checks above.
3. Open a pull request that says what changed and why, and how you tested it.
   Add screenshots for UI changes.

## License

By contributing you agree that your work is released under the
[MIT License](LICENSE).
