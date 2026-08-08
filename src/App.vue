<script setup lang="ts" vapor>
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./App.css";
import DockWindowView from "./app/DockWindowView.vue";
import MainWindowView from "./app/MainWindowView.vue";
import { isTauriRuntime } from "./lib/env";
import { DOCK_WINDOW_LABEL, MAIN_WINDOW_LABEL } from "./lib/windowControls";

const currentWindowLabel = isTauriRuntime() ? getCurrentWindow().label : MAIN_WINDOW_LABEL;
const isDockWindow = currentWindowLabel === DOCK_WINDOW_LABEL;
</script>

<template>
  <DockWindowView v-if="isDockWindow" />
  <MainWindowView v-else />
</template>
