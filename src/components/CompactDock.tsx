import { useRef, type MouseEvent, type PointerEvent } from "react";
import type { Translation } from "../i18n";
import { QMark } from "./QMark";

interface CompactDockProps {
  onContextMenu: (event: MouseEvent<HTMLButtonElement>) => void;
  onDragStart: () => void;
  onHover: () => void;
  t: Translation;
}

export function CompactDock({ onContextMenu, onDragStart, onHover, t }: CompactDockProps) {
  const hoverTimerRef = useRef<number | null>(null);

  function clearHoverTimer() {
    if (hoverTimerRef.current) {
      window.clearTimeout(hoverTimerRef.current);
      hoverTimerRef.current = null;
    }
  }

  function handlePointerDown(event: PointerEvent<HTMLButtonElement>) {
    if (event.button !== 0) {
      return;
    }

    clearHoverTimer();
    onDragStart();
  }

  return (
    <main className="dock-shell">
      <button
        aria-label={t.restore}
        className="dock-button"
        onContextMenu={onContextMenu}
        onMouseEnter={() => {
          clearHoverTimer();
          hoverTimerRef.current = window.setTimeout(onHover, 480);
        }}
        onMouseLeave={clearHoverTimer}
        onPointerDown={handlePointerDown}
        title={t.restore}
        type="button"
      >
        <QMark />
      </button>
    </main>
  );
}
