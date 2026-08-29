import assert from "node:assert/strict";
import test from "node:test";
import {
  createDefaultSettings,
  normalizeSettings,
  parseStoredSettings,
} from "../src/lib/settingsState.ts";

test("parseStoredSettings accepts a valid settings JSON string", () => {
  const parsed = parseStoredSettings(
    JSON.stringify({
      language: "zh",
      alwaysOnTop: true,
      autoStart: true,
      dockOnEdge: true,
      docked: false,
      dockEdge: "right",
      keepFullMain: false,
      window: { width: 360, height: 500, x: 12, y: 24 },
    }),
  );

  assert.equal(parsed.language, "zh");
  assert.equal(parsed.alwaysOnTop, true);
  assert.equal(parsed.autoStart, true);
  assert.equal(parsed.dockEdge, "right");
  assert.deepEqual(parsed.window, { width: 360, height: 500, x: 12, y: 24 });
});

test("parseStoredSettings uses defaults when JSON is corrupt", () => {
  const parsed = parseStoredSettings("{not-json");
  const defaults = createDefaultSettings();

  assert.deepEqual(parsed, defaults);
});

test("parseStoredSettings uses defaults for empty or non-object JSON", () => {
  assert.deepEqual(parseStoredSettings("null"), createDefaultSettings());
  assert.deepEqual(parseStoredSettings("[]"), createDefaultSettings());
});

test("normalizeSettings fills missing fields without dropping valid ones", () => {
  const parsed = normalizeSettings({ language: "en", alwaysOnTop: 1 });

  assert.equal(parsed.language, "en");
  assert.equal(parsed.alwaysOnTop, true);
  assert.equal(parsed.autoStart, false);
  assert.equal(parsed.docked, false);
  assert.equal(parsed.dockEdge, null);
  assert.equal(parsed.window, null);
});

test("normalizeSettings ignores unknown languages and dock edges", () => {
  const parsed = normalizeSettings({ language: "fr", dockEdge: "diagonal" });

  assert.equal(parsed.language === "zh" || parsed.language === "en", true);
  assert.equal(parsed.dockEdge, null);
});
