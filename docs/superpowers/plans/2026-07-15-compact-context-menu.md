# Compact Context Menu Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver smaller, focused context menus that stay visible at every viewport edge, then publish patch release 0.1.6.

**Architecture:** Add a pure positioning helper beside the context-menu component, test it with Node's built-in TypeScript test support, and let the component measure and clamp itself after rendering. Keep menu composition in `menuItems.tsx`, removing only redundant main-window global actions while leaving the dock menu unchanged.

**Tech Stack:** React 19, TypeScript 6, CSS, Node test runner, pnpm, Tauri 2 release tooling.

---

### Task 1: Test viewport-safe positioning

**Files:**

- Create: `src/components/contextMenuPosition.test.ts`
- Create: `src/components/contextMenuPosition.ts`
- Modify: `package.json`

- [ ] **Step 1: Write failing tests**

Add tests that request positions in the center, beyond the right/bottom edges, at the top/left edges, and inside a viewport smaller than the menu. Assert an 8px margin whenever the viewport permits it and non-negative coordinates otherwise.

- [ ] **Step 2: Run the focused test and verify RED**

Run: `pnpm test:context-menu`

Expected: failure because `clampContextMenuPosition` does not exist yet.

- [ ] **Step 3: Implement the pure helper**

Export `clampContextMenuPosition` with numeric inputs for requested coordinates, menu dimensions, viewport dimensions, and an optional margin. Clamp each axis independently and handle undersized viewports.

- [ ] **Step 4: Run the focused test and verify GREEN**

Run: `pnpm test:context-menu`

Expected: all positioning tests pass.

### Task 2: Apply measurement and boundary detection

**Files:**

- Modify: `src/components/ContextMenu.tsx`

- [ ] **Step 1: Add a menu ref and measured position state**

Use `useLayoutEffect` to measure `getBoundingClientRect()`, calculate a safe position with the helper, and update local position state only when coordinates change.

- [ ] **Step 2: Recalculate on resize**

Attach a `resize` listener while the menu is mounted and remove it during cleanup.

- [ ] **Step 3: Render at the safe measured position**

Use the computed position for `left` and `top`; preserve the existing close and click behavior.

### Task 3: Make the menu compact and focused

**Files:**

- Modify: `src/App.css`
- Modify: `src/lib/menuItems.tsx`
- Modify: `src/features/menu/useMenuController.ts`

- [ ] **Step 1: Reduce custom-menu dimensions**

Set a 152px minimum width, 13px text/icons, 6px by 8px item padding, 6px gap, and a smaller shadow. Add an 8px viewport-based maximum height and vertical overflow.

- [ ] **Step 2: Remove redundant main-window global items**

Remove settings, always-on-top, auto-start, and dock switching from `createMainContextItems`. Keep note actions and empty-area new/delete-all actions. Do not change `createDockMenuItems`.

- [ ] **Step 3: Remove obsolete main-menu dependencies**

Narrow `MainMenuOptions` and the `createMainContextItems` call so the type checker catches any leftover wiring.

### Task 4: Verify behavior

**Files:**

- No production files expected.

- [ ] **Step 1: Run automated checks**

Run `pnpm test:context-menu`, `pnpm typecheck`, `pnpm check`, and `pnpm build`. Every command must exit 0 with no new errors.

- [ ] **Step 2: Run visual checks in the local app**

Start `pnpm dev`, open the app in the in-app browser, and inspect menus opened in the center and near all four edges. Confirm compact sizing, no clipping, correct main-menu contents, and unchanged dock-menu contents.

### Task 5: Release and push

**Files:**

- Modify through release tooling: `package.json`
- Modify through release tooling: `src-tauri/tauri.conf.json`
- Modify through release tooling: `src-tauri/Cargo.toml`
- Modify through release tooling as needed: `src-tauri/Cargo.lock`

- [ ] **Step 1: Commit the feature changes**

Create a conventional feature commit after confirming the worktree contains only intended files.

- [ ] **Step 2: Create patch release 0.1.6**

Run: `pnpm release:patch`

Expected: release commit `release: v0.1.6` and tag `v0.1.6` are created.

- [ ] **Step 3: Push branch and tags once**

Run: `git push --follow-tags origin main`

Expected: `main` and `v0.1.6` are updated on `origin`.
