import type { IconNode } from "../icons";

export interface ContextMenuItem {
  destructive?: boolean;
  icon: IconNode;
  id: string;
  label: string;
  onSelect: () => void;
}

export interface ImagePreviewItem {
  alt: string;
  id: string;
  src: string;
}
