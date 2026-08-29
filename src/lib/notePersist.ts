import type { AppData, AppSettings, Note, NoteAttachment } from "../types.ts";
import { normalizeSettings } from "./settingsState.ts";

export const APP_SETTINGS_KEY = "app";

export type NotePersistOp =
  | "upsertNote"
  | "deleteAttachmentsForNote"
  | "insertAttachment"
  | "deleteNote"
  | "deleteAllNotes"
  | "deleteAllSettings"
  | "upsertSettings"
  | "updateNoteOrder";

export interface NotePersistClient {
  transaction<T>(work: () => Promise<T>): Promise<T>;
  upsertNote(note: Note): Promise<void>;
  deleteAttachmentsForNote(noteId: string): Promise<void>;
  insertAttachment(noteId: string, attachment: NoteAttachment): Promise<void>;
  deleteNote(noteId: string): Promise<void>;
  deleteAllNotes(): Promise<void>;
  deleteAllSettings(): Promise<void>;
  upsertSettings(key: string, value: string): Promise<void>;
  updateNoteOrder(note: Pick<Note, "id" | "pinned" | "sortOrder">): Promise<void>;
}

export async function persistSaveNote(client: NotePersistClient, note: Note): Promise<void> {
  await client.transaction(async () => {
    await client.upsertNote(note);
    await client.deleteAttachmentsForNote(note.id);
    for (const attachment of note.attachments) {
      await client.insertAttachment(note.id, attachment);
    }
  });
}

export async function persistSaveNotesOrder(
  client: NotePersistClient,
  notes: Note[],
): Promise<void> {
  await client.transaction(async () => {
    for (const note of notes) {
      await client.updateNoteOrder(note);
    }
  });
}

export async function persistDeleteNote(client: NotePersistClient, id: string): Promise<void> {
  await client.transaction(async () => {
    await client.deleteAttachmentsForNote(id);
    await client.deleteNote(id);
  });
}

export async function persistDeleteAllNotes(client: NotePersistClient): Promise<void> {
  await client.transaction(async () => {
    await client.deleteAllNotes();
  });
}

export async function persistReplaceAppData(
  client: NotePersistClient,
  data: AppData,
): Promise<void> {
  await client.transaction(async () => {
    await client.deleteAllNotes();
    await client.deleteAllSettings();
    await client.upsertSettings(APP_SETTINGS_KEY, JSON.stringify(normalizeSettings(data.settings)));
    for (const note of data.notes) {
      await client.upsertNote(note);
      for (const attachment of note.attachments) {
        await client.insertAttachment(note.id, attachment);
      }
    }
  });
}

export interface MemoryNotePersistClient extends NotePersistClient {
  failOn(op: NotePersistOp, remaining?: number): void;
  getNotes(): Note[];
  getSettingsValue(key: string): string | undefined;
}

function cloneNote(note: Note): Note {
  return {
    ...note,
    attachments: note.attachments.map((attachment) => ({ ...attachment })),
  };
}

function cloneNotes(notes: Map<string, Note>): Map<string, Note> {
  return new Map([...notes.entries()].map(([id, note]) => [id, cloneNote(note)]));
}

export function createMemoryNotePersistClient(
  initial: { notes?: Note[]; settings?: AppSettings } = {},
): MemoryNotePersistClient {
  let notes = new Map((initial.notes ?? []).map((note) => [note.id, cloneNote(note)]));
  let settings = new Map<string, string>();
  if (initial.settings) {
    settings.set(APP_SETTINGS_KEY, JSON.stringify(normalizeSettings(initial.settings)));
  }

  let depth = 0;
  let txNotes: Map<string, Note> | null = null;
  let txSettings: Map<string, string> | null = null;
  const failures = new Map<NotePersistOp, number>();

  function currentNotes() {
    return txNotes ?? notes;
  }

  function currentSettings() {
    return txSettings ?? settings;
  }

  async function guard(op: NotePersistOp) {
    const remaining = failures.get(op);
    if (remaining == null) {
      return;
    }
    if (remaining <= 1) {
      failures.delete(op);
      throw new Error(`forced ${op} failure`);
    }
    failures.set(op, remaining - 1);
  }

  const client: MemoryNotePersistClient = {
    failOn(op, remaining = 1) {
      failures.set(op, remaining);
    },
    getNotes() {
      return [...notes.values()].map(cloneNote);
    },
    getSettingsValue(key) {
      return settings.get(key);
    },
    async transaction(work) {
      if (depth > 0) {
        return work();
      }
      txNotes = cloneNotes(notes);
      txSettings = new Map(settings);
      depth += 1;
      try {
        const result = await work();
        notes = txNotes ?? notes;
        settings = txSettings ?? settings;
        return result;
      } catch (error) {
        throw error;
      } finally {
        txNotes = null;
        txSettings = null;
        depth -= 1;
      }
    },
    async upsertNote(note) {
      await guard("upsertNote");
      currentNotes().set(note.id, cloneNote(note));
    },
    async deleteAttachmentsForNote(noteId) {
      await guard("deleteAttachmentsForNote");
      const note = currentNotes().get(noteId);
      if (note) {
        currentNotes().set(noteId, { ...note, attachments: [] });
      }
    },
    async insertAttachment(noteId, attachment) {
      await guard("insertAttachment");
      const note = currentNotes().get(noteId);
      if (!note) {
        throw new Error(`note ${noteId} is missing`);
      }
      currentNotes().set(noteId, {
        ...note,
        attachments: [...note.attachments, { ...attachment }],
      });
    },
    async deleteNote(noteId) {
      await guard("deleteNote");
      currentNotes().delete(noteId);
    },
    async deleteAllNotes() {
      await guard("deleteAllNotes");
      currentNotes().clear();
    },
    async deleteAllSettings() {
      await guard("deleteAllSettings");
      currentSettings().clear();
    },
    async upsertSettings(key, value) {
      await guard("upsertSettings");
      currentSettings().set(key, value);
    },
    async updateNoteOrder(note) {
      await guard("updateNoteOrder");
      const current = currentNotes().get(note.id);
      if (!current) {
        return;
      }
      currentNotes().set(note.id, {
        ...current,
        pinned: note.pinned,
        sortOrder: note.sortOrder,
      });
    },
  };

  return client;
}
