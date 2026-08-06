<script setup lang="ts" vapor>
import type { Translation } from "../i18n";
import type { UpdateInfo } from "../lib/updater";

defineProps<{
  body: string;
  confirmLabel: string;
  t: Translation;
  update: UpdateInfo;
}>();

const emit = defineEmits<{
  cancel: [];
  confirm: [];
}>();
</script>

<template>
  <div class="modal-backdrop" @mousedown="emit('cancel')">
    <section
      aria-modal="true"
      class="confirm-dialog update-confirm-dialog"
      role="dialog"
      @mousedown.stop
    >
      <h2>{{ t.updateAvailableTitle(update.latestVersion) }}</h2>
      <p>{{ body }}</p>
      <small v-if="update.body" class="update-confirm-dialog__notes">{{ update.body }}</small>
      <footer>
        <button class="text-button" type="button" @click="emit('cancel')">
          {{ t.cancel }}
        </button>
        <button class="primary-button" type="button" @click="emit('confirm')">
          {{ confirmLabel }}
        </button>
      </footer>
    </section>
  </div>
</template>
