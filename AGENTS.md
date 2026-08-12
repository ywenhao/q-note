# AGENTS.md

## Project

Q Note is a GPUI desktop app (Rust) with `gpui-component` UI, SQLite persistence, and a system tray.

## Commands

- Install Rust toolchain (stable) and platform GPUI/GTK deps as needed.
- Run the app with `cargo run`.
- Typecheck/build with `cargo check` / `cargo build --release`.
- Format with `cargo fmt`.

## Release

- Bump `Cargo.toml` version, commit, tag `vX.Y.Z`, and push the tag.
- Pushing a `v*` tag triggers `.github/workflows/release.yml`, which builds platform binaries.

## Working Rules

- Keep the frontend in GPUI (no Vue/React/Tauri webview layer).
- Prefer `gpui-component` for interactive controls (Button, Input, Dialog, Switch, Notification, menus) while painting the yellow board shell to match the original look (`#ffd150`).
- Do **not** depend on crates.io `gpui-base` — it is an empty stub (`Hello, world!`) and cannot style the app.
- Keep UI text in `src/i18n.rs` so Chinese and English stay aligned.
- Store durable note and setting data through `src/storage/` (same `~/.q-note/q-note.db` schema as before).
- Keep comments in English and add them only when they clarify non-obvious logic.
- Preserve the main background color `#ffd150`.
- Keep card color choices in `src/models.rs` so the palette remains centralized.
- Do not remove user data during import/export changes unless the import flow explicitly replaces data.
