<script setup lang="ts" vapor>
import { MinusIcon, PinIcon, PinOffIcon, XIcon } from "../icons";
import type { Translation } from "../i18n";
import Icon from "./Icon.vue";
import IconButton from "./IconButton.vue";
import QMark from "./QMark.vue";

defineProps<{
  alwaysOnLabel: string;
  alwaysOnTop: boolean;
  t: Translation;
}>();

const emit = defineEmits<{
  close: [];
  dragStart: [event: PointerEvent];
  minimize: [];
  toggleAlwaysOnTop: [];
}>();
</script>

<template>
  <header class="top-bar" @pointerdown="emit('dragStart', $event)">
    <div class="brand">
      <QMark class="brand-mark" />
      <h1>{{ t.appTitle }}</h1>
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
      <IconButton class="is-window-minimize" :label="t.minimize" subtle @click="emit('minimize')">
        <template #icon><Icon :nodes="MinusIcon" :size="16" /></template>
      </IconButton>
      <IconButton class="is-window-close" :label="t.closePanel" subtle @click="emit('close')">
        <template #icon><Icon :nodes="XIcon" :size="16" /></template>
      </IconButton>
    </div>
  </header>
</template>
