use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, LogicalSize, Manager, PhysicalPosition, State, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};
use tauri_plugin_sql::{Migration, MigrationKind};

const DOCK_WINDOW_SIZE: f64 = 30.0;
const EDITOR_WINDOW_WIDTH: f64 = 520.0;
const EDITOR_WINDOW_HEIGHT: f64 = 640.0;
const EDITOR_WINDOW_MIN_WIDTH: f64 = 420.0;
const EDITOR_WINDOW_MIN_HEIGHT: f64 = 520.0;
const MAIN_WINDOW_MAX_WIDTH: f64 = 1000.0;
const MAIN_WINDOW_MAX_HEIGHT: f64 = 800.0;
const EDITOR_WINDOW_GAP: i32 = 12;
const APP_ICON_BYTES: &[u8] = include_bytes!("../icons/icon.png");
const DATABASE_FILE_NAME: &str = "q-note.db";
const DATA_DIR_NAME: &str = ".q-note";
const APP_IDENTIFIER: &str = "com.win11.q-note";

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct EditorOpenPayload {
    note_id: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DockClipRect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[derive(Default)]
struct TrayMenuState {
    topmost: Mutex<Option<MenuItem<tauri::Wry>>>,
    toggle_language: Mutex<Option<MenuItem<tauri::Wry>>>,
    toggle_dock: Mutex<Option<MenuItem<tauri::Wry>>>,
    quit: Mutex<Option<MenuItem<tauri::Wry>>>,
}

fn migrations() -> Vec<Migration> {
    vec![
        Migration {
            version: 1,
            description: "create_q_note_tables",
            sql: r#"
            CREATE TABLE IF NOT EXISTS notes (
                id TEXT PRIMARY KEY NOT NULL,
                content TEXT NOT NULL,
                color TEXT NOT NULL,
                pinned INTEGER NOT NULL DEFAULT 0,
                text_height INTEGER,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS attachments (
                id TEXT PRIMARY KEY NOT NULL,
                note_id TEXT NOT NULL,
                source TEXT NOT NULL,
                value TEXT NOT NULL,
                name TEXT,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (note_id) REFERENCES notes(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY NOT NULL,
                value TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_notes_sort
                ON notes (pinned DESC, updated_at DESC);
            CREATE INDEX IF NOT EXISTS idx_attachments_note
                ON attachments (note_id, created_at ASC);
        "#,
            kind: MigrationKind::Up,
        },
        Migration {
            version: 2,
            description: "add_attachment_kind",
            sql: r#"
            ALTER TABLE attachments
                ADD COLUMN kind TEXT NOT NULL DEFAULT 'image';
        "#,
            kind: MigrationKind::Up,
        },
        Migration {
            version: 3,
            description: "add_note_sort_order",
            sql: r#"
            ALTER TABLE notes
                ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0;

            UPDATE notes
                SET sort_order = -updated_at;

            DROP INDEX IF EXISTS idx_notes_sort;
            CREATE INDEX IF NOT EXISTS idx_notes_sort
                ON notes (pinned DESC, sort_order ASC, updated_at DESC);
        "#,
            kind: MigrationKind::Up,
        },
    ]
}

fn home_dir() -> Result<PathBuf, String> {
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

fn database_path() -> Result<PathBuf, String> {
    let dir = home_dir()?.join(DATA_DIR_NAME);
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    Ok(dir.join(DATABASE_FILE_NAME))
}

fn database_url() -> Result<String, String> {
    Ok(format!("sqlite:{}", database_path()?.to_string_lossy()))
}

fn config_dir_candidates() -> Vec<PathBuf> {
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

fn legacy_database_candidates() -> Vec<PathBuf> {
    config_dir_candidates()
        .into_iter()
        .map(|dir| dir.join(DATABASE_FILE_NAME))
        .collect()
}

fn copy_first_existing_database(next_path: &Path, candidates: &[PathBuf]) -> Result<bool, String> {
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

fn migrate_legacy_database_before_open() -> Result<bool, String> {
    copy_first_existing_database(&database_path()?, &legacy_database_candidates())
}

fn migrate_legacy_database(app: &tauri::AppHandle, next_path: &PathBuf) -> Result<bool, String> {
    let mut candidates = legacy_database_candidates();
    if let Ok(app_config) = app.path().app_config_dir() {
        candidates.insert(0, app_config.join(DATABASE_FILE_NAME));
    }
    copy_first_existing_database(next_path, &candidates)
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn hide_dock_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("dock") {
        let _ = window.hide();
    }
}

fn clamp_i32(value: i32, min: i32, max: i32) -> i32 {
    value.max(min).min(max)
}

fn position_editor_window(app: &tauri::AppHandle, editor: &WebviewWindow) {
    let Some(main) = app.get_webview_window("main") else {
        return;
    };

    let (Ok(main_position), Ok(main_size), Ok(editor_size)) = (
        main.outer_position(),
        main.outer_size(),
        editor.outer_size(),
    ) else {
        return;
    };

    let gap = editor
        .scale_factor()
        .map(|scale_factor| (EDITOR_WINDOW_GAP as f64 * scale_factor).round() as i32)
        .unwrap_or(EDITOR_WINDOW_GAP);
    let target_x = main_position.x - editor_size.width as i32 - gap;
    let target_y = main_position.y + (main_size.height as i32 - editor_size.height as i32) / 2;

    if let Ok(Some(monitor)) = main.current_monitor() {
        let area = monitor.work_area();
        let area_position = area.position;
        let area_size = area.size;
        let min_x = area_position.x;
        let max_x = area_position.x + area_size.width as i32 - editor_size.width as i32;
        let min_y = area_position.y;
        let max_y = area_position.y + area_size.height as i32 - editor_size.height as i32;

        let _ = editor.set_position(PhysicalPosition::new(
            clamp_i32(target_x, min_x, max_x),
            clamp_i32(target_y, min_y, max_y),
        ));
        return;
    }

    let _ = editor.set_position(PhysicalPosition::new(target_x, target_y));
}

fn editor_window_size() -> LogicalSize<f64> {
    LogicalSize::new(EDITOR_WINDOW_WIDTH, EDITOR_WINDOW_HEIGHT)
}

fn editor_window_min_size() -> LogicalSize<f64> {
    LogicalSize::new(EDITOR_WINDOW_MIN_WIDTH, EDITOR_WINDOW_MIN_HEIGHT)
}

fn apply_main_window_size_constraints(window: &WebviewWindow) -> Result<(), String> {
    window
        .set_max_size(Some(LogicalSize::new(
            MAIN_WINDOW_MAX_WIDTH,
            MAIN_WINDOW_MAX_HEIGHT,
        )))
        .map_err(|error| error.to_string())?;
    let _ = window.set_maximizable(false);

    let scale_factor = window.scale_factor().map_err(|error| error.to_string())?;
    let logical_size = window
        .inner_size()
        .map_err(|error| error.to_string())?
        .to_logical::<f64>(scale_factor);

    if logical_size.width > MAIN_WINDOW_MAX_WIDTH || logical_size.height > MAIN_WINDOW_MAX_HEIGHT {
        window
            .set_size(LogicalSize::new(
                logical_size.width.min(MAIN_WINDOW_MAX_WIDTH),
                logical_size.height.min(MAIN_WINDOW_MAX_HEIGHT),
            ))
            .map_err(|error| error.to_string())?;
    }

    Ok(())
}

#[tauri::command]
fn set_dock_window_clip(app: tauri::AppHandle, clip: Option<DockClipRect>) -> Result<(), String> {
    let window = app
        .get_webview_window("dock")
        .ok_or_else(|| "Dock window not found".to_string())?;

    #[cfg(windows)]
    {
        use windows::Win32::Graphics::Gdi::{CreateRectRgn, DeleteObject, SetWindowRgn};

        let hwnd = window.hwnd().map_err(|error| error.to_string())?;
        if let Some(clip) = clip {
            if clip.left < 0 || clip.top < 0 || clip.right <= clip.left || clip.bottom <= clip.top {
                return Err("Invalid dock window clip rectangle".to_string());
            }

            let region = unsafe { CreateRectRgn(clip.left, clip.top, clip.right, clip.bottom) };
            if region.is_invalid() {
                return Err(std::io::Error::last_os_error().to_string());
            }

            if unsafe { SetWindowRgn(hwnd, Some(region), true) } == 0 {
                unsafe {
                    let _ = DeleteObject(region.into());
                }
                return Err(std::io::Error::last_os_error().to_string());
            }
        } else if unsafe { SetWindowRgn(hwnd, None, true) } == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
    }

    #[cfg(not(windows))]
    let _ = (window, clip);

    Ok(())
}

#[tauri::command]
fn apply_main_window_max_size(app: tauri::AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Main window not found".to_string())?;
    apply_main_window_size_constraints(&window)
}

fn apply_editor_window_size_constraints(window: &WebviewWindow) -> Result<(), String> {
    window
        .set_min_size(Some(editor_window_min_size()))
        .map_err(|error| error.to_string())?;

    let scale_factor = window.scale_factor().map_err(|error| error.to_string())?;
    let logical_size = window
        .inner_size()
        .map_err(|error| error.to_string())?
        .to_logical::<f64>(scale_factor);

    if logical_size.width < EDITOR_WINDOW_MIN_WIDTH
        || logical_size.height < EDITOR_WINDOW_MIN_HEIGHT
    {
        window
            .set_size(editor_window_size())
            .map_err(|error| error.to_string())?;
    }

    Ok(())
}

#[derive(Clone, Copy, serde::Serialize)]
#[serde(rename_all = "lowercase")]
#[allow(dead_code)]
enum RuntimeOs {
    Linux,
    Windows,
    Macos,
    Other,
}

#[derive(Clone, Copy, serde::Serialize)]
#[serde(rename_all = "lowercase")]
#[allow(dead_code)]
enum AppBundleType {
    AppImage,
    Deb,
    Rpm,
    Nsis,
    Msi,
    App,
    Unknown,
}

#[derive(Clone, Copy, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeBundleInfo {
    os: RuntimeOs,
    bundle_type: AppBundleType,
}

fn current_os() -> RuntimeOs {
    #[cfg(target_os = "linux")]
    {
        return RuntimeOs::Linux;
    }

    #[cfg(target_os = "windows")]
    {
        return RuntimeOs::Windows;
    }

    #[cfg(target_os = "macos")]
    {
        return RuntimeOs::Macos;
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
        RuntimeOs::Other
    }
}

fn current_bundle_type() -> AppBundleType {
    #[cfg(target_os = "macos")]
    {
        return AppBundleType::App;
    }

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    {
        use tauri::utils::config::BundleType;
        use tauri::utils::platform::bundle_type;

        return match bundle_type() {
            Some(BundleType::AppImage) => AppBundleType::AppImage,
            Some(BundleType::Deb) => AppBundleType::Deb,
            Some(BundleType::Rpm) => AppBundleType::Rpm,
            Some(BundleType::Nsis) => AppBundleType::Nsis,
            Some(BundleType::Msi) => AppBundleType::Msi,
            Some(BundleType::App) => AppBundleType::App,
            Some(BundleType::Dmg) => AppBundleType::Unknown,
            None => AppBundleType::Unknown,
        };
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
        AppBundleType::Unknown
    }
}

#[tauri::command]
fn get_bundle_type() -> RuntimeBundleInfo {
    RuntimeBundleInfo {
        os: current_os(),
        bundle_type: current_bundle_type(),
    }
}

#[tauri::command]
fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

#[tauri::command]
fn get_database_url(app: tauri::AppHandle) -> Result<String, String> {
    let path = database_path()?;
    let _ = migrate_legacy_database(&app, &path)?;
    Ok(format!("sqlite:{}", path.to_string_lossy()))
}

#[tauri::command]
fn set_tray_menu_labels(
    topmost: String,
    toggle_language: String,
    toggle_dock: String,
    quit: String,
    state: State<'_, TrayMenuState>,
) {
    let topmost_item = state.topmost.lock().ok().and_then(|item| item.clone());
    let language_item = state
        .toggle_language
        .lock()
        .ok()
        .and_then(|item| item.clone());
    let dock_item = state.toggle_dock.lock().ok().and_then(|item| item.clone());
    let quit_item = state.quit.lock().ok().and_then(|item| item.clone());

    if let Some(item) = topmost_item {
        let _ = item.set_text(topmost);
    }
    if let Some(item) = language_item {
        let _ = item.set_text(toggle_language);
    }
    if let Some(item) = dock_item {
        let _ = item.set_text(toggle_dock);
    }
    if let Some(item) = quit_item {
        let _ = item.set_text(quit);
    }
}

#[tauri::command]
async fn open_editor_window(
    app: tauri::AppHandle,
    note_id: Option<String>,
    always_on_top: bool,
    title: String,
) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("editor") {
        window
            .set_icon(Image::from_bytes(APP_ICON_BYTES).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
        window
            .set_title(&title)
            .map_err(|error| error.to_string())?;
        window
            .set_decorations(false)
            .map_err(|error| error.to_string())?;
        window
            .set_shadow(false)
            .map_err(|error| error.to_string())?;
        apply_editor_window_size_constraints(&window)?;
        window
            .set_always_on_top(always_on_top)
            .map_err(|error| error.to_string())?;
        position_editor_window(&app, &window);
        window.unminimize().map_err(|error| error.to_string())?;
        window.show().map_err(|error| error.to_string())?;
        window.set_focus().map_err(|error| error.to_string())?;
        app.emit_to(
            "editor",
            "q-note-editor-open",
            EditorOpenPayload { note_id },
        )
        .map_err(|error| error.to_string())?;
        return Ok(());
    }

    let window = WebviewWindowBuilder::new(&app, "editor", WebviewUrl::App("editor.html".into()))
        .title(title)
        .icon(Image::from_bytes(APP_ICON_BYTES).map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())?
        .inner_size(EDITOR_WINDOW_WIDTH, EDITOR_WINDOW_HEIGHT)
        .min_inner_size(EDITOR_WINDOW_MIN_WIDTH, EDITOR_WINDOW_MIN_HEIGHT)
        .resizable(true)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(always_on_top)
        .visible(false)
        .focused(false)
        .build()
        .map_err(|error| error.to_string())?;

    position_editor_window(&app, &window);
    window.show().map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())?;
    let _ = window.emit("q-note-editor-open", EditorOpenPayload { note_id });

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = migrate_legacy_database_before_open();
    let db_url = database_url().expect("failed to resolve Q Note database path");

    tauri::Builder::default()
        .manage(TrayMenuState::default())
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .app_name("Q Note")
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(
            tauri_plugin_sql::Builder::default()
                .add_migrations(&db_url, migrations())
                .build(),
        )
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                match window.label() {
                    "main" | "dock" => {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                    "editor" => {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                    _ => {}
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            apply_main_window_max_size,
            set_dock_window_clip,
            get_bundle_type,
            quit_app,
            get_database_url,
            set_tray_menu_labels,
            open_editor_window
        ])
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                window.set_icon(Image::from_bytes(APP_ICON_BYTES)?)?;
                apply_main_window_size_constraints(&window).ok();
            }

            if app.get_webview_window("dock").is_none() {
                WebviewWindowBuilder::new(app, "dock", WebviewUrl::App("index.html".into()))
                    .title("Q Note")
                    .icon(Image::from_bytes(APP_ICON_BYTES)?)?
                    .inner_size(DOCK_WINDOW_SIZE, DOCK_WINDOW_SIZE)
                    .min_inner_size(DOCK_WINDOW_SIZE, DOCK_WINDOW_SIZE)
                    .max_inner_size(DOCK_WINDOW_SIZE, DOCK_WINDOW_SIZE)
                    .resizable(false)
                    .decorations(false)
                    .transparent(true)
                    .shadow(false)
                    .always_on_top(true)
                    .skip_taskbar(true)
                    .visible(false)
                    .focused(false)
                    .build()?;
            }

            let topmost = MenuItem::with_id(app, "toggle-topmost", "置顶", true, None::<&str>)?;
            let toggle_language =
                MenuItem::with_id(app, "toggle-language", "切换语言", true, None::<&str>)?;
            let toggle_dock =
                MenuItem::with_id(app, "toggle-dock", "切换悬浮球", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&topmost, &toggle_language, &toggle_dock, &quit])?;
            let icon = Image::from_bytes(APP_ICON_BYTES)?;
            let tray_state = app.state::<TrayMenuState>();
            if let Ok(mut state) = tray_state.topmost.lock() {
                *state = Some(topmost.clone());
            }
            if let Ok(mut state) = tray_state.toggle_language.lock() {
                *state = Some(toggle_language.clone());
            }
            if let Ok(mut state) = tray_state.toggle_dock.lock() {
                *state = Some(toggle_dock.clone());
            }
            if let Ok(mut state) = tray_state.quit.lock() {
                *state = Some(quit.clone());
            }

            TrayIconBuilder::with_id("main")
                .icon(icon)
                .tooltip("Q Note")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main_window(tray.app_handle());
                        hide_dock_window(tray.app_handle());
                        let _ = tray.app_handle().emit_to("main", "q-note-show-main", ());
                    }
                })
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "toggle-topmost" => {
                        let _ = app.emit_to("main", "q-note-toggle-always-on-top", ());
                    }
                    "toggle-language" => {
                        let _ = app.emit_to("main", "q-note-toggle-language", ());
                    }
                    "toggle-dock" => {
                        let _ = app.emit_to("main", "q-note-toggle-dock", ());
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
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
}
