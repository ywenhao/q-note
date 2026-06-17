import { invoke } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";
import { Menu } from "@tauri-apps/api/menu";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type MouseEvent,
  type PointerEvent,
} from "react";
import "./App.css";
import { AppHeader } from "./components/AppHeader";
import { AppToolbar } from "./components/AppToolbar";
import { ConfirmDialog } from "./components/ConfirmDialog";
import { CompactDock } from "./components/CompactDock";
import { ContextMenu, type ContextMenuItem } from "./components/ContextMenu";
import { NoteEditor, type NoteDraft } from "./components/NoteEditor";
import { NoteList } from "./components/NoteList";
import { QMark } from "./components/QMark";
import { SettingsDialog } from "./components/SettingsDialog";
import { StatusBar } from "./components/StatusBar";
import { Toast } from "./components/Toast";
import { UpdateDownloadDialog } from "./components/UpdateDownloadDialog";
import { useAppBoot } from "./hooks/useAppBoot";
import { useDockMode } from "./hooks/useDockMode";
import { useTauriEventBridge } from "./hooks/useTauriEventBridge";
import { useToast } from "./hooks/useToast";
import { useTrayMenuLabels } from "./hooks/useTrayMenuLabels";
import { useUpdateManager } from "./hooks/useUpdateManager";
import { useWindowStatePersistence } from "./hooks/useWindowStatePersistence";
import { translations } from "./i18n";
import { applyAutoStart } from "./lib/autoStart";
import { writeClipboard } from "./lib/clipboard";
import { createId, isTauriRuntime } from "./lib/env";
import { exportJson, importJson } from "./lib/fileIo";
import { createDockMenuItems, createMainContextItems } from "./lib/menuItems";
import { getTopSortOrder, normalizeManualOrder, sortNotes } from "./lib/noteOrdering";
import {
  createDefaultSettings,
  createExportPayload,
  deleteAllNotes,
  deleteNote,
  normalizeSettings,
  normalizeImportPayload,
  replaceAppData,
  saveNote,
  saveNotesOrder,
  saveSettings,
} from "./lib/storage";
import {
  DOCK_WINDOW_LABEL,
  MAIN_WINDOW_LABEL,
  applyAlwaysOnTop,
  openEditorWindow,
  startMainWindowDrag,
} from "./lib/windowControls";
import type { AppSettings, Note } from "./types";

interface MenuState {
  noteId?: string;
  x: number;
  y: number;
}

