//! Q Note — Slint desktop note board.

#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
slint::include_modules!();

mod app_state;
mod autostart;
mod i18n;
mod models;
mod note_ordering;
mod slint_app;
mod slint_media;
mod slint_tray;
mod storage;
mod updater;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    slint_app::run()
}
