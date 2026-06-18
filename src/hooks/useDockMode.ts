import { emitTo, listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef, type MutableRefObject } from "react";
import {
  DOCK_RETURN_SNAP_EVENT,
  DOCK_RETURN_SNAP_DELAY,
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
  settingsRef: MutableRefObject<AppSettings>;
}

interface DockReturnSnapPayload {
  edge: DockEdge;
  token: string;
}

function isDockEdge(value: unknown): value is DockEdge {
  return value === "left" || value === "right" || value === "top" || value === "bottom";
}

export function useDockMode({
  currentWindowLabel,
  persistSettings,
  settingsRef,
}: UseDockModeOptions) {
  const dockGuardRef = useRef(false);
  const dockGuardTimerRef = useRef<number | null>(null);
  const dockReturnSnapTimerRef = useRef<number | null>(null);
  const dockDragRef = useRef(false);
  const dockDragMovePendingRef = useRef(false);
  const dockDragSessionRef = useRef<QIconDragSession | null>(null);
  const dockTransitionRef = useRef(false);
  const iconWindowRef = useRef<WindowState | null>(null);

  const setDockGuard = useCallback(() => {
    dockGuardRef.current = true;
    setSharedDockGuard();
    if (dockGuardTimerRef.current) {
      window.clearTimeout(dockGuardTimerRef.current);
    }

    dockGuardTimerRef.current = window.setTimeout(() => {
      dockGuardRef.current = false;
      dockGuardTimerRef.current = null;
    }, 700);
  }, []);

  const persistIconSnap = useCallback(
    async (edge: DockEdge) => {
      setDockGuard();
      iconWindowRef.current = await snapQIconWindow(edge);
      await persistSettings({
        docked: true,
        dockEdge: edge,
      });
    },
    [persistSettings, setDockGuard],
  );

  const restoreDock = useCallback(
    async (options: { keepFull?: boolean; preserveRevealAnchor?: boolean } = {}) => {
      if (dockTransitionRef.current) {
        return;
      }

      if (!options.preserveRevealAnchor) {
        clearDockRevealAnchor();
      }

      dockTransitionRef.current = true;
      const token = beginDockTransition("main");

      try {
        setDockGuard();
        await persistSettings({
          docked: false,
          dockEdge: null,
          keepFullMain: options.keepFull ?? settingsRef.current.keepFullMain,
        });
        await showMainWindow(settingsRef.current.window, settingsRef.current.alwaysOnTop);

        if (isActiveDockTransition(token) && !settingsRef.current.docked) {
          await hideDockWindow();
        } else if (getActiveDockTransitionTarget() === "dock") {
          await hideMainWindow();
        }
      } finally {
        dockTransitionRef.current = false;
      }
    },
    [persistSettings, setDockGuard, settingsRef],
  );

  const collapseToQIcon = useCallback(
    async (options: { useRevealAnchor?: boolean } = {}) => {
      if (dockTransitionRef.current) {
        return;
      }

      dockTransitionRef.current = true;
      const token = beginDockTransition("dock");
      const revealAnchor = options.useRevealAnchor ? takeDockRevealAnchor() : null;

      try {
        const snapshot = await captureWindowState(MAIN_WINDOW_LABEL);
        setDockGuard();
        await persistSettings({
          docked: true,
          dockEdge: revealAnchor?.edge ?? null,
          keepFullMain: false,
          window: snapshot ?? settingsRef.current.window,
        });
        iconWindowRef.current = await showDockWindow(
          revealAnchor ?? snapshot ?? settingsRef.current.window,
          settingsRef.current.alwaysOnTop,
        );

        if (isActiveDockTransition(token) && settingsRef.current.docked) {
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
        dockTransitionRef.current = false;
      }
    },
    [persistIconSnap, persistSettings, setDockGuard, settingsRef],
  );

  const toggleDockOnEdge = useCallback(async () => {
    if (settingsRef.current.docked) {
      await restoreDock({ keepFull: true });
      return;
    }

    await collapseToQIcon();
  }, [collapseToQIcon, restoreDock, settingsRef]);

  const dragQIcon = useCallback(async () => {
    if (dockDragRef.current) {
      return;
    }

    clearDockRevealAnchor();
    dockDragRef.current = true;
    dockDragSessionRef.current = await beginQIconDrag();
    if (!dockDragSessionRef.current) {
      dockDragRef.current = false;
    }
  }, []);

  const moveQIcon = useCallback(async () => {
    const session = dockDragSessionRef.current;
    if (!dockDragRef.current || !session || dockDragMovePendingRef.current) {
      return;
    }

    dockDragMovePendingRef.current = true;
    try {
      await moveQIconDrag(session);
    } finally {
      dockDragMovePendingRef.current = false;
    }
  }, []);

  const finishQIconDrag = useCallback(async () => {
    if (!dockDragRef.current) {
      return;
    }

    const session = dockDragSessionRef.current;
    dockDragRef.current = false;
    dockDragMovePendingRef.current = false;
    dockDragSessionRef.current = null;
    if (!settingsRef.current.docked) {
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

    iconWindowRef.current = snapshot;
    clearDockRevealAnchor();
    await persistSettings({ dockEdge: null });
  }, [persistIconSnap, persistSettings, settingsRef]);

  const revealDockIcon = useCallback(async () => {
    if (dockDragRef.current) {
      return;
    }

    const edge = settingsRef.current.dockEdge;
    if (!edge) {
      return;
    }

    setDockGuard();
    iconWindowRef.current = await revealQIconWindow(edge);
  }, [setDockGuard, settingsRef]);

  const concealDockIcon = useCallback(async () => {
    if (dockDragRef.current) {
      return;
    }

    const edge = settingsRef.current.dockEdge;
    if (!edge) {
      return;
    }

    setDockGuard();
    iconWindowRef.current = await snapQIconWindow(edge);
  }, [setDockGuard, settingsRef]);

  const openMainFromDockIcon = useCallback(async () => {
    const edge = settingsRef.current.dockEdge;
    if (edge) {
      clearDockRevealAnchor();
      setDockGuard();
      iconWindowRef.current = await revealQIconWindow(edge);
      rememberDockRevealAnchor(edge, iconWindowRef.current);
      await restoreDock({ keepFull: true, preserveRevealAnchor: true });
      return;
    }

    clearDockRevealAnchor();
    await restoreDock({ keepFull: true });
  }, [restoreDock, setDockGuard, settingsRef]);

  useEffect(
    () => () => {
      if (dockGuardTimerRef.current) {
        window.clearTimeout(dockGuardTimerRef.current);
      }
      if (dockReturnSnapTimerRef.current) {
        window.clearTimeout(dockReturnSnapTimerRef.current);
      }
    },
    [],
  );

  useEffect(() => {
    if (currentWindowLabel !== DOCK_WINDOW_LABEL) {
      return;
    }

    let disposed = false;
    let unlisten: (() => void) | null = null;

    void listen<DockReturnSnapPayload>(DOCK_RETURN_SNAP_EVENT, (event) => {
      const { edge, token } = event.payload;
      if (!isDockEdge(edge) || typeof token !== "string") {
        return;
      }

      if (dockReturnSnapTimerRef.current) {
        window.clearTimeout(dockReturnSnapTimerRef.current);
      }

      dockReturnSnapTimerRef.current = window.setTimeout(() => {
        dockReturnSnapTimerRef.current = null;
        if (!isActiveDockTransition(token) || !settingsRef.current.docked || dockDragRef.current) {
          return;
        }

        void persistIconSnap(edge);
      }, DOCK_RETURN_SNAP_DELAY);
    }).then((handler) => {
      if (disposed) {
        handler();
        return;
      }

      unlisten = handler;
    });

    return () => {
      disposed = true;
      unlisten?.();
      if (dockReturnSnapTimerRef.current) {
        window.clearTimeout(dockReturnSnapTimerRef.current);
        dockReturnSnapTimerRef.current = null;
      }
    };
  }, [currentWindowLabel, persistIconSnap, settingsRef]);

  return {
    collapseToQIcon,
    concealDockIcon,
    dockDragRef,
    dockGuardRef,
    dragQIcon,
    finishQIconDrag,
    openMainFromDockIcon,
    persistIconSnap,
    restoreDock,
    revealDockIcon,
    setDockGuard,
    toggleDockOnEdge,
    moveQIcon,
  };
}
