import type { DockEdge, WindowState } from "../types";

export type DockTransitionTarget = "dock" | "main";

type DockRevealAnchor = WindowState & {
  edge: DockEdge;
  savedAt: number;
};

const DOCK_TRANSITION_KEY = "q-note:dock-transition";
const DOCK_GUARD_KEY = "q-note:dock-guard-until";
const DOCK_REVEAL_ANCHOR_KEY = "q-note:dock-reveal-anchor";
const DOCK_GUARD_MS = 800;
const DOCK_REVEAL_ANCHOR_MAX_AGE = 5 * 60 * 1000;

export const DOCK_RETURN_SNAP_EVENT = "q-note-snap-dock-after-return";
export const DOCK_RETURN_SNAP_DELAY = 500;

function isDockEdge(value: unknown): value is DockEdge {
  return value === "left" || value === "right" || value === "top" || value === "bottom";
}

function isFiniteNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

export function setSharedDockGuard() {
  try {
    localStorage.setItem(DOCK_GUARD_KEY, String(Date.now() + DOCK_GUARD_MS));
  } catch {
    // localStorage can be unavailable in unusual webview states; the local ref still guards moves.
  }
}

export function isSharedDockGuardActive() {
  try {
    const expiresAt = Number(localStorage.getItem(DOCK_GUARD_KEY));
    return Number.isFinite(expiresAt) && expiresAt > Date.now();
  } catch {
    return false;
  }
}

export function rememberDockRevealAnchor(edge: DockEdge, state: WindowState | null) {
  if (!state) {
    return;
  }

  try {
    localStorage.setItem(
      DOCK_REVEAL_ANCHOR_KEY,
      JSON.stringify({
        ...state,
        edge,
        savedAt: Date.now(),
      } satisfies DockRevealAnchor),
    );
  } catch {
    // This only improves return placement, so failing silently is fine.
  }
}

export function clearDockRevealAnchor() {
  try {
    localStorage.removeItem(DOCK_REVEAL_ANCHOR_KEY);
  } catch {
    // This state is only used to polish the dock return transition.
  }
}

export function takeDockRevealAnchor() {
  let raw: string | null = null;

  try {
    raw = localStorage.getItem(DOCK_REVEAL_ANCHOR_KEY);
    localStorage.removeItem(DOCK_REVEAL_ANCHOR_KEY);
  } catch {
    return null;
  }

  if (!raw) {
    return null;
  }

  try {
    const value = JSON.parse(raw) as Partial<DockRevealAnchor>;
    if (
      !isDockEdge(value.edge) ||
      !isFiniteNumber(value.x) ||
      !isFiniteNumber(value.y) ||
      !isFiniteNumber(value.width) ||
      !isFiniteNumber(value.height) ||
      !isFiniteNumber(value.savedAt) ||
      Date.now() - value.savedAt > DOCK_REVEAL_ANCHOR_MAX_AGE
    ) {
      return null;
    }

    return value as DockRevealAnchor;
  } catch {
    return null;
  }
}

export function beginDockTransition(target: DockTransitionTarget) {
  const token = `${target}:${Date.now()}:${Math.random().toString(36).slice(2)}`;
  localStorage.setItem(DOCK_TRANSITION_KEY, token);
  return token;
}

export function getActiveDockTransitionTarget(): DockTransitionTarget | null {
  const target = localStorage.getItem(DOCK_TRANSITION_KEY)?.split(":")[0];
  return target === "dock" || target === "main" ? target : null;
}

export function isActiveDockTransition(token: string) {
  return localStorage.getItem(DOCK_TRANSITION_KEY) === token;
}
