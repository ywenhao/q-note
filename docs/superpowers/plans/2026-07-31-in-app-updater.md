# In-App Updater Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace Q Note's custom package launcher with Tauri's signed updater, preserve unsaved editor drafts, install without extra clicks, and relaunch the updated application.

**Architecture:** Keep pure draft comparison and normalization in `src/lib/updateDraft.ts`, persistence in `src/lib/storage.ts`, and cross-window preparation in `src/lib/updatePreparation.ts`. The React update manager owns a Tauri `Update` resource, downloads it with progress, flushes application state, then calls `install()`; Tauri handles platform replacement and the process plugin relaunches platforms that remain alive after installation.

**Tech Stack:** React 19, TypeScript 6, Tauri 2 updater/process plugins, Rust, SQLite/Drizzle, Node test runner, pnpm, GitHub Actions.

---

### Task 1: Define and test recoverable editor drafts

**Files:**

- Create: `src/lib/updateDraft.ts`
- Create: `tests/updateDraft.test.ts`
- Modify: `src/types.ts`
- Modify: `src/components/NoteEditor.tsx`
- Modify: `package.json`

- [ ] **Step 1: Add the focused test command and failing draft tests**

Add `"test:update-draft": "node --test tests/updateDraft.test.ts"` to `package.json`. Test the public contract with fixtures for a new note, unchanged existing note, changed content, changed attachment, and malformed persisted JSON:

```ts
import assert from "node:assert/strict";
import test from "node:test";
import {
  createNoteDraft,
  isEditorDraftDirty,
  normalizePendingUpdateDraft,
} from "../src/lib/updateDraft.ts";

test("an unchanged existing note is not dirty", () => {
  const note = makeNote();
  assert.equal(isEditorDraftDirty(createNoteDraft(note), note), false);
});

test("a new note with content is dirty", () => {
  assert.equal(isEditorDraftDirty({ ...emptyDraft(), content: "recover me" }, null), true);
});

test("attachment changes are dirty", () => {
  const note = makeNote();
  assert.equal(
    isEditorDraftDirty(
      { ...createNoteDraft(note), attachments: [...note.attachments, makeAttachment()] },
      note,
    ),
    true,
  );
});

test("malformed pending drafts are rejected", () => {
  assert.equal(normalizePendingUpdateDraft({ noteId: 3, draft: null }), null);
});
```

- [ ] **Step 2: Run the tests and verify RED**

Run: `pnpm test:update-draft`

Expected: FAIL because `src/lib/updateDraft.ts` does not exist.

- [ ] **Step 3: Move the shared draft shape into the data types**

Add this type to `src/types.ts` and make `NoteEditor.tsx` re-export it so existing imports stay compatible:

```ts
export interface NoteDraft {
  attachments: NoteAttachment[];
  color: string;
  content: string;
  pinned: boolean;
}
```

In `NoteEditor.tsx`, import `type NoteDraft` from `../types`, remove the local interface, and add `export type { NoteDraft } from "../types";`.

- [ ] **Step 4: Implement the pure draft helpers**

Create `src/lib/updateDraft.ts` with:

```ts
import { DEFAULT_NOTE_COLOR, type Note, type NoteDraft } from "../types";

export interface PendingUpdateDraft {
  draft: NoteDraft;
  noteId: string | null;
  savedAt: number;
}

export function createNoteDraft(note: Note | null): NoteDraft {
  return {
    attachments: note?.attachments ?? [],
    color: note?.color ?? DEFAULT_NOTE_COLOR,
    content: note?.content ?? "",
    pinned: note?.pinned ?? false,
  };
}

export function isEditorDraftDirty(draft: NoteDraft, note: Note | null) {
  return JSON.stringify(draft) !== JSON.stringify(createNoteDraft(note));
}

export function normalizePendingUpdateDraft(value: unknown): PendingUpdateDraft | null {
  // Accept only a string-or-null noteId, finite savedAt, valid color/content/pinned fields,
  // and attachments whose id/kind/source/value/createdAt fields match NoteAttachment.
  // Return cloned arrays and objects so callers cannot mutate parsed storage in place.
}
```

The normalizer must reject the entire record when any required field is invalid; it must not invent note ids or attachment payloads during recovery.

- [ ] **Step 5: Run the focused tests and verify GREEN**

Run: `pnpm test:update-draft`

Expected: all draft helper tests pass.

- [ ] **Step 6: Commit the draft contract**

Run:

```powershell
git add -- 'package.json' 'src/types.ts' 'src/components/NoteEditor.tsx' 'src/lib/updateDraft.ts' 'tests/updateDraft.test.ts'
git commit -m 'feat: define recoverable editor drafts'
```

### Task 2: Persist pending drafts outside normal exports

**Files:**

- Modify: `src/lib/storage.ts`
- Test: `tests/updateDraft.test.ts`

