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
import { useWindowStatePersistence } from "../hooks/useWindowStatePersistence";
import { translations } from "../i18n";
import { isTauriRuntime } from "../lib/env";
import { DOCK_WINDOW_LABEL } from "../lib/windowControls";
import type { Note } from "../types";

export function useDockWindow() {
  const editorNote = ref<Note | null | undefined>(undefined);
  const ready = ref(false);
  const { showToast, toast } = useToast();
  const { commitNotes, notes } = useNotesState();
  const { settings } = useSettingsState();

  const currentWindowLabel = isTauriRuntime() ? getCurrentWindow().label : DOCK_WINDOW_LABEL;
  const t = computed(() => translations[settings.value.language]);
  const dockToggleLabel = computed(() =>
    settings.value.docked ? t.value.switchMainWindow : t.value.switchFloatingBall,
  );
  const alwaysOnLabel = computed(() =>
    settings.value.alwaysOnTop ? t.value.alwaysOff : t.value.alwaysOn,
  );
  const editorOpen = computed(() => editorNote.value !== undefined);

  const { persistSettings, toggleAlwaysOnTop, toggleLanguage } = useSettingsController({
    commitNotes,
    notes,
    settings,
    showToast,
    t,
  });

  const {
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

  const { handleCopy, handleDelete, openEditor, patchNote } = useNotesController({
    commitNotes,
    editorNote,
    notes,
    restoreDock,
    settings,
    showToast,
    t,
  });

  const { quitApp } = useWindowController(() => restoreDock({ keepFull: true }));

  const { closeMenu, dockMenuItems, menu, openDockMenu } = useMenuController({
    alwaysOnLabel,
    dockToggleLabel,
    handleCopy,
    handleDelete,
    notes,
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

  return {
    closeMenu,
    concealDockIcon,
    dockMenuItems,
    dragQIcon,
    finishQIconDrag,
    menu,
    moveQIcon,
    openDockMenu,
    openMainFromDockIcon,
    ready,
    revealDockIcon,
    t,
    toast,
  };
}
