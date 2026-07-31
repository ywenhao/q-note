import { useCallback, useEffect, useRef, useState } from "react";
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
  language: Language;
  prepareForUpdate: () => Promise<void>;
  ready: boolean;
  showToast: ShowToast;
}

export function useUpdateManager({
  currentWindowLabel,
  language,
  prepareForUpdate,
  ready,
  showToast,
}: UseUpdateManagerOptions) {
  const [appVersion, setAppVersion] = useState(FALLBACK_VERSION);
  const [checkingUpdate, setCheckingUpdate] = useState(false);
  const [downloadingUpdate, setDownloadingUpdate] = useState(false);
  const [updateDialogOpen, setUpdateDialogOpen] = useState(false);
  const [updateDownloadProgress, setUpdateDownloadProgress] =
    useState<UpdateDownloadProgress | null>(null);
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [updatePhase, setUpdatePhase] = useState<UpdatePhase>("downloading");

  const appVersionRef = useRef(appVersion);
  const downloadingUpdateRef = useRef(downloadingUpdate);
  const languageRef = useRef(language);
  const updateCheckRef = useRef(false);
  const updateRef = useRef<AppUpdate | null>(null);

  useEffect(() => {
    appVersionRef.current = appVersion;
  }, [appVersion]);

  useEffect(() => {
    downloadingUpdateRef.current = downloadingUpdate;
  }, [downloadingUpdate]);

  useEffect(() => {
    languageRef.current = language;
  }, [language]);

  const replaceUpdate = useCallback(async (nextUpdate: AppUpdate | null) => {
    const previousUpdate = updateRef.current;
    updateRef.current = nextUpdate;
    setUpdateInfo(nextUpdate ? toUpdateInfo(nextUpdate) : null);
    if (previousUpdate && previousUpdate !== nextUpdate) {
      await previousUpdate.close().catch(() => undefined);
    }
  }, []);

  const disposeCurrentUpdate = useCallback(async () => {
    const currentUpdate = updateRef.current;
    updateRef.current = null;
    await currentUpdate?.close().catch(() => undefined);
  }, []);

  const runUpdateCheck = useCallback(
    async (manual: boolean) => {
      if (!isTauriRuntime() || updateCheckRef.current) {
        return updateRef.current;
      }

      updateCheckRef.current = true;
      setCheckingUpdate(true);

      try {
        const nextUpdate = await checkForUpdate();
        await replaceUpdate(nextUpdate);

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
    [replaceUpdate, showToast],
  );

  const startUpdateDownload = useCallback(
    async (update: AppUpdate | null = updateRef.current) => {
      if (!update || downloadingUpdateRef.current) {
        return;
      }

      downloadingUpdateRef.current = true;
      setDownloadingUpdate(true);
      setUpdateDialogOpen(true);
      setUpdateDownloadProgress(null);
      setUpdatePhase("downloading");
      let phase: UpdatePhase = "downloading";

      try {
        await downloadUpdate(update, (event) => {
          setUpdateDownloadProgress((current) => reduceUpdateDownloadProgress(current, event));
        });

        phase = "preparing";
        setUpdatePhase(phase);
        await prepareForUpdate();

        phase = "installing";
        setUpdatePhase(phase);
        await installDownloadedUpdate(update);
        await relaunchUpdatedApp();
      } catch {
        setUpdateDialogOpen(false);
        const t = translations[languageRef.current];
        showToast(
          phase === "downloading"
            ? t.updateDownloadFailed
            : phase === "preparing"
              ? t.updatePrepareFailed
              : t.updateInstallFailed,
          { kind: "error" },
        );
        await disposeCurrentUpdate();
      } finally {
        downloadingUpdateRef.current = false;
        setDownloadingUpdate(false);
      }
    },
    [disposeCurrentUpdate, prepareForUpdate, showToast],
  );

  const handleCheckUpdate = useCallback(async () => {
    if (updateCheckRef.current || downloadingUpdateRef.current) {
      return;
    }

    const update = updateRef.current ?? (await runUpdateCheck(true));
    if (update) {
      await startUpdateDownload(update);
    }
  }, [runUpdateCheck, startUpdateDownload]);

  const handleOpenCurrentRelease = useCallback(async () => {
    await openCurrentRelease(appVersionRef.current);
  }, []);

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

  useEffect(
    () => () => {
      void updateRef.current?.close();
    },
    [],
  );

  return {
    appVersion,
    checkingUpdate,
    downloadingUpdate,
    handleCheckUpdate,
    handleOpenCurrentRelease,
    updateDialogOpen,
    updateDownloadProgress,
    updateInfo,
    updatePhase,
  };
}
