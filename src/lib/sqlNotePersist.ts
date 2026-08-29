import type { Note, NoteAttachment } from "../types.ts";
import { runSqlTransaction, type SqlExecute } from "./sqlTransaction.ts";
import type { NotePersistClient } from "./notePersist.ts";

export const Q_NOTE_SCHEMA_SQL = `
CREATE TABLE IF NOT EXISTS notes (
  id TEXT PRIMARY KEY NOT NULL,
  content TEXT NOT NULL,
  color TEXT NOT NULL,
  pinned INTEGER NOT NULL DEFAULT 0,
  sort_order INTEGER NOT NULL DEFAULT 0,
  text_height INTEGER,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS attachments (
  id TEXT PRIMARY KEY NOT NULL,
  note_id TEXT NOT NULL,
  kind TEXT NOT NULL DEFAULT 'image',
  source TEXT NOT NULL,
  value TEXT NOT NULL,
  name TEXT,
  created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS settings (
  key TEXT PRIMARY KEY NOT NULL,
  value TEXT NOT NULL
);
`;

export function createSqlNotePersistClient(execute: SqlExecute): NotePersistClient {
  let depth = 0;

  return {
    async transaction(work) {
      if (depth > 0) {
        return work();
      }
      depth += 1;
      try {
        return await runSqlTransaction(execute, work);
      } finally {
        depth -= 1;
      }
    },
    async upsertNote(note) {
      await execute(
        `INSERT INTO notes (id, content, color, pinned, sort_order, text_height, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET
           content = excluded.content,
           color = excluded.color,
           pinned = excluded.pinned,
           sort_order = excluded.sort_order,
           text_height = excluded.text_height,
           updated_at = excluded.updated_at`,
        [
          note.id,
          note.content,
          note.color,
          note.pinned ? 1 : 0,
          note.sortOrder,
          note.textHeight,
          note.createdAt,
          note.updatedAt,
        ],
      );
    },
    async deleteAttachmentsForNote(noteId) {
      await execute("DELETE FROM attachments WHERE note_id = ?", [noteId]);
    },
    async insertAttachment(noteId, attachment: NoteAttachment) {
      await execute(
        `INSERT INTO attachments (id, note_id, kind, source, value, name, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)`,
        [
          attachment.id,
          noteId,
          attachment.kind,
          attachment.source,
          attachment.value,
          attachment.name ?? null,
          attachment.createdAt,
        ],
      );
    },
    async deleteNote(noteId) {
      await execute("DELETE FROM notes WHERE id = ?", [noteId]);
    },
    async deleteAllNotes() {
      await execute("DELETE FROM attachments");
      await execute("DELETE FROM notes");
    },
    async deleteAllSettings() {
      await execute("DELETE FROM settings");
    },
    async upsertSettings(key, value) {
      await execute(
        `INSERT INTO settings (key, value) VALUES (?, ?)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value`,
        [key, value],
      );
    },
    async updateNoteOrder(note: Pick<Note, "id" | "pinned" | "sortOrder">) {
      await execute("UPDATE notes SET pinned = ?, sort_order = ? WHERE id = ?", [
        note.pinned ? 1 : 0,
        note.sortOrder,
        note.id,
      ]);
    },
  };
}
