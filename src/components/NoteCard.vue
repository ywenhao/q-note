<script setup lang="ts" vapor>
import { computed, onBeforeUnmount, ref, watch } from "vue";
import {
  CheckIcon,
  ChevronLeftIcon,
  CopyIcon,
  FileTextIcon,
  GripHorizontalIcon,
  PaletteIcon,
  PencilIcon,
  PinIcon,
  PinOffIcon,
  Trash2Icon,
} from "../icons";
import type { Translation } from "../i18n";
import { getAttachmentSrc, isImageAttachment } from "../lib/images";
import { NOTE_COLORS, type Note } from "../types";
import type { ImagePreviewItem } from "./componentTypes";
import Icon from "./Icon.vue";
import IconButton from "./IconButton.vue";

const props = defineProps<{
  note: Note;
  shouldSuppressCopy: () => boolean;
  t: Translation;
}>();
const emit = defineEmits<{
  colorChange: [id: string, color: string];
  contextMenu: [event: MouseEvent, noteId: string];
  copy: [note: Note];
  delete: [id: string];
  edit: [note: Note];
  heightChange: [id: string, height: number];
  previewImages: [items: ImagePreviewItem[], index: number];
  togglePin: [id: string];
}>();

const LINE_HEIGHT = 22;
const colorPopover = ref<HTMLDivElement | null>(null);
const textElement = ref<HTMLParagraphElement | null>(null);
const actionsOpen = ref(false);
const draftHeight = ref<number | null>(null);
const paletteOpen = ref(false);
let suppressCopyUntil = 0;

const defaultHeight = computed(() => getDefaultLines(props.note.content) * LINE_HEIGHT);
const textHeight = computed(
  () => draftHeight.value ?? props.note.textHeight ?? defaultHeight.value,
);
const textLines = computed(() => Math.max(1, Math.round(textHeight.value / LINE_HEIGHT)));
const hasText = computed(() => props.note.content.trim().length > 0);
const imageAttachments = computed(() => props.note.attachments.filter(isImageAttachment));
const previewImages = computed<ImagePreviewItem[]>(() =>
  imageAttachments.value.map((attachment) => ({
    alt: attachment.name ?? props.t.addImage,
    id: attachment.id,
    src: getAttachmentSrc(attachment),
  })),
);
const fileAttachments = computed(() =>
  props.note.attachments.filter((attachment) => !isImageAttachment(attachment)),
);
const textStyle = computed(() => ({
  "--note-lines": String(textLines.value),
}));

watch(paletteOpen, (open) => {
  if (open) {
    window.addEventListener("pointerdown", closePaletteOnOutsidePointer);
  } else {
    window.removeEventListener("pointerdown", closePaletteOnOutsidePointer);
  }
});
onBeforeUnmount(() => window.removeEventListener("pointerdown", closePaletteOnOutsidePointer));

function getDefaultLines(content: string) {
  if (!content.trim()) {
    return 1;
  }
  return content.length <= 34 && !content.includes("\n") ? 1 : 2;
}

function closePaletteOnOutsidePointer(event: PointerEvent) {
  if (event.target instanceof Node && colorPopover.value?.contains(event.target)) {
    return;
  }
  paletteOpen.value = false;
}

function beginResize(event: PointerEvent) {
  event.preventDefault();
  event.stopPropagation();
  if (!textElement.value) {
    return;
  }
  const maxHeight = Math.max(
    defaultHeight.value,
    Math.ceil(textElement.value.scrollHeight / LINE_HEIGHT) * LINE_HEIGHT,
  );
  const minHeight = Math.min(defaultHeight.value, maxHeight);
  const startY = event.clientY;
  const startHeight = textHeight.value;

  const handleMove = (moveEvent: PointerEvent) => {
    draftHeight.value = clamp(startHeight + moveEvent.clientY - startY, minHeight, maxHeight);
  };
  const handleUp = (upEvent: PointerEvent) => {
    const nextHeight = clamp(startHeight + upEvent.clientY - startY, minHeight, maxHeight);
    const snappedHeight = clamp(
      Math.ceil(nextHeight / LINE_HEIGHT) * LINE_HEIGHT,
      minHeight,
      maxHeight,
    );
    suppressCopyUntil = Date.now() + 300;
    draftHeight.value = null;
    emit("heightChange", props.note.id, snappedHeight);
    window.removeEventListener("pointermove", handleMove);
  };
  window.addEventListener("pointermove", handleMove);
  window.addEventListener("pointerup", handleUp, { once: true });
}

function handleCardClick() {
  if (Date.now() >= suppressCopyUntil && !props.shouldSuppressCopy()) {
    emit("copy", props.note);
  }
}

