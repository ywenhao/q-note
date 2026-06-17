use serde::Deserialize;

const PACKAGE_JSON: &str = include_str!("../../package.json");
const FALLBACK_GITHUB_REPOSITORY: &str = "ywenhao/q-note";

#[derive(Deserialize)]
#[serde(untagged)]
enum PackageRepository {
    Url(String),
    Detail { url: Option<String> },
}

#[derive(Deserialize)]
struct PackageJson {
    repository: Option<PackageRepository>,
}

fn package_repository_url() -> Option<String> {
    let package = serde_json::from_str::<PackageJson>(PACKAGE_JSON).ok()?;

    match package.repository? {
        PackageRepository::Url(url) => Some(url),
        PackageRepository::Detail { url } => url,
    }
}

fn github_repository_path_from_url(url: &str) -> Option<String> {
    let normalized = url
        .trim()
        .trim_start_matches("git+")
        .trim_start_matches("https://github.com/")
        .trim_start_matches("http://github.com/")
        .trim_start_matches("ssh://git@github.com/")
        .trim_start_matches("git@github.com:")
        .trim_end_matches(".git")
        .trim_matches('/');

    let mut parts = normalized.split('/');
    let owner = parts.next()?.trim();
    let repo = parts.next()?.trim();

    if owner.is_empty() || repo.is_empty() {
        return None;
    }

    Some(format!("{owner}/{repo}"))
}

pub fn github_repository_path() -> String {
    package_repository_url()
        .as_deref()
        .and_then(github_repository_path_from_url)
        .unwrap_or_else(|| FALLBACK_GITHUB_REPOSITORY.to_string())
}

pub fn github_latest_release_api_url() -> String {
    format!(
        "https://api.github.com/repos/{}/releases/latest",
        github_repository_path()
    )
}

pub fn github_update_manifest_urls() -> Vec<String> {
    let repository = github_repository_path();

    vec![
        format!("https://cdn.jsdelivr.net/gh/{repository}@main/update.json"),
        format!("https://raw.githubusercontent.com/{repository}/main/update.json"),
    ]
}
