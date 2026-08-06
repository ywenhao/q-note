import type { BundleType } from "./bundleType";
import type { Translation } from "../i18n";

export function getUpdateConfirmBody(t: Translation, bundleType: BundleType) {
  if (bundleType === "appimage") {
    return t.updateConfirmAppImage;
  }

  if (bundleType === "deb") {
    return t.updateConfirmDeb;
  }

  if (bundleType === "rpm") {
    return t.updateConfirmRpm;
  }

  return t.updateConfirmDefault;
}

export function getUpdateInstallingHint(t: Translation, bundleType: BundleType) {
  if (bundleType === "appimage") {
    return t.updateInstallingAppImage;
  }

  if (bundleType === "deb" || bundleType === "rpm") {
    return t.updateInstallingPackage;
  }

  return null;
}
