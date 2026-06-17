import { invoke } from "@tauri-apps/api/core";
import Database from "@tauri-apps/plugin-sql";
import { desc, eq } from "drizzle-orm";
import { drizzle, type SqliteRemoteDatabase } from "drizzle-orm/sqlite-proxy";
import {
  DEFAULT_NOTE_COLOR,
  DEFAULT_WINDOW_HEIGHT,
  DEFAULT_WINDOW_WIDTH,
  type AppData,
  type AppSettings,
  type AttachmentKind,
  type AttachmentSource,
  type ExportPayload,
  type Language,
  type Note,
  type NoteAttachment,
  type WindowState,
} from "../types";
import { isTauriRuntime } from "./env";
import { isLikelyImagePath } from "./images";
import { attachmentsTable, notesTable, schema, settingsTable } from "./schema";

const FALLBACK_DB_URL = "sqlite:q-note.db";
const SETTINGS_KEY = "app";
const WEB_STORAGE_KEY = "q-note:web-data";

let dbUrlPromise: Promise<string> | null = null;
let dbPromise: Promise<Database> | null = null;
let drizzlePromise: Promise<SqliteRemoteDatabase<typeof schema>> | null = null;

function readSystemLanguage() {
  if (typeof navigator === "undefined") {
    return "en";
  }

  return navigator.language || navigator.languages?.[0] || "en";
}

function detectDefaultLanguage(): Language {
  const language = readSystemLanguage().toLowerCase();

  if (language.startsWith("zh")) {
    return "zh";
  }

  return "en";
}

export function createDefaultSettings(): AppSettings {
  return {
    language: detectDefaultLanguage(),
    alwaysOnTop: false,
    autoStart: false,
    dockOnEdge: false,
    docked: false,
    dockEdge: null,
    keepFullMain: false,
    window: null,
  };
}

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
          const values = rows.map((row) => Object.values(row));
          return { rows: method === "get" ? values[0] : values };
        }

        await db.execute(query, params);
        return { rows: [] };
      },
      { schema },
    ),
  );

  return drizzlePromise;
}

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function toLanguage(value: unknown): Language {
  if (value === "zh" || value === "en") {
    return value;
  }

  return detectDefaultLanguage();
}

function normalizeWindowState(value: unknown): WindowState | null {
  if (!isObject(value)) {
    return null;
  }

  const width = Number(value.width);
  const height = Number(value.height);
  const x = Number(value.x);
  const y = Number(value.y);

  return {
    width: Math.max(DEFAULT_WINDOW_WIDTH, Number.isFinite(width) ? Math.round(width) : 0),
    height: Math.max(DEFAULT_WINDOW_HEIGHT, Number.isFinite(height) ? Math.round(height) : 0),
    x: Number.isFinite(x) ? Math.round(x) : 0,
    y: Number.isFinite(y) ? Math.round(y) : 0,
  };
}

function inferAttachmentKind(source: AttachmentSource, value: string): AttachmentKind {
  if (source === "data") {
    return /^data:image\//i.test(value) ? "image" : "file";
  }

  return isLikelyImagePath(value) ? "image" : "file";
}

export function normalizeSettings(value: unknown): AppSettings {
  const defaults = createDefaultSettings();

  if (!isObject(value)) {
    return defaults;
  }

  const windowState = normalizeWindowState(value.window);
  return {
    language: toLanguage(value.language),
    alwaysOnTop: Boolean(value.alwaysOnTop),
    autoStart: Boolean(value.autoStart),
    dockOnEdge: typeof value.dockOnEdge === "boolean" ? value.dockOnEdge : defaults.dockOnEdge,
    docked: Boolean(value.docked),
    dockEdge:
      value.dockEdge === "left" ||
      value.dockEdge === "right" ||
      value.dockEdge === "top" ||
      value.dockEdge === "bottom"
        ? value.dockEdge
        : null,
    keepFullMain: Boolean(value.keepFullMain),
    window: windowState,
  };
}

function normalizeAttachment(value: unknown): NoteAttachment | null {
  if (!isObject(value) || typeof value.value !== "string") {
    return null;
  }

  const source =
    value.source === "url" || value.source === "path" || value.source === "data"
      ? value.source
      : "url";
  const kind =
    value.kind === "file" || value.kind === "image"
      ? value.kind
      : inferAttachmentKind(source, value.value);

  return {
    id: typeof value.id === "string" ? value.id : crypto.randomUUID(),
    kind,
    source,
    value: value.value,
    name: typeof value.name === "string" ? value.name : undefined,
    createdAt: Number(value.createdAt) || Date.now(),
  };
}

