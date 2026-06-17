use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, Manager};

use crate::repository::{github_latest_release_api_url, github_update_manifest_urls};

const USER_AGENT: &str = concat!("Q-Note/", env!("CARGO_PKG_VERSION"));
const DOWNLOAD_PROGRESS_EVENT: &str = "q-note-update-download-progress";
const UPDATE_TMP_DIR_NAME: &str = "tmp";
const UPDATE_CLEANUP_FILE_NAME: &str = "update-packages.json";
const IP_REGION_TIMEOUT_SECONDS: u64 = 3;

#[derive(Default)]
pub struct UpdateDownloadState {
    active: Mutex<bool>,
    cancel_requested: AtomicBool,
}

struct ActiveDownloadGuard {
    state: Arc<UpdateDownloadState>,
}

impl Drop for ActiveDownloadGuard {
    fn drop(&mut self) {
        if let Ok(mut active) = self.state.active.lock() {
            *active = false;
        }
        self.state.cancel_requested.store(false, Ordering::SeqCst);
    }
}

impl UpdateDownloadState {
    fn begin(self: &Arc<Self>) -> Result<ActiveDownloadGuard, String> {
        let mut active = self
            .active
            .lock()
            .map_err(|_| "Unable to lock update download state".to_string())?;

        if *active {
            return Err("update-download-active".to_string());
        }

        *active = true;
        self.cancel_requested.store(false, Ordering::SeqCst);
        Ok(ActiveDownloadGuard {
            state: self.clone(),
        })
    }
}

#[derive(Debug)]
enum UpdateDownloadError {
    Cancelled,
    Failed(String),
}

impl From<std::io::Error> for UpdateDownloadError {
    fn from(error: std::io::Error) -> Self {
        Self::Failed(error.to_string())
    }
}

impl From<reqwest::Error> for UpdateDownloadError {
    fn from(error: reqwest::Error) -> Self {
        Self::Failed(error.to_string())
    }
}

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<GithubAsset>,
}

#[derive(Deserialize)]
struct GithubAsset {
    name: String,
    size: u64,
    digest: Option<String>,
    browser_download_url: String,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDownloadSource {
    label: String,
    url: String,
    official: bool,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAsset {
    name: String,
    size: u64,
    digest: Option<String>,
    browser_download_url: String,
    download_urls: Vec<UpdateDownloadSource>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    latest_version: String,
    tag_name: String,
    html_url: String,
    asset: Option<UpdateAsset>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDownloadRequest {
    asset: UpdateAsset,
    version: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateDownloadProgress {
    downloaded: u64,
    file_name: String,
    percent: f64,
    source_label: String,
    total: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDownloadResult {
    file_name: String,
    path: String,
    source_label: String,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadedUpdatePackage {
    path: String,
    version: String,
}

fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(8))
        .timeout(Duration::from_secs(900))
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|error| error.to_string())
}

fn ip_region_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(IP_REGION_TIMEOUT_SECONDS))
        .timeout(Duration::from_secs(IP_REGION_TIMEOUT_SECONDS))
        .redirect(reqwest::redirect::Policy::limited(3))
        .build()
        .map_err(|error| error.to_string())
}

fn read_country_code(payload: &serde_json::Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| payload.get(*key)?.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_uppercase)
}

async fn fetch_country_code(client: &reqwest::Client, url: &str, keys: &[&str]) -> Option<String> {
    let payload = client
        .get(url)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .json::<serde_json::Value>()
        .await
        .ok()?;

    read_country_code(&payload, keys)
}

async fn fetch_release_metadata(
    client: &reqwest::Client,
    url: &str,
) -> Result<GithubRelease, String> {
    client
        .get(url)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .send()
        .await
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?
        .json::<GithubRelease>()
        .await
        .map_err(|error| error.to_string())
}

fn release_metadata_urls_for_region(use_china_mirrors: bool) -> Vec<String> {
    let api_url = github_latest_release_api_url();

    if !use_china_mirrors {
        return vec![api_url];
    }

    let mut urls = github_update_manifest_urls();
    urls.push(api_url);
    urls
}

async fn check_release_sources(
    client: &reqwest::Client,
    current_version: &str,
    urls: Vec<String>,
) -> Result<Option<GithubRelease>, String> {
    let mut saw_release = false;
    let mut last_error = "No update source is available".to_string();

    for url in urls {
        match fetch_release_metadata(client, &url).await {
            Ok(release) => {
                saw_release = true;

                if release.draft
                    || release.prerelease
                    || !is_newer_version(&release.tag_name, current_version)
                {
                    continue;
                }

                return Ok(Some(release));
            }
            Err(error) => last_error = error,
        }
    }

    if saw_release {
        Ok(None)
    } else {
        Err(last_error)
    }
}

