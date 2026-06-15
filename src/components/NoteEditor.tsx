import { getCurrentWindow } from "@tauri-apps/api/window";
import { FileText, Folder, ImagePlus, Link2, Trash2, X } from "lucide-react";
import {
  useEffect,
  useRef,
  useState,
  type ClipboardEvent,
  type DragEvent,
  type PointerEvent,
} from "react";
import type { Translation } from "../i18n";
import { createId, isTauriRuntime } from "../lib/env";
import {
  getAttachmentSrc,
  isLikelyImagePath,
  isImageAttachment,
  readFileAsDataUrl,
  resolveDraggedFileUrls,
  resolveDraggedImageUrls,
} from "../lib/images";
import { DEFAULT_NOTE_COLOR, NOTE_COLORS, type Note, type NoteAttachment } from "../types";
import { IconButton } from "./IconButton";

export interface NoteDraft {
  attachments: NoteAttachment[];
  color: string;
  content: string;
  pinned: boolean;
}

interface NoteEditorProps {
  mode?: "modal" | "window";
  note: Note | null;
  onCancel: () => void;
  onDragStart?: (event: PointerEvent<HTMLElement>) => void;
  onSave: (draft: NoteDraft) => void;
  t: Translation;
}

export function NoteEditor({
  mode = "modal",
  note,
  onCancel,
  onDragStart,
  onSave,
  t,
}: NoteEditorProps) {
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [attachments, setAttachments] = useState<NoteAttachment[]>([]);
  const [color, setColor] = useState<string>(DEFAULT_NOTE_COLOR);
  const [content, setContent] = useState("");
  const [dragging, setDragging] = useState(false);
  const [mediaValue, setMediaValue] = useState("");
  const [previewAttachment, setPreviewAttachment] = useState<NoteAttachment | null>(null);
  const [pinned, setPinned] = useState(false);
  const isWindowMode = mode === "window";

  useEffect(() => {
    setAttachments(note?.attachments ?? []);
    setColor(note?.color ?? DEFAULT_NOTE_COLOR);
    setContent(note?.content ?? "");
    setMediaValue("");
    setPinned(note?.pinned ?? false);
    setPreviewAttachment(null);
  }, [note]);

  function createAttachment(
    value: string,
    source: NoteAttachment["source"],
    kind: NoteAttachment["kind"],
    nameFallback: string,
  ): NoteAttachment {
    return {
      id: createId("asset"),
      kind,
      source,
      value,
      name: value.split(/[\\/]/).pop() || nameFallback,
      createdAt: Date.now(),
    };
  }

  useEffect(() => {
    if (!isTauriRuntime()) {
      return;
    }

    let disposed = false;
    let unlisten: (() => void) | null = null;

    void getCurrentWindow()
      .onDragDropEvent((event) => {
        if (event.payload.type !== "drop") {
          return;
        }

        setDragging(false);
        appendPathAttachments(event.payload.paths);
      })
      .then((cleanup) => {
        if (disposed) {
          cleanup();
          return;
        }

        unlisten = cleanup;
      });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  function appendUrlAttachments(urls: string[]) {
    const nextAttachments = urls.map((url) => createAttachment(url, "url", "image", t.url));

    setAttachments((current) => [...current, ...nextAttachments]);
  }

  function appendFileUrlAttachments(urls: string[]) {
    const nextAttachments = urls.map((url) =>
      createAttachment(url, "url", isLikelyImagePath(url) ? "image" : "file", t.url),
    );

    setAttachments((current) => [...current, ...nextAttachments]);
  }

  function appendPathAttachments(paths: string[]) {
    const nextAttachments = paths.map((path) =>
      createAttachment(path, "path", isLikelyImagePath(path) ? "image" : "file", t.path),
    );

    setAttachments((current) => [...current, ...nextAttachments]);
  }

  async function appendFiles(files: FileList | File[]) {
    const nextAttachments: NoteAttachment[] = [];

    for (const file of Array.from(files)) {
      nextAttachments.push({
        id: createId("asset"),
        kind: file.type.startsWith("image/") ? "image" : "file",
        source: "data",
        value: await readFileAsDataUrl(file),
        name: file.name,
        createdAt: Date.now(),
      });
    }

    if (nextAttachments.length > 0) {
      setAttachments((current) => [...current, ...nextAttachments]);
    }
  }

  function addMediaValue() {
    const value = mediaValue.trim();
    if (!value) {
      return;
    }

    const source = /^https?:\/\//i.test(value) ? "url" : "path";
    const kind = isLikelyImagePath(value) ? "image" : "file";

    setAttachments((current) => [
      ...current,
      createAttachment(value, source, kind, source === "url" ? t.url : t.path),
    ]);
    setMediaValue("");
  }

  async function handlePaste(event: ClipboardEvent<HTMLTextAreaElement>) {
    const files = Array.from(event.clipboardData.items)
      .filter((item) => item.type.startsWith("image/"))
      .map((item) => item.getAsFile())
      .filter((file): file is File => Boolean(file));

    if (files.length > 0) {
      await appendFiles(files);
    }
  }

  async function handleDrop(event: DragEvent<HTMLDivElement>) {
    event.preventDefault();
    setDragging(false);
    const urls = resolveDraggedImageUrls(event.dataTransfer);
    if (urls.length > 0) {
      appendUrlAttachments(urls);
      return;
    }

    const fileUrls = resolveDraggedFileUrls(event.dataTransfer);
    if (fileUrls.length > 0) {
      appendFileUrlAttachments(fileUrls);
      return;
    }

    if (!isTauriRuntime() && event.dataTransfer.files.length > 0) {
      await appendFiles(event.dataTransfer.files);
    }
  }

  function submit() {
    if (!content.trim() && attachments.length === 0) {
      onCancel();
      return;
    }

    onSave({
      attachments,
      color,
      content: content.trim(),
      pinned,
    });
  }

  return (
    <div
      className={isWindowMode ? "editor-window-shell" : "modal-backdrop"}
      onMouseDown={isWindowMode ? undefined : onCancel}
    >
      <section
        aria-modal={isWindowMode ? undefined : true}
        className={`editor-dialog ${isWindowMode ? "is-window" : ""} ${dragging ? "is-dragging" : ""}`}
        onDragEnter={() => setDragging(true)}
        onDragLeave={(event) => {
          const relatedTarget = event.relatedTarget;
          if (!(relatedTarget instanceof Node) || !event.currentTarget.contains(relatedTarget)) {
            setDragging(false);
          }
        }}
        onDragOver={(event) => event.preventDefault()}
        onDrop={handleDrop}
        onMouseDown={(event) => event.stopPropagation()}
        role="dialog"
      >
        <header className="editor-dialog__header" onPointerDown={onDragStart}>
          <div className="editor-dialog__colors" onPointerDown={(event) => event.stopPropagation()}>
            {NOTE_COLORS.map((item) => (
              <button
                aria-label={item}
                className={`color-swatch ${color === item ? "is-selected" : ""}`}
                key={item}
                onClick={() => setColor(item)}
                style={{ backgroundColor: item }}
                type="button"
              />
            ))}
          </div>
          <div onPointerDown={(event) => event.stopPropagation()}>
            <IconButton icon={<X size={18} />} label={t.cancel} onClick={onCancel} subtle />
          </div>
        </header>

        <textarea
          autoFocus
          className="editor-textarea"
          onChange={(event) => setContent(event.currentTarget.value)}
          onPaste={handlePaste}
          placeholder={t.contentPlaceholder}
          value={content}
        />

        <div className="editor-media-row">
          <IconButton
            icon={<ImagePlus size={17} />}
            label={t.addImage}
            onClick={() => fileInputRef.current?.click()}
          >
            {t.addImage}
          </IconButton>
          <input
            accept="image/*"
            hidden
            multiple
            onChange={(event) => {
              if (event.currentTarget.files) {
                void appendFiles(event.currentTarget.files);
              }
              event.currentTarget.value = "";
            }}
            ref={fileInputRef}
            type="file"
          />
          <div className="media-input">
            <Link2 size={16} />
            <input
              onChange={(event) => setMediaValue(event.currentTarget.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter") {
                  event.preventDefault();
                  addMediaValue();
                }
              }}
              placeholder={t.mediaPlaceholder}
              value={mediaValue}
            />
            <Folder size={16} />
          </div>
          <IconButton icon={<ImagePlus size={17} />} label={t.addMedia} onClick={addMediaValue}>
            {t.addMedia}
          </IconButton>
        </div>

        {attachments.length > 0 ? (
          <div className="editor-attachments">
            {attachments.map((attachment) =>
              isImageAttachment(attachment) ? (
                <figure className="editor-image" key={attachment.id}>
                  <button
                    className="editor-image__preview"
                    onClick={() => setPreviewAttachment(attachment)}
                    title={attachment.name ?? t.addImage}
                    type="button"
                  >
                    <img alt={attachment.name ?? t.addImage} src={getAttachmentSrc(attachment)} />
                  </button>
                  <button
                    aria-label={t.removeAttachment}
                    onClick={() =>
                      setAttachments((current) =>
                        current.filter((item) => item.id !== attachment.id),
                      )
                    }
                    title={t.removeAttachment}
                    type="button"
                  >
                    <Trash2 size={14} />
                  </button>
                </figure>
              ) : (
                <div className="editor-file" key={attachment.id}>
                  <FileText size={16} />
                  <span title={attachment.value}>{attachment.name ?? attachment.value}</span>
                  <button
                    aria-label={t.removeAttachment}
                    onClick={() =>
                      setAttachments((current) =>
                        current.filter((item) => item.id !== attachment.id),
                      )
                    }
                    title={t.removeAttachment}
                    type="button"
                  >
                    <Trash2 size={14} />
                  </button>
                </div>
              ),
            )}
          </div>
        ) : null}

        <footer className="editor-dialog__footer">
          <label className="pin-toggle">
            <input
              checked={pinned}
              onChange={(event) => setPinned(event.currentTarget.checked)}
              type="checkbox"
            />
            <span>{t.pinned}</span>
          </label>
          <div className="editor-dialog__buttons">
            <button className="text-button" onClick={onCancel} type="button">
              {t.cancel}
            </button>
            <button className="primary-button" onClick={submit} type="button">
              {t.save}
            </button>
          </div>
        </footer>
      </section>

      {previewAttachment ? (
        <div
          className="image-preview"
          onMouseDown={(event) => {
            event.stopPropagation();
            setPreviewAttachment(null);
          }}
        >
          <button
            aria-label={t.cancel}
            className="image-preview__close"
            onClick={() => setPreviewAttachment(null)}
            type="button"
          >
            <X size={18} />
          </button>
          <img
            alt={previewAttachment.name ?? t.addImage}
            onMouseDown={(event) => event.stopPropagation()}
            src={getAttachmentSrc(previewAttachment)}
          />
        </div>
      ) : null}
    </div>
  );
}
