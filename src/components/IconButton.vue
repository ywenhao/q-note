<script setup lang="ts" vapor>
import { useSlots } from "vue";

withDefaults(
  defineProps<{
    active?: boolean;
    badge?: boolean;
    label: string;
    subtle?: boolean;
    type?: "button" | "submit" | "reset";
  }>(),
  {
    active: false,
    badge: false,
    subtle: false,
    type: "button",
  },
);

const slots = useSlots();
</script>

<template>
  <button
    :aria-label="label"
    :class="['icon-button', { 'is-active': active, 'is-badged': badge, 'is-subtle': subtle }]"
    :title="label"
    :type="type"
  >
    <slot name="icon" />
    <span v-if="slots.default"><slot /></span>
  </button>
</template>
