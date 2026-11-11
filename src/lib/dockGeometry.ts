export interface DockPoint {
  x: number;
  y: number;
}

interface PointerDisplay {
  id: string;
  bounds: { origin: DockPoint };
  scaleFactor: number;
}

export function dockScreenPointer(
  sample: { display: string; x: number; y: number },
  displays: readonly PointerDisplay[],
): DockPoint | null {
  const display = displays.find((item) => item.id === sample.display);
  if (!display || display.scaleFactor <= 0) return null;
  return {
    x: display.bounds.origin.x + sample.x / display.scaleFactor,
    y: display.bounds.origin.y + sample.y / display.scaleFactor,
  };
}

export function dockDragOrigin(pointer: DockPoint, grab: DockPoint): DockPoint {
  return { x: pointer.x - grab.x, y: pointer.y - grab.y };
}
