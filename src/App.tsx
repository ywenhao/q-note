import {
  Download,
  FileInput,
  Languages,
  Minus,
  PanelRightClose,
  Pencil,
  Pin,
  PinOff,
  Plus,
  Power,
  Settings,
  Trash2,
  Upload,
  X,
} from "lucide-react";
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
import { ConfirmDialog } from "./components/ConfirmDialog";
import { CompactDock } from "./components/CompactDock";
import { ContextMenu, type ContextMenuItem } from "./components/ContextMenu";
import { IconButton } from "./components/IconButton";
import { NoteCard } from "./components/NoteCard";
import { NoteEditor, type NoteDraft } from "./components/NoteEditor";
import { QMark } from "./components/QMark";
import { StatusBar } from "./components/StatusBar";
import { Toast } from "./components/Toast";
import { translations } from "./i18n";
import { applyAutoStart, readAutoStartEnabled } from "./lib/autoStart";
import { createId, isTauriRuntime } from "./lib/env";
import { exportJson, importJson } from "./lib/fileIo";
import {
  createDefaultSettings,
  createExportPayload,
  deleteAllNotes,
  deleteNote,
  loadAppData,
  normalizeImportPayload,
  replaceAppData,
  saveNote,
  saveSettings,
} from "./lib/storage";
import {
  applyAlwaysOnTop,
  applyQIconWindow,
  captureWindowState,
  detectSnapEdge,
  ensureEditorRoom,
  restoreWindowState,
  restoreWindowFromQIcon,
  snapQIconWindow,
  startMainWindowDrag,
  startQIconDrag,
} from "./lib/windowControls";
import type { AppSettings, DockEdge, Note, WindowState } from "./types";

interface MenuState {
  noteId?: string;
  x: number;
  y: number;
}

function sortNotes(notes: Note[]) {
  return [...notes].sort((a, b) => {
    if (a.pinned !== b.pinned) {
      return Number(b.pinned) - Number(a.pinned);
    }

    return b.updatedAt - a.updatedAt;
  });
}

async function writeClipboard(value: string) {
  if (navigator.clipboard?.writeText) {
    await navigator.clipboard.writeText(value);
    return;
  }

  const textArea = document.createElement("textarea");
  textArea.value = value;
  textArea.style.position = "fixed";
  textArea.style.opacity = "0";
  document.body.append(textArea);
  textArea.select();
  document.execCommand("copy");
  textArea.remove();
}

