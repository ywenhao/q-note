import { emitTo, listen } from "@tauri-apps/api/event";
import { onBeforeUnmount, onMounted, ref, type Ref } from "vue";
import {
  DOCK_RETURN_SNAP_DELAY,
  DOCK_RETURN_SNAP_EVENT,
  beginDockTransition,
  clearDockRevealAnchor,
  getActiveDockTransitionTarget,
  isActiveDockTransition,
  rememberDockRevealAnchor,
  setSharedDockGuard,
  takeDockRevealAnchor,
} from "../lib/dockPersistence";
import {
  DOCK_WINDOW_LABEL,
  MAIN_WINDOW_LABEL,
  beginQIconDrag,
  captureWindowState,
  detectSnapEdge,
  hideDockWindow,
  hideMainWindow,
  moveQIconDrag,
  revealQIconWindow,
  showDockWindow,
  showMainWindow,
  snapQIconWindow,
  type QIconDragSession,
} from "../lib/windowControls";
import type { AppSettings, DockEdge, WindowState } from "../types";

interface UseDockModeOptions {
  currentWindowLabel: string;
  persistSettings: (patch: Partial<AppSettings>) => Promise<void>;
  settings: Ref<AppSettings>;
}

interface DockReturnSnapPayload {
  edge: DockEdge;
  token: string;
}

function isDockEdge(value: unknown): value is DockEdge {
  return value === "left" || value === "right" || value === "top" || value === "bottom";
}

