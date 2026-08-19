use anyhow::{Context as _, Result, anyhow, bail};
use chrono::Timelike;
use serde::Deserialize;
use sha2::{Digest as _, Sha256};
use std::{
    collections::HashMap,
    io::{Read as _, Write as _},
    path::PathBuf,
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

pub const PACKAGE_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const REPOSITORY_URL: &str = "https://github.com/ywenhao/q-note";
pub const LATEST_JSON_URL: &str =
    "https://github.com/ywenhao/q-note/releases/latest/download/latest.json";

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateInfo {
    pub version: String,
    pub notes: Option<String>,
    #[serde(rename = "pub_date")]
    pub _pub_date: Option<String>,
    #[serde(default)]
    pub platforms: HashMap<String, PlatformArtifact>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlatformArtifact {
    pub url: String,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub format: Option<String>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: Option<u64>,
}

pub struct DownloadedUpdate {
    _directory: tempfile::TempDir,
    path: PathBuf,
}

impl UpdateInfo {
    pub fn current_binary(&self) -> Option<&PlatformArtifact> {
        let artifact = self.platforms.get(current_platform_key()?)?;
        let sha256 = artifact.sha256.as_deref()?;
        let size = artifact.size?;
        if artifact.format.as_deref() != Some("binary")
            || size == 0
            || sha256.len() != 64
            || !sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
            || !artifact.url.starts_with("https://")
        {
            return None;
        }
        Some(artifact)
    }
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

pub fn download_update(
    info: &UpdateInfo,
    cancelled: &Arc<AtomicBool>,
    mut on_progress: impl FnMut(DownloadProgress),
) -> Result<DownloadedUpdate> {
    let artifact = info
        .current_binary()
        .ok_or_else(|| anyhow!("no compatible binary update for this platform"))?;
    let expected_size = artifact.size.expect("validated binary size");
    let expected_sha256 = artifact
        .sha256
        .as_deref()
        .expect("validated binary hash")
        .to_ascii_lowercase();
    let mut response = ureq::get(&artifact.url)
        .call()
        .map_err(|error| anyhow!("update download failed: {error}"))?;
    let response_size = response.body().content_length();
    if let Some(response_size) = response_size
        && response_size != expected_size
    {
        bail!("update size header mismatch");
    }

    let directory = tempfile::Builder::new()
        .prefix("q-note-update-")
        .tempdir()
        .context("create update directory")?;
    let filename = if cfg!(target_os = "windows") {
        "q-note-update.exe"
    } else {
        "q-note-update"
    };
    let path = directory.path().join(filename);
    let mut file = std::fs::File::create(&path).context("create downloaded update")?;
    let mut reader = response.body_mut().as_reader();
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut downloaded = 0_u64;
    on_progress(DownloadProgress {
        downloaded,
        total: Some(expected_size),
    });

    loop {
        if cancelled.load(Ordering::Relaxed) {
            bail!("update cancelled");
        }
        let read = reader.read(&mut buffer).context("read update response")?;
        if read == 0 {
            break;
        }
        downloaded = downloaded
            .checked_add(read as u64)
            .ok_or_else(|| anyhow!("update size overflow"))?;
        if downloaded > expected_size {
            bail!("downloaded update exceeds declared size");
        }
        file.write_all(&buffer[..read])
            .context("write downloaded update")?;
        hasher.update(&buffer[..read]);
        on_progress(DownloadProgress {
            downloaded,
            total: Some(expected_size),
        });
    }
    file.flush().context("flush downloaded update")?;
    if downloaded != expected_size {
        bail!("downloaded update size mismatch");
    }
    let actual_sha256 = format!("{:x}", hasher.finalize());
    if actual_sha256 != expected_sha256 {
        bail!("downloaded update checksum mismatch");
    }

    Ok(DownloadedUpdate {
        _directory: directory,
        path,
    })
}

pub fn install_and_relaunch(update: DownloadedUpdate) -> Result<()> {
    let current_exe = std::env::current_exe().context("resolve current executable")?;
    self_replace::self_replace(&update.path).context("replace current executable")?;
    Command::new(&current_exe)
        .spawn()
        .context("launch updated application")?;
    Ok(())
}

pub fn open_release_page(version: Option<&str>) {
    let url = match version {
        Some(v) => release_tag_url(v),
        None => releases_url(),
    };
    let _ = open::that(url);
}

/// Delay until the next local 17:00, matching the Tauri daily check.
pub fn duration_until_next_daily_check() -> Duration {
    const TARGET_HOUR: u32 = 17;
    let now = chrono::Local::now();
    let passed = u64::from(now.num_seconds_from_midnight());
    let target = u64::from(TARGET_HOUR) * 3600;
    let wait_secs = if passed < target {
        target - passed
    } else {
        24 * 3600 - passed + target
    };
    Duration::from_secs(wait_secs.max(1))
}

fn current_platform_key() -> Option<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => Some("windows-x86_64"),
        ("linux", "x86_64") => Some("linux-x86_64"),
        ("macos", "x86_64") => Some("darwin-x86_64"),
        ("macos", "aarch64") => Some("darwin-aarch64"),
        _ => None,
    }
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
