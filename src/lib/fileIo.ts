import { open, save } from "@tauri-apps/plugin-dialog";
import { readTextFile, writeTextFile } from "@tauri-apps/plugin-fs";
import type { ExportPayload } from "../types";
import { isTauriRuntime } from "./env";

const JSON_FILTER = [{ name: "Q Note", extensions: ["json"] }];

export async function exportJson(payload: ExportPayload) {
  const content = JSON.stringify(payload, null, 2);

  if (isTauriRuntime()) {
    const filePath = await save({
      defaultPath: `q-note-${new Date().toISOString().slice(0, 10)}.json`,
      filters: JSON_FILTER,
    });

    if (!filePath) {
      return false;
    }

    await writeTextFile(filePath, content);
    return true;
  }

  const url = URL.createObjectURL(new Blob([content], { type: "application/json" }));
  const link = document.createElement("a");
  link.href = url;
  link.download = "q-note.json";
  link.click();
  URL.revokeObjectURL(url);
  return true;
}

export async function importJson(): Promise<unknown | null> {
  if (isTauriRuntime()) {
    const filePath = await open({
      multiple: false,
      filters: JSON_FILTER,
    });

    if (!filePath || Array.isArray(filePath)) {
      return null;
    }

    return JSON.parse(await readTextFile(filePath));
  }

  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = "application/json,.json";
    input.onchange = () => {
      const file = input.files?.[0];
      if (!file) {
        resolve(null);
        return;
      }

      const reader = new FileReader();
      reader.onload = () => {
        try {
          resolve(JSON.parse(String(reader.result)));
        } catch {
          resolve(null);
        }
      };
      reader.readAsText(file);
    };
    input.click();
  });
}
