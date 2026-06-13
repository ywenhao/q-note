import type { ReactNode } from "react";

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
  return (
    <div
      className="context-menu"
      onClick={(event) => event.stopPropagation()}
      onContextMenu={(event) => event.preventDefault()}
      style={{ left: x, top: y }}
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
