import {
  PhysicalPosition,
  PhysicalSize,
  Window,
  currentMonitor,
  getCurrentWindow,
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
const SNAP_THRESHOLD = 28;
const MAIN_MIN_SIZE = new PhysicalSize(DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT);
export const MAIN_WINDOW_LABEL = "main";
export const DOCK_WINDOW_LABEL = "dock";

function clamp(value: number, min: number, max: number) {
  return Math.max(min, Math.min(value, max));
}

async function getWindowByLabel(label: string) {
  return Window.getByLabel(label);
}

export async function applyAlwaysOnTop(enabled: boolean) {
  if (!isTauriRuntime()) {
    return;
  }

  const windows = await Promise.all([
    getWindowByLabel(MAIN_WINDOW_LABEL),
    getWindowByLabel(DOCK_WINDOW_LABEL),
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
    return;
  }

  await window.setSize(new PhysicalSize(state.width, state.height));
  await window.setPosition(new PhysicalPosition(state.x, state.y));
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

  const monitor = await currentMonitor();
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
    await window.setPosition(new PhysicalPosition(nextState.x, nextState.y));
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

export async function ensureEditorRoom() {
  if (!isTauriRuntime()) {
    return null;
  }

  const snapshot = await captureWindowState();
  if (!snapshot) {
    return null;
  }

  const nextWidth = Math.max(snapshot.width, 960);
  const nextHeight = Math.max(snapshot.height, 720);

  if (nextWidth !== snapshot.width || nextHeight !== snapshot.height) {
    await getCurrentWindow().setSize(new PhysicalSize(nextWidth, nextHeight));
  }

  return snapshot;
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
  const nearLeft = position.x <= area.left + SNAP_THRESHOLD;
  const nearRight = position.x + size.width >= area.right - SNAP_THRESHOLD;
  const nearTop = position.y <= area.top + SNAP_THRESHOLD;
  const nearBottom = position.y + size.height >= area.bottom - SNAP_THRESHOLD;

  if (nearLeft) return "left";
  if (nearRight) return "right";
  if (nearTop) return "top";
  if (nearBottom) return "bottom";
  return null;
}

export async function snapQIconWindow(edge: DockEdge) {
  if (!isTauriRuntime()) {
    return null;
  }

  const window = await getWindowByLabel(DOCK_WINDOW_LABEL);
  if (!window) {
    return null;
  }

  const [monitor, position, size] = await Promise.all([
    currentMonitor(),
    window.outerPosition(),
    window.outerSize(),
  ]);

  if (!monitor) {
    return null;
  }

  const area = getWorkArea(monitor);
  const centerX = position.x + size.width / 2;
  const centerY = position.y + size.height / 2;

  const x =
    edge === "left"
      ? area.left
      : edge === "right"
        ? area.right - DOCK_WINDOW_SIZE
        : clamp(centerX - DOCK_WINDOW_SIZE / 2, area.left, area.right - DOCK_WINDOW_SIZE);

  const y =
    edge === "top"
      ? area.top
      : edge === "bottom"
        ? area.bottom - DOCK_WINDOW_SIZE
        : clamp(centerY - DOCK_WINDOW_SIZE / 2, area.top, area.bottom - DOCK_WINDOW_SIZE);

  await window.setDecorations(false);
  await window.setShadow(false);
  await window.setMinSize(new PhysicalSize(DOCK_WINDOW_SIZE, DOCK_WINDOW_SIZE));
  await window.setMaxSize(new PhysicalSize(DOCK_WINDOW_SIZE, DOCK_WINDOW_SIZE));
  await window.setSize(new PhysicalSize(DOCK_WINDOW_SIZE, DOCK_WINDOW_SIZE));
  const nextState = {
    width: DOCK_WINDOW_SIZE,
    height: DOCK_WINDOW_SIZE,
    x: Math.round(x),
    y: Math.round(y),
  };
  await window.setPosition(new PhysicalPosition(nextState.x, nextState.y));

  return nextState;
}
