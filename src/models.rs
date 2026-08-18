use serde::{Deserialize, Serialize};

pub const APP_BACKGROUND: u32 = 0xffd150;
pub const EDITOR_BACKGROUND: u32 = 0xfff9df;
pub const DOCK_WINDOW_SIZE: f32 = 30.0;
pub const DEFAULT_WINDOW_WIDTH: f32 = 300.0;
pub const DEFAULT_WINDOW_HEIGHT: f32 = 450.0;
pub const MAX_WINDOW_WIDTH: f32 = 1000.0;
pub const MAX_WINDOW_HEIGHT: f32 = 800.0;
pub const EDITOR_WINDOW_WIDTH: f32 = 520.0;
pub const EDITOR_WINDOW_HEIGHT: f32 = 640.0;
pub const EDITOR_WINDOW_MIN_WIDTH: f32 = 420.0;
pub const EDITOR_WINDOW_MIN_HEIGHT: f32 = 520.0;
pub const NOTE_LINE_HEIGHT: f32 = 22.0;

pub const NOTE_COLORS: [&str; 12] = [
    "#fff9db", "#ffe8cc", "#ffd8a8", "#d8f5a2", "#b2f2bb", "#c5f6fa", "#d0ebff", "#dbe4ff",
    "#e5dbff", "#ffdeeb", "#ffe3e3", "#f1f3f5",
];

pub const DEFAULT_NOTE_COLOR: &str = NOTE_COLORS[0];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Zh,
    En,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AttachmentKind {
    Image,
    File,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AttachmentSource {
    Data,
    Url,
    Path,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteAttachment {
    pub id: String,
    pub kind: AttachmentKind,
    pub source: AttachmentSource,
    pub value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: String,
    pub content: String,
    pub color: String,
    pub pinned: bool,
    pub sort_order: i64,
    pub text_height: Option<i64>,
    pub attachments: Vec<NoteAttachment>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Note {
    pub fn copy_text(&self) -> String {
        let content = self.content.trim();
        if !content.is_empty() {
            return content.to_string();
        }
        self.attachments
            .iter()
            .map(|a| a.value.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn display_lines(&self) -> i64 {
        if let Some(h) = self.text_height {
            return (h as f32 / NOTE_LINE_HEIGHT).round().max(1.0) as i64;
        }
        let chars = self.content.chars().count();
        if chars <= 34 && !self.content.contains('\n') {
            1
        } else {
            2
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteDraft {
    pub attachments: Vec<NoteAttachment>,
    pub color: String,
    pub content: String,
    pub pinned: bool,
}

impl Default for NoteDraft {
    fn default() -> Self {
        Self {
            attachments: Vec::new(),
            color: DEFAULT_NOTE_COLOR.to_string(),
            content: String::new(),
            pinned: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DockEdge {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowState {
    pub width: f32,
    pub height: f32,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub language: Language,
    pub always_on_top: bool,
    pub auto_start: bool,
    pub dock_on_edge: bool,
    pub docked: bool,
    pub dock_edge: Option<DockEdge>,
    pub keep_full_main: bool,
    pub window: Option<WindowState>,
}

pub fn create_default_settings() -> AppSettings {
    AppSettings {
        language: detect_default_language(),
        always_on_top: false,
        auto_start: false,
        dock_on_edge: false,
        docked: false,
        dock_edge: None,
        keep_full_main: false,
        window: None,
    }
}

fn detect_default_language() -> Language {
    let lang = std::env::var("LANG")
        .or_else(|_| std::env::var("LC_ALL"))
        .unwrap_or_default()
        .to_lowercase();
    if lang.starts_with("zh") {
        Language::Zh
    } else {
        Language::En
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppData {
    pub notes: Vec<Note>,
    pub settings: AppSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportPayload {
    pub version: u32,
    pub exported_at: String,
    pub notes: Vec<Note>,
    pub settings: AppSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingUpdateDraft {
    pub note_id: Option<String>,
    pub draft: NoteDraft,
    pub saved_at: i64,
}

pub fn parse_hex_color(hex: &str) -> u32 {
    let h = hex.trim_start_matches('#');
    u32::from_str_radix(h, 16).unwrap_or(0xfff9db)
}

pub fn is_likely_image_path(value: &str) -> bool {
    let lower = value.to_lowercase();
    [
        ".avif", ".bmp", ".gif", ".jpeg", ".jpg", ".png", ".svg", ".webp",
    ]
    .iter()
    .any(|ext| lower.ends_with(ext))
        || lower.starts_with("data:image/")
}
