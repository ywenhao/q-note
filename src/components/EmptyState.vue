<script setup lang="ts" vapor>
import type { Translation } from "../i18n";
import QMark from "./QMark.vue";

defineProps<{ t: Translation }>();
const emit = defineEmits<{ newNote: [] }>();
</script>

<template>
  <section class="empty-state">
    <QMark class="empty-mark" />
    <h2>{{ t.emptyTitle }}</h2>
    <p>{{ t.noNotesBody }}</p>
    <button class="primary-button" type="button" @click="emit('newNote')">
      {{ t.emptyAction }}
    </button>
  </section>
</template>
