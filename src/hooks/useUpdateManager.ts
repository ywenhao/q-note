import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef, useState } from "react";
import { translations } from "../i18n";
import { isTauriRuntime } from "../lib/env";
import {
  UPDATE_DOWNLOAD_PROGRESS_EVENT,
  FALLBACK_VERSION,
  cancelUpdateDownload,
  checkForUpdate,
  downloadUpdate,
  installDownloadedUpdate,
  openCurrentRelease,
  openReleaseUrl,
  readAppVersion,
  revealDownloadedUpdate,
  type UpdateDownloadProgress,
  type UpdateDownloadResult,
  type UpdateInfo,
} from "../lib/updater";
import { MAIN_WINDOW_LABEL } from "../lib/windowControls";
import type { Language } from "../types";
import type { ShowToast } from "./useToast";

interface UseUpdateManagerOptions {
  currentWindowLabel: string;
  language: Language;
  ready: boolean;
  showToast: ShowToast;
}

export function useUpdateManager({
  currentWindowLabel,
  language,
  ready,
  showToast,
}: UseUpdateManagerOptions) {
  const [appVersion, setAppVersion] = useState(FALLBACK_VERSION);
  const [checkingUpdate, setCheckingUpdate] = useState(false);
  const [downloadingUpdate, setDownloadingUpdate] = useState(false);
  const [updateDialogOpen, setUpdateDialogOpen] = useState(false);
  const [updateDownloadProgress, setUpdateDownloadProgress] =
    useState<UpdateDownloadProgress | null>(null);
  const [updateDownloadResult, setUpdateDownloadResult] = useState<UpdateDownloadResult | null>(
    null,
  );
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);

  const appVersionRef = useRef(appVersion);
  const downloadingUpdateRef = useRef(downloadingUpdate);
  const languageRef = useRef(language);
  const updateCheckRef = useRef(false);
  const updateInfoRef = useRef<UpdateInfo | null>(null);

  useEffect(() => {
    appVersionRef.current = appVersion;
  }, [appVersion]);

  useEffect(() => {
    downloadingUpdateRef.current = downloadingUpdate;
  }, [downloadingUpdate]);

  useEffect(() => {
    languageRef.current = language;
  }, [language]);

  useEffect(() => {
    updateInfoRef.current = updateInfo;
  }, [updateInfo]);

  const runUpdateCheck = useCallback(
    async (manual: boolean) => {
      if (!isTauriRuntime() || updateCheckRef.current) {
        return updateInfoRef.current;
      }

      updateCheckRef.current = true;
      setCheckingUpdate(true);

      try {
        const nextUpdate = await checkForUpdate(appVersionRef.current);
        updateInfoRef.current = nextUpdate;
        setUpdateInfo(nextUpdate);

        if (manual && !nextUpdate) {
          showToast(translations[languageRef.current].updateNone, { icon: false, kind: "info" });
        }

        return nextUpdate;
      } catch {
        if (manual) {
          showToast(translations[languageRef.current].updateCheckFailed, { kind: "error" });
        }

        return null;
      } finally {
        updateCheckRef.current = false;
        setCheckingUpdate(false);
      }
    },
    [showToast],
  );

  const startUpdateDownload = useCallback(
    async (update: UpdateInfo | null = updateInfoRef.current) => {
      if (!update || downloadingUpdateRef.current) {
        return;
      }

      if (!update.asset) {
        showToast(translations[languageRef.current].updateNoAsset, { kind: "info" });
        await openReleaseUrl(update.htmlUrl);
        return;
      }

      downloadingUpdateRef.current = true;
      setDownloadingUpdate(true);
      setUpdateDialogOpen(true);
      setUpdateDownloadProgress(null);
      setUpdateDownloadResult(null);

      try {
        const result = await downloadUpdate(update);
        setUpdateDownloadResult(result);
        try {
          await installDownloadedUpdate(result.path);
        } catch {
          showToast(translations[languageRef.current].updateInstallFailed, { kind: "error" });
        }
      } catch (error) {
        setUpdateDialogOpen(false);
        if (String(error).includes("update-download-cancelled")) {
          showToast(translations[languageRef.current].updateDownloadCancelled, { kind: "info" });
        } else {
          showToast(translations[languageRef.current].updateDownloadFailed, { kind: "error" });
        }
      } finally {
        downloadingUpdateRef.current = false;
        setDownloadingUpdate(false);
      }
    },
    [showToast],
  );

  const handleCheckUpdate = useCallback(async () => {
    if (updateCheckRef.current || downloadingUpdateRef.current) {
      return;
    }

    if (updateInfoRef.current) {
      await startUpdateDownload(updateInfoRef.current);
      return;
    }

    const nextUpdate = await runUpdateCheck(true);
    if (nextUpdate) {
      await startUpdateDownload(nextUpdate);
    }
  }, [runUpdateCheck, startUpdateDownload]);

  const handleCancelUpdateDownload = useCallback(async () => {
    await cancelUpdateDownload();
  }, []);

  const handleRevealDownloadedUpdate = useCallback(async (path: string) => {
    await revealDownloadedUpdate(path);
  }, []);

  const handleOpenCurrentRelease = useCallback(async () => {
    await openCurrentRelease(appVersionRef.current);
  }, []);

  useEffect(() => {
    if (!ready || !isTauriRuntime() || currentWindowLabel !== MAIN_WINDOW_LABEL) {
      return;
    }

    let disposed = false;
    let unlisten: (() => void) | null = null;

    void listen<UpdateDownloadProgress>(UPDATE_DOWNLOAD_PROGRESS_EVENT, (event) => {
      setUpdateDownloadProgress(event.payload);
    }).then((nextUnlisten) => {
      if (disposed) {
        nextUnlisten();
        return;
      }

      unlisten = nextUnlisten;
    });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [currentWindowLabel, ready]);

  useEffect(() => {
    if (!ready || !isTauriRuntime() || currentWindowLabel !== MAIN_WINDOW_LABEL) {
      return;
    }

    let disposed = false;
    let timer: number | null = null;

    const scheduleDailyCheck = () => {
      const now = new Date();
      const nextCheck = new Date(now);
      nextCheck.setHours(17, 0, 0, 0);
      if (nextCheck <= now) {
        nextCheck.setDate(nextCheck.getDate() + 1);
      }

      timer = window.setTimeout(() => {
        if (disposed) {
          return;
        }

        void runUpdateCheck(false).finally(scheduleDailyCheck);
      }, nextCheck.getTime() - now.getTime());
    };

    void (async () => {
      try {
        const version = await readAppVersion();
        if (disposed) {
          return;
        }

        appVersionRef.current = version;
        setAppVersion(version);
      } catch {
        // The fallback version still lets the settings panel render in dev/web contexts.
      }

      if (!disposed) {
        void runUpdateCheck(false);
        scheduleDailyCheck();
      }
    })();

    return () => {
      disposed = true;
      if (timer) {
        window.clearTimeout(timer);
      }
    };
  }, [currentWindowLabel, ready, runUpdateCheck]);

  return {
    appVersion,
    checkingUpdate,
    downloadingUpdate,
    handleCancelUpdateDownload,
    handleCheckUpdate,
    handleOpenCurrentRelease,
    handleRevealDownloadedUpdate,
    setUpdateDialogOpen,
    updateDialogOpen,
    updateDownloadProgress,
    updateDownloadResult,
    updateInfo,
  };
}
