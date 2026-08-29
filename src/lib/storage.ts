import { invoke } from "@tauri-apps/api/core";
import Database from "@tauri-apps/plugin-sql";
import { desc, eq } from "drizzle-orm";
import { drizzle, type SqliteRemoteDatabase } from "drizzle-orm/sqlite-proxy";
import { type AppData, type AppSettings, type Note, type NoteAttachment } from "../types";
import { inferAttachmentKind, normalizeImportPayload } from "./appData";
import { isTauriRuntime } from "./env";
import {
  APP_SETTINGS_KEY,
  persistDeleteAllNotes,
  persistDeleteNote,
  persistReplaceAppData,
  persistSaveNote,
  persistSaveNotesOrder,
  type NotePersistClient,
} from "./notePersist";
import { attachmentsTable, notesTable, schema, settingsTable } from "./schema";
import { createSqlNotePersistClient } from "./sqlNotePersist";
import { createDefaultSettings, normalizeSettings, parseStoredSettings } from "./settingsState";
import { mapSqliteProxyResult } from "./sqliteProxy";
import {
  parsePendingUpdateDraft,
  serializePendingUpdateDraft,
  type PendingUpdateDraft,
} from "./updateDraft";

export { createExportPayload, normalizeImportPayload } from "./appData";
export { createDefaultSettings, normalizeSettings, parseStoredSettings } from "./settingsState";
export { windowSizeMatches } from "./windowState";

const FALLBACK_DB_URL = "sqlite:q-note.db";
const SETTINGS_KEY = APP_SETTINGS_KEY;
const PENDING_UPDATE_DRAFT_KEY = "pending-update-editor-draft";
const WEB_STORAGE_KEY = "q-note:web-data";
const WEB_PENDING_UPDATE_DRAFT_KEY = "q-note:pending-update-editor-draft";

let dbUrlPromise: Promise<string> | null = null;
let dbPromise: Promise<Database> | null = null;
let drizzlePromise: Promise<SqliteRemoteDatabase<typeof schema>> | null = null;
let persistClient: NotePersistClient | null = null;

function getDbUrl() {
  if (!isTauriRuntime()) {
    return Promise.resolve(FALLBACK_DB_URL);
  }

  dbUrlPromise ??= invoke<string>("get_database_url");
  return dbUrlPromise;
}

function getDb() {
  dbPromise ??= getDbUrl().then((url) => Database.load(url));
  return dbPromise;
}

function getDrizzleDb() {
  drizzlePromise ??= getDb().then((db) =>
    drizzle(
      // Drizzle builds the SQL while Tauri owns the native SQLite connection.
      async (query, params, method) => {
        if (method === "all" || method === "get" || method === "values") {
          const rows = await db.select<Record<string, unknown>[]>(query, params);
          return mapSqliteProxyResult(query, rows, method);
        }

        await db.execute(query, params);
        return { rows: [] };
      },
      { schema },
    ),
  );

  return drizzlePromise;
}

function loadWebData(): AppData {
  const raw = localStorage.getItem(WEB_STORAGE_KEY);
  if (!raw) {
    return { notes: [], settings: createDefaultSettings() };
  }

  try {
    return normalizeImportPayload(JSON.parse(raw));
  } catch {
    return { notes: [], settings: createDefaultSettings() };
  }
}

function saveWebData(data: AppData) {
  localStorage.setItem(WEB_STORAGE_KEY, JSON.stringify(data));
}

export async function loadAppData(): Promise<AppData> {
  if (!isTauriRuntime()) {
    return loadWebData();
  }

  const db = await getDrizzleDb();
  const noteRows = await db
    .select()
    .from(notesTable)
    .orderBy(desc(notesTable.pinned), notesTable.sortOrder, desc(notesTable.updatedAt));
  const attachmentRows = await db
    .select()
    .from(attachmentsTable)
    .orderBy(attachmentsTable.createdAt);
  const settingsRows = await db
    .select({ value: settingsTable.value })
    .from(settingsTable)
    .where(eq(settingsTable.key, SETTINGS_KEY))
    .limit(1);

  const attachmentMap = new Map<string, NoteAttachment[]>();
  for (const row of attachmentRows) {
    const list = attachmentMap.get(row.noteId) ?? [];
    list.push({
      id: row.id,
      kind: row.kind ?? inferAttachmentKind(row.source, row.value),
      source: row.source,
      value: row.value,
      name: row.name ?? undefined,
      createdAt: row.createdAt,
    });
    attachmentMap.set(row.noteId, list);
  }

  const notes = noteRows.map<Note>((row) => ({
    id: row.id,
    content: row.content,
    color: row.color,
    pinned: row.pinned === 1,
    sortOrder: row.sortOrder,
    textHeight: row.textHeight,
    attachments: attachmentMap.get(row.id) ?? [],
    createdAt: row.createdAt,
    updatedAt: row.updatedAt,
  }));

  const settings = settingsRows[0]?.value
    ? parseStoredSettings(settingsRows[0].value)
    : createDefaultSettings();

  return { notes, settings };
}

