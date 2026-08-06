<script setup lang="ts" vapor>
import { computed, nextTick, onMounted, ref, watch } from "vue";
import {
  ChevronLeftIcon,
  ChevronRightIcon,
  RotateCcwIcon,
  XIcon,
  ZoomInIcon,
  ZoomOutIcon,
} from "../icons";
import type { Translation } from "../i18n";
import type { ImagePreviewItem } from "./componentTypes";
import Icon from "./Icon.vue";

const props = withDefaults(
  defineProps<{
    initialIndex?: number;
    items: ImagePreviewItem[];
    t: Translation;
  }>(),
  { initialIndex: 0 },
);
const emit = defineEmits<{ close: [] }>();

interface PanOffset {
  x: number;
  y: number;
}
interface DragState {
  offset: PanOffset;
  pointerId: number;
  startX: number;
  startY: number;
}

const MIN_SCALE = 0.5;
const MAX_SCALE = 6;
const SCALE_STEP = 0.25;
const KEY_PAN_STEP = 36;
const PAN_OVERSCROLL = 80;

const imageElement = ref<HTMLImageElement | null>(null);
const rootElement = ref<HTMLDivElement | null>(null);
const stageElement = ref<HTMLDivElement | null>(null);
const drag = ref<DragState | null>(null);
const currentIndex = ref(clamp(props.initialIndex, 0, props.items.length - 1));
const offset = ref<PanOffset>({ x: 0, y: 0 });
const scale = ref(1);

const currentItem = computed(() => props.items[currentIndex.value]);
const canNavigate = computed(() => props.items.length > 1);
const canPan = computed(() => scale.value > 1);
const imageStyle = computed(() => ({
  cursor: canPan.value ? "grab" : "zoom-in",
  transform: `translate3d(${offset.value.x}px, ${offset.value.y}px, 0) scale(${scale.value})`,
}));

onMounted(() => void nextTick(() => rootElement.value?.focus()));
watch(
  () => [props.initialIndex, props.items.length] as const,
  ([initialIndex]) => {
    currentIndex.value = clamp(initialIndex, 0, props.items.length - 1);
    resetTransform();
  },
);

function clamp(value: number, min: number, max: number) {
  return Math.max(min, Math.min(value, max));
}

function stopPreviewEvent(event: Event) {
  event.stopPropagation();
}

function resetTransform() {
  drag.value = null;
  offset.value = { x: 0, y: 0 };
  scale.value = 1;
}

function getClampedOffset(nextOffset: PanOffset, nextScale = scale.value) {
  const stage = stageElement.value;
  const image = imageElement.value;
  if (!stage || !image || nextScale <= 1) {
    return { x: 0, y: 0 };
  }
  const maxX = Math.max((image.clientWidth * nextScale - stage.clientWidth) / 2, 0);
  const maxY = Math.max((image.clientHeight * nextScale - stage.clientHeight) / 2, 0);
  return {
    x: clamp(nextOffset.x, -maxX - PAN_OVERSCROLL, maxX + PAN_OVERSCROLL),
    y: clamp(nextOffset.y, -maxY - PAN_OVERSCROLL, maxY + PAN_OVERSCROLL),
  };
}

function setOffset(nextOffset: PanOffset) {
  offset.value = nextOffset;
}

function updateScale(nextScale: number) {
  scale.value = clamp(nextScale, MIN_SCALE, MAX_SCALE);
  setOffset(getClampedOffset(offset.value, scale.value));
}

function panBy(deltaX: number, deltaY: number) {
  if (canPan.value) {
    setOffset(getClampedOffset({ x: offset.value.x + deltaX, y: offset.value.y + deltaY }));
  }
}

function showImage(index: number) {
  currentIndex.value = (index + props.items.length) % props.items.length;
  resetTransform();
}

function showPrevious() {
  if (canNavigate.value) {
    showImage(currentIndex.value - 1);
  }
}

function showNext() {
  if (canNavigate.value) {
    showImage(currentIndex.value + 1);
  }
}

