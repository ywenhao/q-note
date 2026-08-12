use anyhow::{Context, Result, anyhow};
use rusqlite::{Connection, OptionalExtension, params};

use crate::models::{
    AppData, AppSettings, AttachmentKind, AttachmentSource, ExportPayload, Language, Note,
    NoteAttachment, PendingUpdateDraft, WindowState, create_default_settings, is_likely_image_path,
};

const SETTINGS_KEY: &str = "app";
const PENDING_UPDATE_DRAFT_KEY: &str = "pending-update-editor-draft";
const DATA_DIR_NAME: &str = ".q-note";
const DATABASE_FILE_NAME: &str = "q-note.db";

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open() -> Result<Self> {
        let path = database_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        migrate_legacy_database(&path)?;
        let conn = Connection::open(&path).with_context(|| format!("open {}", path.display()))?;
        conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
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
        )?;

        // Migration 2: attachment kind
        let has_kind: bool = self
            .conn
            .prepare("PRAGMA table_info(attachments)")?
            .query_map([], |row| row.get::<_, String>(1))?
            .filter_map(|r| r.ok())
            .any(|name| name == "kind");
        if !has_kind {
            self.conn.execute_batch(
                "ALTER TABLE attachments ADD COLUMN kind TEXT NOT NULL DEFAULT 'image';",
            )?;
        }

        // Migration 3: sort_order
        let has_sort: bool = self
            .conn
            .prepare("PRAGMA table_info(notes)")?
            .query_map([], |row| row.get::<_, String>(1))?
            .filter_map(|r| r.ok())
            .any(|name| name == "sort_order");
        if !has_sort {
            self.conn.execute_batch(
                r#"
                ALTER TABLE notes ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0;
                UPDATE notes SET sort_order = -updated_at;
                DROP INDEX IF EXISTS idx_notes_sort;
                CREATE INDEX IF NOT EXISTS idx_notes_sort
                    ON notes (pinned DESC, sort_order ASC, updated_at DESC);
                "#,
            )?;
        }

        Ok(())
    }

    pub fn load_app_data(&self) -> Result<AppData> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, content, color, pinned, sort_order, text_height, created_at, updated_at
            FROM notes
            ORDER BY pinned DESC, sort_order ASC, updated_at DESC
            "#,
        )?;
        let notes_iter = stmt.query_map([], |row| {
            Ok(Note {
                id: row.get(0)?,
                content: row.get(1)?,
                color: row.get(2)?,
                pinned: row.get::<_, i64>(3)? == 1,
                sort_order: row.get(4)?,
                text_height: row.get(5)?,
                attachments: Vec::new(),
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;

        let mut notes = Vec::new();
        for note in notes_iter {
            notes.push(note?);
        }

        let mut att_stmt = self.conn.prepare(
            r#"
            SELECT id, note_id, kind, source, value, name, created_at
            FROM attachments
            ORDER BY created_at ASC
            "#,
        )?;
        let attachments = att_stmt.query_map([], |row| {
            let kind_raw: String = row.get(2)?;
            let source_raw: String = row.get(3)?;
            let value: String = row.get(4)?;
            let source = match source_raw.as_str() {
                "url" => AttachmentSource::Url,
                "path" => AttachmentSource::Path,
                _ => AttachmentSource::Data,
            };
            let kind = match kind_raw.as_str() {
                "file" => AttachmentKind::File,
                "image" => AttachmentKind::Image,
                _ => {
                    if is_likely_image_path(&value) {
                        AttachmentKind::Image
                    } else {
                        AttachmentKind::File
                    }
                }
            };
            Ok((
                row.get::<_, String>(1)?,
                NoteAttachment {
                    id: row.get(0)?,
                    kind,
                    source,
                    value,
                    name: row.get(5)?,
                    created_at: row.get(6)?,
                },
            ))
        })?;

        for item in attachments {
            let (note_id, attachment) = item?;
            if let Some(note) = notes.iter_mut().find(|n| n.id == note_id) {
                note.attachments.push(attachment);
            }
        }

        let settings = self.load_settings()?;
        Ok(AppData { notes, settings })
    }

    pub fn load_settings(&self) -> Result<AppSettings> {
        let value: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![SETTINGS_KEY],
                |row| row.get(0),
            )
            .optional()?;
        Ok(match value {
            Some(raw) => normalize_settings(&serde_json::from_str(&raw)?),
            None => create_default_settings(),
        })
    }

    pub fn save_settings(&self, settings: &AppSettings) -> Result<()> {
        let value = serde_json::to_string(settings)?;
        self.conn.execute(
            r#"
            INSERT INTO settings (key, value) VALUES (?1, ?2)
            ON CONFLICT(key) DO UPDATE SET value = excluded.value
            "#,
            params![SETTINGS_KEY, value],
        )?;
        Ok(())
    }

    pub fn save_note(&self, note: &Note) -> Result<()> {
        self.conn.execute(
            r#"
            INSERT INTO notes (id, content, color, pinned, sort_order, text_height, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(id) DO UPDATE SET
                content = excluded.content,
                color = excluded.color,
                pinned = excluded.pinned,
                sort_order = excluded.sort_order,
                text_height = excluded.text_height,
                updated_at = excluded.updated_at
            "#,
            params![
                note.id,
                note.content,
                note.color,
                if note.pinned { 1 } else { 0 },
                note.sort_order,
                note.text_height,
                note.created_at,
                note.updated_at,
            ],
        )?;
        self.conn
            .execute("DELETE FROM attachments WHERE note_id = ?1", params![note.id])?;
        for attachment in &note.attachments {
            let kind = match attachment.kind {
                AttachmentKind::Image => "image",
                AttachmentKind::File => "file",
            };
            let source = match attachment.source {
                AttachmentSource::Data => "data",
                AttachmentSource::Url => "url",
                AttachmentSource::Path => "path",
            };
            self.conn.execute(
                r#"
                INSERT INTO attachments (id, note_id, kind, source, value, name, created_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                "#,
                params![
                    attachment.id,
                    note.id,
                    kind,
                    source,
                    attachment.value,
                    attachment.name,
                    attachment.created_at,
                ],
            )?;
        }
        Ok(())
    }

    pub fn save_notes_order(&self, notes: &[Note]) -> Result<()> {
        for note in notes {
            self.conn.execute(
                "UPDATE notes SET pinned = ?1, sort_order = ?2 WHERE id = ?3",
                params![if note.pinned { 1 } else { 0 }, note.sort_order, note.id],
            )?;
        }
        Ok(())
    }

    pub fn delete_note(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM attachments WHERE note_id = ?1", params![id])?;
        self.conn
            .execute("DELETE FROM notes WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn delete_all_notes(&self) -> Result<()> {
        self.conn.execute("DELETE FROM attachments", [])?;
        self.conn.execute("DELETE FROM notes", [])?;
        Ok(())
    }

    pub fn replace_app_data(&self, data: &AppData) -> Result<()> {
        self.conn.execute("DELETE FROM attachments", [])?;
        self.conn.execute("DELETE FROM notes", [])?;
        self.conn.execute("DELETE FROM settings", [])?;
        self.save_settings(&data.settings)?;
        for note in &data.notes {
            self.save_note(note)?;
        }
        Ok(())
    }

    pub fn flush(&self) -> Result<()> {
        self.conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        Ok(())
    }

    pub fn load_pending_update_draft(&self) -> Result<Option<PendingUpdateDraft>> {
        let value: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![PENDING_UPDATE_DRAFT_KEY],
                |row| row.get(0),
            )
            .optional()?;
        Ok(match value {
            Some(raw) => Some(serde_json::from_str(&raw)?),
            None => None,
        })
    }

    pub fn save_pending_update_draft(&self, draft: &PendingUpdateDraft) -> Result<()> {
        let value = serde_json::to_string(draft)?;
        self.conn.execute(
            r#"
            INSERT INTO settings (key, value) VALUES (?1, ?2)
            ON CONFLICT(key) DO UPDATE SET value = excluded.value
            "#,
            params![PENDING_UPDATE_DRAFT_KEY, value],
        )?;
        Ok(())
    }

    pub fn clear_pending_update_draft(&self) -> Result<()> {
        self.conn.execute(
            "DELETE FROM settings WHERE key = ?1",
            params![PENDING_UPDATE_DRAFT_KEY],
        )?;
        Ok(())
    }
}

fn database_path() -> Result<std::path::PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("Unable to resolve home directory"))?;
    Ok(home.join(DATA_DIR_NAME).join(DATABASE_FILE_NAME))
}

fn migrate_legacy_database(next_path: &std::path::Path) -> Result<()> {
    if next_path.exists() {
        return Ok(());
    }
    // Legacy Windows / Tauri app config location
    if let Some(config) = dirs::config_dir() {
        let legacy = config.join("com.win11.q-note").join(DATABASE_FILE_NAME);
        if legacy.exists() {
            std::fs::copy(legacy, next_path)?;
        }
    }
    Ok(())
}

pub fn create_export_payload(data: &AppData) -> ExportPayload {
    ExportPayload {
        version: 1,
        exported_at: chrono::Utc::now().to_rfc3339(),
        notes: data.notes.clone(),
        settings: data.settings.clone(),
    }
}

pub fn normalize_import_payload(value: &serde_json::Value) -> Result<AppData> {
    let obj = value
        .as_object()
        .ok_or_else(|| anyhow!("Invalid Q Note data"))?;
    let notes = obj
        .get("notes")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| normalize_note(item).ok())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let settings = obj
        .get("settings")
        .map(normalize_settings_value)
        .unwrap_or_else(create_default_settings);
    Ok(AppData { notes, settings })
}

