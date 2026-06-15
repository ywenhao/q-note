import { invoke } from "@tauri-apps/api/core";
import {
  PhysicalPosition,
  PhysicalSize,
  Window,
  currentMonitor,
  getCurrentWindow,
  monitorFromPoint,
  type Monitor,
} from "@tauri-apps/api/window";
import {
  DOCK_WINDOW_SIZE,
  DEFAULT_WINDOW_HEIGHT,
  DEFAULT_WINDOW_WIDTH,
  type DockEdge,
  type WindowState,
} from "../types";
import { isTauriRuntime } from "./env";

const DOCK_MARGIN = 12;
const MAIN_START_MARGIN = 40;
const SNAP_THRESHOLD = 28;
const EDGE_PEEK_SIZE = DOCK_WINDOW_SIZE / 2;
const DOCK_SLIDE_DURATION = 130;
const MAIN_MIN_SIZE = new PhysicalSize(DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT);
export const MAIN_WINDOW_LABEL = "main";
export const DOCK_WINDOW_LABEL = "dock";
export const EDITOR_WINDOW_LABEL = "editor";

const EDITOR_REQUEST_KEY = "q-note:editor-request";

function clamp(value: number, min: number, max: number) {
  return Math.max(min, Math.min(value, max));
}

function getRightCenterMainState(width: number, height: number, monitor: Monitor): WindowState {
  const area = getWorkArea(monitor);

  return {
    width,
    height,
    x: Math.round(clamp(area.right - width - MAIN_START_MARGIN, area.left, area.right - width)),
    y: Math.round(
      clamp(area.top + (area.bottom - area.top - height) / 2, area.top, area.bottom - height),
    ),
  };
}

async function getWindowByLabel(label: string) {
  return Window.getByLabel(label);
}

async function getMonitorForState(state: WindowState | null) {
  if (!state) {
    return currentMonitor();
  }

  const centerX = Math.round(state.x + state.width / 2);
  const centerY = Math.round(state.y + state.height / 2);
  return (await monitorFromPoint(centerX, centerY)) ?? currentMonitor();
}

async function getMonitorForDockWindow(
  edge: DockEdge,
  position: PhysicalPosition,
  size: PhysicalSize,
) {
  const probeX =
    edge === "left"
      ? position.x + size.width * 0.75
      : edge === "right"
        ? position.x + size.width * 0.25
        : position.x + size.width / 2;
  const probeY =
    edge === "top"
      ? position.y + size.height * 0.75
      : edge === "bottom"
        ? position.y + size.height * 0.25
        : position.y + size.height / 2;

  return (await monitorFromPoint(Math.round(probeX), Math.round(probeY))) ?? currentMonitor();
}

export async function applyAlwaysOnTop(enabled: boolean) {
  if (!isTauriRuntime()) {
    return;
  }

  const windows = await Promise.all([
    getWindowByLabel(MAIN_WINDOW_LABEL),
    getWindowByLabel(DOCK_WINDOW_LABEL),
    getWindowByLabel(EDITOR_WINDOW_LABEL),
  ]);

  await Promise.all(
    windows
      .filter((window): window is Window => Boolean(window))
      .map((window) => window.setAlwaysOnTop(enabled)),
  );
}

export async function captureWindowState(label?: string): Promise<WindowState | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const window = label ? await getWindowByLabel(label) : getCurrentWindow();
  if (!window) {
    return null;
  }

  const [size, position] = await Promise.all([window.outerSize(), window.outerPosition()]);

  return {
    width: size.width,
    height: size.height,
    x: position.x,
    y: position.y,
  };
}

export async function restoreWindowState(state: WindowState | null, label = MAIN_WINDOW_LABEL) {
  if (!isTauriRuntime()) {
    return;
  }

  const window = await getWindowByLabel(label);
  if (!window) {
    return;
  }

  await window.setDecorations(false);
  await window.setShadow(true);
  await window.setMinSize(MAIN_MIN_SIZE);

  if (!state) {
    await window.setSize(new PhysicalSize(DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT));
    const monitor = await currentMonitor();
    if (monitor) {
      const nextState = getRightCenterMainState(
        DEFAULT_WINDOW_WIDTH,
        DEFAULT_WINDOW_HEIGHT,
        monitor,
      );
      await window.setPosition(new PhysicalPosition(nextState.x, nextState.y));
    }
    return;
  }

  await window.setSize(new PhysicalSize(state.width, state.height));
  await window.setPosition(new PhysicalPosition(state.x, state.y));
}

export async function positionMainWindowAtStartup(state: WindowState | null) {
  if (!isTauriRuntime()) {
    return;
  }

  const window = await getWindowByLabel(MAIN_WINDOW_LABEL);
  const monitor = await currentMonitor();
  if (!window) {
    return;
  }

  const width = state?.width ?? DEFAULT_WINDOW_WIDTH;
  const height = state?.height ?? DEFAULT_WINDOW_HEIGHT;

  await window.setDecorations(false);
  await window.setShadow(true);
  await window.setMinSize(MAIN_MIN_SIZE);
  await window.setSize(new PhysicalSize(width, height));

  if (monitor) {
    const nextState = getRightCenterMainState(width, height, monitor);
    await window.setPosition(new PhysicalPosition(nextState.x, nextState.y));
  }

  await window.show();
  await window.setFocus();
}

