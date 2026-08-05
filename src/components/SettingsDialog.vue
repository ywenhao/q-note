<script setup lang="ts" vapor>
import {
  DownloadIcon,
  LoaderCircleIcon,
  PowerIcon,
  RefreshCwIcon,
  UploadIcon,
  XIcon,
} from "../icons";
import type { Translation } from "../i18n";
import Icon from "./Icon.vue";

defineProps<{
  appVersion: string;
  autoStart: boolean;
  checkingUpdate: boolean;
  hasUpdate: boolean;
  t: Translation;
}>();

const emit = defineEmits<{
  checkUpdate: [];
  close: [];
  export: [];
  import: [];
  openCurrentRelease: [];
  toggleAutoStart: [];
}>();
</script>

<template>
  <div class="modal-backdrop" @mousedown="emit('close')">
    <section aria-modal="true" class="settings-dialog" role="dialog" @mousedown.stop>
      <header class="settings-dialog__header">
        <h2>{{ t.settingsTitle }}</h2>
        <button class="settings-close" :aria-label="t.cancel" type="button" @click="emit('close')">
          <Icon :nodes="XIcon" :size="14" />
        </button>
      </header>

      <div class="settings-panel">
        <button class="settings-row" type="button" @click="emit('toggleAutoStart')">
          <span class="settings-row__label"
            ><Icon :nodes="PowerIcon" :size="14" />{{ t.startupSetting }}</span
          >
          <span :class="['switch', { 'is-on': autoStart }]" aria-hidden="true"><span /></span>
        </button>

        <div class="settings-actions">
          <button class="settings-action" type="button" @click="emit('import')">
            <Icon :nodes="UploadIcon" :size="14" />{{ t.import }}
          </button>
          <button class="settings-action" type="button" @click="emit('export')">
            <Icon :nodes="DownloadIcon" :size="14" />{{ t.export }}
          </button>
        </div>

        <button
          :class="['settings-row', 'settings-row--center', { 'is-loading': checkingUpdate }]"
          :disabled="checkingUpdate"
          type="button"
          @click="emit('checkUpdate')"
        >
          <span class="settings-row__label">
            <Icon v-if="checkingUpdate" class="spin-icon" :nodes="LoaderCircleIcon" :size="14" />
            <Icon v-else :nodes="RefreshCwIcon" :size="14" />
            <span v-if="hasUpdate" class="update-dot" aria-hidden="true" />
            {{ t.checkUpdate }}
          </span>
          <span v-if="hasUpdate" class="settings-row__value">{{ t.updateAvailable }}</span>
        </button>
      </div>

      <footer class="settings-footer">
        <button class="settings-version" type="button" @click="emit('openCurrentRelease')">
          v{{ appVersion }}
        </button>
      </footer>
    </section>
  </div>
</template>
