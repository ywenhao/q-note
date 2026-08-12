use gpui::{AppContext, Context, Entity, WindowHandle};
use gpui_component::Root;

use crate::i18n::{Translation, t};
use crate::models::Language;
use crate::models::{
    AppData, AppSettings, Note, NoteDraft, PendingUpdateDraft, create_default_settings,
};
use crate::note_ordering::{get_top_sort_order, normalize_manual_order, sort_notes};
use crate::storage::{self, Database};
use crate::updater::UpdateInfo;

#[derive(Clone)]
pub struct ToastMessage {
    pub text: String,
    pub created_at: std::time::Instant,
}

pub struct AppState {
    pub db: Database,
    pub notes: Vec<Note>,
    pub settings: AppSettings,
    pub toast: Option<ToastMessage>,
    pub update: UpdateUiState,
    pub editor_draft_recovery: Option<PendingUpdateDraft>,
    pub main_window: Option<WindowHandle<Root>>,
    pub editor_window: Option<WindowHandle<Root>>,
    pub dock_window: Option<WindowHandle<Root>>,
}

#[derive(Clone, Default)]
pub struct UpdateUiState {
    pub checking: bool,
    pub available: Option<UpdateInfo>,
    pub error: Option<String>,
}

impl AppState {
    pub fn new(_cx: &mut Context<Self>) -> Self {
        let db = Database::open().expect("failed to open Q Note database");
        let data = db.load_app_data().unwrap_or_else(|_| AppData {
            notes: Vec::new(),
            settings: create_default_settings(),
        });
        let recovery = db.load_pending_update_draft().ok().flatten();

        let mut state = Self {
            db,
            notes: sort_notes(data.notes),
            settings: data.settings,
            toast: None,
            update: UpdateUiState::default(),
            editor_draft_recovery: recovery,
            main_window: None,
            editor_window: None,
            dock_window: None,
        };

        let _ = crate::autostart::apply(state.settings.auto_start);
        if state.settings.docked {
            state.settings.keep_full_main = false;
            let _ = state.persist_settings();
        }
        state
    }

    pub fn tr(&self) -> Translation {
        t(self.settings.language)
    }

