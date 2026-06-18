import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useCallback, type PointerEvent } from "react";
import { isTauriRuntime } from "../../lib/env";
import { startMainWindowDrag } from "../../lib/windowControls";

interface UseWindowControllerOptions {
  collapseToQIcon: () => Promise<void>;
}

export function useWindowController({ collapseToQIcon }: UseWindowControllerOptions) {
  const minimizeWindow = useCallback(async () => {
    if (!isTauriRuntime()) {
      await collapseToQIcon();
      return;
    }

    await getCurrentWindow().minimize();
  }, [collapseToQIcon]);

  const closeWindow = useCallback(async () => {
    if (!isTauriRuntime()) {
      await collapseToQIcon();
      return;
    }

    await getCurrentWindow().close();
  }, [collapseToQIcon]);

  const quitApp = useCallback(async () => {
    if (!isTauriRuntime()) {
      return;
    }

    await invoke("quit_app");
  }, []);

  const dragMainWindow = useCallback((event: PointerEvent<HTMLElement>) => {
    if (event.button !== 0) {
      return;
    }

    void startMainWindowDrag();
  }, []);

  return {
    closeWindow,
    dragMainWindow,
    minimizeWindow,
    quitApp,
  };
}
