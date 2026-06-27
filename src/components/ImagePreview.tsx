import { ChevronLeft, ChevronRight, RotateCcw, X, ZoomIn, ZoomOut } from "lucide-react";
import {
  useEffect,
  useRef,
  useState,
  type KeyboardEvent,
  type MouseEvent,
  type PointerEvent,
  type WheelEvent,
} from "react";
import type { Translation } from "../i18n";

export interface ImagePreviewItem {
  alt: string;
  id: string;
  src: string;
}

interface ImagePreviewProps {
  initialIndex?: number;
  items: ImagePreviewItem[];
  onClose: () => void;
  t: Translation;
}

interface PanOffset {
  x: number;
  y: number;
}

interface DragState {
  offset: PanOffset;
  pointerId: number;
  startX: number;
  startY: number;
}

const MIN_SCALE = 0.5;
const MAX_SCALE = 6;
const SCALE_STEP = 0.25;
const KEY_PAN_STEP = 36;
const PAN_OVERSCROLL = 80;

function clamp(value: number, min: number, max: number) {
  return Math.max(min, Math.min(value, max));
}

function stopPreviewEvent(event: MouseEvent | PointerEvent) {
  event.stopPropagation();
}

export function ImagePreview({ initialIndex = 0, items, onClose, t }: ImagePreviewProps) {
  const imageRef = useRef<HTMLImageElement>(null);
  const rootRef = useRef<HTMLDivElement>(null);
  const stageRef = useRef<HTMLDivElement>(null);
  const dragRef = useRef<DragState | null>(null);
  const [currentIndex, setCurrentIndex] = useState(() => clamp(initialIndex, 0, items.length - 1));
  const [offset, setOffset] = useState<PanOffset>({ x: 0, y: 0 });
  const [scale, setScale] = useState(1);

  const currentItem = items[currentIndex];
  const canNavigate = items.length > 1;
  const canPan = scale > 1;

  useEffect(() => {
    rootRef.current?.focus();
  }, []);

  useEffect(() => {
    setCurrentIndex(clamp(initialIndex, 0, items.length - 1));
    resetTransform();
  }, [initialIndex, items.length]);

  if (!currentItem) {
    return null;
  }

  function resetTransform() {
    dragRef.current = null;
    setOffset({ x: 0, y: 0 });
    setScale(1);
  }

  function getClampedOffset(nextOffset: PanOffset, nextScale = scale) {
    const stage = stageRef.current;
    const image = imageRef.current;
    if (!stage || !image || nextScale <= 1) {
      return { x: 0, y: 0 };
    }

    const maxX = Math.max((image.clientWidth * nextScale - stage.clientWidth) / 2, 0);
    const maxY = Math.max((image.clientHeight * nextScale - stage.clientHeight) / 2, 0);

    return {
      x: clamp(nextOffset.x, -maxX - PAN_OVERSCROLL, maxX + PAN_OVERSCROLL),
      y: clamp(nextOffset.y, -maxY - PAN_OVERSCROLL, maxY + PAN_OVERSCROLL),
    };
  }

  function updateScale(nextScale: number) {
    const clampedScale = clamp(nextScale, MIN_SCALE, MAX_SCALE);
    setScale(clampedScale);
    setOffset((current) => getClampedOffset(current, clampedScale));
  }

  function panBy(deltaX: number, deltaY: number) {
    if (!canPan) {
      return;
    }

    setOffset((current) =>
      getClampedOffset({
        x: current.x + deltaX,
        y: current.y + deltaY,
      }),
    );
  }

  function showImage(index: number) {
    setCurrentIndex((current) => {
      const nextIndex = (index + items.length) % items.length;
      return nextIndex === current ? current : nextIndex;
    });
    resetTransform();
  }

  function showPrevious() {
    if (canNavigate) {
      showImage(currentIndex - 1);
    }
  }

  function showNext() {
    if (canNavigate) {
      showImage(currentIndex + 1);
    }
  }

  function handleKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    if (event.key === "Escape") {
      event.preventDefault();
      onClose();
      return;
    }

    if (event.key === "[" || event.key === "PageUp") {
      event.preventDefault();
      showPrevious();
      return;
    }

    if (event.key === "]" || event.key === "PageDown") {
      event.preventDefault();
      showNext();
      return;
    }

    if (event.key === "+" || event.key === "=") {
      event.preventDefault();
      updateScale(scale + SCALE_STEP);
      return;
    }

    if (event.key === "-") {
      event.preventDefault();
      updateScale(scale - SCALE_STEP);
      return;
    }

    if (event.key === "0") {
      event.preventDefault();
      resetTransform();
      return;
    }

    if (event.key === "ArrowLeft") {
      event.preventDefault();
      if (canPan) {
        panBy(KEY_PAN_STEP, 0);
      } else {
        showPrevious();
      }
      return;
    }

    if (event.key === "ArrowRight") {
      event.preventDefault();
      if (canPan) {
        panBy(-KEY_PAN_STEP, 0);
      } else {
        showNext();
      }
      return;
    }

    if (event.key === "ArrowUp") {
      event.preventDefault();
      panBy(0, KEY_PAN_STEP);
      return;
    }

    if (event.key === "ArrowDown") {
      event.preventDefault();
      panBy(0, -KEY_PAN_STEP);
    }
  }

  function handleWheel(event: WheelEvent<HTMLDivElement>) {
    event.preventDefault();
    event.stopPropagation();
    updateScale(scale + (event.deltaY < 0 ? SCALE_STEP : -SCALE_STEP));
  }

  function handlePointerDown(event: PointerEvent<HTMLImageElement>) {
    event.stopPropagation();
    if (!canPan || event.button !== 0) {
      return;
    }

    dragRef.current = {
      offset,
      pointerId: event.pointerId,
      startX: event.clientX,
      startY: event.clientY,
    };
    event.currentTarget.setPointerCapture(event.pointerId);
  }

  function handlePointerMove(event: PointerEvent<HTMLImageElement>) {
    const drag = dragRef.current;
    if (!drag || drag.pointerId !== event.pointerId) {
      return;
    }

    setOffset(
      getClampedOffset({
        x: drag.offset.x + event.clientX - drag.startX,
        y: drag.offset.y + event.clientY - drag.startY,
      }),
    );
  }

  function handlePointerUp(event: PointerEvent<HTMLImageElement>) {
    if (dragRef.current?.pointerId !== event.pointerId) {
      return;
    }

    dragRef.current = null;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
  }

  return (
    <div
      className="image-preview"
      onKeyDown={handleKeyDown}
      onMouseDown={(event) => {
        event.stopPropagation();
        onClose();
      }}
      ref={rootRef}
      tabIndex={-1}
    >
      <div className="image-preview__toolbar" onMouseDown={stopPreviewEvent}>
        <span className="image-preview__counter">
          {currentIndex + 1} / {items.length}
        </span>
        <span className="image-preview__counter">{Math.round(scale * 100)}%</span>
        <button
          aria-label={t.zoomOut}
          onClick={() => updateScale(scale - SCALE_STEP)}
          title={t.zoomOut}
          type="button"
        >
          <ZoomOut size={15} />
        </button>
        <button
          aria-label={t.zoomIn}
          onClick={() => updateScale(scale + SCALE_STEP)}
          title={t.zoomIn}
          type="button"
        >
          <ZoomIn size={15} />
        </button>
        <button aria-label={t.resetView} onClick={resetTransform} title={t.resetView} type="button">
          <RotateCcw size={15} />
        </button>
        <button aria-label={t.cancel} onClick={onClose} title={t.cancel} type="button">
          <X size={16} />
        </button>
      </div>

      {canNavigate ? (
        <>
          <button
            aria-label={t.previousImage}
            className="image-preview__nav image-preview__nav--prev"
            onClick={(event) => {
              event.stopPropagation();
              showPrevious();
            }}
            onMouseDown={stopPreviewEvent}
            title={t.previousImage}
            type="button"
          >
            <ChevronLeft size={20} />
          </button>
          <button
            aria-label={t.nextImage}
            className="image-preview__nav image-preview__nav--next"
            onClick={(event) => {
              event.stopPropagation();
              showNext();
            }}
            onMouseDown={stopPreviewEvent}
            title={t.nextImage}
            type="button"
          >
            <ChevronRight size={20} />
          </button>
        </>
      ) : null}

      <div className="image-preview__stage" onWheel={handleWheel} ref={stageRef}>
        <img
          alt={currentItem.alt}
          draggable={false}
          onDoubleClick={resetTransform}
          onMouseDown={stopPreviewEvent}
          onPointerCancel={handlePointerUp}
          onPointerDown={handlePointerDown}
          onPointerMove={handlePointerMove}
          onPointerUp={handlePointerUp}
          ref={imageRef}
          src={currentItem.src}
          style={{
            cursor: canPan ? "grab" : "zoom-in",
            transform: `translate3d(${offset.x}px, ${offset.y}px, 0) scale(${scale})`,
          }}
        />
      </div>
    </div>
  );
}
