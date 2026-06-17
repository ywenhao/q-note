import { Languages, Plus, Settings, Trash2 } from "lucide-react";
import type { Translation } from "../i18n";
import { IconButton } from "./IconButton";

interface AppToolbarProps {
  hasUpdate: boolean;
  notesCount: number;
  onDeleteAll: () => void;
  onNewNote: () => void;
  onOpenSettings: () => void;
  onToggleLanguage: () => void;
  t: Translation;
  updateVersion?: string;
}

export function AppToolbar({
  hasUpdate,
  notesCount,
  onDeleteAll,
  onNewNote,
  onOpenSettings,
  onToggleLanguage,
  t,
  updateVersion,
}: AppToolbarProps) {
  const settingsLabel =
    hasUpdate && updateVersion ? t.updateAvailableTitle(updateVersion) : t.settings;

  return (
    <div className="toolbar">
      <IconButton icon={<Plus size={18} />} label={t.newNote} onClick={onNewNote} />
      <IconButton
        className="is-danger"
        disabled={notesCount === 0}
        icon={<Trash2 size={18} />}
        label={t.deleteAll}
        onClick={onDeleteAll}
      />
      <IconButton
        badge={hasUpdate}
        icon={<Settings size={18} />}
        label={settingsLabel}
        onClick={onOpenSettings}
      />
      <IconButton icon={<Languages size={18} />} label={t.language} onClick={onToggleLanguage}>
        {t.language}
      </IconButton>
    </div>
  );
}
