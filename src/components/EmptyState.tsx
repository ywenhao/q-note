import type { Translation } from "../i18n";
import { QMark } from "./QMark";

interface EmptyStateProps {
  onNewNote: () => void;
  t: Translation;
}

export function EmptyState({ onNewNote, t }: EmptyStateProps) {
  return (
    <section className="empty-state">
      <QMark className="empty-mark" />
      <h2>{t.emptyTitle}</h2>
      <p>{t.noNotesBody}</p>
      <button className="primary-button" onClick={onNewNote} type="button">
        {t.emptyAction}
      </button>
    </section>
  );
}
