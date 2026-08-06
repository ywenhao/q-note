import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch, type Ref } from "vue";
import { clampContextMenuPosition } from "../components/contextMenuPosition";

export interface MenuState {
  noteId?: string;
  x: number;
  y: number;
}

export function useContextMenu() {
  const menu = ref<MenuState | null>(null);

  function closeMenu() {
    menu.value = null;
  }

  function openMenu(event: MouseEvent, noteId?: string) {
    event.preventDefault();
    event.stopPropagation();
    menu.value = {
      noteId,
      x: event.clientX,
      y: event.clientY,
    };
  }

  return { closeMenu, menu, openMenu };
}

interface UseContextMenuPositionOptions {
  itemsLength: () => number;
  menuElement: Ref<HTMLDivElement | null>;
  x: () => number;
  y: () => number;
}

export function useContextMenuPosition(options: UseContextMenuPositionOptions) {
  const position = ref({ left: 0, top: 0 });

  const menuStyle = computed(() => ({
    left: `${position.value.left}px`,
    top: `${position.value.top}px`,
  }));

  async function updatePosition() {
    await nextTick();
    const menu = options.menuElement.value;
    if (!menu) {
      return;
    }

    const bounds = menu.getBoundingClientRect();
    position.value = clampContextMenuPosition({
      menuHeight: bounds.height,
      menuWidth: bounds.width,
      viewportHeight: window.innerHeight,
      viewportWidth: window.innerWidth,
      x: options.x(),
      y: options.y(),
    });
  }

  watch(
    () => [options.x(), options.y(), options.itemsLength()] as const,
    () => {
      void updatePosition();
    },
    { flush: "post" },
  );

  onMounted(() => {
    void updatePosition();
    window.addEventListener("resize", updatePosition);
  });

  onBeforeUnmount(() => {
    window.removeEventListener("resize", updatePosition);
  });

  return { menuStyle, updatePosition };
}
