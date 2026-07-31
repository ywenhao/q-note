import { emitTo, listen } from "@tauri-apps/api/event";
import { getAllWindows } from "@tauri-apps/api/window";
import { createId, isTauriRuntime } from "./env";
import { EDITOR_WINDOW_LABEL } from "./windowControls";

export const PREPARE_UPDATE_EVENT = "q-note-prepare-update";
export const PREPARE_UPDATE_ACK_EVENT = "q-note-prepare-update-ack";

export interface PrepareUpdateRequest {
  requestId: string;
}

export interface PrepareUpdateAcknowledgement {
  error?: string;
  ok: boolean;
  requestId: string;
}

export async function prepareEditorForUpdate(timeoutMs = 5000) {
  if (!isTauriRuntime()) {
    return;
  }

  const editorWindow = (await getAllWindows()).find(
    (window) => window.label === EDITOR_WINDOW_LABEL,
  );
  if (!editorWindow || !(await editorWindow.isVisible())) {
    return;
  }

  const requestId = createId("update");
  let resolveResponse: (() => void) | null = null;
  let rejectResponse: ((error: Error) => void) | null = null;
  const response = new Promise<void>((resolve, reject) => {
    resolveResponse = resolve;
    rejectResponse = reject;
  });
  const unlisten = await listen<PrepareUpdateAcknowledgement>(PREPARE_UPDATE_ACK_EVENT, (event) => {
    if (event.payload.requestId !== requestId) {
      return;
    }

    if (event.payload.ok) {
      resolveResponse?.();
    } else {
      rejectResponse?.(new Error(event.payload.error ?? "update-editor-prepare-failed"));
    }
  });
  const timer = window.setTimeout(
    () => rejectResponse?.(new Error("update-editor-prepare-timeout")),
    timeoutMs,
  );

  try {
    await emitTo(EDITOR_WINDOW_LABEL, PREPARE_UPDATE_EVENT, {
      requestId,
    } satisfies PrepareUpdateRequest);
    await response;
  } finally {
    window.clearTimeout(timer);
    unlisten();
  }
}
