# AGENTS.md

## Project

Q Note is a Tauri 2 desktop app with Vue 3.6 Vapor, TypeScript, Vite+, Tailwind CSS, and SQLite.

## Commands

- Install dependencies with `pnpm install`.
- Start the web dev server with `pnpm dev`.
- Start the desktop app with `pnpm tauri dev`.
- Run TypeScript checks with `pnpm typecheck`.
- Run the test suite with `pnpm test`.
- Run Vite+ checks with `pnpm check`.
- Format with `pnpm format`.
- Build the frontend with `pnpm build`.

## Release

- Bump version, sync `Cargo.lock`, commit, tag, and push with `pnpm release:patch`, `pnpm release:minor`, or `pnpm release:major`.
- The release script uses `bumpp` with `--all` so `src-tauri/Cargo.lock` is included in the same `release: vX.Y.Z` commit. Do not remove `--all`; without it, `bumpp` only commits the version files it edits.
- `scripts/sync-cargo-lock.mjs` runs during release via `cargo update` to refresh the `q-note` entry in `Cargo.lock` after `src-tauri/Cargo.toml` is bumped.
- Pushing a `v*` tag triggers `.github/workflows/release.yml`, which builds platform artifacts and uploads `latest.json` for the in-app updater.

## Working Rules

- Use `pnpm` for JavaScript dependencies and scripts.
- Write components as Vue SFCs with `<script setup lang="ts" vapor>` and use Vue composables for reusable stateful behavior.
- Keep the frontend in Vapor mode and do not add React or a VDOM compatibility layer.
- Keep UI text in `src/i18n.ts` so Chinese and English stay aligned.
- Store durable note and setting data through `src/lib/storage.ts`.
- Keep Tauri permissions in `src-tauri/capabilities/default.json` narrow and explicit.
- Keep comments in English and add them only when they clarify non-obvious logic.
- Preserve the main background color `#ffd150`.
- Keep card color choices in `src/types.ts` so the palette remains centralized.
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
