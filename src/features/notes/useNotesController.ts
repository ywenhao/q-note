import { useCallback, useEffect, useRef, useState, type MutableRefObject } from "react";
import type { Translation } from "../../i18n";
import { writeClipboard } from "../../lib/clipboard";
import { createId, isTauriRuntime } from "../../lib/env";
import { getTopSortOrder, normalizeManualOrder, sortNotes } from "../../lib/noteOrdering";
import { deleteAllNotes, deleteNote, saveNote, saveNotesOrder } from "../../lib/storage";
import { openEditorWindow } from "../../lib/windowControls";
import type { AppSettings, Note } from "../../types";
import type { NoteDraft } from "../../components/NoteEditor";
import type { ShowToast } from "../../hooks/useToast";

interface UseNotesControllerOptions {
  commitNotes: (nextNotes: Note[]) => void;
  editorNote: Note | null | undefined;
  notesRef: MutableRefObject<Note[]>;
  restoreDock: (options?: { keepFull?: boolean; preserveRevealAnchor?: boolean }) => Promise<void>;
  setEditorNote: (note: Note | null | undefined) => void;
  settingsRef: MutableRefObject<AppSettings>;
  showToast: ShowToast;
  t: Translation;
}

export function useNotesState() {
  const [notes, setNotes] = useState<Note[]>([]);
  const notesRef = useRef<Note[]>([]);

  const commitNotes = useCallback((nextNotes: Note[]) => {
    const sorted = sortNotes(nextNotes);
    notesRef.current = sorted;
    setNotes(sorted);
  }, []);

  useEffect(() => {
    notesRef.current = notes;
  }, [notes]);

  return {
    commitNotes,
    notes,
    notesRef,
    setNotes,
  };
}

export function useNotesController({
  commitNotes,
  editorNote,
  notesRef,
  restoreDock,
  setEditorNote,
  settingsRef,
  showToast,
  t,
}: UseNotesControllerOptions) {
  const openEditor = useCallback(
    async (note: Note | null) => {
      if (settingsRef.current.docked) {
        await restoreDock({ keepFull: true });
      }

      if (isTauriRuntime()) {
        await openEditorWindow(
          note?.id ?? null,
          settingsRef.current.alwaysOnTop,
          note ? t.editorEditTitle : t.editorNewTitle,
        );
        return;
      }

      setEditorNote(note);
    },
    [restoreDock, setEditorNote, settingsRef, t],
  );

  const closeEditor = useCallback(async () => {
    setEditorNote(undefined);
  }, [setEditorNote]);

  const handleSaveDraft = useCallback(
    async (draft: NoteDraft) => {
      const now = Date.now();
      const nextNote: Note = editorNote
        ? {
            ...editorNote,
            attachments: draft.attachments,
            color: draft.color,
            content: draft.content,
            pinned: draft.pinned,
            sortOrder:
              editorNote.pinned === draft.pinned
                ? editorNote.sortOrder
                : getTopSortOrder(
                    notesRef.current.filter((note) => note.id !== editorNote.id),
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
            sortOrder: getTopSortOrder(notesRef.current, draft.pinned),
            textHeight: null,
            createdAt: now,
            updatedAt: now,
          };

      commitNotes([nextNote, ...notesRef.current.filter((note) => note.id !== nextNote.id)]);
      await saveNote(nextNote);
      showToast(t.saved);
      await closeEditor();
    },
    [closeEditor, commitNotes, editorNote, notesRef, showToast, t],
  );

  const handleCopy = useCallback(
    async (note: Note) => {
      const copyValue = note.content.trim()
        ? note.content
        : note.attachments.map((attachment) => attachment.value).join("\n");

      if (!copyValue) {
        return;
      }

      await writeClipboard(copyValue);
      showToast(t.copied);
    },
    [showToast, t],
  );

  const patchNote = useCallback(
    async (id: string, patch: Partial<Note>) => {
      const target = notesRef.current.find((note) => note.id === id);
      if (!target) {
        return;
      }

      const targetPinned =
        typeof patch.pinned === "boolean" && patch.pinned !== target.pinned
          ? patch.pinned
          : target.pinned;
      const nextNote = {
        ...target,
        ...patch,
        sortOrder:
          typeof patch.pinned === "boolean" && patch.pinned !== target.pinned
            ? getTopSortOrder(
                notesRef.current.filter((note) => note.id !== id),
                targetPinned,
              )
            : (patch.sortOrder ?? target.sortOrder),
      };
      commitNotes([nextNote, ...notesRef.current.filter((note) => note.id !== id)]);
      await saveNote(nextNote);
    },
    [commitNotes, notesRef],
  );

  const reorderNotes = useCallback(
    async (draggedId: string, targetId: string, placement: "before" | "after") => {
      if (draggedId === targetId) {
        return;
      }

      const draggedNote = notesRef.current.find((note) => note.id === draggedId);
      const targetNote = notesRef.current.find((note) => note.id === targetId);
      if (!draggedNote || !targetNote) {
        return;
      }

      const nextNotes = notesRef.current.filter((note) => note.id !== draggedId);
      const targetIndex = nextNotes.findIndex((note) => note.id === targetId);
      if (targetIndex < 0) {
        return;
      }

      nextNotes.splice(targetIndex + (placement === "after" ? 1 : 0), 0, {
        ...draggedNote,
        pinned: targetNote.pinned,
      });

      const orderedNotes = normalizeManualOrder(nextNotes);
      commitNotes(orderedNotes);
      await saveNotesOrder(orderedNotes);
    },
    [commitNotes, notesRef],
  );

  const handleDelete = useCallback(
    async (id: string) => {
      commitNotes(notesRef.current.filter((note) => note.id !== id));
      await deleteNote(id);
    },
    [commitNotes, notesRef],
  );

  const handleDeleteAll = useCallback(async () => {
    commitNotes([]);
    await deleteAllNotes();
  }, [commitNotes]);

  return {
    closeEditor,
    handleCopy,
    handleDelete,
    handleDeleteAll,
    handleSaveDraft,
    openEditor,
    patchNote,
    reorderNotes,
  };
}
