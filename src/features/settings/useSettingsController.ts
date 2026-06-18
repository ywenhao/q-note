import { emit } from "@tauri-apps/api/event";
import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type Dispatch,
  type MutableRefObject,
  type SetStateAction,
} from "react";
import type { Translation } from "../../i18n";
import { applyAutoStart } from "../../lib/autoStart";
import { isTauriRuntime } from "../../lib/env";
import { exportJson, importJson } from "../../lib/fileIo";
import {
  createExportPayload,
  createDefaultSettings,
  normalizeImportPayload,
  normalizeSettings,
  replaceAppData,
  saveSettings,
} from "../../lib/storage";
import { applyAlwaysOnTop } from "../../lib/windowControls";
import type { AppSettings, Note } from "../../types";
import type { ShowToast } from "../../hooks/useToast";

interface UseSettingsControllerOptions {
  commitNotes: (nextNotes: Note[]) => void;
  notesRef: MutableRefObject<Note[]>;
  setSettings: Dispatch<SetStateAction<AppSettings>>;
  settingsRef: MutableRefObject<AppSettings>;
  showToast: ShowToast;
  t: Translation;
}

export function useSettingsState() {
  const [settings, setSettings] = useState<AppSettings>(() => createDefaultSettings());
  const settingsRef = useRef(settings);

  useEffect(() => {
    settingsRef.current = settings;
  }, [settings]);

  return {
    setSettings,
    settings,
    settingsRef,
  };
}

export function useSettingsController({
  commitNotes,
  notesRef,
  setSettings,
  settingsRef,
  showToast,
  t,
}: UseSettingsControllerOptions) {
  const persistSettings = useCallback(
    async (patch: Partial<AppSettings>) => {
      const nextSettings = normalizeSettings({ ...settingsRef.current, ...patch });
      settingsRef.current = nextSettings;
      setSettings(nextSettings);
      await saveSettings(nextSettings);

      if (isTauriRuntime()) {
        await emit("q-note-settings-updated", nextSettings);
      }
    },
    [setSettings, settingsRef],
  );

  const handleExport = useCallback(async () => {
    const exported = await exportJson(
      createExportPayload({
        notes: notesRef.current,
        settings: settingsRef.current,
      }),
    );

    if (exported) {
      showToast(t.exported);
    }
  }, [notesRef, settingsRef, showToast, t]);

  const handleImport = useCallback(async () => {
    try {
      const payload = await importJson();
      if (!payload) {
        return;
      }

      const nextData = normalizeImportPayload(payload);
      commitNotes(nextData.notes);
      settingsRef.current = nextData.settings;
      setSettings(nextData.settings);
      await replaceAppData(nextData);
      await applyAlwaysOnTop(nextData.settings.alwaysOnTop);
      const autoStart = await applyAutoStart(nextData.settings.autoStart);
      await persistSettings({ autoStart });
      showToast(t.imported);
    } catch {
      showToast(t.importFailed, { kind: "error" });
    }
  }, [commitNotes, persistSettings, setSettings, settingsRef, showToast, t]);

  const toggleAlwaysOnTop = useCallback(async () => {
    const nextValue = !settingsRef.current.alwaysOnTop;
    await applyAlwaysOnTop(nextValue);
    await persistSettings({ alwaysOnTop: nextValue });
  }, [persistSettings, settingsRef]);

  const toggleAutoStart = useCallback(async () => {
    try {
      const autoStart = await applyAutoStart(!settingsRef.current.autoStart);
      await persistSettings({ autoStart });
      showToast(t.autoStartUpdated);
    } catch {
      showToast(t.autoStartFailed, { kind: "error" });
    }
  }, [persistSettings, settingsRef, showToast, t]);

  const toggleLanguage = useCallback(async () => {
    await persistSettings({
      language: settingsRef.current.language === "zh" ? "en" : "zh",
    });
  }, [persistSettings, settingsRef]);

  return {
    handleExport,
    handleImport,
    persistSettings,
    toggleAlwaysOnTop,
    toggleAutoStart,
    toggleLanguage,
  };
}
