import { convertFileSrc } from "@tauri-apps/api/core";
import type { NoteAttachment } from "../types";
import { isTauriRuntime } from "./env";

const IMAGE_EXTENSIONS = new Set(["avif", "bmp", "gif", "jpeg", "jpg", "png", "svg", "webp"]);

export function getAttachmentSrc(attachment: NoteAttachment) {
  if (attachment.source === "path" && isTauriRuntime()) {
    return convertFileSrc(attachment.value);
  }

  return attachment.value;
}

export function isImageAttachment(attachment: Pick<NoteAttachment, "kind" | "source" | "value">) {
  if (attachment.kind === "file") {
    return false;
  }

  return (
    attachment.kind === "image" ||
    /^data:image\//i.test(attachment.value) ||
    isLikelyImagePath(attachment.value)
  );
}

export function isLikelyImagePath(value: string) {
  const cleanValue = value.split(/[?#]/)[0] ?? value;
  const extension = cleanValue.split(".").pop()?.toLowerCase();

  return Boolean(extension && IMAGE_EXTENSIONS.has(extension));
}

function readUrls(value: string, onlyImages = true) {
  return value
    .split(/\r?\n/)
    .map((item) => item.trim())
    .filter((item) => /^https?:\/\//i.test(item) && (!onlyImages || isLikelyImagePath(item)));
}

function readImageUrlsFromHtml(value: string) {
  const parser = new DOMParser();
  const document = parser.parseFromString(value, "text/html");
  const urls = Array.from(document.querySelectorAll("img"))
    .map((image) => image.currentSrc || image.src)
    .filter((src) => /^https?:\/\//i.test(src));

  return Array.from(new Set(urls));
}

export function resolveDraggedImageUrls(dataTransfer: DataTransfer) {
  const uriList = dataTransfer.getData("text/uri-list");
  const plainText = dataTransfer.getData("text/plain");
  const html = dataTransfer.getData("text/html");
  const urls = [...readUrls(uriList), ...readUrls(plainText), ...readImageUrlsFromHtml(html)];

  return Array.from(new Set(urls));
}

export function resolveDraggedFileUrls(dataTransfer: DataTransfer) {
  const uriList = dataTransfer.getData("text/uri-list");
  const plainText = dataTransfer.getData("text/plain");

  return Array.from(new Set([...readUrls(uriList, false), ...readUrls(plainText, false)]));
}

export function readFileAsDataUrl(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onerror = () => reject(reader.error);
    reader.onload = () => resolve(String(reader.result));
    reader.readAsDataURL(file);
  });
}