fn normalize_settings_value(value: &serde_json::Value) -> AppSettings {
    normalize_settings(value)
}

fn normalize_settings(value: &serde_json::Value) -> AppSettings {
    let defaults = create_default_settings();
    let Some(obj) = value.as_object() else {
        return defaults;
    };
    AppSettings {
        language: match obj.get("language").and_then(|v| v.as_str()) {
            Some("zh") => Language::Zh,
            Some("en") => Language::En,
            _ => defaults.language,
        },
        always_on_top: obj
            .get("alwaysOnTop")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        auto_start: obj
            .get("autoStart")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        dock_on_edge: obj
            .get("dockOnEdge")
            .and_then(|v| v.as_bool())
            .unwrap_or(defaults.dock_on_edge),
        docked: obj
            .get("docked")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        dock_edge: match obj.get("dockEdge").and_then(|v| v.as_str()) {
            Some("left") => Some(crate::models::DockEdge::Left),
            Some("right") => Some(crate::models::DockEdge::Right),
            Some("top") => Some(crate::models::DockEdge::Top),
            Some("bottom") => Some(crate::models::DockEdge::Bottom),
            _ => None,
        },
        keep_full_main: obj
            .get("keepFullMain")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        window: obj.get("window").and_then(|v| {
            let w = v.as_object()?;
            Some(WindowState {
                width: w.get("width")?.as_f64()? as f32,
                height: w.get("height")?.as_f64()? as f32,
                x: w.get("x")?.as_f64()? as f32,
                y: w.get("y")?.as_f64()? as f32,
            })
        }),
    }
}

