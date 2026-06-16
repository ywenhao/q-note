import { Download, FolderOpen, X } from "lucide-react";
import type { Translation } from "../i18n";
import type { UpdateDownloadProgress, UpdateDownloadResult, UpdateInfo } from "../lib/updater";

interface UpdateDownloadDialogProps {
  onCancel: () => void;
  onClose: () => void;
  onReveal: (path: string) => void;
  progress: UpdateDownloadProgress | null;
  result: UpdateDownloadResult | null;
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

export function UpdateDownloadDialog({
  onCancel,
  onClose,
  onReveal,
  progress,
  result,
  t,
  update,
}: UpdateDownloadDialogProps) {
  const percent = result ? 100 : Math.round(progress?.percent ?? 0);
  const total = progress?.total ?? update.asset?.size ?? 0;
  const downloaded = progress?.downloaded ?? 0;
  const sourceLabel = result?.sourceLabel ?? progress?.sourceLabel;

  return (
    <div className="update-download-backdrop">
      <section className="update-download-dialog" role="dialog" aria-modal="true">
        <header>
          <span>
            <Download size={15} />
            {result ? t.updateDownloaded : t.updateDownloading}
          </span>
          <button aria-label={t.closePanel} onClick={result ? onClose : onCancel} type="button">
            <X size={14} />
          </button>
        </header>

        <div className="update-download-body">
          <strong>{`Q Note v${update.latestVersion}`}</strong>
          <span>{update.asset?.name ?? update.tagName}</span>
          {sourceLabel ? <small>{`${t.updateSource}: ${sourceLabel}`}</small> : null}
        </div>

        <div
          aria-label={t.updateDownloadProgress}
          aria-valuemax={100}
          aria-valuemin={0}
          aria-valuenow={percent}
          className="update-progress"
          role="progressbar"
        >
          <span style={{ width: `${percent}%` }} />
        </div>

        <div className="update-download-meta">
          <span>{`${percent}%`}</span>
          {total > 0 ? <span>{`${formatBytes(downloaded)} / ${formatBytes(total)}`}</span> : null}
        </div>

        <footer>
          {result ? (
            <>
              <button className="text-button" onClick={onClose} type="button">
                {t.closePanel}
              </button>
              <button
                className="primary-button"
                onClick={() => onReveal(result.path)}
                type="button"
              >
                <FolderOpen size={14} />
                {t.openDownloadedFile}
              </button>
            </>
          ) : (
            <button className="text-button" onClick={onCancel} type="button">
              {t.cancel}
            </button>
          )}
        </footer>
      </section>
    </div>
  );
}