async fn should_use_china_download_mirrors() -> bool {
    let Ok(client) = ip_region_client() else {
        return false;
    };

    // Use mirrors only when the public IP region is confirmed as China.
    for (url, keys) in [
        ("https://ipapi.co/json/", &["country_code"][..]),
        ("https://ipinfo.io/json", &["country"][..]),
        ("https://ipwho.is/", &["country_code"][..]),
    ] {
        match fetch_country_code(&client, url, keys).await.as_deref() {
            Some("CN") => return true,
            Some(_) => return false,
            None => {}
        }
    }

    false
}

fn parse_version_parts(value: &str) -> Vec<u64> {
    value
        .trim()
        .trim_start_matches('v')
        .split(|character: char| !character.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse::<u64>().ok())
        .collect()
}

fn is_newer_version(remote: &str, current: &str) -> bool {
    let remote_parts = parse_version_parts(remote);
    let current_parts = parse_version_parts(current);
    let max_len = remote_parts.len().max(current_parts.len());

    for index in 0..max_len {
        let remote_part = *remote_parts.get(index).unwrap_or(&0);
        let current_part = *current_parts.get(index).unwrap_or(&0);

        if remote_part > current_part {
            return true;
        }
        if remote_part < current_part {
            return false;
        }
    }

    false
}

fn asset_score(name: &str) -> i32 {
    let lower_name = name.to_ascii_lowercase();

    if lower_name.ends_with(".sig") || lower_name.ends_with(".zip") {
        return -1;
    }

    match std::env::consts::OS {
        "windows" => {
            if lower_name.ends_with("-setup.exe") {
                120
            } else if lower_name.ends_with(".exe") {
                110
            } else if lower_name.ends_with(".msi") {
                100
            } else {
                -1
            }
        }
        "macos" => {
            if !lower_name.ends_with(".dmg") {
                return -1;
            }

            match std::env::consts::ARCH {
                "aarch64" if lower_name.contains("aarch64") || lower_name.contains("arm64") => 120,
                "x86_64" if lower_name.contains("x64") || lower_name.contains("x86_64") => 120,
                _ => 100,
            }
        }
        "linux" => {
            let package_score = if lower_name.ends_with(".appimage") {
                120
            } else if lower_name.ends_with(".deb") {
                110
            } else if lower_name.ends_with(".rpm") {
                100
            } else {
                -1
            };

            if package_score < 0 {
                return -1;
            }

            match std::env::consts::ARCH {
                "x86_64" if lower_name.contains("amd64") || lower_name.contains("x86_64") => {
                    package_score + 10
                }
                "aarch64" if lower_name.contains("aarch64") || lower_name.contains("arm64") => {
                    package_score + 10
                }
                _ => package_score,
            }
        }
        _ => -1,
    }
}

fn build_download_sources(
    official_url: &str,
    use_china_mirrors: bool,
) -> Vec<UpdateDownloadSource> {
    let mut sources = Vec::new();

    if use_china_mirrors && official_url.starts_with("https://github.com/") {
        for (label, prefix) in [
            ("gh-proxy.com", "https://gh-proxy.com/"),
            ("ghproxy.net", "https://ghproxy.net/"),
            ("ghproxy.site", "https://ghproxy.site/"),
        ] {
            sources.push(UpdateDownloadSource {
                label: label.to_string(),
                url: format!("{prefix}{official_url}"),
                official: false,
            });
        }
    }

    sources.push(UpdateDownloadSource {
        label: "GitHub".to_string(),
        url: official_url.to_string(),
        official: true,
    });

    sources
}

fn pick_release_asset(assets: Vec<GithubAsset>, use_china_mirrors: bool) -> Option<UpdateAsset> {
    assets
        .into_iter()
        .filter_map(|asset| {
            let score = asset_score(&asset.name);
            (score >= 0).then_some((score, asset))
        })
        .max_by_key(|(score, _)| *score)
        .map(|(_, asset)| UpdateAsset {
            download_urls: build_download_sources(&asset.browser_download_url, use_china_mirrors),
            browser_download_url: asset.browser_download_url,
            digest: asset.digest,
            name: asset.name,
            size: asset.size,
        })
}

fn expected_sha256(digest: &Option<String>) -> Option<String> {
    digest
        .as_deref()
        .and_then(|value| value.strip_prefix("sha256:"))
        .map(str::to_ascii_lowercase)
}

