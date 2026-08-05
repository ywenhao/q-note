# Vue Vapor Architecture

Q Note's frontend uses Vue 3.6 Vapor with TypeScript. This document describes the current architecture after the React migration; older implementation plans under `docs/superpowers` remain historical records and may still mention React-era `.tsx` paths.

## Compilation

- `@vitejs/plugin-vue` enables `features.vapor` globally.
- Every SFC also carries the explicit `<script setup lang="ts" vapor>` marker so Vite and `vue-tsc` agree on component types.
- `src/main.ts` and `src/editor.ts` mount their roots with `createVaporApp`.
- `tsconfig.app.json` enables the Vue language tooling's `vapor` option.
- Vue is pinned to `3.6.0-rc.2` because Vapor is provided by the Vue 3.6 line used by this project.

## Component and state conventions

- UI is implemented with Vue SFC templates, not JSX.
- Local UI state uses `ref` and `reactive`; derived state uses `computed`.
- Cross-window, updater, dock, note, and settings behavior lives in composables that use Vue lifecycle hooks and `watch`/`watchEffect`.
- Components communicate through typed `defineProps` and `defineEmits` declarations.
- Durable notes and settings still go through `src/lib/storage.ts`; the migration did not change the SQLite schema or Tauri event payloads.

## Third-party integration

Card ordering uses the `useDraggable` composable from vue-draggable-plus. Vapor does not expose a traditional VDOM component instance through `getCurrentInstance()`, so `NoteList.vue` deliberately disables the package's automatic startup and starts/destroys the SortableJS instance from Vapor's own `onMounted` and `onBeforeUnmount` hooks.

Lucide's Vue components are traditional functional VDOM components. Q Note therefore imports only their icon-node data and renders it through the native Vapor `Icon.vue` SFC. This keeps the icon set and appearance without enabling Vue's VDOM interoperability plugin.

## Entrypoints

- `index.html` -> `src/main.ts` -> `src/App.vue`
- `editor.html` -> `src/editor.ts` -> `src/components/EditorWindow.vue`

## Verification

Run the following before shipping frontend changes:

```bash
pnpm typecheck
pnpm check
pnpm build
pnpm test:context-menu
pnpm test:update-draft
pnpm test:updater
cargo check --manifest-path src-tauri/Cargo.toml
```
