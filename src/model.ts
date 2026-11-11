import {
  batch,
  cleanup,
  Clipboard,
  computed,
  locale,
  openUrl,
  platform,
  signal,
  Storage,
} from "nsh";
import { Application } from "nsh/desktop";
import { Files, Startup, Updates } from "q-note/native";
import { translations } from "./i18n";
import { createExportPayload, normalizeImportPayload } from "./lib/appData";
import {
  getTopSortOrder,
  normalizeManualOrder,
  sortNotes,
} from "./lib/noteOrdering";
import * as persistence from "./lib/storage";
import { createDefaultSettings } from "./lib/settingsState";
import { createNoteDraft, createPendingUpdateDraft } from "./lib/updateDraft";
import { PACKAGE_VERSION, getReleaseTagUrl } from "./lib/repository";
import type { AppSettings, Note, NoteAttachment, NoteDraft } from "./types";

interface UpdateInfo {
  version: string;
  notes: string | null;
  url: string;
  canInstall: boolean;
}

export function createModel() {
  const notes = signal<Note[]>([]);
  const settings = signal<AppSettings>({
    ...createDefaultSettings(),
    language: locale().startsWith("zh") ? "zh" : "en",
  });
  const t = computed(() => translations[settings().language]);
  const loading = signal(true);
  const saving = signal(false);
  const toast = signal("");
  const settingsOpen = signal(false);
  const deleteOpen = signal(false);
  const docked = signal(false);
  const editor = signal<{ key: string; original: Note | null } | null>(null);
  const draft = signal<NoteDraft>(createNoteDraft(null));
  const preview = signal<{
    images: NoteAttachment[];
    index: number;
    editor: boolean;
  } | null>(null);
  const update = signal<UpdateInfo | null>(null);
  const updateOpen = signal(false);
  const checking = signal(false);
  const updatePhase = signal<"" | "downloading" | "preparing" | "installing">(
    "",
  );
  const progress = signal({ received: 0, total: 0 });
  let toastTimer: ReturnType<typeof setTimeout> | undefined;
  cleanup(() => clearTimeout(toastTimer));
  let cancelled = false;
  let desktopSync = () => {};
  let flushWindow = async () => {};

  function notify(text: string) {
    clearTimeout(toastTimer);
    toast.set(text);
    toastTimer = setTimeout(() => toast.set(""), 1700);
  }

  async function guard(work: () => Promise<void>, message: () => string) {
    if (saving()) return false;
    saving.set(true);
    try {
      await work();
      return true;
    } catch (error) {
      console.error(error);
      notify(message());
      return false;
    } finally {
      saving.set(false);
    }
  }

  async function boot() {
    try {
      const data = await persistence.loadAppData();
      data.settings = { ...data.settings, docked: false, keepFullMain: false };
      batch(() => {
        notes.set(sortNotes(data.notes));
        settings.set(data.settings);
      });
      try {
        if (platform.os !== "web")
          settings.set({ ...settings(), autoStart: Startup.enabled() });
      } catch (error) {
        console.error(error);
      }
      const recovery = await persistence.loadPendingUpdateDraft();
      if (recovery) {
        editor.set({
          key: crypto.randomUUID(),
          original: notes().find((n) => n.id === recovery.noteId) ?? null,
        });
        draft.set(recovery.draft);
      }
    } catch (error) {
      console.error(error);
      notify(t().loadFailed);
    } finally {
      loading.set(false);
    }
  }

  function openEditor(note: Note | null = null) {
    docked.set(false);
    preview.set(null);
    draft.set(createNoteDraft(note));
    editor.set({ key: crypto.randomUUID(), original: note });
    desktopSync();
  }

  async function closeEditor() {
    if (saving()) return;
    const closed = await guard(
      async () => {
        await persistence.clearPendingUpdateDraft();
      },
      () => t().saveFailed,
    );
    if (closed) {
      editor.set(null);
      preview.set(null);
    }
  }

  async function saveEditor() {
    if (!editor()) return;
    if (!draft().content.trim() && !draft().attachments.length) {
      await closeEditor();
      return;
    }
    const session = editor()!;
    const current = draft();
    const original = session.original;
    const now = Date.now();
    const note: Note = {
      ...current,
      content: current.content.trim(),
      id: original?.id ?? crypto.randomUUID(),
      sortOrder:
        original && original.pinned === current.pinned
          ? original.sortOrder
          : getTopSortOrder(notes(), current.pinned),
      createdAt: original?.createdAt ?? now,
      updatedAt: now,
      textHeight: original?.textHeight ?? null,
    };
    await guard(
      async () => {
        await persistence.saveNote(note, true);
        notes.set(
          sortNotes([note, ...notes().filter((n) => n.id !== note.id)]),
        );
        editor.set(null);
        preview.set(null);
        notify(t().saved);
      },
      () => t().saveFailed,
    );
  }

  async function changeNote(note: Note, patch: Partial<Note>) {
    await guard(
      async () => {
        const next = { ...note, ...patch };
        if (patch.pinned !== undefined && patch.pinned !== note.pinned)
          next.sortOrder = getTopSortOrder(notes(), patch.pinned);
        await persistence.saveNote(next);
        notes.set(sortNotes(notes().map((n) => (n.id === next.id ? next : n))));
      },
      () => t().saveFailed,
    );
  }

  async function remove(id: string) {
    await guard(
      async () => {
        await persistence.deleteNote(id);
        notes.set(notes().filter((n) => n.id !== id));
      },
      () => t().deleteFailed,
    );
  }

  async function clear() {
    await guard(
      async () => {
        await persistence.deleteAllNotes();
        notes.set([]);
        deleteOpen.set(false);
      },
      () => t().deleteFailed,
    );
  }

  async function reorder(id: string, target: string) {
    if (id === target) return;
    await guard(
      async () => {
        const list = notes().map((n) => ({ ...n }));
        const from = list.findIndex((n) => n.id === id);
        const to = list.findIndex((n) => n.id === target);
        if (from < 0 || to < 0) return;
        const moving = list.splice(from, 1)[0]!;
        moving.pinned = list.find((n) => n.id === target)!.pinned;
        list.splice(to, 0, moving);
        const next = normalizeManualOrder(list);
        await persistence.saveNotesOrder(next);
        notes.set(sortNotes(next));
      },
      () => t().saveFailed,
    );
  }

  async function copy(note: Note) {
    try {
      await Clipboard.writeText(
        note.content || note.attachments.map((a) => a.value).join("\n"),
      );
      notify(t().copied);
    } catch (error) {
      console.error(error);
      notify(t().copyFailed);
    }
  }

  async function changeSettings(patch: Partial<AppSettings>) {
    const next = { ...settings(), ...patch };
    return guard(
      async () => {
        await persistence.saveSettings(next);
        settings.set(next);
        desktopSync();
      },
      () => t().saveFailed,
    );
  }

  async function toggleAutoStart(checked: boolean) {
    const previous = settings().autoStart;
    await guard(
      async () => {
        if (platform.os === "web")
          throw new Error("Launch at login requires the desktop app");
        Startup.setEnabled(checked);
        try {
          await persistence.saveSettings({ ...settings(), autoStart: checked });
        } catch (error) {
          Startup.setEnabled(previous);
          throw error;
        }
        settings.set({ ...settings(), autoStart: checked });
        notify(t().autoStartUpdated);
      },
      () => t().autoStartFailed,
    );
  }

  async function exportData() {
    try {
      const date = new Date();
      const pad = (v: number) => String(v).padStart(2, "0");
      const filename = `q-note_${date.getFullYear()}${pad(date.getMonth() + 1)}${pad(date.getDate())}${pad(date.getHours())}${pad(date.getMinutes())}.json`;
      if (
        await Files.export(
          JSON.stringify(
            createExportPayload({ notes: notes(), settings: settings() }),
            null,
            2,
          ),
          filename,
        )
      )
        notify(t().exported);
    } catch (error) {
      console.error(error);
      notify(t().exportFailed);
    }
  }

  async function importData() {
    await guard(
      async () => {
        const raw = await Files.import();
        if (raw === null) return;
        const data = normalizeImportPayload(JSON.parse(raw));
        data.settings.docked = false;
        const previous = settings();
        if (platform.os !== "web") Startup.setEnabled(data.settings.autoStart);
        try {
          await persistence.replaceAppData(data);
        } catch (error) {
          if (platform.os !== "web") Startup.setEnabled(previous.autoStart);
          throw error;
        }
        batch(() => {
          notes.set(sortNotes(data.notes));
          settings.set(data.settings);
          editor.set(null);
          docked.set(false);
        });
        desktopSync();
        notify(t().imported);
      },
      () => t().importFailed,
    );
  }

  async function secureData() {
    await flushWindow();
    await persistence.savePendingUpdateDraft(
      createPendingUpdateDraft({
        draft: draft(),
        note: editor()?.original ?? null,
        now: Date.now(),
        visible: !!editor(),
      }),
    );
    await persistence.flushDatabase();
  }

  async function checkUpdate(manual = false) {
    if (checking() || updatePhase()) return;
    checking.set(true);
    try {
      const raw = await Updates.check();
      const available: UpdateInfo | null = raw ? JSON.parse(raw) : null;
      update.set(available);
      if (available) updateOpen.set(true);
      else if (manual) notify(t().updateNone);
    } catch (error) {
      console.error(error);
      if (manual) notify(t().updateCheckFailed);
    } finally {
      checking.set(false);
    }
  }

  async function installUpdate() {
    const available = update();
    if (!available || updatePhase()) return;
    if (!available.canInstall) {
      await openUrl(getReleaseTagUrl(available.version));
      notify(t().updateOpenRelease);
      updateOpen.set(false);
      return;
    }
    cancelled = false;
    updatePhase.set("downloading");
    progress.set({ received: 0, total: 0 });
    const progressTimer = setInterval(() => {
      if (!cancelled) progress.set(JSON.parse(Updates.progress()));
    }, 100);
    try {
      await Updates.download();
      progress.set(JSON.parse(Updates.progress()));
      if (cancelled) return;
      updatePhase.set("preparing");
      await secureData();
      if (cancelled) return;
      updatePhase.set("installing");
      await Updates.install();
      Application.restart();
    } catch (error) {
      console.error(error);
      if (!cancelled)
        notify(
          updatePhase() === "downloading"
            ? t().updateDownloadFailed
            : updatePhase() === "preparing"
              ? t().updatePrepareFailed
              : t().updateInstallFailed,
        );
    } finally {
      clearInterval(progressTimer);
      updatePhase.set("");
      updateOpen.set(false);
    }
  }

  function cancelUpdate() {
    if (updatePhase() === "installing") return;
    cancelled = true;
    Updates.cancel();
    updateOpen.set(false);
  }
  const version = PACKAGE_VERSION;

  return {
    notes,
    settings,
    t,
    loading,
    saving,
    toast,
    settingsOpen,
    deleteOpen,
    docked,
    editor,
    draft,
    preview,
    update,
    updateOpen,
    checking,
    updatePhase,
    progress,
    version,
    boot,
    notify,
    openEditor,
    closeEditor,
    saveEditor,
    changeNote,
    remove,
    clear,
    reorder,
    copy,
    changeSettings,
    toggleAutoStart,
    exportData,
    importData,
    secureData,
    checkUpdate,
    installUpdate,
    cancelUpdate,
    setDesktopSync(fn: () => void) {
      desktopSync = fn;
    },
    setFlushWindow(fn: () => Promise<void>) {
      flushWindow = fn;
    },
    currentRelease: () => openUrl(getReleaseTagUrl(version)),
    toggleLanguage: () =>
      changeSettings({ language: settings().language === "zh" ? "en" : "zh" }),
    updateDraft(patch: Partial<NoteDraft>) {
      draft.set({ ...draft(), ...patch });
    },
  };
}

export type NoteModel = ReturnType<typeof createModel>;