fn normalize_note(value: &serde_json::Value) -> Result<Note> {
    let obj = value
        .as_object()
        .ok_or_else(|| anyhow!("invalid note"))?;
    let updated_at = obj
        .get("updatedAt")
        .and_then(|v| v.as_i64())
        .unwrap_or_else(crate::app_state::now_ms);
    let attachments = obj
        .get("attachments")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| normalize_attachment(item).ok())
                .collect()
        })
        .unwrap_or_default();
    Ok(Note {
        id: obj
            .get("id")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| crate::app_state::create_id("note")),
        content: obj
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        color: obj
            .get("color")
            .and_then(|v| v.as_str())
            .unwrap_or(crate::models::DEFAULT_NOTE_COLOR)
            .to_string(),
        pinned: obj
            .get("pinned")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        sort_order: obj
            .get("sortOrder")
            .and_then(|v| v.as_i64())
            .unwrap_or(-updated_at),
        text_height: obj.get("textHeight").and_then(|v| v.as_i64()),
        attachments,
        created_at: obj
            .get("createdAt")
            .and_then(|v| v.as_i64())
            .unwrap_or(updated_at),
        updated_at,
    })
}

fn normalize_attachment(value: &serde_json::Value) -> Result<NoteAttachment> {
    let obj = value
        .as_object()
        .ok_or_else(|| anyhow!("invalid attachment"))?;
    let value_str = obj
        .get("value")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("missing value"))?
        .to_string();
    let source = match obj.get("source").and_then(|v| v.as_str()) {
        Some("url") => AttachmentSource::Url,
        Some("path") => AttachmentSource::Path,
        _ => AttachmentSource::Data,
    };
    let kind = match obj.get("kind").and_then(|v| v.as_str()) {
        Some("file") => AttachmentKind::File,
        Some("image") => AttachmentKind::Image,
        _ => {
            if is_likely_image_path(&value_str) {
                AttachmentKind::Image
            } else {
                AttachmentKind::File
            }
        }
    };
    Ok(NoteAttachment {
        id: obj
            .get("id")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| crate::app_state::create_id("asset")),
        kind,
        source,
        value: value_str,
        name: obj
            .get("name")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        created_at: obj
            .get("createdAt")
            .and_then(|v| v.as_i64())
            .unwrap_or_else(crate::app_state::now_ms),
    })
}
