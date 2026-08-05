import { computed, onBeforeUnmount, ref, watch, type ComputedRef, type Ref } from "vue";
import { translations } from "../i18n";
import { isTauriRuntime } from "../lib/env";
import { reduceUpdateDownloadProgress, type UpdateDownloadProgress } from "../lib/updateProgress";
import {
  FALLBACK_VERSION,
  checkForUpdate,
  downloadUpdate,
  installDownloadedUpdate,
  openCurrentRelease,
  readAppVersion,
  relaunchUpdatedApp,
  toUpdateInfo,
  type AppUpdate,
  type UpdateInfo,
  type UpdatePhase,
} from "../lib/updater";
import { MAIN_WINDOW_LABEL } from "../lib/windowControls";
import type { Language } from "../types";
import type { ShowToast } from "./useToast";

interface UseUpdateManagerOptions {
  currentWindowLabel: string;
  language: ComputedRef<Language>;
  prepareForUpdate: () => Promise<void>;
  ready: Ref<boolean>;
  showToast: ShowToast;
}

export function useUpdateManager(options: UseUpdateManagerOptions) {
  const appVersion = ref(FALLBACK_VERSION);
  const checkingUpdate = ref(false);
  const downloadingUpdate = ref(false);
  const updateDialogOpen = ref(false);
  const updateDownloadProgress = ref<UpdateDownloadProgress | null>(null);
  const updatePhase = ref<UpdatePhase>("downloading");
  let updateCheckActive = false;
  let currentUpdate: AppUpdate | null = null;

  const updateInfo = ref<UpdateInfo | null>(null);
  const hasUpdate = computed(() => Boolean(updateInfo.value));

  async function replaceUpdate(nextUpdate: AppUpdate | null) {
    const previousUpdate = currentUpdate;
    currentUpdate = nextUpdate;
    updateInfo.value = nextUpdate ? toUpdateInfo(nextUpdate) : null;
    if (previousUpdate && previousUpdate !== nextUpdate) {
      await previousUpdate.close().catch(() => undefined);
    }
  }

  async function disposeCurrentUpdate() {
    const update = currentUpdate;
    currentUpdate = null;
    await update?.close().catch(() => undefined);
  }

  async function runUpdateCheck(manual: boolean) {
    if (!isTauriRuntime() || updateCheckActive) {
      return currentUpdate;
    }
    updateCheckActive = true;
    checkingUpdate.value = true;
    try {
      const nextUpdate = await checkForUpdate();
      await replaceUpdate(nextUpdate);
      if (manual && !nextUpdate) {
        options.showToast(translations[options.language.value].updateNone, {
          icon: false,
          kind: "info",
        });
      }
      return nextUpdate;
    } catch {
      if (manual) {
        options.showToast(translations[options.language.value].updateCheckFailed, {
          kind: "error",
        });
      }
      return null;
    } finally {
      updateCheckActive = false;
      checkingUpdate.value = false;
    }
  }

  async function startUpdateDownload(update: AppUpdate | null = currentUpdate) {
    if (!update || downloadingUpdate.value) {
      return;
    }
    downloadingUpdate.value = true;
    updateDialogOpen.value = true;
    updateDownloadProgress.value = null;
    updatePhase.value = "downloading";
    let phase: UpdatePhase = "downloading";
    try {
      await downloadUpdate(update, (event) => {
        updateDownloadProgress.value = reduceUpdateDownloadProgress(
          updateDownloadProgress.value,
          event,
        );
      });
      phase = "preparing";
      updatePhase.value = phase;
      await options.prepareForUpdate();
      phase = "installing";
      updatePhase.value = phase;
      await installDownloadedUpdate(update);
      await relaunchUpdatedApp();
    } catch {
      updateDialogOpen.value = false;
      const t = translations[options.language.value];
      options.showToast(
        phase === "downloading"
          ? t.updateDownloadFailed
          : phase === "preparing"
            ? t.updatePrepareFailed
            : t.updateInstallFailed,
        { kind: "error" },
      );
      await disposeCurrentUpdate();
    } finally {
      downloadingUpdate.value = false;
    }
  }

  async function handleCheckUpdate() {
    if (updateCheckActive || downloadingUpdate.value) {
      return;
    }
    const update = currentUpdate ?? (await runUpdateCheck(true));
    if (update) {
      await startUpdateDownload(update);
    }
  }

  async function handleOpenCurrentRelease() {
    await openCurrentRelease(appVersion.value);
  }

  watch(
    options.ready,
    (ready, _, onCleanup) => {
      if (!ready || !isTauriRuntime() || options.currentWindowLabel !== MAIN_WINDOW_LABEL) {
        return;
      }
      let disposed = false;
      let timer: number | null = null;
      const scheduleDailyCheck = () => {
        if (disposed) {
          return;
        }
        const now = new Date();
        const nextCheck = new Date(now);
        nextCheck.setHours(17, 0, 0, 0);
        if (nextCheck <= now) {
          nextCheck.setDate(nextCheck.getDate() + 1);
        }
        timer = window.setTimeout(() => {
          if (!disposed) {
            void runUpdateCheck(false).finally(scheduleDailyCheck);
          }
        }, nextCheck.getTime() - now.getTime());
      };

      void (async () => {
        try {
          const version = await readAppVersion();
          if (!disposed) {
            appVersion.value = version;
          }
        } catch {
          // The fallback version keeps settings usable in browser development.
        }
        if (!disposed) {
          void runUpdateCheck(false);
          scheduleDailyCheck();
        }
      })();

      onCleanup(() => {
        disposed = true;
        if (timer) {
          window.clearTimeout(timer);
        }
      });
    },
    { immediate: true },
  );

  onBeforeUnmount(() => {
    void currentUpdate?.close();
  });

  return {
    appVersion,
    checkingUpdate,
    downloadingUpdate,
    handleCheckUpdate,
    handleOpenCurrentRelease,
    hasUpdate,
    updateDialogOpen,
    updateDownloadProgress,
    updateInfo,
    updatePhase,
  };
}
