import { emit } from "@tauri-apps/api/event";
import { ref, type ComputedRef, type Ref } from "vue";
import type { Translation } from "../../i18n";
import { applyAutoStart } from "../../lib/autoStart";
import { isTauriRuntime } from "../../lib/env";
import { exportJson, importJson } from "../../lib/fileIo";
import {
  createDefaultSettings,
  createExportPayload,
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
  notes: Ref<Note[]>;
  settings: Ref<AppSettings>;
  showToast: ShowToast;
  t: ComputedRef<Translation>;
}

export function useSettingsState() {
  const settings = ref<AppSettings>(createDefaultSettings());
  return { settings };
}

export function useSettingsController(options: UseSettingsControllerOptions) {
  const { commitNotes, notes, settings, showToast, t } = options;

  async function persistSettings(patch: Partial<AppSettings>) {
    const nextSettings = normalizeSettings({ ...settings.value, ...patch });
    await saveSettings(nextSettings);
    settings.value = nextSettings;
    if (isTauriRuntime()) {
      await emit("q-note-settings-updated", nextSettings);
    }
    return nextSettings;
  }

  async function handleExport() {
    const exported = await exportJson(
      createExportPayload({ notes: notes.value, settings: settings.value }),
    );
    if (exported) {
      showToast(t.value.exported);
    }
  }

  async function handleImport() {
    try {
      const payload = await importJson();
      if (!payload) {
        return;
      }
      const nextData = normalizeImportPayload(payload);
      await replaceAppData(nextData);
      commitNotes(nextData.notes);
      settings.value = nextData.settings;
      await applyAlwaysOnTop(nextData.settings.alwaysOnTop);
      const autoStart = await applyAutoStart(nextData.settings.autoStart);
      await persistSettings({ autoStart });
      showToast(t.value.imported);
    } catch {
      showToast(t.value.importFailed, { kind: "error" });
    }
  }

  async function toggleAlwaysOnTop() {
    const nextValue = !settings.value.alwaysOnTop;
    try {
      await persistSettings({ alwaysOnTop: nextValue });
      await applyAlwaysOnTop(nextValue);
    } catch {
      showToast(t.value.saveFailed, { kind: "error" });
    }
  }

  async function toggleAutoStart() {
    try {
      const autoStart = await applyAutoStart(!settings.value.autoStart);
      await persistSettings({ autoStart });
      showToast(t.value.autoStartUpdated);
    } catch {
      showToast(t.value.autoStartFailed, { kind: "error" });
    }
  }

  async function toggleLanguage() {
    await persistSettings({ language: settings.value.language === "zh" ? "en" : "zh" });
  }

  return {
    handleExport,
    handleImport,
    persistSettings,
    toggleAlwaysOnTop,
    toggleAutoStart,
    toggleLanguage,
  };
}
