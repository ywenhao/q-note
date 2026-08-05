<script setup lang="ts" vapor>
import { getCurrentWindow } from "@tauri-apps/api/window";
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { FileTextIcon, FolderIcon, ImagePlusIcon, Link2Icon, Trash2Icon, XIcon } from "../icons";
import type { Translation } from "../i18n";
import { createId, isTauriRuntime } from "../lib/env";
import {
  getAttachmentSrc,
  isImageAttachment,
  isLikelyImagePath,
  readFileAsDataUrl,
  resolveDraggedFileUrls,
  resolveDraggedImageUrls,
} from "../lib/images";
import { createNoteDraft } from "../lib/updateDraft";
import {
  DEFAULT_NOTE_COLOR,
  NOTE_COLORS,
  type Note,
  type NoteAttachment,
  type NoteDraft,
} from "../types";
import type { ImagePreviewItem } from "./componentTypes";
import Icon from "./Icon.vue";
import IconButton from "./IconButton.vue";
import ImagePreview from "./ImagePreview.vue";

const props = withDefaults(
  defineProps<{
    initialDraft?: NoteDraft | null;
    mode?: "modal" | "window";
    note: Note | null;
    t: Translation;
  }>(),
  { initialDraft: null, mode: "modal" },
);

const emit = defineEmits<{
  cancel: [];
  draftChange: [draft: NoteDraft];
  dragStart: [event: PointerEvent];
  save: [draft: NoteDraft];
}>();

const fileInput = ref<HTMLInputElement | null>(null);
const attachments = ref<NoteAttachment[]>([]);
const color = ref<string>(DEFAULT_NOTE_COLOR);
const content = ref("");
const dragging = ref(false);
const mediaValue = ref("");
const previewImageIndex = ref<number | null>(null);
const pinned = ref(false);
const isWindowMode = computed(() => props.mode === "window");
const imageAttachments = computed(() => attachments.value.filter(isImageAttachment));
const previewImages = computed<ImagePreviewItem[]>(() =>
  imageAttachments.value.map((attachment) => ({
    alt: attachment.name ?? props.t.addImage,
    id: attachment.id,
    src: getAttachmentSrc(attachment),
  })),
);

let unlistenDragDrop: (() => void) | null = null;
let disposed = false;

watch(
  () => [props.initialDraft, props.note] as const,
  () => {
    const draft = props.initialDraft ?? createNoteDraft(props.note);
    attachments.value = [...draft.attachments];
    color.value = draft.color;
    content.value = draft.content;
    mediaValue.value = "";
    pinned.value = draft.pinned;
    previewImageIndex.value = null;
  },
  { immediate: true },
);

watch(
  [attachments, color, content, pinned],
  () => {
    emit("draftChange", {
      attachments: attachments.value,
      color: color.value,
      content: content.value,
      pinned: pinned.value,
    });
  },
  { deep: true },
);

onMounted(async () => {
  if (!isTauriRuntime()) {
    return;
  }
  const cleanup = await getCurrentWindow().onDragDropEvent((event) => {
    if (event.payload.type === "drop") {
      dragging.value = false;
      appendPathAttachments(event.payload.paths);
    }
  });
  if (disposed) {
    cleanup();
  } else {
    unlistenDragDrop = cleanup;
  }
});

onBeforeUnmount(() => {
  disposed = true;
  unlistenDragDrop?.();
});

function createAttachment(
  value: string,
  source: NoteAttachment["source"],
  kind: NoteAttachment["kind"],
  nameFallback: string,
): NoteAttachment {
  return {
    id: createId("asset"),
    kind,
    source,
    value,
    name: value.split(/[\\/]/).pop() || nameFallback,
    createdAt: Date.now(),
  };
}

function appendUrlAttachments(urls: string[]) {
  attachments.value.push(...urls.map((url) => createAttachment(url, "url", "image", props.t.url)));
}

function appendFileUrlAttachments(urls: string[]) {
  attachments.value.push(
    ...urls.map((url) =>
      createAttachment(url, "url", isLikelyImagePath(url) ? "image" : "file", props.t.url),
    ),
  );
}

function appendPathAttachments(paths: string[]) {
  attachments.value.push(
    ...paths.map((path) =>
      createAttachment(path, "path", isLikelyImagePath(path) ? "image" : "file", props.t.path),
    ),
  );
}

async function appendFiles(files: FileList | File[]) {
  const nextAttachments: NoteAttachment[] = [];
  for (const file of Array.from(files)) {
    nextAttachments.push({
      id: createId("asset"),
      kind: file.type.startsWith("image/") ? "image" : "file",
      source: "data",
      value: await readFileAsDataUrl(file),
      name: file.name,
      createdAt: Date.now(),
    });
  }
  attachments.value.push(...nextAttachments);
}

function addMediaValue() {
  const value = mediaValue.value.trim();
  if (!value) {
    return;
  }
  const source = /^https?:\/\//i.test(value) ? "url" : "path";
  attachments.value.push(
    createAttachment(
      value,
      source,
      isLikelyImagePath(value) ? "image" : "file",
      source === "url" ? props.t.url : props.t.path,
    ),
  );
  mediaValue.value = "";
}

async function handlePaste(event: ClipboardEvent) {
  const files = Array.from(event.clipboardData?.items ?? [])
    .filter((item) => item.type.startsWith("image/"))
    .map((item) => item.getAsFile())
    .filter((file): file is File => Boolean(file));
  if (files.length > 0) {
    await appendFiles(files);
  }
}

