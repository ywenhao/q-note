import assert from "node:assert/strict";
import test from "node:test";
import {
  createMemoryNotePersistClient,
  persistDeleteNote,
  persistReplaceAppData,
  persistSaveNote,
  persistSaveNotesOrder,
} from "../src/lib/notePersist.ts";
import { createDefaultSettings } from "../src/lib/settingsState.ts";
import { DEFAULT_NOTE_COLOR, type Note, type NoteAttachment } from "../src/types.ts";

function makeAttachment(overrides: Partial<NoteAttachment> = {}): NoteAttachment {
  return {
    id: "att-1",
    kind: "image",
    source: "data",
    value: "data:image/png;base64,abc",
    name: "shot.png",
    createdAt: 100,
    ...overrides,
  };
}

function makeNote(overrides: Partial<Note> = {}): Note {
  return {
    id: "note-1",
    content: "original",
    color: DEFAULT_NOTE_COLOR,
    pinned: false,
    sortOrder: 0,
    textHeight: null,
    attachments: [makeAttachment()],
    createdAt: 10,
    updatedAt: 20,
    ...overrides,
  };
}

test("saveNote replaces attachments atomically", async () => {
  const original = makeNote();
  const client = createMemoryNotePersistClient({ notes: [original] });
  const next = makeNote({
    content: "updated",
    updatedAt: 30,
    attachments: [
      makeAttachment({ id: "att-2", value: "data:image/png;base64,def" }),
      makeAttachment({ id: "att-3", value: "https://example.com/a.png", source: "url" }),
    ],
  });

  await persistSaveNote(client, next);

  const saved = client.getNotes();
  assert.equal(saved.length, 1);
  assert.equal(saved[0]?.content, "updated");
  assert.deepEqual(
    saved[0]?.attachments.map((attachment) => attachment.id),
    ["att-2", "att-3"],
  );
});

test("saveNote rolls back when an attachment insert fails", async () => {
  const original = makeNote();
  const client = createMemoryNotePersistClient({ notes: [original] });
  client.failOn("insertAttachment", 2);
  const next = makeNote({
    content: "updated",
    attachments: [
      makeAttachment({ id: "att-2" }),
      makeAttachment({ id: "att-3", value: "https://example.com/a.png", source: "url" }),
    ],
  });

  await assert.rejects(() => persistSaveNote(client, next), /insertAttachment/);

  const saved = client.getNotes();
  assert.equal(saved[0]?.content, "original");
  assert.deepEqual(
    saved[0]?.attachments.map((attachment) => attachment.id),
    ["att-1"],
  );
});

test("replaceAppData is atomic when a later note insert fails", async () => {
  const existing = makeNote();
  const settings = { ...createDefaultSettings(), language: "zh" as const, alwaysOnTop: true };
  const client = createMemoryNotePersistClient({ notes: [existing], settings });
  client.failOn("upsertNote", 2);

  await assert.rejects(
    () =>
      persistReplaceAppData(client, {
        notes: [
          makeNote({ id: "imported-1", content: "one" }),
          makeNote({ id: "imported-2", content: "two" }),
        ],
        settings: { ...createDefaultSettings(), language: "en" },
      }),
    /upsertNote/,
  );

  const notes = client.getNotes();
  assert.equal(notes.length, 1);
  assert.equal(notes[0]?.id, "note-1");
  assert.equal(notes[0]?.content, "original");
  assert.match(client.getSettingsValue("app") ?? "", /"language":"zh"/);
});

test("replaceAppData replaces notes and settings on success", async () => {
  const client = createMemoryNotePersistClient({
    notes: [makeNote()],
    settings: createDefaultSettings(),
  });

  await persistReplaceAppData(client, {
    notes: [makeNote({ id: "imported-1", content: "imported", attachments: [] })],
    settings: { ...createDefaultSettings(), language: "en", alwaysOnTop: true },
  });

  const notes = client.getNotes();
  assert.equal(notes.length, 1);
  assert.equal(notes[0]?.content, "imported");
  assert.match(client.getSettingsValue("app") ?? "", /"alwaysOnTop":true/);
});

test("saveNotesOrder rolls back a partial reorder", async () => {
  const first = makeNote({ id: "a", sortOrder: 0 });
  const second = makeNote({ id: "b", sortOrder: 1, attachments: [] });
  const client = createMemoryNotePersistClient({ notes: [first, second] });
  client.failOn("updateNoteOrder", 2);

  await assert.rejects(
    () =>
      persistSaveNotesOrder(client, [
        { ...first, sortOrder: 5 },
        { ...second, sortOrder: 6 },
      ]),
    /updateNoteOrder/,
  );

  const notes = Object.fromEntries(client.getNotes().map((note) => [note.id, note.sortOrder]));
  assert.equal(notes.a, 0);
  assert.equal(notes.b, 1);
});

test("deleteNote removes the note only after both writes succeed", async () => {
  const client = createMemoryNotePersistClient({ notes: [makeNote()] });
  client.failOn("deleteNote");

  await assert.rejects(() => persistDeleteNote(client, "note-1"), /deleteNote/);
  assert.equal(client.getNotes().length, 1);

  await persistDeleteNote(client, "note-1");
  assert.equal(client.getNotes().length, 0);
});
