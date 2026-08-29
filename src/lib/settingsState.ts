import type { AppSettings, Language } from "../types.ts";
import { normalizeWindowState } from "./windowState.ts";

function readSystemLanguage() {
  if (typeof navigator === "undefined") {
    return "en";
  }

  return navigator.language || navigator.languages?.[0] || "en";
}

function detectDefaultLanguage(): Language {
  const language = readSystemLanguage().toLowerCase();

  if (language.startsWith("zh")) {
    return "zh";
  }

  return "en";
}

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function toLanguage(value: unknown): Language {
  if (value === "zh" || value === "en") {
    return value;
  }

  return detectDefaultLanguage();
}

export function createDefaultSettings(): AppSettings {
  return {
    language: detectDefaultLanguage(),
    alwaysOnTop: false,
    autoStart: false,
    dockOnEdge: false,
    docked: false,
    dockEdge: null,
    keepFullMain: false,
    window: null,
  };
}

export function normalizeSettings(value: unknown): AppSettings {
  const defaults = createDefaultSettings();

  if (!isObject(value)) {
    return defaults;
  }

  const windowState = normalizeWindowState(value.window);
  return {
    language: toLanguage(value.language),
    alwaysOnTop: Boolean(value.alwaysOnTop),
    autoStart: Boolean(value.autoStart),
    dockOnEdge: typeof value.dockOnEdge === "boolean" ? value.dockOnEdge : defaults.dockOnEdge,
    docked: Boolean(value.docked),
    dockEdge:
      value.dockEdge === "left" ||
      value.dockEdge === "right" ||
      value.dockEdge === "top" ||
      value.dockEdge === "bottom"
        ? value.dockEdge
        : null,
    keepFullMain: Boolean(value.keepFullMain),
    window: windowState,
  };
}

export function parseStoredSettings(value: unknown): AppSettings {
  if (typeof value !== "string") {
    return normalizeSettings(value);
  }

  try {
    return normalizeSettings(JSON.parse(value));
  } catch {
    return createDefaultSettings();
  }
}
