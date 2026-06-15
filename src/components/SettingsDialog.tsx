import { Download, Power, Upload, X } from "lucide-react";
import type { Translation } from "../i18n";
import { IconButton } from "./IconButton";

interface SettingsDialogProps {
  autoStart: boolean;
  onClose: () => void;
  onExport: () => void;
  onImport: () => void;
  onToggleAutoStart: () => void;
  t: Translation;
}

export function SettingsDialog({
  autoStart,
  onClose,
  onExport,
  onImport,
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
        <header>
          <h2>{t.settingsTitle}</h2>
          <IconButton icon={<X size={18} />} label={t.cancel} onClick={onClose} subtle />
        </header>
        <button className="settings-row" onClick={onToggleAutoStart} type="button">
          <span>
            <Power size={18} />
            {t.startupSetting}
          </span>
          <span className={`switch ${autoStart ? "is-on" : ""}`} aria-hidden="true">
            <span />
          </span>
        </button>
        <button className="settings-row" onClick={onImport} type="button">
          <span>
            <Upload size={18} />
            {t.import}
          </span>
        </button>
        <button className="settings-row" onClick={onExport} type="button">
          <span>
            <Download size={18} />
            {t.export}
          </span>
        </button>
      </section>
    </div>
  );
}
