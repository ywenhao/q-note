import { useRef, type MouseEvent, type PointerEvent } from "react";
import type { Translation } from "../i18n";
import { QMark } from "./QMark";

const DRAG_THRESHOLD = 4;

interface CompactDockProps {
  onContextMenu: (event: MouseEvent<HTMLButtonElement>) => void;
  onDragStart: () => void;
  onHoverEnd: () => void;
  onHoverStart: () => void;
  onOpenMain: () => void;
  t: Translation;
}

export function CompactDock({
  onContextMenu,
  onDragStart,
  onHoverEnd,
  onHoverStart,
  onOpenMain,
  t,
}: CompactDockProps) {
  const pointerRef = useRef<{
    dragging: boolean;
    pointerId: number;
    startX: number;
    startY: number;
  } | null>(null);

  function handlePointerDown(event: PointerEvent<HTMLButtonElement>) {
    if (event.button !== 0) {
      return;
    }

    pointerRef.current = {
      dragging: false,
      pointerId: event.pointerId,
      startX: event.clientX,
      startY: event.clientY,
    };
    event.currentTarget.setPointerCapture(event.pointerId);
  }

  function handlePointerMove(event: PointerEvent<HTMLButtonElement>) {
    const pointer = pointerRef.current;
    if (!pointer || pointer.dragging || event.pointerId !== pointer.pointerId) {
      return;
    }

    const moved = Math.hypot(event.clientX - pointer.startX, event.clientY - pointer.startY);
    if (moved < DRAG_THRESHOLD) {
      return;
    }

    pointer.dragging = true;
    onDragStart();
  }

  function handlePointerUp(event: PointerEvent<HTMLButtonElement>) {
    const pointer = pointerRef.current;
    if (!pointer || event.pointerId !== pointer.pointerId) {
      return;
    }

    pointerRef.current = null;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }

    if (!pointer.dragging) {
      onOpenMain();
    }
  }

  function handlePointerCancel() {
    pointerRef.current = null;
  }

  return (
    <main className="dock-shell">
      <button
        aria-label={t.switchMainWindow}
        className="dock-button"
        onContextMenu={onContextMenu}
        onMouseEnter={onHoverStart}
        onMouseLeave={onHoverEnd}
        onPointerCancel={handlePointerCancel}
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
        title={t.switchMainWindow}
        type="button"
      >
        <QMark />
      </button>
    </main>
  );
}
