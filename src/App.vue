<script setup lang="ts" vapor>
import { getCurrentWindow } from "@tauri-apps/api/window";
import { computed, ref } from "vue";
import "./App.css";
import DockWindowView from "./app/DockWindowView.vue";
import MainWindowView from "./app/MainWindowView.vue";
import { useMenuController } from "./features/menu/useMenuController";
import { useNotesController, useNotesState } from "./features/notes/useNotesController";
import { useSettingsController, useSettingsState } from "./features/settings/useSettingsController";
import { useWindowController } from "./features/window/useWindowController";
import { useAppBoot } from "./hooks/useAppBoot";
import { useDockMode } from "./hooks/useDockMode";
import { useTauriEventBridge } from "./hooks/useTauriEventBridge";
import { useToast } from "./hooks/useToast";
import { useTrayMenuLabels } from "./hooks/useTrayMenuLabels";
import { useUpdateManager } from "./hooks/useUpdateManager";
import { useWindowStatePersistence } from "./hooks/useWindowStatePersistence";
import { translations } from "./i18n";
import { isTauriRuntime } from "./lib/env";
import { prepareEditorForUpdate } from "./lib/updatePreparation";
import { DOCK_WINDOW_LABEL, MAIN_WINDOW_LABEL, captureWindowState } from "./lib/windowControls";
import type { Note, NoteDraft } from "./types";

const editorNote = ref<Note | null | undefined>(undefined);
const ready = ref(false);
const showDeleteAllConfirm = ref(false);
const showSettings = ref(false);
const { showToast, toast } = useToast();
const { commitNotes, notes } = useNotesState();
const { settings } = useSettingsState();

const currentWindowLabel = isTauriRuntime() ? getCurrentWindow().label : MAIN_WINDOW_LABEL;
const isDockWindow = currentWindowLabel === DOCK_WINDOW_LABEL;
const t = computed(() => translations[settings.value.language]);
const dockToggleLabel = computed(() =>
  settings.value.docked ? t.value.switchMainWindow : t.value.switchFloatingBall,
);
const alwaysOnLabel = computed(() =>
  settings.value.alwaysOnTop ? t.value.alwaysOff : t.value.alwaysOn,
);
const editorOpen = computed(() => editorNote.value !== undefined);
const language = computed(() => settings.value.language);

const {
  handleExport,
  handleImport,
  persistSettings,
  toggleAlwaysOnTop,
  toggleAutoStart,
  toggleLanguage,
} = useSettingsController({ commitNotes, notes, settings, showToast, t });

async function prepareForUpdate() {
  if (!settings.value.docked) {
    const snapshot = await captureWindowState(MAIN_WINDOW_LABEL);
    await persistSettings(snapshot ? { window: snapshot } : {});
  } else {
    await persistSettings({});
  }
  await prepareEditorForUpdate();
}

const {
  appVersion,
  bundleType,
  cancelUpdateConfirm,
  checkingUpdate,
  confirmUpdate,
  handleCheckUpdate,
  handleOpenCurrentRelease,
  hasUpdate,
  updateConfirmBody,
  updateConfirmOpen,
  updateDialogOpen,
  updateDownloadProgress,
  updateInfo,
  updatePhase,
} = useUpdateManager({
  currentWindowLabel,
  language,
  prepareForUpdate,
  ready,
  showToast,
});

const {
  collapseToQIcon,
  concealDockIcon,
  dockDrag,
  dockGuard,
  dragQIcon,
  finishQIconDrag,
  moveQIcon,
  openMainFromDockIcon,
  persistIconSnap,
  restoreDock,
  revealDockIcon,
  toggleDockOnEdge,
} = useDockMode({ currentWindowLabel, persistSettings, settings });

const {
  closeEditor,
  handleCopy,
  handleDelete,
  handleDeleteAll,
  handleSaveDraft,
  openEditor,
  patchNote,
  reorderNotes,
} = useNotesController({
  commitNotes,
  editorNote,
  notes,
  restoreDock,
  settings,
  showToast,
  t,
});

const { closeWindow, dragMainWindow, minimizeWindow, quitApp } = useWindowController(() =>
  collapseToQIcon(),
);