async function handleDrop(event: DragEvent) {
  event.preventDefault();
  dragging.value = false;
  if (!event.dataTransfer) {
    return;
  }
  const urls = resolveDraggedImageUrls(event.dataTransfer);
  if (urls.length > 0) {
    appendUrlAttachments(urls);
    return;
  }
  const fileUrls = resolveDraggedFileUrls(event.dataTransfer);
  if (fileUrls.length > 0) {
    appendFileUrlAttachments(fileUrls);
    return;
  }
  if (!isTauriRuntime() && event.dataTransfer.files.length > 0) {
    await appendFiles(event.dataTransfer.files);
  }
}

function handleDragLeave(event: DragEvent) {
  const relatedTarget = event.relatedTarget;
  if (!(relatedTarget instanceof Node) || !(event.currentTarget as Node).contains(relatedTarget)) {
    dragging.value = false;
  }
}

function handleFileInput(event: Event) {
  const input = event.currentTarget as HTMLInputElement;
  if (input.files) {
    void appendFiles(input.files);
  }
  input.value = "";
}

function removeAttachment(id: string) {
  attachments.value = attachments.value.filter((item) => item.id !== id);
}

function openImagePreview(id: string) {
  previewImageIndex.value = imageAttachments.value.findIndex((item) => item.id === id);
}

function handleMediaKeydown(event: KeyboardEvent) {
  if (event.key === "Enter") {
    event.preventDefault();
    addMediaValue();
  }
}

function submit() {
  if (!content.value.trim() && attachments.value.length === 0) {
    emit("cancel");
    return;
  }
  emit("save", {
    attachments: attachments.value,
    color: color.value,
    content: content.value.trim(),
    pinned: pinned.value,
  });
}
</script>

<template>
  <div
    :class="isWindowMode ? 'editor-window-shell' : 'modal-backdrop'"
    @mousedown="isWindowMode ? undefined : emit('cancel')"
  >
    <section
      :aria-modal="isWindowMode ? undefined : true"
      :class="['editor-dialog', { 'is-window': isWindowMode, 'is-dragging': dragging }]"
      role="dialog"
      @dragenter="dragging = true"
      @dragleave="handleDragLeave"
      @dragover.prevent
      @drop="handleDrop"
      @mousedown.stop
    >
      <header class="editor-dialog__header" @pointerdown="emit('dragStart', $event)">
        <div class="editor-dialog__colors" @pointerdown.stop>
          <button
            v-for="item in NOTE_COLORS"
            :key="item"
            :aria-label="item"
            :class="['color-swatch', { 'is-selected': color === item }]"
            :style="{ backgroundColor: item }"
            type="button"
            @click="color = item"
          />
        </div>
        <div v-if="!isWindowMode" @pointerdown.stop>
          <IconButton :label="t.cancel" subtle @click="emit('cancel')">
            <template #icon><Icon :nodes="XIcon" :size="18" /></template>
          </IconButton>
        </div>
      </header>

      <textarea
        v-model="content"
        autofocus
        class="editor-textarea"
        :placeholder="t.contentPlaceholder"
        @paste="handlePaste"
      />

      <div class="editor-media-row">
        <IconButton :label="t.addImage" @click="fileInput?.click()">
          <template #icon><Icon :nodes="ImagePlusIcon" :size="17" /></template>
          {{ t.addImage }}
        </IconButton>
        <input
          ref="fileInput"
          accept="image/*"
          hidden
          multiple
          type="file"
          @change="handleFileInput"
        />
        <div class="media-input">
          <Icon :nodes="Link2Icon" :size="16" />
          <input
            v-model="mediaValue"
            :placeholder="t.mediaPlaceholder"
            @keydown="handleMediaKeydown"
          />
          <Icon :nodes="FolderIcon" :size="16" />
        </div>
        <IconButton :label="t.addMedia" @click="addMediaValue">
          <template #icon><Icon :nodes="ImagePlusIcon" :size="17" /></template>
          {{ t.addMedia }}
        </IconButton>
      </div>

      <div v-if="attachments.length > 0" class="editor-attachments">
        <template v-for="attachment in attachments" :key="attachment.id">
          <figure v-if="isImageAttachment(attachment)" class="editor-image">
            <button
              class="editor-image__preview"
              :title="attachment.name ?? t.addImage"
              type="button"
              @click="openImagePreview(attachment.id)"
            >
              <img :alt="attachment.name ?? t.addImage" :src="getAttachmentSrc(attachment)" />
            </button>
            <button
              :aria-label="t.removeAttachment"
              :title="t.removeAttachment"
              type="button"
              @click="removeAttachment(attachment.id)"
            >
              <Icon :nodes="Trash2Icon" :size="14" />
            </button>
          </figure>
          <div v-else class="editor-file">
            <Icon :nodes="FileTextIcon" :size="16" />
            <span :title="attachment.value">{{ attachment.name ?? attachment.value }}</span>
            <button
              :aria-label="t.removeAttachment"
              :title="t.removeAttachment"
              type="button"
              @click="removeAttachment(attachment.id)"
            >
              <Icon :nodes="Trash2Icon" :size="14" />
            </button>
          </div>
        </template>
      </div>

      <footer class="editor-dialog__footer">
        <label class="pin-toggle">
          <input v-model="pinned" type="checkbox" />
          <span>{{ t.pinned }}</span>
        </label>
        <div class="editor-dialog__buttons">
          <button class="text-button" type="button" @click="emit('cancel')">{{ t.cancel }}</button>
          <button class="primary-button" type="button" @click="submit">{{ t.save }}</button>
        </div>
      </footer>
    </section>

    <ImagePreview
      v-if="previewImageIndex !== null && previewImages.length > 0"
      :initial-index="previewImageIndex"
      :items="previewImages"
      :t="t"
      @close="previewImageIndex = null"
    />
  </div>
</template>
