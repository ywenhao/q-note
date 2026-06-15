import type { MouseEvent as ReactMouseEvent } from "react";
import type { Translation } from "../i18n";
import type { Note } from "../types";
import { EmptyState } from "./EmptyState";
import { NoteCard } from "./NoteCard";

interface NoteListProps {
  notes: Note[];
  onColorChange: (id: string, color: string) => void;
  onContextMenu: (event: ReactMouseEvent<HTMLElement>, noteId: string) => void;
  onCopy: (note: Note) => void;
  onDelete: (id: string) => void;
  onEdit: (note: Note) => void;
  onHeightChange: (id: string, textHeight: number) => void;
  onNewNote: () => void;
  onTogglePin: (id: string) => void;
  t: Translation;
}

export function NoteList({
  notes,
  onColorChange,
  onContextMenu,
  onCopy,
  onDelete,
  onEdit,
  onHeightChange,
  onNewNote,
  onTogglePin,
  t,
}: NoteListProps) {
  if (notes.length === 0) {
    return <EmptyState onNewNote={onNewNote} t={t} />;
  }

  return (
    <section className="note-list">
      {notes.map((note) => (
        <NoteCard
          key={note.id}
          note={note}
          onColorChange={onColorChange}
          onContextMenu={onContextMenu}
          onCopy={onCopy}
          onDelete={onDelete}
          onEdit={onEdit}
          onHeightChange={onHeightChange}
          onTogglePin={onTogglePin}
          t={t}
        />
      ))}
    </section>
  );
}
