import { invoke } from "@tauri-apps/api/core";
import { useEffect } from "react";
import type { Translation } from "../i18n";
import { isTauriRuntime } from "../lib/env";

interface UseTrayMenuLabelsOptions {
  alwaysOnLabel: string;
  dockToggleLabel: string;
  ready: boolean;
  t: Translation;
}

export function useTrayMenuLabels({
  alwaysOnLabel,
  dockToggleLabel,
  ready,
  t,
}: UseTrayMenuLabelsOptions) {
  useEffect(() => {
    if (!ready || !isTauriRuntime()) {
      return;
    }

    void invoke("set_tray_menu_labels", {
      topmost: alwaysOnLabel,
      quit: t.quit,
      toggleDock: dockToggleLabel,
      toggleLanguage: t.switchLanguage,
    });
  }, [alwaysOnLabel, dockToggleLabel, ready, t.quit, t.switchLanguage]);
}
