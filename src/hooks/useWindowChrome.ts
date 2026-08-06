import { getCurrentWindow } from "@tauri-apps/api/window";
import { isTauriRuntime } from "../lib/env";

export async function minimizeCurrentWindow() {
  if (!isTauriRuntime()) {
    return;
  }
  await getCurrentWindow().minimize();
}

export function startCurrentWindowDrag(event: PointerEvent) {
  if (event.button === 0 && isTauriRuntime()) {
    void getCurrentWindow().startDragging();
  }
}

export async function setCurrentWindowAlwaysOnTop(enabled: boolean) {
  if (!isTauriRuntime()) {
    return;
  }
  await getCurrentWindow().setAlwaysOnTop(enabled);
}