export async function showMainWindow(state: WindowState | null, alwaysOnTop: boolean) {
  if (!isTauriRuntime()) {
    return;
  }

  await restoreWindowState(state, MAIN_WINDOW_LABEL);

  const window = await getWindowByLabel(MAIN_WINDOW_LABEL);
  if (!window) {
    return;
  }

  await window.setAlwaysOnTop(alwaysOnTop);
  await window.unminimize();
  await window.show();
  await window.setFocus();
}

export async function hideMainWindow() {
  if (!isTauriRuntime()) {
    return;
  }

  await (await getWindowByLabel(MAIN_WINDOW_LABEL))?.hide();
}

function getDefaultDockState(monitor: Monitor): WindowState {
  const area = getWorkArea(monitor);

  return {
    width: DOCK_WINDOW_SIZE,
    height: DOCK_WINDOW_SIZE,
    x: Math.round(area.right - DOCK_WINDOW_SIZE - DOCK_MARGIN),
    y: Math.round(area.bottom - DOCK_WINDOW_SIZE - DOCK_MARGIN),
  };
}

function getDockEdgeState(
  edge: DockEdge,
  monitor: Monitor,
  position: PhysicalPosition,
  size: PhysicalSize,
  hidden: boolean,
): WindowState {
  const area = getWorkArea(monitor);
  const centerX = position.x + size.width / 2;
  const centerY = position.y + size.height / 2;
  const halfHidden = hidden ? EDGE_PEEK_SIZE : 0;

  const x =
    edge === "left"
      ? area.left - halfHidden
      : edge === "right"
        ? area.right - DOCK_WINDOW_SIZE + halfHidden
        : clamp(centerX - DOCK_WINDOW_SIZE / 2, area.left, area.right - DOCK_WINDOW_SIZE);

  const y =
    edge === "top"
      ? area.top - halfHidden
      : edge === "bottom"
        ? area.bottom - DOCK_WINDOW_SIZE + halfHidden
        : clamp(centerY - DOCK_WINDOW_SIZE / 2, area.top, area.bottom - DOCK_WINDOW_SIZE);

  return {
    width: DOCK_WINDOW_SIZE,
    height: DOCK_WINDOW_SIZE,
    x: Math.round(x),
    y: Math.round(y),
  };
}

async function moveDockWindow(window: Window, state: WindowState, animate = false) {
  if (!animate) {
    await window.setPosition(new PhysicalPosition(state.x, state.y));
    return;
  }

  const start = await window.outerPosition();
  const deltaX = state.x - start.x;
  const deltaY = state.y - start.y;

  if (deltaX === 0 && deltaY === 0) {
    return;
  }

  await new Promise<void>((resolve) => {
    const startedAt = performance.now();
    const step = () => {
      void (async () => {
        const progress = clamp((performance.now() - startedAt) / DOCK_SLIDE_DURATION, 0, 1);
        const eased = 1 - Math.pow(1 - progress, 3);
        await window.setPosition(
          new PhysicalPosition(
            Math.round(start.x + deltaX * eased),
            Math.round(start.y + deltaY * eased),
          ),
        );

        if (progress < 1) {
          requestAnimationFrame(step);
          return;
        }

        resolve();
      })();
    };

    requestAnimationFrame(step);
  });
}

function normalizeDockState(state: WindowState | null, monitor: Monitor): WindowState {
  if (!state) {
    return getDefaultDockState(monitor);
  }

  const area = getWorkArea(monitor);
  const centerX = state.x + state.width / 2;
  const centerY = state.y + state.height / 2;
  const centerInWorkArea =
    centerX >= area.left && centerX <= area.right && centerY >= area.top && centerY <= area.bottom;

  if (!centerInWorkArea) {
    return getDefaultDockState(monitor);
  }

  return {
    width: DOCK_WINDOW_SIZE,
    height: DOCK_WINDOW_SIZE,
    x: Math.round(clamp(centerX - DOCK_WINDOW_SIZE / 2, area.left, area.right - DOCK_WINDOW_SIZE)),
    y: Math.round(clamp(centerY - DOCK_WINDOW_SIZE / 2, area.top, area.bottom - DOCK_WINDOW_SIZE)),
  };
}

export async function applyQIconWindow(state: WindowState | null): Promise<WindowState | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const window = await getWindowByLabel(DOCK_WINDOW_LABEL);
  if (!window) {
    return null;
  }

  const monitor = await getMonitorForState(state);
  const nextState = monitor
    ? normalizeDockState(state, monitor)
    : state
      ? {
          width: DOCK_WINDOW_SIZE,
          height: DOCK_WINDOW_SIZE,
          x: state.x,
          y: state.y,
        }
      : null;

  await window.setDecorations(false);
  await window.setShadow(false);
  await window.setMinSize(new PhysicalSize(DOCK_WINDOW_SIZE, DOCK_WINDOW_SIZE));
  await window.setMaxSize(new PhysicalSize(DOCK_WINDOW_SIZE, DOCK_WINDOW_SIZE));
  await window.setSize(new PhysicalSize(DOCK_WINDOW_SIZE, DOCK_WINDOW_SIZE));

  if (nextState) {
    await moveDockWindow(window, nextState);
  }

  return nextState;
}

