<script setup lang="ts" vapor>
import CompactDock from "../components/CompactDock.vue";
import ContextMenu from "../components/ContextMenu.vue";
import QMark from "../components/QMark.vue";
import Toast from "../components/Toast.vue";
import { useDockWindow } from "./useDockWindow";

const {
  closeMenu,
  concealDockIcon,
  dockMenuItems,
  dragQIcon,
  finishQIconDrag,
  menu,
  moveQIcon,
  openDockMenu,
  openMainFromDockIcon,
  ready,
  revealDockIcon,
  t,
  toast,
} = useDockWindow();
</script>

<template>
  <main v-if="!ready" class="dock-shell">
    <QMark class="dock-loading-mark" />
  </main>
  <template v-else>
    <CompactDock
      :t="t"
      @context-menu="openDockMenu"
      @drag-end="finishQIconDrag"
      @drag-move="moveQIcon"
      @drag-start="dragQIcon"
      @hover-end="concealDockIcon"
      @hover-start="revealDockIcon"
      @open-main="openMainFromDockIcon"
    />
    <ContextMenu v-if="menu" :items="dockMenuItems" :x="menu.x" :y="menu.y" @close="closeMenu" />
    <Toast :icon="toast?.icon" :kind="toast?.kind" :message="toast?.message ?? null" />
  </template>
</template>