fn safe_file_name(name: &str) -> String {
    let sanitized = name
        .chars()
        .map(|character| match character {
            '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => character,
        })
        .collect::<String>()
        .trim()
        .to_string();

    if sanitized.is_empty() {
        "q-note-update.bin".to_string()
    } else {
        sanitized
    }
}

fn update_tmp_dir() -> Result<PathBuf, String> {
    let dir = super::home_dir()?
        .join(super::DATA_DIR_NAME)
        .join(UPDATE_TMP_DIR_NAME);
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    Ok(dir)
}

fn update_cleanup_file() -> Result<PathBuf, String> {
    Ok(update_tmp_dir()?.join(UPDATE_CLEANUP_FILE_NAME))
}

fn read_downloaded_update_packages() -> Vec<DownloadedUpdatePackage> {
    let Ok(path) = update_cleanup_file() else {
        return Vec::new();
    };

    let Ok(content) = fs::read_to_string(path) else {
        return Vec::new();
    };

    serde_json::from_str::<Vec<DownloadedUpdatePackage>>(&content).unwrap_or_default()
}

fn write_downloaded_update_packages(packages: &[DownloadedUpdatePackage]) -> Result<(), String> {
    let path = update_cleanup_file()?;

    if packages.is_empty() {
        if path.exists() {
            fs::remove_file(path).map_err(|error| error.to_string())?;
        }
        return Ok(());
    }

    let content = serde_json::to_string_pretty(packages).map_err(|error| error.to_string())?;
    fs::write(path, content).map_err(|error| error.to_string())
}

fn remember_downloaded_update_package(path: &Path, version: &str) -> Result<(), String> {
    let path = path.to_string_lossy().to_string();
    let mut packages = read_downloaded_update_packages();
    packages.retain(|package| package.path != path);
    packages.push(DownloadedUpdatePackage {
        path,
        version: version.to_string(),
    });
    write_downloaded_update_packages(&packages)
}

fn cleanup_partial_update_downloads() -> Result<(), String> {
    let dir = update_tmp_dir()?;

    for entry in fs::read_dir(dir).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        let is_partial = path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("download"));

        if is_partial {
            let _ = fs::remove_file(path);
        }
    }

    Ok(())
}

pub fn cleanup_installed_update_packages() -> Result<(), String> {
    cleanup_partial_update_downloads()?;

    let packages = read_downloaded_update_packages();
    if packages.is_empty() {
        return Ok(());
    }

    let mut pending = Vec::new();
    for package in packages {
        if is_newer_version(&package.version, env!("CARGO_PKG_VERSION")) {
            pending.push(package);
            continue;
        }

        match fs::remove_file(&package.path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => pending.push(package),
        }
    }

    write_downloaded_update_packages(&pending)
}

fn unique_path(path: PathBuf) -> PathBuf {
    if !path.exists() {
        return path;
    }

    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let stem = path
        .file_stem()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| "q-note-update".to_string());
    let extension = path
        .extension()
        .map(|value| value.to_string_lossy().to_string());

    for index in 1..1000 {
        let file_name = match &extension {
            Some(extension) => format!("{stem} ({index}).{extension}"),
            None => format!("{stem} ({index})"),
        };
        let candidate = parent.join(file_name);

        if !candidate.exists() {
            return candidate;
        }
    }

    path
}

fn emit_progress(
    app: &AppHandle,
    file_name: &str,
    downloaded: u64,
    total: Option<u64>,
    source_label: &str,
) {
    let percent = total
        .filter(|value| *value > 0)
        .map(|value| (downloaded as f64 / value as f64 * 100.0).clamp(0.0, 100.0))
        .unwrap_or(0.0);

    let _ = app.emit_to(
        "main",
        DOWNLOAD_PROGRESS_EVENT,
        UpdateDownloadProgress {
            downloaded,
            file_name: file_name.to_string(),
            percent,
            source_label: source_label.to_string(),
            total,
        },
    );
}

