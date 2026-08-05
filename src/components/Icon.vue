<script setup lang="ts" vapor>
import { computed } from "vue";
import type { IconNode } from "../icons";

const props = withDefaults(
  defineProps<{
    nodes: IconNode;
    size?: number;
    strokeWidth?: number;
  }>(),
  { size: 24, strokeWidth: 2 },
);

const children = computed(() =>
  props.nodes.map(([tag, sourceAttrs], index) => {
    const { key, ...attrs } = sourceAttrs;
    return { attrs, key: String(key ?? index), tag };
  }),
);
</script>

<template>
  <svg
    xmlns="http://www.w3.org/2000/svg"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    :stroke-width="strokeWidth"
    stroke-linecap="round"
    stroke-linejoin="round"
  >
    <template v-for="childNode in children" :key="childNode.key">
      <path v-if="childNode.tag === 'path'" v-bind="childNode.attrs" />
      <circle v-else-if="childNode.tag === 'circle'" v-bind="childNode.attrs" />
      <line v-else-if="childNode.tag === 'line'" v-bind="childNode.attrs" />
      <polyline v-else-if="childNode.tag === 'polyline'" v-bind="childNode.attrs" />
      <rect v-else-if="childNode.tag === 'rect'" v-bind="childNode.attrs" />
      <polygon v-else-if="childNode.tag === 'polygon'" v-bind="childNode.attrs" />
      <ellipse v-else-if="childNode.tag === 'ellipse'" v-bind="childNode.attrs" />
    </template>
  </svg>
</template>