function closeActionPanel() {
  actionsOpen.value = false;
  paletteOpen.value = false;
  if (document.activeElement instanceof HTMLElement) {
    document.activeElement.blur();
  }
}

function runAndClose(action: () => void) {
  action();
  closeActionPanel();
}

function clamp(value: number, min: number, max: number) {
  return Math.max(min, Math.min(value, max));
}
</script>

<template>
  <article
    class="note-card"
    :class="{ 'is-pinned': note.pinned }"
    :data-note-id="note.id"
    :style="{ backgroundColor: note.color }"
    @click="handleCardClick"
    @contextmenu="emit('contextMenu', $event, note.id)"
  >
    <span v-if="note.pinned" :aria-label="t.pinned" class="note-card__pin-badge" role="img">
      <Icon :nodes="PinIcon" :size="12" />
    </span>

    <div class="note-card__body">
      <div class="note-card__content">
        <p
          ref="textElement"
          :class="['note-card__text', { 'is-muted': !hasText }]"
          :style="textStyle"
        >
          {{ hasText ? note.content : t.imageOnly }}
        </p>

        <div
          v-if="imageAttachments.length > 0"
          class="note-card__images"
          @click.stop
          @pointerdown.stop
        >
          <button
            v-for="(attachment, index) in imageAttachments.slice(0, 4)"
            :key="attachment.id"
            class="note-card__image"
            :title="attachment.name ?? t.addImage"
            type="button"
            @click.stop="emit('previewImages', previewImages, index)"
          >
            <img :alt="attachment.name ?? t.addImage" :src="getAttachmentSrc(attachment)" />
          </button>
        </div>

        <div v-if="fileAttachments.length > 0" class="note-card__files" @click.stop>
          <span
            v-for="attachment in fileAttachments.slice(0, 3)"
            :key="attachment.id"
            :title="attachment.value"
          >
            <Icon :nodes="FileTextIcon" :size="14" />{{ attachment.name ?? attachment.value }}
          </span>
        </div>
      </div>

      <div
        :class="['note-card__action-dock', { 'is-actions-open': actionsOpen }]"
        @click.stop
        @focus.capture="actionsOpen = true"
        @pointerenter="actionsOpen = true"
        @pointerleave="closeActionPanel"
        @pointerdown.stop
      >
        <button
          :aria-label="t.moreActions"
          class="note-card__actions-trigger"
          :title="t.moreActions"
          type="button"
        >
          <Icon :nodes="ChevronLeftIcon" :size="12" />
        </button>

        <div class="note-card__actions">
          <IconButton
            :active="note.pinned"
            :label="note.pinned ? t.unpin : t.pin"
            subtle
            @click="runAndClose(() => emit('togglePin', note.id))"
          >
            <template #icon>
              <Icon v-if="note.pinned" :nodes="PinOffIcon" :size="16" />
              <Icon v-else :nodes="PinIcon" :size="16" />
            </template>
          </IconButton>
          <IconButton :label="t.edit" subtle @click="runAndClose(() => emit('edit', note))">
            <template #icon><Icon :nodes="PencilIcon" :size="16" /></template>
          </IconButton>
          <div ref="colorPopover" class="color-popover-wrap">
            <IconButton
              :label="t.color"
              subtle
              @click="
                actionsOpen = true;
                paletteOpen = !paletteOpen;
              "
            >
              <template #icon><Icon :nodes="PaletteIcon" :size="16" /></template>
            </IconButton>
            <div v-if="paletteOpen" class="color-popover">
              <button
                v-for="item in NOTE_COLORS"
                :key="item"
                :aria-label="item"
                class="color-swatch"
                :style="{ backgroundColor: item }"
                type="button"
                @click="
                  emit('colorChange', note.id, item);
                  closeActionPanel();
                "
              >
                <Icon v-if="note.color === item" :nodes="CheckIcon" :size="13" />
              </button>
            </div>
          </div>
          <IconButton :label="t.copy" subtle @click="runAndClose(() => emit('copy', note))">
            <template #icon><Icon :nodes="CopyIcon" :size="16" /></template>
          </IconButton>
          <IconButton
            class="is-danger"
            :label="t.delete"
            subtle
            @click="runAndClose(() => emit('delete', note.id))"
          >
            <template #icon><Icon :nodes="Trash2Icon" :size="16" /></template>
          </IconButton>
        </div>
      </div>
    </div>

    <button
      :aria-label="t.resize"
      class="resize-handle"
      :title="t.resize"
      type="button"
      @click.stop
      @pointerdown="beginResize"
    >
      <Icon :nodes="GripHorizontalIcon" :size="16" />
    </button>
  </article>
</template>
