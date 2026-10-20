import {
  chmodSync,
  copyFileSync,
  mkdirSync,
  readFileSync,
  writeFileSync,
} from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";

const root = resolve(fileURLToPath(new URL("..", import.meta.url)));
const { version } = JSON.parse(
  readFileSync(join(root, "package.json"), "utf8"),
);
const uiVersion = JSON.parse(
  readFileSync(join(root, "ui/nsh.json"), "utf8"),
).version;
const rustVersion = readFileSync(join(root, "Cargo.toml"), "utf8").match(
  /^version\s*=\s*"([^"]+)"/m,
)?.[1];
if (
  uiVersion !== version ||
  rustVersion !== version ||
  (process.env.RELEASE_TAG &&
    process.env.RELEASE_TAG.replace(/^v/, "") !== version)
)
  throw new Error("Package, UI, Rust, and release tag versions must agree");
const os = { win32: "windows", darwin: "darwin", linux: "linux" }[
  process.platform
];
const arch = { x64: "x86_64", arm64: "aarch64" }[process.arch];
if (!os || !arch) throw new Error("Unsupported packaging host");
const platform = `${os}-${arch}`;
const stem = `q-note_${version}_${platform}`;
const output = join(root, "dist", "packages");
const stage = join(output, stem);
const executable = process.platform === "win32" ? "q-note.exe" : "q-note";
const binary =
  process.env.Q_NOTE_PACKAGE_BINARY ||
  join(
    process.env.CARGO_TARGET_DIR || join(root, "target"),
    "release",
    executable,
  );
mkdirSync(stage, { recursive: true });
const run = (command, args) => {
  const result = spawnSync(command, args, { cwd: root, stdio: "inherit" });
  if (result.error) throw result.error;
  if (result.status !== 0)
    throw new Error(`${command} exited with ${result.status}`);
};
let packagedBinary;
if (os === "darwin") {
  const app = join(stage, "Q Note.app", "Contents");
  mkdirSync(join(app, "MacOS"), { recursive: true });
  mkdirSync(join(app, "Resources"), { recursive: true });
  packagedBinary = join(app, "MacOS", "q-note");
  copyFileSync(binary, packagedBinary);
  chmodSync(packagedBinary, 0o755);
  copyFileSync(
    join(root, "desktop/resources/icon.icns"),
    join(app, "Resources", "icon.icns"),
  );
  writeFileSync(
    join(app, "Info.plist"),
    `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleName</key><string>Q Note</string>
<key>CFBundleDisplayName</key><string>Q Note</string>
<key>CFBundleIdentifier</key><string>com.win11.q-note</string>
<key>CFBundleExecutable</key><string>q-note</string>
<key>CFBundleIconFile</key><string>icon.icns</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>${version}</string>
<key>CFBundleVersion</key><string>${version}</string>
<key>LSMinimumSystemVersion</key><string>12.0</string>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
`,
  );
  run("codesign", [
    "--force",
    "--sign",
    "-",
    "--identifier",
    "com.win11.q-note",
    packagedBinary,
  ]);
  run("ditto", [
    "-c",
    "-k",
    "--sequesterRsrc",
    "--keepParent",
    join(stage, "Q Note.app"),
    join(output, `${stem}.app.zip`),
  ]);
} else {
  packagedBinary = join(stage, executable);
  copyFileSync(binary, packagedBinary);
  if (os === "linux") chmodSync(packagedBinary, 0o755);
  copyFileSync(join(root, "LICENSE"), join(stage, "LICENSE"));
  copyFileSync(join(root, "app-icon.png"), join(stage, "app-icon.png"));
  if (os === "linux") {
    writeFileSync(
      join(stage, "q-note.desktop"),
      "[Desktop Entry]\nType=Application\nName=Q Note\nExec=q-note\nIcon=q-note\nCategories=Utility;Office;\nTerminal=false\n",
    );
    run("tar", ["-czf", join(output, `${stem}.tar.gz`), "-C", output, stem]);
  } else {
    run("tar", ["-a", "-cf", join(output, `${stem}.zip`), "-C", stage, "."]);
  }
}
const updateName = `${stem}.update${os === "windows" ? ".exe" : ""}`;
copyFileSync(packagedBinary, join(output, updateName));
const bytes = readFileSync(join(output, updateName));
writeFileSync(
  join(output, `${platform}.json`),
  JSON.stringify(
    {
      platform,
      name: updateName,
      format: "binary",
      size: bytes.length,
      sha256: createHash("sha256").update(bytes).digest("hex"),
    },
    null,
    2,
  ) + "\n",
);
console.log(`Packaged ${platform} in ${output}`);
