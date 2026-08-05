import { onBeforeUnmount, onMounted, type Ref } from "vue";
import { translations } from "../i18n";
import { readAutoStartEnabled } from "../lib/autoStart";
import { sortNotes } from "../lib/noteOrdering";
import { loadAppData, loadPendingUpdateDraft, saveSettings } from "../lib/storage";
import { restorePendingUpdateEditor } from "../lib/updateRecovery";
import {
  MAIN_WINDOW_LABEL,
  applyAlwaysOnTop,
  hideDockWindow,
  openEditorWindow,
  positionMainWindowAtStartup,
} from "../lib/windowControls";
import type { AppSettings, Note } from "../types";

interface UseAppBootOptions {
  currentWindowLabel: string;
  notes: Ref<Note[]>;
  ready: Ref<boolean>;
  settings: Ref<AppSettings>;
}

export function useAppBoot(options: UseAppBootOptions) {
  let cancelled = false;

  onMounted(async () => {
    const data = await loadAppData();
    if (cancelled) {
      return;
    }
    const bootSettings =
      options.currentWindowLabel === MAIN_WINDOW_LABEL && data.settings.docked
        ? {
            ...data.settings,
            docked: false,
            dockEdge: null,
            keepFullMain: false,
          }
        : data.settings;

    options.notes.value = sortNotes(data.notes);
    options.settings.value = bootSettings;
    await applyAlwaysOnTop(bootSettings.alwaysOnTop);
    const autoStart = await readAutoStartEnabled();
    if (autoStart !== bootSettings.autoStart) {
      options.settings.value = { ...options.settings.value, autoStart };
    }
    if (options.settings.value !== data.settings) {
      await saveSettings(options.settings.value);
    }

    if (options.currentWindowLabel === MAIN_WINDOW_LABEL) {
      await positionMainWindowAtStartup(bootSettings.window);
      await hideDockWindow();
      const t = translations[bootSettings.language];
      await restorePendingUpdateEditor(loadPendingUpdateDraft, (noteId) =>
        openEditorWindow(
          noteId,
          bootSettings.alwaysOnTop,
          noteId ? t.editorEditTitle : t.editorNewTitle,
        ),
      ).catch(() => false);
    }
    if (!cancelled) {
      options.ready.value = true;
    }
  });

  onBeforeUnmount(() => {
    cancelled = true;
  });
}
