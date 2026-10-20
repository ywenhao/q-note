import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createHash } from "node:crypto";
import { createManifest } from "../scripts/updater.mjs";

test("release metadata maps native binaries and rejects unusable artifacts", () => {
  const dir = mkdtempSync(join(tmpdir(), "q-note-release-"));
  try {
    assert.throws(() => createManifest(dir, "ywenhao/q-note", "v1.2.3"));
    const metadata = {
      platform: "windows-x86_64",
      name: "q-note_1.2.3_windows-x86_64.update.exe",
      format: "binary",
      size: 1024,
      sha256: createHash("sha256").update(Buffer.alloc(1024, 42)).digest("hex"),
    };
    const file = join(dir, "windows-x86_64.json");
    writeFileSync(join(dir, metadata.name), Buffer.alloc(1024, 42));
    writeFileSync(file, JSON.stringify(metadata));
    const result = createManifest(
      dir,
      "ywenhao/q-note",
      "v1.2.3",
      "Notes",
      "2026-10-20T00:00:00Z",
    );
    assert.equal(result.version, "1.2.3");
    assert.equal(result.notes, "Notes");
    assert.deepEqual(result.platforms[metadata.platform], {
      url: "https://github.com/ywenhao/q-note/releases/download/v1.2.3/q-note_1.2.3_windows-x86_64.update.exe",
      format: "binary",
      size: 1024,
      sha256: metadata.sha256,
    });
    for (const invalid of [
      { size: 0 },
      { sha256: "bad" },
      { name: "../../bad" },
      { format: "zip" },
      { platform: "unknown" },
    ]) {
      writeFileSync(file, JSON.stringify({ ...metadata, ...invalid }));
      assert.throws(() => createManifest(dir, "ywenhao/q-note", "v1.2.3"));
    }
    assert.throws(() => createManifest(dir, "invalid", "v1.2.3"));
    assert.throws(() => createManifest(dir, "ywenhao/q-note", "bad"));
    writeFileSync(file, JSON.stringify(metadata));
    writeFileSync(join(dir, metadata.name), Buffer.alloc(1024, 43));
    assert.throws(() => createManifest(dir, "ywenhao/q-note", "v1.2.3"));
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
