use anyhow::{Result, anyhow};
use serde::Deserialize;

pub const PACKAGE_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const REPOSITORY_URL: &str = "https://github.com/ywenhao/q-note";
pub const LATEST_JSON_URL: &str =
    "https://github.com/ywenhao/q-note/releases/latest/download/latest.json";

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct UpdateInfo {
    pub version: String,
    pub notes: Option<String>,
    pub pub_date: Option<String>,
    pub platforms: Option<serde_json::Value>,
}

pub fn release_tag_url(version: &str) -> String {
    format!("{REPOSITORY_URL}/releases/tag/v{version}")
}

pub fn releases_url() -> String {
    format!("{REPOSITORY_URL}/releases")
}

pub fn check_for_update() -> Result<Option<UpdateInfo>> {
    let info: UpdateInfo = ureq::get(LATEST_JSON_URL)
        .call()
        .map_err(|e| anyhow!("update check failed: {e}"))?
        .body_mut()
        .read_json()
        .map_err(|e| anyhow!("invalid latest.json: {e}"))?;

    if is_newer(&info.version, PACKAGE_VERSION) {
        Ok(Some(info))
    } else {
        Ok(None)
    }
}

pub fn open_release_page(version: Option<&str>) {
    let url = match version {
        Some(v) => release_tag_url(v),
        None => releases_url(),
    };
    let _ = open::that(url);
}

fn is_newer(remote: &str, current: &str) -> bool {
    parse_version(remote) > parse_version(current)
}

fn parse_version(v: &str) -> (u64, u64, u64) {
    let mut parts = v.trim_start_matches('v').split('.');
    let major = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let minor = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let patch = parts
        .next()
        .and_then(|p| {
            p.chars()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>()
                .parse()
                .ok()
        })
        .unwrap_or(0);
    (major, minor, patch)
}
