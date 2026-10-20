use std::{
    collections::{HashMap, VecDeque},
    sync::{
        Arc, Mutex, MutexGuard, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
};

use anyhow::{Result, anyhow, bail};
use base64::Engine as _;
use serde::Serialize;
use serde_json::{Value, json};

use crate::{autostart, models::*, note_ordering::*, storage::*, updater};

pub fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}
pub fn create_id(prefix: &str) -> String {
    format!("{prefix}-{}", uuid::Uuid::new_v4())
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorSession {
    pub id: String,
    pub note_id: Option<String>,
    pub draft: NoteDraft,
}

#[derive(Clone, Debug)]
pub struct HostCommand {
    pub action: String,
    pub window: String,
    pub payload: Value,
}

struct Store {
    db: Option<Database>,
    data: AppData,
    editor: Option<EditorSession>,
    revision: u64,
    commands: VecDeque<HostCommand>,
    events: HashMap<String, Vec<Value>>,
    previews: HashMap<String, bool>,
    toast: Value,
    update: Value,
    cancellation: Arc<AtomicBool>,
    window_save: Option<std::sync::mpsc::SyncSender<std::result::Result<(), String>>>,
}

static STORE: OnceLock<Mutex<Store>> = OnceLock::new();

fn store() -> MutexGuard<'static, Store> {
    STORE
        .get_or_init(|| {
            let db = Database::open().ok();
            let loaded = db.as_ref().and_then(|db| db.load_app_data().ok());
            let failed = loaded.is_none();
            let mut data = loaded.unwrap_or_else(|| AppData {
                notes: vec![],
                settings: create_default_settings(),
            });
            data.settings.docked = false;
            data.settings.dock_on_edge = false;
            data.settings.dock_edge = None;
            data.settings.keep_full_main = false;
            if let Ok(enabled) = autostart::is_enabled() {
                data.settings.auto_start = enabled;
            }
            if let Some(db) = &db {
                let _ = db.save_settings(&data.settings);
            }
            let editor = db
                .as_ref()
                .and_then(|db| db.load_pending_update_draft().ok().flatten())
                .map(|pending| EditorSession {
                    id: create_id("session"),
                    note_id: pending.note_id,
                    draft: pending.draft,
                });
            Mutex::new(Store {
                db,
                data,
                editor,
                revision: 1,
                commands: VecDeque::new(),
                events: HashMap::new(),
                previews: HashMap::new(),
                toast: if failed {
                    json!({"id":create_id("toast"),"key":"loadFailed","error":true,"at":now_ms()})
                } else {
                    Value::Null
                },
                update: json!({"phase":"idle"}),
                cancellation: Arc::new(AtomicBool::new(false)),
                window_save: None,
            })
        })
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

impl Store {
    fn db(&self) -> Result<&Database> {
        self.db
            .as_ref()
            .ok_or_else(|| anyhow!("database unavailable"))
    }
    fn changed(&mut self) {
        self.revision += 1;
    }
    fn notify(&mut self, key: &str, error: bool) {
        self.toast = json!({"id":create_id("toast"),"key":key,"error":error,"at":now_ms()});
        self.changed();
    }
    fn host(&mut self, action: &str, window: &str, payload: Value) {
        self.commands.push_back(HostCommand {
            action: action.into(),
            window: window.into(),
            payload,
        });
    }
    fn settings(&mut self, next: AppSettings) -> Result<()> {
        self.db()?.save_settings(&next)?;
        self.data.settings = next;
        self.changed();
        self.host("settings", "all", Value::Null);
        Ok(())
    }
    fn save(&mut self, note: Note) -> Result<()> {
        self.db()?.save_note(&note)?;
        self.data.notes.retain(|n| n.id != note.id);
        self.data.notes.push(note);
        self.data.notes = sort_notes(std::mem::take(&mut self.data.notes));
        self.changed();
        Ok(())
    }
}

pub fn initialize() {
    drop(store());
}
pub fn settings() -> AppSettings {
    store().data.settings.clone()
}
pub fn has_editor() -> bool {
    store().editor.is_some()
}
pub fn editing_existing() -> bool {
    store()
        .editor
        .as_ref()
        .is_some_and(|editor| editor.note_id.is_some())
}
pub fn take_commands() -> Vec<HostCommand> {
    store().commands.drain(..).collect()
}
pub fn event(window: &str, event: Value) {
    store().events.entry(window.into()).or_default().push(event);
}
pub fn preview_open(window: &str) -> bool {
    *store().previews.get(window).unwrap_or(&false)
}

pub fn report_save_error(result: Result<()>) {
    if let Err(error) = result {
        eprintln!("{error}");
        store().notify("saveFailed", true);
    }
}

pub fn window_saved(result: Result<()>) {
    if let Some(sender) = store().window_save.take() {
        let _ = sender.send(result.map_err(|error| error.to_string()));
    }
}

fn prepare_window() -> Result<()> {
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    {
        let mut s = store();
        s.window_save = Some(sender);
        s.host("saveWindow", "main", Value::Null);
    }
    receiver
        .recv_timeout(std::time::Duration::from_secs(5))
        .map_err(|_| anyhow!("Window state was not saved"))?
        .map_err(|error| anyhow!(error))
}

fn clipboard() -> Result<std::sync::MutexGuard<'static, Option<arboard::Clipboard>>> {
    static CLIPBOARD: OnceLock<Mutex<Option<arboard::Clipboard>>> = OnceLock::new();
    let mut handle = CLIPBOARD
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    if handle.is_none() {
        *handle = Some(arboard::Clipboard::new()?);
    }
    Ok(handle)
}

