import { listen } from "@tauri-apps/api/event";
import { watch, type Ref } from "vue";
import { translations } from "../i18n";
import { isTauriRuntime } from "../lib/env";
import type { AppSettings, Note } from "../types";
import type { ShowToast } from "./useToast";

interface UseTauriEventBridgeOptions {
  commitNotes: (nextNotes: Note[]) => void;
  notes: Ref<Note[]>;
  ready: Ref<boolean>;
  restoreDock: (options?: { keepFull?: boolean; preserveRevealAnchor?: boolean }) => Promise<void>;
  settings: Ref<AppSettings>;
  showToast: ShowToast;
  toggleAlwaysOnTop: () => Promise<void>;
  toggleDockOnEdge: () => Promise<void>;
  toggleLanguage: () => Promise<void>;
}

export function useTauriEventBridge(options: UseTauriEventBridgeOptions) {
  watch(
    options.ready,
    (ready, _, onCleanup) => {
      if (!ready || !isTauriRuntime()) {
        return;
      }
      let disposed = false;
      let unlistenHandlers: Array<() => void> = [];
      void Promise.all([
        listen("q-note-toggle-always-on-top", () => void options.toggleAlwaysOnTop()),
        listen("q-note-toggle-language", () => void options.toggleLanguage()),
        listen("q-note-toggle-dock", () => void options.toggleDockOnEdge()),
        listen("q-note-show-main", () => {
          if (options.settings.value.docked) {
            void options.restoreDock({ keepFull: true });
          }
        }),
        listen<Note>("q-note-note-saved", (event) => {
          options.commitNotes([
            event.payload,
            ...options.notes.value.filter((note) => note.id !== event.payload.id),
          ]);
          options.showToast(translations[options.settings.value.language].saved);
        }),
        listen<AppSettings>("q-note-settings-updated", (event) => {
          options.settings.value = event.payload;
        }),
      ]).then((handlers) => {
        if (disposed) {
          handlers.forEach((unlisten) => unlisten());
        } else {
          unlistenHandlers = handlers;
        }
      });
      onCleanup(() => {
        disposed = true;
        unlistenHandlers.forEach((unlisten) => unlisten());
      });
    },
    { immediate: true },
  );
}
