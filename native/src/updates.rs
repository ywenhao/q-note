use nsh_updater::{Available, Config, Download, install::Installation};
use serde_json::json;
use std::{
    io::Write,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
use tempfile::NamedTempFile;
use tokio::sync::watch;

type Result<T> = std::result::Result<T, String>;

struct Pending {
    available: Available,
    installation: Installation,
    cancel: watch::Sender<bool>,
    received: Arc<AtomicU64>,
    archive: Option<NamedTempFile>,
}

static PENDING: Mutex<Option<Pending>> = Mutex::new(None);

fn client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .https_only(!cfg!(test))
        .connect_timeout(Duration::from_secs(20))
        .read_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(1800))
        .user_agent("Q-Note-Updater")
        .build()
        .map_err(err)
}

fn configuration() -> Option<(Config, Installation)> {
    let executable = std::env::current_exe().ok()?;
    let resources = if cfg!(target_os = "macos") {
        executable.parent()?.parent()?.join("Resources/nsh")
    } else {
        executable.parent()?.join("nsh")
    };
    let bytes = std::fs::read(resources.join("assets").join(nsh_updater::CONFIG_ASSET)).ok()?;
    let config = Config::parse(&bytes).ok()?;
    if config.id != "com.win11.q-note" || config.version != env!("CARGO_PKG_VERSION") {
        return None;
    }
    let installation = Installation::current(&config.id, &bytes)?;
    installation.remove_leftovers();
    Some((config, installation))
}

pub async fn check() -> Result<Option<String>> {
    let Some((config, installation)) = configuration() else {
        return Ok(None);
    };
    let http = client()?;
    let repository = config
        .github
        .as_deref()
        .ok_or("Missing update repository")?;
    let releases = http
        .get(format!(
            "https://api.github.com/repos/{repository}/releases?per_page=100"
        ))
        .send()
        .await
        .map_err(err)?
        .error_for_status()
        .map_err(err)?
        .bytes()
        .await
        .map_err(err)?;
    let releases: serde_json::Value = serde_json::from_slice(&releases).map_err(err)?;
    let Some(release) = releases
        .as_array()
        .ok_or("Invalid release list")?
        .iter()
        .find(|release| {
            release["draft"] == false
                && release["prerelease"] == false
                && release["tag_name"]
                    .as_str()
                    .is_some_and(|tag| tag.starts_with("nsh-v"))
        })
    else {
        return Ok(None);
    };
    let assets = release["assets"]
        .as_array()
        .ok_or("Invalid release assets")?;
    let asset_url = |name: &str| {
        assets
            .iter()
            .find(|asset| asset["name"] == name)
            .and_then(|asset| asset["browser_download_url"].as_str())
    };
    let Some(url) = asset_url(nsh_updater::MANIFEST) else {
        return Ok(None);
    };
    let response = http.get(url).send().await.map_err(err)?;
    let manifest = response
        .error_for_status()
        .map_err(err)?
        .bytes()
        .await
        .map_err(err)?;
    let signature = if config.public_key.is_some() {
        let url = asset_url(nsh_updater::SIGNATURE).ok_or("Missing update signature")?;
        Some(
            http.get(url)
                .send()
                .await
                .map_err(err)?
                .error_for_status()
                .map_err(err)?
                .bytes()
                .await
                .map_err(err)?,
        )
    } else {
        None
    };
    let Some(available) = config
        .check(&manifest, signature.as_deref(), &nsh_updater::platform())
        .map_err(err)?
    else {
        return Ok(None);
    };
    let can_install = available
        .sha256
        .as_ref()
        .is_some_and(|hash| hash.len() == 64 && hash.bytes().all(|c| c.is_ascii_hexdigit()))
        && available.size.is_some_and(|size| size > 0)
        && supported_archive(&available.url)
        && installation.can_install();
    let result = json!({ "version": available.version, "notes": available.notes, "url": available.url, "canInstall": can_install }).to_string();
    let (cancel, _) = watch::channel(false);
    let mut pending = PENDING.lock().map_err(err)?;
    if let Some(old) = pending.take() {
        old.cancel.send_replace(true);
    }
    *pending = Some(Pending {
        available,
        installation,
        cancel,
        received: Arc::new(AtomicU64::new(0)),
        archive: None,
    });
    Ok(Some(result))
}

