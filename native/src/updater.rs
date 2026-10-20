use anyhow::{Context as _, Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::{
    collections::HashMap,
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpdateInfo {
    pub version: String,
    pub notes: Option<String>,
    #[serde(rename = "pub_date")]
    pub _pub_date: Option<String>,
    #[serde(default)]
    pub platforms: HashMap<String, PlatformArtifact>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
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
            || !valid_download_url(&artifact.url)
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
    let info: UpdateInfo = agent()
        .get(LATEST_JSON_URL)
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
    let mut response = agent()
        .get(&artifact.url)
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
    verify_transfer(
        response.body_mut().as_reader(),
        &mut file,
        expected_size,
        &expected_sha256,
        cancelled,
        &mut on_progress,
    )?;
    file.sync_all().context("sync downloaded update")?;
    Ok(DownloadedUpdate {
        _directory: directory,
        path,
    })
}

fn verify_transfer(
    mut reader: impl std::io::Read,
    mut writer: impl std::io::Write,
    expected_size: u64,
    expected_sha256: &str,
    cancelled: &AtomicBool,
    mut on_progress: impl FnMut(DownloadProgress),
) -> Result<()> {
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
        writer
            .write_all(&buffer[..read])
            .context("write downloaded update")?;
        hasher.update(&buffer[..read]);
        on_progress(DownloadProgress {
            downloaded,
            total: Some(expected_size),
        });
    }
    writer.flush().context("flush downloaded update")?;
    if downloaded != expected_size {
        bail!("downloaded update size mismatch");
    }
    let actual_sha256 = format!("{:x}", hasher.finalize());
    if actual_sha256 != expected_sha256 {
        bail!("downloaded update checksum mismatch");
    }

    Ok(())
}

pub fn install_and_relaunch(update: DownloadedUpdate) -> Result<()> {
    let current_exe = std::env::current_exe().context("resolve current executable")?;
    self_replace::self_replace(&update.path).context("replace current executable")?;
    Command::new(&current_exe)
        .args(["--wait-for", &std::process::id().to_string()])
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

/// Delay until the next local 17:00.
pub fn duration_until_next_daily_check() -> Duration {
    until_daily_check(chrono::Local::now())
}

fn until_daily_check<T: chrono::TimeZone>(now: chrono::DateTime<T>) -> Duration {
    let timezone = now.timezone();
    let mut date = now.date_naive();
    loop {
        if let Some(target) = date
            .and_hms_opt(17, 0, 0)
            .and_then(|time| timezone.from_local_datetime(&time).earliest())
            && target > now
        {
            return (target - now).to_std().unwrap_or(Duration::from_secs(1));
        }
        date = date.succ_opt().expect("next calendar day");
    }
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
    match (
        semver::Version::parse(remote.trim_start_matches('v')),
        semver::Version::parse(current.trim_start_matches('v')),
    ) {
        (Ok(remote), Ok(current)) => remote > current,
        _ => false,
    }
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .https_only(true)
        .timeout_global(Some(Duration::from_secs(600)))
        .timeout_connect(Some(Duration::from_secs(15)))
        .timeout_recv_body(Some(Duration::from_secs(5)))
        .build()
        .into()
}

fn valid_download_url(value: &str) -> bool {
    url::Url::parse(value).is_ok_and(|url| {
        url.scheme() == "https"
            && url.host_str().is_some()
            && url.username().is_empty()
            && url.password().is_none()
            && url.fragment().is_none()
    })
}

pub fn wait_for_previous_process() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    let Some(index) = args.iter().position(|value| value == "--wait-for") else {
        return Ok(());
    };
    let pid: u32 = args
        .get(index + 1)
        .context("Missing previous process ID")?
        .parse()?;
    if pid == 0 || pid == std::process::id() {
        bail!("Invalid previous process ID");
    }
    #[cfg(unix)]
    {
        let start = std::time::Instant::now();
        while unsafe { libc::kill(pid as libc::pid_t, 0) } == 0 {
            if start.elapsed() > Duration::from_secs(30) {
                bail!("Previous application did not exit");
            }
            std::thread::sleep(Duration::from_millis(30));
        }
    }
    #[cfg(target_os = "windows")]
    unsafe {
        use windows_sys::Win32::{
            Foundation::{CloseHandle, WAIT_TIMEOUT},
            System::Threading::{OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject},
        };
        let process = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
        if !process.is_null() {
            let result = WaitForSingleObject(process, 30000);
            CloseHandle(process);
            if result == WAIT_TIMEOUT {
                bail!("Previous application did not exit");
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn daily_check_uses_next_local_seventeen_oclock() {
        use chrono::TimeZone;
        let zone = chrono::FixedOffset::east_opt(8 * 3600).unwrap();
        assert_eq!(
            until_daily_check(zone.with_ymd_and_hms(2026, 10, 20, 16, 59, 59).unwrap()),
            Duration::from_secs(1)
        );
        assert_eq!(
            until_daily_check(zone.with_ymd_and_hms(2026, 10, 20, 17, 0, 0).unwrap()),
            Duration::from_secs(86400)
        );
        assert_eq!(
            until_daily_check(zone.with_ymd_and_hms(2026, 10, 20, 23, 0, 0).unwrap()),
            Duration::from_secs(18 * 3600)
        );
    }
    #[test]
    fn streamed_transfer_rejects_corruption_truncation_oversize_and_cancellation() {
        let bytes = vec![42_u8; 150_000];
        let hash = format!("{:x}", Sha256::digest(&bytes));
        let cancel = AtomicBool::new(false);
        let mut output = Vec::new();
        verify_transfer(
            bytes.as_slice(),
            &mut output,
            bytes.len() as u64,
            &hash,
            &cancel,
            |_| {},
        )
        .unwrap();
        assert_eq!(output, bytes);
        for (size, hash) in [
            (149_999, hash.clone()),
            (150_001, hash.clone()),
            (150_000, "0".repeat(64)),
        ] {
            assert!(
                verify_transfer(bytes.as_slice(), Vec::new(), size, &hash, &cancel, |_| {})
                    .is_err()
            );
        }
        let mut output = Vec::new();
        assert!(
            verify_transfer(
                bytes.as_slice(),
                &mut output,
                bytes.len() as u64,
                &hash,
                &cancel,
                |p| {
                    if p.downloaded > 0 {
                        cancel.store(true, Ordering::Relaxed);
                    }
                }
            )
            .is_err()
        );
        assert!(output.len() < bytes.len());
    }
    #[test]
    fn versions_and_artifact_metadata_reject_invalid_updates() {
        assert!(is_newer("v0.3.0", "0.2.12"));
        assert!(!is_newer("0.2.12-beta.1", "0.2.12"));
        assert!(!is_newer("bad", "0.2.12"));
        for value in [
            "http://example.org/file",
            "https://",
            "https://user:secret@example.org/file",
            "file:///tmp/x",
            "https://example.org/file#fragment",
        ] {
            assert!(!valid_download_url(value), "{value}");
        }
        assert!(valid_download_url(
            "https://github.com/ywenhao/q-note/releases/download/v1/q-note"
        ));
        let mut info = UpdateInfo {
            version: "99.0.0".into(),
            notes: None,
            _pub_date: None,
            platforms: HashMap::new(),
        };
        let Some(key) = current_platform_key() else {
            return;
        };
        let mut artifact = PlatformArtifact {
            url: "https://example.org/q-note".into(),
            format: Some("binary".into()),
            size: Some(100),
            sha256: Some("a".repeat(64)),
        };
        info.platforms.insert(key.into(), artifact.clone());
        assert!(info.current_binary().is_some());
        artifact.sha256 = Some("x".repeat(64));
        info.platforms.insert(key.into(), artifact.clone());
        assert!(info.current_binary().is_none());
        artifact.sha256 = Some("a".repeat(64));
        artifact.format = Some("zip".into());
        info.platforms.insert(key.into(), artifact);
        assert!(info.current_binary().is_none());
    }
}
