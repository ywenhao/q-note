import { readFileSync, readdirSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { createHash } from "node:crypto";

export function createManifest(
  directory,
  repository,
  tag,
  notes = "",
  date = new Date().toISOString(),
) {
  if (!/^[\w.-]+\/[\w.-]+$/.test(repository))
    throw new Error("Invalid repository");
  const version = tag.replace(/^v/, "");
  if (!/^\d+\.\d+\.\d+(?:-[\w.-]+)?$/.test(version))
    throw new Error("Invalid release version");
  const platforms = {};
  for (const file of readdirSync(directory).filter((name) =>
    /^(windows|linux|darwin)-[\w_]+\.json$/.test(name),
  )) {
    const metadata = JSON.parse(readFileSync(join(directory, file), "utf8"));
    if (
      metadata.format !== "binary" ||
      !Number.isSafeInteger(metadata.size) ||
      metadata.size <= 0 ||
      !/^[a-f0-9]{64}$/.test(metadata.sha256) ||
      !/^[\w.-]+$/.test(metadata.name) ||
      ![
        "windows-x86_64",
        "linux-x86_64",
        "darwin-aarch64",
        "darwin-x86_64",
      ].includes(metadata.platform)
    )
      throw new Error(`Invalid metadata: ${file}`);
    const bytes = readFileSync(join(directory, metadata.name));
    if (
      file !== `${metadata.platform}.json` ||
      bytes.length !== metadata.size ||
      createHash("sha256").update(bytes).digest("hex") !== metadata.sha256
    )
      throw new Error(`Artifact does not match metadata: ${file}`);
    platforms[metadata.platform] = {
      url: `https://github.com/${repository}/releases/download/${encodeURIComponent(tag)}/${encodeURIComponent(metadata.name)}`,
      format: "binary",
      size: metadata.size,
      sha256: metadata.sha256,
    };
  }
  if (!Object.keys(platforms).length)
    throw new Error("No native updater artifacts");
  return { version, notes, pub_date: date, platforms };
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
) {
  const directory = resolve(process.argv[2] || "dist/packages");
  const manifest = createManifest(
    directory,
    process.env.GITHUB_REPOSITORY || "ywenhao/q-note",
    process.env.RELEASE_TAG || "",
    process.env.RELEASE_NOTES || "",
  );
  writeFileSync(
    join(directory, "latest.json"),
    JSON.stringify(manifest, null, 2) + "\n",
  );
  console.log(
    `Generated latest.json for ${Object.keys(manifest.platforms).join(", ")}`,
  );
}
