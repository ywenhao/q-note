# In-App Updater Design

## Goal

Replace the custom package downloader with Tauri 2's signed updater so Q Note can safely install updates from inside the app. Windows must close Q Note and run the installer in passive mode with a native progress window. macOS must replace the application bundle without asking the user to mount a DMG or click an installer. A successful update must reopen Q Note automatically.

## User Experience

The existing update entry point and download dialog remain. Checking for an update uses the official updater endpoint. Once the user starts an update, the dialog shows signed-package download progress and cannot be dismissed during installation.

After the download completes, Q Note captures the latest window settings and asks the editor window to persist any unsaved draft. Installation starts only after both operations succeed. On Windows, Tauri exits the app and launches the NSIS updater in `passive` mode, which displays a small native progress window without requiring clicks. On macOS, Tauri verifies and replaces the `.app` bundle, then the process plugin relaunches Q Note. The updated app restores any saved editor draft.

If checking, downloading, draft persistence, signature verification, or installation fails, Q Note stays on the current version and shows an error. A failed pre-install step must never close the app. A failed installation clears only the update-session marker, not notes, settings, or the recovered draft.

## Architecture

### Official updater

- Add the Tauri updater and process plugins in Rust and TypeScript.
- Enable updater artifacts in `tauri.conf.json`, configure the updater public key and GitHub `latest.json` endpoint, and use Windows `passive` install mode.
- Grant only the updater check/download/install and process relaunch permissions required by the three app windows.
- Replace the custom Rust update commands with the plugin APIs. Keep ordinary platform installers in releases so clients running the old updater still have a migration path.
- Remove the custom HTTP downloader, checksum code, temporary-package cleanup, manifest scripts, and Rust dependencies that become unused.

The frontend deliberately calls `download()` and `install()` separately instead of `downloadAndInstall()`. This provides a safe boundary after the signed package is downloaded but before Windows automatically exits the running app.

### Release pipeline and signing

The release workflow supplies `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` from GitHub Actions secrets. Tauri creates platform updater archives and signatures; `tauri-action` publishes them with `latest.json`. The public key is embedded in the app configuration. The private key never enters the repository.

The first release containing this change remains compatible with the existing custom updater because it still publishes the normal NSIS/MSI, DMG, and Linux bundles. Clients on that older version may need to complete one final legacy installation; all later updates use the signed in-app flow.

### Draft persistence

Add an `UpdateEditorDraft` record containing the edited note id, draft fields, original note snapshot, and save timestamp. Store it through `src/lib/storage.ts` under a dedicated settings-table key so it is durable but excluded from normal settings and data export.

`NoteEditor` reports its current draft to `EditorWindow`. When the main window sends a prepare-update request, the editor compares the current draft with its original note:

- If the editor is hidden or unchanged, it acknowledges without storing a draft.
- If it is visible and dirty, it stores the draft and then acknowledges.
- If persistence fails or no acknowledgement arrives before the timeout, the update is aborted before installation.

On startup, `EditorWindow` reads the pending draft. When present, it loads the matching note, applies the saved draft, shows and focuses the editor, and continues persisting changes until the user saves or cancels. Saving or canceling clears the pending draft. This preserves recovery without silently turning an unfinished draft into a formal note.

### App-state flush

Notes and ordinary setting changes are already written at the time the user performs them. Before installation, the main window additionally captures any debounced window position/size and saves the current settings snapshot. Only after that write and the editor acknowledgement succeed does the updater call `install()`.

## State Flow

1. `check()` returns a signed update descriptor from `latest.json`.
2. `download()` emits Started, Progress, and Finished events to the existing dialog.
3. Q Note persists the current main-window settings.
4. The main window requests an editor draft snapshot and waits for acknowledgement.
5. `install()` verifies and installs the downloaded artifact.
6. Windows exits automatically and the passive installer completes the replacement; macOS replaces the bundle in-process.
7. The updated application relaunches and restores any pending editor draft.

## Error Handling

- Check failures retain the current manual-check toast behavior.
- Download failures close the progress dialog and leave the update available for retry.
- Draft or settings flush failures stop before `install()` and report that data could not be secured.
- Signature or install failures keep the current executable and leave the persisted draft available for recovery.
- The progress dialog does not expose the old “open downloaded file” fallback because updater archives are implementation artifacts, not user-installable packages.
- Update packages are accepted only after Tauri's signature verification; SHA-256 metadata from the former downloader is no longer the trust boundary.

## Validation

- Unit-test draft normalization, dirty comparison, and restoration decisions for new notes, existing notes, attachments, malformed stored data, and unchanged editors.
- Run the focused tests, TypeScript checks, Vite+ checks, the frontend production build, and Rust `cargo check`.
- Verify capability and updater configuration contains only the intended updater/process permissions.
- Confirm a release build produces signed Windows and macOS updater artifacts plus `latest.json` when signing secrets are present.
- On Windows, test an older signed build updating to a newer build: in-app progress, editor snapshot, automatic exit, passive installer progress, restart, version change, and draft restoration.
- On macOS, test the same path and confirm there is no DMG interaction.
- Test failure cases for network interruption, invalid signature, editor acknowledgement timeout, and install failure; the old version and user data must remain usable.

## Compatibility Decisions

- Automatic relaunch is the default after a successful install.
- Windows uses `passive`, not `quiet`, so progress remains visible and elevation can still be requested when required.
- The official GitHub updater endpoint replaces the custom China-mirror download selection in the first implementation. Reintroducing regional artifact routing requires a trusted dynamic updater endpoint and is outside this change.
- Linux continues to receive official updater artifacts where supported, but Windows and macOS are the release-blocking acceptance platforms for this work.
