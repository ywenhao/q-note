import { useEffect, useRef, type MouseEvent, type PointerEvent } from "react";
import type { Translation } from "../i18n";
import { QMark } from "./QMark";

const DRAG_THRESHOLD = 4;

interface CompactDockProps {
  onContextMenu: (event: MouseEvent<HTMLButtonElement>) => void;
  onDragEnd: () => void;
  onDragMove: () => void;
  onDragStart: () => void;
  onHoverEnd: () => void;
  onHoverStart: () => void;
  onOpenMain: () => void;
  t: Translation;
}

export function CompactDock({
  onContextMenu,
  onDragEnd,
  onDragMove,
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
  const dragEndCleanupRef = useRef<(() => void) | null>(null);

  useEffect(() => () => cleanupDragEndListeners(), []);

  function cleanupDragEndListeners() {
    dragEndCleanupRef.current?.();
    dragEndCleanupRef.current = null;
  }

  function finishDrag() {
    const pointer = pointerRef.current;
    if (!pointer?.dragging) {
      return;
    }

    pointerRef.current = null;
    cleanupDragEndListeners();
    onDragEnd();
  }

  function listenForDragEnd() {
    cleanupDragEndListeners();

    const handleEnd = () => finishDrag();
    window.addEventListener("pointerup", handleEnd, { capture: true, once: true });
    window.addEventListener("mouseup", handleEnd, { capture: true, once: true });
    dragEndCleanupRef.current = () => {
      window.removeEventListener("pointerup", handleEnd, { capture: true });
      window.removeEventListener("mouseup", handleEnd, { capture: true });
    };
  }

  function handlePointerDown(event: PointerEvent<HTMLButtonElement>) {
    if (event.button !== 0) {
      return;
    }

    cleanupDragEndListeners();
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
    if (!pointer || event.pointerId !== pointer.pointerId) {
      return;
    }

    if (pointer.dragging) {
      onDragMove();
      return;
    }

    const moved = Math.hypot(event.clientX - pointer.startX, event.clientY - pointer.startY);
    if (moved < DRAG_THRESHOLD) {
      return;
    }

    pointer.dragging = true;
    listenForDragEnd();
    onDragStart();
    onDragMove();
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

    if (pointer.dragging) {
      cleanupDragEndListeners();
      onDragEnd();
      return;
    }

    cleanupDragEndListeners();
    onOpenMain();
  }

  function handlePointerCancel() {
    if (pointerRef.current?.dragging) {
      return;
    }

    pointerRef.current = null;
    cleanupDragEndListeners();
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
