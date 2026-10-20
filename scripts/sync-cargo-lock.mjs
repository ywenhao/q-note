import { spawnSync } from "node:child_process";
const result = spawnSync("cargo", ["update", "--workspace", "--offline"], {
  stdio: "inherit",
});
if (result.error) throw result.error;
process.exit(result.status ?? 1);