    pub fn show_toast(&mut self, text: impl Into<String>, cx: &mut Context<Self>) {
        self.toast = Some(ToastMessage {
            text: text.into(),
            created_at: std::time::Instant::now(),
        });
        cx.notify();
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(1700))
                .await;
            this.update(cx, |this, cx| {
                this.toast = None;
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub fn persist_settings(&mut self) -> anyhow::Result<()> {
        self.db.save_settings(&self.settings)
    }

    pub fn toggle_language(&mut self, cx: &mut Context<Self>) {
        self.settings.language = match self.settings.language {
            Language::Zh => Language::En,
            Language::En => Language::Zh,
        };
        let _ = self.persist_settings();
        crate::tray::update_labels(self);
        cx.notify();
    }

    pub fn set_always_on_top(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.settings.always_on_top = enabled;
        let _ = self.persist_settings();
        crate::tray::update_labels(self);
        cx.notify();
    }

    pub fn set_auto_start(&mut self, enabled: bool, cx: &mut Context<Self>) {
        match crate::autostart::apply(enabled) {
            Ok(()) => {
                self.settings.auto_start = enabled;
                let _ = self.persist_settings();
                let msg = self.tr().auto_start_updated.to_string();
                self.show_toast(msg, cx);
            }
            Err(_) => {
                let msg = self.tr().auto_start_failed.to_string();
                self.show_toast(msg, cx);
            }
        }
    }

    pub fn save_note(&mut self, note: Note, cx: &mut Context<Self>) -> anyhow::Result<()> {
        self.db.save_note(&note)?;
        if let Some(idx) = self.notes.iter().position(|n| n.id == note.id) {
            self.notes[idx] = note;
        } else {
            self.notes.push(note);
        }
        self.notes = sort_notes(std::mem::take(&mut self.notes));
        cx.notify();
        Ok(())
    }

    pub fn delete_note(&mut self, id: &str, cx: &mut Context<Self>) -> anyhow::Result<()> {
        self.db.delete_note(id)?;
        self.notes.retain(|n| n.id != id);
        cx.notify();
        Ok(())
    }

    pub fn delete_all_notes(&mut self, cx: &mut Context<Self>) -> anyhow::Result<()> {
        self.db.delete_all_notes()?;
        self.notes.clear();
        cx.notify();
        Ok(())
    }

    pub fn patch_note<F>(&mut self, id: &str, cx: &mut Context<Self>, f: F) -> anyhow::Result<()>
    where
        F: FnOnce(&mut Note),
    {
        let Some(note) = self.notes.iter_mut().find(|n| n.id == id) else {
            return Ok(());
        };
        f(note);
        note.updated_at = now_ms();
        let note = note.clone();
        self.db.save_note(&note)?;
        self.notes = sort_notes(std::mem::take(&mut self.notes));
        cx.notify();
        Ok(())
    }

    pub fn toggle_pin(&mut self, id: &str, cx: &mut Context<Self>) -> anyhow::Result<()> {
        let pinned = self
            .notes
            .iter()
            .find(|n| n.id == id)
            .map(|n| n.pinned)
            .unwrap_or(false);
        let top = get_top_sort_order(&self.notes, !pinned);
        self.patch_note(id, cx, |note| {
            note.pinned = !pinned;
            note.sort_order = top;
        })
    }

    pub fn reorder_notes(
        &mut self,
        notes: Vec<Note>,
        cx: &mut Context<Self>,
    ) -> anyhow::Result<()> {
        let notes = normalize_manual_order(notes);
        self.db.save_notes_order(&notes)?;
        self.notes = notes;
        cx.notify();
        Ok(())
    }

    pub fn note_by_id(&self, id: &str) -> Option<&Note> {
        self.notes.iter().find(|n| n.id == id)
    }

    pub fn upsert_from_draft(
        &mut self,
        note_id: Option<String>,
        draft: NoteDraft,
        cx: &mut Context<Self>,
    ) -> anyhow::Result<Note> {
        let now = now_ms();
        let note = if let Some(existing) = note_id
            .as_ref()
            .and_then(|id| self.notes.iter().find(|n| &n.id == id).cloned())
        {
            let sort_order = if draft.pinned != existing.pinned {
                get_top_sort_order(&self.notes, draft.pinned)
            } else {
                existing.sort_order
            };
            Note {
                id: existing.id,
                content: draft.content,
                color: draft.color,
                pinned: draft.pinned,
                sort_order,
                text_height: existing.text_height,
                attachments: draft.attachments,
                created_at: existing.created_at,
                updated_at: now,
            }
        } else {
            Note {
                id: create_id("note"),
                content: draft.content,
                color: draft.color,
                pinned: draft.pinned,
                sort_order: get_top_sort_order(&self.notes, draft.pinned),
                text_height: None,
                attachments: draft.attachments,
                created_at: now,
                updated_at: now,
            }
        };
        self.save_note(note.clone(), cx)?;
        Ok(note)
    }

    pub fn export_json(&mut self, cx: &mut Context<Self>) -> anyhow::Result<()> {
        let payload = storage::create_export_payload(&AppData {
            notes: self.notes.clone(),
            settings: self.settings.clone(),
        });
        let json = serde_json::to_string_pretty(&payload)?;
        let stamp = chrono::Local::now().format("%Y%m%d%H%M");
        let filename = format!("q-note_{stamp}.json");
        if let Some(path) = rfd::FileDialog::new()
            .set_file_name(&filename)
            .add_filter("JSON", &["json"])
            .save_file()
        {
            std::fs::write(path, json)?;
            let msg = self.tr().exported.to_string();
            self.show_toast(msg, cx);
        }
        Ok(())
    }

    pub fn import_json(&mut self, cx: &mut Context<Self>) -> anyhow::Result<()> {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("JSON", &["json"])
            .pick_file()
        else {
            return Ok(());
        };
        let raw = std::fs::read_to_string(path)?;
        let value: serde_json::Value = serde_json::from_str(&raw)?;
        let data = storage::normalize_import_payload(&value)?;
        self.db.replace_app_data(&data)?;
        self.notes = sort_notes(data.notes);
        self.settings = data.settings;
        let _ = crate::autostart::apply(self.settings.auto_start);
        let msg = self.tr().imported.to_string();
        self.show_toast(msg, cx);
        Ok(())
    }

    pub fn copy_note(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(note) = self.note_by_id(id) else {
            return;
        };
        let text = note.copy_text();
        cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));
        let msg = self.tr().copied.to_string();
        self.show_toast(msg, cx);
    }
}

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn create_id(prefix: &str) -> String {
    format!("{prefix}_{}", uuid::Uuid::new_v4())
}
