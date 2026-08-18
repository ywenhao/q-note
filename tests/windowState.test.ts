import assert from "node:assert/strict";
import test from "node:test";
import {
  CONTENT_COLUMN_MAX_WIDTH,
  MAX_WINDOW_HEIGHT,
  MAX_WINDOW_WIDTH,
} from "../src/types.ts";
import {
  clampPhysicalWindowSize,
  getEffectiveMaxWindowSize,
  normalizeWindowState,
  windowSizeMatches,
} from "../src/lib/windowState.ts";

test("normalizeWindowState keeps physical sizes without logical clamping", () => {
  assert.deepEqual(normalizeWindowState({ width: 900, height: 1200, x: 10, y: 20 }), {
    width: 900,
    height: 1200,
    x: 10,
    y: 20,
  });
});

test("getEffectiveMaxWindowSize follows the work area on small screens", () => {
  assert.deepEqual(
    getEffectiveMaxWindowSize({ width: 1500, height: 1200 }, { width: 1280, height: 720 }),
    { width: 1280, height: 720 },
  );
  assert.deepEqual(
    getEffectiveMaxWindowSize({ width: 1500, height: 1200 }, { width: 1920, height: 1080 }),
    { width: 1500, height: 1200 },
  );
});

test("clampPhysicalWindowSize converges oversized restored windows", () => {
  assert.deepEqual(
    clampPhysicalWindowSize(
      { width: 2560, height: 1440 },
      { width: 300, height: 450 },
      { width: 1500, height: 1200 },
    ),
    { width: 1500, height: 1200 },
  );
});

test("clampPhysicalWindowSize keeps windows at or above the minimum size", () => {
  assert.deepEqual(
    clampPhysicalWindowSize(
      { width: 120, height: 80 },
      { width: 300, height: 450 },
      { width: 1500, height: 1200 },
    ),
    { width: 300, height: 450 },
  );
});

test("normalizeWindowState drops invalid window sizes", () => {
  assert.equal(normalizeWindowState({ width: 0, height: 450, x: 0, y: 0 }), null);
});

test("windowSizeMatches compares width and height only", () => {
  assert.equal(
    windowSizeMatches(
      { width: 900, height: 1200, x: 1, y: 2 },
      { width: 900, height: 1200, x: 99, y: 88 },
    ),
    true,
  );
  assert.equal(
    windowSizeMatches(
      { width: 900, height: 1200, x: 1, y: 2 },
      { width: 800, height: 1200, x: 1, y: 2 },
    ),
    false,
  );
});

test("content column stays inside the main window max size", () => {
  assert.equal(MAX_WINDOW_WIDTH, 1000);
  assert.equal(MAX_WINDOW_HEIGHT, 800);
  assert.equal(CONTENT_COLUMN_MAX_WIDTH, 980);
  assert.ok(CONTENT_COLUMN_MAX_WIDTH < MAX_WINDOW_WIDTH);
});
