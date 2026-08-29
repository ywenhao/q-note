import type { ComputedRef, Ref } from "vue";
import type { Translation } from "../../i18n";
import { writeClipboard } from "../../lib/clipboard";
import { createId, isTauriRuntime } from "../../lib/env";
import { getTopSortOrder, normalizeManualOrder, sortNotes } from "../../lib/noteOrdering";
import { deleteAllNotes, deleteNote, saveNote, saveNotesOrder } from "../../lib/storage";
import { openEditorWindow } from "../../lib/windowControls";
import type { AppSettings, Note, NoteDraft } from "../../types";
import type { ShowToast } from "../../hooks/useToast";
import { ref } from "vue";

interface UseNotesControllerOptions {
  commitNotes: (nextNotes: Note[]) => void;
  editorNote: Ref<Note | null | undefined>;
  notes: Ref<Note[]>;
  restoreDock: (options?: { keepFull?: boolean; preserveRevealAnchor?: boolean }) => Promise<void>;
  settings: Ref<AppSettings>;
  showToast: ShowToast;
  t: ComputedRef<Translation>;
}

export function useNotesState() {
  const notes = ref<Note[]>([]);
  const commitNotes = (nextNotes: Note[]) => {
    notes.value = sortNotes(nextNotes);
  };
  return { commitNotes, notes };
}

export function useNotesController(options: UseNotesControllerOptions) {
  const { commitNotes, editorNote, notes, restoreDock, settings, showToast, t } = options;

  async function openEditor(note: Note | null) {
    if (settings.value.docked) {
      await restoreDock({ keepFull: true });
    }
    if (isTauriRuntime()) {
      await openEditorWindow(
        note?.id ?? null,
        settings.value.alwaysOnTop,
        note ? t.value.editorEditTitle : t.value.editorNewTitle,
      );
      return;
    }
    editorNote.value = note;
  }

  async function closeEditor() {
    editorNote.value = undefined;
  }

  async function handleSaveDraft(draft: NoteDraft) {
    const now = Date.now();
    const currentNote = editorNote.value;
    const nextNote: Note = currentNote
      ? {
          ...currentNote,
          attachments: draft.attachments,
          color: draft.color,
          content: draft.content,
          pinned: draft.pinned,
          sortOrder:
            currentNote.pinned === draft.pinned
              ? currentNote.sortOrder
              : getTopSortOrder(
                  notes.value.filter((note) => note.id !== currentNote.id),
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
          sortOrder: getTopSortOrder(notes.value, draft.pinned),
          textHeight: null,
          createdAt: now,
          updatedAt: now,
        };

    try {
      await saveNote(nextNote);
      commitNotes([nextNote, ...notes.value.filter((note) => note.id !== nextNote.id)]);
      showToast(t.value.saved);
      await closeEditor();
    } catch {
      showToast(t.value.saveFailed, { kind: "error" });
    }
  }

  async function handleCopy(note: Note) {
    const copyValue = note.content.trim()
      ? note.content
      : note.attachments.map((attachment) => attachment.value).join("\n");
    if (copyValue) {
      await writeClipboard(copyValue);
      showToast(t.value.copied);
    }
  }

  async function patchNote(id: string, patch: Partial<Note>) {
    const target = notes.value.find((note) => note.id === id);
    if (!target) {
      return;
    }
    const targetPinned =
      typeof patch.pinned === "boolean" && patch.pinned !== target.pinned
        ? patch.pinned
        : target.pinned;
    const nextNote: Note = {
      ...target,
      ...patch,
      sortOrder:
        typeof patch.pinned === "boolean" && patch.pinned !== target.pinned
          ? getTopSortOrder(
              notes.value.filter((note) => note.id !== id),
              targetPinned,
            )
          : (patch.sortOrder ?? target.sortOrder),
    };
    try {
      await saveNote(nextNote);
      commitNotes([nextNote, ...notes.value.filter((note) => note.id !== id)]);
    } catch {
      showToast(t.value.saveFailed, { kind: "error" });
    }
  }

  async function reorderNotes(draggedId: string, targetId: string, placement: "before" | "after") {
    if (draggedId === targetId) {
      return;
    }
    const draggedNote = notes.value.find((note) => note.id === draggedId);
    const targetNote = notes.value.find((note) => note.id === targetId);
    if (!draggedNote || !targetNote) {
      return;
    }
    const nextNotes = notes.value.filter((note) => note.id !== draggedId);
    const targetIndex = nextNotes.findIndex((note) => note.id === targetId);
    if (targetIndex < 0) {
      return;
    }
    nextNotes.splice(targetIndex + (placement === "after" ? 1 : 0), 0, {
      ...draggedNote,
      pinned: targetNote.pinned,
    });
    const orderedNotes = normalizeManualOrder(nextNotes);
    try {
      await saveNotesOrder(orderedNotes);
      commitNotes(orderedNotes);
    } catch {
      showToast(t.value.saveFailed, { kind: "error" });
    }
  }

  async function handleDelete(id: string) {
    try {
      await deleteNote(id);
      commitNotes(notes.value.filter((note) => note.id !== id));
    } catch {
      showToast(t.value.deleteFailed, { kind: "error" });
    }
  }

  async function handleDeleteAll() {
    try {
      await deleteAllNotes();
      commitNotes([]);
    } catch {
      showToast(t.value.deleteFailed, { kind: "error" });
    }
  }

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
