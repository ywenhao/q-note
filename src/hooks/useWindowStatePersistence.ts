import { getCurrentWindow } from "@tauri-apps/api/window";
import { watch, type Ref } from "vue";
import { clearDockRevealAnchor, isSharedDockGuardActive } from "../lib/dockPersistence";
import { isTauriRuntime } from "../lib/env";
import {
  DOCK_WINDOW_LABEL,
  MAIN_WINDOW_LABEL,
  captureWindowState,
  detectSnapEdge,
} from "../lib/windowControls";
import type { AppSettings, DockEdge } from "../types";

interface UseWindowStatePersistenceOptions {
  currentWindowLabel: string;
  dockDrag: Ref<boolean>;
  dockGuard: Ref<boolean>;
  editorOpen: Ref<boolean>;
  persistIconSnap: (edge: DockEdge) => Promise<void>;
  persistSettings: (patch: Partial<AppSettings>) => Promise<void>;
  ready: Ref<boolean>;
  settings: Ref<AppSettings>;
}

export function useWindowStatePersistence(options: UseWindowStatePersistenceOptions) {
  watch(
    [options.ready, options.editorOpen],
    ([ready, editorOpen], _, onCleanup) => {
      if (!ready || !isTauriRuntime()) {
        return;
      }
      let disposed = false;
      let saveTimer: number | null = null;
      let moveTimer: number | null = null;
      let unlistenMove: (() => void) | null = null;
      let unlistenResize: (() => void) | null = null;
      const isMainWindow = options.currentWindowLabel === MAIN_WINDOW_LABEL;
      const isDockRuntimeWindow = options.currentWindowLabel === DOCK_WINDOW_LABEL;

      const saveWindowSoon = () => {
        if (
          !isMainWindow ||
          options.settings.value.docked ||
          options.dockGuard.value ||
          isSharedDockGuardActive()
        ) {
          return;
        }
        if (saveTimer) {
          window.clearTimeout(saveTimer);
        }
        saveTimer = window.setTimeout(() => {
          void captureWindowState(MAIN_WINDOW_LABEL).then((snapshot) => {
            if (snapshot && !options.settings.value.docked) {
              void options.persistSettings({ window: snapshot });
            }
          });
        }, 300);
      };

      const handleMoved = () => {
        if (isDockRuntimeWindow) {
          if (moveTimer) {
            window.clearTimeout(moveTimer);
            moveTimer = null;
          }
          if (
            !options.settings.value.docked ||
            options.dockDrag.value ||
            options.dockGuard.value ||
            isSharedDockGuardActive()
          ) {
            return;
          }
          moveTimer = window.setTimeout(() => {
            void Promise.all([
              detectSnapEdge(DOCK_WINDOW_LABEL),
              captureWindowState(DOCK_WINDOW_LABEL),
            ]).then(([edge, snapshot]) => {
              if (!snapshot || !options.settings.value.docked) {
                return;
              }
              if (edge) {
                void options.persistIconSnap(edge);
              } else {
                clearDockRevealAnchor();
                void options.persistSettings({ dockEdge: null });
              }
            });
          }, 220);
          return;
        }

        if (!isMainWindow) {
          return;
        }
        if (options.dockGuard.value || isSharedDockGuardActive() || editorOpen) {
          saveWindowSoon();
          return;
        }
        if (moveTimer) {
          window.clearTimeout(moveTimer);
        }
        moveTimer = window.setTimeout(() => {
          void captureWindowState(MAIN_WINDOW_LABEL).then((snapshot) => {
            if (snapshot) {
              void options.persistSettings({ window: snapshot });
            }
          });
        }, 220);
      };

      void (async () => {
        const currentWindow = getCurrentWindow();
        const [moveCleanup, resizeCleanup] = await Promise.all([
          currentWindow.onMoved(handleMoved),
          currentWindow.onResized(saveWindowSoon),
        ]);
        if (disposed) {
          moveCleanup();
          resizeCleanup();
        } else {
          unlistenMove = moveCleanup;
          unlistenResize = resizeCleanup;
        }
      })();

      onCleanup(() => {
        disposed = true;
        if (saveTimer) {
          window.clearTimeout(saveTimer);
        }
        if (moveTimer) {
          window.clearTimeout(moveTimer);
        }
        unlistenMove?.();
        unlistenResize?.();
      });
    },
    { immediate: true },
  );
}