function App() {
  const [editorNote, setEditorNote] = useState<Note | null | undefined>(undefined);
  const [menu, setMenu] = useState<MenuState | null>(null);
  const [notes, setNotes] = useState<Note[]>([]);
  const [ready, setReady] = useState(false);
  const [settings, setSettings] = useState<AppSettings>(() => createDefaultSettings());
  const [showDeleteAllConfirm, setShowDeleteAllConfirm] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const { showToast, toast } = useToast();

  const notesRef = useRef<Note[]>([]);
  const settingsRef = useRef(settings);

  const currentWindowLabel = isTauriRuntime() ? getCurrentWindow().label : MAIN_WINDOW_LABEL;
  const isDockWindow = currentWindowLabel === DOCK_WINDOW_LABEL;
  const t = translations[settings.language];
  const {
    appVersion,
    checkingUpdate,
    handleCancelUpdateDownload,
    handleCheckUpdate,
    handleOpenCurrentRelease,
    handleRevealDownloadedUpdate,
    setUpdateDialogOpen,
    updateDialogOpen,
    updateDownloadProgress,
    updateDownloadResult,
    updateInfo,
  } = useUpdateManager({
    currentWindowLabel,
    language: settings.language,
    ready,
    showToast,
  });
  const editorOpen = editorNote !== undefined;
  const dockToggleLabel = settings.docked ? t.switchMainWindow : t.switchFloatingBall;
  const alwaysOnLabel = settings.alwaysOnTop ? t.alwaysOff : t.alwaysOn;

  const commitNotes = useCallback((nextNotes: Note[]) => {
    const sorted = sortNotes(nextNotes);
    notesRef.current = sorted;
    setNotes(sorted);
  }, []);

  const persistSettings = useCallback(async (patch: Partial<AppSettings>) => {
    const nextSettings = normalizeSettings({ ...settingsRef.current, ...patch });
    settingsRef.current = nextSettings;
    setSettings(nextSettings);
    await saveSettings(nextSettings);

    if (isTauriRuntime()) {
      await emit("q-note-settings-updated", nextSettings);
    }
  }, []);

  const {
    collapseToQIcon,
    concealDockIcon,
    dockDragRef,
    dockGuardRef,
    dragQIcon,
    finishQIconDrag,
    moveQIcon,
    openMainFromDockIcon,
    persistIconSnap,
    restoreDock,
    revealDockIcon,
    toggleDockOnEdge,
  } = useDockMode({
    persistSettings,
    settingsRef,
  });

  async function openEditor(note: Note | null) {
    if (settingsRef.current.docked) {
      await restoreDock({ keepFull: true });
    }

    if (isTauriRuntime()) {
      await openEditorWindow(
        note?.id ?? null,
        settingsRef.current.alwaysOnTop,
        note
          ? translations[settingsRef.current.language].editorEditTitle
          : translations[settingsRef.current.language].editorNewTitle,
      );
      return;
    }

    setEditorNote(note);
  }

  async function closeEditor() {
    setEditorNote(undefined);
  }

  async function handleSaveDraft(draft: NoteDraft) {
    const now = Date.now();
    const nextNote: Note = editorNote
      ? {
          ...editorNote,
          attachments: draft.attachments,
          color: draft.color,
          content: draft.content,
          pinned: draft.pinned,
          sortOrder:
            editorNote.pinned === draft.pinned
              ? editorNote.sortOrder
              : getTopSortOrder(
                  notesRef.current.filter((note) => note.id !== editorNote.id),
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
          sortOrder: getTopSortOrder(notesRef.current, draft.pinned),
          textHeight: null,
          createdAt: now,
          updatedAt: now,
        };

    commitNotes([nextNote, ...notesRef.current.filter((note) => note.id !== nextNote.id)]);
    await saveNote(nextNote);
    showToast(t.saved);
    await closeEditor();
  }

  async function handleCopy(note: Note) {
    const copyValue = note.content.trim()
      ? note.content
      : note.attachments.map((attachment) => attachment.value).join("\n");

    if (!copyValue) {
      return;
    }

    await writeClipboard(copyValue);
    showToast(t.copied);
  }

  async function patchNote(id: string, patch: Partial<Note>) {
    const target = notesRef.current.find((note) => note.id === id);
    if (!target) {
      return;
    }

    const targetPinned =
      typeof patch.pinned === "boolean" && patch.pinned !== target.pinned
        ? patch.pinned
        : target.pinned;
    const nextNote = {
      ...target,
      ...patch,
      sortOrder:
        typeof patch.pinned === "boolean" && patch.pinned !== target.pinned
          ? getTopSortOrder(
              notesRef.current.filter((note) => note.id !== id),
              targetPinned,
            )
          : (patch.sortOrder ?? target.sortOrder),
    };
    commitNotes([nextNote, ...notesRef.current.filter((note) => note.id !== id)]);
    await saveNote(nextNote);
  }

  async function reorderNotes(draggedId: string, targetId: string, placement: "before" | "after") {
    if (draggedId === targetId) {
      return;
    }

    const draggedNote = notesRef.current.find((note) => note.id === draggedId);
    const targetNote = notesRef.current.find((note) => note.id === targetId);
    if (!draggedNote || !targetNote) {
      return;
    }

    const nextNotes = notesRef.current.filter((note) => note.id !== draggedId);
    const targetIndex = nextNotes.findIndex((note) => note.id === targetId);
    if (targetIndex < 0) {
      return;
    }

    nextNotes.splice(targetIndex + (placement === "after" ? 1 : 0), 0, {
      ...draggedNote,
      pinned: targetNote.pinned,
    });

    const orderedNotes = normalizeManualOrder(nextNotes);
    commitNotes(orderedNotes);
    await saveNotesOrder(orderedNotes);
  }

  async function handleDelete(id: string) {
    commitNotes(notesRef.current.filter((note) => note.id !== id));
    await deleteNote(id);
  }

  async function handleDeleteAll() {
    commitNotes([]);
    setShowDeleteAllConfirm(false);
    await deleteAllNotes();
  }

  async function handleExport() {
    const exported = await exportJson(
      createExportPayload({
        notes: notesRef.current,
        settings: settingsRef.current,
      }),
    );

    if (exported) {
      showToast(t.exported);
    }
  }

  async function handleImport() {
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
      showToast(t.importFailed);
    }
  }

  async function toggleAlwaysOnTop() {
    const nextValue = !settingsRef.current.alwaysOnTop;
    await applyAlwaysOnTop(nextValue);
    await persistSettings({ alwaysOnTop: nextValue });
  }

  async function toggleAutoStart() {
    try {
      const autoStart = await applyAutoStart(!settingsRef.current.autoStart);
      await persistSettings({ autoStart });
      showToast(t.autoStartUpdated);
    } catch {
      showToast(t.autoStartFailed);
    }
  }

  async function toggleLanguage() {
    await persistSettings({
      language: settingsRef.current.language === "zh" ? "en" : "zh",
    });
  }

  async function minimizeWindow() {
    if (!isTauriRuntime()) {
      await collapseToQIcon();
      return;
    }

    await getCurrentWindow().minimize();
  }

  async function closeWindow() {
    if (!isTauriRuntime()) {
      await collapseToQIcon();
      return;
    }

    await getCurrentWindow().close();
  }

  async function quitApp() {
    if (!isTauriRuntime()) {
      return;
    }

    await invoke("quit_app");
  }

  function dragMainWindow(event: PointerEvent<HTMLElement>) {
    if (event.button !== 0) {
      return;
    }

    void startMainWindowDrag();
  }

  function openMenu(event: MouseEvent<HTMLElement>, noteId?: string) {
    event.preventDefault();
    event.stopPropagation();
    setMenu({
      noteId,
      x: event.clientX,
      y: event.clientY,
    });
  }

  async function openDockMenu(event: MouseEvent<HTMLButtonElement>) {
    event.preventDefault();
    event.stopPropagation();

    if (!isTauriRuntime()) {
      setMenu({
        x: event.clientX,
        y: event.clientY,
      });
      return;
    }

    const nativeMenu = await Menu.new({
      items: [
        {
          id: "topmost",
          text: alwaysOnLabel,
          action: () => void toggleAlwaysOnTop(),
        },
        {
          id: "toggle-language",
          text: t.switchLanguage,
          action: () => void toggleLanguage(),
        },
        {
          id: "toggle-dock",
          text: dockToggleLabel,
          action: () => void toggleDockOnEdge(),
        },
        {
          id: "quit",
          text: t.quit,
          action: () => void quitApp(),
        },
      ],
    });

    await nativeMenu.popup(undefined, getCurrentWindow());
  }

  function getContextItems(): ContextMenuItem[] {
    const note = menu?.noteId ? notesRef.current.find((item) => item.id === menu.noteId) : null;
    return createMainContextItems({
      alwaysOnLabel,
      dockToggleLabel,
      note: note ?? null,
      notesCount: notesRef.current.length,
      onCopyNote: (item) => void handleCopy(item),
      onDeleteAll: () => setShowDeleteAllConfirm(true),
      onDeleteNote: (id) => void handleDelete(id),
      onEditNote: (item) => void openEditor(item),
      onNewNote: () => void openEditor(null),
      onOpenSettings: () => setShowSettings(true),
      onToggleAlwaysOnTop: () => void toggleAlwaysOnTop(),
      onToggleAutoStart: () => void toggleAutoStart(),
      onToggleDock: () => void toggleDockOnEdge(),
      onToggleNotePin: (item) => void patchNote(item.id, { pinned: !item.pinned }),
      settings,
      t,
    });
  }

  function getDockMenuItems(): ContextMenuItem[] {
    return createDockMenuItems({
      alwaysOnLabel,
      dockToggleLabel,
      onQuit: () => void quitApp(),
      onToggleAlwaysOnTop: () => void toggleAlwaysOnTop(),
      onToggleDock: () => void toggleDockOnEdge(),
      onToggleLanguage: () => void toggleLanguage(),
      settings,
      t,
    });
  }

  useAppBoot({
    currentWindowLabel,
    notesRef,
    settingsRef,
    setNotes,
    setReady,
    setSettings,
  });

  useEffect(() => {
    notesRef.current = notes;
  }, [notes]);

  useEffect(() => {
    settingsRef.current = settings;
  }, [settings]);

  useTrayMenuLabels({
    alwaysOnLabel,
    dockToggleLabel,
    ready,
    t,
  });

  useTauriEventBridge({
    commitNotes,
    notesRef,
    ready,
    restoreDock,
    setSettings,
    settingsRef,
    showToast,
    toggleAlwaysOnTop,
    toggleDockOnEdge,
    toggleLanguage,
  });

  useWindowStatePersistence({
    currentWindowLabel,
    dockDragRef,
    dockGuardRef,
    editorOpen,
    persistIconSnap,
    persistSettings,
    ready,
    settingsRef,
  });

  if (!ready) {
    if (isDockWindow) {
      return (
        <main className="dock-shell">
          <QMark className="dock-loading-mark" />
        </main>
      );
    }

    return (
      <main className="app-shell is-loading">
        <QMark className="loading-mark" />
      </main>
    );
  }

  if (isDockWindow) {
    return (
      <>
        <CompactDock
          onContextMenu={(event) => void openDockMenu(event)}
          onDragEnd={() => void finishQIconDrag()}
          onDragMove={() => void moveQIcon()}
          onDragStart={() => void dragQIcon()}
          onHoverEnd={() => void concealDockIcon()}
          onHoverStart={() => void revealDockIcon()}
          onOpenMain={() => void openMainFromDockIcon()}
          t={t}
        />
        {menu ? (
          <ContextMenu
            items={getDockMenuItems()}
            onClose={() => setMenu(null)}
            x={menu.x}
            y={menu.y}
          />
        ) : null}
        <Toast icon={toast?.icon} message={toast?.message ?? null} />
      </>
    );
  }

  return (
    <main
      className="app-shell"
      onClick={() => setMenu(null)}
      onContextMenu={(event) => openMenu(event)}
    >
      <AppHeader
        alwaysOnLabel={alwaysOnLabel}
        alwaysOnTop={settings.alwaysOnTop}
        onClose={() => void closeWindow()}
        onDragStart={dragMainWindow}
        onMinimize={() => void minimizeWindow()}
        onToggleAlwaysOnTop={() => void toggleAlwaysOnTop()}
        t={t}
      />

      <AppToolbar
        hasUpdate={Boolean(updateInfo)}
        notesCount={notes.length}
        onDeleteAll={() => setShowDeleteAllConfirm(true)}
        onNewNote={() => void openEditor(null)}
        onOpenSettings={() => setShowSettings(true)}
        onToggleLanguage={() => void toggleLanguage()}
        t={t}
        updateVersion={updateInfo?.latestVersion}
      />

      <NoteList
        notes={notes}
        onColorChange={(id, color) => void patchNote(id, { color })}
        onContextMenu={openMenu}
        onCopy={(item) => void handleCopy(item)}
        onDelete={(id) => void handleDelete(id)}
        onEdit={(item) => void openEditor(item)}
        onHeightChange={(id, textHeight) => void patchNote(id, { textHeight })}
        onNewNote={() => void openEditor(null)}
        onReorder={(draggedId, targetId, placement) =>
          void reorderNotes(draggedId, targetId, placement)
        }
        onTogglePin={(id) => {
          const note = notesRef.current.find((item) => item.id === id);
          if (note) {
            void patchNote(id, { pinned: !note.pinned });
          }
        }}
        t={t}
      />

      {editorOpen ? (
        <NoteEditor
          note={editorNote}
          onCancel={() => void closeEditor()}
          onSave={(draft) => void handleSaveDraft(draft)}
          t={t}
        />
      ) : null}
      {showSettings ? (
        <SettingsDialog
          appVersion={appVersion}
          autoStart={settings.autoStart}
          checkingUpdate={checkingUpdate}
          hasUpdate={Boolean(updateInfo)}
          onCheckUpdate={() => void handleCheckUpdate()}
          onClose={() => setShowSettings(false)}
          onExport={() => void handleExport()}
          onImport={() => void handleImport()}
          onOpenCurrentRelease={() => void handleOpenCurrentRelease()}
          onToggleAutoStart={() => void toggleAutoStart()}
          t={t}
        />
      ) : null}
      {updateDialogOpen && updateInfo ? (
        <UpdateDownloadDialog
          onCancel={() => void handleCancelUpdateDownload()}
          onClose={() => setUpdateDialogOpen(false)}
          onReveal={(path) => void handleRevealDownloadedUpdate(path)}
          progress={updateDownloadProgress}
          result={updateDownloadResult}
          t={t}
          update={updateInfo}
        />
      ) : null}

      {menu ? (
        <ContextMenu
          items={getContextItems()}
          onClose={() => setMenu(null)}
          x={menu.x}
          y={menu.y}
        />
      ) : null}
      {showDeleteAllConfirm ? (
        <ConfirmDialog
          body={t.deleteAllBody}
          cancelLabel={t.cancel}
          confirmLabel={t.deleteAll}
          onCancel={() => setShowDeleteAllConfirm(false)}
          onConfirm={() => void handleDeleteAll()}
          title={t.confirmDeleteAll}
        />
      ) : null}
      <button
        aria-label={t.switchFloatingBall}
        className="panel-dock-button"
        onClick={(event) => {
          event.stopPropagation();
          void collapseToQIcon({ useRevealAnchor: true });
        }}
        title={t.switchFloatingBall}
        type="button"
      >
        <QMark />
      </button>
      <StatusBar notes={notes} t={t} />
      <Toast icon={toast?.icon} message={toast?.message ?? null} />
    </main>
  );
}

export default App;
