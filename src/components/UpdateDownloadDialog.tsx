import { Download } from "lucide-react";
import type { Translation } from "../i18n";
import type { UpdateDownloadProgress } from "../lib/updateProgress";
import type { UpdateInfo, UpdatePhase } from "../lib/updater";

interface UpdateDownloadDialogProps {
  phase: UpdatePhase;
  progress: UpdateDownloadProgress | null;
  t: Translation;
  update: UpdateInfo;
}

function formatBytes(value: number) {
  if (value < 1024) {
    return `${value} B`;
  }

  const units = ["KB", "MB", "GB"];
  let nextValue = value / 1024;
  let unitIndex = 0;

  while (nextValue >= 1024 && unitIndex < units.length - 1) {
    nextValue /= 1024;
    unitIndex += 1;
  }

  return `${nextValue.toFixed(nextValue >= 10 ? 0 : 1)} ${units[unitIndex]}`;
}

export function UpdateDownloadDialog({ phase, progress, t, update }: UpdateDownloadDialogProps) {
  const percent = phase === "downloading" ? Math.round(progress?.percent ?? 0) : 100;
  const status =
    phase === "downloading"
      ? t.updateDownloading
      : phase === "preparing"
        ? t.updatePreparing
        : t.updateInstalling;

  return (
    <div className="update-download-backdrop">
      <section className="update-download-dialog" role="dialog" aria-modal="true">
        <header>
          <span>
            <Download size={15} />
            {status}
          </span>
        </header>

        <div className="update-download-body">
          <strong>{`Q Note v${update.latestVersion}`}</strong>
          {update.body ? <small>{update.body}</small> : null}
        </div>

        <div
          aria-label={t.updateDownloadProgress}
          aria-valuemax={100}
          aria-valuemin={0}
          aria-valuenow={phase === "downloading" ? percent : undefined}
          className={`update-progress ${phase === "downloading" ? "" : "is-indeterminate"}`}
          role="progressbar"
        >
          <span style={{ width: phase === "downloading" ? `${percent}%` : "35%" }} />
        </div>

        <div className="update-download-meta">
          <span>{phase === "downloading" ? `${percent}%` : status}</span>
          {phase === "downloading" && progress?.total ? (
            <span>{`${formatBytes(progress.downloaded)} / ${formatBytes(progress.total)}`}</span>
          ) : null}
        </div>
      </section>
    </div>
  );
}
