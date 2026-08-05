<script setup lang="ts" vapor>
import type { Translation } from "../i18n";

defineProps<{
  notesCount: number;
  t: Translation;
}>();
</script>

<template>
  <footer class="app-statusbar">{{ t.statusSummary(notesCount) }}</footer>
</template>
