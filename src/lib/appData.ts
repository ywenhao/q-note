import {
  DEFAULT_NOTE_COLOR,
  type AppData,
  type AttachmentKind,
  type AttachmentSource,
  type ExportPayload,
  type Note,
  type NoteAttachment,
} from "../types.ts";
import { isLikelyImagePath } from "./imagePath.ts";
import { normalizeSettings } from "./settingsState.ts";

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

export function inferAttachmentKind(source: AttachmentSource, value: string): AttachmentKind {
  if (source === "data") {
    return /^data:image\//i.test(value) ? "image" : "file";
  }

  return isLikelyImagePath(value) ? "image" : "file";
}

function normalizeAttachment(value: unknown): NoteAttachment | null {
  if (!isObject(value) || typeof value.value !== "string") {
    return null;
  }

  const source =
    value.source === "url" || value.source === "path" || value.source === "data"
      ? value.source
      : "url";
  const kind =
    value.kind === "file" || value.kind === "image"
      ? value.kind
      : inferAttachmentKind(source, value.value);

  return {
    id: typeof value.id === "string" ? value.id : crypto.randomUUID(),
    kind,
    source,
    value: value.value,
    name: typeof value.name === "string" ? value.name : undefined,
    createdAt: Number(value.createdAt) || Date.now(),
  };
}

function normalizeNote(value: unknown): Note | null {
  if (!isObject(value)) {
    return null;
  }

  const updatedAt = Number(value.updatedAt) || Date.now();
  const attachments = Array.isArray(value.attachments)
    ? value.attachments
        .map(normalizeAttachment)
        .filter((item): item is NoteAttachment => Boolean(item))
    : [];

  return {
    id: typeof value.id === "string" ? value.id : crypto.randomUUID(),
    content: typeof value.content === "string" ? value.content : "",
    color: typeof value.color === "string" ? value.color : DEFAULT_NOTE_COLOR,
    pinned: Boolean(value.pinned),
    sortOrder: Number.isFinite(Number(value.sortOrder)) ? Number(value.sortOrder) : -updatedAt,
    textHeight: typeof value.textHeight === "number" ? value.textHeight : null,
    attachments,
    createdAt: Number(value.createdAt) || Date.now(),
    updatedAt,
  };
}

export function normalizeImportPayload(value: unknown): AppData {
  if (!isObject(value)) {
    throw new Error("Invalid Q Note data");
  }

  const notes = Array.isArray(value.notes)
    ? value.notes.map(normalizeNote).filter((item): item is Note => Boolean(item))
    : [];

  return {
    notes,
    settings: normalizeSettings(value.settings),
  };
}

export function createExportPayload(data: AppData): ExportPayload {
  return {
    version: 1,
    exportedAt: new Date().toISOString(),
    notes: data.notes,
    settings: data.settings,
  };
}