pub fn snapshot(window: &str, revision: u64) -> String {
    let mut state = store();
    if state.toast["at"]
        .as_i64()
        .is_some_and(|at| now_ms() - at > 1700)
    {
        state.toast = Value::Null;
    }
    let events = state.events.remove(window).unwrap_or_default();
    if state.revision == revision && events.is_empty() {
        return String::new();
    }
    json!({"revision":state.revision,"data":state.data,"editor":state.editor,"toast":state.toast,"update":state.update,"events":events,"palette":NOTE_COLORS,"version":updater::PACKAGE_VERSION}).to_string()
}

pub fn save_window(frame: WindowState) -> Result<()> {
    let mut state = store();
    let mut settings = state.data.settings.clone();
    settings.window = Some(frame);
    state.db()?.save_settings(&settings)?;
    state.data.settings = settings;
    Ok(())
}

pub fn flush_for_exit() -> Result<()> {
    let state = store();
    if let Some(editor) = &state.editor {
        state.db()?.save_pending_update_draft(&PendingUpdateDraft {
            note_id: editor.note_id.clone(),
            draft: editor.draft.clone(),
            saved_at: now_ms(),
        })?;
    }
    state.db()?.save_settings(&state.data.settings)?;
    state.db()?.flush()
}

pub fn invoke(command: &str, payload: Value) -> Result<Value> {
    let result = execute(command, payload);
    if result.is_err() {
        let key = match command {
            "delete" | "deleteAll" => "deleteFailed",
            "import" => "importFailed",
            "autoStart" => "autoStartFailed",
            _ => "saveFailed",
        };
        store().notify(key, true);
    }
    result
}

