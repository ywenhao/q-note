import { spawnSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../", import.meta.url));
const args = process.argv.slice(2);
const kind = args[args.indexOf("--release") + 1] || "patch";
if (!["patch", "minor", "major"].includes(kind))
  throw new Error("Use --release patch, minor or major");

function run(command, argv, capture = false) {
  if (command === "pnpm" && process.env.npm_execpath) {
    argv = /\.[cm]?js$/.test(process.env.npm_execpath)
      ? [process.env.npm_execpath, ...argv]
      : argv;
    command = /\.[cm]?js$/.test(process.env.npm_execpath)
      ? process.execPath
      : process.env.npm_execpath;
  }
  const result = spawnSync(command, argv, {
    cwd: root,
    encoding: "utf8",
    stdio: capture ? "pipe" : "inherit",
    windowsHide: true,
  });
  if (result.error || result.status !== 0)
    throw result.error || new Error(`${command} failed (${result.status})`);
  return result.stdout?.trim() || "";
}

const read = (file) => readFileSync(path.join(root, file), "utf8");
const current = JSON.parse(read("package.json")).version;
const parts = current.split(".").map(Number);
if (parts.length !== 3 || parts.some((n) => !Number.isSafeInteger(n) || n < 0))
  throw new Error("Expected a stable X.Y.Z version");
const index = { major: 0, minor: 1, patch: 2 }[kind];
parts[index]++;
for (let i = index + 1; i < parts.length; i++) parts[i] = 0;
const version = parts.join(".");
const tag = `nsh-v${version}`;
console.log(`NSH release: ${current} -> ${version} (${tag})`);
if (args.includes("--dry-run")) process.exit(0);
if (run("git", ["status", "--porcelain"], true))
  throw new Error("Release requires a clean working tree");

for (const file of ["package.json", "nsh.json"]) {
  const metadata = JSON.parse(read(file));
  if (metadata.version !== current)
    throw new Error(`Version mismatch in ${file}`);
}
const cargo = read("native/Cargo.toml");
const lock = read("native/Cargo.lock");
if (
  !cargo.includes(`version = "${current}"`) ||
  !lock.includes(`name = "q-note-native"\nversion = "${current}"`)
) {
  throw new Error("Native versions are out of sync");
}
for (const file of ["package.json", "nsh.json"]) {
  const metadata = JSON.parse(read(file));
  metadata.version = version;
  if (file === "nsh.json") metadata.build++;
  writeFileSync(
    path.join(root, file),
    JSON.stringify(metadata, null, 2) + "\n",
  );
}
writeFileSync(
  path.join(root, "native/Cargo.toml"),
  cargo.replace(`version = "${current}"`, `version = "${version}"`),
);
writeFileSync(
  path.join(root, "native/Cargo.lock"),
  lock.replace(
    `name = "q-note-native"\nversion = "${current}"`,
    `name = "q-note-native"\nversion = "${version}"`,
  ),
);
for (const script of ["check", "test", "test:native", "test:ui"])
  run("pnpm", [script]);
run("git", [
  "add",
  "--",
  "package.json",
  "nsh.json",
  "native/Cargo.toml",
  "native/Cargo.lock",
]);
run("git", ["commit", "-m", `release: ${tag}`]);
run("git", ["tag", tag]);
run("git", ["push", "--atomic", "origin", "HEAD", `refs/tags/${tag}`]);