async fn download_from_source(
    app: &AppHandle,
    client: &reqwest::Client,
    source: &UpdateDownloadSource,
    asset: &UpdateAsset,
    target_path: &Path,
    temp_path: &Path,
    state: &UpdateDownloadState,
) -> Result<UpdateDownloadResult, UpdateDownloadError> {
    if state.cancel_requested.load(Ordering::SeqCst) {
        return Err(UpdateDownloadError::Cancelled);
    }

    emit_progress(
        app,
        &asset.name,
        0,
        Some(asset.size).filter(|size| *size > 0),
        &source.label,
    );

    let response = client
        .get(&source.url)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(UpdateDownloadError::Failed(format!(
            "{} returned {}",
            source.label,
            response.status()
        )));
    }

    let total = response
        .content_length()
        .filter(|value| *value > 0)
        .or_else(|| Some(asset.size).filter(|value| *value > 0));
    let mut stream = response.bytes_stream();
    let mut file = fs::File::create(temp_path)?;
    let mut hasher = Sha256::new();
    let mut downloaded = 0u64;
    let mut last_emit = Instant::now();

    while let Some(chunk) = stream.next().await {
        if state.cancel_requested.load(Ordering::SeqCst) {
            return Err(UpdateDownloadError::Cancelled);
        }

        let chunk = chunk?;
        file.write_all(&chunk)?;
        hasher.update(&chunk);
        downloaded += chunk.len() as u64;

        if last_emit.elapsed() >= Duration::from_millis(120) {
            emit_progress(app, &asset.name, downloaded, total, &source.label);
            last_emit = Instant::now();
        }
    }

    if state.cancel_requested.load(Ordering::SeqCst) {
        return Err(UpdateDownloadError::Cancelled);
    }

    file.flush()?;
    drop(file);

    if let Some(expected_digest) = expected_sha256(&asset.digest) {
        let actual_digest = format!("{:x}", hasher.finalize());
        if actual_digest != expected_digest {
            return Err(UpdateDownloadError::Failed(format!(
                "{} checksum mismatch",
                source.label
            )));
        }
    }

    fs::rename(temp_path, target_path)?;
    emit_progress(app, &asset.name, downloaded, total, &source.label);

    Ok(UpdateDownloadResult {
        file_name: asset.name.clone(),
        path: target_path.to_string_lossy().to_string(),
        source_label: source.label.clone(),
    })
}

#[tauri::command]
pub async fn check_update(current_version: String) -> Result<Option<UpdateInfo>, String> {
    let client = http_client()?;
    let use_china_mirrors = should_use_china_download_mirrors().await;
    let urls = release_metadata_urls_for_region(use_china_mirrors);
    let Some(release) = check_release_sources(&client, &current_version, urls).await? else {
        return Ok(None);
    };

    Ok(Some(UpdateInfo {
        asset: pick_release_asset(release.assets, use_china_mirrors),
        html_url: release.html_url,
        latest_version: release.tag_name.trim_start_matches('v').to_string(),
        tag_name: release.tag_name,
    }))
}

#[tauri::command]
pub async fn download_update(
    app: AppHandle,
    request: UpdateDownloadRequest,
) -> Result<UpdateDownloadResult, String> {
    let state = app.state::<Arc<UpdateDownloadState>>().inner().clone();
    let _guard = state.begin()?;
    let client = http_client()?;
    let file_name = safe_file_name(&request.asset.name);
    let target_path = unique_path(update_tmp_dir()?.join(&file_name));
    let temp_path = target_path.with_extension(format!(
        "{}download",
        target_path
            .extension()
            .map(|value| format!("{}.", value.to_string_lossy()))
            .unwrap_or_default()
    ));
    let mut last_error = "No download source is available".to_string();

    for source in &request.asset.download_urls {
        match download_from_source(
            &app,
            &client,
            source,
            &request.asset,
            &target_path,
            &temp_path,
            &state,
        )
        .await
        {
            Ok(result) => {
                remember_downloaded_update_package(&target_path, &request.version)?;
                return Ok(result);
            }
            Err(UpdateDownloadError::Cancelled) => {
                let _ = fs::remove_file(&temp_path);
                return Err("update-download-cancelled".to_string());
            }
            Err(UpdateDownloadError::Failed(error)) => {
                let _ = fs::remove_file(&temp_path);
                last_error = error;
            }
        }
    }

    Err(last_error)
}

#[tauri::command]
pub fn cancel_update_download(app: AppHandle) {
    let state = app.state::<Arc<UpdateDownloadState>>().inner().clone();
    state.cancel_requested.store(true, Ordering::SeqCst);
}

#[tauri::command]
pub fn install_update_package(path: String) -> Result<(), String> {
    let package_path = PathBuf::from(path);
    if !package_path.is_file() {
        return Err("update-package-missing".to_string());
    }

    #[cfg(target_os = "linux")]
    if package_path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("appimage"))
    {
        use std::os::unix::fs::PermissionsExt;
        use std::process::Command;

        let mut permissions = fs::metadata(&package_path)
            .map_err(|error| error.to_string())?
            .permissions();
        permissions.set_mode(permissions.mode() | 0o755);
        fs::set_permissions(&package_path, permissions).map_err(|error| error.to_string())?;

        Command::new(&package_path)
            .spawn()
            .map_err(|error| error.to_string())?;
        return Ok(());
    }

    tauri_plugin_opener::open_path(package_path, None::<&str>).map_err(|error| error.to_string())
}
