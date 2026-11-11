import { locale, platform, Storage } from "nsh";
import { Notes } from "q-note/native";
import type { AppData, AppSettings, Note } from "../types";
import { normalizeImportPayload } from "./appData";
import { createDefaultSettings, normalizeSettings } from "./settingsState";
import {
  parsePendingUpdateDraft,
  type PendingUpdateDraft,
} from "./updateDraft";

const key = "q-note:web-data";
const draftKey = "q-note:pending-update-editor-draft";
const native = () => platform.os !== "web";
let queue: Promise<unknown> = Promise.resolve();

function serialized<T>(work: () => Promise<T>): Promise<T> {
  const next = queue.then(work);
  queue = next.catch(() => {});
  return next;
}

export async function loadAppData(): Promise<AppData> {
  const raw = native() ? Notes.load() : await Storage.getItem(key);
  const value = raw ? JSON.parse(raw) : {};
  const data = normalizeImportPayload(value);
  if (!value.settings)
    data.settings = {
      ...createDefaultSettings(),
      language: locale().toLowerCase().startsWith("zh") ? "zh" : "en",
    };
  return data;
}

async function mutate(work: (data: AppData) => AppData) {
  await serialized(async () => {
    const next = work(await loadAppData());
    await Storage.setItem(key, JSON.stringify(next));
  });
}

export async function saveNote(note: Note, clearDraft = false) {
  if (native()) {
    Notes.save(JSON.stringify(note), clearDraft);
    return;
  }
  if (clearDraft) await Storage.removeItem(draftKey);
  await mutate((data) => ({
    ...data,
    notes: [note, ...data.notes.filter((n) => n.id !== note.id)],
  }));
}

export async function deleteNote(id: string) {
  if (native()) {
    Notes.delete(id);
    return;
  }
  await mutate((data) => ({
    ...data,
    notes: data.notes.filter((n) => n.id !== id),
  }));
}

export async function deleteAllNotes() {
  if (native()) {
    Notes.clear();
    return;
  }
  await mutate((data) => ({ ...data, notes: [] }));
}

export async function saveSettings(settings: AppSettings) {
  const next = normalizeSettings(settings);
  if (native()) Notes.settings(JSON.stringify(next));
  else await mutate((data) => ({ ...data, settings: next }));
  return next;
}

export async function saveNotesOrder(notes: Note[]) {
  if (native()) {
    Notes.reorder(JSON.stringify(notes));
    return;
  }
  await mutate((data) => ({
    ...data,
    notes: data.notes.map((n) => {
      const order = notes.find((item) => item.id === n.id);
      return order
        ? { ...n, pinned: order.pinned, sortOrder: order.sortOrder }
        : n;
    }),
  }));
}

export async function replaceAppData(data: AppData) {
  if (native()) Notes.replace(JSON.stringify(data));
  else await serialized(() => Storage.setItem(key, JSON.stringify(data)));
}

export async function loadPendingUpdateDraft() {
  const value = native() ? Notes.readDraft() : await Storage.getItem(draftKey);
  return value ? parsePendingUpdateDraft(value) : null;
}

export async function savePendingUpdateDraft(draft: PendingUpdateDraft | null) {
  const value = draft ? JSON.stringify(draft) : null;
  if (native()) Notes.draft(value);
  else if (value) await Storage.setItem(draftKey, value);
  else await Storage.removeItem(draftKey);
}

export async function clearPendingUpdateDraft() {
  await savePendingUpdateDraft(null);
}
export async function flushDatabase() {
  await queue;
  if (native()) Notes.flush();
}
