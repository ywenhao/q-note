import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";
import { isTauriRuntime } from "./env";

export async function readAutoStartEnabled() {
  if (!isTauriRuntime()) {
    return false;
  }

  return isEnabled();
}

export async function applyAutoStart(enabled: boolean) {
  if (!isTauriRuntime()) {
    return false;
  }

  if (enabled) {
    await enable();
  } else {
    await disable();
  }

  return isEnabled();
}
