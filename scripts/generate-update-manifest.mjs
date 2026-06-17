import { writeFile } from "node:fs/promises";
import packageJson from "../package.json" with { type: "json" };

const FALLBACK_REPOSITORY = "ywenhao/q-note";
const UPDATE_MANIFEST_PATH = new URL("../update.json", import.meta.url);

function repositoryPathFromPackage() {
  const repository = packageJson.repository;
  const repositoryUrl = typeof repository === "string" ? repository : repository?.url;

  if (!repositoryUrl) {
    return FALLBACK_REPOSITORY;
  }

  const normalized = repositoryUrl
    .replace(/^git\+/, "")
    .replace(/^https:\/\/github\.com\//, "")
    .replace(/^http:\/\/github\.com\//, "")
    .replace(/^ssh:\/\/git@github\.com\//, "")
    .replace(/^git@github\.com:/, "")
    .replace(/\.git$/, "")
    .replace(/^\/+|\/+$/g, "");

  const [owner, repo] = normalized.split("/");

  return owner && repo ? `${owner}/${repo}` : FALLBACK_REPOSITORY;
}

async function readRelease(repository, tagName) {
  const token = process.env.GITHUB_TOKEN;
  const response = await fetch(
    `https://api.github.com/repos/${repository}/releases/tags/${tagName}`,
    {
      headers: {
        Accept: "application/vnd.github+json",
        ...(token ? { Authorization: `Bearer ${token}` } : {}),
        "User-Agent": `${packageJson.name}/update-manifest`,
        "X-GitHub-Api-Version": "2022-11-28",
      },
    },
  );

  if (!response.ok) {
    throw new Error(`GitHub release API returned ${response.status}`);
  }

  return response.json();
}

function toUpdateManifest(release) {
  return {
    tag_name: release.tag_name,
    html_url: release.html_url,
    draft: Boolean(release.draft),
    prerelease: Boolean(release.prerelease),
    assets: [...(release.assets ?? [])]
      .sort((left, right) => left.name.localeCompare(right.name))
      .map((asset) => ({
        name: asset.name,
        size: asset.size,
        digest: asset.digest ?? null,
        browser_download_url: asset.browser_download_url,
      })),
  };
}

const repository = repositoryPathFromPackage();
const tagName = process.env.GITHUB_REF_NAME ?? `v${packageJson.version}`;
const release = await readRelease(repository, tagName);
const manifest = toUpdateManifest(release);

await writeFile(UPDATE_MANIFEST_PATH, `${JSON.stringify(manifest, null, 2)}\n`);
console.log(`Generated update.json for ${repository}@${manifest.tag_name}`);
