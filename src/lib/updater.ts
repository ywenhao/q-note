import { getVersion } from "@tauri-apps/api/app";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type DownloadEvent, type Update } from "@tauri-apps/plugin-updater";
import { openUrl } from "@tauri-apps/plugin-opener";
import { isTauriRuntime } from "./env";
import { getReleaseTagUrl, PACKAGE_VERSION } from "./repository";

export const FALLBACK_VERSION = PACKAGE_VERSION;

export interface UpdateInfo {
  body: string | null;
  latestVersion: string;
}

export type UpdatePhase = "downloading" | "preparing" | "installing";

export type AppUpdate = Update;

export async function readAppVersion() {
  if (!isTauriRuntime()) {
    return FALLBACK_VERSION;
  }

  return getVersion();
}

export function getReleaseUrl(version: string) {
  return getReleaseTagUrl(version);
}

export async function openReleaseUrl(url: string) {
  if (isTauriRuntime()) {
    await openUrl(url);
    return;
  }

  window.open(url, "_blank", "noopener,noreferrer");
}

export async function openCurrentRelease(version: string) {
  await openReleaseUrl(getReleaseUrl(version));
}

export async function checkForUpdate() {
  if (!isTauriRuntime()) {
    return null;
  }

  return check();
}

export function toUpdateInfo(update: Update): UpdateInfo {
  return {
    body: update.body ?? null,
    latestVersion: update.version,
  };
}

export async function downloadUpdate(update: Update, onEvent: (event: DownloadEvent) => void) {
  await update.download(onEvent);
}

export async function installDownloadedUpdate(update: Update) {
  await update.install();
}

export async function relaunchUpdatedApp() {
  await relaunch();
}
