use q_note_native::{models::*, storage::*};
use rusqlite::Connection;
use serde_json::json;

fn note(id: &str) -> Note {
    Note {
        id: id.into(),
        content: format!("正文 {id}\nsecond line"),
        color: NOTE_COLORS[7].into(),
        pinned: true,
        sort_order: -14,
        text_height: Some(88),
        attachments: vec![
            NoteAttachment {
                id: format!("{id}-image"),
                kind: AttachmentKind::Image,
                source: AttachmentSource::Data,
                value: "data:image/png;base64,dGVzdA==".into(),
                name: Some("测试.png".into()),
                created_at: 11,
            },
            NoteAttachment {
                id: format!("{id}-file"),
                kind: AttachmentKind::File,
                source: AttachmentSource::Path,
                value: "/tmp/report.pdf".into(),
                name: None,
                created_at: 12,
            },
        ],
        created_at: 5,
        updated_at: 13,
    }
}

#[test]
fn database_and_json_roundtrip_preserve_all_fields() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("q-note.db");
    let data = AppData {
        notes: vec![note("one"), note("two")],
        settings: AppSettings {
            language: Language::Zh,
            always_on_top: true,
            auto_start: true,
            dock_on_edge: true,
            docked: true,
            dock_edge: Some(DockEdge::Right),
            keep_full_main: true,
            window: Some(WindowState {
                width: 420.0,
                height: 610.0,
                x: -740.0,
                y: 52.0,
            }),
        },
    };
    let db = Database::open_path(&path).unwrap();
    db.replace_app_data(&data).unwrap();
    db.flush().unwrap();
    drop(db);
    let restored = Database::open_path(&path).unwrap().load_app_data().unwrap();
    assert_eq!(
        serde_json::to_value(&restored).unwrap(),
        serde_json::to_value(&data).unwrap()
    );
    let imported =
        normalize_import_payload(&serde_json::to_value(create_export_payload(&restored)).unwrap())
            .unwrap();
    assert_eq!(
        serde_json::to_value(imported).unwrap(),
        serde_json::to_value(data).unwrap()
    );
}

#[test]
fn failed_note_save_restores_content_and_attachments() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open_path(&dir.path().join("db")).unwrap();
    let a = note("a");
    let b = note("b");
    db.save_note(&a).unwrap();
    db.save_note(&b).unwrap();
    let before = db.load_app_data().unwrap();
    let mut next = a.clone();
    next.content = "changed".into();
    next.attachments.push(b.attachments[0].clone());
    assert!(db.save_note(&next).is_err());
    assert_eq!(
        json!(db.load_app_data().unwrap().notes),
        json!(before.notes)
    );
}

#[test]
fn failed_import_rolls_back_notes_attachments_and_settings() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open_path(&dir.path().join("db")).unwrap();
    db.save_note(&note("original")).unwrap();
    let before = db.load_app_data().unwrap();
    let mut a = note("new-a");
    let b = note("new-b");
    a.attachments.push(b.attachments[0].clone());
    assert!(
        db.replace_app_data(&AppData {
            notes: vec![a, b],
            settings: AppSettings {
                always_on_top: true,
                ..create_default_settings()
            }
        })
        .is_err()
    );
    assert_eq!(json!(db.load_app_data().unwrap()), json!(before));
}

#[test]
fn order_delete_and_editor_draft_failures_are_atomic() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("db");
    let db = Database::open_path(&path).unwrap();
    db.save_note(&note("a")).unwrap();
    db.save_note(&note("b")).unwrap();
    let before = db.load_app_data().unwrap();
    let control = Connection::open(&path).unwrap();
    control.execute_batch("CREATE TRIGGER reject_order BEFORE UPDATE ON notes WHEN NEW.id='b' BEGIN SELECT RAISE(ABORT,'disk failure'); END;").unwrap();
    let mut order = before.notes.clone();
    for (i, n) in order.iter_mut().enumerate() {
        n.pinned = false;
        n.sort_order = i as i64;
    }
    assert!(db.save_notes_order(&order).is_err());
    assert_eq!(json!(db.load_app_data().unwrap()), json!(before));
    control.execute_batch("DROP TRIGGER reject_order; CREATE TRIGGER reject_delete BEFORE DELETE ON notes BEGIN SELECT RAISE(ABORT,'disk failure'); END;").unwrap();
    assert!(db.delete_all_notes().is_err());
    assert!(db.delete_note("a").is_err());
    assert_eq!(json!(db.load_app_data().unwrap()), json!(before));
    control.execute_batch("DROP TRIGGER reject_delete; CREATE TRIGGER reject_draft_delete BEFORE DELETE ON settings BEGIN SELECT RAISE(ABORT,'disk failure'); END;").unwrap();
    let draft = PendingUpdateDraft {
        note_id: Some("a".into()),
        draft: NoteDraft {
            content: "unfinished".into(),
            ..Default::default()
        },
        saved_at: 17,
    };
    db.save_pending_update_draft(&draft).unwrap();
    let mut changed = note("a");
    changed.content = "saved".into();
    assert!(db.save_editor_note(&changed).is_err());
    assert_eq!(json!(db.load_app_data().unwrap()), json!(before));
    assert_eq!(
        json!(db.load_pending_update_draft().unwrap()),
        json!(Some(draft))
    );
}

#[test]
fn legacy_schema_and_malformed_settings_do_not_lose_notes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("legacy.db");
    let legacy = Connection::open(&path).unwrap();
    legacy.execute_batch("CREATE TABLE notes (id TEXT PRIMARY KEY,content TEXT NOT NULL,color TEXT NOT NULL,pinned INTEGER NOT NULL,text_height INTEGER,created_at INTEGER NOT NULL,updated_at INTEGER NOT NULL); CREATE TABLE attachments (id TEXT PRIMARY KEY,note_id TEXT NOT NULL,source TEXT NOT NULL,value TEXT NOT NULL,name TEXT,created_at INTEGER NOT NULL); CREATE TABLE settings (key TEXT PRIMARY KEY,value TEXT NOT NULL); INSERT INTO notes VALUES ('old','keep','#fff9db',0,66,3,9); INSERT INTO attachments VALUES('asset','old','url','https://example.org/a.png',NULL,4); INSERT INTO settings VALUES('app','{broken');").unwrap();
    drop(legacy);
    let db = Database::open_path(&path).unwrap();
    let data = db.load_app_data().unwrap();
    assert_eq!(data.notes.len(), 1);
    assert_eq!(data.notes[0].sort_order, -9);
    assert_eq!(data.notes[0].text_height, Some(66));
    assert_eq!(data.notes[0].attachments[0].kind, AttachmentKind::Image);
    assert!(!data.settings.always_on_top);
}

#[test]
fn import_skips_invalid_items_and_preserves_duplicate_names() {
    let data=normalize_import_payload(&json!({"notes":[null,17,{"id":"same","content":"same name","attachments":[null,{"value":17},{"id":"dup","value":"data:image/png;base64,abc","source":"data"}]},{"id":"same","content":"same name","attachments":[{"id":"dup","value":"https://example.org/photo.JPG?size=2#top","source":"url"}]}]})).unwrap();
    assert_eq!(data.notes.len(), 2);
    assert_ne!(data.notes[0].id, data.notes[1].id);
    assert_ne!(
        data.notes[0].attachments[0].id,
        data.notes[1].attachments[0].id
    );
    assert_eq!(data.notes[0].attachments.len(), 1);
    assert_eq!(data.notes[1].attachments[0].kind, AttachmentKind::Image);
    assert!(normalize_import_payload(&json!("invalid")).is_err());
}
