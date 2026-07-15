import assert from "node:assert/strict";
import test from "node:test";
import { clampContextMenuPosition } from "../src/components/contextMenuPosition.ts";

test("keeps a menu at the requested position when it fits", () => {
  assert.deepEqual(
    clampContextMenuPosition({
      menuHeight: 120,
      menuWidth: 152,
      viewportHeight: 600,
      viewportWidth: 800,
      x: 100,
      y: 120,
    }),
    { left: 100, top: 120 },
  );
});

test("moves a menu away from the right and bottom edges", () => {
  assert.deepEqual(
    clampContextMenuPosition({
      menuHeight: 120,
      menuWidth: 152,
      viewportHeight: 600,
      viewportWidth: 800,
      x: 760,
      y: 560,
    }),
    { left: 640, top: 472 },
  );
});

test("keeps the menu inside the top and left safety margin", () => {
  assert.deepEqual(
    clampContextMenuPosition({
      menuHeight: 120,
      menuWidth: 152,
      viewportHeight: 600,
      viewportWidth: 800,
      x: -20,
      y: 0,
    }),
    { left: 8, top: 8 },
  );
});

test("uses non-negative coordinates when the viewport is smaller than the menu", () => {
  assert.deepEqual(
    clampContextMenuPosition({
      menuHeight: 120,
      menuWidth: 152,
      viewportHeight: 80,
      viewportWidth: 100,
      x: 40,
      y: 30,
    }),
    { left: 0, top: 0 },
  );
});
