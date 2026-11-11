use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

type Result<T> = std::result::Result<T, String>;

pub struct Database(Connection);

pub fn open() -> Result<Database> {
    let root = if let Some(root) = std::env::var_os("Q_NOTE_DATA_DIR") {
        PathBuf::from(root)
    } else {
        dirs::home_dir()
            .ok_or("Cannot locate home directory")?
            .join(".q-note")
    };
    let path = root.join("q-note.db");
    if !path.exists() && std::env::var_os("Q_NOTE_DATA_DIR").is_none() {
        let mut candidates = vec![];
        if let Some(dir) = dirs::config_dir() {
            candidates.push(dir.join("com.win11.q-note/q-note.db"));
        }
        if let Some(dir) = dirs::data_dir() {
            candidates.push(dir.join("com.win11.q-note/q-note.db"));
        }
        migrate_before_open(&path, &candidates)?;
    }
    Database::at(&path)
}

fn migrate_before_open(path: &Path, candidates: &[PathBuf]) -> Result<()> {
    if path.exists() {
        return Ok(());
    }
    if let Some(source) = candidates.iter().find(|p| p.is_file()) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(err)?;
        }
        // VACUUM INTO includes committed WAL pages and leaves the source intact.
        let source = Connection::open(source).map_err(err)?;
        let temporary = tempfile::Builder::new()
            .prefix("q-note-migration-")
            .tempfile_in(path.parent().ok_or("Missing database parent")?)
            .map_err(err)?;
        source
            .execute(
                "VACUUM INTO ?1",
                [temporary.path().to_string_lossy().as_ref()],
            )
            .map_err(err)?;
        temporary.persist_noclobber(path).map_err(err)?;
    }
    Ok(())
}

