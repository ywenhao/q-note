import { emit, listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useCallback, useEffect, useState, type PointerEvent } from "react";
import { translations } from "../i18n";
import { createId, isTauriRuntime } from "../lib/env";
import { createDefaultSettings, loadAppData, saveNote } from "../lib/storage";
import { readPendingEditorNoteId } from "../lib/windowControls";
import type { AppSettings, Note } from "../types";
import { NoteEditor, type NoteDraft } from "./NoteEditor";
import { QMark } from "./QMark";

interface EditorOpenPayload {
  noteId: string | null;
}

function getInitialNoteId() {
  return new URLSearchParams(window.location.search).get("noteId") ?? readPendingEditorNoteId();
}

function getTopSortOrder(notes: Note[], pinned: boolean) {
  const group = notes.filter((note) => note.pinned === pinned);
  if (group.length === 0) {
    return 0;
  }

  return Math.min(...group.map((note) => note.sortOrder)) - 1;
}

export function EditorWindow() {
  const [activeNoteId, setActiveNoteId] = useState<string | null>(() => getInitialNoteId());
  const [editorSession, setEditorSession] = useState(0);
  const [note, setNote] = useState<Note | null>(null);
  const [notes, setNotes] = useState<Note[]>([]);
  const [ready, setReady] = useState(false);
  const [settings, setSettings] = useState<AppSettings>(() => createDefaultSettings());
  const t = translations[settings.language];
  const editorTitle = activeNoteId ? t.editorEditTitle : t.editorNewTitle;

  const loadEditorData = useCallback(async (noteId: string | null) => {
    setActiveNoteId(noteId);
    setEditorSession((current) => current + 1);
    setNote(null);
    try {
      const data = await loadAppData();
      setSettings(data.settings);
      setNotes(data.notes);
      setNote(noteId ? (data.notes.find((item) => item.id === noteId) ?? null) : null);
    } catch {
      setSettings(createDefaultSettings());
      setNotes([]);
      setNote(null);
    } finally {
      setReady(true);
    }
  }, []);

  useEffect(() => {
    let disposed = false;
    const initialNoteId = getInitialNoteId();

    void loadEditorData(initialNoteId);

    if (!isTauriRuntime()) {
      return () => {
        disposed = true;
      };
    }

    let unlistenOpen: (() => void) | null = null;
    let unlistenSettings: (() => void) | null = null;

    void (async () => {
      [unlistenOpen, unlistenSettings] = await Promise.all([
        listen<EditorOpenPayload>("q-note-editor-open", (event) => {
          void loadEditorData(event.payload.noteId);
        }),
        listen<AppSettings>("q-note-settings-updated", (event) => {
          setSettings(event.payload);
        }),
      ]);

      if (disposed) {
        unlistenOpen?.();
        unlistenSettings?.();
      }
    })();

    return () => {
      disposed = true;
      unlistenOpen?.();
      unlistenSettings?.();
    };
  }, [loadEditorData]);

  useEffect(() => {
    document.title = editorTitle;
    if (isTauriRuntime()) {
      void getCurrentWindow().setTitle(editorTitle);
    }
  }, [editorTitle]);

  async function closeEditorWindow() {
    setActiveNoteId(null);
    setEditorSession((current) => current + 1);
    setNote(null);
    if (isTauriRuntime()) {
      await getCurrentWindow().hide();
    }
  }

  async function handleSaveDraft(draft: NoteDraft) {
    const now = Date.now();
    const nextNote: Note = note
      ? {
          ...note,
          attachments: draft.attachments,
          color: draft.color,
          content: draft.content,
          pinned: draft.pinned,
          sortOrder:
            note.pinned === draft.pinned
              ? note.sortOrder
              : getTopSortOrder(
                  notes.filter((item) => item.id !== note.id),
                  draft.pinned,
                ),
          updatedAt: now,
        }
      : {
          id: createId("note"),
          attachments: draft.attachments,
          color: draft.color,
          content: draft.content,
          pinned: draft.pinned,
          sortOrder: getTopSortOrder(notes, draft.pinned),
          textHeight: null,
          createdAt: now,
          updatedAt: now,
        };

    await saveNote(nextNote);
    await emit("q-note-note-saved", nextNote);
    await closeEditorWindow();
  }

  function dragEditorWindow(event: PointerEvent<HTMLElement>) {
    if (event.button !== 0 || !isTauriRuntime()) {
      return;
    }

    void getCurrentWindow().startDragging();
  }

  if (!ready) {
    return (
      <main className="editor-window-shell is-loading">
        <QMark className="loading-mark" />
      </main>
    );
  }

  return (
    <NoteEditor
      key={editorSession}
      mode="window"
      note={note}
      onCancel={() => void closeEditorWindow()}
      onDragStart={dragEditorWindow}
      onSave={(draft) => void handleSaveDraft(draft)}
      t={t}
    />
  );
}
