import { Menu } from "@tauri-apps/api/menu";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { computed, type ComputedRef, type Ref } from "vue";
import { useContextMenu, type MenuState } from "../../hooks/useContextMenu";
import type { Translation } from "../../i18n";
import { isTauriRuntime } from "../../lib/env";
import { createDockMenuItems, createMainContextItems } from "../../lib/menuItems";
import type { AppSettings, Note } from "../../types";

export type { MenuState };

interface UseMenuControllerOptions {
  alwaysOnLabel: ComputedRef<string>;
  dockToggleLabel: ComputedRef<string>;
  handleCopy: (note: Note) => Promise<void>;
  handleDelete: (id: string) => Promise<void>;
  notes: Ref<Note[]>;
  onDeleteAll: () => void;
  openEditor: (note: Note | null) => Promise<void>;
  patchNote: (id: string, patch: Partial<Note>) => Promise<void>;
  quitApp: () => Promise<void>;
  settings: Ref<AppSettings>;
  t: ComputedRef<Translation>;
  toggleAlwaysOnTop: () => Promise<void>;
  toggleDockOnEdge: () => Promise<void>;
  toggleLanguage: () => Promise<void>;
}

export function useMenuController(options: UseMenuControllerOptions) {
  const { closeMenu, menu, openMenu } = useContextMenu();

  async function openDockMenu(event: MouseEvent) {
    event.preventDefault();
    event.stopPropagation();
    if (!isTauriRuntime()) {
      openMenu(event);
      return;
    }

    const nativeMenu = await Menu.new({
      items: [
        {
          id: "topmost",
          text: options.alwaysOnLabel.value,
          action: () => void options.toggleAlwaysOnTop(),
        },
        {
          id: "toggle-language",
          text: options.t.value.switchLanguage,
          action: () => void options.toggleLanguage(),
        },
        {
          id: "toggle-dock",
          text: options.dockToggleLabel.value,
          action: () => void options.toggleDockOnEdge(),
        },
        {
          id: "quit",
          text: options.t.value.quit,
          action: () => void options.quitApp(),
        },
      ],
    });
    await nativeMenu.popup(undefined, getCurrentWindow());
  }

  const contextItems = computed(() => {
    const note = menu.value?.noteId
      ? (options.notes.value.find((item) => item.id === menu.value?.noteId) ?? null)
      : null;
    return createMainContextItems({
      note,
      notesCount: options.notes.value.length,
      onCopyNote: (item) => void options.handleCopy(item),
      onDeleteAll: options.onDeleteAll,
      onDeleteNote: (id) => void options.handleDelete(id),
      onEditNote: (item) => void options.openEditor(item),
      onNewNote: () => void options.openEditor(null),
      onToggleNotePin: (item) => void options.patchNote(item.id, { pinned: !item.pinned }),
      settings: options.settings.value,
      t: options.t.value,
    });
  });

  const dockMenuItems = computed(() =>
    createDockMenuItems({
      alwaysOnLabel: options.alwaysOnLabel.value,
      dockToggleLabel: options.dockToggleLabel.value,
      onQuit: () => void options.quitApp(),
      onToggleAlwaysOnTop: () => void options.toggleAlwaysOnTop(),
      onToggleDock: () => void options.toggleDockOnEdge(),
      onToggleLanguage: () => void options.toggleLanguage(),
      settings: options.settings.value,
      t: options.t.value,
    }),
  );

  return { closeMenu, contextItems, dockMenuItems, menu, openDockMenu, openMenu };
}