export async function showDockWindow(state: WindowState | null, alwaysOnTop: boolean) {
  if (!isTauriRuntime()) {
    return null;
  }

  const nextState = await applyQIconWindow(state);
  const window = await getWindowByLabel(DOCK_WINDOW_LABEL);
  if (!window) {
    return nextState;
  }

  await window.setAlwaysOnTop(alwaysOnTop);
  await window.show();
  return nextState;
}

export async function hideDockWindow() {
  if (!isTauriRuntime()) {
    return;
  }

  await (await getWindowByLabel(DOCK_WINDOW_LABEL))?.hide();
}

export async function startQIconDrag() {
  if (!isTauriRuntime()) {
    return;
  }

  const window = await getWindowByLabel(DOCK_WINDOW_LABEL);
  await window?.startDragging();
}

export async function startMainWindowDrag() {
  if (!isTauriRuntime()) {
    return;
  }

  await getCurrentWindow().startDragging();
}

export async function openEditorWindow(noteId: string | null, alwaysOnTop: boolean) {
  if (!isTauriRuntime()) {
    return;
  }

  localStorage.setItem(
    EDITOR_REQUEST_KEY,
    JSON.stringify({
      noteId,
      requestedAt: Date.now(),
    }),
  );

  await invoke("open_editor_window", {
    alwaysOnTop,
    noteId,
  });
}

export function readPendingEditorNoteId() {
  const raw = localStorage.getItem(EDITOR_REQUEST_KEY);
  if (!raw) {
    return null;
  }

  try {
    const value = JSON.parse(raw) as { noteId?: unknown };
    return typeof value.noteId === "string" ? value.noteId : null;
  } catch {
    return null;
  }
}

function getWorkArea(monitor: Monitor) {
  return {
    left: monitor.workArea.position.x,
    top: monitor.workArea.position.y,
    right: monitor.workArea.position.x + monitor.workArea.size.width,
    bottom: monitor.workArea.position.y + monitor.workArea.size.height,
  };
}

export async function detectSnapEdge(label?: string): Promise<DockEdge | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const window = label ? await getWindowByLabel(label) : getCurrentWindow();
  if (!window) {
    return null;
  }

  const [monitor, size, position] = await Promise.all([
    currentMonitor(),
    window.outerSize(),
    window.outerPosition(),
  ]);

  if (!monitor) {
    return null;
  }

  const area = getWorkArea(monitor);
  const distances: Array<[DockEdge, number]> = [
    ["left", Math.abs(position.x - area.left)],
    ["right", Math.abs(area.right - (position.x + size.width))],
    ["top", Math.abs(position.y - area.top)],
    ["bottom", Math.abs(area.bottom - (position.y + size.height))],
  ];
  const candidates = distances.filter(([, distance]) => distance <= SNAP_THRESHOLD);

  candidates.sort((a, b) => a[1] - b[1]);
  return candidates[0]?.[0] ?? null;
}

export async function snapQIconWindow(edge: DockEdge) {
  if (!isTauriRuntime()) {
    return null;
  }

  const window = await getWindowByLabel(DOCK_WINDOW_LABEL);
  if (!window) {
    return null;
  }

  const [position, size] = await Promise.all([window.outerPosition(), window.outerSize()]);
  const monitor = await getMonitorForDockWindow(edge, position, size);

  if (!monitor) {
    return null;
  }

  const nextState = getDockEdgeState(edge, monitor, position, size, true);

  await window.setDecorations(false);
  await window.setShadow(false);
  await window.setMinSize(new PhysicalSize(DOCK_WINDOW_SIZE, DOCK_WINDOW_SIZE));
  await window.setMaxSize(new PhysicalSize(DOCK_WINDOW_SIZE, DOCK_WINDOW_SIZE));
  await window.setSize(new PhysicalSize(DOCK_WINDOW_SIZE, DOCK_WINDOW_SIZE));
  await moveDockWindow(window, nextState, true);

  return nextState;
}

export async function revealQIconWindow(edge: DockEdge) {
  if (!isTauriRuntime()) {
    return null;
  }

  const window = await getWindowByLabel(DOCK_WINDOW_LABEL);
  if (!window) {
    return null;
  }

  const [position, size] = await Promise.all([window.outerPosition(), window.outerSize()]);
  const monitor = await getMonitorForDockWindow(edge, position, size);

  if (!monitor) {
    return null;
  }

  const nextState = getDockEdgeState(edge, monitor, position, size, false);
  await moveDockWindow(window, nextState, true);

  return nextState;
}
