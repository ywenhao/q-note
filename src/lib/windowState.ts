import type { WindowState } from "../types";

export interface WindowExtent {
  width: number;
  height: number;
}

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function clampExtent(value: number, min: number, max: number) {
  return Math.round(Math.max(min, Math.min(value, Math.max(min, max))));
}

export function getEffectiveMaxWindowSize(
  maxSize: WindowExtent,
  workAreaSize?: WindowExtent | null,
): WindowExtent {
  if (!workAreaSize) {
    return maxSize;
  }

  return {
    width: Math.max(1, Math.min(maxSize.width, workAreaSize.width)),
    height: Math.max(1, Math.min(maxSize.height, workAreaSize.height)),
  };
}

export function clampPhysicalWindowSize(
  size: WindowExtent,
  minSize: WindowExtent,
  maxSize: WindowExtent,
): WindowExtent {
  return {
    width: clampExtent(size.width, minSize.width, maxSize.width),
    height: clampExtent(size.height, minSize.height, maxSize.height),
  };
}

export function normalizeWindowState(value: unknown): WindowState | null {
  if (!isObject(value)) {
    return null;
  }

  const width = Number(value.width);
  const height = Number(value.height);
  const x = Number(value.x);
  const y = Number(value.y);
  if (!Number.isFinite(width) || !Number.isFinite(height) || width <= 0 || height <= 0) {
    return null;
  }

  return {
    // Stored values are physical pixels from outerSize(); clamp to logical max on restore.
    width: Math.round(width),
    height: Math.round(height),
    x: Number.isFinite(x) ? Math.round(x) : 0,
    y: Number.isFinite(y) ? Math.round(y) : 0,
  };
}

export function windowSizeMatches(left: WindowState | null, right: WindowState | null) {
  if (!left || !right) {
    return left === right;
  }

  return left.width === right.width && left.height === right.height;
}
