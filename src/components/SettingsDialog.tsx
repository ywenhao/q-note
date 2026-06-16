import { Download, LoaderCircle, Power, RefreshCw, Upload, X } from "lucide-react";
import type { Translation } from "../i18n";

interface SettingsDialogProps {
  appVersion: string;
  autoStart: boolean;
  checkingUpdate: boolean;
  hasUpdate: boolean;
  onClose: () => void;
  onCheckUpdate: () => void;
  onExport: () => void;
  onImport: () => void;
  onOpenCurrentRelease: () => void;
  onToggleAutoStart: () => void;
  t: Translation;
}

export function SettingsDialog({
  appVersion,
  autoStart,
  checkingUpdate,
  hasUpdate,
  onCheckUpdate,
  onClose,
  onExport,
  onImport,
  onOpenCurrentRelease,
  onToggleAutoStart,
  t,
}: SettingsDialogProps) {
  return (
    <div className="modal-backdrop" onMouseDown={onClose}>
      <section
        aria-modal="true"
        className="settings-dialog"
        onMouseDown={(event) => event.stopPropagation()}
        role="dialog"
      >
        <header className="settings-dialog__header">
          <h2>{t.settingsTitle}</h2>
          <button className="settings-close" aria-label={t.cancel} onClick={onClose} type="button">
            <X size={14} />
          </button>
        </header>

        <div className="settings-list">
          <button className="settings-row" onClick={onToggleAutoStart} type="button">
            <span className="settings-row__label">
              <Power size={18} />
              {t.startupSetting}
            </span>
            <span className={`switch ${autoStart ? "is-on" : ""}`} aria-hidden="true">
              <span />
            </span>
          </button>
        </div>

        <div className="settings-actions">
          <button className="settings-action" onClick={onImport} type="button">
            <Upload size={18} />
            {t.import}
          </button>
          <button className="settings-action" onClick={onExport} type="button">
            <Download size={18} />
            {t.export}
          </button>
        </div>

        <button
          className={`settings-row ${checkingUpdate ? "is-loading" : ""}`}
          disabled={checkingUpdate}
          onClick={onCheckUpdate}
          type="button"
        >
          <span className="settings-row__label">
            {checkingUpdate ? (
              <LoaderCircle className="spin-icon" size={18} />
            ) : (
              <RefreshCw size={18} />
            )}
            {hasUpdate ? <span className="update-dot" aria-hidden="true" /> : null}
            {t.checkUpdate}
          </span>
          {hasUpdate ? <span className="settings-row__value">{t.updateAvailable}</span> : null}
        </button>

        <footer className="settings-footer">
          <button className="settings-version" onClick={onOpenCurrentRelease} type="button">
            {`v${appVersion}`}
          </button>
        </footer>
      </section>
    </div>
  );
}