fn execute(command: &str, payload: Value) -> Result<Value> {
    let string = |key: &str| payload.get(key).and_then(Value::as_str).unwrap_or_default();
    match command {
        "copy" => {
            let value = store()
                .data
                .notes
                .iter()
                .find(|n| n.id == string("id"))
                .map(Note::copy_text)
                .ok_or_else(|| anyhow!("note missing"))?;
            clipboard()?.as_mut().unwrap().set_text(value)?;
            store().notify("copied", false);
        }
        "delete" => {
            let mut s = store();
            s.db()?.delete_note(string("id"))?;
            s.data.notes.retain(|n| n.id != string("id"));
            s.changed();
        }
        "deleteAll" => {
            let mut s = store();
            s.db()?.delete_all_notes()?;
            s.data.notes.clear();
            s.changed();
        }
        "pin" | "color" | "height" => {
            let mut s = store();
            let mut note = s
                .data
                .notes
                .iter()
                .find(|n| n.id == string("id"))
                .cloned()
                .ok_or_else(|| anyhow!("note missing"))?;
            match command {
                "pin" => {
                    note.pinned = !note.pinned;
                    note.sort_order = get_top_sort_order(&s.data.notes, note.pinned);
                }
                "color" => {
                    if !NOTE_COLORS.contains(&string("color")) {
                        bail!("invalid color");
                    }
                    note.color = string("color").into();
                }
                _ => {
                    note.text_height = payload
                        .get("height")
                        .and_then(Value::as_i64)
                        .map(|h| h.max(22))
                }
            }
            note.updated_at = now_ms();
            s.save(note)?;
        }
        "reorder" => {
            let mut s = store();
            if let Some(notes) = reorder_note(
                &s.data.notes,
                string("id"),
                string("target"),
                payload["after"].as_bool().unwrap_or(false),
            ) {
                s.db()?.save_notes_order(&notes)?;
                s.data.notes = notes;
                s.changed();
            }
        }
        "language" | "topmost" => {
            let mut s = store();
            let mut next = s.data.settings.clone();
            if command == "language" {
                next.language = if next.language == Language::Zh {
                    Language::En
                } else {
                    Language::Zh
                };
            } else {
                next.always_on_top = !next.always_on_top;
            }
            s.settings(next)?;
        }
        "autoStart" => {
            let enabled = payload["enabled"].as_bool().unwrap_or(false);
            let previous = autostart::is_enabled()?;
            autostart::apply(enabled)?;
            let mut s = store();
            let mut next = s.data.settings.clone();
            next.auto_start = enabled;
            if let Err(error) = s.settings(next) {
                let _ = autostart::apply(previous);
                return Err(error);
            }
            s.notify("autoStartUpdated", false);
        }
        "readAutoStart" => {
            if let Ok(value) = autostart::is_enabled() {
                let mut s = store();
                s.data.settings.auto_start = value;
                s.changed();
            }
        }
        "edit" => {
            let mut s = store();
            s.db()?.clear_pending_update_draft()?;
            let note = s.data.notes.iter().find(|n| n.id == string("id")).cloned();
            let draft = note
                .as_ref()
                .map(|n| NoteDraft {
                    attachments: n.attachments.clone(),
                    color: n.color.clone(),
                    content: n.content.clone(),
                    pinned: n.pinned,
                })
                .unwrap_or_default();
            s.editor = Some(EditorSession {
                id: create_id("session"),
                note_id: note.map(|n| n.id),
                draft,
            });
            s.changed();
            s.host("edit", "editor", Value::Null);
        }
        "draft" => {
            let mut s = store();
            if let Some(editor) = s.editor.as_mut().filter(|e| e.id == string("session")) {
                editor.draft = serde_json::from_value(payload["draft"].clone())?;
            }
        }
        "save" => {
            let mut s = store();
            let session = s
                .editor
                .clone()
                .filter(|e| e.id == string("session"))
                .ok_or_else(|| anyhow!("stale editor session"))?;
            let mut draft: NoteDraft = serde_json::from_value(payload["draft"].clone())?;
            draft.content = draft.content.trim().into();
            if !draft.content.is_empty() || !draft.attachments.is_empty() {
                let existing = session
                    .note_id
                    .as_ref()
                    .and_then(|id| s.data.notes.iter().find(|n| &n.id == id));
                let now = now_ms();
                let note = Note {
                    id: session.note_id.clone().unwrap_or_else(|| create_id("note")),
                    content: draft.content,
                    color: draft.color,
                    pinned: draft.pinned,
                    sort_order: existing.filter(|n| n.pinned == draft.pinned).map_or_else(
                        || get_top_sort_order(&s.data.notes, draft.pinned),
                        |n| n.sort_order,
                    ),
                    text_height: existing.and_then(|n| n.text_height),
                    attachments: draft.attachments,
                    created_at: existing.map_or(now, |n| n.created_at),
                    updated_at: now,
                };
                s.db()?.save_editor_note(&note)?;
                s.data.notes.retain(|n| n.id != note.id);
                s.data.notes.push(note);
                s.data.notes = sort_notes(std::mem::take(&mut s.data.notes));
                s.notify("saved", false);
            } else {
                s.db()?.clear_pending_update_draft()?;
            }
            s.editor = None;
            s.changed();
            s.host("close", "editor", Value::Null);
        }
        "close" => {
            let window = string("window");
            let mut s = store();
            if window == "editor" {
                s.db()?.clear_pending_update_draft()?;
                s.editor = None;
                s.previews.insert("editor".into(), false);
                s.changed();
            }
            s.host("close", window, Value::Null);
        }
        "quit" => {
            store().host("quit", "all", Value::Null);
        }
        "dock" | "show" | "minimize" | "beginDockDrag" | "dragDock" | "endDockDrag"
        | "hoverDock" | "dragWindow" => {
            store().host(command, string("window"), payload.clone());
        }
        "preview" => {
            store().previews.insert(
                string("window").into(),
                payload["open"].as_bool().unwrap_or(false),
            );
        }
        "imageSrc" => {
            crate::images::request(
                string("source").into(),
                string("value").into(),
                string("window").into(),
                string("request").into(),
            );
        }
        "paste" => {
            let mut clipboard = arboard::Clipboard::new()?;
            if payload["image"].as_bool().unwrap_or(true)
                && let Ok(image) = clipboard.get_image()
            {
                let mut bytes = std::io::Cursor::new(Vec::new());
                image::RgbaImage::from_raw(
                    image.width as u32,
                    image.height as u32,
                    image.bytes.into_owned(),
                )
                .ok_or_else(|| anyhow!("invalid clipboard image"))?
                .write_to(&mut bytes, image::ImageFormat::Png)?;
                return Ok(
                    json!({"image":attachment(data_url("clipboard.png",bytes.get_ref()),AttachmentSource::Data,Some("clipboard.png".into()))}),
                );
            }
            return Ok(json!({"text":clipboard.get_text().unwrap_or_default()}));
        }
        "pickImages" => {
            let session = string("session").to_owned();
            std::thread::spawn(move || {
                if let Some(paths) = rfd::FileDialog::new()
                    .add_filter(
                        "Images",
                        &["png", "jpg", "jpeg", "gif", "webp", "bmp", "svg"],
                    )
                    .pick_files()
                {
                    let mut items = vec![];
                    for path in paths {
                        match std::fs::read(&path) {
                            Ok(bytes) => items.push(attachment(
                                data_url(&path.to_string_lossy(), &bytes),
                                AttachmentSource::Data,
                                path.file_name().map(|n| n.to_string_lossy().into_owned()),
                            )),
                            Err(_) => {
                                store().notify("loadFailed", true);
                            }
                        }
                    }
                    append_attachments(&session, items);
                }
            });
        }
        "export" => {
            std::thread::spawn(|| {
                let name = format!("q-note_{}.json", chrono::Local::now().format("%Y%m%d%H%M"));
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("JSON", &["json"])
                    .set_file_name(name)
                    .save_file()
                {
                    let result = (|| -> Result<bool> {
                        let data = create_export_payload(&store().data);
                        std::fs::write(path, serde_json::to_string_pretty(&data)?)?;
                        Ok(true)
                    })();
                    store().notify(
                        if result.is_ok() {
                            "exported"
                        } else {
                            "saveFailed"
                        },
                        result.is_err(),
                    );
                }
            });
        }
        "import" => {
            std::thread::spawn(|| {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("JSON", &["json"])
                    .pick_file()
                {
                    let result = (|| -> Result<bool> {
                        let data = normalize_import_payload(&serde_json::from_str(
                            &std::fs::read_to_string(path)?,
                        )?)?;
                        let mut s = store();
                        s.db()?.replace_app_data(&data)?;
                        s.data = data;
                        s.data.notes = sort_notes(std::mem::take(&mut s.data.notes));
                        s.editor = None;
                        s.previews.clear();
                        s.host("close", "editor", Value::Null);
                        s.changed();
                        s.host("settings", "all", Value::Null);
                        let auto = s.data.settings.auto_start;
                        drop(s);
                        if autostart::apply(auto).is_err() {
                            let mut s = store();
                            if let Ok(actual) = autostart::is_enabled() {
                                let mut settings = s.data.settings.clone();
                                settings.auto_start = actual;
                                s.settings(settings)?;
                            }
                            return Ok(false);
                        }
                        Ok(true)
                    })();
                    store().notify(
                        match result {
                            Ok(true) => "imported",
                            Ok(false) => "autoStartFailed",
                            Err(_) => "importFailed",
                        },
                        !matches!(result, Ok(true)),
                    );
                }
            });
        }
        "release" => {
            let version = payload["version"].as_str().map(str::to_owned);
            std::thread::spawn(move || updater::open_release_page(version.as_deref()));
        }
        "checkUpdate" => {
            check_update(false);
        }
        "dismissUpdate" => {
            let mut s = store();
            s.update["show"] = json!(false);
            s.changed();
        }
        "cancelUpdate" => {
            let mut s = store();
            if s.update["phase"] != "installing" {
                s.cancellation.store(true, Ordering::Relaxed);
                s.update["show"] = json!(false);
                s.changed();
            }
        }
        "installUpdate" => {
            install_update()?;
        }
        _ => bail!("unknown command: {command}"),
    }
    Ok(Value::Null)
}