function App() {
  const [editorNote, setEditorNote] = useState<Note | null | undefined>(undefined);
  const [menu, setMenu] = useState<MenuState | null>(null);
  const [notes, setNotes] = useState<Note[]>([]);
  const [ready, setReady] = useState(false);
  const [isCollapsing, setIsCollapsing] = useState(false);
  const [settings, setSettings] = useState<AppSettings>(() => createDefaultSettings());
  const [showDeleteAllConfirm, setShowDeleteAllConfirm] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [toast, setToast] = useState<string | null>(null);

  const dockGuardRef = useRef(false);
  const editorWindowRef = useRef<WindowState | null>(null);
  const hoverExpandedRef = useRef(false);
  const notesRef = useRef<Note[]>([]);
  const settingsRef = useRef(settings);
  const toastTimerRef = useRef<number | null>(null);

  const t = translations[settings.language];
  const editorOpen = editorNote !== undefined;

  const commitNotes = useCallback((nextNotes: Note[]) => {
    const sorted = sortNotes(nextNotes);
    notesRef.current = sorted;
    setNotes(sorted);
  }, []);

  const showToast = useCallback((message: string) => {
    setToast(message);
    if (toastTimerRef.current) {
      window.clearTimeout(toastTimerRef.current);
    }
    toastTimerRef.current = window.setTimeout(() => setToast(null), 1700);
  }, []);

  const persistSettings = useCallback(async (patch: Partial<AppSettings>) => {
    const nextSettings = { ...settingsRef.current, ...patch };
    settingsRef.current = nextSettings;
    setSettings(nextSettings);
    await saveSettings(nextSettings);
  }, []);

  const setDockGuard = useCallback(() => {
    dockGuardRef.current = true;
    window.setTimeout(() => {
      dockGuardRef.current = false;
    }, 700);
  }, []);

  const persistIconSnap = useCallback(
    async (edge: DockEdge) => {
      setDockGuard();
      const iconWindow = await snapQIconWindow(edge);
      await persistSettings({
        docked: true,
        dockEdge: edge,
        iconWindow: iconWindow ?? settingsRef.current.iconWindow,
      });
    },
    [persistSettings, setDockGuard],
  );

  async function restoreDock(options: { keepFull?: boolean } = {}) {
    setDockGuard();
    await restoreWindowState(settingsRef.current.window);
    await applyAlwaysOnTop(settingsRef.current.alwaysOnTop);
    await persistSettings({
      docked: false,
      dockEdge: null,
      keepFullMain: options.keepFull ?? settingsRef.current.keepFullMain,
    });
  }

  async function collapseToQIcon() {
    const snapshot = await captureWindowState();
    setDockGuard();
    await applyQIconWindow(settingsRef.current.iconWindow);
    await persistSettings({
      docked: true,
      dockEdge: null,
      keepFullMain: false,
      window: snapshot ?? settingsRef.current.window,
    });
    hoverExpandedRef.current = false;
  }

  async function collapseToQIconWithAnimation() {
    if (isCollapsing) {
      return;
    }

    setIsCollapsing(true);
    await new Promise((resolve) => window.setTimeout(resolve, 170));
    await collapseToQIcon();
    setIsCollapsing(false);
  }

  async function hoverRestoreFromQIcon() {
    hoverExpandedRef.current = true;
    setDockGuard();
    await restoreWindowFromQIcon(settingsRef.current.window, settingsRef.current.iconWindow);
    await applyAlwaysOnTop(settingsRef.current.alwaysOnTop);
    await persistSettings({
      docked: false,
      dockEdge: null,
      keepFullMain: settingsRef.current.keepFullMain,
    });
  }

  async function returnToQIcon() {
    setDockGuard();
    const snapshot = await captureWindowState();
    await applyQIconWindow(settingsRef.current.iconWindow);
    await persistSettings({
      docked: true,
      dockEdge: settingsRef.current.dockEdge,
      keepFullMain: false,
      window: snapshot ?? settingsRef.current.window,
    });
    hoverExpandedRef.current = false;
  }

  async function handleMainMouseLeave() {
    if (!hoverExpandedRef.current || settingsRef.current.keepFullMain || editorOpen || menu) {
      return;
    }

    await returnToQIcon();
  }

  async function openEditor(note: Note | null) {
    if (settingsRef.current.docked) {
      await restoreDock({ keepFull: true });
    }

    editorWindowRef.current = await ensureEditorRoom();
    setEditorNote(note);
  }

  async function closeEditor() {
    setEditorNote(undefined);
    const previousWindow = editorWindowRef.current;
    editorWindowRef.current = null;

    if (previousWindow && !settingsRef.current.docked) {
      await restoreWindowState(previousWindow);
    }
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
          updatedAt: now,
        }
      : {
          id: createId("note"),
          attachments: draft.attachments,
          color: draft.color,
          content: draft.content,
          pinned: draft.pinned,
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

    const nextNote = { ...target, ...patch };
    commitNotes([nextNote, ...notesRef.current.filter((note) => note.id !== id)]);
    await saveNote(nextNote);
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

  async function toggleDockOnEdge() {
    if (settingsRef.current.docked) {
      await restoreDock({ keepFull: true });
      return;
    }

    await collapseToQIcon();
  }

  async function toggleKeepFullMain() {
    if (settingsRef.current.docked) {
      await restoreDock({ keepFull: true });
      hoverExpandedRef.current = false;
      return;
    }

    const nextValue = !settingsRef.current.keepFullMain;
    await persistSettings({ keepFullMain: nextValue });
    hoverExpandedRef.current = nextValue;
    if (!nextValue) {
      await returnToQIcon();
    }
  }

  async function dragQIcon() {
    try {
      await startQIconDrag();
    } finally {
      window.setTimeout(() => {
        void Promise.all([detectSnapEdge(), captureWindowState()]).then(([edge, snapshot]) => {
          if (!settingsRef.current.docked || !snapshot) {
            return;
          }

          if (edge) {
            void persistIconSnap(edge);
            return;
          }

          void persistSettings({ dockEdge: null, iconWindow: snapshot });
        });
      }, 240);
    }
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

  function getContextItems(): ContextMenuItem[] {
    const note = menu?.noteId ? notesRef.current.find((item) => item.id === menu.noteId) : null;
    const items: ContextMenuItem[] = [];

    if (note) {
      items.push(
        {
          id: "copy",
          icon: <FileInput size={16} />,
          label: t.copy,
          onSelect: () => void handleCopy(note),
        },
        {
          id: "edit",
          icon: <Pencil size={16} />,
          label: t.edit,
          onSelect: () => void openEditor(note),
        },
        {
          id: "pin",
          icon: note.pinned ? <PinOff size={16} /> : <Pin size={16} />,
          label: note.pinned ? t.unpin : t.pin,
          onSelect: () => void patchNote(note.id, { pinned: !note.pinned }),
        },
        {
          destructive: true,
          id: "delete",
          icon: <Trash2 size={16} />,
          label: t.delete,
          onSelect: () => void handleDelete(note.id),
        },
      );
    } else if (!settings.docked) {
      items.push({
        id: "new",
        icon: <Plus size={16} />,
        label: t.newNote,
        onSelect: () => void openEditor(null),
      });
      if (notesRef.current.length > 0) {
        items.push({
          destructive: true,
          id: "delete-all",
          icon: <Trash2 size={16} />,
          label: t.deleteAll,
          onSelect: () => setShowDeleteAllConfirm(true),
        });
      }
    }

    items.push(
      {
        id: "settings",
        icon: <Settings size={16} />,
        label: t.settings,
        onSelect: () => setShowSettings(true),
      },
      {
        id: "topmost",
        icon: settings.alwaysOnTop ? <PinOff size={16} /> : <Pin size={16} />,
        label: settings.alwaysOnTop ? t.alwaysOff : t.alwaysOn,
        onSelect: () => void toggleAlwaysOnTop(),
      },
      {
        id: "autostart",
        icon: <Power size={16} />,
        label: settings.autoStart ? t.autoStartOff : t.autoStartOn,
        onSelect: () => void toggleAutoStart(),
      },
      {
        id: "dock-edge",
        icon: <PanelRightClose size={16} />,
        label: settings.docked ? t.keepFull : t.dockOn,
        onSelect: () => void toggleDockOnEdge(),
      },
      {
        id: "keep-full",
        icon: <PanelRightClose size={16} />,
        label: settings.keepFullMain ? t.keepCompact : t.keepFull,
        onSelect: () => void toggleKeepFullMain(),
      },
    );

    return items;
  }

  useEffect(() => {
    let cancelled = false;

    async function boot() {
      const data = await loadAppData();
      if (cancelled) {
        return;
      }

      notesRef.current = sortNotes(data.notes);
      settingsRef.current = data.settings;
      setNotes(notesRef.current);
      setSettings(data.settings);
      await applyAlwaysOnTop(data.settings.alwaysOnTop);
      const autoStart = await readAutoStartEnabled();
      if (autoStart !== data.settings.autoStart) {
        settingsRef.current = { ...settingsRef.current, autoStart };
        setSettings(settingsRef.current);
        await saveSettings(settingsRef.current);
      }

      if (data.settings.docked) {
        await applyQIconWindow(data.settings.iconWindow);
      }

      setReady(true);
    }

    void boot();

    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    notesRef.current = notes;
  }, [notes]);

  useEffect(() => {
    settingsRef.current = settings;
  }, [settings]);

  useEffect(() => {
    if (!ready || !isTauriRuntime()) {
      return;
    }

    let saveTimer: number | null = null;
    let unlistenMove: (() => void) | null = null;
    let unlistenResize: (() => void) | null = null;

    const saveWindowSoon = () => {
      if (settingsRef.current.docked || dockGuardRef.current) {
        return;
      }

      if (saveTimer) {
        window.clearTimeout(saveTimer);
      }

      saveTimer = window.setTimeout(() => {
        void captureWindowState().then((snapshot) => {
          if (snapshot && !settingsRef.current.docked) {
            void persistSettings({ window: snapshot });
          }
        });
      }, 300);
    };

    const handleMoved = () => {
      if (dockGuardRef.current || editorOpen) {
        saveWindowSoon();
        return;
      }

      window.setTimeout(() => {
        void Promise.all([detectSnapEdge(), captureWindowState()]).then(([edge, snapshot]) => {
          if (!snapshot) {
            return;
          }

          if (settingsRef.current.docked) {
            if (edge) {
              void persistIconSnap(edge);
              return;
            }

            void persistSettings({ dockEdge: null, iconWindow: snapshot });
            return;
          }

          void persistSettings({ window: snapshot });
        });
      }, 160);
    };

    void (async () => {
      const currentWindow = getCurrentWindow();
      unlistenMove = await currentWindow.onMoved(handleMoved);
      unlistenResize = await currentWindow.onResized(saveWindowSoon);
    })();

    return () => {
      if (saveTimer) {
        window.clearTimeout(saveTimer);
      }
      unlistenMove?.();
      unlistenResize?.();
    };
  }, [editorOpen, persistIconSnap, persistSettings, ready]);

  if (!ready) {
    return (
      <main className="app-shell is-loading">
        <QMark className="loading-mark" />
      </main>
    );
  }

  if (settings.docked) {
    return (
      <>
        <CompactDock
          onContextMenu={(event) => openMenu(event)}
          onDragStart={() => void dragQIcon()}
          onHover={() => void hoverRestoreFromQIcon()}
          t={t}
        />
        {menu ? (
          <ContextMenu
            items={getContextItems()}
            onClose={() => setMenu(null)}
            x={menu.x}
            y={menu.y}
          />
        ) : null}
        <Toast message={toast} />
      </>
    );
  }

  return (
    <main
      className={`app-shell ${isCollapsing ? "is-collapsing" : ""}`}
      onClick={() => setMenu(null)}
      onContextMenu={(event) => openMenu(event)}
      onMouseLeave={() => void handleMainMouseLeave()}
    >
      <header className="top-bar" onPointerDown={dragMainWindow}>
        <div className="brand">
          <QMark className="brand-mark" />
          <h1>{t.appTitle}</h1>
        </div>
        <div className="title-controls" onPointerDown={(event) => event.stopPropagation()}>
          <IconButton
            active={settings.alwaysOnTop}
            className="is-window-pin"
            icon={settings.alwaysOnTop ? <PinOff size={16} /> : <Pin size={16} />}
            label={settings.alwaysOnTop ? t.alwaysOff : t.alwaysOn}
            onClick={() => void toggleAlwaysOnTop()}
            subtle
          />
          <IconButton
            className="is-window-minimize"
            icon={<Minus size={16} />}
            label={t.minimize}
            onClick={() => void minimizeWindow()}
            subtle
          />
          <IconButton
            className="is-window-close"
            icon={<X size={16} />}
            label={t.closePanel}
            onClick={() => void closeWindow()}
            subtle
          />
        </div>
      </header>

      <div className="toolbar">
        <IconButton
          icon={<Plus size={18} />}
          label={t.newNote}
          onClick={() => void openEditor(null)}
        />
        <IconButton
          icon={<Upload size={18} />}
          label={t.import}
          onClick={() => void handleImport()}
        />
        <IconButton
          icon={<Download size={18} />}
          label={t.export}
          onClick={() => void handleExport()}
        />
        <IconButton
          className="is-danger"
          disabled={notes.length === 0}
          icon={<Trash2 size={18} />}
          label={t.deleteAll}
          onClick={() => setShowDeleteAllConfirm(true)}
        />
        <IconButton
          icon={<Settings size={18} />}
          label={t.settings}
          onClick={() => setShowSettings(true)}
        />
        <IconButton
          icon={<Languages size={18} />}
          label={t.language}
          onClick={() => void toggleLanguage()}
        >
          {t.language}
        </IconButton>
      </div>

      {notes.length > 0 ? (
        <section className="note-list">
          {notes.map((note) => (
            <NoteCard
              key={note.id}
              note={note}
              onColorChange={(id, color) => void patchNote(id, { color })}
              onContextMenu={openMenu}
              onCopy={(item) => void handleCopy(item)}
              onDelete={(id) => void handleDelete(id)}
              onEdit={(item) => void openEditor(item)}
              onHeightChange={(id, textHeight) => void patchNote(id, { textHeight })}
              onTogglePin={(id) => {
                const note = notesRef.current.find((item) => item.id === id);
                if (note) {
                  void patchNote(id, { pinned: !note.pinned });
                }
              }}
              t={t}
            />
          ))}
        </section>
      ) : (
        <section className="empty-state">
          <QMark className="empty-mark" />
          <h2>{t.emptyTitle}</h2>
          <p>{t.noNotesBody}</p>
          <button className="primary-button" onClick={() => void openEditor(null)} type="button">
            {t.emptyAction}
          </button>
        </section>
      )}

      {editorOpen ? (
        <NoteEditor
          note={editorNote}
          onCancel={() => void closeEditor()}
          onSave={(draft) => void handleSaveDraft(draft)}
          t={t}
        />
      ) : null}
      {showSettings ? (
        <div className="modal-backdrop" onMouseDown={() => setShowSettings(false)}>
          <section
            aria-modal="true"
            className="settings-dialog"
            onMouseDown={(event) => event.stopPropagation()}
            role="dialog"
          >
            <header>
              <h2>{t.settingsTitle}</h2>
              <IconButton
                icon={<X size={18} />}
                label={t.cancel}
                onClick={() => setShowSettings(false)}
                subtle
              />
            </header>
            <button className="settings-row" onClick={() => void toggleAutoStart()} type="button">
              <span>
                <Power size={18} />
                {t.startupSetting}
              </span>
              <span className={`switch ${settings.autoStart ? "is-on" : ""}`} aria-hidden="true">
                <span />
              </span>
            </button>
          </section>
        </div>
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
        aria-label={t.dockOn}
        className="panel-dock-button"
        onClick={(event) => {
          event.stopPropagation();
          void collapseToQIconWithAnimation();
        }}
        title={t.dockOn}
        type="button"
      >
        <QMark />
      </button>
      <StatusBar notes={notes} t={t} />
      <Toast message={toast} />
    </main>
  );
}

export default App;
