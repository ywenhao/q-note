use std::path::Path;

use crate::i18n::{Translation, t};
use crate::models::{
    AppData, AppSettings, Language, Note, NoteDraft, PendingUpdateDraft, create_default_settings,
};
use crate::note_ordering::{get_top_sort_order, reorder_note, sort_notes};
use crate::storage::{self, Database};
use crate::updater::UpdateInfo;

/// Framework-independent application state shared by all Slint windows.
pub struct AppState {
    pub db: Database,
    pub notes: Vec<Note>,
    pub settings: AppSettings,
    pub update: UpdateUiState,
    pub editor_draft_recovery: Option<PendingUpdateDraft>,
    pub editor_draft: Option<PendingUpdateDraft>,
    pub editor_recovery_active: bool,
}

#[derive(Clone, Default)]
pub struct UpdateUiState {
    pub checking: bool,
    pub available: Option<UpdateInfo>,
    pub error: Option<String>,
}

impl AppState {
    pub fn new() -> anyhow::Result<Self> {
        let db = Database::open()?;
        let data = db.load_app_data().unwrap_or_else(|_| AppData {
            notes: Vec::new(),
            settings: create_default_settings(),
        });
        let recovery = db.load_pending_update_draft().ok().flatten();
        let editor_recovery_active = recovery.is_some();

        let mut state = Self {
            db,
            notes: sort_notes(data.notes),
            settings: data.settings,
            update: UpdateUiState::default(),
            editor_draft_recovery: recovery,
            editor_draft: None,
            editor_recovery_active,
        };

        let mut settings_changed = false;
        if state.settings.docked {
            // Dock mode is a transient session state. Always boot into the board.
            state.settings.docked = false;
            state.settings.dock_edge = None;
            state.settings.dock_on_edge = false;
            state.settings.keep_full_main = false;
            settings_changed = true;
        }
        if let Ok(auto_start) = crate::autostart::is_enabled()
            && auto_start != state.settings.auto_start
        {
            state.settings.auto_start = auto_start;
            settings_changed = true;
        }
        if settings_changed {
            state.persist_settings()?;
        }
        Ok(state)
    }

    pub fn tr(&self) -> Translation {
        t(self.settings.language)
    }

    pub fn persist_settings(&self) -> anyhow::Result<()> {
        self.db.save_settings(&self.settings)
    }

    pub fn toggle_language(&mut self) -> anyhow::Result<Language> {
        self.settings.language = match self.settings.language {
            Language::Zh => Language::En,
            Language::En => Language::Zh,
        };
        self.persist_settings()?;
        Ok(self.settings.language)
    }

    pub fn set_always_on_top(&mut self, enabled: bool) -> anyhow::Result<()> {
        self.settings.always_on_top = enabled;
        self.persist_settings()
    }

    pub fn set_auto_start(&mut self, enabled: bool) -> anyhow::Result<()> {
        crate::autostart::apply(enabled)?;
        self.settings.auto_start = enabled;
        self.persist_settings()
    }

    pub fn save_note(&mut self, note: Note) -> anyhow::Result<()> {
        self.db.save_note(&note)?;
        if let Some(index) = self.notes.iter().position(|item| item.id == note.id) {
            self.notes[index] = note;
        } else {
            self.notes.push(note);
        }
        self.notes = sort_notes(std::mem::take(&mut self.notes));
        Ok(())
    }

    pub fn delete_note(&mut self, id: &str) -> anyhow::Result<()> {
        self.db.delete_note(id)?;
        self.notes.retain(|note| note.id != id);
        Ok(())
    }

    pub fn delete_all_notes(&mut self) -> anyhow::Result<()> {
        self.db.delete_all_notes()?;
        self.notes.clear();
        Ok(())
    }

    pub fn patch_note(&mut self, id: &str, update: impl FnOnce(&mut Note)) -> anyhow::Result<()> {
        let Some(note) = self.notes.iter_mut().find(|note| note.id == id) else {
            return Ok(());
        };
        update(note);
        self.db.save_note(note)?;
        self.notes = sort_notes(std::mem::take(&mut self.notes));
        Ok(())
    }

