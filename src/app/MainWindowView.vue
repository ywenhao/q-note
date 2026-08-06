<script setup lang="ts" vapor>
import { ref } from "vue";
import AppHeader from "../components/AppHeader.vue";
import AppToolbar from "../components/AppToolbar.vue";
import type { ContextMenuItem, ImagePreviewItem } from "../components/componentTypes";
import ConfirmDialog from "../components/ConfirmDialog.vue";
import ContextMenu from "../components/ContextMenu.vue";
import ImagePreview from "../components/ImagePreview.vue";
import NoteEditor from "../components/NoteEditor.vue";
import NoteList from "../components/NoteList.vue";
import QMark from "../components/QMark.vue";
import SettingsDialog from "../components/SettingsDialog.vue";
import StatusBar from "../components/StatusBar.vue";
import Toast from "../components/Toast.vue";
import UpdateConfirmDialog from "../components/UpdateConfirmDialog.vue";
import UpdateDownloadDialog from "../components/UpdateDownloadDialog.vue";
import type { MenuState } from "../features/menu/useMenuController";
import type { ToastState } from "../hooks/useToast";
import type { Translation } from "../i18n";
import type { BundleType } from "../lib/bundleType";
import type { UpdateDownloadProgress } from "../lib/updateProgress";
import type { UpdateInfo, UpdatePhase } from "../lib/updater";
import type { Note, NoteDraft } from "../types";

defineProps<{
  alwaysOnLabel: string;
  alwaysOnTop: boolean;
  appVersion: string;
  autoStart: boolean;
  bundleType: BundleType;
  checkingUpdate: boolean;
  contextItems: ContextMenuItem[];
  dockButtonLabel: string;
  editorNote: Note | null | undefined;
  hasUpdate: boolean;
  menu: MenuState | null;
  notes: Note[];
  ready: boolean;
  showDeleteAllConfirm: boolean;
  showSettings: boolean;
  t: Translation;
  toast: ToastState | null;
  updateConfirmBody: string;
  updateConfirmOpen: boolean;
  updateDialogOpen: boolean;
  updateDownloadProgress: UpdateDownloadProgress | null;
  updateInfo: UpdateInfo | null;
  updatePhase: UpdatePhase;
}>();

const emit = defineEmits<{
  cancelEditor: [];
  cancelUpdateConfirm: [];
  checkUpdate: [];
  closeConfirmDeleteAll: [];
  closeMenu: [];
  closeSettings: [];
  closeWindow: [];
  collapseToDock: [];
  colorChange: [id: string, color: string];
  confirmDeleteAll: [];
  confirmUpdate: [];
  copyNote: [note: Note];
  deleteAll: [];
  deleteNote: [id: string];
  dragMainWindow: [event: PointerEvent];
  editNote: [note: Note];
  export: [];
  heightChange: [id: string, textHeight: number];
  import: [];
  minimizeWindow: [];
  newNote: [];
  openCurrentRelease: [];
  openMenu: [event: MouseEvent, noteId?: string];
  openSettings: [];
  reorderNotes: [draggedId: string, targetId: string, placement: "before" | "after"];
  saveDraft: [draft: NoteDraft];
  toggleAlwaysOnTop: [];
  toggleAutoStart: [];
  toggleLanguage: [];
  toggleNotePin: [id: string];
}>();

interface ImagePreviewState {
  index: number;
  items: ImagePreviewItem[];
}
const imagePreview = ref<ImagePreviewState | null>(null);

function previewImages(items: ImagePreviewItem[], index: number) {
  imagePreview.value = { index, items };
}
function forwardColorChange(id: string, color: string) {
  emit("colorChange", id, color);
}
function forwardContextMenu(event: MouseEvent, noteId: string) {
  emit("openMenu", event, noteId);
}
function forwardHeightChange(id: string, height: number) {
  emit("heightChange", id, height);
}
function forwardReorder(draggedId: string, targetId: string, placement: "before" | "after") {
  emit("reorderNotes", draggedId, targetId, placement);
}
</script>