function normalizeNote(value: unknown): Note | null {
  if (!isObject(value)) {
    return null;
  }

  const updatedAt = Number(value.updatedAt) || Date.now();
  const attachments = Array.isArray(value.attachments)
    ? value.attachments
        .map(normalizeAttachment)
        .filter((item): item is NoteAttachment => Boolean(item))
    : [];

  return {
    id: typeof value.id === "string" ? value.id : crypto.randomUUID(),
    content: typeof value.content === "string" ? value.content : "",
    color: typeof value.color === "string" ? value.color : DEFAULT_NOTE_COLOR,
    pinned: Boolean(value.pinned),
    sortOrder: Number.isFinite(Number(value.sortOrder)) ? Number(value.sortOrder) : -updatedAt,
    textHeight: typeof value.textHeight === "number" ? value.textHeight : null,
    attachments,
    createdAt: Number(value.createdAt) || Date.now(),
    updatedAt,
  };
}

export function normalizeImportPayload(value: unknown): AppData {
  if (!isObject(value)) {
    throw new Error("Invalid Q Note data");
  }

  const notes = Array.isArray(value.notes)
    ? value.notes.map(normalizeNote).filter((item): item is Note => Boolean(item))
    : [];

  return {
    notes,
    settings: normalizeSettings(value.settings),
  };
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
    ? normalizeSettings(JSON.parse(settingsRows[0].value))
    : createDefaultSettings();

  return { notes, settings };
}

export async function saveSettings(settings: AppSettings) {
  const nextSettings = normalizeSettings(settings);

  if (!isTauriRuntime()) {
    const data = loadWebData();
    saveWebData({ ...data, settings: nextSettings });
    return;
  }

  const db = await getDrizzleDb();
  await db
    .insert(settingsTable)
    .values({ key: SETTINGS_KEY, value: JSON.stringify(nextSettings) })
    .onConflictDoUpdate({
      target: settingsTable.key,
      set: { value: JSON.stringify(nextSettings) },
    });
}

export async function saveNote(note: Note) {
  if (!isTauriRuntime()) {
    const data = loadWebData();
    const nextNotes = data.notes.filter((item) => item.id !== note.id);
    saveWebData({ ...data, notes: [note, ...nextNotes] });
    return;
  }

  const db = await getDrizzleDb();
  await db
    .insert(notesTable)
    .values({
      id: note.id,
      content: note.content,
      color: note.color,
      pinned: note.pinned ? 1 : 0,
      sortOrder: note.sortOrder,
      textHeight: note.textHeight,
      createdAt: note.createdAt,
      updatedAt: note.updatedAt,
    })
    .onConflictDoUpdate({
      target: notesTable.id,
      set: {
        content: note.content,
        color: note.color,
        pinned: note.pinned ? 1 : 0,
        sortOrder: note.sortOrder,
        textHeight: note.textHeight,
        updatedAt: note.updatedAt,
      },
    });

  await db.delete(attachmentsTable).where(eq(attachmentsTable.noteId, note.id));
  for (const attachment of note.attachments) {
    await db.insert(attachmentsTable).values({
      id: attachment.id,
      noteId: note.id,
      kind: attachment.kind,
      source: attachment.source,
      value: attachment.value,
      name: attachment.name ?? null,
      createdAt: attachment.createdAt,
    });
  }
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

  const db = await getDrizzleDb();
  for (const note of notes) {
    await db
      .update(notesTable)
      .set({
        pinned: note.pinned ? 1 : 0,
        sortOrder: note.sortOrder,
      })
      .where(eq(notesTable.id, note.id));
  }
}

export async function deleteNote(id: string) {
  if (!isTauriRuntime()) {
    const data = loadWebData();
    saveWebData({ ...data, notes: data.notes.filter((note) => note.id !== id) });
    return;
  }

  const db = await getDrizzleDb();
  await db.delete(attachmentsTable).where(eq(attachmentsTable.noteId, id));
  await db.delete(notesTable).where(eq(notesTable.id, id));
}

export async function deleteAllNotes() {
  if (!isTauriRuntime()) {
    const data = loadWebData();
    saveWebData({ ...data, notes: [] });
    return;
  }

  const db = await getDrizzleDb();
  await db.delete(attachmentsTable);
  await db.delete(notesTable);
}

export async function replaceAppData(data: AppData) {
  if (!isTauriRuntime()) {
    saveWebData(data);
    return;
  }

  const db = await getDrizzleDb();
  await db.delete(attachmentsTable);
  await db.delete(notesTable);
  await db.delete(settingsTable);

  await saveSettings(data.settings);
  for (const note of data.notes) {
    await saveNote(note);
  }
}

export function createExportPayload(data: AppData): ExportPayload {
  return {
    version: 1,
    exportedAt: new Date().toISOString(),
    notes: data.notes,
    settings: data.settings,
  };
}
