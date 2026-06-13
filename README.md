<p align="center">
  <img src="./app-icon.png" alt="Q Note icon" width="120" height="120" />
</p>

<h1 align="center">Q Note</h1>

<p align="center">
  A small, fast desktop note board for content you copy often.
</p>

<p align="center">
  <a href="./read.md">中文说明</a>
</p>

## Purpose

Q Note is a desktop note app built with Tauri 2, React, TypeScript, Vite+, Tailwind CSS, SQLite, and Drizzle. It is designed for short snippets, image notes, web image URLs, local paths, dropped files, pasted screenshots, quick copy actions, always-on-top mode, a tray icon, launch-at-login, a dockable Q icon, color marking, import/export, and guarded bulk deletion.

## Quick Start

```bash
pnpm install
pnpm tauri dev
```

## Commands

| Command             | Purpose                     |
| ------------------- | --------------------------- |
| `pnpm dev`          | Start the Vite dev server   |
| `pnpm tauri dev`    | Start the Tauri desktop app |
| `pnpm typecheck`    | Run TypeScript build checks |
| `pnpm check`        | Run Vite+ checks            |
| `pnpm check:fix`    | Fix Vite+ check issues      |
| `pnpm format`       | Format with Vite+           |
| `pnpm format:check` | Check formatting            |
| `pnpm build`        | Build the frontend          |

## Features

| Feature          | Details                                                                                                     |
| ---------------- | ----------------------------------------------------------------------------------------------------------- |
| Language switch  | Toggle Chinese and English from the toolbar; the choice is saved locally                                    |
| Quick copy       | Click a card to copy its text; attachment-only notes copy attachment values                                 |
| Card management  | Create, edit, delete, pin, recolor, and resize note cards                                                   |
| Delete all       | Remove every note only after a red confirmation dialog                                                      |
| Images and files | Add images, drop files/images, paste screenshots, use web image URLs, local paths, and base64 fallback data |
| Image preview    | Click editor thumbnails to view a larger image                                                              |
| Topmost window   | Toggle always-on-top from the toolbar or right-click menu                                                   |
| Tray icon        | Keep a resident Q icon in the system tray; click it to show the app                                         |
| Launch at login  | Enable or disable startup launch from Settings; it is off by default                                        |
| Q icon mode      | Collapse to a yellow Q icon, drag it, snap it to screen edges, and hover to restore the full UI             |
| Persistence      | Notes, attachments, colors, card heights, window size, topmost state, and language are saved in SQLite      |
| Import/export    | Export and import notes plus local settings as JSON                                                         |

## Stack

| Area          | Tooling                                               |
| ------------- | ----------------------------------------------------- |
| Desktop shell | Tauri 2                                               |
| Frontend      | React 19 + TypeScript                                 |
| Build         | Vite 8 + Vite+                                        |
| Styling       | Tailwind CSS 4 + CSS                                  |
| Storage       | SQLite + `@tauri-apps/plugin-sql` + Drizzle proxy     |
| Files         | `@tauri-apps/plugin-dialog` + `@tauri-apps/plugin-fs` |
| Icons         | lucide-react + yellow Q app icon                      |

## License

[MIT](./LICENSE)