<template>
  <main v-if="!ready" class="app-shell is-loading">
    <QMark class="loading-mark" />
  </main>
  <main v-else class="app-shell" @click="emit('closeMenu')" @contextmenu="emit('openMenu', $event)">
    <AppHeader
      :always-on-label="alwaysOnLabel"
      :always-on-top="alwaysOnTop"
      :t="t"
      @close="emit('closeWindow')"
      @drag-start="emit('dragMainWindow', $event)"
      @minimize="emit('minimizeWindow')"
      @toggle-always-on-top="emit('toggleAlwaysOnTop')"
    />
    <AppToolbar
      :has-update="hasUpdate"
      :notes-count="notes.length"
      :t="t"
      :update-version="updateInfo?.latestVersion"
      @delete-all="emit('deleteAll')"
      @new-note="emit('newNote')"
      @open-settings="emit('openSettings')"
      @toggle-language="emit('toggleLanguage')"
    />
    <NoteList
      :notes="notes"
      :t="t"
      @color-change="forwardColorChange"
      @context-menu="forwardContextMenu"
      @copy="emit('copyNote', $event)"
      @delete="emit('deleteNote', $event)"
      @edit="emit('editNote', $event)"
      @height-change="forwardHeightChange"
      @new-note="emit('newNote')"
      @preview-images="previewImages"
      @reorder="forwardReorder"
      @toggle-pin="emit('toggleNotePin', $event)"
    />

    <NoteEditor
      v-if="editorNote !== undefined"
      :note="editorNote"
      :t="t"
      @cancel="emit('cancelEditor')"
      @save="emit('saveDraft', $event)"
    />
    <SettingsDialog
      v-if="showSettings"
      :app-version="appVersion"
      :auto-start="autoStart"
      :checking-update="checkingUpdate"
      :has-update="hasUpdate"
      :t="t"
      @check-update="emit('checkUpdate')"
      @close="emit('closeSettings')"
      @export="emit('export')"
      @import="emit('import')"
      @open-current-release="emit('openCurrentRelease')"
      @toggle-auto-start="emit('toggleAutoStart')"
    />
    <UpdateConfirmDialog
      v-if="updateConfirmOpen && updateInfo"
      :body="updateConfirmBody"
      :confirm-label="t.updateConfirm"
      :t="t"
      :update="updateInfo"
      @cancel="emit('cancelUpdateConfirm')"
      @confirm="emit('confirmUpdate')"
    />
    <UpdateDownloadDialog
      v-if="updateDialogOpen && updateInfo"
      :bundle-type="bundleType"
      :phase="updatePhase"
      :progress="updateDownloadProgress"
      :t="t"
      :update="updateInfo"
    />
    <ContextMenu
      v-if="menu"
      :items="contextItems"
      :x="menu.x"
      :y="menu.y"
      @close="emit('closeMenu')"
    />
    <ConfirmDialog
      v-if="showDeleteAllConfirm"
      :body="t.deleteAllBody"
      :cancel-label="t.cancel"
      :confirm-label="t.deleteAll"
      :title="t.confirmDeleteAll"
      @cancel="emit('closeConfirmDeleteAll')"
      @confirm="emit('confirmDeleteAll')"
    />
    <button
      :aria-label="dockButtonLabel"
      class="panel-dock-button"
      :title="dockButtonLabel"
      type="button"
      @click.stop="emit('collapseToDock')"
    >
      <QMark />
    </button>
    <StatusBar :notes-count="notes.length" :t="t" />
    <Toast :icon="toast?.icon" :kind="toast?.kind" :message="toast?.message ?? null" />
    <ImagePreview
      v-if="imagePreview"
      :initial-index="imagePreview.index"
      :items="imagePreview.items"
      :t="t"
      @close="imagePreview = null"
    />
  </main>
</template>
