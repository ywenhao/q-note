# AGENTS.md

## Project

Q Note is a compact desktop note app with durable local persistence and a system tray. The current `slint` branch uses Rust + Slint, but product behavior must remain independent from any specific framework or language.

## Commands

- Install the stable Rust toolchain and required platform desktop/GTK dependencies as needed.
- Run the app with `cargo run`.
- Typecheck/build with `cargo check` / `cargo build --release`.
- Format with `cargo fmt`.

## Release

- Bump `Cargo.toml` version, commit, tag `vX.Y.Z`, and push the tag.
- Pushing a `v*` tag triggers `.github/workflows/release.yml`, which builds platform binaries.

## Working Rules

- Unless the user explicitly requests a framework or language migration, keep the current branch's native desktop architecture and do not introduce a webview layer.
- When the user explicitly requests a framework or language migration, preserve every behavior in `docs/FEATURES.md`; framework-specific conventions must not silently remove or change product behavior.
- In the current Rust implementation, keep UI text in `src/i18n.rs` so Chinese and English stay aligned. A requested language migration must keep an equivalent centralized bilingual text source.
- In the current Rust implementation, store durable note and setting data through `src/storage/`. A requested migration must preserve the same `~/.q-note/q-note.db` data and compatibility behavior.
- Keep comments in English and add them only when they clarify non-obvious logic.
- Preserve the main background color `#ffd150`.
- In the current Rust implementation, keep card color choices in `src/models.rs`. A requested migration must preserve the same ordered palette in one centralized source.
- Do not remove user data during import/export changes unless the import flow explicitly replaces data.

## Functional Specification Sync (Mandatory)

- `docs/FEATURES.md` is the single source of truth for current product functionality, interaction details, visual invariants, persistence behavior, and migration acceptance.
- Read `docs/FEATURES.md` completely before changing features, UI behavior, interaction flow, settings, persistence semantics, import/export, updates, tray behavior, window behavior, docking, language behavior, timing, thresholds, defaults, or user-visible styling.
- Update `docs/FEATURES.md` in the same task whenever any of those behaviors are added, removed, renamed, reordered, or adjusted. Small interaction details and changed constants count as behavior changes.
- If it is unclear whether a change affects product behavior, treat it as affecting behavior and update the document.
- Keep `docs/FEATURES.md` framework- and language-independent. Record what users observe and what constraints must hold; do not record source paths, modules, types, callbacks, libraries, APIs, or implementation plans.
- When a requested migration changes the active framework, language, build commands, or release workflow, update the corresponding `Project`, `Commands`, `Release`, and current-implementation rules in this `AGENTS.md` during the same task. Keep the functional specification independent from those implementation updates.
- Internal refactors, dependency upgrades, formatting, build changes, and equivalent bug fixes do not require a functional-spec edit when observable behavior is unchanged.
- Before finishing a task, compare the implementation changes with `docs/FEATURES.md`. Do not report completion while the implementation and functional specification disagree.
- `docs/SLINT_MIGRATION_PLAN.md` is a historical implementation record. It may help explain the current migration, but it does not replace `docs/FEATURES.md` as the functional baseline.
