use std::sync::Mutex;

use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder,
};
use tauri_plugin_sql::{Migration, MigrationKind};

const DOCK_WINDOW_SIZE: f64 = 30.0;

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
    ]
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

#[tauri::command]
fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
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
                .add_migrations("sqlite:q-note.db", migrations())
                .build(),
        )
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![quit_app, set_tray_menu_labels])
        .setup(|app| {
            if app.get_webview_window("dock").is_none() {
                WebviewWindowBuilder::new(app, "dock", WebviewUrl::App("index.html".into()))
                    .title("Q Note")
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
            let icon = Image::from_bytes(include_bytes!("../icons/32x32.png"))?;
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
