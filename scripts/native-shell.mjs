import { createHash } from "node:crypto";
import {
  existsSync,
  mkdirSync,
  readFileSync,
  writeFileSync,
  openSync,
  closeSync,
  watch,
  symlinkSync,
  copyFileSync,
} from "node:fs";
import { resolve, join } from "node:path";
import { spawn, spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const root = resolve(fileURLToPath(new URL("..", import.meta.url)));
const version = "0.1.1";
const commit = "85be2a2e34ab4879acfa74bde3c5bbdc3cc737d0";
const cache = join(root, ".tools", "nsh", version);
const executable = process.platform === "win32" ? "nsh.exe" : "nsh";
const cli = process.env.NSH_BIN || join(cache, "nsh", "bin", executable);
const source = process.env.NSH_SOURCE || join(cache, "source");
const env = {
  ...process.env,
  NSH_SOURCE: source,
  CARGO_TARGET_DIR: resolve(
    process.env.CARGO_TARGET_DIR || join(root, "target"),
  ),
};
const entries = [
  ["App", "main"],
  ["EditorApp", "editor"],
  ["DockApp", "dock"],
];

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: root,
    env,
    stdio: "inherit",
    ...options,
  });
  if (result.error) throw result.error;
  if (result.status !== 0)
    throw new Error(`${command} exited with ${result.status}`);
}

function setup() {
  mkdirSync(cache, { recursive: true });
  if (!existsSync(join(source, "Cargo.toml"))) {
    mkdirSync(source, { recursive: true });
    const archive = join(cache, "source.tar.gz");
    const output = openSync(archive, "w");
    try {
      run("gh", ["api", `repos/longbridge/nsh/tarball/${commit}`], {
        stdio: ["ignore", output, "inherit"],
      });
    } finally {
      closeSync(output);
    }
    run("tar", ["-xf", archive, "-C", source, "--strip-components=1"]);
    writeFileSync(join(source, ".q-note-commit"), `${commit}\n`);
  }
  if (!process.env.NSH_SOURCE) {
    const sdkTarget = join(env.CARGO_TARGET_DIR, "nsh-sdk");
    mkdirSync(sdkTarget, { recursive: true });
    if (!existsSync(join(source, "target")))
      symlinkSync(
        sdkTarget,
        join(source, "target"),
        process.platform === "win32" ? "junction" : "dir",
      );
  }
  if (
    !existsSync(cli) &&
    process.platform === "darwin" &&
    process.arch === "x64"
  ) {
    run(
      "cargo",
      [
        "build",
        "--release",
        "--locked",
        "--manifest-path",
        join(source, "Cargo.toml"),
        "-p",
        "nsh-cli",
      ],
      { env: { ...env, CARGO_TARGET_DIR: join(source, "target") } },
    );
    mkdirSync(join(cache, "nsh", "bin"), { recursive: true });
    copyFileSync(join(source, "target", "release", executable), cli);
  }
  if (!existsSync(cli)) {
    const platform = {
      linux: "linux-x86_64.tar.gz",
      darwin: "macos-arm64.tar.gz",
      win32: "windows-x86_64.zip",
    }[process.platform];
    if (
      !platform ||
      (process.arch !== "x64" &&
        !(process.platform === "darwin" && process.arch === "arm64"))
    )
      throw new Error(
        "No NSH 0.1.1 CLI for this host. Set NSH_BIN and NSH_SOURCE to a compatible build.",
      );
    const asset = `nsh-${platform}`;
    run("gh", [
      "release",
      "download",
      `v${version}`,
      "--repo",
      "longbridge/nsh",
      "--pattern",
      asset,
      "--pattern",
      `${asset}.sha256`,
      "--dir",
      cache,
      "--clobber",
    ]);
    const expected = readFileSync(join(cache, `${asset}.sha256`), "utf8")
      .trim()
      .split(/\s+/)[0];
    const actual = createHash("sha256")
      .update(readFileSync(join(cache, asset)))
      .digest("hex");
    if (actual !== expected) throw new Error("NSH download checksum mismatch");
    run("tar", ["-xf", join(cache, asset), "-C", cache]);
    if (!existsSync(cli)) throw new Error(`NSH archive did not contain ${cli}`);
  }
}

function buildUI(check = false) {
  setup();
  const mark = `export const mark = "data:image/png;base64,${readFileSync(join(root, "app-icon.png")).toString("base64")}";\n`;
  writeFileSync(join(root, "ui", "mark.ts"), mark);
  for (const [entry, role] of entries)
    run(
      cli,
      check
        ? ["check", `ui/${entry}.tsx`]
        : ["build", `ui/${entry}.tsx`, "--out", `dist/native/${role}`],
    );
}

async function dev() {
  let app;
  let busy = false;
  let pending = false;
  const rebuild = () => {
    if (busy) {
      pending = true;
      return;
    }
    busy = true;
    app?.kill("SIGTERM");
    try {
      buildUI();
      run("cargo", ["build", "-p", "q-note", "--locked"]);
      app = spawn(
        join(
          env.CARGO_TARGET_DIR,
          "debug",
          process.platform === "win32" ? "q-note.exe" : "q-note",
        ),
        [],
        { cwd: root, env, stdio: "inherit" },
      );
    } catch (error) {
      console.error(error.message);
    } finally {
      busy = false;
      if (pending) {
        pending = false;
        rebuild();
      }
    }
  };
  rebuild();
  let timer;
  const watchers = ["ui", "native/src", "desktop/src"].map((dir) =>
    watch(join(root, dir), { recursive: true }, (_, name) => {
      if (!name || name.includes(".nsh") || name === "mark.ts") return;
      clearTimeout(timer);
      timer = setTimeout(rebuild, 300);
    }),
  );
  const stop = () => {
    app?.kill("SIGTERM");
    watchers.forEach((w) => w.close());
    clearTimeout(timer);
    process.exit();
  };
  process.on("SIGINT", stop);
  process.on("SIGTERM", stop);
}

try {
  switch (process.argv[2] || "build") {
    case "setup":
      setup();
      break;
    case "check":
      buildUI(true);
      break;
    case "ui":
      buildUI();
      break;
    case "dev":
      await dev();
      break;
    case "build":
      buildUI(true);
      buildUI();
      run("cargo", [
        "build",
        "-p",
        "q-note",
        "--release",
        "--locked",
        ...process.argv.slice(3),
      ]);
      break;
    default:
      throw new Error("Expected setup, check, ui, dev or build");
  }
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
}