pub async fn download() -> Result<()> {
    let (available, received, mut cancelled) = {
        let mut pending = PENDING.lock().map_err(err)?;
        let state = pending.as_mut().ok_or("Check for updates first")?;
        state.cancel.send_replace(false);
        state.archive = None;
        state.received.store(0, Ordering::Relaxed);
        (
            state.available.clone(),
            state.received.clone(),
            state.cancel.subscribe(),
        )
    };
    let hash = available
        .sha256
        .as_deref()
        .ok_or("Update has no checksum")?;
    if hash.len() != 64
        || !hash.bytes().all(|c| c.is_ascii_hexdigit())
        || !supported_archive(&available.url)
    {
        return Err("Invalid update archive or checksum".into());
    }
    let total = available
        .size
        .filter(|n| *n > 0)
        .ok_or("Update has no size")?;
    let suffix = if available
        .url
        .split('?')
        .next()
        .unwrap_or("")
        .ends_with(".zip")
    {
        ".zip"
    } else {
        ".tar.gz"
    };
    let mut archive = tempfile::Builder::new()
        .prefix("q-note-update-")
        .suffix(suffix)
        .tempfile()
        .map_err(err)?;
    let http = client()?;
    let transfer = async {
        let mut response = http
            .get(&available.url)
            .send()
            .await
            .map_err(err)?
            .error_for_status()
            .map_err(err)?;
        let mut verifier = Download::new(hash, Some(total));
        while let Some(chunk) = response.chunk().await.map_err(err)? {
            let count = verifier.update(&chunk).map_err(err)?;
            archive.write_all(&chunk).map_err(err)?;
            received.store(count, Ordering::Relaxed);
        }
        verifier.finish().map_err(err)?;
        archive.as_file().sync_all().map_err(err)?;
        Ok::<(), String>(())
    };
    tokio::select! {
        biased;
        _ = cancelled.changed() => return Err("Download cancelled".into()),
        result = transfer => result?,
    }
    let mut pending = PENDING.lock().map_err(err)?;
    let state = pending.as_mut().ok_or("Update cancelled")?;
    if *state.cancel.borrow() || !Arc::ptr_eq(&received, &state.received) {
        return Err("Download cancelled".into());
    }
    state.archive = Some(archive);
    Ok(())
}

pub fn cancel() {
    if let Ok(mut state) = PENDING.lock() {
        if let Some(state) = state.as_mut() {
            state.cancel.send_replace(true);
            state.archive = None;
        }
    }
}

pub fn progress() -> String {
    let state = PENDING.lock().unwrap_or_else(|e| e.into_inner());
    state.as_ref().map(|s| json!({
        "received": s.received.load(Ordering::Relaxed), "total": s.available.size.unwrap_or(0)
    })).unwrap_or_else(|| json!({"received":0,"total":0})).to_string()
}

pub fn install() -> Result<()> {
    let (installation, archive) = {
        let mut state = PENDING.lock().map_err(err)?;
        let state = state.as_mut().ok_or("No update")?;
        if *state.cancel.borrow() {
            return Err("Update cancelled".into());
        }
        (
            state.installation.clone(),
            state.archive.take().ok_or("No verified download")?,
        )
    };
    installation.install(archive.path()).map_err(err)
}

fn err(error: impl std::fmt::Display) -> String {
    error.to_string()
}

fn supported_archive(url: &str) -> bool {
    let name = url.split('?').next().unwrap_or("");
    name.ends_with(".zip") || name.ends_with(".tar.gz")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Read, net::TcpListener};

    #[tokio::test]
    async fn cancelling_stalled_transfer_stops_request_and_prevents_installation() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/update.zip", listener.local_addr().unwrap());
        let (release, stalled) = std::sync::mpsc::channel();
        let server = std::thread::spawn(move || {
            let (mut connection, _) = listener.accept().unwrap();
            let mut request = [0u8; 4096];
            let _ = connection.read(&mut request);
            connection
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 128\r\n\r\npartial")
                .unwrap();
            let _ = stalled.recv_timeout(Duration::from_secs(3));
        });
        let dir = tempfile::tempdir().unwrap();
        let installation =
            Installation::directory(dir.path(), &dir.path().join("Q Note.exe")).unwrap();
        let mut available = Available::default();
        available.url = url;
        available.sha256 = Some("a".repeat(64));
        available.size = Some(128);
        let (cancel, _) = watch::channel(false);
        *PENDING.lock().unwrap() = Some(Pending {
            available,
            installation,
            cancel,
            received: Arc::new(AtomicU64::new(0)),
            archive: None,
        });
        let download = tokio::spawn(download());
        for _ in 0..100 {
            if PENDING
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .received
                .load(Ordering::Relaxed)
                > 0
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(
            PENDING
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .received
                .load(Ordering::Relaxed)
                > 0
        );
        super::cancel();
        let result = tokio::time::timeout(Duration::from_secs(1), download)
            .await
            .unwrap()
            .unwrap();
        assert!(result.unwrap_err().contains("cancelled"));
        assert!(install().is_err());
        assert!(PENDING.lock().unwrap().as_ref().unwrap().archive.is_none());
        release.send(()).unwrap();
        server.join().unwrap();
        *PENDING.lock().unwrap() = None;
    }
}
