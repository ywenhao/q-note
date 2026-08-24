<p align="center">
  <img src="./app-icon.png" alt="Q Note icon" width="120" height="120" />
</p>

<h1 align="center">Q Note</h1>

<p align="center">
  A compact desktop note board for snippets, images, links, and local paths you reuse often.
</p>

<p align="center">
  <a href="./README.zh-CN.md">中文说明</a>
</p>

## Screenshots

| Main board                                                                      | Editor window                                                                        |
| ------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| <img src="./docs/images/q-note-main.png" alt="Q Note main board" width="288" /> | <img src="./docs/images/q-note-editor.png" alt="Q Note editor window" width="360" /> |

## Overview

Q Note is a small desktop app for keeping short notes close to your cursor. It is built for quick capture and repeated copy workflows: save text snippets, screenshots, image URLs, local file paths, and small reference notes; pin the important ones; mark cards with color; and collapse the whole app into a tiny floating Q icon when you need the screen space.

See the framework-independent [functional specification](./docs/FEATURES.md) for the complete behavior, interaction, and migration baseline.

This branch rewrites the app with **Slint 1.17** + Rust (replacing the GPUI version and the earlier Tauri + Vue version). Data stays in the same SQLite database under `~/.q-note/q-note.db`.

## Quick Start

```bash
cargo run
```

Linux may need GTK / Vulkan packages, for example:

```bash
sudo apt-get install -y libgtk-3-dev libcairo2-dev libpango1.0-dev \
  libgdk-pixbuf-2.0-dev libxkbcommon-dev libwayland-dev libvulkan-dev
```

## Commands

| Command                 | Purpose                |
| ----------------------- | ---------------------- |
| `cargo run`             | Start the desktop app  |
| `cargo check`           | Typecheck              |
| `cargo build --release` | Release binary         |
| `cargo fmt`             | Format Rust sources    |

## Features

| Feature            | Details                                                                                                                        |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------ |
| Compact note board | Keep frequently used snippets in a narrow, always-available desktop panel                                                      |
| Quick copy         | Click a card to copy its text; attachment-only notes copy attachment values                                                    |
| Card management    | Create, edit, delete, pin, recolor note cards                                                                                  |
| Separate editor    | Create and edit notes in an independent editor window without changing the main panel size                                     |
| Images and files   | Add images via file picker, web image URLs, or local paths                                                                     |
| Floating Q icon    | Collapse to a 30px Q icon, drag it, and snap it to screen edges                                                                |
| Topmost window     | Toggle always-on-top from the chrome or tray menu                                                                              |
| Tray icon          | Keep a resident Q icon in the system tray; click it to show the app                                                            |
| Language switch    | Toggle Chinese and English from the toolbar; the choice is saved locally                                                       |
| Launch at login    | Enable or disable startup launch from Settings; it is off by default                                                           |
| Persistence        | Notes, attachments, colors, card order, card heights, window size, topmost state, language, and dock state are saved in SQLite |
| Import/export      | Export and import notes plus local settings as JSON                                                                            |

## Stack

| Area          | Tooling                                      |
| ------------- | -------------------------------------------- |
| Desktop UI    | Slint 1.17 + winit backend                   |
| Styling       | Custom Slint components + Q Note tokens      |
| Storage       | SQLite (`rusqlite`, bundled)                 |
| Tray          | `tray-icon`                                  |
| File dialogs  | `rfd`                                        |
| Autostart     | `auto-launch`                                |

The UI is rendered with declarative Slint components and connected from Rust to SQLite, the system tray, clipboard, file dialogs, edge docking, and self-update workflows. The yellow `#ffd150` board, cream editor, and pastel note palette are preserved.

## Data Location

Q Note stores its SQLite database in the user home folder:

| Platform    | Path                                    |
| ----------- | --------------------------------------- |
| Windows     | `C:\Users\<username>\.q-note\q-note.db` |
| macOS/Linux | `~/.q-note/q-note.db`                   |

If an older Windows install already has data in `%APPDATA%\com.win11.q-note\q-note.db`, Q Note copies it to the new location on first launch.

## License

[MIT](./LICENSE)
