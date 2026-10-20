# AGENTS.md

## Project

Q Note is a desktop app built with Longbridge Native Shell (NSH 0.1.1), TypeScript TSX native components, Rust, and SQLite. It supports Windows, macOS, and Linux; mobile is outside scope. The UI uses NSH signals, native views, and Canvas icons. It does not use a WebView, Vue, React, or a browser DOM.

## Commands

- Install JavaScript tooling with `pnpm install`.
- Download the pinned NSH CLI and source with `pnpm setup` (requires `gh` authentication or `GH_TOKEN`).
- Start the native app with rebuild-on-change using `pnpm dev`.
- Check all three window entry points with `pnpm typecheck`.
- Generate embedded UI bundles with `pnpm build:ui` before direct Cargo desktop builds.
- Run storage, ordering, geometry, image, updater, and release-metadata tests with `pnpm test`.
- Build a debug desktop binary with `cargo build -p q-note --locked`.
- Run real Linux native-window regression with `pnpm test:desktop` after the debug build (requires Xvfb, Openbox, xdotool, xclip, ImageMagick, and D-Bus).
- Run type, formatting, and workspace checks with `pnpm check`.
- Format with `pnpm format`; verify with `pnpm format:check`.
- Build the release desktop executable with `pnpm build`.
- Package the current host executable and updater metadata with `pnpm package`.
- Set `Q_NOTE_DATA_DIR` to an isolated directory for manual and automated tests. Never exercise destructive scenarios on personal data.

## Release

- Bump version, sync the root `Cargo.lock`, commit, tag, and push with `pnpm release:patch`, `pnpm release:minor`, or `pnpm release:major`.
- Version files are `package.json`, the root `Cargo.toml`, and `ui/nsh.json`.
- Keep `bumpp --all` so both workspace package versions in `Cargo.lock` enter the release commit.
- `scripts/sync-cargo-lock.mjs` refreshes workspace versions offline before committing.
- Pushing a `v*` tag triggers native Windows x64, Linux x64, and macOS builds, then publishes portable packages, raw executable updates, checksums/sizes, and `latest.json`.
- Packaging and updater manifest generation must agree on platform keys and the `binary` artifact format. Do not publish Tauri updater signatures for native executable updates.
- NSH crates and the downloaded CLI/source must stay on the same pinned release. See `scripts/native-shell.mjs`.

## Working Rules

- Use `pnpm` for JavaScript dependencies and scripts.
- Write TSX with NSH primitives and signals. Keep native rendering; do not add a WebView or a VDOM compatibility layer.
- Keep application UI text in `ui/i18n.ts`; keep operating-system window/tray labels centralized in `desktop/src/host.rs`. Chinese and English must cover identical actions.
- Put durable data mutations in `native/src/storage`; commit before changing shared application state.
- Reuse the existing SQLite schema and data paths. Legacy copying must happen before a new database is created and must never overwrite an existing destination.
- Keep NSH capability declarations in `desktop/src/host.rs` explicit and limited to actual UI needs.
- Keep comments in English and add them only for non-obvious logic.
- Preserve the main background color `#ffd150` and editor background `#fff9df`.
- Keep the ordered 12-color palette centralized in `native/src/models.rs`; the UI reads it from shared state.
- Key note and attachment lists by their durable IDs so asynchronous image and input events cannot dispose active controls.
- UI bundles are embedded in the Rust executable. Regenerate them before validating or packaging desktop changes.
- Do not remove user data during import/export changes unless the import flow explicitly replaces data.

## Functional Specification Sync (Mandatory)

