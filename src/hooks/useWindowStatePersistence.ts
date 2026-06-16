import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, type MutableRefObject } from "react";
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
  dockDragRef: MutableRefObject<boolean>;
  dockGuardRef: MutableRefObject<boolean>;
  editorOpen: boolean;
  persistIconSnap: (edge: DockEdge) => Promise<void>;
  persistSettings: (patch: Partial<AppSettings>) => Promise<void>;
  ready: boolean;
  settingsRef: MutableRefObject<AppSettings>;
}

export function useWindowStatePersistence({
  currentWindowLabel,
  dockDragRef,
  dockGuardRef,
  editorOpen,
  persistIconSnap,
  persistSettings,
  ready,
  settingsRef,
}: UseWindowStatePersistenceOptions) {
  useEffect(() => {
    if (!ready || !isTauriRuntime()) {
      return;
    }

    let saveTimer: number | null = null;
    let moveTimer: number | null = null;
    let unlistenMove: (() => void) | null = null;
    let unlistenResize: (() => void) | null = null;

    const isMainWindow = currentWindowLabel === MAIN_WINDOW_LABEL;
    const isDockRuntimeWindow = currentWindowLabel === DOCK_WINDOW_LABEL;

    const saveWindowSoon = () => {
      if (
        !isMainWindow ||
        settingsRef.current.docked ||
        dockGuardRef.current ||
        isSharedDockGuardActive()
      ) {
        return;
      }

      if (saveTimer) {
        window.clearTimeout(saveTimer);
      }

      saveTimer = window.setTimeout(() => {
        void captureWindowState(MAIN_WINDOW_LABEL).then((snapshot) => {
          if (snapshot && !settingsRef.current.docked) {
            void persistSettings({ window: snapshot });
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

        if (!settingsRef.current.docked || dockDragRef.current) {
          return;
        }

        if (dockGuardRef.current || isSharedDockGuardActive()) {
          return;
        }

        moveTimer = window.setTimeout(() => {
          void Promise.all([
            detectSnapEdge(DOCK_WINDOW_LABEL),
            captureWindowState(DOCK_WINDOW_LABEL),
          ]).then(([edge, snapshot]) => {
            if (!snapshot || !settingsRef.current.docked) {
              return;
            }

            if (edge) {
              void persistIconSnap(edge);
              return;
            }

            clearDockRevealAnchor();
            void persistSettings({ dockEdge: null });
          });
        }, 220);

        return;
      }

      if (!isMainWindow) {
        return;
      }

      if (dockGuardRef.current || isSharedDockGuardActive() || editorOpen) {
        saveWindowSoon();
        return;
      }

      if (moveTimer) {
        window.clearTimeout(moveTimer);
      }

      moveTimer = window.setTimeout(() => {
        void captureWindowState(MAIN_WINDOW_LABEL).then((snapshot) => {
          if (!snapshot) {
            return;
          }

          void persistSettings({ window: snapshot });
        });
      }, 220);
    };

    void (async () => {
      const currentWindow = getCurrentWindow();
      unlistenMove = await currentWindow.onMoved(handleMoved);
      unlistenResize = await currentWindow.onResized(saveWindowSoon);
    })();

    return () => {
      if (saveTimer) {
        window.clearTimeout(saveTimer);
      }
      if (moveTimer) {
        window.clearTimeout(moveTimer);
      }
      unlistenMove?.();
      unlistenResize?.();
    };
  }, [
    currentWindowLabel,
    dockDragRef,
    dockGuardRef,
    editorOpen,
    persistIconSnap,
    persistSettings,
    ready,
    settingsRef,
  ]);
}