- [ ] **Step 1: Add normalization coverage for persisted records**

Extend `tests/updateDraft.test.ts` to assert that a valid pending record round-trips through `normalizePendingUpdateDraft`, attachment objects are cloned, an invalid attachment rejects the record, and `noteId: null` is retained for a new note.

- [ ] **Step 2: Run the focused tests**

Run: `pnpm test:update-draft`

Expected: PASS; these tests lock down the pure boundary used by storage.

- [ ] **Step 3: Add storage functions using a dedicated settings key**

In `src/lib/storage.ts`, add `PENDING_UPDATE_DRAFT_KEY = "pending-update-editor-draft"` and a separate web fallback key. Export:

```ts
export async function loadPendingUpdateDraft(): Promise<PendingUpdateDraft | null>;
export async function savePendingUpdateDraft(value: PendingUpdateDraft): Promise<void>;
export async function clearPendingUpdateDraft(): Promise<void>;
```

For Tauri, read/upsert/delete a row in `settingsTable` using the dedicated key. For web, use the separate local-storage key. Parse with `normalizePendingUpdateDraft`; if JSON is malformed, return `null`. Do not add the pending draft to `AppSettings`, `AppData`, or `ExportPayload`.

- [ ] **Step 4: Verify types and focused tests**

Run: `pnpm test:update-draft` and `pnpm typecheck`.

Expected: both commands exit 0.

- [ ] **Step 5: Commit storage support**

Run:

```powershell
git add -- 'src/lib/storage.ts' 'tests/updateDraft.test.ts'
git commit -m 'feat: persist update recovery drafts'
```

### Task 3: Snapshot and restore the editor window

**Files:**

- Create: `src/lib/updatePreparation.ts`
- Modify: `src/components/NoteEditor.tsx`
- Modify: `src/components/EditorWindow.tsx`
- Modify: `src/lib/storage.ts`
- Modify: `src/i18n.ts`

- [ ] **Step 1: Add controlled draft initialization and observation**

Extend `NoteEditorProps` with:

```ts
initialDraft?: NoteDraft | null;
onDraftChange?: (draft: NoteDraft) => void;
```

Initialize attachments, color, content, and pinned from `initialDraft ?? createNoteDraft(note)`. Emit the complete draft from an effect whenever those four fields change. Do not emit transient media-input text or image-preview state.

- [ ] **Step 2: Implement cross-window preparation**

Create `src/lib/updatePreparation.ts` with request/response event constants and:

```ts
export async function prepareEditorForUpdate(timeoutMs = 5000): Promise<void>;
```

Use `getAllWindows()` to find the `editor` window. Return immediately if it is missing or hidden. Register the acknowledgement listener before sending a request containing a cryptographically unique `requestId`. Resolve only for the matching id and `ok: true`; reject on `ok: false` or timeout; always clear the listener and timer.

- [ ] **Step 3: Make `EditorWindow` persist the latest visible draft**

Keep the current `NoteDraft` in a ref updated by `onDraftChange`. Listen for the prepare event. If the window is visible and `isEditorDraftDirty(ref.current, note)` is true, call:

```ts
await savePendingUpdateDraft({
  draft: ref.current,
  noteId: activeNoteId,
  savedAt: Date.now(),
});
```

Then emit a matching success acknowledgement to `main`. Emit `ok: false` after a storage error. Hidden or unchanged editors acknowledge success without creating a record.

- [ ] **Step 4: Restore and maintain a recovered draft**

During editor boot, read `loadPendingUpdateDraft()` before selecting initial content. If present, load its matching note, pass its draft as `initialDraft`, show and focus the editor window, and mark the session as recovery-backed. While that session is open, debounce `savePendingUpdateDraft` by 250 ms on draft changes. Clear the record after a successful note save or explicit cancel.

- [ ] **Step 5: Add aligned status strings**

Add Chinese and English strings for securing data before installation and for a failed data flush. Keep all text in `src/i18n.ts`.

- [ ] **Step 6: Verify and commit editor recovery**

Run: `pnpm test:update-draft`, `pnpm typecheck`, and `pnpm check`.

Expected: all commands exit 0.

Then commit:

```powershell
git add -- 'src/components/NoteEditor.tsx' 'src/components/EditorWindow.tsx' 'src/lib/updatePreparation.ts' 'src/lib/storage.ts' 'src/i18n.ts'
git commit -m 'feat: restore drafts across updates'
```

### Task 4: Install and configure the official Tauri updater

**Files:**

- Modify: `package.json`
- Modify: `pnpm-lock.yaml`
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/Cargo.lock`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/capabilities/default.json`
- Modify: `.github/workflows/release.yml`
- Create outside repository: `C:\Users\admin\.tauri\q-note.key`
- Create outside repository: `C:\Users\admin\.tauri\q-note.key.pub`

