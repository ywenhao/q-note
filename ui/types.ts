export type Language = "zh" | "en";

export type AttachmentKind = "image" | "file";

export type AttachmentSource = "data" | "url" | "path";

export interface NoteAttachment {
  id: string;
  kind: AttachmentKind;
  source: AttachmentSource;
  value: string;
  name?: string;
  createdAt: number;
}

export interface Note {
  id: string;
  content: string;
  color: string;
  pinned: boolean;
  sortOrder: number;
  textHeight: number | null;
  attachments: NoteAttachment[];
  createdAt: number;
  updatedAt: number;
}

export interface NoteDraft {
  attachments: NoteAttachment[];
  color: string;
  content: string;
  pinned: boolean;
}

export type DockEdge = "left" | "right" | "top" | "bottom";

export interface WindowState {
  width: number;
  height: number;
  x: number;
  y: number;
}

export interface AppSettings {
  language: Language;
  alwaysOnTop: boolean;
  autoStart: boolean;
  dockOnEdge: boolean;
  docked: boolean;
  dockEdge: DockEdge | null;
  keepFullMain: boolean;
  window: WindowState | null;
}

export interface AppData {
  notes: Note[];
  settings: AppSettings;
}

export interface ExportPayload extends AppData {
  version: 1;
  exportedAt: string;
}
