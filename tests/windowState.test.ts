import assert from "node:assert/strict";
import test from "node:test";
import { normalizeWindowState, windowSizeMatches } from "../src/lib/windowState.ts";

test("normalizeWindowState keeps physical sizes without logical clamping", () => {
  assert.deepEqual(normalizeWindowState({ width: 900, height: 1200, x: 10, y: 20 }), {
    width: 900,
    height: 1200,
    x: 10,
    y: 20,
  });
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