- [ ] **Step 1: Add matching updater and process dependencies**

Run:

```powershell
pnpm add '@tauri-apps/plugin-updater' '@tauri-apps/plugin-process'
cargo add --manifest-path 'src-tauri/Cargo.toml' 'tauri-plugin-updater@2' 'tauri-plugin-process@2'
```

Expected: package and Cargo lockfiles contain compatible Tauri 2 plugin versions.

- [ ] **Step 2: Generate the signing identity outside the repository**

Create `C:\Users\admin\.tauri` if missing, then run `pnpm tauri signer generate --ci -w 'C:\Users\admin\.tauri\q-note.key'`. Confirm the private key path is outside the repository and neither key is reported by `git status --short`.

- [ ] **Step 3: Register the plugins and configure updater artifacts**

Add to the Tauri builder:

```rust
.plugin(tauri_plugin_process::init())
.plugin(tauri_plugin_updater::Builder::new().build())
```

Set `bundle.createUpdaterArtifacts` to `true`. Add:

```json
"plugins": {
  "updater": {
    "pubkey": "the exact single-line contents of q-note.key.pub",
    "endpoints": [
      "https://github.com/ywenhao/q-note/releases/latest/download/latest.json"
    ],
    "windows": { "installMode": "passive" }
  }
}
```

Use the generated public key value verbatim; never place the private key in the repository.

- [ ] **Step 4: Grant narrow plugin permissions**

Add `updater:allow-check`, `updater:allow-download`, `updater:allow-install`, and `process:allow-restart` to `src-tauri/capabilities/default.json`. Do not add updater or process defaults that expose unrelated commands.

- [ ] **Step 5: Sign release artifacts in CI**

Set these environment values on the `tauri-action` build step:

```yaml
TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
```

The generated private key has no password, so the password secret may be an empty repository secret. Record the private key as the `TAURI_SIGNING_PRIVATE_KEY` GitHub Actions secret before publishing the first updater-enabled tag.

- [ ] **Step 6: Verify Rust and configuration**

Run: `cargo check --manifest-path 'src-tauri/Cargo.toml'` and `pnpm typecheck`.

Expected: both commands exit 0 and Tauri capability generation recognizes all four permission identifiers.

- [ ] **Step 7: Commit plugin configuration**

Run:

```powershell
git add -- 'package.json' 'pnpm-lock.yaml' 'src-tauri/Cargo.toml' 'src-tauri/Cargo.lock' 'src-tauri/src/lib.rs' 'src-tauri/tauri.conf.json' 'src-tauri/capabilities/default.json' '.github/workflows/release.yml'
git commit -m 'feat: configure signed Tauri updates'
```

### Task 5: Replace the frontend update lifecycle

**Files:**

- Modify: `src/lib/updater.ts`
- Modify: `src/hooks/useUpdateManager.ts`
- Modify: `src/components/UpdateDownloadDialog.tsx`
- Modify: `src/app/MainWindowView.tsx`
- Modify: `src/App.tsx`
- Modify: `src/i18n.ts`
- Modify: `src/App.css`

- [ ] **Step 1: Wrap the official plugin with project UI types**

Replace invoke-based update calls with `check` from `@tauri-apps/plugin-updater` and `relaunch` from `@tauri-apps/plugin-process`. Retain `readAppVersion`, release-link helpers, and a serializable UI descriptor:

```ts
export interface UpdateInfo {
  body: string | null;
  latestVersion: string;
}

export type UpdatePhase = "downloading" | "preparing" | "installing";
```

Keep the live `Update` object in a ref, not React-rendered props. Convert Started/Progress/Finished events into the existing byte and percentage progress shape.

- [ ] **Step 2: Flush state before install**

In `App.tsx`, define a `prepareForUpdate` callback that captures the main window state, persists the current settings snapshot through `persistSettings`, and awaits `prepareEditorForUpdate()`. Pass it to `useUpdateManager`.

Because `persistSettings` is declared by `useSettingsController`, move the `useUpdateManager` call below that controller call without changing hook ordering conditionally.

- [ ] **Step 3: Implement the safe update sequence**

In `useUpdateManager`, implement:

```ts
await update.download(handleDownloadEvent);
setPhase("preparing");
await prepareForUpdate();
setPhase("installing");
await update.install();
await relaunch();
```

On Windows, Tauri exits during `install()` and the passive installer takes over. On macOS/Linux, `install()` resolves and `relaunch()` restarts the new binary. On any caught error, keep `updateInfo` available for retry, close the blocking dialog, and show the phase-appropriate translated toast.

- [ ] **Step 4: Simplify the progress dialog**

Render phase-specific download/preparation/installation text. Remove the custom cancel command, downloaded-file result, folder reveal action, close button during an active update, and the “open downloaded file” footer. Keep the progress bar and byte count during download; show an indeterminate/full-width treatment during preparation and installation.

