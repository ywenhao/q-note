use std::{
    fs,
    path::{Path, PathBuf},
};

pub const DATABASE_FILE_NAME: &str = "q-note.db";
pub const DATA_DIR_NAME: &str = ".q-note";
pub const APP_IDENTIFIER: &str = "com.win11.q-note";

pub fn home_dir() -> Result<PathBuf, String> {
    #[cfg(windows)]
    {
        if let Some(path) = std::env::var_os("USERPROFILE").map(PathBuf::from) {
            return Ok(path);
        }

        if let (Some(drive), Some(path)) =
            (std::env::var_os("HOMEDRIVE"), std::env::var_os("HOMEPATH"))
        {
            let mut home = PathBuf::from(drive);
            home.push(path);
            return Ok(home);
        }
    }

    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "Unable to resolve the user home directory".to_string())
}

pub fn database_path() -> Result<PathBuf, String> {
    let dir = home_dir()?.join(DATA_DIR_NAME);
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    Ok(dir.join(DATABASE_FILE_NAME))
}

pub fn config_dir_candidates() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    #[cfg(windows)]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            let appdata = PathBuf::from(appdata);
            dirs.push(appdata.join(APP_IDENTIFIER));
            dirs.push(appdata.join("Q Note"));
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = home_dir() {
            let support = home.join("Library").join("Application Support");
            dirs.push(support.join(APP_IDENTIFIER));
            dirs.push(support.join("Q Note"));
        }
    }

    #[cfg(target_os = "linux")]
    {
        let config_home = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| home_dir().ok().map(|home| home.join(".config")));
        if let Some(config_home) = config_home {
            dirs.push(config_home.join(APP_IDENTIFIER));
            dirs.push(config_home.join("q-note"));
            dirs.push(config_home.join("Q Note"));
        }
    }

    dirs
}

pub fn legacy_database_candidates() -> Vec<PathBuf> {
    config_dir_candidates()
        .into_iter()
        .map(|dir| dir.join(DATABASE_FILE_NAME))
        .collect()
}

pub fn copy_first_existing_database(
    next_path: &Path,
    candidates: &[PathBuf],
) -> Result<bool, String> {
    if next_path.exists() {
        return Ok(false);
    }

    for legacy_path in candidates {
        if legacy_path.exists() && legacy_path != next_path {
            if let Some(parent) = next_path.parent() {
                fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            fs::copy(legacy_path, next_path).map_err(|error| error.to_string())?;
            return Ok(true);
        }
    }

    Ok(false)
}

pub fn migrate_legacy_database_before_open() -> Result<bool, String> {
    copy_first_existing_database(&database_path()?, &legacy_database_candidates())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_case_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "q-note-legacy-{}-{}-{}",
            label,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    #[test]
    fn skips_legacy_copy_when_destination_exists() {
        let dir = temp_case_dir("skip");
        let dest = dir.join("next.db");
        let src = dir.join("legacy.db");
        fs::write(&dest, b"new").unwrap();
        fs::write(&src, b"old").unwrap();

        let copied = copy_first_existing_database(&dest, &[src]).unwrap();

        assert!(!copied);
        assert_eq!(fs::read(&dest).unwrap(), b"new");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn does_not_overwrite_an_already_created_empty_destination() {
        let dir = temp_case_dir("empty-dest");
        let dest = dir.join("next.db");
        let src = dir.join("legacy.db");
        fs::write(&dest, b"").unwrap();
        fs::write(&src, b"legacy-bytes").unwrap();

        let copied = copy_first_existing_database(&dest, &[src]).unwrap();

        assert!(!copied);
        assert_eq!(fs::read(&dest).unwrap(), b"");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn copies_first_existing_legacy_database() {
        let dir = temp_case_dir("copy");
        let dest = dir.join("nested").join("next.db");
        let missing = dir.join("missing.db");
        let src = dir.join("legacy.db");
        fs::write(&src, b"legacy-bytes").unwrap();

        let copied = copy_first_existing_database(&dest, &[missing, src]).unwrap();

        assert!(copied);
        assert_eq!(fs::read(&dest).unwrap(), b"legacy-bytes");
        let _ = fs::remove_dir_all(dir);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_legacy_candidates_include_xdg_config_identifier() {
        let candidates = legacy_database_candidates();
        assert!(
            candidates
                .iter()
                .any(|path| path.ends_with("com.win11.q-note/q-note.db")),
            "expected a ~/.config/com.win11.q-note/q-note.db candidate, got {candidates:?}"
        );
    }
}
