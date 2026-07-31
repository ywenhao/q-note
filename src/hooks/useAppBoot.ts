import { useEffect, type Dispatch, type MutableRefObject, type SetStateAction } from "react";
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
  notesRef: MutableRefObject<Note[]>;
  settingsRef: MutableRefObject<AppSettings>;
  setNotes: Dispatch<SetStateAction<Note[]>>;
  setReady: Dispatch<SetStateAction<boolean>>;
  setSettings: Dispatch<SetStateAction<AppSettings>>;
}

export function useAppBoot({
  currentWindowLabel,
  notesRef,
  settingsRef,
  setNotes,
  setReady,
  setSettings,
}: UseAppBootOptions) {
  useEffect(() => {
    let cancelled = false;

    async function boot() {
      const data = await loadAppData();
      if (cancelled) {
        return;
      }

      const bootSettings =
        currentWindowLabel === MAIN_WINDOW_LABEL && data.settings.docked
          ? {
              ...data.settings,
              docked: false,
              dockEdge: null,
              keepFullMain: false,
            }
          : data.settings;

      notesRef.current = sortNotes(data.notes);
      settingsRef.current = bootSettings;
      setNotes(notesRef.current);
      setSettings(bootSettings);
      await applyAlwaysOnTop(bootSettings.alwaysOnTop);
      const autoStart = await readAutoStartEnabled();
      if (autoStart !== bootSettings.autoStart) {
        settingsRef.current = { ...settingsRef.current, autoStart };
        setSettings(settingsRef.current);
      }

      if (settingsRef.current !== data.settings) {
        await saveSettings(settingsRef.current);
      }

      if (currentWindowLabel === MAIN_WINDOW_LABEL) {
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

      setReady(true);
    }

    void boot();

    return () => {
      cancelled = true;
    };
  }, [currentWindowLabel, notesRef, setNotes, setReady, setSettings, settingsRef]);
}
