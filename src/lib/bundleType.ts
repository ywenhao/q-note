import { invoke } from "@tauri-apps/api/core";
import { isTauriRuntime } from "./env.ts";

export type BundleType = "appimage" | "deb" | "rpm" | "nsis" | "msi" | "app" | "unknown";

export type RuntimeOs = "linux" | "windows" | "macos" | "other";

export interface RuntimeBundleInfo {
  bundleType: BundleType;
  os: RuntimeOs;
}

const DEFAULT_RUNTIME_BUNDLE_INFO: RuntimeBundleInfo = {
  bundleType: "unknown",
  os: "other",
};

export function isLinuxPackageBundle(bundleType: BundleType) {
  return bundleType === "deb" || bundleType === "rpm";
}

export function supportsInAppLinuxUpdate(bundleType: BundleType) {
  return bundleType === "appimage" || isLinuxPackageBundle(bundleType);
}

export function shouldOpenReleaseForUpdate(info: RuntimeBundleInfo) {
  return info.os === "linux" && !supportsInAppLinuxUpdate(info.bundleType);
}

export async function readRuntimeBundleInfo(): Promise<RuntimeBundleInfo> {
  if (!isTauriRuntime()) {
    return DEFAULT_RUNTIME_BUNDLE_INFO;
  }

  const info = await invoke<{ bundleType: BundleType; os: RuntimeOs }>("get_bundle_type");
  return {
    bundleType: info.bundleType,
    os: info.os,
  };
}
