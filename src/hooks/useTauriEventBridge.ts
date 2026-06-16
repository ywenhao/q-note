import { listen } from "@tauri-apps/api/event";
import { useEffect, type Dispatch, type MutableRefObject, type SetStateAction } from "react";
import type { ShowToast } from "./useToast";
import { translations } from "../i18n";
import { isTauriRuntime } from "../lib/env";
import type { AppSettings, Note } from "../types";

interface UseTauriEventBridgeOptions {
  commitNotes: (nextNotes: Note[]) => void;
  notesRef: MutableRefObject<Note[]>;
  ready: boolean;
  restoreDock: (options?: { keepFull?: boolean; preserveRevealAnchor?: boolean }) => Promise<void>;
  setSettings: Dispatch<SetStateAction<AppSettings>>;
  settingsRef: MutableRefObject<AppSettings>;
  showToast: ShowToast;
  toggleAlwaysOnTop: () => Promise<void>;
  toggleDockOnEdge: () => Promise<void>;
  toggleLanguage: () => Promise<void>;
}

export function useTauriEventBridge({
  commitNotes,
  notesRef,
  ready,
  restoreDock,
  setSettings,
  settingsRef,
  showToast,
  toggleAlwaysOnTop,
  toggleDockOnEdge,
  toggleLanguage,
}: UseTauriEventBridgeOptions) {
  useEffect(() => {
    if (!ready || !isTauriRuntime()) {
      return;
    }

    let disposed = false;
    let unlistenHandlers: Array<() => void> = [];

    void (async () => {
      const handlers = await Promise.all([
        listen("q-note-toggle-always-on-top", () => {
          void toggleAlwaysOnTop();
        }),
        listen("q-note-toggle-language", () => {
          void toggleLanguage();
        }),
        listen("q-note-toggle-dock", () => {
          void toggleDockOnEdge();
        }),
        listen("q-note-show-main", () => {
          if (settingsRef.current.docked) {
            void restoreDock({ keepFull: true });
          }
        }),
        listen<Note>("q-note-note-saved", (event) => {
          commitNotes([
            event.payload,
            ...notesRef.current.filter((note) => note.id !== event.payload.id),
          ]);
          showToast(translations[settingsRef.current.language].saved);
        }),
        listen<AppSettings>("q-note-settings-updated", (event) => {
          settingsRef.current = event.payload;
          setSettings(event.payload);
        }),
      ]);

      if (disposed) {
        handlers.forEach((unlisten) => unlisten());
        return;
      }

      unlistenHandlers = handlers;
    })();

    return () => {
      disposed = true;
      unlistenHandlers.forEach((unlisten) => unlisten());
    };
  }, [
    commitNotes,
    notesRef,
    ready,
    restoreDock,
    setSettings,
    settingsRef,
    showToast,
    toggleAlwaysOnTop,
    toggleDockOnEdge,
    toggleLanguage,
  ]);
}