const { closeMenu, contextItems, dockMenuItems, menu, openDockMenu, openMenu } = useMenuController({
  alwaysOnLabel,
  dockToggleLabel,
  handleCopy,
  handleDelete,
  notes,
  onDeleteAll: () => {
    showDeleteAllConfirm.value = true;
  },
  openEditor,
  patchNote,
  quitApp,
  settings,
  t,
  toggleAlwaysOnTop,
  toggleDockOnEdge,
  toggleLanguage,
});

useAppBoot({ currentWindowLabel, notes, ready, settings });
useTrayMenuLabels({ alwaysOnLabel, dockToggleLabel, ready, t });
useTauriEventBridge({
  commitNotes,
  notes,
  ready,
  restoreDock,
  settings,
  showToast,
  toggleAlwaysOnTop,
  toggleDockOnEdge,
  toggleLanguage,
});
useWindowStatePersistence({
  currentWindowLabel,
  dockDrag,
  dockGuard,
  editorOpen,
  persistIconSnap,
  persistSettings,
  ready,
  settings,
});

function confirmDeleteAll() {
  showDeleteAllConfirm.value = false;
  void handleDeleteAll();
}

function changeNoteColor(id: string, color: string) {
  void patchNote(id, { color });
}

function changeNoteHeight(id: string, textHeight: number) {
  void patchNote(id, { textHeight });
}

function reorderNote(draggedId: string, targetId: string, placement: "before" | "after") {
  void reorderNotes(draggedId, targetId, placement);
}

function saveDraft(draft: NoteDraft) {
  void handleSaveDraft(draft);
}

function toggleNotePin(id: string) {
  const note = notes.value.find((item) => item.id === id);
  if (note) {
    void patchNote(id, { pinned: !note.pinned });
  }
}
</script>

<template>
  <DockWindowView
    v-if="isDockWindow"
    :menu="menu"
    :menu-items="dockMenuItems"
    :ready="ready"
    :t="t"
    :toast="toast"
    @close-menu="closeMenu"
    @conceal-dock-icon="concealDockIcon"
    @dock-drag-end="finishQIconDrag"
    @dock-drag-move="moveQIcon"
    @dock-drag-start="dragQIcon"
    @open-dock-menu="openDockMenu"
    @open-main="openMainFromDockIcon"
    @reveal-dock-icon="revealDockIcon"
  />
  <MainWindowView
    v-else
    :always-on-label="alwaysOnLabel"
    :always-on-top="settings.alwaysOnTop"
    :app-version="appVersion"
    :auto-start="settings.autoStart"
    :bundle-type="bundleType"
    :checking-update="checkingUpdate"
    :context-items="contextItems"
    :dock-button-label="t.switchFloatingBall"
    :editor-note="editorNote"
    :has-update="hasUpdate"
    :menu="menu"
    :notes="notes"
    :ready="ready"
    :show-delete-all-confirm="showDeleteAllConfirm"
    :show-settings="showSettings"
    :t="t"
    :toast="toast"
    :update-confirm-body="updateConfirmBody"
    :update-confirm-open="updateConfirmOpen"
    :update-dialog-open="updateDialogOpen"
    :update-download-progress="updateDownloadProgress"
    :update-info="updateInfo"
    :update-phase="updatePhase"
    @cancel-editor="closeEditor"
    @cancel-update-confirm="cancelUpdateConfirm"
    @check-update="handleCheckUpdate"
    @close-confirm-delete-all="showDeleteAllConfirm = false"
    @close-menu="closeMenu"
    @close-settings="showSettings = false"
    @close-window="closeWindow"
    @collapse-to-dock="collapseToQIcon({ useRevealAnchor: true })"
    @color-change="changeNoteColor"
    @confirm-delete-all="confirmDeleteAll"
    @confirm-update="confirmUpdate"
    @copy-note="handleCopy"
    @delete-all="showDeleteAllConfirm = true"
    @delete-note="handleDelete"
    @drag-main-window="dragMainWindow"
    @edit-note="openEditor"
    @export="handleExport"
    @height-change="changeNoteHeight"
    @import="handleImport"
    @minimize-window="minimizeWindow"
    @new-note="openEditor(null)"
    @open-current-release="handleOpenCurrentRelease"
    @open-menu="openMenu"
    @open-settings="showSettings = true"
    @reorder-notes="reorderNote"
    @save-draft="saveDraft"
    @toggle-always-on-top="toggleAlwaysOnTop"
    @toggle-auto-start="toggleAutoStart"
    @toggle-language="toggleLanguage"
    @toggle-note-pin="toggleNotePin"
  />
</template>