export async function loadPersistedSettings(): Promise<AppSettings> {
  if (!isTauriRuntime()) {
    return loadWebData().settings;
  }

  const db = await getDrizzleDb();
  const settingsRows = await db
    .select({ value: settingsTable.value })
    .from(settingsTable)
    .where(eq(settingsTable.key, SETTINGS_KEY))
    .limit(1);

  return settingsRows[0]?.value
    ? parseStoredSettings(settingsRows[0].value)
    : createDefaultSettings();
}

export async function flushDatabase() {
  if (!isTauriRuntime()) {
    return;
  }

  try {
    const db = await getDb();
    await db.execute("PRAGMA wal_checkpoint(TRUNCATE)");
    await db.close();
  } finally {
    dbPromise = null;
    drizzlePromise = null;
    persistClient = null;
  }
}

export async function saveSettings(settings: AppSettings) {
  const nextSettings = normalizeSettings(settings);

  if (!isTauriRuntime()) {
    const data = loadWebData();
    saveWebData({ ...data, settings: nextSettings });
    return nextSettings;
  }

  const db = await getDrizzleDb();
  await db
    .insert(settingsTable)
    .values({ key: SETTINGS_KEY, value: JSON.stringify(nextSettings) })
    .onConflictDoUpdate({
      target: settingsTable.key,
      set: { value: JSON.stringify(nextSettings) },
    });
  return nextSettings;
}

export async function loadPendingUpdateDraft(): Promise<PendingUpdateDraft | null> {
  if (!isTauriRuntime()) {
    const value = localStorage.getItem(WEB_PENDING_UPDATE_DRAFT_KEY);
    return value ? parsePendingUpdateDraft(value) : null;
  }

  const db = await getDrizzleDb();
  const rows = await db
    .select({ value: settingsTable.value })
    .from(settingsTable)
    .where(eq(settingsTable.key, PENDING_UPDATE_DRAFT_KEY))
    .limit(1);

  return rows[0]?.value ? parsePendingUpdateDraft(rows[0].value) : null;
}

export async function savePendingUpdateDraft(value: PendingUpdateDraft) {
  const serialized = serializePendingUpdateDraft(value);
  if (!isTauriRuntime()) {
    localStorage.setItem(WEB_PENDING_UPDATE_DRAFT_KEY, serialized);
    return;
  }

  const db = await getDrizzleDb();
  await db
    .insert(settingsTable)
    .values({ key: PENDING_UPDATE_DRAFT_KEY, value: serialized })
    .onConflictDoUpdate({
      target: settingsTable.key,
      set: { value: serialized },
    });
}

export async function clearPendingUpdateDraft() {
  if (!isTauriRuntime()) {
    localStorage.removeItem(WEB_PENDING_UPDATE_DRAFT_KEY);
    return;
  }

  const db = await getDrizzleDb();
  await db.delete(settingsTable).where(eq(settingsTable.key, PENDING_UPDATE_DRAFT_KEY));
}

function getPersistClient(): NotePersistClient {
  persistClient ??= createSqlNotePersistClient(async (sql, params) => {
    const db = await getDb();
    await db.execute(sql, params ?? []);
  });
  return persistClient;
}

export async function saveNote(note: Note) {
  if (!isTauriRuntime()) {
    const data = loadWebData();
    const nextNotes = data.notes.filter((item) => item.id !== note.id);
    saveWebData({ ...data, notes: [note, ...nextNotes] });
    return;
  }

  await persistSaveNote(getPersistClient(), note);
}

export async function saveNotesOrder(notes: Note[]) {
  if (!isTauriRuntime()) {
    const data = loadWebData();
    const noteMap = new Map(notes.map((note) => [note.id, note]));
    saveWebData({
      ...data,
      notes: data.notes.map((note) => noteMap.get(note.id) ?? note),
    });
    return;
  }

  await persistSaveNotesOrder(getPersistClient(), notes);
}

export async function deleteNote(id: string) {
  if (!isTauriRuntime()) {
    const data = loadWebData();
    saveWebData({ ...data, notes: data.notes.filter((note) => note.id !== id) });
    return;
  }

  await persistDeleteNote(getPersistClient(), id);
}

export async function deleteAllNotes() {
  if (!isTauriRuntime()) {
    const data = loadWebData();
    saveWebData({ ...data, notes: [] });
    return;
  }

  await persistDeleteAllNotes(getPersistClient());
}

export async function replaceAppData(data: AppData) {
  if (!isTauriRuntime()) {
    saveWebData(data);
    return;
  }

  await persistReplaceAppData(getPersistClient(), data);
}
