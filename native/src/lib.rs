pub mod app_state;
pub mod autostart;
pub mod geometry;
pub mod images;
pub mod models;
pub mod note_ordering;
pub mod storage;
pub mod updater;

use nsh_core::ui::prelude::*;

/// The desktop app's durable data and host services.
#[native_module("q-note/native")]
fn native(registry: &mut Registry) {
    registry.module::<QNote>();
}

pub struct QNote;

/// Q Note's desktop bridge. All durable mutations complete before views refresh.
#[module]
impl QNote {
    /// Read changes for this window, including its pending native input events.
    pub fn snapshot(window: String, revision: f64, _cx: &mut ModuleCx) -> String {
        app_state::snapshot(&window, revision as u64)
    }

    /// Execute a data mutation or request an operation on the desktop thread.
    pub fn invoke(command: String, payload: String, _cx: &mut ModuleCx) -> Result<String, String> {
        let payload = serde_json::from_str(&payload).map_err(|e| e.to_string())?;
        app_state::invoke(&command, payload)
            .and_then(|v| serde_json::to_string(&v).map_err(Into::into))
            .map_err(|e| e.to_string())
    }
}
