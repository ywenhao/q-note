import { invoke } from "@tauri-apps/api/core";
import { watchEffect, type ComputedRef, type Ref } from "vue";
import type { Translation } from "../i18n";
import { isTauriRuntime } from "../lib/env";

interface UseTrayMenuLabelsOptions {
  alwaysOnLabel: ComputedRef<string>;
  dockToggleLabel: ComputedRef<string>;
  ready: Ref<boolean>;
  t: ComputedRef<Translation>;
}

export function useTrayMenuLabels(options: UseTrayMenuLabelsOptions) {
  watchEffect(() => {
    if (!options.ready.value || !isTauriRuntime()) {
      return;
    }
    void invoke("set_tray_menu_labels", {
      topmost: options.alwaysOnLabel.value,
      quit: options.t.value.quit,
      toggleDock: options.dockToggleLabel.value,
      toggleLanguage: options.t.value.switchLanguage,
    });
  });
}
