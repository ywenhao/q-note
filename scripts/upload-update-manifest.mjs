import { readFile } from "node:fs/promises";
import packageJson from "../package.json" with { type: "json" };

const ASSET_NAME = "update.json";
const MANIFEST_PATH = process.env.UPDATE_MANIFEST_PATH ?? ".manifest/update.json";

function requireEnv(name) {
  const value = process.env[name];

  if (!value) {
    throw new Error(`${name} is required`);
  }

  return value;
}

async function githubRequest(url, options = {}) {
  const response = await fetch(url, {
    ...options,
    headers: {
      Accept: "application/vnd.github+json",
      Authorization: `Bearer ${requireEnv("GITHUB_TOKEN")}`,
      "User-Agent": `${packageJson.name}/update-manifest-upload`,
      "X-GitHub-Api-Version": "2022-11-28",
      ...options.headers,
    },
  });

  if (!response.ok) {
    const body = await response.text();
    throw new Error(`GitHub API returned ${response.status}: ${body}`);
  }

  return response;
}

async function readRelease() {
  const repository = requireEnv("GITHUB_REPOSITORY");
  const tagName = requireEnv("GITHUB_REF_NAME");
  const response = await githubRequest(
    `https://api.github.com/repos/${repository}/releases/tags/${tagName}`,
  );

  return response.json();
}

async function deleteExistingAsset(repository, asset) {
  if (!asset) {
    return;
  }

  await githubRequest(`https://api.github.com/repos/${repository}/releases/assets/${asset.id}`, {
    method: "DELETE",
  });
}

async function uploadAsset(release, content) {
  const repository = requireEnv("GITHUB_REPOSITORY");
  const uploadUrl = release.upload_url.replace(/\{.*$/, "");

  await githubRequest(`${uploadUrl}?name=${encodeURIComponent(ASSET_NAME)}`, {
    method: "POST",
    headers: {
      "Content-Length": String(content.byteLength),
      "Content-Type": "application/json",
    },
    body: content,
  });

  console.log(`Uploaded ${ASSET_NAME} to ${repository}@${release.tag_name}`);
}

const repository = requireEnv("GITHUB_REPOSITORY");
const release = await readRelease();
const manifest = await readFile(MANIFEST_PATH);
const existingAsset = release.assets?.find((asset) => asset.name === ASSET_NAME);

await deleteExistingAsset(repository, existingAsset);
await uploadAsset(release, manifest);
