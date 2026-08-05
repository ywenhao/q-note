<script setup lang="ts" vapor>
import { ref, watch } from "vue";
import { CheckIcon, CircleAlertIcon, InfoIcon } from "../icons";
import type { ToastKind } from "../hooks/useToast";
import Icon from "./Icon.vue";

const props = withDefaults(
  defineProps<{
    icon?: boolean;
    kind?: ToastKind;
    message: string | null;
  }>(),
  { icon: true, kind: "success" },
);

const visibleIcon = ref(props.icon);
const visibleKind = ref(props.kind);
const visibleMessage = ref(props.message);

watch(
  () => [props.icon, props.kind, props.message] as const,
  ([icon, kind, message]) => {
    if (message) {
      visibleIcon.value = icon;
      visibleKind.value = kind;
      visibleMessage.value = message;
    }
  },
);

function clearHiddenMessage() {
  if (!props.message) {
    visibleMessage.value = null;
  }
}
</script>

<template>
  <div
    aria-live="polite"
    :class="['toast', `toast--${visibleKind}`, { 'is-visible': message }]"
    role="status"
    @transitionend="clearHiddenMessage"
  >
    <Icon
      v-if="visibleIcon && visibleKind === 'error'"
      aria-hidden="true"
      :nodes="CircleAlertIcon"
    />
    <Icon v-else-if="visibleIcon && visibleKind === 'info'" aria-hidden="true" :nodes="InfoIcon" />
    <Icon v-else-if="visibleIcon" aria-hidden="true" :nodes="CheckIcon" />
    <span>{{ visibleMessage }}</span>
  </div>
</template>
