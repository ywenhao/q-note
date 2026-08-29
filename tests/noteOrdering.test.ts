import assert from "node:assert/strict";
import test from "node:test";
import { getTopSortOrder, normalizeManualOrder, sortNotes } from "../src/lib/noteOrdering.ts";
import { DEFAULT_NOTE_COLOR, type Note } from "../src/types.ts";

function makeNote(overrides: Partial<Note> = {}): Note {
  return {
    id: "note",
    content: "",
    color: DEFAULT_NOTE_COLOR,
    pinned: false,
    sortOrder: 0,
    textHeight: null,
    attachments: [],
    createdAt: 1,
    updatedAt: 1,
    ...overrides,
  };
}

test("sortNotes keeps pinned notes first, then sortOrder, then newer updates", () => {
  const notes = [
    makeNote({ id: "plain-old", pinned: false, sortOrder: 1, updatedAt: 10 }),
    makeNote({ id: "pinned-new", pinned: true, sortOrder: 1, updatedAt: 30 }),
    makeNote({ id: "plain-new", pinned: false, sortOrder: 1, updatedAt: 40 }),
    makeNote({ id: "pinned-old", pinned: true, sortOrder: 0, updatedAt: 5 }),
  ];

  assert.deepEqual(
    sortNotes(notes).map((note) => note.id),
    ["pinned-old", "pinned-new", "plain-new", "plain-old"],
  );
});

test("getTopSortOrder places a new note above the current group", () => {
  const notes = [
    makeNote({ id: "a", pinned: true, sortOrder: -2 }),
    makeNote({ id: "b", pinned: true, sortOrder: -1 }),
    makeNote({ id: "c", pinned: false, sortOrder: 0 }),
  ];

  assert.equal(getTopSortOrder(notes, true), -3);
  assert.equal(getTopSortOrder(notes, false), -1);
  assert.equal(getTopSortOrder([], true), 0);
});

test("normalizeManualOrder assigns sequential sort orders with pinned notes first", () => {
  const notes = [
    makeNote({ id: "plain", pinned: false, sortOrder: 99 }),
    makeNote({ id: "pinned", pinned: true, sortOrder: 50 }),
  ];

  assert.deepEqual(
    normalizeManualOrder(notes).map((note) => ({ id: note.id, sortOrder: note.sortOrder })),
    [
      { id: "pinned", sortOrder: 0 },
      { id: "plain", sortOrder: 1 },
    ],
  );
});
