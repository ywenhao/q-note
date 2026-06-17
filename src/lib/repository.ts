import packageJson from "../../package.json";

type PackageRepository =
  | string
  | {
      url?: string;
    };

type PackageJson = {
  repository?: PackageRepository;
  version: string;
};

const metadata = packageJson as PackageJson;
const FALLBACK_REPOSITORY_URL = "https://github.com/ywenhao/q-note";

export const PACKAGE_VERSION = metadata.version;

function readRepositoryUrl() {
  const repository = metadata.repository;

  if (typeof repository === "string") {
    return repository;
  }

  return repository?.url ?? FALLBACK_REPOSITORY_URL;
}

export function getRepositoryWebUrl() {
  return readRepositoryUrl()
    .replace(/^git\+/, "")
    .replace(/^git@github\.com:/, "https://github.com/")
    .replace(/\.git$/, "");
}

export function getReleaseTagUrl(version: string) {
  return `${getRepositoryWebUrl()}/releases/tag/v${version}`;
}