- [ ] **Step 5: Remove obsolete props and translations**

Remove `onCancelUpdateDownload`, `onRevealDownloadedUpdate`, `UpdateDownloadResult`, and their call sites from `App.tsx`, `MainWindowView.tsx`, and the dialog. Replace the old downloaded/open-file strings with aligned Chinese and English preparation/install/error text.

- [ ] **Step 6: Verify and commit the frontend lifecycle**

Run: `pnpm test:update-draft`, `pnpm typecheck`, `pnpm check`, and `pnpm build`.

Expected: every command exits 0.

Then commit:

```powershell
git add -- 'src/lib/updater.ts' 'src/hooks/useUpdateManager.ts' 'src/components/UpdateDownloadDialog.tsx' 'src/app/MainWindowView.tsx' 'src/App.tsx' 'src/i18n.ts' 'src/App.css'
git commit -m 'feat: install updates inside Q Note'
```

### Task 6: Remove the legacy downloader and manifest publisher

**Files:**

- Delete: `src-tauri/src/update.rs`
- Delete: `src-tauri/src/repository.rs`
- Delete: `scripts/generate-update-manifest.mjs`
- Delete: `scripts/upload-update-manifest.mjs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/Cargo.lock`
- Modify: `package.json`
- Modify: `.github/workflows/release.yml`

- [ ] **Step 1: Remove custom command registration**

Delete `mod update`, `mod repository`, update download state management, startup package cleanup, and the four legacy update commands from `src-tauri/src/lib.rs`.

- [ ] **Step 2: Remove custom network dependencies and scripts**

Remove `futures-util`, `reqwest`, and `sha2` from `src-tauri/Cargo.toml`. Remove `update:manifest` and `update:manifest:upload` from `package.json`. Regenerate lockfiles with `cargo check --manifest-path 'src-tauri/Cargo.toml'` and `pnpm install --lockfile-only` if needed.

- [ ] **Step 3: Delete obsolete source and workflow job**

Delete the two Rust modules and two manifest scripts. Remove the `release-manifest` job from `.github/workflows/release.yml`; `tauri-action` now publishes updater artifacts and `latest.json`. Keep ordinary installers enabled for legacy clients.

- [ ] **Step 4: Verify no legacy references remain**

Run:

```powershell
rg -n -S 'download_update|install_update_package|update:manifest|q-note-update-download-progress|github_update_manifest_urls' -- 'src' 'src-tauri' 'scripts' 'package.json' '.github/workflows'
```

Expected: no matches.

- [ ] **Step 5: Run full static verification and commit cleanup**

Run: `pnpm test:update-draft`, `pnpm typecheck`, `pnpm check`, `pnpm build`, and `cargo check --manifest-path 'src-tauri/Cargo.toml'`.

Expected: all commands exit 0.

Then commit:

```powershell
git add -A -- 'src-tauri/src/update.rs' 'src-tauri/src/repository.rs' 'scripts/generate-update-manifest.mjs' 'scripts/upload-update-manifest.mjs' 'src-tauri/src/lib.rs' 'src-tauri/Cargo.toml' 'src-tauri/Cargo.lock' 'package.json' '.github/workflows/release.yml'
git commit -m 'refactor: remove legacy update downloader'
```

### Task 7: Produce and inspect a signed Windows updater artifact

**Files:**

- Generated outside source: `src-tauri/target/release/bundle/nsis/*`
- No committed production files expected.

- [ ] **Step 1: Load the local signing key without printing it**

Set `TAURI_SIGNING_PRIVATE_KEY` for the current PowerShell process from `C:\Users\admin\.tauri\q-note.key`. Leave `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` unset because the generated key is unencrypted. Do not echo either value.

- [ ] **Step 2: Build the signed NSIS updater bundle**

Run: `pnpm tauri build --bundles nsis`

Expected: exit 0 and creation of a normal setup executable, an updater archive, and its `.sig` signature under `src-tauri/target/release/bundle/nsis`.

- [ ] **Step 3: Inspect artifact names and repository state**

List only file names and sizes in the NSIS bundle directory. Run `git status --short` and confirm build outputs and private keys are not tracked.

- [ ] **Step 4: Run final verification**

Run: `pnpm test:update-draft`, `pnpm typecheck`, `pnpm check`, `pnpm build`, and `cargo check --manifest-path 'src-tauri/Cargo.toml'` once more.

Expected: all commands exit 0 after the release build.

- [ ] **Step 5: Record the external release prerequisite**

Confirm the final handoff states that `C:\Users\admin\.tauri\q-note.key` must be backed up securely and its exact contents must be added to the repository secret named `TAURI_SIGNING_PRIVATE_KEY` before pushing an updater-enabled release tag. Do not commit or paste the private key.
