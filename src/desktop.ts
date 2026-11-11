import { cleanup, effect, untracked, type Bounds } from "nsh";
import {
  Application,
  currentWindow,
  displays,
  isAvailable,
  Menu,
  Tray,
  Window,
  windows,
  type MenuItem,
  type WindowRef,
} from "nsh/desktop";
import { DockClip } from "q-note/native";
import type { NoteModel } from "./model";
import { dockScreenPointer, type DockPoint } from "./lib/dockGeometry";
import icon from "../app-icon.png";

export const clamp = (v: number, low: number, high: number) =>
  Math.min(Math.max(v, low), Math.max(low, high));

export function createDesktop(m: NoteModel) {
  let timer: ReturnType<typeof setTimeout> | undefined;
  let initialized = false;
  let pendingBounds: Bounds | null = null;
  let cachedDock: { bounds: Bounds; edge: string | null; at: number } | null =
    null;
  let dockEdge: string | null = null;
  let dockExpanded: Bounds | null = null;
  let dockAnimation: ReturnType<typeof setTimeout> | undefined;
  let dockPosition: DockPoint | null = null;
  let dockTargetExpanded: boolean | null = null;
  let dockDragging = false;
  let dockHeld = false;
  let dockMenuOpen = false;
  let dockHovered = false;
  const main = isAvailable ? currentWindow() : null;

  function area(window?: WindowRef | null) {
    return (
      window?.state()?.display?.visibleBounds ??
      displays().find((d) => d.primary)?.visibleBounds ??
      displays()[0]?.visibleBounds ?? {
        origin: { x: 0, y: 0 },
        size: { width: 1280, height: 800 },
      }
    );
  }

  async function saveBounds() {
    clearTimeout(timer);
    const bounds = pendingBounds ?? main?.state()?.bounds;
    if (!bounds || m.loading()) return;
    pendingBounds = null;
    const scale = main?.state()?.scaleFactor || 1;
    const next = {
      ...m.settings(),
      window: {
        width: Math.round(bounds.size.width * scale),
        height: Math.round(bounds.size.height * scale),
        x: Math.round(bounds.origin.x * scale),
        y: Math.round(bounds.origin.y * scale),
      },
    };
    const { saveSettings } = await import("./lib/storage");
    await saveSettings(next);
    m.settings.set(next);
  }

  async function quit() {
    try {
      await m.secureData();
      Application.quit();
    } catch (error) {
      console.error(error);
      m.notify(m.t().saveFailed);
    }
  }

  function restore() {
    clearTimeout(dockAnimation);
    if (m.docked()) {
      const dock = Window.get("dock");
      if (dockExpanded)
        cachedDock = { bounds: dockExpanded, edge: dockEdge, at: Date.now() };
      dock?.hide();
      m.docked.set(false);
    }
    main?.focus();
  }

  function menu(): MenuItem[] {
    return [
      {
        label: m.settings().alwaysOnTop ? m.t().alwaysOff : m.t().alwaysOn,
        checked: m.settings().alwaysOnTop,
        onSelect: () => {
          void m.changeSettings({ alwaysOnTop: !m.settings().alwaysOnTop });
        },
      },
      {
        label: m.t().switchLanguage,
        onSelect: () => {
          void m.toggleLanguage();
        },
      },
      {
        label: m.docked() ? m.t().switchMainWindow : m.t().switchFloatingBall,
        onSelect: () => {
          if (m.docked()) restore();
          else collapse();
        },
      },
      { type: "separator" },
      {
        label: m.t().quit,
        onSelect: () => {
          void quit();
        },
      },
    ];
  }

  function sync() {
    if (!isAvailable) return;
    for (const window of windows())
      window.set({ level: m.settings().alwaysOnTop ? "floating" : "normal" });
    Tray.set({ icon, tooltip: "Q Note", onClick: restore, menu: menu() });
  }

  async function collapse() {
    if (!isAvailable) return;
    try {
      await saveBounds();
    } catch (error) {
      console.error(error);
      m.notify(m.t().saveFailed);
      return;
    }
    const current = main?.state()?.bounds;
    dockEdge = null;
    const remembered =
      cachedDock && Date.now() - cachedDock.at < 300000 ? cachedDock : null;
    const cachedCenter = remembered && {
      x: remembered.bounds.origin.x + 15,
      y: remembered.bounds.origin.y + 15,
    };
    const cachedDisplay =
      cachedCenter &&
      displays().find(
        (display) =>
          cachedCenter.x >= display.bounds.origin.x &&
          cachedCenter.x <=
            display.bounds.origin.x + display.bounds.size.width &&
          cachedCenter.y >= display.bounds.origin.y &&
          cachedCenter.y <=
            display.bounds.origin.y + display.bounds.size.height,
      );
    const work = cachedDisplay?.visibleBounds ?? area(main);
    dockExpanded = remembered?.bounds ?? {
      origin: {
        x: clamp(
          (current?.origin.x ?? work.origin.x + work.size.width - 50) +
            (current?.size.width ?? 30) -
            30,
          work.origin.x,
          work.origin.x + work.size.width - 30,
        ),
        y: clamp(
          (current?.origin.y ?? work.origin.y) +
            (current?.size.height ?? 30) -
            30,
          work.origin.y,
          work.origin.y + work.size.height - 30,
        ),
      },
      size: { width: 30, height: 30 },
    };
    dockEdge = remembered?.edge ?? null;
    dockDragging = false;
    dockHeld = false;
    dockMenuOpen = false;
    dockHovered = false;
    dockTargetExpanded = null;
    dockExpanded.origin = {
      x: clamp(
        dockExpanded.origin.x,
        work.origin.x,
        work.origin.x + work.size.width - 30,
      ),
      y: clamp(
        dockExpanded.origin.y,
        work.origin.y,
        work.origin.y + work.size.height - 30,
      ),
    };
    if (dockEdge === "left") dockExpanded.origin.x = work.origin.x;
    if (dockEdge === "right")
      dockExpanded.origin.x = work.origin.x + work.size.width - 30;
    if (dockEdge === "top") dockExpanded.origin.y = work.origin.y;
    if (dockEdge === "bottom")
      dockExpanded.origin.y = work.origin.y + work.size.height - 30;
    m.docked.set(true);
    main?.hide();
    setTimeout(() => {
      const dock = Window.get("dock");
      dockPosition = { ...dockExpanded!.origin };
      dock?.setBounds(dockExpanded!);
      dock?.show();
      DockClip.set("", 0);
      if (dockEdge) dockAnimation = setTimeout(() => animateDock(false), 500);
    }, 0);
  }

  function animateDock(expand: boolean) {
    if (
      dockHeld ||
      dockDragging ||
      dockMenuOpen ||
      dockTargetExpanded === expand
    )
      return;
    clearTimeout(dockAnimation);
    if (!dockExpanded || !dockEdge) return;
    const dock = Window.get("dock");
    if (!dock) return;
    const base = dockExpanded.origin;
    const hidden = {
      x: base.x + (dockEdge === "left" ? -15 : dockEdge === "right" ? 15 : 0),
      y: base.y + (dockEdge === "top" ? -15 : dockEdge === "bottom" ? 15 : 0),
    };
    dockTargetExpanded = expand;
    const from = dockPosition ?? dock.state()?.bounds.origin ?? base;
    const to = expand ? base : hidden;
    const start = Date.now();
    const frame = () => {
      const ratio = Math.min(1, (Date.now() - start) / 130);
      const eased = 1 - (1 - ratio) ** 3;
      const point = {
        x: from.x + (to.x - from.x) * eased,
        y: from.y + (to.y - from.y) * eased,
      };
      dockPosition = point;
      dock.setBounds({ origin: point });
      DockClip.set(
        dockEdge!,
        Math.abs(
          dockEdge === "left" || dockEdge === "right"
            ? point.x - base.x
            : point.y - base.y,
        ),
      );
      if (ratio < 1) dockAnimation = setTimeout(frame, 16);
    };
    frame();
  }

  function beginDockDrag() {
    dockDragging = true;
    dockTargetExpanded = null;
    clearTimeout(dockAnimation);
    DockClip.set("", 0);
    if (dockExpanded) {
      dockPosition = { ...dockExpanded.origin };
      Window.get("dock")?.setBounds(dockExpanded);
    }
    dockEdge = null;
    return (
      dockExpanded?.origin ??
      Window.get("dock")?.state()?.bounds.origin ?? { x: 0, y: 0 }
    );
  }

  function moveDock(origin: { x: number; y: number }) {
    const dock = Window.get("dock");
    const target = displays().find(
      (d) =>
        origin.x + 15 >= d.bounds.origin.x &&
        origin.x + 15 <= d.bounds.origin.x + d.bounds.size.width &&
        origin.y + 15 >= d.bounds.origin.y &&
        origin.y + 15 <= d.bounds.origin.y + d.bounds.size.height,
    );
    const work = target?.visibleBounds ?? area(dock);
    dockExpanded = {
      origin: {
        x: clamp(origin.x, work.origin.x, work.origin.x + work.size.width - 30),
        y: clamp(
          origin.y,
          work.origin.y,
          work.origin.y + work.size.height - 30,
        ),
      },
      size: { width: 30, height: 30 },
    };
    dockPosition = { ...dockExpanded.origin };
    dock?.setBounds(dockExpanded);
  }

  function endDockDrag() {
    dockHeld = false;
    dockDragging = false;
    if (!dockExpanded) return;
    const work = area(Window.get("dock"));
    const origin = dockExpanded.origin;
    const edges = [
      { edge: "left", distance: Math.abs(origin.x - work.origin.x) },
      {
        edge: "right",
        distance: Math.abs(work.origin.x + work.size.width - 30 - origin.x),
      },
      { edge: "top", distance: Math.abs(origin.y - work.origin.y) },
      {
        edge: "bottom",
        distance: Math.abs(work.origin.y + work.size.height - 30 - origin.y),
      },
    ].sort((a, b) => a.distance - b.distance);
    dockEdge = edges[0]!.distance <= 28 ? edges[0]!.edge : null;
    if (dockEdge === "left") origin.x = work.origin.x;
    if (dockEdge === "right") origin.x = work.origin.x + work.size.width - 30;
    if (dockEdge === "top") origin.y = work.origin.y;
    if (dockEdge === "bottom") origin.y = work.origin.y + work.size.height - 30;
    Window.get("dock")?.setBounds(dockExpanded);
    dockPosition = { ...origin };
    if (dockEdge) animateDock(dockHovered);
  }

  function dockPointer() {
    const sample = DockClip.pointer();
    return sample ? dockScreenPointer(JSON.parse(sample), displays()) : null;
  }

  async function openDockMenu() {
    if (dockMenuOpen || dockDragging) return;
    dockMenuOpen = true;
    dockTargetExpanded = null;
    clearTimeout(dockAnimation);
    try {
      // A child window's surface coordinates are not the main window's menu coordinates.
      await Menu.popup(menu());
    } finally {
      dockMenuOpen = false;
      if (m.docked()) animateDock(dockHovered);
    }
  }

  if (main) {
    main.set({
      frame: "none",
      minSize: { width: 300, height: 450 },
      maxSize: { width: 1000, height: 800 },
      maximizable: false,
      closeOnPress: false,
    });
    m.setFlushWindow(saveBounds);
    m.setDesktopSync(sync);
    cleanup(
      main.on("closePress", () => {
        main.hide();
      }),
    );
    cleanup(
      main.on("boundsChange", ({ bounds }) => {
        if (!initialized) return;
        pendingBounds = bounds;
        clearTimeout(timer);
        timer = setTimeout(() => {
          void saveBounds().catch((error) => {
            console.error(error);
            m.notify(m.t().saveFailed);
          });
        }, 300);
      }),
    );
    effect(() => {
      if (m.loading()) return;
      if (!initialized)
        untracked(() => {
          const work = area(main);
          const saved = m.settings().window;
          const scale =
            main.state()?.scaleFactor ||
            displays().find((d) => d.primary)?.scaleFactor ||
            1;
          const size = {
            width: clamp(
              saved ? saved.width / scale : 300,
              300,
              Math.min(1000, work.size.width),
            ),
            height: clamp(
              saved ? saved.height / scale : 450,
              450,
              Math.min(800, work.size.height),
            ),
          };
          main.setBounds({
            origin: {
              x: clamp(
                work.origin.x + work.size.width - size.width - 40,
                work.origin.x,
                work.origin.x + work.size.width - size.width,
              ),
              y: clamp(
                work.origin.y + (work.size.height - size.height) / 2,
                work.origin.y,
                work.origin.y + work.size.height - size.height,
              ),
            },
            size,
          });
          main.show();
          initialized = true;
        });
      sync();
    });
  }
  cleanup(() => {
    clearTimeout(timer);
    clearTimeout(dockAnimation);
    if (isAvailable) Tray.set(null);
  });
  return {
    menu,
    restore,
    collapse,
    beginDockDrag,
    moveDock,
    endDockDrag,
    dockPointer,
    openDockMenu,
    holdDock: () => {
      dockHeld = true;
      dockTargetExpanded = null;
      clearTimeout(dockAnimation);
    },
    releaseDock: () => {
      dockHeld = false;
      if (m.docked()) animateDock(dockHovered);
    },
    expandDock: () => {
      dockHovered = true;
      animateDock(true);
    },
    hideDock: () => {
      dockHovered = false;
      animateDock(false);
    },
    area,
  };
}

export type DesktopController = ReturnType<typeof createDesktop>;
