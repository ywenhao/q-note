import { useCallback, useEffect, useRef, useState } from "react";

export type ToastKind = "success" | "error" | "info";

export interface ToastState {
  icon: boolean;
  kind: ToastKind;
  message: string;
}

export type ShowToast = (message: string, options?: { icon?: boolean; kind?: ToastKind }) => void;

export function useToast() {
  const [toast, setToast] = useState<ToastState | null>(null);
  const toastTimerRef = useRef<number | null>(null);

  const showToast = useCallback<ShowToast>((message, options = {}) => {
    setToast({
      icon: options.icon ?? true,
      kind: options.kind ?? "success",
      message,
    });

    if (toastTimerRef.current) {
      window.clearTimeout(toastTimerRef.current);
    }

    toastTimerRef.current = window.setTimeout(() => setToast(null), 1700);
  }, []);

  useEffect(
    () => () => {
      if (toastTimerRef.current) {
        window.clearTimeout(toastTimerRef.current);
      }
    },
    [],
  );

  return {
    showToast,
    toast,
  };
}
