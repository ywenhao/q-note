import { Menu } from "@tauri-apps/api/menu";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useCallback, useState, type MouseEvent, type MutableRefObject } from "react";
import type { ContextMenuItem } from "../../components/ContextMenu";
import type { Translation } from "../../i18n";
import { isTauriRuntime } from "../../lib/env";
import { createDockMenuItems, createMainContextItems } from "../../lib/menuItems";
import type { AppSettings, Note } from "../../types";

export interface MenuState {
  noteId?: string;
  x: number;
  y: number;
}

interface UseMenuControllerOptions {
  alwaysOnLabel: string;
  dockToggleLabel: string;
  handleCopy: (note: Note) => Promise<void>;
  handleDelete: (id: string) => Promise<void>;
  notesCount: number;
  notesRef: MutableRefObject<Note[]>;
  onDeleteAll: () => void;
  onOpenSettings: () => void;
  openEditor: (note: Note | null) => Promise<void>;
  patchNote: (id: string, patch: Partial<Note>) => Promise<void>;
  quitApp: () => Promise<void>;
  settings: AppSettings;
  t: Translation;
  toggleAlwaysOnTop: () => Promise<void>;
  toggleAutoStart: () => Promise<void>;
  toggleDockOnEdge: () => Promise<void>;
  toggleLanguage: () => Promise<void>;
}

export function useMenuController({
  alwaysOnLabel,
  dockToggleLabel,
  handleCopy,
  handleDelete,
  notesCount,
  notesRef,
  onDeleteAll,
  onOpenSettings,
  openEditor,
  patchNote,
  quitApp,
  settings,
  t,
  toggleAlwaysOnTop,
  toggleAutoStart,
  toggleDockOnEdge,
  toggleLanguage,
}: UseMenuControllerOptions) {
  const [menu, setMenu] = useState<MenuState | null>(null);

  const closeMenu = useCallback(() => {
    setMenu(null);
  }, []);

  const openMenu = useCallback((event: MouseEvent<HTMLElement>, noteId?: string) => {
    event.preventDefault();
    event.stopPropagation();
    setMenu({
      noteId,
      x: event.clientX,
      y: event.clientY,
    });
  }, []);

  const openDockMenu = useCallback(
    async (event: MouseEvent<HTMLButtonElement>) => {
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
    },
    [
      alwaysOnLabel,
      dockToggleLabel,
      quitApp,
      t,
      toggleAlwaysOnTop,
      toggleDockOnEdge,
      toggleLanguage,
    ],
  );

  const getContextItems = useCallback((): ContextMenuItem[] => {
    const note = menu?.noteId ? notesRef.current.find((item) => item.id === menu.noteId) : null;
    return createMainContextItems({
      alwaysOnLabel,
      dockToggleLabel,
      note: note ?? null,
      notesCount,
      onCopyNote: (item) => void handleCopy(item),
      onDeleteAll,
      onDeleteNote: (id) => void handleDelete(id),
      onEditNote: (item) => void openEditor(item),
      onNewNote: () => void openEditor(null),
      onOpenSettings,
      onToggleAlwaysOnTop: () => void toggleAlwaysOnTop(),
      onToggleAutoStart: () => void toggleAutoStart(),
      onToggleDock: () => void toggleDockOnEdge(),
      onToggleNotePin: (item) => void patchNote(item.id, { pinned: !item.pinned }),
      settings,
      t,
    });
  }, [
    alwaysOnLabel,
    dockToggleLabel,
    handleCopy,
    handleDelete,
    menu?.noteId,
    notesCount,
    notesRef,
    onDeleteAll,
    onOpenSettings,
    openEditor,
    patchNote,
    settings,
    t,
    toggleAlwaysOnTop,
    toggleAutoStart,
    toggleDockOnEdge,
  ]);

  const getDockMenuItems = useCallback(
    (): ContextMenuItem[] =>
      createDockMenuItems({
        alwaysOnLabel,
        dockToggleLabel,
        onQuit: () => void quitApp(),
        onToggleAlwaysOnTop: () => void toggleAlwaysOnTop(),
        onToggleDock: () => void toggleDockOnEdge(),
        onToggleLanguage: () => void toggleLanguage(),
        settings,
        t,
      }),
    [
      alwaysOnLabel,
      dockToggleLabel,
      quitApp,
      settings,
      t,
      toggleAlwaysOnTop,
      toggleDockOnEdge,
      toggleLanguage,
    ],
  );

  return {
    closeMenu,
    getContextItems,
    getDockMenuItems,
    menu,
    openDockMenu,
    openMenu,
  };
}
