# AGENTS.md

## Project

Q Note is a Tauri 2 desktop app with React, TypeScript, Vite+, Tailwind CSS, and SQLite.

## Commands

- Install dependencies with `pnpm install`.
- Start the web dev server with `pnpm dev`.
- Start the desktop app with `pnpm tauri dev`.
- Run TypeScript checks with `pnpm typecheck`.
- Run Vite+ checks with `pnpm check`.
- Format with `pnpm format`.
- Build the frontend with `pnpm build`.

## Working Rules

- Use `pnpm` for JavaScript dependencies and scripts.
- Keep UI text in `src/i18n.ts` so Chinese and English stay aligned.
- Store durable note and setting data through `src/lib/storage.ts`.
- Keep Tauri permissions in `src-tauri/capabilities/default.json` narrow and explicit.
- Keep comments in English and add them only when they clarify non-obvious logic.
- Preserve the main background color `#ffd150`.
- Keep card color choices in `src/types.ts` so the palette remains centralized.
- Do not remove user data during import/export changes unless the import flow explicitly replaces data.
