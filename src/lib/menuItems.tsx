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
import type { ContextMenuItem } from "../components/ContextMenu";
import type { Translation } from "../i18n";
import type { AppSettings, Note } from "../types";

interface CommonMenuOptions {
  alwaysOnLabel: string;
  dockToggleLabel: string;
  onToggleAlwaysOnTop: () => void;
  onToggleDock: () => void;
  settings: AppSettings;
  t: Translation;
}

interface MainMenuOptions extends CommonMenuOptions {
  note: Note | null;
  notesCount: number;
  onCopyNote: (note: Note) => void;
  onDeleteAll: () => void;
  onDeleteNote: (id: string) => void;
  onEditNote: (note: Note) => void;
  onNewNote: () => void;
  onOpenSettings: () => void;
  onToggleAutoStart: () => void;
  onToggleNotePin: (note: Note) => void;
}

interface DockMenuOptions extends CommonMenuOptions {
  onQuit: () => void;
  onToggleLanguage: () => void;
}

export function createMainContextItems({
  alwaysOnLabel,
  dockToggleLabel,
  note,
  notesCount,
  onCopyNote,
  onDeleteAll,
  onDeleteNote,
  onEditNote,
  onNewNote,
  onOpenSettings,
  onToggleAlwaysOnTop,
  onToggleAutoStart,
  onToggleDock,
  onToggleNotePin,
  settings,
  t,
}: MainMenuOptions): ContextMenuItem[] {
  const items: ContextMenuItem[] = [];

  if (note) {
    items.push(
      {
        id: "copy",
        icon: <FileInput size={16} />,
        label: t.copy,
        onSelect: () => onCopyNote(note),
      },
      {
        id: "edit",
        icon: <Pencil size={16} />,
        label: t.edit,
        onSelect: () => onEditNote(note),
      },
      {
        id: "pin",
        icon: note.pinned ? <PinOff size={16} /> : <Pin size={16} />,
        label: note.pinned ? t.unpin : t.pin,
        onSelect: () => onToggleNotePin(note),
      },
      {
        destructive: true,
        id: "delete",
        icon: <Trash2 size={16} />,
        label: t.delete,
        onSelect: () => onDeleteNote(note.id),
      },
    );
  } else if (!settings.docked) {
    items.push({
      id: "new",
      icon: <Plus size={16} />,
      label: t.newNote,
      onSelect: onNewNote,
    });

    if (notesCount > 0) {
      items.push({
        destructive: true,
        id: "delete-all",
        icon: <Trash2 size={16} />,
        label: t.deleteAll,
        onSelect: onDeleteAll,
      });
    }
  }

  items.push(
    {
      id: "settings",
      icon: <Settings size={16} />,
      label: t.settings,
      onSelect: onOpenSettings,
    },
    {
      id: "topmost",
      icon: settings.alwaysOnTop ? <PinOff size={16} /> : <Pin size={16} />,
      label: alwaysOnLabel,
      onSelect: onToggleAlwaysOnTop,
    },
    {
      id: "autostart",
      icon: <Power size={16} />,
      label: settings.autoStart ? t.autoStartOff : t.autoStartOn,
      onSelect: onToggleAutoStart,
    },
    {
      id: "dock-edge",
      icon: <PanelRightClose size={16} />,
      label: dockToggleLabel,
      onSelect: onToggleDock,
    },
  );

  return items;
}

export function createDockMenuItems({
  alwaysOnLabel,
  dockToggleLabel,
  onQuit,
  onToggleAlwaysOnTop,
  onToggleDock,
  onToggleLanguage,
  settings,
  t,
}: DockMenuOptions): ContextMenuItem[] {
  return [
    {
      id: "topmost",
      icon: settings.alwaysOnTop ? <PinOff size={16} /> : <Pin size={16} />,
      label: alwaysOnLabel,
      onSelect: onToggleAlwaysOnTop,
    },
    {
      id: "toggle-language",
      icon: <Languages size={16} />,
      label: t.switchLanguage,
      onSelect: onToggleLanguage,
    },
    {
      id: "toggle-dock",
      icon: <PanelRightClose size={16} />,
      label: dockToggleLabel,
      onSelect: onToggleDock,
    },
    {
      destructive: true,
      id: "quit",
      icon: <Power size={16} />,
      label: t.quit,
      onSelect: onQuit,
    },
  ];
}
