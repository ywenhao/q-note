import assert from "node:assert/strict";
import test from "node:test";
import { restorePendingUpdateEditor } from "../src/lib/updateRecovery.ts";
import { DEFAULT_NOTE_COLOR, type NoteDraft } from "../src/types.ts";

function emptyDraft(): NoteDraft {
  return {
    attachments: [],
    color: DEFAULT_NOTE_COLOR,
    content: "recover me",
    pinned: false,
  };
}

test("reopens the editor for a pending update draft", async () => {
  const openedNoteIds: Array<string | null> = [];

  const restored = await restorePendingUpdateEditor(
    async () => ({
      draft: emptyDraft(),
      noteId: "note-1",
      savedAt: 300,
    }),
    async (noteId) => {
      openedNoteIds.push(noteId);
    },
  );

  assert.equal(restored, true);
  assert.deepEqual(openedNoteIds, ["note-1"]);
});

test("reopens a new-note editor when the pending draft has no note id", async () => {
  const openedNoteIds: Array<string | null> = [];

  const restored = await restorePendingUpdateEditor(
    async () => ({
      draft: emptyDraft(),
      noteId: null,
      savedAt: 300,
    }),
    async (noteId) => {
      openedNoteIds.push(noteId);
    },
  );

  assert.equal(restored, true);
  assert.deepEqual(openedNoteIds, [null]);
});

test("leaves the editor closed when no update draft is pending", async () => {
  let opened = false;

  const restored = await restorePendingUpdateEditor(
    async () => null,
    async () => {
      opened = true;
    },
  );

  assert.equal(restored, false);
  assert.equal(opened, false);
});
