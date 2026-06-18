import { CompactDock } from "../components/CompactDock";
import { ContextMenu, type ContextMenuItem } from "../components/ContextMenu";
import { QMark } from "../components/QMark";
import { Toast } from "../components/Toast";
import type { MouseEvent } from "react";
import type { MenuState } from "../features/menu/useMenuController";
import type { ToastState } from "../hooks/useToast";
import type { Translation } from "../i18n";

interface DockWindowViewProps {
  menu: MenuState | null;
  menuItems: ContextMenuItem[];
  onCloseMenu: () => void;
  onConcealDockIcon: () => void;
  onDockDragEnd: () => void;
  onDockDragMove: () => void;
  onDockDragStart: () => void;
  onOpenDockMenu: (event: MouseEvent<HTMLButtonElement>) => void;
  onOpenMain: () => void;
  onRevealDockIcon: () => void;
  ready: boolean;
  t: Translation;
  toast: ToastState | null;
}

export function DockWindowView({
  menu,
  menuItems,
  onCloseMenu,
  onConcealDockIcon,
  onDockDragEnd,
  onDockDragMove,
  onDockDragStart,
  onOpenDockMenu,
  onOpenMain,
  onRevealDockIcon,
  ready,
  t,
  toast,
}: DockWindowViewProps) {
  if (!ready) {
    return (
      <main className="dock-shell">
        <QMark className="dock-loading-mark" />
      </main>
    );
  }

  return (
    <>
      <CompactDock
        onContextMenu={onOpenDockMenu}
        onDragEnd={onDockDragEnd}
        onDragMove={onDockDragMove}
        onDragStart={onDockDragStart}
        onHoverEnd={onConcealDockIcon}
        onHoverStart={onRevealDockIcon}
        onOpenMain={onOpenMain}
        t={t}
      />
      {menu ? <ContextMenu items={menuItems} onClose={onCloseMenu} x={menu.x} y={menu.y} /> : null}
      <Toast icon={toast?.icon} kind={toast?.kind} message={toast?.message ?? null} />
    </>
  );
}