- `docs/FEATURES.md` is the single source of truth for current product functionality, interaction details, visual invariants, persistence behavior, and migration acceptance.
- The completeness target is a clean-room reimplementation: a new AI must be able to read only `docs/FEATURES.md` and reproduce all user-visible behavior in another framework or language without inspecting the existing source code, comparing another branch, or repeatedly asking the user for missing product decisions.
- Read `docs/FEATURES.md` completely before changing features, UI behavior, interaction flow, settings, persistence semantics, import/export, updates, tray behavior, window behavior, docking, language behavior, timing, thresholds, defaults, or user-visible styling.
- Update `docs/FEATURES.md` in the same task whenever any of those behaviors are added, removed, renamed, reordered, or adjusted. Small interaction details and changed constants count as behavior changes.
- If it is unclear whether a change affects product behavior, treat it as affecting behavior and update the document.
- Keep `docs/FEATURES.md` framework- and language-independent. Record what users observe and what constraints must hold; do not record source paths, modules, types, callbacks, libraries, APIs, or implementation plans.
- When a requested migration changes the active framework, language, build commands, or release workflow, update the corresponding `Project`, `Commands`, `Release`, and current-implementation rules in this `AGENTS.md` during the same task. Keep the functional specification independent from those implementation updates.
- Internal refactors, dependency upgrades, formatting, build changes, and equivalent bug fixes do not require a functional-spec edit when observable behavior is unchanged.
- Before finishing a task, compare the implementation changes with `docs/FEATURES.md`. Do not report completion while the implementation and functional specification disagree.
- `docs/SLINT_MIGRATION_PLAN.md` is a historical implementation record. It may help explain the current migration, but it does not replace `docs/FEATURES.md` as the functional baseline.

### Specification Completeness Standard (Mandatory)

- Write requirements as testable product rules, not feature-name summaries. Each feature section must contain every applicable item below; omit an item only when it truly does not apply.
- Define scope and entry points: what the feature does, where it is available, every way it can be opened or triggered, prerequisites, and how the user exits or cancels it.
- Define the visual structure and stacking order: parent/child regions, overlays, clipping, masks, rounded corners, shadows, and which element must appear above another when states overlap.
- Record exact visual constraints: window and component dimensions, minimums and maximums, padding, gaps, alignment, icon and hit-target sizes, typography, colors and opacity, border widths, radii, breakpoints, and width/height recalculation rules. Distinguish fixed values from content-dependent or viewport-dependent values.
- Record the complete state matrix: default, hover, pressed, focused, selected, disabled, loading, empty, error, open, closing, dragging, resizing, docked, hidden, and any feature-specific states. For every reachable transition, state the trigger, visible result, and permitted next actions.
- Record every animation in reproducible terms: animated properties, start and end values, duration, delay, easing, opening and closing directions, and behavior when interrupted, reversed, or retriggered. If an element intentionally has no animation, state that when users could otherwise expect one.
- Record pointer, keyboard, focus, and window behavior: exact hit areas, click/double-click/right-click results, hover ownership and retention, click-outside and Escape behavior, tab/focus rules, drag thresholds, resize bounds, snap/clamp rules, click-through behavior, and native-window fallbacks where they affect the user experience.
- Record interaction-conflict rules explicitly. Examples include nested menus versus parent hover leave, resize handles versus menu arrows, drag versus click suppression, modal versus background input, multiple simultaneous popups, and close actions during animation. State which interaction wins, what remains visible, and when state resets.
- Record content behavior for all relevant data shapes: zero/one/many items, short and long text, explicit line breaks, wrapping, truncation and ellipsis placement, scrolling, overflow, duplicate names, invalid input, and localized text expansion.
- Record data and lifecycle semantics: defaults, durable versus session-only state, when writes occur, what is restored after restart, retention windows, ordering, deletion effects, import/export replacement or merge behavior, compatibility expectations, and failure/retry/cancellation outcomes.
- Record environmental boundaries where applicable: minimum and maximum window sizes, live resizing, DPI scaling, multi-monitor coordinates, taskbar/work-area edges, off-screen recovery, startup and shutdown, repeated rapid actions, platform differences, and the required fallback behavior.
- Record cross-feature invariants once and reference them by an unambiguous rule name. This includes shared menu ordering, bilingual parity, palette ordering, common modal/menu motion, global corner treatment, and consistency between tray, dock, main-window, and editor entry points.
- End each substantial feature with executable acceptance scenarios written as preconditions, user actions, and exact expected results. Include normal flow, cancellation, boundary values, restart persistence, and at least the interaction conflicts most likely to regress.
- Do not use vague requirements such as “appropriate”, “normal”, “smooth”, “similar to the original”, “keep consistent”, or “as before” unless they reference a separately named rule with exact values. Replace them with measurable values, enumerated states, or a deterministic rule.
- Do not guess missing values. Inspect the running behavior and relevant reference implementation, then document the verified result. If verification is temporarily impossible, mark the item as `待核实` with the exact missing decision; do not present the functional specification as complete or finish a migration while such an item remains unresolved.
- When behavior is adjusted, update the existing rule and all affected acceptance scenarios in the same task. Remove obsolete values and resolve contradictions instead of appending a second competing description.
