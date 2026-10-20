use anyhow::Result;
use auto_launch::AutoLaunchBuilder;

pub fn apply(enabled: bool) -> Result<()> {
    let auto = build_auto_launch()?;

    if enabled {
        if !auto.is_enabled()? {
            auto.enable()?;
        }
    } else if auto.is_enabled()? {
        auto.disable()?;
    }
    Ok(())
}

/// Read the operating system's current autostart registration.
///
/// The persisted setting is only a cache; startup should prefer this value so an
/// external settings change is reflected in the UI.
pub fn is_enabled() -> Result<bool> {
    Ok(build_auto_launch()?.is_enabled()?)
}

fn build_auto_launch() -> Result<auto_launch::AutoLaunch> {
    let exe = std::env::current_exe()?;
    let app_path = exe.to_string_lossy().to_string();
    Ok(AutoLaunchBuilder::new()
        .set_app_name("Q Note")
        .set_app_path(&app_path)
        .build()?)
}
