import {
  DndContext,
  DragOverlay,
  PointerSensor,
  closestCenter,
  useSensor,
  useSensors,
  type DragEndEvent,
  type DragStartEvent,
} from "@dnd-kit/core";
import { SortableContext, useSortable, verticalListSortingStrategy } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import {
  useMemo,
  useState,
  type CSSProperties,
  type HTMLAttributes,
  type MouseEvent as ReactMouseEvent,
} from "react";
import type { Translation } from "../i18n";
import type { Note } from "../types";
import { EmptyState } from "./EmptyState";
import type { ImagePreviewItem } from "./ImagePreview";
import { NoteCard } from "./NoteCard";

type DropPlacement = "before" | "after";

interface NoteListProps {
  notes: Note[];
  onColorChange: (id: string, color: string) => void;
  onContextMenu: (event: ReactMouseEvent<HTMLElement>, noteId: string) => void;
  onCopy: (note: Note) => void;
  onDelete: (id: string) => void;
  onEdit: (note: Note) => void;
  onHeightChange: (id: string, textHeight: number) => void;
  onNewNote: () => void;
  onPreviewImages: (items: ImagePreviewItem[], index: number) => void;
  onReorder: (draggedId: string, targetId: string, placement: DropPlacement) => void;
  onTogglePin: (id: string) => void;
  t: Translation;
}

interface SortableNoteCardProps {
  activeId: string | null;
  note: Note;
  onColorChange: (id: string, color: string) => void;
  onContextMenu: (event: ReactMouseEvent<HTMLElement>, noteId: string) => void;
  onCopy: (note: Note) => void;
  onDelete: (id: string) => void;
  onEdit: (note: Note) => void;
  onHeightChange: (id: string, textHeight: number) => void;
  onPreviewImages: (items: ImagePreviewItem[], index: number) => void;
  onTogglePin: (id: string) => void;
  shouldSuppressCopy: () => boolean;
  t: Translation;
}

function SortableNoteCard({
  activeId,
  note,
  onColorChange,
  onContextMenu,
  onCopy,
  onDelete,
  onEdit,
  onHeightChange,
  onPreviewImages,
  onTogglePin,
  shouldSuppressCopy,
  t,
}: SortableNoteCardProps) {
  const { attributes, isDragging, listeners, setNodeRef, transform, transition } = useSortable({
    id: note.id,
  });

  const style: CSSProperties = {
    transform: CSS.Transform.toString(transform),
    transition: isDragging ? undefined : transition,
  };

  return (
    <NoteCard
      dragHandleProps={{ ...attributes, ...listeners } as HTMLAttributes<HTMLElement>}
      dragging={isDragging || activeId === note.id}
      key={note.id}
      note={note}
      onColorChange={onColorChange}
      onContextMenu={onContextMenu}
      onCopy={onCopy}
      onDelete={onDelete}
      onEdit={onEdit}
      onHeightChange={onHeightChange}
      onPreviewImages={onPreviewImages}
      onTogglePin={onTogglePin}
      rootRef={setNodeRef}
      shouldSuppressCopy={shouldSuppressCopy}
      sortableStyle={style}
      t={t}
    />
  );
}

const noop = () => undefined;

export function NoteList({
  notes,
  onColorChange,
  onContextMenu,
  onCopy,
  onDelete,
  onEdit,
  onHeightChange,
  onNewNote,
  onPreviewImages,
  onReorder,
  onTogglePin,
  t,
}: NoteListProps) {
  const [activeId, setActiveId] = useState<string | null>(null);
  const [suppressCopyUntil, setSuppressCopyUntil] = useState(0);
  const noteIds = useMemo(() => notes.map((note) => note.id), [notes]);
  const activeNote = activeId ? (notes.find((note) => note.id === activeId) ?? null) : null;
  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: {
        distance: 6,
      },
    }),
  );

  function shouldSuppressCopy() {
    return Boolean(activeId) || Date.now() < suppressCopyUntil;
  }

  function handleDragStart(event: DragStartEvent) {
    setActiveId(String(event.active.id));
    setSuppressCopyUntil(Date.now() + 800);
    document.body.classList.add("is-sorting-note");
  }

  function clearDragState() {
    setActiveId(null);
    setSuppressCopyUntil(Date.now() + 400);
    document.body.classList.remove("is-sorting-note");
  }

  function handleDragEnd(event: DragEndEvent) {
    const { active, over } = event;

    if (!over || active.id === over.id) {
      clearDragState();
      return;
    }

    const activeIndex = noteIds.indexOf(String(active.id));
    const overIndex = noteIds.indexOf(String(over.id));
    if (activeIndex < 0 || overIndex < 0) {
      clearDragState();
      return;
    }

    onReorder(String(active.id), String(over.id), activeIndex < overIndex ? "after" : "before");
    clearDragState();
  }

  if (notes.length === 0) {
    return <EmptyState onNewNote={onNewNote} t={t} />;
  }

  return (
    <DndContext
      collisionDetection={closestCenter}
      onDragCancel={clearDragState}
      onDragEnd={handleDragEnd}
      onDragStart={handleDragStart}
      sensors={sensors}
    >
      <SortableContext items={noteIds} strategy={verticalListSortingStrategy}>
        <section className="note-list">
          {notes.map((note) => (
            <SortableNoteCard
              activeId={activeId}
              key={note.id}
              note={note}
              onColorChange={onColorChange}
              onContextMenu={onContextMenu}
              onCopy={onCopy}
              onDelete={onDelete}
              onEdit={onEdit}
              onHeightChange={onHeightChange}
              onPreviewImages={onPreviewImages}
              onTogglePin={onTogglePin}
              shouldSuppressCopy={shouldSuppressCopy}
              t={t}
            />
          ))}
        </section>
      </SortableContext>
      <DragOverlay dropAnimation={null}>
        {activeNote ? (
          <NoteCard
            dragOverlay
            note={activeNote}
            onColorChange={noop}
            onContextMenu={(event) => event.preventDefault()}
            onCopy={noop}
            onDelete={noop}
            onEdit={noop}
            onHeightChange={noop}
            onPreviewImages={noop}
            onTogglePin={noop}
            shouldSuppressCopy={() => true}
            t={t}
          />
        ) : null}
      </DragOverlay>
    </DndContext>
  );
}
