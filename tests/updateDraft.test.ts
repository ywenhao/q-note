import assert from "node:assert/strict";
import test from "node:test";
import {
  createNoteDraft,
  isEditorDraftDirty,
  normalizePendingUpdateDraft,
} from "../src/lib/updateDraft.ts";
import {
  DEFAULT_NOTE_COLOR,
  type Note,
  type NoteAttachment,
  type NoteDraft,
} from "../src/types.ts";

function makeAttachment(overrides: Partial<NoteAttachment> = {}): NoteAttachment {
  return {
    id: "attachment-1",
    kind: "image",
    source: "url",
    value: "https://example.com/image.png",
    name: "image.png",
    createdAt: 100,
    ...overrides,
  };
}

function makeNote(overrides: Partial<Note> = {}): Note {
  return {
    id: "note-1",
    content: "Existing note",
    color: DEFAULT_NOTE_COLOR,
    pinned: false,
    sortOrder: 0,
    textHeight: null,
    attachments: [makeAttachment()],
    createdAt: 100,
    updatedAt: 200,
    ...overrides,
  };
}

function emptyDraft(overrides: Partial<NoteDraft> = {}): NoteDraft {
  return {
    attachments: [],
    color: DEFAULT_NOTE_COLOR,
    content: "",
    pinned: false,
    ...overrides,
  };
}

test("creates a draft from an existing note", () => {
  const note = makeNote();

  assert.deepEqual(createNoteDraft(note), {
    attachments: note.attachments,
    color: note.color,
    content: note.content,
    pinned: note.pinned,
  });
});

test("creates an empty draft for a new note", () => {
  assert.deepEqual(createNoteDraft(null), emptyDraft());
});

test("an unchanged existing note is not dirty", () => {
  const note = makeNote();

  assert.equal(isEditorDraftDirty(createNoteDraft(note), note), false);
});

test("a new note with content is dirty", () => {
  assert.equal(isEditorDraftDirty(emptyDraft({ content: "recover me" }), null), true);
});

test("a new note with an attachment is dirty", () => {
  assert.equal(isEditorDraftDirty(emptyDraft({ attachments: [makeAttachment()] }), null), true);
});

test("attachment changes are dirty", () => {
  const note = makeNote();

  assert.equal(
    isEditorDraftDirty(
      {
        ...createNoteDraft(note),
        attachments: [...note.attachments, makeAttachment({ id: "attachment-2" })],
      },
      note,
    ),
    true,
  );
});

test("normalizes a valid pending draft without sharing attachment objects", () => {
  const attachment = makeAttachment();
  const value = {
    draft: emptyDraft({ attachments: [attachment], content: "recover me" }),
    noteId: null,
    savedAt: 300,
  };

  const normalized = normalizePendingUpdateDraft(value);

  assert.deepEqual(normalized, value);
  assert.notEqual(normalized?.draft.attachments, value.draft.attachments);
  assert.notEqual(normalized?.draft.attachments[0], attachment);
});

test("rejects malformed pending draft fields", () => {
  assert.equal(normalizePendingUpdateDraft({ noteId: 3, draft: null, savedAt: 300 }), null);
  assert.equal(
    normalizePendingUpdateDraft({
      draft: emptyDraft({
        attachments: [{ ...makeAttachment(), source: "unknown" as "url" }],
      }),
      noteId: "note-1",
      savedAt: 300,
    }),
    null,
  );
});
