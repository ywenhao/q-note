<script setup lang="ts" vapor>
import { onBeforeUnmount, ref } from "vue";
import type { Translation } from "../i18n";
import QMark from "./QMark.vue";

defineProps<{ t: Translation }>();
const emit = defineEmits<{
  contextMenu: [event: MouseEvent];
  dragEnd: [];
  dragMove: [];
  dragStart: [];
  hoverEnd: [];
  hoverStart: [];
  openMain: [];
}>();

const DRAG_THRESHOLD = 4;
interface PointerState {
  dragging: boolean;
  pointerId: number;
  startX: number;
  startY: number;
}

const pointer = ref<PointerState | null>(null);
let dragEndCleanup: (() => void) | null = null;

function cleanupDragEndListeners() {
  dragEndCleanup?.();
  dragEndCleanup = null;
}

function finishDrag() {
  if (!pointer.value?.dragging) {
    return;
  }
  pointer.value = null;
  cleanupDragEndListeners();
  emit("dragEnd");
}

function listenForDragEnd() {
  cleanupDragEndListeners();
  const handleEnd = () => finishDrag();
  window.addEventListener("pointerup", handleEnd, { capture: true, once: true });
  window.addEventListener("mouseup", handleEnd, { capture: true, once: true });
  dragEndCleanup = () => {
    window.removeEventListener("pointerup", handleEnd, { capture: true });
    window.removeEventListener("mouseup", handleEnd, { capture: true });
  };
}

function handlePointerDown(event: PointerEvent) {
  if (event.button !== 0) {
    return;
  }
  cleanupDragEndListeners();
  pointer.value = {
    dragging: false,
    pointerId: event.pointerId,
    startX: event.clientX,
    startY: event.clientY,
  };
  (event.currentTarget as HTMLButtonElement).setPointerCapture(event.pointerId);
}

function handlePointerMove(event: PointerEvent) {
  const state = pointer.value;
  if (!state || event.pointerId !== state.pointerId) {
    return;
  }
  if (state.dragging) {
    emit("dragMove");
    return;
  }
  if (Math.hypot(event.clientX - state.startX, event.clientY - state.startY) < DRAG_THRESHOLD) {
    return;
  }
  state.dragging = true;
  listenForDragEnd();
  emit("dragStart");
  emit("dragMove");
}

function handlePointerUp(event: PointerEvent) {
  const state = pointer.value;
  if (!state || event.pointerId !== state.pointerId) {
    return;
  }
  pointer.value = null;
  const button = event.currentTarget as HTMLButtonElement;
  if (button.hasPointerCapture(event.pointerId)) {
    button.releasePointerCapture(event.pointerId);
  }
  cleanupDragEndListeners();
  if (state.dragging) {
    emit("dragEnd");
  } else {
    emit("openMain");
  }
}

function handlePointerCancel() {
  if (pointer.value?.dragging) {
    return;
  }
  pointer.value = null;
  cleanupDragEndListeners();
}

onBeforeUnmount(cleanupDragEndListeners);
</script>

<template>
  <main class="dock-shell">
    <button
      :aria-label="t.switchMainWindow"
      class="dock-button"
      :title="t.switchMainWindow"
      type="button"
      @contextmenu="emit('contextMenu', $event)"
      @mouseenter="emit('hoverStart')"
      @mouseleave="emit('hoverEnd')"
      @pointercancel="handlePointerCancel"
      @pointerdown="handlePointerDown"
      @pointermove="handlePointerMove"
      @pointerup="handlePointerUp"
    >
      <QMark />
    </button>
  </main>
</template>
