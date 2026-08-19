//! Q Note — GPUI desktop note board.

#![cfg_attr(target_os = "windows", allow(linker_messages))]

use std::borrow::Cow;

mod app_state;
mod autostart;
mod i18n;
mod models;
mod note_ordering;
mod storage;
mod tray;
mod ui;
mod updater;

use gpui::{App, AppContext, Application, AssetSource, SharedString};
use gpui_component_assets::Assets as ComponentAssets;

use crate::app_state::AppState;

pub(crate) const PIN_ICON_PATH: &str = "icons/pin.svg";
pub(crate) const PIN_OFF_ICON_PATH: &str = "icons/pin-off.svg";
pub(crate) const POWER_ICON_PATH: &str = "icons/power.svg";
pub(crate) const UPLOAD_ICON_PATH: &str = "icons/upload.svg";
pub(crate) const DOWNLOAD_ICON_PATH: &str = "icons/download.svg";
pub(crate) const REFRESH_ICON_PATH: &str = "icons/refresh-cw.svg";

struct AppAssets;

impl AssetSource for AppAssets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        match path {
            PIN_ICON_PATH => Ok(Some(Cow::Borrowed(include_bytes!("../assets/pin.svg")))),
            PIN_OFF_ICON_PATH => Ok(Some(Cow::Borrowed(include_bytes!("../assets/pin-off.svg")))),
            POWER_ICON_PATH => Ok(Some(Cow::Borrowed(include_bytes!("../assets/power.svg")))),
            UPLOAD_ICON_PATH => Ok(Some(Cow::Borrowed(include_bytes!("../assets/upload.svg")))),
            DOWNLOAD_ICON_PATH => Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/download.svg"
            )))),
            REFRESH_ICON_PATH => Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/refresh-cw.svg"
            )))),
            _ => ComponentAssets.load(path),
        }
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<SharedString>> {
        let mut assets = ComponentAssets.list(path)?;
        assets.extend(
            [
                PIN_ICON_PATH,
                PIN_OFF_ICON_PATH,
                POWER_ICON_PATH,
                UPLOAD_ICON_PATH,
                DOWNLOAD_ICON_PATH,
                REFRESH_ICON_PATH,
            ]
            .into_iter()
            .filter(|asset| asset.starts_with(path))
            .map(SharedString::from),
        );
        Ok(assets)
    }
}

fn main() {
    Application::new()
        .with_assets(AppAssets)
        .run(|cx: &mut App| {
            gpui_component::init(cx);
            ui::init(cx);

            let state = cx.new(AppState::new);
            tray::spawn_tray(state.clone(), cx);

            // Match the Tauri boot path: the main board is the initial window;
            // dock mode is entered explicitly by the user during the session.
            ui::main_window::open_main_window_at_startup(state.clone(), cx);

            cx.activate(true);
        });
}
