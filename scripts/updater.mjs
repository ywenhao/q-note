const repository = process.env.GITHUB_REPOSITORY;
const token = process.env.GITHUB_TOKEN;
const releaseTag = process.env.RELEASE_TAG?.replace(/^refs\/tags\//, "");

if (!repository || !token || !releaseTag) {
  throw new Error("GITHUB_REPOSITORY, GITHUB_TOKEN, and RELEASE_TAG are required");
}

const [owner, repo] = repository.split("/");

async function github(path, init = {}) {
  const response = await fetch(`https://api.github.com${path}`, {
    ...init,
    headers: {
      Accept: "application/vnd.github+json",
      Authorization: `Bearer ${token}`,
      "X-GitHub-Api-Version": "2022-11-28",
      ...init.headers,
    },
  });

  if (!response.ok) {
    const body = await response.text();
    throw new Error(`GitHub API ${response.status} for ${path}: ${body}`);
  }

  if (response.status === 204) {
    return null;
  }

  return response.json();
}

async function getSignature(url) {
  const response = await fetch(url, {
    headers: { Accept: "application/octet-stream" },
  });
  if (!response.ok) {
    throw new Error(`Failed to download signature ${url}: ${response.status}`);
  }
  const text = await response.text();
  return text.trim().split("\n").at(-1) ?? text.trim();
}

function setPlatform(platforms, key, url, signature) {
  if (!url || !signature) {
    return;
  }
  platforms[key] = { signature, url };
}

async function buildPlatforms(assets) {
  const platforms = {};

  for (const asset of assets) {
    const { name, browser_download_url: url } = asset;

    if (name.endsWith("x64-setup.exe.sig")) {
      const signature = await getSignature(url);
      setPlatform(platforms, "windows-x86_64", url.replace(/\.sig$/, ""), signature);
      setPlatform(platforms, "windows-x86_64-nsis", url.replace(/\.sig$/, ""), signature);
      continue;
    }

    if (name.endsWith("arm64-setup.exe.sig")) {
      const signature = await getSignature(url);
      setPlatform(platforms, "windows-aarch64", url.replace(/\.sig$/, ""), signature);
      setPlatform(platforms, "windows-aarch64-nsis", url.replace(/\.sig$/, ""), signature);
      continue;
    }

    if (name.endsWith("aarch64.app.tar.gz.sig")) {
      const signature = await getSignature(url);
      const bundleUrl = url.replace(/\.sig$/, "");
      setPlatform(platforms, "darwin-aarch64", bundleUrl, signature);
      setPlatform(platforms, "darwin-aarch64-app", bundleUrl, signature);
      continue;
    }

    if (name.endsWith(".app.tar.gz.sig") && !name.includes("aarch")) {
      const signature = await getSignature(url);
      const bundleUrl = url.replace(/\.sig$/, "");
      setPlatform(platforms, "darwin-x86_64", bundleUrl, signature);
      setPlatform(platforms, "darwin-x86_64-app", bundleUrl, signature);
      continue;
    }

    if (name.endsWith("amd64.AppImage.tar.gz.sig")) {
      const signature = await getSignature(url);
      const bundleUrl = url.replace(/\.sig$/, "");
      setPlatform(platforms, "linux-x86_64-appimage", bundleUrl, signature);
      setPlatform(platforms, "linux-x86_64", bundleUrl, signature);
      continue;
    }

    if (name.endsWith("amd64.deb.sig")) {
      const signature = await getSignature(url);
      const bundleUrl = url.replace(/\.sig$/, "");
      setPlatform(platforms, "linux-x86_64-deb", bundleUrl, signature);
      continue;
    }

    if (name.endsWith("x86_64.rpm.sig")) {
      const signature = await getSignature(url);
      const bundleUrl = url.replace(/\.sig$/, "");
      setPlatform(platforms, "linux-x86_64-rpm", bundleUrl, signature);
    }
  }

  return platforms;
}

async function uploadReleaseAsset(release, fileName, content, contentType) {
  const uploadUrl = release.upload_url.replace(
    "{?name,label}",
    `?name=${encodeURIComponent(fileName)}`,
  );

  const response = await fetch(uploadUrl, {
    method: "POST",
    headers: {
      Accept: "application/vnd.github+json",
      Authorization: `Bearer ${token}`,
      "X-GitHub-Api-Version": "2022-11-28",
      "Content-Type": contentType,
    },
    body: content,
  });

  if (!response.ok) {
    const body = await response.text();
    throw new Error(`GitHub upload ${response.status} for ${fileName}: ${body}`);
  }

  return response.json();
}

async function replaceLatestJson(release, latestJson) {
  for (const asset of release.assets) {
    if (asset.name === "latest.json") {
      await github(`/repos/${owner}/${repo}/releases/assets/${asset.id}`, {
        method: "DELETE",
      });
    }
  }

  const content = JSON.stringify(latestJson, null, 2);
  await uploadReleaseAsset(release, "latest.json", content, "application/json");
}

async function main() {
  const release = await github(`/repos/${owner}/${repo}/releases/tags/${releaseTag}`);
  const platforms = await buildPlatforms(release.assets);

  if (Object.keys(platforms).length === 0) {
    throw new Error("No updater artifacts found in release assets");
  }

  const latestJson = {
    version: releaseTag.replace(/^v/, ""),
    notes: release.body ?? "",
    pub_date: release.published_at ?? new Date().toISOString(),
    platforms,
  };

  await replaceLatestJson(release, latestJson);
  console.log(
    `Updated latest.json for ${releaseTag} with keys: ${Object.keys(platforms).join(", ")}`,
  );
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
