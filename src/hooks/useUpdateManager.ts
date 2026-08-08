import { computed, onBeforeUnmount, ref, watch, type ComputedRef, type Ref } from "vue";
import { translations } from "../i18n";
import {
  readRuntimeBundleInfo,
  shouldOpenReleaseForUpdate,
  type BundleType,
  type RuntimeBundleInfo,
} from "../lib/bundleType";
import { isTauriRuntime } from "../lib/env";
import { getUpdateConfirmBody } from "../lib/updateInstallHint";
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
  persistStateBeforeUpdate: () => Promise<void>;
  prepareForUpdate: () => Promise<void>;
  ready: Ref<boolean>;
  showToast: ShowToast;
}

export function useUpdateManager(options: UseUpdateManagerOptions) {
  const appVersion = ref(FALLBACK_VERSION);
  const bundleType = ref<BundleType>("unknown");
  const checkingUpdate = ref(false);
  const downloadingUpdate = ref(false);
  const runtimeBundleInfo = ref<RuntimeBundleInfo>({ bundleType: "unknown", os: "other" });
  const updateConfirmOpen = ref(false);
  const updateDialogOpen = ref(false);
  const updateDownloadProgress = ref<UpdateDownloadProgress | null>(null);
  const updatePhase = ref<UpdatePhase>("downloading");
  let updateCheckActive = false;
  let currentUpdate: AppUpdate | null = null;
  let downloadCancelled = false;

  const updateInfo = ref<UpdateInfo | null>(null);
  const hasUpdate = computed(() => Boolean(updateInfo.value));
  const updateConfirmBody = computed(() =>
    getUpdateConfirmBody(translations[options.language.value], bundleType.value),
  );

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
    updateInfo.value = null;
    await update?.close().catch(() => undefined);
  }

  async function refreshRuntimeBundleInfo() {
    const info = await readRuntimeBundleInfo();
    runtimeBundleInfo.value = info;
    bundleType.value = info.bundleType;
  }

  async function openReleaseFallback(version: string) {
    await openCurrentRelease(version);
    options.showToast(translations[options.language.value].updateOpenRelease, {
      icon: false,
      kind: "info",
    });
  }

  async function runUpdateCheck(manual: boolean) {
    if (!isTauriRuntime() || updateCheckActive) {
      return currentUpdate;
    }
    updateCheckActive = true;
    checkingUpdate.value = true;
    try {
      await refreshRuntimeBundleInfo();
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
    downloadCancelled = false;
    downloadingUpdate.value = true;
    updateDialogOpen.value = true;
    updateDownloadProgress.value = null;
    updatePhase.value = "downloading";
    let phase: UpdatePhase = "downloading";
    try {
      await downloadUpdate(update, (event) => {
        if (downloadCancelled) {
          return;
        }
        updateDownloadProgress.value = reduceUpdateDownloadProgress(
          updateDownloadProgress.value,
          event,
        );
      });
      if (downloadCancelled) {
        await disposeCurrentUpdate();
        return;
      }
      phase = "preparing";
      updatePhase.value = phase;
      await options.prepareForUpdate();
      if (downloadCancelled) {
        await disposeCurrentUpdate();
        return;
      }
      phase = "installing";
      updatePhase.value = phase;
      await installDownloadedUpdate(update);
      await relaunchUpdatedApp();
    } catch {
      if (downloadCancelled) {
        await disposeCurrentUpdate();
        return;
      }
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

  async function cancelUpdateDownload() {
    if (!updateDialogOpen.value && !downloadingUpdate.value) {
      return;
    }
    downloadCancelled = true;
    updateDialogOpen.value = false;
    updateConfirmOpen.value = false;
    updateDownloadProgress.value = null;
    // Download already finished: free the in-memory package immediately.
    // Mid-download cleanup happens when downloadUpdate settles.
    if (updatePhase.value !== "downloading") {
      await disposeCurrentUpdate();
    }
  }

  async function handleCheckUpdate() {
    if (updateCheckActive || downloadingUpdate.value || updateConfirmOpen.value) {
      return;
    }
    const update = currentUpdate ?? (await runUpdateCheck(true));
    if (!update) {
      return;
    }

    if (shouldOpenReleaseForUpdate(runtimeBundleInfo.value)) {
      await openReleaseFallback(update.version);
      return;
    }

    updateConfirmOpen.value = true;
  }

  function cancelUpdateConfirm() {
    updateConfirmOpen.value = false;
  }

  async function confirmUpdate() {
    const update = currentUpdate;
    updateConfirmOpen.value = false;
    if (!update) {
      return;
    }

    try {
      await options.persistStateBeforeUpdate();
    } catch {
      options.showToast(translations[options.language.value].updatePrepareFailed, {
        kind: "error",
      });
      return;
    }

    await startUpdateDownload(update);
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
          await refreshRuntimeBundleInfo();
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
    bundleType,
    cancelUpdateConfirm,
    cancelUpdateDownload,
    checkingUpdate,
    confirmUpdate,
    downloadingUpdate,
    handleCheckUpdate,
    handleOpenCurrentRelease,
    hasUpdate,
    updateConfirmBody,
    updateConfirmOpen,
    updateDialogOpen,
    updateDownloadProgress,
    updateInfo,
    updatePhase,
  };
}
