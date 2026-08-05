import {
  FileInputIcon,
  LanguagesIcon,
  PanelRightCloseIcon,
  PencilIcon,
  PinIcon,
  PinOffIcon,
  PlusIcon,
  PowerIcon,
  Trash2Icon,
} from "../icons";
import type { ContextMenuItem } from "../components/componentTypes";
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

interface MainMenuOptions {
  note: Note | null;
  notesCount: number;
  onCopyNote: (note: Note) => void;
  onDeleteAll: () => void;
  onDeleteNote: (id: string) => void;
  onEditNote: (note: Note) => void;
  onNewNote: () => void;
  onToggleNotePin: (note: Note) => void;
  settings: AppSettings;
  t: Translation;
}

interface DockMenuOptions extends CommonMenuOptions {
  onQuit: () => void;
  onToggleLanguage: () => void;
}

export function createMainContextItems(options: MainMenuOptions): ContextMenuItem[] {
  const { note, notesCount, settings, t } = options;
  if (note) {
    return [
      {
        id: "copy",
        icon: FileInputIcon,
        label: t.copy,
        onSelect: () => options.onCopyNote(note),
      },
      {
        id: "edit",
        icon: PencilIcon,
        label: t.edit,
        onSelect: () => options.onEditNote(note),
      },
      {
        id: "pin",
        icon: note.pinned ? PinOffIcon : PinIcon,
        label: note.pinned ? t.unpin : t.pin,
        onSelect: () => options.onToggleNotePin(note),
      },
      {
        destructive: true,
        id: "delete",
        icon: Trash2Icon,
        label: t.delete,
        onSelect: () => options.onDeleteNote(note.id),
      },
    ];
  }

  if (settings.docked) {
    return [];
  }

  const items: ContextMenuItem[] = [
    { id: "new", icon: PlusIcon, label: t.newNote, onSelect: options.onNewNote },
  ];
  if (notesCount > 0) {
    items.push({
      destructive: true,
      id: "delete-all",
      icon: Trash2Icon,
      label: t.deleteAll,
      onSelect: options.onDeleteAll,
    });
  }
  return items;
}

export function createDockMenuItems(options: DockMenuOptions): ContextMenuItem[] {
  const { alwaysOnLabel, dockToggleLabel, settings, t } = options;
  return [
    {
      id: "topmost",
      icon: settings.alwaysOnTop ? PinOffIcon : PinIcon,
      label: alwaysOnLabel,
      onSelect: options.onToggleAlwaysOnTop,
    },
    {
      id: "toggle-language",
      icon: LanguagesIcon,
      label: t.switchLanguage,
      onSelect: options.onToggleLanguage,
    },
    {
      id: "toggle-dock",
      icon: PanelRightCloseIcon,
      label: dockToggleLabel,
      onSelect: options.onToggleDock,
    },
    {
      destructive: true,
      id: "quit",
      icon: PowerIcon,
      label: t.quit,
      onSelect: options.onQuit,
    },
  ];
}
