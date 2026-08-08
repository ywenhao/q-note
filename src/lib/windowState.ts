import type { WindowState } from "../types";

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
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
    // Stored values are physical pixels from outerSize(); do not clamp with logical defaults.
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
