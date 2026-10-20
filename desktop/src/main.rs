#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod host;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    q_note_native::updater::wait_for_previous_process()?;
    q_note_native::app_state::initialize();
    #[cfg(target_os = "linux")]
    linux::run()?;
    #[cfg(target_os = "windows")]
    windows::run()?;
    #[cfg(target_os = "macos")]
    macos::run()?;
    Ok(())
}