function handleKeyDown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.preventDefault();
    emit("close");
  } else if (event.key === "[" || event.key === "PageUp") {
    event.preventDefault();
    showPrevious();
  } else if (event.key === "]" || event.key === "PageDown") {
    event.preventDefault();
    showNext();
  } else if (event.key === "+" || event.key === "=") {
    event.preventDefault();
    updateScale(scale.value + SCALE_STEP);
  } else if (event.key === "-") {
    event.preventDefault();
    updateScale(scale.value - SCALE_STEP);
  } else if (event.key === "0") {
    event.preventDefault();
    resetTransform();
  } else if (event.key === "ArrowLeft") {
    event.preventDefault();
    if (canPan.value) {
      panBy(KEY_PAN_STEP, 0);
    } else {
      showPrevious();
    }
  } else if (event.key === "ArrowRight") {
    event.preventDefault();
    if (canPan.value) {
      panBy(-KEY_PAN_STEP, 0);
    } else {
      showNext();
    }
  } else if (event.key === "ArrowUp") {
    event.preventDefault();
    panBy(0, KEY_PAN_STEP);
  } else if (event.key === "ArrowDown") {
    event.preventDefault();
    panBy(0, -KEY_PAN_STEP);
  }
}

function handleWheel(event: WheelEvent) {
  event.preventDefault();
  event.stopPropagation();
  updateScale(scale.value + (event.deltaY < 0 ? SCALE_STEP : -SCALE_STEP));
}

function handlePointerDown(event: PointerEvent) {
  event.stopPropagation();
  if (!canPan.value || event.button !== 0) {
    return;
  }
  drag.value = {
    offset: { ...offset.value },
    pointerId: event.pointerId,
    startX: event.clientX,
    startY: event.clientY,
  };
  (event.currentTarget as HTMLImageElement).setPointerCapture(event.pointerId);
}

function handlePointerMove(event: PointerEvent) {
  const state = drag.value;
  if (!state || state.pointerId !== event.pointerId) {
    return;
  }
  setOffset(
    getClampedOffset({
      x: state.offset.x + event.clientX - state.startX,
      y: state.offset.y + event.clientY - state.startY,
    }),
  );
}

function handlePointerUp(event: PointerEvent) {
  if (drag.value?.pointerId !== event.pointerId) {
    return;
  }
  drag.value = null;
  const image = event.currentTarget as HTMLImageElement;
  if (image.hasPointerCapture(event.pointerId)) {
    image.releasePointerCapture(event.pointerId);
  }
}
</script>

<template>
  <div
    v-if="currentItem"
    ref="rootElement"
    class="image-preview"
    tabindex="-1"
    @keydown="handleKeyDown"
    @mousedown.stop="emit('close')"
  >
    <div class="image-preview__toolbar" @mousedown="stopPreviewEvent">
      <span class="image-preview__counter">{{ currentIndex + 1 }} / {{ items.length }}</span>
      <span class="image-preview__counter">{{ Math.round(scale * 100) }}%</span>
      <button
        :aria-label="t.zoomOut"
        :title="t.zoomOut"
        type="button"
        @click="updateScale(scale - SCALE_STEP)"
      >
        <Icon :nodes="ZoomOutIcon" :size="15" />
      </button>
      <button
        :aria-label="t.zoomIn"
        :title="t.zoomIn"
        type="button"
        @click="updateScale(scale + SCALE_STEP)"
      >
        <Icon :nodes="ZoomInIcon" :size="15" />
      </button>
      <button :aria-label="t.resetView" :title="t.resetView" type="button" @click="resetTransform">
        <Icon :nodes="RotateCcwIcon" :size="15" />
      </button>
      <button :aria-label="t.cancel" :title="t.cancel" type="button" @click="emit('close')">
        <Icon :nodes="XIcon" :size="16" />
      </button>
    </div>

    <template v-if="canNavigate">
      <button
        :aria-label="t.previousImage"
        class="image-preview__nav image-preview__nav--prev"
        :title="t.previousImage"
        type="button"
        @click.stop="showPrevious"
        @mousedown="stopPreviewEvent"
      >
        <Icon :nodes="ChevronLeftIcon" :size="20" />
      </button>
      <button
        :aria-label="t.nextImage"
        class="image-preview__nav image-preview__nav--next"
        :title="t.nextImage"
        type="button"
        @click.stop="showNext"
        @mousedown="stopPreviewEvent"
      >
        <Icon :nodes="ChevronRightIcon" :size="20" />
      </button>
    </template>

    <div ref="stageElement" class="image-preview__stage" @wheel="handleWheel">
      <img
        ref="imageElement"
        :alt="currentItem.alt"
        :draggable="false"
        :src="currentItem.src"
        :style="imageStyle"
        @dblclick="resetTransform"
        @mousedown="stopPreviewEvent"
        @pointercancel="handlePointerUp"
        @pointerdown="handlePointerDown"
        @pointermove="handlePointerMove"
        @pointerup="handlePointerUp"
      />
    </div>
  </div>
</template>