pub fn attachment(value: String, source: AttachmentSource, name: Option<String>) -> NoteAttachment {
    NoteAttachment {
        id: create_id("asset"),
        kind: if is_likely_image_path(&value) {
            AttachmentKind::Image
        } else {
            AttachmentKind::File
        },
        source,
        value,
        name,
        created_at: now_ms(),
    }
}

fn data_url(path: &str, bytes: &[u8]) -> String {
    let extension = path.rsplit('.').next().unwrap_or_default().to_lowercase();
    let mime = match extension.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "svg" => "image/svg+xml",
        _ => "image/png",
    };
    format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )
}

pub fn append_paths(paths: Vec<String>) {
    let session = store().editor.as_ref().map(|e| e.id.clone());
    if let Some(session) = session {
        append_attachments(
            &session,
            paths
                .into_iter()
                .map(|p| {
                    let name = std::path::Path::new(&p)
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned());
                    attachment(p, AttachmentSource::Path, name)
                })
                .collect(),
        );
    }
}

fn append_attachments(session: &str, items: Vec<NoteAttachment>) {
    let mut s = store();
    if let Some(editor) = s.editor.as_mut().filter(|e| e.id == session) {
        editor.draft.attachments.extend(items);
        s.changed();
    }
}

pub fn start_update_schedule() {
    if cfg!(debug_assertions) && std::env::var_os("Q_NOTE_TEST_OFFLINE").is_some() {
        return;
    }
    std::thread::spawn(|| {
        check_update(true);
        loop {
            std::thread::sleep(updater::duration_until_next_daily_check());
            check_update(true);
        }
    });
}

