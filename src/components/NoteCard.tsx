import {
  Check,
  Copy,
  FileText,
  GripHorizontal,
  Palette,
  Pencil,
  Pin,
  PinOff,
  Trash2,
} from "lucide-react";
import {
  useRef,
  useState,
  type CSSProperties,
  type MouseEvent as ReactMouseEvent,
  type PointerEvent,
} from "react";
import type { Translation } from "../i18n";
import { getAttachmentSrc, isImageAttachment } from "../lib/images";
import { NOTE_COLORS, type Note } from "../types";
import { IconButton } from "./IconButton";

const LINE_HEIGHT = 22;

type NoteTextStyle = CSSProperties & {
  "--note-lines": number;
};

interface NoteCardProps {
  note: Note;
  onColorChange: (id: string, color: string) => void;
  onContextMenu: (event: ReactMouseEvent<HTMLElement>, noteId: string) => void;
  onCopy: (note: Note) => void;
  onDelete: (id: string) => void;
  onEdit: (note: Note) => void;
  onHeightChange: (id: string, height: number) => void;
  onTogglePin: (id: string) => void;
  t: Translation;
}

function getDefaultLines(content: string) {
  if (!content.trim()) {
    return 1;
  }

  return content.length <= 34 && !content.includes("\n") ? 1 : 2;
}

function stopCardClick(event: ReactMouseEvent) {
  event.stopPropagation();
}

export function NoteCard({
  note,
  onColorChange,
  onContextMenu,
  onCopy,
  onDelete,
  onEdit,
  onHeightChange,
  onTogglePin,
  t,
}: NoteCardProps) {
  const textRef = useRef<HTMLParagraphElement>(null);
  const [draftHeight, setDraftHeight] = useState<number | null>(null);
  const [paletteOpen, setPaletteOpen] = useState(false);

  const defaultHeight = getDefaultLines(note.content) * LINE_HEIGHT;
  const textHeight = draftHeight ?? note.textHeight ?? defaultHeight;
  const textLines = Math.max(1, Math.round(textHeight / LINE_HEIGHT));
  const hasText = note.content.trim().length > 0;
  const imageAttachments = note.attachments.filter(isImageAttachment);
  const fileAttachments = note.attachments.filter((attachment) => !isImageAttachment(attachment));

  function beginResize(event: PointerEvent<HTMLButtonElement>) {
    event.preventDefault();
    event.stopPropagation();

    const textElement = textRef.current;
    if (!textElement) {
      return;
    }

    const maxHeight = Math.max(
      defaultHeight,
      Math.ceil(textElement.scrollHeight / LINE_HEIGHT) * LINE_HEIGHT,
    );
    const minHeight = Math.min(defaultHeight, maxHeight);
    const startY = event.clientY;
    const startHeight = textHeight;

    const handleMove = (moveEvent: globalThis.PointerEvent) => {
      const nextHeight = clamp(startHeight + moveEvent.clientY - startY, minHeight, maxHeight);
      setDraftHeight(nextHeight);
    };

    const handleUp = (upEvent: globalThis.PointerEvent) => {
      const nextHeight = clamp(startHeight + upEvent.clientY - startY, minHeight, maxHeight);
      const snappedHeight = clamp(
        Math.ceil(nextHeight / LINE_HEIGHT) * LINE_HEIGHT,
        minHeight,
        maxHeight,
      );
      setDraftHeight(null);
      onHeightChange(note.id, snappedHeight);
      window.removeEventListener("pointermove", handleMove);
    };

    window.addEventListener("pointermove", handleMove);
    window.addEventListener("pointerup", handleUp, { once: true });
  }

  return (
    <article
      className={`note-card ${note.pinned ? "is-pinned" : ""}`}
      onClick={() => onCopy(note)}
      onContextMenu={(event) => onContextMenu(event, note.id)}
      style={{ backgroundColor: note.color }}
    >
      {note.pinned ? (
        <span aria-label={t.pinned} className="note-card__pin-badge" role="img">
          <Pin size={12} />
        </span>
      ) : null}

      <div className="note-card__body">
        <div className="note-card__content">
          <p
            className={`note-card__text ${hasText ? "" : "is-muted"}`}
            ref={textRef}
            style={{ "--note-lines": textLines } as NoteTextStyle}
          >
            {hasText ? note.content : t.imageOnly}
          </p>

          {imageAttachments.length > 0 ? (
            <div className="note-card__images" onClick={stopCardClick}>
              {imageAttachments.slice(0, 4).map((attachment) => (
                <img
                  alt={attachment.name ?? t.addImage}
                  key={attachment.id}
                  src={getAttachmentSrc(attachment)}
                />
              ))}
            </div>
          ) : null}

          {fileAttachments.length > 0 ? (
            <div className="note-card__files" onClick={stopCardClick}>
              {fileAttachments.slice(0, 3).map((attachment) => (
                <span key={attachment.id} title={attachment.value}>
                  <FileText size={14} />
                  {attachment.name ?? attachment.value}
                </span>
              ))}
            </div>
          ) : null}
        </div>

        <div className="note-card__actions" onClick={stopCardClick}>
          <IconButton
            active={note.pinned}
            icon={note.pinned ? <PinOff size={16} /> : <Pin size={16} />}
            label={note.pinned ? t.unpin : t.pin}
            onClick={() => onTogglePin(note.id)}
            subtle
          />
          <IconButton
            icon={<Pencil size={16} />}
            label={t.edit}
            onClick={() => onEdit(note)}
            subtle
          />
          <div className="color-popover-wrap">
            <IconButton
              icon={<Palette size={16} />}
              label={t.color}
              onClick={() => setPaletteOpen((open) => !open)}
              subtle
            />
            {paletteOpen ? (
              <div className="color-popover">
                {NOTE_COLORS.map((color) => (
                  <button
                    aria-label={color}
                    className="color-swatch"
                    key={color}
                    onClick={() => {
                      onColorChange(note.id, color);
                      setPaletteOpen(false);
                    }}
                    style={{ backgroundColor: color }}
                    type="button"
                  >
                    {note.color === color ? <Check size={13} /> : null}
                  </button>
                ))}
              </div>
            ) : null}
          </div>
          <IconButton
            icon={<Copy size={16} />}
            label={t.copy}
            onClick={() => onCopy(note)}
            subtle
          />
          <IconButton
            className="is-danger"
            icon={<Trash2 size={16} />}
            label={t.delete}
            onClick={() => onDelete(note.id)}
            subtle
          />
        </div>
      </div>

      <button
        aria-label={t.resize}
        className="resize-handle"
        onClick={stopCardClick}
        onPointerDown={beginResize}
        title={t.resize}
        type="button"
      >
        <GripHorizontal size={16} />
      </button>
    </article>
  );
}

function clamp(value: number, min: number, max: number) {
  return Math.max(min, Math.min(value, max));
}
