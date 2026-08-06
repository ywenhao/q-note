<script setup lang="ts" vapor>
import { ref } from "vue";
import { useContextMenuPosition } from "../hooks/useContextMenu";
import type { ContextMenuItem } from "./componentTypes";
import Icon from "./Icon.vue";

const props = defineProps<{
  items: ContextMenuItem[];
  x: number;
  y: number;
}>();
const emit = defineEmits<{ close: [] }>();

const menuElement = ref<HTMLDivElement | null>(null);
const { menuStyle } = useContextMenuPosition({
  itemsLength: () => props.items.length,
  menuElement,
  x: () => props.x,
  y: () => props.y,
});

function selectItem(item: ContextMenuItem) {
  item.onSelect();
  emit("close");
}
</script>

<template>
  <div ref="menuElement" class="context-menu" :style="menuStyle" @click.stop @contextmenu.prevent>
    <button
      v-for="item in items"
      :key="item.id"
      :class="{ 'is-danger': item.destructive }"
      type="button"
      @click="selectItem(item)"
    >
      <Icon :nodes="item.icon" :size="16" />
      <span>{{ item.label }}</span>
    </button>
  </div>
</template>
