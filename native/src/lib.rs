//! Native persistence and OS integration for the NSH user interface.

use nsh_core::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
mod database;
#[cfg(not(target_arch = "wasm32"))]
mod system;
#[cfg(not(target_arch = "wasm32"))]
mod updates;

macro_rules! native_only {
    ($work:expr) => {{
        #[cfg(not(target_arch = "wasm32"))]
        {
            ($work)()
        }
        #[cfg(target_arch = "wasm32")]
        {
            Err("This operation requires the desktop app".to_owned())
        }
    }};
}

#[native_module("q-note/native")]
fn native(registry: &mut Registry) {
    registry
        .module::<Notes>()
        .module::<Files>()
        .module::<Startup>()
        .module::<Updates>()
        .module::<DockClip>();
}

/// The existing Q Note SQLite database, including transactional writes.
#[module]
pub struct Notes;

#[module]
impl Notes {
    /// Reads notes and settings without changing their format or location.
    pub fn load() -> Result<String, String> {
        native_only!(|| database::open()?.load())
    }

    /// Saves a note and all its attachments atomically.
    pub fn save(note: String, clear_draft: bool) -> Result<(), String> {
        native_only!(|| database::open()?.save(&note, clear_draft))
    }

    /// Deletes one note and its attachments atomically.
    pub fn delete(id: String) -> Result<(), String> {
        native_only!(|| database::open()?.delete(&id))
    }

    /// Deletes all notes and their attachments, retaining settings.
    pub fn clear() -> Result<(), String> {
        native_only!(|| database::open()?.clear())
    }

    /// Persists manual order and pinned groups in one transaction.
    pub fn reorder(notes: String) -> Result<(), String> {
        native_only!(|| database::open()?.reorder(&notes))
    }

    /// Persists settings before any corresponding UI change.
    pub fn settings(value: String) -> Result<(), String> {
        native_only!(|| database::open()?.set_setting("app", Some(&value)))
    }

    /// Replaces imported notes and settings atomically.
    pub fn replace(value: String) -> Result<(), String> {
        native_only!(|| database::open()?.replace(&value))
    }

    /// Returns the editor recovery record, if any.
    pub fn read_draft() -> Result<Option<String>, String> {
        native_only!(|| database::open()?.setting("pending-update-editor-draft"))
    }

    /// Writes or clears the editor recovery record.
    pub fn draft(value: Option<String>) -> Result<(), String> {
        native_only!(
            || database::open()?.set_setting("pending-update-editor-draft", value.as_deref())
        )
    }

    /// Checkpoints SQLite before quitting or installing an update.
    pub fn flush() -> Result<(), String> {
        native_only!(|| database::open()?.flush())
    }
}

/// System file dialogs and embedded image data.
#[module]
pub struct Files;

#[module]
impl Files {
    /// Exports data to a user-selected file; false means the dialog was cancelled.
    pub async fn export(value: String, name: String) -> Result<bool, String> {
        use nsh_core::file_dialog::{FileFilter, SaveDialog};
        let path = pick_save_path(
            SaveDialog::new()
                .file_name(name)
                .filter(FileFilter::new("JSON", ["json"])),
        )
        .await
        .map_err(|e| e.to_string())?;
        let Some(path) = path else { return Ok(false) };
        #[cfg(not(target_arch = "wasm32"))]
        {
            std::fs::write(path, value).map_err(|e| e.to_string())?;
            Ok(true)
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (path, value);
            Err("Desktop file dialogs are unavailable".into())
        }
    }

    /// Reads a JSON file selected by the user; null means cancellation.
    pub async fn import() -> Result<Option<String>, String> {
        use nsh_core::file_dialog::{FileFilter, OpenDialog};
        let paths = pick_paths(OpenDialog::files().filter(FileFilter::new("JSON", ["json"])))
            .await
            .map_err(|e| e.to_string())?;
        let Some(path) = paths.and_then(|paths| paths.into_iter().next()) else {
            return Ok(None);
        };
        #[cfg(not(target_arch = "wasm32"))]
        {
            std::fs::read_to_string(path)
                .map(Some)
                .map_err(|e| e.to_string())
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = path;
            Err("Desktop file dialogs are unavailable".into())
        }
    }

    /// Reads an explicitly attached local image as an embedded data URL.
    pub async fn image(path: String) -> Result<String, String> {
        native_only!(|| system::image(&path))
    }
}

/// Launch-at-login integration for the current user's desktop session.
#[module]
pub struct Startup;

#[module]
impl Startup {
    /// Reads the actual operating system launch-at-login setting.
    pub fn enabled() -> Result<bool, String> {
        native_only!(|| system::autostart()?.is_enabled().map_err(|e| e.to_string()))
    }

    /// Enables or disables launch at login for the packaged executable.
    pub fn set_enabled(enabled: bool) -> Result<(), String> {
        native_only!(|| {
            let app = system::autostart()?;
            if enabled { app.enable() } else { app.disable() }.map_err(|e| e.to_string())
        })
    }
}

/// Verified desktop updates with interruptible network transfers.
#[module]
pub struct Updates;

/// Platform clipping of the transparent floating window at adjacent monitors.
#[module]
pub struct DockClip;

#[module]
impl DockClip {
    /// Clips only the off-screen portion of this process's floating window.
    pub fn set(edge: String, hidden: f64) {
        #[cfg(windows)]
        system::clip_dock(&edge, hidden);
        #[cfg(not(windows))]
        let _ = (edge, hidden);
    }
}

#[module]
impl Updates {
    /// Checks the packaged application's update source.
    pub async fn check() -> Result<Option<String>, String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            updates::check().await
        }
        #[cfg(target_arch = "wasm32")]
        {
            Ok(None)
        }
    }

    /// Downloads and verifies an archive, dropping the request when cancelled.
    pub async fn download() -> Result<(), String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            updates::download().await
        }
        #[cfg(target_arch = "wasm32")]
        {
            Err("Desktop updates are unavailable".into())
        }
    }

    /// Cancels a pending request and removes its staged archive.
    pub fn cancel() {
        #[cfg(not(target_arch = "wasm32"))]
        updates::cancel();
    }

    /// Returns byte counts for the current download.
    pub fn progress() -> String {
        #[cfg(not(target_arch = "wasm32"))]
        {
            updates::progress()
        }
        #[cfg(target_arch = "wasm32")]
        {
            r#"{"received":0,"total":0}"#.into()
        }
    }

    /// Replaces a packaged application using the verified NSH installer.
    pub async fn install() -> Result<(), String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            blocking(updates::install)
                .await
                .map_err(|e| e.to_string())?
        }
        #[cfg(target_arch = "wasm32")]
        {
            Err("Desktop updates are unavailable".into())
        }
    }
}
