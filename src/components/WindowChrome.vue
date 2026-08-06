<script setup lang="ts" vapor>
import { MinusIcon, PinIcon, PinOffIcon, XIcon } from "../icons";
import type { Translation } from "../i18n";
import { minimizeCurrentWindow, startCurrentWindowDrag } from "../hooks/useWindowChrome";
import Icon from "./Icon.vue";
import IconButton from "./IconButton.vue";
import QMark from "./QMark.vue";

defineProps<{
  alwaysOnLabel: string;
  alwaysOnTop: boolean;
  showBrand?: boolean;
  t: Translation;
  title: string;
}>();

const emit = defineEmits<{
  close: [];
  toggleAlwaysOnTop: [];
}>();
</script>

<template>
  <header class="top-bar window-chrome" @pointerdown="startCurrentWindowDrag">
    <div class="brand">
      <QMark v-if="showBrand" class="brand-mark" />
      <h1>{{ title }}</h1>
    </div>
    <div class="title-controls" @pointerdown.stop>
      <IconButton
        :active="alwaysOnTop"
        class="is-window-pin"
        :label="alwaysOnLabel"
        subtle
        @click="emit('toggleAlwaysOnTop')"
      >
        <template #icon>
          <Icon v-if="alwaysOnTop" :nodes="PinOffIcon" :size="16" />
          <Icon v-else :nodes="PinIcon" :size="16" />
        </template>
      </IconButton>
      <IconButton
        class="is-window-minimize"
        :label="t.minimize"
        subtle
        @click="minimizeCurrentWindow"
      >
        <template #icon><Icon :nodes="MinusIcon" :size="16" /></template>
      </IconButton>
      <IconButton class="is-window-close" :label="t.closePanel" subtle @click="emit('close')">
        <template #icon><Icon :nodes="XIcon" :size="16" /></template>
      </IconButton>
    </div>
  </header>
</template>
