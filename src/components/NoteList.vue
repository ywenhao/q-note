<script setup lang="ts" vapor>
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { useDraggable, type DraggableEvent } from "vue-draggable-plus";
import type { Translation } from "../i18n";
import type { Note } from "../types";
import type { ImagePreviewItem } from "./componentTypes";
import EmptyState from "./EmptyState.vue";
import NoteCard from "./NoteCard.vue";

const props = defineProps<{
  notes: Note[];
  t: Translation;
}>();
const emit = defineEmits<{
  colorChange: [id: string, color: string];
  contextMenu: [event: MouseEvent, noteId: string];
  copy: [note: Note];
  delete: [id: string];
  edit: [note: Note];
  heightChange: [id: string, textHeight: number];
  newNote: [];
  previewImages: [items: ImagePreviewItem[], index: number];
  reorder: [draggedId: string, targetId: string, placement: "before" | "after"];
  togglePin: [id: string];
}>();

const listElement = ref<HTMLElement | null>(null);
const activeId = ref<string | null>(null);
const noteIds = computed(() => props.notes.map((note) => note.id));
let suppressCopyUntil = 0;

function shouldSuppressCopy() {
  return Boolean(activeId.value) || Date.now() < suppressCopyUntil;
}

function clearDragState() {
  activeId.value = null;
  suppressCopyUntil = Date.now() + 400;
  document.body.classList.remove("is-sorting-note");
}

function handleDragStart(event: DraggableEvent<Note>) {
  activeId.value = event.item.dataset.noteId ?? null;
  suppressCopyUntil = Date.now() + 800;
  document.body.classList.add("is-sorting-note");
}

function handleDragEnd(event: DraggableEvent<Note>) {
  const draggedId = activeId.value;
  const oldIndex = event.oldIndex;
  const newIndex = event.newIndex;
  if (draggedId && oldIndex !== undefined && newIndex !== undefined && oldIndex !== newIndex) {
    const targetId = noteIds.value[newIndex];
    if (targetId && targetId !== draggedId) {
      emit("reorder", draggedId, targetId, oldIndex < newIndex ? "after" : "before");
    }
  }
  clearDragState();
}

function forwardColorChange(id: string, color: string) {
  emit("colorChange", id, color);
}

function forwardContextMenu(event: MouseEvent, noteId: string) {
  emit("contextMenu", event, noteId);
}

function forwardHeightChange(id: string, height: number) {
  emit("heightChange", id, height);
}

function forwardPreviewImages(items: ImagePreviewItem[], index: number) {
  emit("previewImages", items, index);
}

const draggable = useDraggable<Note>(listElement, {
  animation: 160,
  dragClass: "is-drag-overlay",
  fallbackOnBody: true,
  fallbackTolerance: 6,
  filter: "button, input, textarea, a, .note-card__images, .note-card__files",
  forceFallback: true,
  ghostClass: "is-dragging",
  immediate: false,
  onEnd: handleDragEnd,
  onStart: handleDragStart,
  preventOnFilter: false,
});

onMounted(() => {
  if (listElement.value) {
    draggable.start(listElement.value);
  }
});

onBeforeUnmount(() => {
  draggable.destroy();
  document.body.classList.remove("is-sorting-note");
});
</script>

<template>
  <EmptyState v-if="notes.length === 0" :t="t" @new-note="emit('newNote')" />
  <section v-show="notes.length > 0" ref="listElement" class="note-list">
    <NoteCard
      v-for="note in notes"
      :key="note.id"
      :note="note"
      :should-suppress-copy="shouldSuppressCopy"
      :t="t"
      @color-change="forwardColorChange"
      @context-menu="forwardContextMenu"
      @copy="emit('copy', $event)"
      @delete="emit('delete', $event)"
      @edit="emit('edit', $event)"
      @height-change="forwardHeightChange"
      @preview-images="forwardPreviewImages"
      @toggle-pin="emit('togglePin', $event)"
    />
  </section>
</template>
