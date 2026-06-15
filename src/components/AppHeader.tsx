import { Minus, Pin, PinOff, X } from "lucide-react";
import type { PointerEvent } from "react";
import type { Translation } from "../i18n";
import { IconButton } from "./IconButton";
import { QMark } from "./QMark";

interface AppHeaderProps {
  alwaysOnLabel: string;
  alwaysOnTop: boolean;
  onClose: () => void;
  onDragStart: (event: PointerEvent<HTMLElement>) => void;
  onMinimize: () => void;
  onToggleAlwaysOnTop: () => void;
  t: Translation;
}

export function AppHeader({
  alwaysOnLabel,
  alwaysOnTop,
  onClose,
  onDragStart,
  onMinimize,
  onToggleAlwaysOnTop,
  t,
}: AppHeaderProps) {
  return (
    <header className="top-bar" onPointerDown={onDragStart}>
      <div className="brand">
        <QMark className="brand-mark" />
        <h1>{t.appTitle}</h1>
      </div>
      <div className="title-controls" onPointerDown={(event) => event.stopPropagation()}>
        <IconButton
          active={alwaysOnTop}
          className="is-window-pin"
          icon={alwaysOnTop ? <PinOff size={16} /> : <Pin size={16} />}
          label={alwaysOnLabel}
          onClick={onToggleAlwaysOnTop}
          subtle
        />
        <IconButton
          className="is-window-minimize"
          icon={<Minus size={16} />}
          label={t.minimize}
          onClick={onMinimize}
          subtle
        />
        <IconButton
          className="is-window-close"
          icon={<X size={16} />}
          label={t.closePanel}
          onClick={onClose}
          subtle
        />
      </div>
    </header>
  );
}