fn check_update(silent: bool) {
    {
        let mut s = store();
        if s.update["phase"] != "idle" {
            return;
        }
        s.update["phase"] = json!("checking");
        s.changed();
    }
    std::thread::spawn(move || {
        let result = updater::check_for_update();
        let mut s = store();
        s.update["phase"] = json!("idle");
        match result {
            Ok(Some(info)) => s.update = json!({"phase":"idle","info":info,"show":true}),
            Ok(None) => {
                if !silent {
                    s.notify("updateNone", false);
                }
            }
            Err(_) => {
                if !silent {
                    s.notify("updateCheckFailed", true);
                }
            }
        }
        s.changed();
    });
}

fn install_update() -> Result<()> {
    let (info, cancel) = {
        let mut s = store();
        if s.update["phase"] != "idle" {
            return Ok(());
        }
        let info: updater::UpdateInfo = serde_json::from_value(s.update["info"].clone())?;
        if info.current_binary().is_none() {
            drop(s);
            updater::open_release_page(Some(&info.version));
            let mut s = store();
            s.update["show"] = json!(false);
            s.notify("updateOpenRelease", false);
            return Ok(());
        }
        s.cancellation = Arc::new(AtomicBool::new(false));
        s.update["phase"] = json!("downloading");
        s.update["show"] = json!(true);
        s.changed();
        (info, Arc::clone(&s.cancellation))
    };
    std::thread::spawn(move || {
        let result = updater::download_update(&info, &cancel, |p| {
            let mut s = store();
            s.update["downloaded"] = json!(p.downloaded);
            s.update["total"] = json!(p.total);
            s.changed();
        });
        if cancel.load(Ordering::Relaxed) {
            let mut s = store();
            s.update["phase"] = json!("idle");
            s.update["show"] = json!(false);
            s.changed();
            return;
        }
        match result {
            Ok(download) => {
                {
                    let mut s = store();
                    s.update["phase"] = json!("preparing");
                    s.changed();
                }
                if prepare_window().and_then(|_| flush_for_exit()).is_err() {
                    let mut s = store();
                    s.update["phase"] = json!("idle");
                    s.update["show"] = json!(false);
                    s.notify("updatePrepareFailed", true);
                    return;
                }
                {
                    let mut s = store();
                    if cancel.load(Ordering::Relaxed) {
                        s.update["phase"] = json!("idle");
                        s.update["show"] = json!(false);
                        s.changed();
                        return;
                    }
                    s.update["phase"] = json!("installing");
                    s.changed();
                }
                if updater::install_and_relaunch(download).is_ok() {
                    store().host("quit", "all", Value::Null);
                } else {
                    let mut s = store();
                    s.update["phase"] = json!("idle");
                    s.update["show"] = json!(false);
                    s.notify("updateInstallFailed", true);
                }
            }
            Err(_) => {
                let mut s = store();
                s.update["phase"] = json!("idle");
                s.update["show"] = json!(false);
                s.notify("updateDownloadFailed", true);
            }
        }
    });
    Ok(())
}
