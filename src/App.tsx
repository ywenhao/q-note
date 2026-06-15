import {
  FileInput,
  Languages,
  PanelRightClose,
  Pencil,
  Pin,
  PinOff,
  Plus,
  Power,
  Settings,
  Trash2,
} from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { emit, listen } from "@tauri-apps/api/event";
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
import { EditorWindow } from "./components/EditorWindow";
import { NoteEditor, type NoteDraft } from "./components/NoteEditor";
import { NoteList } from "./components/NoteList";
import { QMark } from "./components/QMark";
import { SettingsDialog } from "./components/SettingsDialog";
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
  DOCK_WINDOW_LABEL,
  EDITOR_WINDOW_LABEL,
  MAIN_WINDOW_LABEL,
  applyAlwaysOnTop,
  captureWindowState,
  detectSnapEdge,
  hideDockWindow,
  hideMainWindow,
  openEditorWindow,
  revealQIconWindow,
  showDockWindow,
  showMainWindow,
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

type DockTransitionTarget = "dock" | "main";

// A cross-window token prevents stale dock transitions from hiding the wrong window.
const DOCK_TRANSITION_KEY = "q-note:dock-transition";

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

interface MainAppProps {
  currentWindowLabel: string;
}

function MainApp({ currentWindowLabel }: MainAppProps) {
  const [editorNote, setEditorNote] = useState<Note | null | undefined>(undefined);
  const [menu, setMenu] = useState<MenuState | null>(null);
  const [notes, setNotes] = useState<Note[]>([]);
  const [ready, setReady] = useState(false);
  const [settings, setSettings] = useState<AppSettings>(() => createDefaultSettings());
  const [showDeleteAllConfirm, setShowDeleteAllConfirm] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [toast, setToast] = useState<string | null>(null);

  const dockGuardRef = useRef(false);
  const iconWindowRef = useRef<WindowState | null>(null);
  const notesRef = useRef<Note[]>([]);
  const settingsRef = useRef(settings);
  const toastTimerRef = useRef<number | null>(null);
  const dockGuardTimerRef = useRef<number | null>(null);
  const dockTransitionRef = useRef(false);

  const isDockWindow = currentWindowLabel === DOCK_WINDOW_LABEL;
  const t = translations[settings.language];
  const editorOpen = editorNote !== undefined;
  const dockToggleLabel = settings.docked ? t.switchMainWindow : t.switchFloatingBall;
  const alwaysOnLabel = settings.alwaysOnTop ? t.alwaysOff : t.alwaysOn;

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

    if (isTauriRuntime()) {
      await emit("q-note-settings-updated", nextSettings);
    }
  }, []);

  const setDockGuard = useCallback(() => {
    dockGuardRef.current = true;
    if (dockGuardTimerRef.current) {
      window.clearTimeout(dockGuardTimerRef.current);
    }

    dockGuardTimerRef.current = window.setTimeout(() => {
      dockGuardRef.current = false;
      dockGuardTimerRef.current = null;
    }, 700);
  }, []);

  const persistIconSnap = useCallback(
    async (edge: DockEdge) => {
      setDockGuard();
      iconWindowRef.current = await snapQIconWindow(edge);
      await persistSettings({
        docked: true,
        dockEdge: edge,
      });
    },
    [persistSettings, setDockGuard],
  );

  function beginDockTransition(target: DockTransitionTarget) {
    const token = `${target}:${Date.now()}:${Math.random().toString(36).slice(2)}`;
    localStorage.setItem(DOCK_TRANSITION_KEY, token);
    return token;
  }

  function getActiveDockTransitionTarget(): DockTransitionTarget | null {
    const target = localStorage.getItem(DOCK_TRANSITION_KEY)?.split(":")[0];
    return target === "dock" || target === "main" ? target : null;
  }

  function isActiveDockTransition(token: string) {
    return localStorage.getItem(DOCK_TRANSITION_KEY) === token;
  }

  async function restoreDock(options: { keepFull?: boolean } = {}) {
    if (dockTransitionRef.current) {
      return;
    }

    dockTransitionRef.current = true;
    const token = beginDockTransition("main");

    try {
      setDockGuard();
      await persistSettings({
        docked: false,
        dockEdge: null,
        keepFullMain: options.keepFull ?? settingsRef.current.keepFullMain,
      });
      await showMainWindow(settingsRef.current.window, settingsRef.current.alwaysOnTop);

      if (isActiveDockTransition(token) && !settingsRef.current.docked) {
        await hideDockWindow();
      } else if (getActiveDockTransitionTarget() === "dock") {
        await hideMainWindow();
      }
    } finally {
      dockTransitionRef.current = false;
    }
  }

  async function collapseToQIcon() {
    if (dockTransitionRef.current) {
      return;
    }

    dockTransitionRef.current = true;
    const token = beginDockTransition("dock");

    try {
      const snapshot = await captureWindowState(MAIN_WINDOW_LABEL);
      setDockGuard();
      await persistSettings({
        docked: true,
        dockEdge: null,
        keepFullMain: false,
        window: snapshot ?? settingsRef.current.window,
      });
      iconWindowRef.current = await showDockWindow(
        snapshot ?? settingsRef.current.window,
        settingsRef.current.alwaysOnTop,
      );

      if (isActiveDockTransition(token) && settingsRef.current.docked) {
        await hideMainWindow();
      } else if (getActiveDockTransitionTarget() === "main") {
        await hideDockWindow();
      }
    } finally {
      dockTransitionRef.current = false;
    }
  }

  async function openEditor(note: Note | null) {
    if (settingsRef.current.docked) {
      await restoreDock({ keepFull: true });
    }

    if (isTauriRuntime()) {
      await openEditorWindow(note?.id ?? null, settingsRef.current.alwaysOnTop);
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

  async function quitApp() {
    if (!isTauriRuntime()) {
      return;
    }

    await invoke("quit_app");
  }

  async function updateTrayMenuLabels() {
    if (!isTauriRuntime()) {
      return;
    }

    await invoke("set_tray_menu_labels", {
      topmost: alwaysOnLabel,
      quit: t.quit,
      toggleDock: dockToggleLabel,
      toggleLanguage: t.switchLanguage,
    });
  }

  async function toggleDockOnEdge() {
    if (settingsRef.current.docked) {
      await restoreDock({ keepFull: true });
      return;
    }

    await collapseToQIcon();
  }

  async function dragQIcon() {
    await startQIconDrag();
  }

  async function revealDockIcon() {
    const edge = settingsRef.current.dockEdge;
    if (!edge) {
      return;
    }

    setDockGuard();
    iconWindowRef.current = await revealQIconWindow(edge);
  }

  async function concealDockIcon() {
    const edge = settingsRef.current.dockEdge;
    if (!edge) {
      return;
    }

    setDockGuard();
    iconWindowRef.current = await snapQIconWindow(edge);
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
        label: alwaysOnLabel,
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
        label: dockToggleLabel,
        onSelect: () => void toggleDockOnEdge(),
      },
    );

    return items;
  }

  function getDockMenuItems(): ContextMenuItem[] {
    return [
      {
        id: "topmost",
        icon: settings.alwaysOnTop ? <PinOff size={16} /> : <Pin size={16} />,
        label: alwaysOnLabel,
        onSelect: () => void toggleAlwaysOnTop(),
      },
      {
        id: "toggle-language",
        icon: <Languages size={16} />,
        label: t.switchLanguage,
        onSelect: () => void toggleLanguage(),
      },
      {
        id: "toggle-dock",
        icon: <PanelRightClose size={16} />,
        label: dockToggleLabel,
        onSelect: () => void toggleDockOnEdge(),
      },
      {
        destructive: true,
        id: "quit",
        icon: <Power size={16} />,
        label: t.quit,
        onSelect: () => void quitApp(),
      },
    ];
  }

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
        await hideDockWindow();
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
    if (!ready) {
      return;
    }

    void updateTrayMenuLabels();
  }, [alwaysOnLabel, dockToggleLabel, ready, t.quit, t.switchLanguage]);

  useEffect(() => {
    if (!ready || !isTauriRuntime()) {
      return;
    }

    let disposed = false;
    let unlistenHandlers: Array<() => void> = [];

    void (async () => {
      const handlers = await Promise.all([
        listen("q-note-toggle-always-on-top", () => {
          void toggleAlwaysOnTop();
        }),
        listen("q-note-toggle-language", () => {
          void toggleLanguage();
        }),
        listen("q-note-toggle-dock", () => {
          void toggleDockOnEdge();
        }),
        listen("q-note-show-main", () => {
          if (settingsRef.current.docked) {
            void restoreDock({ keepFull: true });
          }
        }),
        listen<Note>("q-note-note-saved", (event) => {
          commitNotes([
            event.payload,
            ...notesRef.current.filter((note) => note.id !== event.payload.id),
          ]);
          showToast(translations[settingsRef.current.language].saved);
        }),
        listen<AppSettings>("q-note-settings-updated", (event) => {
          settingsRef.current = event.payload;
          setSettings(event.payload);
        }),
      ]);

      if (disposed) {
        handlers.forEach((unlisten) => unlisten());
        return;
      }

      unlistenHandlers = handlers;
    })();

    return () => {
      disposed = true;
      unlistenHandlers.forEach((unlisten) => unlisten());
    };
  }, [ready]);

  useEffect(() => {
    if (!ready || !isTauriRuntime()) {
      return;
    }

    let saveTimer: number | null = null;
    let moveTimer: number | null = null;
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

      if (moveTimer) {
        window.clearTimeout(moveTimer);
      }

      moveTimer = window.setTimeout(() => {
        void Promise.all([detectSnapEdge(), captureWindowState()]).then(([edge, snapshot]) => {
          if (!snapshot) {
            return;
          }

          if (settingsRef.current.docked) {
            if (edge) {
              void persistIconSnap(edge);
              return;
            }

            iconWindowRef.current = snapshot;
            void persistSettings({ dockEdge: null });
            return;
          }

          void persistSettings({ window: snapshot });
        });
      }, 220);
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
      if (moveTimer) {
        window.clearTimeout(moveTimer);
      }
      unlistenMove?.();
      unlistenResize?.();
    };
  }, [editorOpen, persistIconSnap, persistSettings, ready]);

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
          onDragStart={() => void dragQIcon()}
          onHoverEnd={() => void concealDockIcon()}
          onHoverStart={() => void revealDockIcon()}
          onOpenMain={() => void restoreDock({ keepFull: true })}
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
        <Toast message={toast} />
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
        notesCount={notes.length}
        onDeleteAll={() => setShowDeleteAllConfirm(true)}
        onNewNote={() => void openEditor(null)}
        onOpenSettings={() => setShowSettings(true)}
        onToggleLanguage={() => void toggleLanguage()}
        t={t}
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
          autoStart={settings.autoStart}
          onClose={() => setShowSettings(false)}
          onExport={() => void handleExport()}
          onImport={() => void handleImport()}
          onToggleAutoStart={() => void toggleAutoStart()}
          t={t}
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
          void collapseToQIcon();
        }}
        title={t.switchFloatingBall}
        type="button"
      >
        <QMark />
      </button>
      <StatusBar notes={notes} t={t} />
      <Toast message={toast} />
    </main>
  );
}

function App() {
  const currentWindowLabel = isTauriRuntime() ? getCurrentWindow().label : MAIN_WINDOW_LABEL;

  if (currentWindowLabel === EDITOR_WINDOW_LABEL) {
    return <EditorWindow />;
  }

  return <MainApp currentWindowLabel={currentWindowLabel} />;
}

export default App;
