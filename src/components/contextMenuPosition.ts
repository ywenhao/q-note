export interface ContextMenuPositionInput {
  margin?: number;
  menuHeight: number;
  menuWidth: number;
  viewportHeight: number;
  viewportWidth: number;
  x: number;
  y: number;
}

export interface ContextMenuPosition {
  left: number;
  top: number;
}

function clampAxis(requested: number, menuSize: number, viewportSize: number, margin: number) {
  if (viewportSize < menuSize + margin * 2) {
    return 0;
  }

  return Math.min(Math.max(requested, margin), viewportSize - menuSize - margin);
}

export function clampContextMenuPosition({
  margin = 8,
  menuHeight,
  menuWidth,
  viewportHeight,
  viewportWidth,
  x,
  y,
}: ContextMenuPositionInput): ContextMenuPosition {
  return {
    left: clampAxis(x, menuWidth, viewportWidth, margin),
    top: clampAxis(y, menuHeight, viewportHeight, margin),
  };
}
