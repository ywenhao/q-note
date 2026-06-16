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

export const APP_BACKGROUND = "#ffd150";
export const DOCK_WINDOW_SIZE = 30;
export const DEFAULT_WINDOW_WIDTH = 300;
export const DEFAULT_WINDOW_HEIGHT = 400;

export const NOTE_COLORS = [
  "#fff9db",
  "#ffe8cc",
  "#ffd8a8",
  "#d8f5a2",
  "#b2f2bb",
  "#c5f6fa",
  "#d0ebff",
  "#dbe4ff",
  "#e5dbff",
  "#ffdeeb",
  "#ffe3e3",
  "#f1f3f5",
] as const;

export const DEFAULT_NOTE_COLOR = NOTE_COLORS[0];
