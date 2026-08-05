<script setup lang="ts" vapor>
import { nextTick, onBeforeUnmount, onMounted, reactive, ref, watch } from "vue";
import { clampContextMenuPosition } from "./contextMenuPosition";
import type { ContextMenuItem } from "./componentTypes";
import Icon from "./Icon.vue";

const props = defineProps<{
  items: ContextMenuItem[];
  x: number;
  y: number;
}>();
const emit = defineEmits<{ close: [] }>();

const menuElement = ref<HTMLDivElement | null>(null);
const position = reactive({ left: props.x, top: props.y });

async function updatePosition() {
  await nextTick();
  const menu = menuElement.value;
  if (!menu) {
    return;
  }

  const bounds = menu.getBoundingClientRect();
  Object.assign(
    position,
    clampContextMenuPosition({
      menuHeight: bounds.height,
      menuWidth: bounds.width,
      viewportHeight: window.innerHeight,
      viewportWidth: window.innerWidth,
      x: props.x,
      y: props.y,
    }),
  );
}

function selectItem(item: ContextMenuItem) {
  item.onSelect();
  emit("close");
}

watch(() => [props.items.length, props.x, props.y], updatePosition, { flush: "post" });
onMounted(() => {
  void updatePosition();
  window.addEventListener("resize", updatePosition);
});
onBeforeUnmount(() => window.removeEventListener("resize", updatePosition));
</script>

<template>
  <div ref="menuElement" class="context-menu" :style="position" @click.stop @contextmenu.prevent>
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
