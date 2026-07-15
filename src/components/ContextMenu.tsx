import { useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { clampContextMenuPosition } from "./contextMenuPosition";

export interface ContextMenuItem {
  destructive?: boolean;
  icon: ReactNode;
  id: string;
  label: string;
  onSelect: () => void;
}

interface ContextMenuProps {
  items: ContextMenuItem[];
  onClose: () => void;
  x: number;
  y: number;
}

export function ContextMenu({ items, onClose, x, y }: ContextMenuProps) {
  const menuRef = useRef<HTMLDivElement>(null);
  const [position, setPosition] = useState({ left: x, top: y });

  useLayoutEffect(() => {
    const updatePosition = () => {
      const menu = menuRef.current;
      if (!menu) return;

      const bounds = menu.getBoundingClientRect();
      const nextPosition = clampContextMenuPosition({
        menuHeight: bounds.height,
        menuWidth: bounds.width,
        viewportHeight: window.innerHeight,
        viewportWidth: window.innerWidth,
        x,
        y,
      });

      setPosition((current) =>
        current.left === nextPosition.left && current.top === nextPosition.top
          ? current
          : nextPosition,
      );
    };

    updatePosition();
    window.addEventListener("resize", updatePosition);
    return () => window.removeEventListener("resize", updatePosition);
  }, [items.length, x, y]);

  return (
    <div
      className="context-menu"
      onClick={(event) => event.stopPropagation()}
      onContextMenu={(event) => event.preventDefault()}
      ref={menuRef}
      style={position}
    >
      {items.map((item) => (
        <button
          className={item.destructive ? "is-danger" : ""}
          key={item.id}
          onClick={() => {
            item.onSelect();
            onClose();
          }}
          type="button"
        >
          {item.icon}
          <span>{item.label}</span>
        </button>
      ))}
    </div>
  );
}
