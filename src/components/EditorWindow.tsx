import { emit, emitTo, listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useCallback, useEffect, useRef, useState, type PointerEvent } from "react";
import { translations } from "../i18n";
import { createId, isTauriRuntime } from "../lib/env";
import {
  clearPendingUpdateDraft,
  createDefaultSettings,
  loadAppData,
  loadPendingUpdateDraft,
  saveNote,
  savePendingUpdateDraft,
} from "../lib/storage";
import { createNoteDraft, createPendingUpdateDraft } from "../lib/updateDraft";
import {
  PREPARE_UPDATE_ACK_EVENT,
  PREPARE_UPDATE_EVENT,
  type PrepareUpdateAcknowledgement,
  type PrepareUpdateRequest,
} from "../lib/updatePreparation";
import { MAIN_WINDOW_LABEL, readPendingEditorNoteId } from "../lib/windowControls";
import type { AppSettings, Note, NoteDraft } from "../types";
import { NoteEditor } from "./NoteEditor";
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
  const [initialDraft, setInitialDraft] = useState<NoteDraft | null>(null);
  const [note, setNote] = useState<Note | null>(null);
  const [notes, setNotes] = useState<Note[]>([]);
  const [ready, setReady] = useState(false);
  const [settings, setSettings] = useState<AppSettings>(() => createDefaultSettings());
  const activeNoteIdRef = useRef(activeNoteId);
  const draftRef = useRef<NoteDraft>(createNoteDraft(null));
  const noteRef = useRef<Note | null>(null);
  const recoveryActiveRef = useRef(false);
  const recoverySaveTimerRef = useRef<number | null>(null);
  const t = translations[settings.language];
  const editorTitle = activeNoteId ? t.editorEditTitle : t.editorNewTitle;

  const loadEditorData = useCallback(
    async (noteId: string | null, restoredDraft: NoteDraft | null = null) => {
      activeNoteIdRef.current = noteId;
      setActiveNoteId(noteId);
      setEditorSession((current) => current + 1);
      setInitialDraft(restoredDraft);
      setNote(null);
      noteRef.current = null;
      draftRef.current = restoredDraft ?? createNoteDraft(null);
      try {
        const data = await loadAppData();
        const nextNote = noteId ? (data.notes.find((item) => item.id === noteId) ?? null) : null;
        setSettings(data.settings);
        setNotes(data.notes);
        setNote(nextNote);
        noteRef.current = nextNote;
        draftRef.current = restoredDraft ?? createNoteDraft(nextNote);
      } catch {
        setSettings(createDefaultSettings());
        setNotes([]);
        setNote(null);
        noteRef.current = null;
      } finally {
        setReady(true);
      }
    },
    [],
  );

  useEffect(() => {
    let disposed = false;
    const initialNoteId = getInitialNoteId();

    if (!isTauriRuntime()) {
      void loadEditorData(initialNoteId);
      return () => {
        disposed = true;
      };
    }

    let unlistenOpen: (() => void) | null = null;
    let unlistenPrepare: (() => void) | null = null;
    let unlistenSettings: (() => void) | null = null;

    void (async () => {
      const pendingDraft = await loadPendingUpdateDraft().catch(() => null);
      recoveryActiveRef.current = Boolean(pendingDraft);
      await loadEditorData(pendingDraft?.noteId ?? initialNoteId, pendingDraft?.draft ?? null);
      if (pendingDraft) {
        const currentWindow = getCurrentWindow();
        await currentWindow.show();
        await currentWindow.setFocus();
      }

      [unlistenOpen, unlistenPrepare, unlistenSettings] = await Promise.all([
        listen<EditorOpenPayload>("q-note-editor-open", (event) => {
          recoveryActiveRef.current = false;
          void clearPendingUpdateDraft();
          void loadEditorData(event.payload.noteId);
        }),
        listen<PrepareUpdateRequest>(PREPARE_UPDATE_EVENT, (event) => {
          void (async () => {
            try {
              const currentWindow = getCurrentWindow();
              const pending = createPendingUpdateDraft({
                draft: draftRef.current,
                note: noteRef.current,
                now: Date.now(),
                visible: await currentWindow.isVisible(),
              });
              if (pending) {
                await savePendingUpdateDraft(pending);
                recoveryActiveRef.current = true;
              }

              await emitTo(MAIN_WINDOW_LABEL, PREPARE_UPDATE_ACK_EVENT, {
                ok: true,
                requestId: event.payload.requestId,
              } satisfies PrepareUpdateAcknowledgement);
            } catch (error) {
              await emitTo(MAIN_WINDOW_LABEL, PREPARE_UPDATE_ACK_EVENT, {
                error: String(error),
                ok: false,
                requestId: event.payload.requestId,
              } satisfies PrepareUpdateAcknowledgement);
            }
          })();
        }),
        listen<AppSettings>("q-note-settings-updated", (event) => {
          setSettings(event.payload);
        }),
      ]);

      if (disposed) {
        unlistenOpen?.();
        unlistenPrepare?.();
        unlistenSettings?.();
      }
    })();

    return () => {
      disposed = true;
      unlistenOpen?.();
      unlistenPrepare?.();
      unlistenSettings?.();
      if (recoverySaveTimerRef.current) {
        window.clearTimeout(recoverySaveTimerRef.current);
        recoverySaveTimerRef.current = null;
      }
    };
  }, [loadEditorData]);

  useEffect(() => {
    document.title = editorTitle;
    if (isTauriRuntime()) {
      void getCurrentWindow().setTitle(editorTitle);
    }
  }, [editorTitle]);

  async function closeEditorWindow() {
    recoveryActiveRef.current = false;
    if (recoverySaveTimerRef.current) {
      window.clearTimeout(recoverySaveTimerRef.current);
      recoverySaveTimerRef.current = null;
    }
    await clearPendingUpdateDraft();
    activeNoteIdRef.current = null;
    setActiveNoteId(null);
    setEditorSession((current) => current + 1);
    setInitialDraft(null);
    setNote(null);
    noteRef.current = null;
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

  const handleDraftChange = useCallback((draft: NoteDraft) => {
    draftRef.current = draft;
    if (!recoveryActiveRef.current) {
      return;
    }

    if (recoverySaveTimerRef.current) {
      window.clearTimeout(recoverySaveTimerRef.current);
    }
    recoverySaveTimerRef.current = window.setTimeout(() => {
      void savePendingUpdateDraft({
        draft: draftRef.current,
        noteId: activeNoteIdRef.current,
        savedAt: Date.now(),
      });
      recoverySaveTimerRef.current = null;
    }, 250);
  }, []);

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
      initialDraft={initialDraft}
      key={editorSession}
      mode="window"
      note={note}
      onCancel={() => void closeEditorWindow()}
      onDragStart={dragEditorWindow}
      onDraftChange={handleDraftChange}
      onSave={(draft) => void handleSaveDraft(draft)}
      t={t}
    />
  );
}
