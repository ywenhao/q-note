<script setup lang="ts" vapor>
defineProps<{
  body: string;
  cancelLabel: string;
  confirmLabel: string;
  title: string;
}>();

const emit = defineEmits<{
  cancel: [];
  confirm: [];
}>();
</script>

<template>
  <div class="modal-backdrop" @mousedown="emit('cancel')">
    <section aria-modal="true" class="confirm-dialog" role="alertdialog" @mousedown.stop>
      <h2>{{ title }}</h2>
      <p>{{ body }}</p>
      <footer>
        <button class="text-button" type="button" @click="emit('cancel')">
          {{ cancelLabel }}
        </button>
        <button class="danger-button" type="button" @click="emit('confirm')">
          {{ confirmLabel }}
        </button>
      </footer>
    </section>
  </div>
</template>
