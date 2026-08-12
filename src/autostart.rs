use anyhow::Result;
use auto_launch::AutoLaunchBuilder;

pub fn apply(enabled: bool) -> Result<()> {
    let exe = std::env::current_exe()?;
    let app_path = exe.to_string_lossy().to_string();
    let auto = AutoLaunchBuilder::new()
        .set_app_name("Q Note")
        .set_app_path(&app_path)
        .build()?;

    if enabled {
        if !auto.is_enabled()? {
            auto.enable()?;
        }
    } else if auto.is_enabled()? {
        auto.disable()?;
    }
    Ok(())
}
