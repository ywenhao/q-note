<p align="center"><img src="./app-icon.png" alt="Q Note" width="120" height="120" /></p>
<h1 align="center">Q Note</h1>
<p align="center">A compact desktop board for reusable text, pictures, and file references. <a href="./README.zh-CN.md">中文说明</a></p>

## Native Shell edition

This branch reimplements Q Note with [Longbridge Native Shell](https://github.com/longbridge/nsh) 0.1.1 and Rust. TSX describes native controls and Canvas icons; the app ships no WebView, Vue, or React runtime. Windows, macOS, and Linux are supported. Mobile is outside this branch's scope.

| Main board                                                               | Independent editor                                                     |
| ------------------------------------------------------------------------ | ---------------------------------------------------------------------- |
| <img src="./docs/images/q-note-main.png" alt="Main board" width="300" /> | <img src="./docs/images/q-note-editor.png" alt="Editor" width="390" /> |

Click a card to copy it. Pin, recolor, reorder, or expand its text; add embedded pictures, image URLs, local paths, dropped files, or clipboard screenshots in the independent editor. The image viewer supports navigation, zoom, and panning. Collapse the yellow board to a 30px Q icon with edge snapping and hover reveal. Chinese/English, global always-on-top, tray restoration, launch at login, JSON import/export, and verified executable updates are included.

The complete, framework-independent behavior baseline is [docs/FEATURES.md](./docs/FEATURES.md). Architecture and verification details are in [docs/NATIVE_SHELL.md](./docs/NATIVE_SHELL.md).

## Development

Install Rust stable, Node.js 24, pnpm, and [GitHub CLI](https://cli.github.com/). Authenticate `gh` or set `GH_TOKEN` to download the pinned NSH release and source. macOS needs Xcode command-line tools; Windows needs the Visual Studio C++ workload and Windows SDK. Linux needs GTK 4 development headers, libclang, CMake, pkg-config, and a desktop session with a file-picker portal.

```bash
# Ubuntu 24.04 or newer
sudo apt-get install libgtk-4-dev libclang-dev cmake pkg-config
pnpm install
pnpm setup
pnpm dev
```

| Command                          | Result                                              |
| -------------------------------- | --------------------------------------------------- |
| `pnpm typecheck`                 | Check all three native window entries               |
| `pnpm test`                      | Rust domain/storage tests and update manifest tests |
| `pnpm build:ui`                  | Generate the UI bundles embedded by Cargo           |
| `cargo build -p q-note --locked` | Build the debug native executable after bundling    |
| `pnpm check`                     | Types, formatting, and Rust workspace checks        |
| `pnpm format`                    | Format TSX/tooling and Rust                         |
| `pnpm build`                     | Typecheck, bundle, and build the release executable |
| `pnpm package`                   | Package the host executable into `dist/packages`    |
| `pnpm test:desktop`              | Run the real Linux native-window regression         |

Desktop regression requires `xvfb openbox xdotool xclip imagemagick dbus-x11`. Its screenshots and log are saved in `dist/smoke`. Set `Q_NOTE_DATA_DIR=/path/to/scratch` when testing manually to keep personal notes isolated. `NSH_BIN` and `NSH_SOURCE` can point to a matching 0.1.1 CLI and checkout for offline development. Intel macOS builds the pinned CLI from source because upstream only publishes an ARM macOS CLI.

## Data

The SQLite database remains compatible with previous Q Note versions:

| Platform      | Database                                |
| ------------- | --------------------------------------- |
| Windows       | `C:\Users\<username>\.q-note\q-note.db` |
| macOS / Linux | `~/.q-note/q-note.db`                   |

Before creating a new database, the app copies an existing legacy `%APPDATA%\com.win11.q-note\q-note.db` without overwriting an existing destination or deleting the original. Live SQLite journal contents are included. JSON export contains notes, attachments, settings, format version, and export time. Import replaces all notes and settings atomically; cancellation or failure preserves the previous data.

## Packaging and releases

The branch uses portable packages: Windows x64 `.zip`, Linux x64 `.tar.gz` (requires GTK 4), and macOS `.app.zip` for Apple Silicon and Intel. There is no separate frontend directory to distribute: UI bundles and icons are embedded in the executable. macOS packages use ad-hoc executable signing and are not Developer ID notarized.

```bash
pnpm release:patch
pnpm release:minor
pnpm release:major
```

These commands update `package.json`, workspace `Cargo.toml`, and `ui/nsh.json`, synchronize both workspace entries in `Cargo.lock`, create a version commit/tag, and push them. Keep `bumpp --all` so the lockfile enters the same commit. Tagged builds run the native release workflow, then publish packages, raw updater executables, their SHA-256/size metadata, and `latest.json`. The updater downloads and verifies a matching executable, saves notes/window/draft state, replaces the executable, and restarts after the old process exits. An unwritable application location produces an installation failure while retaining user data.

macOS may require **Privacy & Security → Open Anyway** or Finder **Open** for an unnotarized download. Linux Wayland compositors can restrict absolute positioning and always-on-top; those window operations safely fall back to compositor behavior. A desktop without a tray host can restore the Linux main window by launching the app again.

## License

[MIT](./LICENSE)
