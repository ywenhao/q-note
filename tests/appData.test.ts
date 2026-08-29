import assert from "node:assert/strict";
import test from "node:test";
import { createExportPayload, normalizeImportPayload } from "../src/lib/appData.ts";
import { DEFAULT_NOTE_COLOR } from "../src/types.ts";

test("normalizeImportPayload keeps valid notes and drops invalid ones", () => {
  const data = normalizeImportPayload({
    notes: [
      {
        id: "keep",
        content: "hello",
        color: DEFAULT_NOTE_COLOR,
        pinned: true,
        sortOrder: 3,
        textHeight: 44,
        attachments: [
          {
            id: "att-1",
            kind: "image",
            source: "url",
            value: "https://example.com/a.png",
            createdAt: 10,
          },
          { value: 123 },
        ],
        createdAt: 1,
        updatedAt: 2,
      },
      null,
      "nope",
    ],
    settings: { language: "en", alwaysOnTop: true },
  });

  assert.equal(data.notes.length, 1);
  assert.equal(data.notes[0]?.id, "keep");
  assert.equal(data.notes[0]?.pinned, true);
  assert.equal(data.notes[0]?.attachments.length, 1);
  assert.equal(data.notes[0]?.attachments[0]?.value, "https://example.com/a.png");
  assert.equal(data.settings.language, "en");
  assert.equal(data.settings.alwaysOnTop, true);
});

test("normalizeImportPayload infers image attachments from data URLs", () => {
  const data = normalizeImportPayload({
    notes: [
      {
        content: "",
        attachments: [{ source: "data", value: "data:image/png;base64,abc" }],
      },
    ],
  });

  assert.equal(data.notes[0]?.attachments[0]?.kind, "image");
  assert.equal(data.notes[0]?.attachments[0]?.source, "data");
});

test("normalizeImportPayload rejects a non-object payload", () => {
  assert.throws(() => normalizeImportPayload("nope"), /Invalid Q Note data/);
});

test("createExportPayload includes version, timestamp, notes, and settings", () => {
  const settings = {
    language: "en" as const,
    alwaysOnTop: false,
    autoStart: false,
    dockOnEdge: false,
    docked: false,
    dockEdge: null,
    keepFullMain: false,
    window: null,
  };
  const payload = createExportPayload({
    notes: [],
    settings,
  });

  assert.equal(payload.version, 1);
  assert.equal(typeof payload.exportedAt, "string");
  assert.equal(Number.isNaN(Date.parse(payload.exportedAt)), false);
  assert.deepEqual(payload.notes, []);
  assert.deepEqual(payload.settings, settings);
});