export function useDockMode(options: UseDockModeOptions) {
  const dockGuard = ref(false);
  const dockDrag = ref(false);
  let dockGuardTimer: number | null = null;
  let dockReturnSnapTimer: number | null = null;
  let dockDragMovePending = false;
  let dockDragSession: QIconDragSession | null = null;
  let dockTransition = false;
  let iconWindow: WindowState | null = null;
  let unlistenReturnSnap: (() => void) | null = null;
  let disposed = false;

  function setDockGuard() {
    dockGuard.value = true;
    setSharedDockGuard();
    if (dockGuardTimer) {
      window.clearTimeout(dockGuardTimer);
    }
    dockGuardTimer = window.setTimeout(() => {
      dockGuard.value = false;
      dockGuardTimer = null;
    }, 700);
  }

  async function persistIconSnap(edge: DockEdge) {
    setDockGuard();
    iconWindow = await snapQIconWindow(edge);
    await options.persistSettings({ docked: true, dockEdge: edge });
  }

  async function restoreDock(
    restoreOptions: { keepFull?: boolean; preserveRevealAnchor?: boolean } = {},
  ) {
    if (dockTransition) {
      return;
    }
    if (!restoreOptions.preserveRevealAnchor) {
      clearDockRevealAnchor();
    }
    dockTransition = true;
    const token = beginDockTransition("main");
    try {
      setDockGuard();
      await options.persistSettings({
        docked: false,
        dockEdge: null,
        keepFullMain: restoreOptions.keepFull ?? options.settings.value.keepFullMain,
      });
      await showMainWindow(options.settings.value.window, options.settings.value.alwaysOnTop);
      if (isActiveDockTransition(token) && !options.settings.value.docked) {
        await hideDockWindow();
      } else if (getActiveDockTransitionTarget() === "dock") {
        await hideMainWindow();
      }
    } finally {
      dockTransition = false;
    }
  }

  async function collapseToQIcon(collapseOptions: { useRevealAnchor?: boolean } = {}) {
    if (dockTransition) {
      return;
    }
    dockTransition = true;
    const token = beginDockTransition("dock");
    const revealAnchor = collapseOptions.useRevealAnchor ? takeDockRevealAnchor() : null;
    try {
      const snapshot = await captureWindowState(MAIN_WINDOW_LABEL);
      setDockGuard();
      await options.persistSettings({
        docked: true,
        dockEdge: revealAnchor?.edge ?? null,
        keepFullMain: false,
        window: snapshot ?? options.settings.value.window,
      });
      iconWindow = await showDockWindow(
        revealAnchor ?? snapshot ?? options.settings.value.window,
        options.settings.value.alwaysOnTop,
      );
      if (isActiveDockTransition(token) && options.settings.value.docked) {
        if (revealAnchor) {
          await emitTo(DOCK_WINDOW_LABEL, DOCK_RETURN_SNAP_EVENT, {
            edge: revealAnchor.edge,
            token,
          } satisfies DockReturnSnapPayload);
        }
        await hideMainWindow();
      } else if (getActiveDockTransitionTarget() === "main") {
        await hideDockWindow();
      }
    } finally {
      dockTransition = false;
    }
  }

  async function toggleDockOnEdge() {
    if (options.settings.value.docked) {
      await restoreDock({ keepFull: true });
    } else {
      await collapseToQIcon();
    }
  }

  async function dragQIcon() {
    if (dockDrag.value) {
      return;
    }
    clearDockRevealAnchor();
    dockDrag.value = true;
    dockDragSession = await beginQIconDrag();
    if (!dockDragSession) {
      dockDrag.value = false;
    }
  }

  async function moveQIcon() {
    if (!dockDrag.value || !dockDragSession || dockDragMovePending) {
      return;
    }
    dockDragMovePending = true;
    try {
      await moveQIconDrag(dockDragSession);
    } finally {
      dockDragMovePending = false;
    }
  }

  async function finishQIconDrag() {
    if (!dockDrag.value) {
      return;
    }
    const session = dockDragSession;
    dockDrag.value = false;
    dockDragMovePending = false;
    dockDragSession = null;
    if (!options.settings.value.docked) {
      return;
    }
    if (session) {
      await moveQIconDrag(session);
    }
    const [edge, snapshot] = await Promise.all([
      detectSnapEdge(DOCK_WINDOW_LABEL),
      captureWindowState(DOCK_WINDOW_LABEL),
    ]);
    if (!snapshot) {
      clearDockRevealAnchor();
      return;
    }
    if (edge) {
      await persistIconSnap(edge);
      return;
    }
    iconWindow = snapshot;
    clearDockRevealAnchor();
    await options.persistSettings({ dockEdge: null });
  }

  async function revealDockIcon() {
    const edge = options.settings.value.dockEdge;
    if (!dockDrag.value && edge) {
      setDockGuard();
      iconWindow = await revealQIconWindow(edge);
    }
  }

  async function concealDockIcon() {
    const edge = options.settings.value.dockEdge;
    if (!dockDrag.value && edge) {
      setDockGuard();
      iconWindow = await snapQIconWindow(edge);
    }
  }

  async function openMainFromDockIcon() {
    const edge = options.settings.value.dockEdge;
    if (edge) {
      clearDockRevealAnchor();
      setDockGuard();
      iconWindow = await revealQIconWindow(edge);
      rememberDockRevealAnchor(edge, iconWindow);
      await restoreDock({ keepFull: true, preserveRevealAnchor: true });
      return;
    }
    clearDockRevealAnchor();
    await restoreDock({ keepFull: true });
  }

  onMounted(async () => {
    if (options.currentWindowLabel !== DOCK_WINDOW_LABEL) {
      return;
    }
    const cleanup = await listen<DockReturnSnapPayload>(DOCK_RETURN_SNAP_EVENT, (event) => {
      const { edge, token } = event.payload;
      if (!isDockEdge(edge) || typeof token !== "string") {
        return;
      }
      if (dockReturnSnapTimer) {
        window.clearTimeout(dockReturnSnapTimer);
      }
      dockReturnSnapTimer = window.setTimeout(() => {
        dockReturnSnapTimer = null;
        if (isActiveDockTransition(token) && options.settings.value.docked && !dockDrag.value) {
          void persistIconSnap(edge);
        }
      }, DOCK_RETURN_SNAP_DELAY);
    });
    if (disposed) {
      cleanup();
    } else {
      unlistenReturnSnap = cleanup;
    }
  });

  onBeforeUnmount(() => {
    disposed = true;
    unlistenReturnSnap?.();
    if (dockGuardTimer) {
      window.clearTimeout(dockGuardTimer);
    }
    if (dockReturnSnapTimer) {
      window.clearTimeout(dockReturnSnapTimer);
    }
  });

  return {
    collapseToQIcon,
    concealDockIcon,
    dockDrag,
    dockGuard,
    dragQIcon,
    finishQIconDrag,
    moveQIcon,
    openMainFromDockIcon,
    persistIconSnap,
    restoreDock,
    revealDockIcon,
    setDockGuard,
    toggleDockOnEdge,
  };
}
