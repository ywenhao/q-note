import { CheckCircle2, Languages, Pin, Power, StickyNote } from "lucide-react";
import type { Translation } from "../i18n";
import type { AppSettings, Note } from "../types";

interface StatusBarProps {
  notes: Note[];
  settings: AppSettings;
  t: Translation;
}

export function StatusBar({ notes, settings, t }: StatusBarProps) {
  const pinnedCount = notes.filter((note) => note.pinned).length;

  return (
    <footer className="app-statusbar">
      <span>
        <CheckCircle2 size={14} />
        {t.statusReady}
      </span>
      <span>
        <StickyNote size={14} />
        {t.statusNotes(notes.length)}
      </span>
      <span>
        <Pin size={14} />
        {t.statusPinned(pinnedCount)}
      </span>
      <span className={settings.alwaysOnTop ? "is-active" : ""}>
        <Pin size={14} />
        {settings.alwaysOnTop ? t.statusTopmostOn : t.statusTopmostOff}
      </span>
      <span className={settings.autoStart ? "is-active" : ""}>
        <Power size={14} />
        {settings.autoStart ? t.statusAutoStartOn : t.statusAutoStartOff}
      </span>
      <span>
        <Languages size={14} />
        {settings.language === "zh" ? "中文" : "English"}
      </span>
    </footer>
  );
}
