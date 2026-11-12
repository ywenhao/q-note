import { spawn, spawnSync } from "node:child_process";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../", import.meta.url));
function toolchain() {
  const binary = process.env.Q_NOTE_NSH_BIN || "nsh";
  const result = spawnSync(binary, ["--version"], {
    encoding: "utf8",
    windowsHide: true,
  });
  if (result.error || result.status !== 0)
    throw new Error(
      `Could not run ${binary}. Install NSH 0.5.0 or newer on PATH, or set Q_NOTE_NSH_BIN to its executable.`,
    );
  const version = result.stdout.trim();
  const stable = /^nsh (\d+)\.(\d+)\.(\d+)(?:\s|$)/.exec(version);
  if (!stable || (Number(stable[1]) === 0 && Number(stable[2]) < 5))
    throw new Error(
      `NSH 0.5.0 or newer stable is required; ${binary} reports ${version}.`,
    );
  return binary;
}

let testData;
try {
  const binary = toolchain();
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
