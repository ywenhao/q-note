# Compact Context Menu Design

## Goal

Make Q Note's custom context menus smaller and more focused, while ensuring they remain fully visible near every window edge.

## Scope

- Reduce the custom menu's minimum width, item padding, icon size, gap, radius, and shadow footprint.
- Keep note-specific actions when right-clicking a note: copy, edit, pin/unpin, and delete.
- Keep empty-area actions: new note and delete all when notes exist.
- Remove settings, always-on-top, auto-start, and dock switching from the main-window context menu because these controls already have dedicated UI entry points.
- Keep the floating-dock menu actions because the compact dock does not expose equivalent controls directly.
- Measure the rendered custom menu and clamp its position inside the viewport with an 8px safe margin.
- Limit menu height to the viewport and allow vertical scrolling on unusually small windows.

## Positioning Behavior

The requested pointer position is treated as the preferred top-left corner. After render, the menu's measured width and height are used to calculate the maximum valid `left` and `top`. Both coordinates are clamped between the safe margin and their maximum valid values. The calculation also handles viewports smaller than the menu without returning negative coordinates.

The position is recalculated when the pointer coordinates, item count, menu dimensions, or viewport size changes. A window resize listener keeps an already-open menu visible.

## Visual Values

- Safe viewport margin: 8px
- Minimum width: 152px
- Button padding: 6px 8px
- Item gap: 6px
- Font size: 13px
- Icon size: 13px
- Border radius: 7px
- Maximum height: viewport height minus 16px

## Validation

- Unit-test the pure coordinate clamping helper for normal placement, right/bottom overflow, top/left safety, and very small viewports.
- Run the focused unit test, TypeScript checks, Vite+ checks, and production build.
- Open the local app in the in-app browser and manually verify compact sizing plus all four edge cases.

## Release

After validation, create a patch release from version 0.1.5 to 0.1.6 using the project's release script, then push `main` and the new tag to `origin` in one push operation.
