<script setup lang="ts" vapor>
import { emit, emitTo, listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
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
import NoteEditor from "./NoteEditor.vue";
import QMark from "./QMark.vue";

interface EditorOpenPayload {
  noteId: string | null;
}

const activeNoteId = ref<string | null>(getInitialNoteId());
const editorSession = ref(0);
const initialDraft = ref<NoteDraft | null>(null);
const note = ref<Note | null>(null);
const notes = ref<Note[]>([]);
const ready = ref(false);
const settings = ref<AppSettings>(createDefaultSettings());
const draft = ref<NoteDraft>(createNoteDraft(null));
let recoveryActive = false;
let recoverySaveTimer: number | null = null;
let disposed = false;
let unlistenOpen: (() => void) | null = null;
let unlistenPrepare: (() => void) | null = null;
let unlistenSettings: (() => void) | null = null;

const t = computed(() => translations[settings.value.language]);
const editorTitle = computed(() =>
  activeNoteId.value ? t.value.editorEditTitle : t.value.editorNewTitle,
);

function getInitialNoteId() {
  return new URLSearchParams(window.location.search).get("noteId") ?? readPendingEditorNoteId();
}

function getTopSortOrder(currentNotes: Note[], pinned: boolean) {
  const group = currentNotes.filter((item) => item.pinned === pinned);
  return group.length === 0 ? 0 : Math.min(...group.map((item) => item.sortOrder)) - 1;
}

async function loadEditorData(noteId: string | null, restoredDraft: NoteDraft | null = null) {
  activeNoteId.value = noteId;
  editorSession.value += 1;
  initialDraft.value = restoredDraft;
  note.value = null;
  draft.value = restoredDraft ?? createNoteDraft(null);
  try {
    const data = await loadAppData();
    const nextNote = noteId ? (data.notes.find((item) => item.id === noteId) ?? null) : null;
    settings.value = data.settings;
    notes.value = data.notes;
    note.value = nextNote;
    draft.value = restoredDraft ?? createNoteDraft(nextNote);
  } catch {
    settings.value = createDefaultSettings();
    notes.value = [];
    note.value = null;
  } finally {
    ready.value = true;
  }
}

async function registerTauriListeners() {
  [unlistenOpen, unlistenPrepare, unlistenSettings] = await Promise.all([
    listen<EditorOpenPayload>("q-note-editor-open", (event) => {
      recoveryActive = false;
      void clearPendingUpdateDraft();
      void loadEditorData(event.payload.noteId);
    }),
    listen<PrepareUpdateRequest>(PREPARE_UPDATE_EVENT, (event) => {
      void (async () => {
        try {
          const pending = createPendingUpdateDraft({
            draft: draft.value,
            note: note.value,
            now: Date.now(),
            visible: await getCurrentWindow().isVisible(),
          });
          if (pending) {
            await savePendingUpdateDraft(pending);
            recoveryActive = true;
          } else {
            await clearPendingUpdateDraft();
            recoveryActive = false;
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
      settings.value = event.payload;
    }),
  ]);

  if (disposed) {
    unlistenOpen?.();
    unlistenPrepare?.();
    unlistenSettings?.();
  }
}

onMounted(async () => {
  const initialNoteId = getInitialNoteId();
  if (!isTauriRuntime()) {
    await loadEditorData(initialNoteId);
    return;
  }
  const pendingDraft = await loadPendingUpdateDraft().catch(() => null);
  recoveryActive = Boolean(pendingDraft);
  await loadEditorData(pendingDraft?.noteId ?? initialNoteId, pendingDraft?.draft ?? null);
  if (pendingDraft) {
    await getCurrentWindow().show();
    await getCurrentWindow().setFocus();
  }
  await registerTauriListeners();
});

watch(
  editorTitle,
  (title) => {
    document.title = title;
    if (isTauriRuntime()) {
      void getCurrentWindow().setTitle(title);
    }
  },
  { immediate: true },
);

onBeforeUnmount(() => {
  disposed = true;
  unlistenOpen?.();
  unlistenPrepare?.();
  unlistenSettings?.();
  if (recoverySaveTimer) {
    window.clearTimeout(recoverySaveTimer);
  }
});

async function closeEditorWindow() {
  recoveryActive = false;
  if (recoverySaveTimer) {
    window.clearTimeout(recoverySaveTimer);
    recoverySaveTimer = null;
  }
  await clearPendingUpdateDraft();
  activeNoteId.value = null;
  editorSession.value += 1;
  initialDraft.value = null;
  note.value = null;
  if (isTauriRuntime()) {
    await getCurrentWindow().hide();
  }
}

async function handleSaveDraft(nextDraft: NoteDraft) {
  const now = Date.now();
  const currentNote = note.value;
  const nextNote: Note = currentNote
    ? {
        ...currentNote,
        attachments: nextDraft.attachments,
        color: nextDraft.color,
        content: nextDraft.content,
        pinned: nextDraft.pinned,
        sortOrder:
          currentNote.pinned === nextDraft.pinned
            ? currentNote.sortOrder
            : getTopSortOrder(
                notes.value.filter((item) => item.id !== currentNote.id),
                nextDraft.pinned,
              ),
        updatedAt: now,
      }
    : {
        id: createId("note"),
        attachments: nextDraft.attachments,
        color: nextDraft.color,
        content: nextDraft.content,
        pinned: nextDraft.pinned,
        sortOrder: getTopSortOrder(notes.value, nextDraft.pinned),
        textHeight: null,
        createdAt: now,
        updatedAt: now,
      };
  await saveNote(nextNote);
  await emit("q-note-note-saved", nextNote);
  await closeEditorWindow();
}

function handleDraftChange(nextDraft: NoteDraft) {
  draft.value = nextDraft;
  if (!recoveryActive) {
    return;
  }
  if (recoverySaveTimer) {
    window.clearTimeout(recoverySaveTimer);
  }
  recoverySaveTimer = window.setTimeout(() => {
    void savePendingUpdateDraft({
      draft: draft.value,
      noteId: activeNoteId.value,
      savedAt: Date.now(),
    });
    recoverySaveTimer = null;
  }, 250);
}

function dragEditorWindow(event: PointerEvent) {
  if (event.button === 0 && isTauriRuntime()) {
    void getCurrentWindow().startDragging();
  }
}
</script>

<template>
  <main v-if="!ready" class="editor-window-shell is-loading">
    <QMark class="loading-mark" />
  </main>
  <NoteEditor
    v-else
    :key="editorSession"
    :initial-draft="initialDraft"
    mode="window"
    :note="note"
    :t="t"
    @cancel="closeEditorWindow"
    @draft-change="handleDraftChange"
    @drag-start="dragEditorWindow"
    @save="handleSaveDraft"
  />
</template>
