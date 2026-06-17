import { getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";
import packageJson from "../../package.json";
import { isTauriRuntime } from "./env";

export const UPDATE_DOWNLOAD_PROGRESS_EVENT = "q-note-update-download-progress";

export const FALLBACK_VERSION = packageJson.version;
const RELEASE_BASE_URL = "https://github.com/ywenhao/q-note/releases";

export interface UpdateDownloadSource {
  label: string;
  official: boolean;
  url: string;
}

export interface UpdateAsset {
  browserDownloadUrl: string;
  digest: string | null;
  downloadUrls: UpdateDownloadSource[];
  name: string;
  size: number;
}

export interface UpdateInfo {
  asset: UpdateAsset | null;
  htmlUrl: string;
  latestVersion: string;
  tagName: string;
}

export interface UpdateDownloadProgress {
  downloaded: number;
  fileName: string;
  percent: number;
  sourceLabel: string;
  total: number | null;
}

export interface UpdateDownloadResult {
  fileName: string;
  path: string;
  sourceLabel: string;
}

export async function readAppVersion() {
  if (!isTauriRuntime()) {
    return FALLBACK_VERSION;
  }

  return getVersion();
}

export function getReleaseUrl(version: string) {
  return `${RELEASE_BASE_URL}/tag/v${version}`;
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

export async function checkForUpdate(currentVersion: string) {
  if (!isTauriRuntime()) {
    return null;
  }

  return invoke<UpdateInfo | null>("check_update", { currentVersion });
}

export async function downloadUpdate(update: UpdateInfo) {
  if (!update.asset) {
    throw new Error("update-asset-missing");
  }

  return invoke<UpdateDownloadResult>("download_update", {
    request: {
      asset: update.asset,
      version: update.latestVersion,
    },
  });
}

export async function cancelUpdateDownload() {
  if (!isTauriRuntime()) {
    return;
  }

  await invoke("cancel_update_download");
}

export async function revealDownloadedUpdate(path: string) {
  if (!isTauriRuntime()) {
    return;
  }

  await revealItemInDir(path);
}

export async function installDownloadedUpdate(path: string) {
  if (!isTauriRuntime()) {
    return;
  }

  await invoke("install_update_package", { path });
}