    pub fn toggle_pin(&mut self, id: &str) -> anyhow::Result<()> {
        let pinned = self
            .notes
            .iter()
            .find(|note| note.id == id)
            .is_some_and(|note| note.pinned);
        let top = get_top_sort_order(&self.notes, !pinned);
        self.patch_note(id, |note| {
            note.pinned = !pinned;
            note.sort_order = top;
        })
    }

    pub fn reorder_note(
        &mut self,
        dragged_id: &str,
        target_id: &str,
        after: bool,
    ) -> anyhow::Result<()> {
        let Some(notes) = reorder_note(&self.notes, dragged_id, target_id, after) else {
            return Ok(());
        };
        self.db.save_notes_order(&notes)?;
        self.notes = notes;
        Ok(())
    }

    pub fn set_note_height(&mut self, id: &str, height: i64) -> anyhow::Result<()> {
        let Some(note) = self.notes.iter_mut().find(|note| note.id == id) else {
            return Ok(());
        };
        if note.text_height == Some(height) {
            return Ok(());
        }
        note.text_height = Some(height);
        self.db.save_note(note)
    }

    pub fn note_by_id(&self, id: &str) -> Option<&Note> {
        self.notes.iter().find(|note| note.id == id)
    }

    pub fn upsert_from_draft(
        &mut self,
        note_id: Option<String>,
        draft: NoteDraft,
    ) -> anyhow::Result<Note> {
        let now = now_ms();
        let note = if let Some(existing) = note_id
            .as_ref()
            .and_then(|id| self.notes.iter().find(|note| &note.id == id).cloned())
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
        self.save_note(note.clone())?;
        Ok(note)
    }

    pub fn export_json_to(&self, path: &Path) -> anyhow::Result<()> {
        let payload = storage::create_export_payload(&AppData {
            notes: self.notes.clone(),
            settings: self.settings.clone(),
        });
        std::fs::write(path, serde_json::to_string_pretty(&payload)?)?;
        Ok(())
    }

    pub fn import_json_from(&mut self, path: &Path) -> anyhow::Result<()> {
        let raw = std::fs::read_to_string(path)?;
        let value: serde_json::Value = serde_json::from_str(&raw)?;
        let data = storage::normalize_import_payload(&value)?;
        self.db.replace_app_data(&data)?;
        self.notes = sort_notes(data.notes);
        self.settings = data.settings;
        let _ = crate::autostart::apply(self.settings.auto_start);
        Ok(())
    }

    pub fn copy_text(&self, id: &str) -> Option<String> {
        self.note_by_id(id).map(Note::copy_text)
    }

    pub fn set_editor_draft(&mut self, draft: Option<PendingUpdateDraft>) {
        self.editor_draft = draft;
    }

    pub fn persist_editor_draft(&self) -> anyhow::Result<()> {
        if let Some(draft) = self.editor_draft.as_ref() {
            self.db.save_pending_update_draft(draft)
        } else {
            self.db.clear_pending_update_draft()
        }
    }

    pub fn clear_editor_session(&mut self) -> anyhow::Result<()> {
        self.db.clear_pending_update_draft()?;
        self.editor_draft = None;
        self.editor_draft_recovery = None;
        self.editor_recovery_active = false;
        Ok(())
    }

    pub fn prepare_for_update(&mut self, editor_open: bool) -> anyhow::Result<()> {
        self.persist_settings()?;
        self.editor_recovery_active = editor_open;
        self.persist_editor_draft()?;
        self.db.flush()
    }

    pub fn prepare_for_shutdown(&self, editor_open: bool) -> anyhow::Result<()> {
        self.persist_settings()?;
        if editor_open {
            self.persist_editor_draft()?;
        }
        self.db.flush()
    }
}

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

pub fn create_id(prefix: &str) -> String {
    format!("{prefix}_{}", uuid::Uuid::new_v4())
}
