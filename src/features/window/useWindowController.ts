import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { isTauriRuntime } from "../../lib/env";
import { startMainWindowDrag } from "../../lib/windowControls";

export function useWindowController(collapseToQIcon: () => Promise<void>) {
  async function minimizeWindow() {
    if (!isTauriRuntime()) {
      await collapseToQIcon();
      return;
    }
    await getCurrentWindow().minimize();
  }

  async function closeWindow() {
    if (!isTauriRuntime()) {
      await collapseToQIcon();
      return;
    }
    await getCurrentWindow().close();
  }

  async function quitApp() {
    if (isTauriRuntime()) {
      await invoke("quit_app");
    }
  }

  function dragMainWindow(event: PointerEvent) {
    if (event.button === 0) {
      void startMainWindowDrag();
    }
  }

  return { closeWindow, dragMainWindow, minimizeWindow, quitApp };
}
