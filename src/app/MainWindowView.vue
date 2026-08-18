<script setup lang="ts" vapor>
import { ref } from "vue";
import AppHeader from "../components/AppHeader.vue";
import AppToolbar from "../components/AppToolbar.vue";
import type { ImagePreviewItem } from "../components/componentTypes";
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
import { useMainWindow } from "./useMainWindow";

const {
  alwaysOnLabel,
  appVersion,
  bundleType,
  cancelUpdateConfirm,
  cancelUpdateDownload,
  changeNoteColor,
  changeNoteHeight,
  checkingUpdate,
  closeEditor,
  closeMenu,
  closeWindow,
  collapseToDock,
  confirmDeleteAll,
  confirmUpdate,
  contextItems,
  editorNote,
  handleCheckUpdate,
  handleCopy,
  handleDelete,
  handleExport,
  handleImport,
  handleOpenCurrentRelease,
  hasUpdate,
  menu,
  notes,
  openEditor,
  openMenu,
  ready,
  reorderNote,
  saveDraft,
  settings,
  showDeleteAllConfirm,
  showSettings,
  t,
  toast,
  toggleAlwaysOnTop,
  toggleAutoStart,
  toggleLanguage,
  toggleNotePin,
  updateConfirmBody,
  updateConfirmOpen,
  updateDialogOpen,
  updateDownloadProgress,
  updateInfo,
  updatePhase,
} = useMainWindow();

interface ImagePreviewState {
  index: number;
  items: ImagePreviewItem[];
}
const imagePreview = ref<ImagePreviewState | null>(null);

function previewImages(items: ImagePreviewItem[], index: number) {
  imagePreview.value = { index, items };
}
</script>

<template>
  <main v-if="!ready" class="app-shell is-loading">
    <QMark class="loading-mark" />
  </main>
  <main v-else class="app-shell" @click="closeMenu" @contextmenu="openMenu($event)">
    <div class="app-column">
      <AppHeader
        :always-on-label="alwaysOnLabel"
        :always-on-top="settings.alwaysOnTop"
        :t="t"
        @close="closeWindow"
        @toggle-always-on-top="toggleAlwaysOnTop"
      />
      <AppToolbar
        :has-update="hasUpdate"
        :notes-count="notes.length"
        :t="t"
        :update-version="updateInfo?.latestVersion"
        @delete-all="showDeleteAllConfirm = true"
        @new-note="openEditor(null)"
        @open-settings="showSettings = true"
        @toggle-language="toggleLanguage"
      />
      <NoteList
        :notes="notes"
        :t="t"
        @color-change="changeNoteColor"
        @context-menu="(event, noteId) => openMenu(event, noteId)"
        @copy="handleCopy"
        @delete="handleDelete"
        @edit="openEditor"
        @height-change="changeNoteHeight"
        @new-note="openEditor(null)"
        @preview-images="previewImages"
        @reorder="reorderNote"
        @toggle-pin="toggleNotePin"
      />
    </div>

    <NoteEditor
      v-if="editorNote !== undefined"
      :note="editorNote"
      :t="t"
      @cancel="closeEditor"
      @save="saveDraft"
    />
    <SettingsDialog
      v-if="showSettings"
      :app-version="appVersion"
      :auto-start="settings.autoStart"
      :checking-update="checkingUpdate"
      :has-update="hasUpdate"
      :t="t"
      @check-update="handleCheckUpdate"
      @close="showSettings = false"
      @export="handleExport"
      @import="handleImport"
      @open-current-release="handleOpenCurrentRelease"
      @toggle-auto-start="toggleAutoStart"
    />
    <UpdateConfirmDialog
      v-if="updateConfirmOpen && updateInfo"
      :body="updateConfirmBody"
      :confirm-label="t.updateConfirm"
      :t="t"
      :update="updateInfo"
      @cancel="cancelUpdateConfirm"
      @confirm="confirmUpdate"
    />
    <UpdateDownloadDialog
      v-if="updateDialogOpen && updateInfo"
      :bundle-type="bundleType"
      :phase="updatePhase"
      :progress="updateDownloadProgress"
      :t="t"
      :update="updateInfo"
      @cancel="cancelUpdateDownload"
    />
    <ContextMenu v-if="menu" :items="contextItems" :x="menu.x" :y="menu.y" @close="closeMenu" />
    <ConfirmDialog
      v-if="showDeleteAllConfirm"
      :body="t.deleteAllBody"
      :cancel-label="t.cancel"
      :confirm-label="t.deleteAll"
      :title="t.confirmDeleteAll"
      @cancel="showDeleteAllConfirm = false"
      @confirm="confirmDeleteAll"
    />
    <button
      :aria-label="t.switchFloatingBall"
      class="panel-dock-button"
      :title="t.switchFloatingBall"
      type="button"
      @click.stop="collapseToDock"
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
