# AGENTS.md

## Project

Q Note is a Tauri 2 desktop app with Vue 3.6 Vapor, TypeScript, Vite+, Tailwind CSS, and SQLite.

## Commands

- Install dependencies with `pnpm install`.
- Start the web dev server with `pnpm dev`.
- Start the desktop app with `pnpm tauri dev`.
- Run TypeScript checks with `pnpm typecheck`.
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
