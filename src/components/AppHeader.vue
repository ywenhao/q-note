<script setup lang="ts" vapor>
import type { Translation } from "../i18n";
import WindowChrome from "./WindowChrome.vue";

defineProps<{
  alwaysOnLabel: string;
  alwaysOnTop: boolean;
  t: Translation;
}>();

const emit = defineEmits<{
  close: [];
  toggleAlwaysOnTop: [];
}>();
</script>

<template>
  <WindowChrome
    show-brand
    :always-on-label="alwaysOnLabel"
    :always-on-top="alwaysOnTop"
    :t="t"
    :title="t.appTitle"
    @close="emit('close')"
    @toggle-always-on-top="emit('toggleAlwaysOnTop')"
  />
</template>
