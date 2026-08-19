import {
  FileInputIcon,
  LanguagesIcon,
  PanelRightCloseIcon,
  PencilIcon,
  PinIcon,
  PinOffIcon,
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
  onCopyNote: (note: Note) => void;
  onDeleteNote: (id: string) => void;
  onEditNote: (note: Note) => void;
  onToggleNotePin: (note: Note) => void;
  t: Translation;
}

interface DockMenuOptions extends CommonMenuOptions {
  onQuit: () => void;
  onToggleLanguage: () => void;
}

export function createMainContextItems(options: MainMenuOptions): ContextMenuItem[] {
  const { note, t } = options;
  if (!note) {
    return [];
  }

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
