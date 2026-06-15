import type { MouseEvent, PointerEvent } from "react";
import type { Translation } from "../i18n";
import { QMark } from "./QMark";

interface CompactDockProps {
  onContextMenu: (event: MouseEvent<HTMLButtonElement>) => void;
  onDragStart: () => void;
  onOpenMain: () => void;
  t: Translation;
}

export function CompactDock({ onContextMenu, onDragStart, onOpenMain, t }: CompactDockProps) {
  function handlePointerDown(event: PointerEvent<HTMLButtonElement>) {
    if (event.button !== 0) {
      return;
    }

    onDragStart();
  }

  return (
    <main className="dock-shell">
      <button
        aria-label={t.switchMainWindow}
        className="dock-button"
        onClick={onOpenMain}
        onContextMenu={onContextMenu}
        onPointerDown={handlePointerDown}
        title={t.switchMainWindow}
        type="button"
      >
        <QMark />
      </button>
    </main>
  );
}
