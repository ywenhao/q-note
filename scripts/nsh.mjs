import { createHash } from "node:crypto";
import { spawn, spawnSync } from "node:child_process";
import {
  existsSync,
  mkdirSync,
  readFileSync,
  writeFileSync,
  mkdtempSync,
  rmSync,
} from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../", import.meta.url));
const commit = "c5214aa";
const revision = "c5214aaf4bbdf33bc1d59e9595e009b6aa54d702";
const distributions = {
  "win32-x64": [
    "windows-x86_64.zip",
    "bdc30ff621f87c755d2693a0bc43f996a15ebdcd6527c6704ef8f3ad0415c5b3",
  ],
  "darwin-arm64": [
    "macos-arm64.tar.gz",
    "b8a8c2f8cebfa9412717b4e7c4478c9a5be8b098e85c7f5f8dfe2f8f6b43b84a",
  ],
  "darwin-x64": [
    "macos-x86_64.tar.gz",
    "848ab6ae7c71b84f2a4e110c850aff50e28f592a938f281e594f37c5fca74870",
  ],
  "linux-x64": [
    "linux-x86_64.tar.gz",
    "021b445b86df0b900a2489644ea7a182ea4aeae1447f9851753cd215a4101acd",
  ],
  "linux-arm64": [
    "linux-arm64.tar.gz",
    "4fe2a229866b62c2af951dfe0c874a0bf139f0cf30810ad6b8b690dabd6b159a",
  ],
};

function matches(binary) {
  return spawnSync(binary, ["--version"], {
    encoding: "utf8",
    windowsHide: true,
  }).stdout?.includes(commit);
}

function sourceToolchain(dir) {
  const source = path.join(dir, "source");
  const binary = path.join(
    source,
    "target",
    "release",
    process.platform === "win32" ? "nsh.exe" : "nsh",
  );
  if (existsSync(binary) && matches(binary)) return binary;
  console.log(`Building NSH at pinned revision ${commit}…`);
  const run = (command, args) => {
    const result = spawnSync(command, args, {
      stdio: "inherit",
      windowsHide: true,
    });
    if (result.error || result.status !== 0)
      throw (
        result.error ||
        new Error(`Could not build the pinned NSH toolchain: ${command}`)
      );
  };
  if (!existsSync(path.join(source, ".git")))
    run("git", [
      "clone",
      "--filter=blob:none",
      "--no-checkout",
      "https://github.com/longbridge/nsh",
      source,
    ]);
  run("git", ["-C", source, "checkout", "--detach", revision]);
  run("cargo", [
    "build",
    "--release",
    "--manifest-path",
    path.join(source, "Cargo.toml"),
    "-p",
    "nsh-cli",
  ]);
  if (!matches(binary))
    throw new Error("The source build does not match the pinned NSH revision");
  return binary;
}

async function toolchain() {
  if (process.env.Q_NOTE_NSH_BIN) {
    if (!matches(process.env.Q_NOTE_NSH_BIN))
      throw new Error(`Q_NOTE_NSH_BIN must be NSH nightly ${commit}`);
    return process.env.Q_NOTE_NSH_BIN;
  }
  if (matches("nsh")) return "nsh";
  const dir = path.join(root, ".nsh", "toolchain", commit);
  const binary = path.join(
    dir,
    "nsh",
    "bin",
    process.platform === "win32" ? "nsh.exe" : "nsh",
  );
  if (existsSync(binary) && matches(binary)) return binary;
  const distribution = distributions[`${process.platform}-${process.arch}`];
  if (!distribution)
    throw new Error(
      "Install the pinned NSH nightly and set Q_NOTE_NSH_BIN for this platform.",
    );
  mkdirSync(dir, { recursive: true });
  const [name, sha256] = distribution;
  const archive = path.join(dir, `nsh-${name}`);
  if (!existsSync(archive)) {
    console.log(`Downloading official NSH nightly ${commit}…`);
    const response = await fetch(
      `https://github.com/longbridge/nsh/releases/download/nightly/nsh-${name}`,
    );
    if (!response.ok) return sourceToolchain(dir);
    writeFileSync(archive, new Uint8Array(await response.arrayBuffer()));
  }
  if (
    createHash("sha256").update(readFileSync(archive)).digest("hex") !== sha256
  ) {
    console.log(
      "The upstream nightly changed; the downloaded binary was not executed.",
    );
    return sourceToolchain(dir);
  }
  const unpack = spawnSync("tar", ["-xf", archive, "-C", dir], {
    stdio: "inherit",
    windowsHide: true,
  });
  if (unpack.status !== 0 || !matches(binary))
    throw new Error("Could not extract the pinned NSH toolchain.");
  return binary;
}

let testData;
try {
  const binary = await toolchain();
  const args = process.argv.slice(2);
  const env = { ...process.env };
  if (args[0] === "test" && !env.Q_NOTE_DATA_DIR) {
    testData = mkdtempSync(path.join(tmpdir(), "q-note-nsh-test-"));
    env.Q_NOTE_DATA_DIR = testData;
  }
  const child = spawn(binary, args, {
    cwd: root,
    env,
    stdio: "inherit",
    windowsHide: true,
  });
  for (const event of ["SIGINT", "SIGTERM"])
    process.on(event, () => child.kill(event));
  process.exitCode = await new Promise((resolve, reject) => {
    child.on("error", reject);
    child.on("exit", (code) => resolve(code ?? 1));
  });
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
} finally {
  if (testData) rmSync(testData, { recursive: true, force: true });
}
