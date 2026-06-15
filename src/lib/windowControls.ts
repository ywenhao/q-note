import {
  PhysicalPosition,
  PhysicalSize,
  currentMonitor,
  getCurrentWindow,
  type Monitor,
} from "@tauri-apps/api/window";
import {
  DEFAULT_WINDOW_HEIGHT,
  DEFAULT_WINDOW_WIDTH,
  type DockEdge,
  type WindowState,
} from "../types";
import { isTauriRuntime } from "./env";

const DOCK_SIZE = 56;
const SNAP_THRESHOLD = 28;
const MAIN_MIN_SIZE = new PhysicalSize(DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT);

function clamp(value: number, min: number, max: number) {
  return Math.max(min, Math.min(value, max));
}

export async function applyAlwaysOnTop(enabled: boolean) {
  if (!isTauriRuntime()) {
    return;
  }

  await getCurrentWindow().setAlwaysOnTop(enabled);
}

export async function captureWindowState(): Promise<WindowState | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const window = getCurrentWindow();
  const [size, position] = await Promise.all([window.outerSize(), window.outerPosition()]);

  return {
    width: size.width,
    height: size.height,
    x: position.x,
    y: position.y,
  };
}

export async function restoreWindowState(state: WindowState | null) {
  if (!isTauriRuntime()) {
    return;
  }

  const window = getCurrentWindow();
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

export async function restoreWindowFromQIcon(
  fullState: WindowState | null,
  iconState: WindowState | null,
) {
  if (!isTauriRuntime()) {
    return;
  }

  const window = getCurrentWindow();
  const [monitor, currentPosition, currentSize] = await Promise.all([
    currentMonitor(),
    window.outerPosition(),
    window.outerSize(),
  ]);
  const area = monitor ? getWorkArea(monitor) : null;
  const width = fullState?.width ?? DEFAULT_WINDOW_WIDTH;
  const height = fullState?.height ?? DEFAULT_WINDOW_HEIGHT;
  const icon = iconState ?? {
    width: currentSize.width,
    height: currentSize.height,
    x: currentPosition.x,
    y: currentPosition.y,
  };
  const centerX = icon.x + icon.width / 2;
  const centerY = icon.y + icon.height / 2;

  await window.setDecorations(false);
  await window.setShadow(true);
  await window.setMinSize(MAIN_MIN_SIZE);
  await window.setSize(new PhysicalSize(width, height));

  if (!area) {
    await window.setPosition(new PhysicalPosition(icon.x, icon.y));
    return;
  }

  const nearLeft = icon.x <= area.left + SNAP_THRESHOLD;
  const nearRight = icon.x + icon.width >= area.right - SNAP_THRESHOLD;
  const nearTop = icon.y <= area.top + SNAP_THRESHOLD;
  const nearBottom = icon.y + icon.height >= area.bottom - SNAP_THRESHOLD;
  const x = nearLeft
    ? area.left
    : nearRight
      ? area.right - width
      : clamp(centerX - width / 2, area.left, area.right - width);
  const y = nearTop
    ? area.top
    : nearBottom
      ? area.bottom - height
      : clamp(centerY - height / 2, area.top, area.bottom - height);

  await window.setPosition(new PhysicalPosition(Math.round(x), Math.round(y)));
}

export async function applyQIconWindow(state: WindowState | null) {
  if (!isTauriRuntime()) {
    return;
  }

  const window = getCurrentWindow();
  await window.setDecorations(false);
  await window.setShadow(false);
  await window.setMinSize(new PhysicalSize(DOCK_SIZE, DOCK_SIZE));
  await window.setSize(new PhysicalSize(DOCK_SIZE, DOCK_SIZE));

  if (state) {
    await window.setPosition(new PhysicalPosition(state.x, state.y));
  }
}

export async function startQIconDrag() {
  if (!isTauriRuntime()) {
    return;
  }

  await getCurrentWindow().startDragging();
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

export async function detectSnapEdge(): Promise<DockEdge | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const window = getCurrentWindow();
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

  const window = getCurrentWindow();
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
        ? area.right - DOCK_SIZE
        : clamp(centerX - DOCK_SIZE / 2, area.left, area.right - DOCK_SIZE);

  const y =
    edge === "top"
      ? area.top
      : edge === "bottom"
        ? area.bottom - DOCK_SIZE
        : clamp(centerY - DOCK_SIZE / 2, area.top, area.bottom - DOCK_SIZE);

  await window.setDecorations(false);
  await window.setShadow(false);
  await window.setMinSize(new PhysicalSize(DOCK_SIZE, DOCK_SIZE));
  await window.setSize(new PhysicalSize(DOCK_SIZE, DOCK_SIZE));
  const nextState = {
    width: DOCK_SIZE,
    height: DOCK_SIZE,
    x: Math.round(x),
    y: Math.round(y),
  };
  await window.setPosition(new PhysicalPosition(nextState.x, nextState.y));

  return nextState;
}
