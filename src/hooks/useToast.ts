import { onBeforeUnmount, ref } from "vue";

export type ToastKind = "success" | "error" | "info";

export interface ToastState {
  icon: boolean;
  kind: ToastKind;
  message: string;
}

export type ShowToast = (message: string, options?: { icon?: boolean; kind?: ToastKind }) => void;

export function useToast() {
  const toast = ref<ToastState | null>(null);
  let toastTimer: number | null = null;

  const showToast: ShowToast = (message, options = {}) => {
    toast.value = {
      icon: options.icon ?? true,
      kind: options.kind ?? "success",
      message,
    };
    if (toastTimer) {
      window.clearTimeout(toastTimer);
    }
    toastTimer = window.setTimeout(() => {
      toast.value = null;
      toastTimer = null;
    }, 1700);
  };

  onBeforeUnmount(() => {
    if (toastTimer) {
      window.clearTimeout(toastTimer);
    }
  });

  return { showToast, toast };
}
