import assert from "node:assert/strict";
import test from "node:test";
import { shouldOpenReleaseForUpdate, supportsInAppLinuxUpdate } from "../src/lib/bundleType.ts";
import { translations } from "../src/i18n.ts";
import { getUpdateConfirmBody, getUpdateInstallingHint } from "../src/lib/updateInstallHint.ts";

test("supports in-app linux updates for appimage deb and rpm", () => {
  assert.equal(supportsInAppLinuxUpdate("appimage"), true);
  assert.equal(supportsInAppLinuxUpdate("deb"), true);
  assert.equal(supportsInAppLinuxUpdate("rpm"), true);
  assert.equal(supportsInAppLinuxUpdate("unknown"), false);
});

test("opens release page for unknown linux install types", () => {
  assert.equal(shouldOpenReleaseForUpdate({ os: "linux", bundleType: "unknown" }), true);
  assert.equal(shouldOpenReleaseForUpdate({ os: "linux", bundleType: "appimage" }), false);
  assert.equal(shouldOpenReleaseForUpdate({ os: "windows", bundleType: "unknown" }), false);
});

test("returns bundle-specific update copy", () => {
  const t = translations.zh;
  assert.match(getUpdateConfirmBody(t, "appimage"), /替换/);
  assert.match(getUpdateConfirmBody(t, "deb"), /密码/);
  assert.match(getUpdateConfirmBody(t, "rpm"), /密码/);
  assert.equal(getUpdateInstallingHint(t, "deb"), t.updateInstallingPackage);
  assert.equal(getUpdateInstallingHint(t, "appimage"), t.updateInstallingAppImage);
});