impl Database {
    pub fn at(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(err)?;
        }
        let db = Connection::open(path).map_err(err)?;
        db.busy_timeout(Duration::from_secs(5)).map_err(err)?;
        db.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL;
            CREATE TABLE IF NOT EXISTS notes (id TEXT PRIMARY KEY NOT NULL, content TEXT NOT NULL,
            color TEXT NOT NULL, pinned INTEGER NOT NULL DEFAULT 0, sort_order INTEGER NOT NULL DEFAULT 0,
            text_height INTEGER, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL);
            CREATE TABLE IF NOT EXISTS attachments (id TEXT PRIMARY KEY NOT NULL, note_id TEXT NOT NULL,
            kind TEXT NOT NULL DEFAULT 'image', source TEXT NOT NULL, value TEXT NOT NULL, name TEXT,
            created_at INTEGER NOT NULL, FOREIGN KEY(note_id) REFERENCES notes(id) ON DELETE CASCADE);
            CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY NOT NULL, value TEXT NOT NULL);")
            .map_err(err)?;
        for (table, column, migration) in [
            (
                "notes",
                "sort_order",
                "ALTER TABLE notes ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0; UPDATE notes SET sort_order=-updated_at;",
            ),
            (
                "notes",
                "text_height",
                "ALTER TABLE notes ADD COLUMN text_height INTEGER;",
            ),
            (
                "attachments",
                "kind",
                "ALTER TABLE attachments ADD COLUMN kind TEXT NOT NULL DEFAULT 'image';",
            ),
        ] {
            let mut stmt = db
                .prepare(&format!("PRAGMA table_info({table})"))
                .map_err(err)?;
            let columns = stmt
                .query_map([], |r| r.get::<_, String>(1))
                .map_err(err)?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(err)?;
            if !columns.iter().any(|name| name == column) {
                db.execute_batch(migration).map_err(err)?;
            }
        }
        Ok(Self(db))
    }

    pub fn load(&self) -> Result<String> {
        let mut stmt = self.0.prepare("SELECT id, content, color, pinned, sort_order, text_height, created_at, updated_at FROM notes ORDER BY pinned DESC, sort_order, updated_at DESC").map_err(err)?;
        let mut notes = stmt.query_map([], |r| Ok(json!({
            "id":r.get::<_,String>(0)?, "content":r.get::<_,String>(1)?, "color":r.get::<_,String>(2)?,
            "pinned":r.get::<_,i64>(3)? != 0, "sortOrder":r.get::<_,i64>(4)?, "textHeight":r.get::<_,Option<i64>>(5)?,
            "createdAt":r.get::<_,i64>(6)?, "updatedAt":r.get::<_,i64>(7)?, "attachments":[]
        }))).map_err(err)?.collect::<std::result::Result<Vec<Value>, _>>().map_err(err)?;
        let mut stmt = self.0.prepare("SELECT id, note_id, kind, source, value, name, created_at FROM attachments ORDER BY created_at, rowid").map_err(err)?;
        let attachments = stmt.query_map([], |r| Ok((r.get::<_,String>(1)?, json!({
            "id":r.get::<_,String>(0)?, "kind":r.get::<_,String>(2)?, "source":r.get::<_,String>(3)?,
            "value":r.get::<_,String>(4)?, "name":r.get::<_,Option<String>>(5)?, "createdAt":r.get::<_,i64>(6)?
        })))).map_err(err)?;
        for entry in attachments {
            let (id, attachment) = entry.map_err(err)?;
            if let Some(note) = notes.iter_mut().find(|n| n["id"] == id) {
                note["attachments"].as_array_mut().unwrap().push(attachment);
            }
        }
        let settings = self
            .setting("app")?
            .and_then(|s| serde_json::from_str::<Value>(&s).ok());
        Ok(json!({ "notes": notes, "settings": settings }).to_string())
    }

    pub fn setting(&self, key: &str) -> Result<Option<String>> {
        self.0
            .query_row("SELECT value FROM settings WHERE key=?1", [key], |r| {
                r.get(0)
            })
            .optional()
            .map_err(err)
    }

    pub fn set_setting(&self, key: &str, value: Option<&str>) -> Result<()> {
        if let Some(value) = value {
            self.0.execute("INSERT INTO settings(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value", params![key, value]).map_err(err)?;
        } else {
            self.0
                .execute("DELETE FROM settings WHERE key=?1", [key])
                .map_err(err)?;
        }
        Ok(())
    }

    pub fn save(&mut self, value: &str, clear_draft: bool) -> Result<()> {
        let note: Value = serde_json::from_str(value).map_err(err)?;
        let tx = self.0.transaction().map_err(err)?;
        write_note(&tx, &note)?;
        if clear_draft {
            tx.execute(
                "DELETE FROM settings WHERE key='pending-update-editor-draft'",
                [],
            )
            .map_err(err)?;
        }
        tx.commit().map_err(err)
    }

    pub fn delete(&mut self, id: &str) -> Result<()> {
        let tx = self.0.transaction().map_err(err)?;
        tx.execute("DELETE FROM attachments WHERE note_id=?1", [id])
            .map_err(err)?;
        tx.execute("DELETE FROM notes WHERE id=?1", [id])
            .map_err(err)?;
        tx.commit().map_err(err)
    }

    pub fn clear(&mut self) -> Result<()> {
        let tx = self.0.transaction().map_err(err)?;
        tx.execute_batch("DELETE FROM attachments; DELETE FROM notes;")
            .map_err(err)?;
        tx.commit().map_err(err)
    }

    pub fn reorder(&mut self, value: &str) -> Result<()> {
        let notes: Vec<Value> = serde_json::from_str(value).map_err(err)?;
        let tx = self.0.transaction().map_err(err)?;
        for n in notes {
            tx.execute(
                "UPDATE notes SET pinned=?1, sort_order=?2 WHERE id=?3",
                params![
                    n["pinned"].as_bool().unwrap_or(false),
                    integer(&n, "sortOrder")?,
                    string(&n, "id")?
                ],
            )
            .map_err(err)?;
        }
        tx.commit().map_err(err)
    }

    pub fn replace(&mut self, value: &str) -> Result<()> {
        let data: Value = serde_json::from_str(value).map_err(err)?;
        let notes = data["notes"].as_array().ok_or("Invalid notes")?;
        let settings = data
            .get("settings")
            .filter(|s| s.is_object())
            .ok_or("Invalid settings")?
            .to_string();
        let tx = self.0.transaction().map_err(err)?;
        tx.execute_batch("DELETE FROM attachments; DELETE FROM notes; DELETE FROM settings;")
            .map_err(err)?;
        tx.execute(
            "INSERT INTO settings(key,value) VALUES('app',?1)",
            [&settings],
        )
        .map_err(err)?;
        for note in notes {
            write_note(&tx, note)?;
        }
        tx.commit().map_err(err)
    }

    pub fn flush(&self) -> Result<()> {
        self.0
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
            .map_err(err)
    }
}

fn write_note(tx: &Transaction<'_>, n: &Value) -> Result<()> {
    let id = string(n, "id")?;
    tx.execute("INSERT INTO notes(id,content,color,pinned,sort_order,text_height,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)
        ON CONFLICT(id) DO UPDATE SET content=excluded.content,color=excluded.color,pinned=excluded.pinned,sort_order=excluded.sort_order,text_height=excluded.text_height,updated_at=excluded.updated_at",
        params![id,string(n,"content")?,string(n,"color")?,n["pinned"].as_bool().unwrap_or(false),integer(n,"sortOrder")?,n["textHeight"].as_i64(),integer(n,"createdAt")?,integer(n,"updatedAt")?]).map_err(err)?;
    tx.execute("DELETE FROM attachments WHERE note_id=?1", [id])
        .map_err(err)?;
    for a in n["attachments"].as_array().ok_or("Invalid attachments")? {
        tx.execute("INSERT INTO attachments(id,note_id,kind,source,value,name,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7)",
            params![string(a,"id")?,id,string(a,"kind")?,string(a,"source")?,string(a,"value")?,a["name"].as_str(),integer(a,"createdAt")?]).map_err(err)?;
    }
    Ok(())
}

fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value[key].as_str().ok_or_else(|| format!("Invalid {key}"))
}
fn integer(value: &Value, key: &str) -> Result<i64> {
    value[key].as_i64().ok_or_else(|| format!("Invalid {key}"))
}
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(id: &str) -> Value {
        json!({"id":id,"content":"旧便签","color":"#fff9db","pinned":true,"sortOrder":-3,"textHeight":88,
            "createdAt":10,"updatedAt":20,"attachments":[
                {"id":format!("{id}-image"),"kind":"image","source":"data","value":"data:image/png;base64,aA==","name":"截图.png","createdAt":11},
                {"id":format!("{id}-file"),"kind":"file","source":"path","value":"C:\\notes\\readme.txt","name":"readme.txt","createdAt":12}]})
    }

    #[test]
    fn round_trip_preserves_all_fields_and_corrupt_settings_do_not_hide_notes() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::at(&dir.path().join("q-note.db")).unwrap();
        let original = note("a");
        db.save(&original.to_string(), false).unwrap();
        db.set_setting("app", Some("{invalid")).unwrap();
        let loaded: Value = serde_json::from_str(&db.load().unwrap()).unwrap();
        assert_eq!(loaded["notes"][0], original);
        assert_eq!(loaded["settings"], Value::Null);
    }

    #[test]
    fn failed_save_and_import_leave_notes_settings_and_recovery_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::at(&dir.path().join("q-note.db")).unwrap();
        db.save(&note("a").to_string(), false).unwrap();
        db.set_setting("app", Some(r#"{"language":"zh"}"#)).unwrap();
        db.set_setting("pending-update-editor-draft", Some("recover"))
            .unwrap();
        let before = db.load().unwrap();
        let mut invalid = note("a");
        invalid["content"] = json!("must roll back");
        invalid["attachments"][1]["id"] = invalid["attachments"][0]["id"].clone();
        assert!(db.save(&invalid.to_string(), true).is_err());
        assert_eq!(db.load().unwrap(), before);
        assert_eq!(
            db.setting("pending-update-editor-draft")
                .unwrap()
                .as_deref(),
            Some("recover")
        );
        let bad_import = json!({"notes":[note("b"),invalid],"settings":{"language":"en"}});
        assert!(db.replace(&bad_import.to_string()).is_err());
        assert_eq!(db.load().unwrap(), before);
        assert_eq!(
            db.setting("pending-update-editor-draft")
                .unwrap()
                .as_deref(),
            Some("recover")
        );
        db.save(&note("a").to_string(), true).unwrap();
        assert_eq!(db.setting("pending-update-editor-draft").unwrap(), None);
    }

    #[test]
    fn legacy_migration_includes_live_wal_and_never_overwrites_destination() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("old.db");
        let target = dir.path().join("new/q-note.db");
        let mut old = Database::at(&source).unwrap();
        old.0.execute_batch("PRAGMA wal_autocheckpoint=0").unwrap();
        old.save(&note("wal").to_string(), false).unwrap();
        migrate_before_open(&target, std::slice::from_ref(&source)).unwrap();
        assert!(source.exists());
        let copied = Database::at(&target).unwrap();
        assert_eq!(copied.load().unwrap(), old.load().unwrap());
        drop(copied);
        old.clear().unwrap();
        migrate_before_open(&target, &[source]).unwrap();
        let copied: Value =
            serde_json::from_str(&Database::at(&target).unwrap().load().unwrap()).unwrap();
        assert_eq!(copied["notes"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn oldest_schema_gets_missing_fields_without_losing_attachments() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("old.db");
        let old = Connection::open(&path).unwrap();
        old.execute_batch("CREATE TABLE notes(id TEXT PRIMARY KEY,content TEXT,color TEXT,pinned INTEGER,created_at INTEGER,updated_at INTEGER);
            CREATE TABLE attachments(id TEXT PRIMARY KEY,note_id TEXT,source TEXT,value TEXT,name TEXT,created_at INTEGER);
            INSERT INTO notes VALUES('old','legacy','#fff9db',0,1,2);
            INSERT INTO attachments VALUES('image','old','url','https://example.com/a.png',NULL,1);").unwrap();
        drop(old);
        let data: Value =
            serde_json::from_str(&Database::at(&path).unwrap().load().unwrap()).unwrap();
        assert_eq!(data["notes"][0]["content"], "legacy");
        assert_eq!(data["notes"][0]["sortOrder"], -2);
        assert_eq!(data["notes"][0]["attachments"][0]["kind"], "image");
    }
}
