import { getCurrentWindow } from "@tauri-apps/api/window";
import { computed, ref } from "vue";
import { useMenuController } from "../features/menu/useMenuController";
import { useNotesController, useNotesState } from "../features/notes/useNotesController";
import {
  useSettingsController,
  useSettingsState,
} from "../features/settings/useSettingsController";
import { useWindowController } from "../features/window/useWindowController";
import { useAppBoot } from "../hooks/useAppBoot";
import { useDockMode } from "../hooks/useDockMode";
import { useTauriEventBridge } from "../hooks/useTauriEventBridge";
import { useToast } from "../hooks/useToast";
import { useTrayMenuLabels } from "../hooks/useTrayMenuLabels";
import { useUpdateManager } from "../hooks/useUpdateManager";
import { useWindowStatePersistence } from "../hooks/useWindowStatePersistence";
import { translations } from "../i18n";
import { isTauriRuntime } from "../lib/env";
import { prepareEditorForUpdate } from "../lib/updatePreparation";
import { MAIN_WINDOW_LABEL, captureWindowState } from "../lib/windowControls";
import type { Note, NoteDraft } from "../types";

export function useMainWindow() {
  const editorNote = ref<Note | null | undefined>(undefined);
  const ready = ref(false);
  const showDeleteAllConfirm = ref(false);
  const showSettings = ref(false);
  const { showToast, toast } = useToast();
  const { commitNotes, notes } = useNotesState();
  const { settings } = useSettingsState();

  const currentWindowLabel = isTauriRuntime() ? getCurrentWindow().label : MAIN_WINDOW_LABEL;
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
    cancelUpdateDownload,
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

  const { collapseToQIcon, dockDrag, dockGuard, persistIconSnap, restoreDock, toggleDockOnEdge } =
    useDockMode({ currentWindowLabel, persistSettings, settings });

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

  const { closeWindow, minimizeWindow, quitApp } = useWindowController(() => collapseToQIcon());

  const { closeMenu, contextItems, menu, openMenu } = useMenuController({
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

  function collapseToDock() {
    void collapseToQIcon({ useRevealAnchor: true });
  }

  return {
    alwaysOnLabel,
    appVersion,
    bundleType,
    cancelUpdateConfirm,
    cancelUpdateDownload,
    changeNoteColor,
    changeNoteHeight,
    checkingUpdate,
    closeEditor,
    closeMenu,
    closeWindow,
    collapseToDock,
    confirmDeleteAll,
    confirmUpdate,
    contextItems,
    editorNote,
    handleCheckUpdate,
    handleCopy,
    handleDelete,
    handleExport,
    handleImport,
    handleOpenCurrentRelease,
    hasUpdate,
    menu,
    minimizeWindow,
    notes,
    openEditor,
    openMenu,
    ready,
    reorderNote,
    saveDraft,
    settings,
    showDeleteAllConfirm,
    showSettings,
    t,
    toast,
    toggleAlwaysOnTop,
    toggleAutoStart,
    toggleLanguage,
    toggleNotePin,
    updateConfirmBody,
    updateConfirmOpen,
    updateDialogOpen,
    updateDownloadProgress,
    updateInfo,
    updatePhase,
  };
}
