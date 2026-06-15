import type { Translation } from "../i18n";
import type { Note } from "../types";

interface StatusBarProps {
  notes: Note[];
  t: Translation;
}

export function StatusBar({ notes, t }: StatusBarProps) {
  return <footer className="app-statusbar">{t.statusSummary(notes.length)}</footer>;
}
