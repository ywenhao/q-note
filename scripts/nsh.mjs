import { spawn, spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../", import.meta.url));

function pinnedVersion() {
  const manifest = readFileSync(path.join(root, "native/Cargo.toml"), "utf8");
  const [core, updater] = ["nsh-core", "nsh-updater"].map(
    (name) =>
      new RegExp(`^${name}\\s*=\\s*"=(\\d+\\.\\d+\\.\\d+)"\\s*$`, "m").exec(
        manifest,
      )?.[1],
  );
  if (!core || core !== updater)
    throw new Error(
      "Pin nsh-core and nsh-updater to the same stable version in native/Cargo.toml.",
    );
  return core;
}

function toolchain(required) {
  const binary = process.env.Q_NOTE_NSH_BIN || "nsh";
  const result = spawnSync(binary, ["--version"], {
    encoding: "utf8",
    windowsHide: true,
  });
  if (result.error || result.status !== 0)
    throw new Error(
      `Could not run ${binary}. Install stable NSH ${required} on PATH, or set Q_NOTE_NSH_BIN to its executable.`,
    );
  const version = result.stdout.trim();
  const installed = /^nsh (\d+\.\d+\.\d+)(?: \([\da-f]+\))?$/.exec(version);
  if (installed?.[1] !== required)
    throw new Error(
      `This project requires stable NSH ${required}. ${binary} reports ${version}. Select the matching executable with Q_NOTE_NSH_BIN.`,
    );
  return binary;
}

let testData;
try {
  const args = process.argv.slice(2);
  const required = pinnedVersion();
  if (args.length === 1 && args[0] === "--required-version") {
    console.log(required);
  } else {
    const binary = toolchain(required);
    const env = { ...process.env };
    // Use the generated workspace's output even when a Cargo wrapper ignores the target directory.
    env.CARGO_TARGET_DIR ||= path.join(root, ".nsh", "rust", "target");
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
  }
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
} finally {
  if (testData) rmSync(testData, { recursive: true, force: true });
}
