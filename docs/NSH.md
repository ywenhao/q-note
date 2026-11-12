# NSH implementation

The `nsh` branch replaces the active Vue/Tauri entry with an NSH TSX application. Development and packaging use the locally installed stable NSH CLI (0.5.0 or newer), whose desktop API supports the independent editor, tray and floating window. The Rust dependencies use release tag `v0.5.0`. The wrapper uses `nsh` from PATH or `Q_NOTE_NSH_BIN` and does not download or build a toolchain; CI and release jobs explicitly install v0.5.0.

## UI and state

`App.tsx` mounts `src/QNote.tsx`. Native `nsh/base` buttons, switches, inputs, dialogs, progress and context menus use the project's existing colors from `src/theme.ts` and the original palette in `src/types.ts`. The model uses NSH signals; all translations remain in `src/i18n.ts`.

`src/desktop.ts` owns window bounds, tray actions, topmost state, dock positioning and edge animations. On Windows a small OS bridge clips the floating window's hidden part so it does not appear on an adjacent monitor. Other platforms use NSH's positioning and transparent windows.

The main window explicitly applies `frame: "none"` before placement so the development host cannot add a second system title bar. Main and editor spacing, circular swatches, Q artwork, neutral toolbar buttons and dark confirmation buttons follow `docs/images` and the main variant's visual baseline. Settings use a surface-sized dialog layer to stay centered.

On Windows the floating window is created at its target position. Its native visible state starts the roughly 180ms fade; only the current window's completed fade may clear stale clipping, bring the floating window forward within its current level, and hide the main window. Restore invalidates pending collapse requests, and unmount cancels the old fade, so a late completion cannot hide a restored main window or affect a replacement floating window. The roughly 500ms cached-edge delay starts at first native visibility and includes the fade. See [WIN-FLOAT-001](./WINDOWS_REGRESSIONS.md#win-float-001切成悬浮球后不可见打开托盘菜单才出现) for the original tray-dependent visibility failure, fix commit, user confirmation and regression requirements.

## Persistence

`src/lib/storage.ts` is the sole data boundary. Desktop reads and writes the existing SQLite schema through `native/src/database.rs`. Note/attachment writes, reorder, delete and import replacement are transactions. An editor save clears its recovery record in the same transaction. Settings parsing errors retain notes.

Before opening a new database, migration uses SQLite's snapshot copy into a temporary file and an exclusive destination create. It includes committed WAL pages, retains the source and never replaces an existing destination. Old missing columns are added in place.

The browser preview uses a separate NSH storage namespace; its editor is an in-page dialog. Desktop-only settings are disabled there.

## Updates and distribution

Published tags use `nsh-v*`; the updater reads only that channel from GitHub. Releases keep `latest: false` to preserve the original variant's current release. Each archive's manifest URL is absolute, its app ID is checked and downloads require a valid SHA-256, positive size and supported archive extension.

HTTP transfers run asynchronously. Cancellation drops the active request immediately, including a stalled response, and discards the temporary file. The NSH updater library verifies received bytes and installs with its own executable/identity checks and rollback. Window settings and dirty editor recovery are flushed before installation or explicit quit.

## Verification

- `pnpm test`: existing data normalization, ordering, window sizing and transaction regressions.
- `pnpm test:ui`: real NSH component interactions in the headless engine, using browser storage and isolated native storage.
- `pnpm test:desktop`: real Windows screenshots, outer/content bounds equality, independent editor size, card resize, settings, initial floating-window visible state/30px bounds/target position, three round trips, topmost changes, floating-menu cancellation and hide-on-close behavior. The test temporarily enlarges the main viewport for the driver's editor selectors, then restores it; opening the editor itself is checked before that resize. Window-state assertions and per-window screenshots do not replace actual desktop presentation or real tray-menu checks; record those separately using [Windows regression history](./WINDOWS_REGRESSIONS.md).
- `pnpm test:native`: SQLite schema compatibility, live WAL migration, failed write/import rollback, and cancellation of a stalled update response.
- `pnpm typecheck` and `pnpm format:check`: generated native declarations and source consistency.
- `pnpm package:windows`: native Windows runner and portable ZIP. Use `Q_NOTE_DATA_DIR` for launch smoke checks.

Native file dialogs, operating-system startup settings, multi-monitor DPI transitions, successful installation/restart and macOS/Linux graphical packaging need their corresponding OS environment. Automated headless tests do not replace those platform checks.
