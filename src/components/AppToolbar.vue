<script setup lang="ts" vapor>
import { computed } from "vue";
import { LanguagesIcon, PlusIcon, SettingsIcon, Trash2Icon } from "../icons";
import type { Translation } from "../i18n";
import Icon from "./Icon.vue";
import IconButton from "./IconButton.vue";

const props = defineProps<{
  hasUpdate: boolean;
  notesCount: number;
  t: Translation;
  updateVersion?: string;
}>();
const emit = defineEmits<{
  deleteAll: [];
  newNote: [];
  openSettings: [];
  toggleLanguage: [];
}>();

const settingsLabel = computed(() =>
  props.hasUpdate && props.updateVersion
    ? props.t.updateAvailableTitle(props.updateVersion)
    : props.t.settings,
);
</script>

<template>
  <div class="toolbar">
    <IconButton :label="t.newNote" @click="emit('newNote')">
      <template #icon><Icon :nodes="PlusIcon" :size="18" /></template>
    </IconButton>
    <IconButton
      class="is-danger"
      :disabled="notesCount === 0"
      :label="t.deleteAll"
      @click="emit('deleteAll')"
    >
      <template #icon><Icon :nodes="Trash2Icon" :size="18" /></template>
    </IconButton>
    <IconButton :badge="hasUpdate" :label="settingsLabel" @click="emit('openSettings')">
      <template #icon><Icon :nodes="SettingsIcon" :size="18" /></template>
    </IconButton>
    <IconButton :label="t.language" @click="emit('toggleLanguage')">
      <template #icon><Icon :nodes="LanguagesIcon" :size="18" /></template>
      {{ t.language }}
    </IconButton>
  </div>
</template>
