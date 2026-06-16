use std::{fs, path::PathBuf, sync::Mutex};

use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, PhysicalPosition, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};
use tauri_plugin_sql::{Migration, MigrationKind};

const DOCK_WINDOW_SIZE: f64 = 30.0;
const EDITOR_WINDOW_WIDTH: f64 = 520.0;
const EDITOR_WINDOW_HEIGHT: f64 = 640.0;
const EDITOR_WINDOW_MIN_WIDTH: f64 = 420.0;
const EDITOR_WINDOW_MIN_HEIGHT: f64 = 520.0;
const EDITOR_WINDOW_GAP: i32 = 12;
const APP_ICON_BYTES: &[u8] = include_bytes!("../icons/icon.png");
const DATABASE_FILE_NAME: &str = "q-note.db";
const DATA_DIR_NAME: &str = ".q-note";

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct EditorOpenPayload {
    note_id: Option<String>,
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

fn migrate_legacy_database(app: &tauri::AppHandle, next_path: &PathBuf) -> Result<(), String> {
    if next_path.exists() {
        return Ok(());
    }

    let legacy_path = app
        .path()
        .app_config_dir()
        .map_err(|error| error.to_string())?
        .join(DATABASE_FILE_NAME);

    if legacy_path.exists() {
        fs::copy(legacy_path, next_path).map_err(|error| error.to_string())?;
    }

    Ok(())
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

    let target_x = main_position.x - editor_size.width as i32 - EDITOR_WINDOW_GAP;
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

#[tauri::command]
fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

#[tauri::command]
fn get_database_url(app: tauri::AppHandle) -> Result<String, String> {
    let path = database_path()?;
    migrate_legacy_database(&app, &path)?;
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
            .set_decorations(true)
            .map_err(|error| error.to_string())?;
        window.set_shadow(true).map_err(|error| error.to_string())?;
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
        .decorations(true)
        .shadow(true)
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
            quit_app,
            get_database_url,
            set_tray_menu_labels,
            open_editor_window
        ])
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                window.set_icon(Image::from_bytes(APP_ICON_BYTES)?)?;
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
