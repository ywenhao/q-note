import { batch, cleanup, signal } from "nsh";
import { QNote } from "q-note/native";
import { translations } from "./i18n";
import type { AppData, NoteAttachment, NoteDraft } from "./types";

export type Role = "main" | "editor" | "dock";
export interface NativeEvent {
  kind: string;
  x?: number;
  y?: number;
  delta?: number;
  paths?: string[];
  request?: string;
  value?: string;
}
export interface Session {
  id: string;
  noteId: string | null;
  draft: NoteDraft;
}
export interface Update {
  phase: string;
  show?: boolean;
  downloaded?: number;
  total?: number;
  info?: { version: string; notes?: string };
}
export interface Snapshot {
  revision: number;
  data: AppData;
  editor: Session | null;
  palette: string[];
  version: string;
  toast: { id: string; key: string; error: boolean } | null;
  update: Update;
  events: NativeEvent[];
}

export function createState(role: Role) {
  const snapshot = signal<Snapshot>({
    revision: 0,
    data: {
      notes: [],
      settings: {
        language: "en",
        alwaysOnTop: false,
        autoStart: false,
        docked: false,
        dockOnEdge: false,
        dockEdge: null,
        keepFullMain: false,
        window: null,
      },
    },
    editor: null,
    palette: [],
    version: "0.2.12",
    toast: null,
    update: { phase: "idle" },
    events: [],
  });
  const loaded = signal(false);
  const toast = signal<{ key: string; error: boolean } | null>(null);
  const listeners = new Set<(event: NativeEvent) => void>();
  let seenToast = "";
  let toastTimer: ReturnType<typeof setTimeout>;
  let polling = false;
  let disposed = false;
  const t = () => translations[snapshot().data.settings.language];
  async function refresh() {
    if (polling || disposed) return;
    polling = true;
    try {
      const raw = await QNote.snapshot(role, snapshot().revision);
      if (raw && !disposed) {
        const next: Snapshot = JSON.parse(raw);
        if (next.revision === snapshot().revision) {
          next.data = snapshot().data;
        }
        batch(() => {
          snapshot.set(next);
          loaded.set(true);
          if (next.toast && next.toast.id !== seenToast) {
            seenToast = next.toast.id;
            toast.set(next.toast);
            if (toastTimer !== undefined) clearTimeout(toastTimer);
            toastTimer = setTimeout(() => toast.set(null), 1700);
          }
        });
        for (const event of next.events)
          for (const listener of listeners) listener(event);
      }
    } catch (error) {
      console.error(error);
    } finally {
      polling = false;
    }
  }
  async function call<T = null>(
    command: string,
    payload: object = {},
  ): Promise<T | undefined> {
    try {
      const result = JSON.parse(
        await QNote.invoke(command, JSON.stringify(payload)),
      ) as T;
      await refresh();
      return result;
    } catch (error) {
      console.error(command, error);
      await refresh();
      return undefined;
    }
  }
  const timer = setInterval(() => void refresh(), 50);
  void refresh();
  cleanup(() => {
    disposed = true;
    clearInterval(timer);
    if (toastTimer !== undefined) clearTimeout(toastTimer);
  });
  return {
    role,
    snapshot,
    loaded,
    toast,
    t,
    call,
    onEvent: (fn: (e: NativeEvent) => void) => {
      listeners.add(fn);
      cleanup(() => listeners.delete(fn));
    },
  };
}

export type AppState = ReturnType<typeof createState>;

export function createAttachment(value: string): NoteAttachment {
  const source = /^https?:\/\//i.test(value)
    ? "url"
    : value.startsWith("data:")
      ? "data"
      : "path";
  const clean = value.split(/[?#]/)[0] ?? value;
  const image =
    /\.(png|jpe?g|gif|webp|bmp|svg|avif)$/i.test(clean) ||
    value.startsWith("data:image/");
  return {
    id: `asset-${Date.now()}-${Math.random().toString(36).slice(2)}`,
    source,
    kind: image ? "image" : "file",
    value,
    name: source === "data" ? undefined : clean.split(/[\\/]/).pop(),
    createdAt: Date.now(),
  };
}
