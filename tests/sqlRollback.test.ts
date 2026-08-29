import assert from "node:assert/strict";
import { DatabaseSync } from "node:sqlite";
import test from "node:test";
import { persistReplaceAppData, persistSaveNote } from "../src/lib/notePersist.ts";
import { createDefaultSettings } from "../src/lib/settingsState.ts";
import { createSqlNotePersistClient, Q_NOTE_SCHEMA_SQL } from "../src/lib/sqlNotePersist.ts";
import { runSqlTransaction, type SqlExecute } from "../src/lib/sqlTransaction.ts";
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

function createSqliteExecute(db: DatabaseSync): SqlExecute {
  return async (sql, params = []) => {
    if (params.length === 0) {
      db.exec(sql);
      return;
    }
    db.prepare(sql).run(...params);
  };
}

function openAppDatabase() {
  const db = new DatabaseSync(":memory:");
  db.exec(Q_NOTE_SCHEMA_SQL);
  return db;
}

function readNotes(db: DatabaseSync) {
  return db.prepare("SELECT id, content FROM notes ORDER BY id").all() as Array<{
    id: string;
    content: string;
  }>;
}

function readAttachments(db: DatabaseSync) {
  return db.prepare("SELECT id, note_id, value FROM attachments ORDER BY id").all() as Array<{
    id: string;
    note_id: string;
    value: string;
  }>;
}

function readSettings(db: DatabaseSync) {
  return db.prepare("SELECT key, value FROM settings ORDER BY key").all() as Array<{
    key: string;
    value: string;
  }>;
}

test("runSqlTransaction rolls back a real SQLite write on failure", async () => {
  const db = openAppDatabase();
  const execute = createSqliteExecute(db);
  await execute("INSERT INTO settings (key, value) VALUES (?, ?)", ["app", "keep-me"]);

  await assert.rejects(
    () =>
      runSqlTransaction(execute, async () => {
        await execute("DELETE FROM settings");
        await execute("INSERT INTO settings (key, value) VALUES (?, ?)", ["app", "replaced"]);
        throw new Error("forced failure after writes");
      }),
    /forced failure after writes/,
  );

  const settings = readSettings(db);
  assert.equal(settings.length, 1);
  assert.equal(settings[0]?.key, "app");
  assert.equal(settings[0]?.value, "keep-me");
});

test("persistSaveNote rolls back attachment replacement on a real SQLite connection", async () => {
  const db = openAppDatabase();
  const rawExecute = createSqliteExecute(db);
  const seed = createSqlNotePersistClient(rawExecute);
  await persistSaveNote(seed, makeNote());

  let attachmentInserts = 0;
  const failingExecute: SqlExecute = async (sql, params) => {
    if (/INSERT INTO attachments/i.test(sql)) {
      attachmentInserts += 1;
      if (attachmentInserts >= 2) {
        throw new Error("forced attachment insert failure");
      }
    }
    await rawExecute(sql, params);
  };

  await assert.rejects(
    () =>
      persistSaveNote(
        createSqlNotePersistClient(failingExecute),
        makeNote({
          content: "updated",
          attachments: [
            makeAttachment({ id: "att-2", value: "data:image/png;base64,def" }),
            makeAttachment({ id: "att-3", source: "url", value: "https://example.com/a.png" }),
          ],
        }),
      ),
    /forced attachment insert failure/,
  );

  const notes = readNotes(db);
  const attachments = readAttachments(db);
  assert.equal(notes.length, 1);
  assert.equal(notes[0]?.id, "note-1");
  assert.equal(notes[0]?.content, "original");
  assert.equal(attachments.length, 1);
  assert.equal(attachments[0]?.id, "att-1");
  assert.equal(attachments[0]?.value, "data:image/png;base64,abc");
});

test("persistReplaceAppData rolls back a mid-import failure on a real SQLite connection", async () => {
  const db = openAppDatabase();
  const rawExecute = createSqliteExecute(db);
  const seed = createSqlNotePersistClient(rawExecute);
  await persistSaveNote(seed, makeNote());
  await seed.upsertSettings("app", JSON.stringify({ ...createDefaultSettings(), language: "zh" }));

  let noteInserts = 0;
  const failingExecute: SqlExecute = async (sql, params) => {
    if (/INSERT INTO notes/i.test(sql)) {
      noteInserts += 1;
      if (noteInserts >= 2) {
        throw new Error("forced import note insert failure");
      }
    }
    await rawExecute(sql, params);
  };

  await assert.rejects(
    () =>
      persistReplaceAppData(createSqlNotePersistClient(failingExecute), {
        notes: [
          makeNote({ id: "imported-1", content: "one", attachments: [] }),
          makeNote({ id: "imported-2", content: "two", attachments: [] }),
        ],
        settings: { ...createDefaultSettings(), language: "en" },
      }),
    /forced import note insert failure/,
  );

  const notes = readNotes(db);
  const attachments = readAttachments(db);
  assert.equal(notes.length, 1);
  assert.equal(notes[0]?.id, "note-1");
  assert.equal(notes[0]?.content, "original");
  assert.equal(attachments.length, 1);
  assert.equal(attachments[0]?.id, "att-1");
  assert.match(readSettings(db).find((row) => row.key === "app")?.value ?? "", /"language":"zh"/);
});
