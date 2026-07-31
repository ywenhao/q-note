import { DEFAULT_NOTE_COLOR } from "../types.ts";
import type {
  AttachmentKind,
  AttachmentSource,
  Note,
  NoteAttachment,
  NoteDraft,
} from "../types.ts";

export interface PendingUpdateDraft {
  draft: NoteDraft;
  noteId: string | null;
  savedAt: number;
}

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function normalizeAttachmentKind(value: unknown): AttachmentKind | null {
  return value === "image" || value === "file" ? value : null;
}

function normalizeAttachmentSource(value: unknown): AttachmentSource | null {
  return value === "data" || value === "url" || value === "path" ? value : null;
}

function normalizeAttachment(value: unknown): NoteAttachment | null {
  if (!isObject(value)) {
    return null;
  }

  const kind = normalizeAttachmentKind(value.kind);
  const source = normalizeAttachmentSource(value.source);
  const createdAt = Number(value.createdAt);
  if (
    typeof value.id !== "string" ||
    !kind ||
    !source ||
    typeof value.value !== "string" ||
    (value.name !== undefined && typeof value.name !== "string") ||
    !Number.isFinite(createdAt)
  ) {
    return null;
  }

  return {
    id: value.id,
    kind,
    source,
    value: value.value,
    name: value.name,
    createdAt,
  };
}

export function createNoteDraft(note: Note | null): NoteDraft {
  return {
    attachments: note?.attachments.map((attachment) => ({ ...attachment })) ?? [],
    color: note?.color ?? DEFAULT_NOTE_COLOR,
    content: note?.content ?? "",
    pinned: note?.pinned ?? false,
  };
}

export function isEditorDraftDirty(draft: NoteDraft, note: Note | null) {
  return JSON.stringify(draft) !== JSON.stringify(createNoteDraft(note));
}

export function normalizePendingUpdateDraft(value: unknown): PendingUpdateDraft | null {
  if (!isObject(value) || !isObject(value.draft)) {
    return null;
  }

  const { draft } = value;
  const savedAt = Number(value.savedAt);
  if (
    (value.noteId !== null && typeof value.noteId !== "string") ||
    !Array.isArray(draft.attachments) ||
    typeof draft.color !== "string" ||
    typeof draft.content !== "string" ||
    typeof draft.pinned !== "boolean" ||
    !Number.isFinite(savedAt)
  ) {
    return null;
  }

  const attachments = draft.attachments.map(normalizeAttachment);
  if (attachments.some((attachment) => attachment === null)) {
    return null;
  }

  return {
    draft: {
      attachments: attachments as NoteAttachment[],
      color: draft.color,
      content: draft.content,
      pinned: draft.pinned,
    },
    noteId: value.noteId,
    savedAt,
  };
}

export function serializePendingUpdateDraft(value: PendingUpdateDraft) {
  return JSON.stringify(value);
}

export function parsePendingUpdateDraft(value: string) {
  try {
    return normalizePendingUpdateDraft(JSON.parse(value));
  } catch {
    return null;
  }
}
