<script setup lang="ts" vapor>
import { computed } from "vue";
import { DownloadIcon } from "../icons";
import type { Translation } from "../i18n";
import type { UpdateDownloadProgress } from "../lib/updateProgress";
import type { UpdateInfo, UpdatePhase } from "../lib/updater";
import Icon from "./Icon.vue";

const props = defineProps<{
  phase: UpdatePhase;
  progress: UpdateDownloadProgress | null;
  t: Translation;
  update: UpdateInfo;
}>();

const percent = computed(() =>
  props.phase === "downloading" ? Math.round(props.progress?.percent ?? 0) : 100,
);
const status = computed(() =>
  props.phase === "downloading"
    ? props.t.updateDownloading
    : props.phase === "preparing"
      ? props.t.updatePreparing
      : props.t.updateInstalling,
);

function formatBytes(value: number) {
  if (value < 1024) {
    return `${value} B`;
  }

  const units = ["KB", "MB", "GB"];
  let nextValue = value / 1024;
  let unitIndex = 0;
  while (nextValue >= 1024 && unitIndex < units.length - 1) {
    nextValue /= 1024;
    unitIndex += 1;
  }
  return `${nextValue.toFixed(nextValue >= 10 ? 0 : 1)} ${units[unitIndex]}`;
}
</script>

<template>
  <div class="update-download-backdrop">
    <section aria-modal="true" class="update-download-dialog" role="dialog">
      <header>
        <span><Icon :nodes="DownloadIcon" :size="15" />{{ status }}</span>
      </header>
      <div class="update-download-body">
        <strong>Q Note v{{ update.latestVersion }}</strong>
        <small v-if="update.body">{{ update.body }}</small>
      </div>
      <div
        :aria-label="t.updateDownloadProgress"
        aria-valuemax="100"
        aria-valuemin="0"
        :aria-valuenow="phase === 'downloading' ? percent : undefined"
        :class="['update-progress', { 'is-indeterminate': phase !== 'downloading' }]"
        role="progressbar"
      >
        <span :style="{ width: phase === 'downloading' ? `${percent}%` : '35%' }" />
      </div>
      <div class="update-download-meta">
        <span>{{ phase === "downloading" ? `${percent}%` : status }}</span>
        <span v-if="phase === 'downloading' && progress?.total">
          {{ formatBytes(progress.downloaded) }} / {{ formatBytes(progress.total) }}
        </span>
      </div>
    </section>
  </div>
</template>
