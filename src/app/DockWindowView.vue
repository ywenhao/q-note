<script setup lang="ts" vapor>
import type { ContextMenuItem } from "../components/componentTypes";
import CompactDock from "../components/CompactDock.vue";
import ContextMenu from "../components/ContextMenu.vue";
import QMark from "../components/QMark.vue";
import Toast from "../components/Toast.vue";
import type { MenuState } from "../features/menu/useMenuController";
import type { ToastState } from "../hooks/useToast";
import type { Translation } from "../i18n";

defineProps<{
  menu: MenuState | null;
  menuItems: ContextMenuItem[];
  ready: boolean;
  t: Translation;
  toast: ToastState | null;
}>();
const emit = defineEmits<{
  closeMenu: [];
  concealDockIcon: [];
  dockDragEnd: [];
  dockDragMove: [];
  dockDragStart: [];
  openDockMenu: [event: MouseEvent];
  openMain: [];
  revealDockIcon: [];
}>();
</script>

<template>
  <main v-if="!ready" class="dock-shell">
    <QMark class="dock-loading-mark" />
  </main>
  <template v-else>
    <CompactDock
      :t="t"
      @context-menu="emit('openDockMenu', $event)"
      @drag-end="emit('dockDragEnd')"
      @drag-move="emit('dockDragMove')"
      @drag-start="emit('dockDragStart')"
      @hover-end="emit('concealDockIcon')"
      @hover-start="emit('revealDockIcon')"
      @open-main="emit('openMain')"
    />
    <ContextMenu
      v-if="menu"
      :items="menuItems"
      :x="menu.x"
      :y="menu.y"
      @close="emit('closeMenu')"
    />
    <Toast :icon="toast?.icon" :kind="toast?.kind" :message="toast?.message ?? null" />
  </template>
</template>
