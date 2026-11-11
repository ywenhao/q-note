import { Buffer } from "buffer";
import { Files } from "q-note/native";
import type { NoteAttachment, AttachmentSource } from "../types";
import { inferAttachmentKind } from "./appData";

export { isLikelyImagePath } from "./imagePath";

export function attachmentFromValue(
  raw: string,
  name?: string,
): NoteAttachment {
  const value = raw.trim();
  const source: AttachmentSource = /^data:/i.test(value)
    ? "data"
    : /^https?:\/\//i.test(value)
      ? "url"
      : "path";
  return {
    id: crypto.randomUUID(),
    kind: inferAttachmentKind(source, value),
    source,
    value,
    name: name ?? value.split(/[\\/]/).pop()?.split(/[?#]/)[0],
    createdAt: Date.now(),
  };
}

export async function readFileAsDataUrl(file: File) {
  return `data:${file.type || "application/octet-stream"};base64,${Buffer.from(await file.arrayBuffer()).toString("base64")}`;
}

export async function getAttachmentSrc(attachment: NoteAttachment) {
  if (attachment.source !== "path") return attachment.value;
  return Files.image(attachment.value);
}

export function pathFromUri(value: string) {
  if (!value.startsWith("file:")) return value;
  let path = decodeURIComponent(new URL(value).pathname);
  if (/^\/[a-z]:/i.test(path)) path = path.slice(1);
  return path;
}
