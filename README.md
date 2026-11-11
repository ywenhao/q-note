<p align="center"><img src="./app-icon.png" alt="Q Note" width="100" /></p>

# Q Note · NSH

[中文说明](./README.zh-CN.md)

A compact desktop note board built with [Native Shell](https://nsh.dev), native NSH controls and TypeScript. The yellow theme, 12 note colors, independent editor, tray and 30px floating Q are preserved.

This is the `nsh` branch. It replaces the Vue/Tauri application with NSH; existing SQLite files and JSON exports remain compatible.

## Development

Install Node 24, pnpm 12 and Rust stable. Windows needs the Visual Studio C++ build tools. Linux requires GTK 4.14+ and clang.

```sh
pnpm install
pnpm dev
```

The first run compiles the application's Rust extension and desktop host. Subsequent starts reuse those artifacts.

| Command | Purpose |
| --- | --- |
| `pnpm dev` | Native desktop app |
| `pnpm dev:web` | Optional browser preview |
| `pnpm typecheck` | Generate native types and check TypeScript |
| `pnpm test` | Data and logic regressions |
| `pnpm test:ui` | NSH UI regressions with isolated storage |
| `pnpm test:desktop` | Windows screenshots, frameless bounds, editor and floating window |
| `pnpm test:native` | SQLite compatibility, rollback and update cancellation |
| `pnpm check` | TypeScript and formatting |
| `pnpm format` | Format TypeScript/TSX |
| `pnpm build` | Compile application bytecode |
| `pnpm package:windows` | Portable Windows app and ZIP |
| `pnpm package:macos` | macOS application and archive |
| `pnpm package:linux` | Linux directory and tar.gz |
| `pnpm package:web` | Static browser preview |

Artifacts are written to `dist/<platform>`. The browser preview uses separate browser storage and an inline editor; native tray, floating window, local file paths, launch at login and desktop import/export require the desktop app.

The optional web runtime also needs the `wasm32-unknown-unknown` Rust target and LLVM/clang on PATH.

## Pinned NSH toolchain

Multi-window and tray support require nightly revision `c5214aaf4bbdf33bc1d59e9595e009b6aa54d702`. The wrapper downloads the official distribution and verifies its SHA-256 before running it. It also accepts a matching global NSH or `Q_NOTE_NSH_BIN`.

If the upstream nightly has moved, the wrapper builds the pinned CLI from source automatically. You can also build it explicitly:

```sh
git clone https://github.com/longbridge/nsh nsh-toolchain
git -C nsh-toolchain checkout c5214aaf4bbdf33bc1d59e9595e009b6aa54d702
cargo build --release --manifest-path nsh-toolchain/Cargo.toml -p nsh-cli
```

Set `Q_NOTE_NSH_BIN` to the absolute path of `nsh-toolchain/target/release/nsh.exe` on Windows, or `nsh` on macOS/Linux. A source checkout builds the required host automatically. The project rejects a different runtime revision rather than silently changing APIs.

## Product and data

Notes support text, embedded images, image URLs and local paths. Cards can be pinned, colored, copied, resized and sorted across pinned groups. The editor supports file picks, drops and screenshot pasting. Image previews provide keyboard controls, zoom and panning. Chinese and English are available throughout the UI.

The database stays at `~/.q-note/q-note.db`. The first run migrates the former configuration-directory database if the new file does not exist, including committed WAL data, and retains the source. Set `Q_NOTE_DATA_DIR` to an isolated directory for manual QA.

[Functional specification](./docs/FEATURES.md) defines the complete behavior. [NSH implementation notes](./docs/NSH.md) describe architecture and verification.

Before changing window behavior, tray integration or animations on Windows, or replacing the desktop stack, read [Windows regression history](./docs/WINDOWS_REGRESSIONS.md). It records the floating Q disappearing until the tray menu opens, its confirmed fix, and the first-display checks future implementations must preserve. The framework-independent [migration test catalog on main](https://github.com/ywenhao/q-note/blob/main/docs/TEST_CASES.md) defines the detailed cases, including DOCK-022–025.

## Releases

`pnpm release:patch --dry-run` previews the next release. The patch/minor/major commands synchronize all versions, run checks, create a commit and `nsh-vX.Y.Z` tag, then push. Run these only when intending to publish.

GitHub Actions builds each desktop platform, produces a checksummed NSH manifest and publishes a dedicated NSH release. The updater checks only `nsh-v*` releases, supports real cancellation and uses NSH's verified installer. The original variant's latest release remains separate. macOS packages need Developer ID signing and notarization for trusted public distribution.

[MIT](./LICENSE)
