//! Separate editor window — cream `#fff9df` shell matching original editor.

use gpui::StyledImage as _;
use gpui::{
    App, AppContext, BoxShadow, ClipboardEntry, Context, Entity, ExternalPaths, FocusHandle,
    Focusable, InteractiveElement, IntoElement, KeyDownEvent, MouseButton, ObjectFit,
    ParentElement, Render, StatefulInteractiveElement, Styled, Window, WindowBounds,
    WindowControlArea, WindowDecorations, WindowOptions, div, img, point, px, relative, rgb, size,
};
use gpui_component::{
    Icon, IconName, IconNamed as _, Sizable as _, StyledExt as _,
    button::{Button, ButtonCustomVariant, ButtonVariants as _},
    h_flex,
    input::{Input, InputEvent, InputState, Paste},
    scroll::ScrollableElement as _,
    tooltip::Tooltip,
    v_flex,
};

use crate::app_state::AppState;
use crate::models::{
    AttachmentKind, AttachmentSource, EDITOR_WINDOW_HEIGHT, EDITOR_WINDOW_MIN_HEIGHT,
    EDITOR_WINDOW_MIN_WIDTH, EDITOR_WINDOW_WIDTH, Language, NOTE_COLORS, NoteAttachment, NoteDraft,
    PendingUpdateDraft,
};
use crate::ui::main_window::{centered_icon_button, q_mark};
use crate::ui::style::{
    ACCENT, DANGER, EDITOR_BG, TEXT, WINDOW_RADIUS, color, color_alpha, parse_note_color,
};

const EDITOR_WINDOW_GAP: f32 = 12.;
const EDITOR_THUMBNAIL_HEIGHT: f32 = 84.;
const PREVIEW_SCALE_STEP: f32 = 0.25;
const PREVIEW_KEY_PAN_STEP: f32 = 36.;

#[derive(Clone)]
struct ImagePreviewState {
    images: Vec<NoteAttachment>,
    index: usize,
    scale: f32,
    offset_x: f32,
    offset_y: f32,
}

#[derive(Clone, Copy)]
struct PreviewDrag {
    start: gpui::Point<gpui::Pixels>,
    offset_x: f32,
    offset_y: f32,
}

pub struct EditorWindow {
    state: Entity<AppState>,
    note_id: Option<String>,
    content: Entity<InputState>,
    media: Entity<InputState>,
    color: String,
    pinned: bool,
    attachments: Vec<NoteAttachment>,
    initial_draft: NoteDraft,
    draft_revision: u64,
    language: Language,
    image_preview: Option<ImagePreviewState>,
    preview_drag: Option<PreviewDrag>,
    preview_focus: FocusHandle,
}

impl EditorWindow {
    pub fn new(
        state: Entity<AppState>,
        note_id: Option<String>,
        recovery: Option<PendingUpdateDraft>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let initial_draft = {
            let app = state.read(cx);
            note_id
                .as_ref()
                .and_then(|id| app.note_by_id(id))
                .map(|note| NoteDraft {
                    attachments: note.attachments.clone(),
                    color: note.color.clone(),
                    content: note.content.clone(),
                    pinned: note.pinned,
                })
                .unwrap_or_default()
        };
        let draft = recovery
            .map(|pending| pending.draft)
            .unwrap_or_else(|| initial_draft.clone());

        let tr = state.read(cx).tr();
        let content = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .placeholder(tr.content_placeholder)
                .default_value(draft.content.clone())
        });
        let media = cx.new(|cx| InputState::new(window, cx).placeholder(tr.media_placeholder));
        cx.subscribe(&content, |this, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                this.sync_draft(cx);
            }
        })
        .detach();
        cx.subscribe_in(&media, window, |this, _, event: &InputEvent, window, cx| {
            if matches!(event, InputEvent::PressEnter { .. }) {
                this.add_media_value(window, cx);
            }
        })
        .detach();

        window.on_window_should_close(cx, {
            let state = state.clone();
            move |window, cx| {
                if state
                    .update(cx, |state, _| state.clear_editor_session())
                    .is_ok()
                {
                    window.remove_window();
                }
                false
            }
        });
        cx.observe(&state, |_, _, cx| cx.notify()).detach();
        let language = state.read(cx).settings.language;

        let mut editor = Self {
            state,
            note_id,
            content,
            media,
            color: draft.color,
            pinned: draft.pinned,
            attachments: draft.attachments,
            initial_draft,
            draft_revision: 0,
            language,
            image_preview: None,
            preview_drag: None,
            preview_focus: cx.focus_handle().tab_stop(true),
        };
        editor.sync_draft(cx);
        editor
    }

    fn current_draft(&self, cx: &Context<Self>) -> NoteDraft {
        NoteDraft {
            attachments: self.attachments.clone(),
            color: self.color.clone(),
            content: self.content.read(cx).value().to_string(),
            pinned: self.pinned,
        }
    }

    fn sync_draft(&mut self, cx: &mut Context<Self>) {
        let draft = self.current_draft(cx);
        let pending = (draft != self.initial_draft).then(|| PendingUpdateDraft {
            note_id: self.note_id.clone(),
            draft,
            saved_at: crate::app_state::now_ms(),
        });
        self.draft_revision = self.draft_revision.wrapping_add(1);
        let revision = self.draft_revision;
        self.state
            .update(cx, |state, _| state.set_editor_draft(pending));

        if !self.state.read(cx).editor_recovery_active {
            return;
        }
        let state = self.state.clone();
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(250))
                .await;
            let is_latest = this
                .update(cx, |this, _| this.draft_revision == revision)
                .unwrap_or(false);
            if is_latest {
                let _ = state.update(cx, |state, _| state.persist_editor_draft());
            }
        })
        .detach();
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let result = self
            .state
            .update(cx, |state, _| state.clear_editor_session());
        if result.is_ok() {
            window.remove_window();
        } else {
            let msg = self.state.read(cx).tr().save_failed.to_string();
            self.state.update(cx, |state, cx| state.show_toast(msg, cx));
        }
    }

    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.content.read(cx).value().to_string();
        if text.trim().is_empty() && self.attachments.is_empty() {
            self.close(window, cx);
            return;
        }
        let draft = NoteDraft {
            attachments: self.attachments.clone(),
            color: self.color.clone(),
            content: text.trim().to_string(),
            pinned: self.pinned,
        };
        let saved_draft = draft.clone();
        let note_id = self.note_id.clone();
        let result = self
            .state
            .update(cx, |s, cx| s.upsert_from_draft(note_id, draft, cx));
        match result {
            Ok(note) => {
                self.note_id = Some(note.id);
                self.initial_draft = saved_draft;
                self.state
                    .update(cx, |state, _| state.set_editor_draft(None));
                let cleared = self
                    .state
                    .update(cx, |state, _| state.clear_editor_session());
                if cleared.is_ok() {
                    let msg = self.state.read(cx).tr().saved.to_string();
                    self.state.update(cx, |s, cx| s.show_toast(msg, cx));
                    window.remove_window();
                } else {
                    let msg = self.state.read(cx).tr().save_failed.to_string();
                    self.state.update(cx, |s, cx| s.show_toast(msg, cx));
                }
            }
            Err(_) => {
                let msg = self.state.read(cx).tr().save_failed.to_string();
                self.state.update(cx, |s, cx| s.show_toast(msg, cx));
            }
        }
    }

    fn pick_images(&mut self, cx: &mut Context<Self>) {
        let files = rfd::FileDialog::new()
            .add_filter(
                "Images",
                &["png", "jpg", "jpeg", "gif", "webp", "bmp", "svg"],
            )
            .pick_files();
        let Some(files) = files else {
            return;
        };
        for path in files {
            let name = path.file_name().map(|s| s.to_string_lossy().to_string());
            let Ok(value) = crate::ui::media::file_to_data_url(&path) else {
                continue;
            };
            self.attachments.push(NoteAttachment {
                id: crate::app_state::create_id("asset"),
                kind: AttachmentKind::Image,
                source: AttachmentSource::Data,
                value,
                name,
                created_at: crate::app_state::now_ms(),
            });
        }
        self.sync_draft(cx);
        cx.notify();
    }

    fn add_media_value(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let value = self.media.read(cx).value().to_string().trim().to_string();
        if value.is_empty() {
            return;
        }
        let source = if value.starts_with("http://") || value.starts_with("https://") {
            AttachmentSource::Url
        } else if value.starts_with("data:") {
            AttachmentSource::Data
        } else {
            AttachmentSource::Path
        };
        let kind = if crate::models::is_likely_image_path(&value) {
            AttachmentKind::Image
        } else {
            AttachmentKind::File
        };
        self.attachments.push(NoteAttachment {
            id: crate::app_state::create_id("asset"),
            kind,
            source,
            value: value.clone(),
            name: attachment_name_from_value(&value),
            created_at: crate::app_state::now_ms(),
        });
        self.media
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.sync_draft(cx);
        cx.notify();
    }

    fn append_paths(&mut self, paths: &ExternalPaths, cx: &mut Context<Self>) {
        for path in paths.paths() {
            let value = path.to_string_lossy().to_string();
            self.attachments.push(NoteAttachment {
                id: crate::app_state::create_id("asset"),
                kind: if crate::models::is_likely_image_path(&value) {
                    AttachmentKind::Image
                } else {
                    AttachmentKind::File
                },
                source: AttachmentSource::Path,
                value,
                name: path
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string()),
                created_at: crate::app_state::now_ms(),
            });
        }
        self.sync_draft(cx);
        cx.notify();
    }

    fn paste_images(&mut self, window: &Window, cx: &mut Context<Self>) -> bool {
        if !self.content.read(cx).focus_handle(cx).is_focused(window) {
            return false;
        }
        let Some(item) = cx.read_from_clipboard() else {
            return false;
        };
        let images: Vec<_> = item
            .entries()
            .iter()
            .filter_map(|entry| match entry {
                ClipboardEntry::Image(image) => Some(image.clone()),
                ClipboardEntry::String(_) => None,
            })
            .collect();
        if images.is_empty() {
            return false;
        }
        for image in images {
            self.attachments.push(NoteAttachment {
                id: crate::app_state::create_id("asset"),
                kind: AttachmentKind::Image,
                source: AttachmentSource::Data,
                value: crate::ui::media::image_to_data_url(&image),
                name: Some(self.state.read(cx).tr().add_image.to_string()),
                created_at: crate::app_state::now_ms(),
            });
        }
        self.sync_draft(cx);
        cx.notify();
        true
    }

    fn open_image_preview(
        &mut self,
        attachment_id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let images: Vec<_> = self
            .attachments
            .iter()
            .filter(|attachment| attachment.kind == AttachmentKind::Image)
            .cloned()
            .collect();
        let Some(index) = images
            .iter()
            .position(|attachment| attachment.id == attachment_id)
        else {
            return;
        };
        self.image_preview = Some(ImagePreviewState {
            images,
            index,
            scale: 1.,
            offset_x: 0.,
            offset_y: 0.,
        });
        self.preview_drag = None;
        self.preview_focus.focus(window);
        cx.notify();
    }

    fn move_image_preview(&mut self, delta: isize, cx: &mut Context<Self>) {
        let Some(preview) = self.image_preview.as_mut() else {
            return;
        };
        let count = preview.images.len() as isize;
        preview.index = (preview.index as isize + delta).rem_euclid(count) as usize;
        preview.scale = 1.;
        preview.offset_x = 0.;
        preview.offset_y = 0.;
        cx.notify();
    }

    fn zoom_image_preview(&mut self, delta: f32, cx: &mut Context<Self>) {
        if let Some(preview) = self.image_preview.as_mut() {
            preview.scale = (preview.scale + delta).clamp(0.5, 6.);
            if preview.scale <= 1. {
                preview.offset_x = 0.;
                preview.offset_y = 0.;
            }
            cx.notify();
        }
    }

    fn reset_image_preview(&mut self, cx: &mut Context<Self>) {
        if let Some(preview) = self.image_preview.as_mut() {
            preview.scale = 1.;
            preview.offset_x = 0.;
            preview.offset_y = 0.;
            cx.notify();
        }
    }

    fn pan_image_preview(&mut self, delta_x: f32, delta_y: f32, cx: &mut Context<Self>) {
        let Some(preview) = self.image_preview.as_mut() else {
            return;
        };
        if preview.scale <= 1. {
            return;
        }
        preview.offset_x += delta_x;
        preview.offset_y += delta_y;
        cx.notify();
    }

    fn handle_image_preview_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let key = event.keystroke.key.to_ascii_lowercase();
        let key_char = event
            .keystroke
            .key_char
            .as_deref()
            .unwrap_or_default()
            .to_ascii_lowercase();
        let key = if key.is_empty() {
            key_char.as_str()
        } else {
            key.as_str()
        };
        match key {
            "escape" | "esc" => {
                self.image_preview = None;
                self.preview_drag = None;
            }
            "left" | "arrowleft" => {
                if self
                    .image_preview
                    .as_ref()
                    .is_some_and(|preview| preview.scale > 1.)
                {
                    self.pan_image_preview(PREVIEW_KEY_PAN_STEP, 0., cx);
                } else {
                    self.move_image_preview(-1, cx);
                }
            }
            "right" | "arrowright" => {
                if self
                    .image_preview
                    .as_ref()
                    .is_some_and(|preview| preview.scale > 1.)
                {
                    self.pan_image_preview(-PREVIEW_KEY_PAN_STEP, 0., cx);
                } else {
                    self.move_image_preview(1, cx);
                }
            }
            "pageup" | "[" => self.move_image_preview(-1, cx),
            "pagedown" | "]" => self.move_image_preview(1, cx),
            "up" | "arrowup" => self.pan_image_preview(0., PREVIEW_KEY_PAN_STEP, cx),
            "down" | "arrowdown" => self.pan_image_preview(0., -PREVIEW_KEY_PAN_STEP, cx),
            "=" | "+" => self.zoom_image_preview(PREVIEW_SCALE_STEP, cx),
            "-" | "_" => self.zoom_image_preview(-PREVIEW_SCALE_STEP, cx),
            "0" => self.reset_image_preview(cx),
            _ => return,
        }
        cx.stop_propagation();
        cx.notify();
    }

    fn begin_image_preview_drag(
        &mut self,
        event: &gpui::MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(preview) = self.image_preview.as_ref() else {
            return;
        };
        cx.stop_propagation();
        if preview.scale <= 1. {
            return;
        }
        self.preview_drag = Some(PreviewDrag {
            start: event.position,
            offset_x: preview.offset_x,
            offset_y: preview.offset_y,
        });
        self.preview_focus.focus(window);
    }

    fn move_image_preview_drag(&mut self, event: &gpui::MouseMoveEvent, cx: &mut Context<Self>) {
        let Some(drag) = self.preview_drag else {
            return;
        };
        let Some(preview) = self.image_preview.as_mut() else {
            self.preview_drag = None;
            return;
        };
        if preview.scale <= 1. {
            self.preview_drag = None;
            return;
        }
        preview.offset_x = drag.offset_x + f32::from(event.position.x - drag.start.x);
        preview.offset_y = drag.offset_y + f32::from(event.position.y - drag.start.y);
        cx.stop_propagation();
        cx.notify();
    }

    fn end_image_preview_drag(&mut self, event: &gpui::MouseUpEvent, cx: &mut Context<Self>) {
        if event.click_count >= 2 {
            self.reset_image_preview(cx);
        }
        self.preview_drag = None;
        cx.stop_propagation();
    }

    fn render_image_preview(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let preview = self.image_preview.clone().unwrap_or(ImagePreviewState {
            images: Vec::new(),
            index: 0,
            scale: 1.,
            offset_x: 0.,
            offset_y: 0.,
        });
        let count = preview.images.len();
        let source = preview
            .images
            .get(preview.index)
            .and_then(crate::ui::media::attachment_image_source);
        let tr = self.state.read(cx).tr();

        div()
            .id("editor-image-preview")
            .absolute()
            .top_0()
            .right_0()
            .bottom_0()
            .left_0()
            .overflow_hidden()
            .bg(color_alpha(0x18212f, 0.76))
            .occlude()
            .track_focus(&self.preview_focus)
            .on_key_down(cx.listener(|this, event, _, cx| {
                this.handle_image_preview_key(event, cx);
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.image_preview = None;
                    cx.notify();
                }),
            )
            .child(
                div()
                    .absolute()
                    .top(px(54.))
                    .right(px(44.))
                    .bottom(px(42.))
                    .left(px(44.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .overflow_hidden()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, event, window, cx| {
                            this.begin_image_preview_drag(event, window, cx);
                        }),
                    )
                    .on_mouse_move(cx.listener(|this, event, _, cx| {
                        this.move_image_preview_drag(event, cx);
                    }))
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(|this, event, _, cx| {
                            this.end_image_preview_drag(event, cx);
                        }),
                    )
                    .on_mouse_up_out(
                        MouseButton::Left,
                        cx.listener(|this, event, _, cx| {
                            this.end_image_preview_drag(event, cx);
                        }),
                    )
                    .on_scroll_wheel(cx.listener(|this, event: &gpui::ScrollWheelEvent, _, cx| {
                        let delta = event.delta.pixel_delta(px(18.));
                        this.zoom_image_preview(
                            if f32::from(delta.y) < 0. {
                                PREVIEW_SCALE_STEP
                            } else {
                                -PREVIEW_SCALE_STEP
                            },
                            cx,
                        );
                        cx.stop_propagation();
                    }))
                    .when_some(source, |stage, source| {
                        stage.child(
                            div()
                                .relative()
                                .left(px(preview.offset_x))
                                .top(px(preview.offset_y))
                                .w(relative(preview.scale))
                                .h(relative(preview.scale))
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    img(source)
                                        .id("editor-preview-image")
                                        .size_full()
                                        .object_fit(ObjectFit::Contain)
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(|this, event, window, cx| {
                                                this.begin_image_preview_drag(event, window, cx);
                                            }),
                                        ),
                                ),
                        )
                    }),
            )
            .when(count > 1, |this| {
                this.child(
                    editor_preview_button(
                        "editor-preview-previous",
                        IconName::ChevronLeft,
                        tr.previous_image,
                    )
                    .absolute()
                    .left_3()
                    .top_1_2()
                    .mt(px(-23.))
                    .h(px(46.))
                    .w(px(34.))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.move_image_preview(-1, cx);
                        cx.stop_propagation();
                    })),
                )
                .child(
                    editor_preview_button(
                        "editor-preview-next",
                        IconName::ChevronRight,
                        tr.next_image,
                    )
                    .absolute()
                    .right_3()
                    .top_1_2()
                    .mt(px(-23.))
                    .h(px(46.))
                    .w(px(34.))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.move_image_preview(1, cx);
                        cx.stop_propagation();
                    })),
                )
            })
            .child(
                h_flex()
                    .absolute()
                    .top_3()
                    .right_3()
                    .gap_1()
                    .p_1()
                    .rounded_full()
                    .border_1()
                    .border_color(color_alpha(0xffffff, 0.36))
                    .bg(color_alpha(0xffffff, 0.22))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .min_w(px(42.))
                            .px_1p5()
                            .text_xs()
                            .text_color(color_alpha(0xffffff, 0.88))
                            .child(format!("{} / {}", preview.index.saturating_add(1), count)),
                    )
                    .child(
                        div()
                            .min_w(px(42.))
                            .px_1p5()
                            .text_xs()
                            .text_color(color_alpha(0xffffff, 0.88))
                            .child(format!("{:.0}%", preview.scale * 100.)),
                    )
                    .child(
                        editor_preview_button(
                            "editor-preview-zoom-out",
                            IconName::Minus,
                            tr.zoom_out,
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.zoom_image_preview(-PREVIEW_SCALE_STEP, cx);
                            cx.stop_propagation();
                        })),
                    )
                    .child(
                        editor_preview_button("editor-preview-zoom-in", IconName::Plus, tr.zoom_in)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.zoom_image_preview(PREVIEW_SCALE_STEP, cx);
                                cx.stop_propagation();
                            })),
                    )
                    .child(
                        editor_preview_button(
                            "editor-preview-reset",
                            IconName::Undo2,
                            tr.reset_view,
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.reset_image_preview(cx);
                            cx.stop_propagation();
                        })),
                    )
                    .child(
                        editor_preview_button("editor-preview-close", IconName::Close, tr.cancel)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.image_preview = None;
                                cx.notify();
                                cx.stop_propagation();
                            })),
                    ),
            )
    }
}

impl Render for EditorWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tr = self.state.read(cx).tr();
        let language = self.state.read(cx).settings.language;
        let lang_key = language.element_key();
        let language_changed = self.language != language;
        if language_changed {
            self.language = language;
            self.content.update(cx, |input, cx| {
                input.set_placeholder(tr.content_placeholder, window, cx);
            });
            self.media.update(cx, |input, cx| {
                input.set_placeholder(tr.media_placeholder, window, cx);
            });
        }
        let title = if self.note_id.is_some() {
            tr.editor_edit_title
        } else {
            tr.editor_new_title
        };
        if language_changed {
            window.set_window_title(title);
        }
        let selected = self.color.clone();
        let always_on_top = self.state.read(cx).settings.always_on_top;
        let state = self.state.clone();
        let window_control_foreground = color_alpha(0x3c3c43, 0.72);
        let image_attachments: Vec<_> = self
            .attachments
            .iter()
            .filter(|attachment| attachment.kind == AttachmentKind::Image)
            .cloned()
            .collect();
        let file_attachments: Vec<_> = self
            .attachments
            .iter()
            .filter(|attachment| attachment.kind == AttachmentKind::File)
            .cloned()
            .collect();

        v_flex()
            .id("editor-shell")
            .relative()
            .size_full()
            .overflow_hidden()
            .bg(color(EDITOR_BG))
            .rounded(px(WINDOW_RADIUS))
            .p_3()
            .gap_3()
            .text_color(color(TEXT))
            .child(
                h_flex()
                    .h(px(34.))
                    .w_full()
                    .items_center()
                    .justify_between()
                    .child(
                        h_flex()
                            .h_full()
                            .min_w_0()
                            .flex_1()
                            .gap_2()
                            .items_center()
                            .window_control_area(WindowControlArea::Drag)
                            .on_mouse_down(MouseButton::Left, |_, window, _| {
                                window.start_window_move();
                            })
                            .child(q_mark(20.))
                            .child(div().text_sm().font_semibold().child(title)),
                    )
                    .child(
                        h_flex()
                            .flex_none()
                            .gap(px(2.))
                            .rounded_full()
                            .p(px(2.))
                            .bg(color_alpha(0xffffff, 0.52))
                            .child(
                                centered_icon_button(
                                    "editor-topmost",
                                    if always_on_top {
                                        crate::PIN_OFF_ICON_PATH
                                    } else {
                                        crate::PIN_ICON_PATH
                                    },
                                    |s| {
                                        if s.settings.always_on_top {
                                            s.tr().always_off
                                        } else {
                                            s.tr().always_on
                                        }
                                    },
                                    state.clone(),
                                    lang_key,
                                    if always_on_top {
                                        color(ACCENT)
                                    } else {
                                        window_control_foreground
                                    },
                                    color(0xffffff),
                                    color(ACCENT),
                                )
                                .on_click({
                                    let state = self.state.clone();
                                    move |_, window, cx| {
                                        let next = !state.read(cx).settings.always_on_top;
                                        crate::ui::apply_window_topmost(window, next);
                                        state.update(cx, |state, cx| {
                                            state.set_always_on_top(next, cx);
                                        });
                                    }
                                }),
                            )
                            .child(
                                centered_icon_button(
                                    "editor-minimize",
                                    IconName::Minus.path(),
                                    |s| s.tr().minimize,
                                    state.clone(),
                                    lang_key,
                                    window_control_foreground,
                                    color(0xffffff),
                                    color(0xffcc00),
                                )
                                .on_click(|_, window, _| window.minimize_window()),
                            )
                            .child(
                                centered_icon_button(
                                    "editor-close",
                                    IconName::Close.path(),
                                    |s| s.tr().close_panel,
                                    state,
                                    lang_key,
                                    window_control_foreground,
                                    color(0xffffff),
                                    color(DANGER),
                                )
                                .on_click(cx.listener(
                                    |this, _, window, cx| {
                                        this.close(window, cx);
                                    },
                                )),
                            ),
                    ),
            )
            .child(
                h_flex()
                    .gap(px(7.))
                    .flex_wrap()
                    .children(NOTE_COLORS.iter().map(|c| {
                        let c = (*c).to_string();
                        let selected = selected == c;
                        div()
                            .id(gpui::SharedString::from(format!(
                                "editor-swatch-{}",
                                c.clone()
                            )))
                            .size(px(20.))
                            .rounded_full()
                            .border_1()
                            .border_color(if selected {
                                color_alpha(0x1d2735, 0.45)
                            } else {
                                color_alpha(0x1d2735, 0.16)
                            })
                            .bg(parse_note_color(&c))
                            .when(selected, |style| {
                                style.shadow(vec![BoxShadow {
                                    color: color_alpha(0x1d2735, 0.08).into(),
                                    offset: point(px(0.), px(0.)),
                                    blur_radius: px(0.),
                                    spread_radius: px(3.),
                                }])
                            })
                            .hover(|style| style.border_color(color_alpha(0x1d2735, 0.45)))
                            .cursor_pointer()
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener({
                                    let c = c.clone();
                                    move |this, _, _, cx| {
                                        this.color = c.clone();
                                        this.sync_draft(cx);
                                        cx.notify();
                                    }
                                }),
                            )
                    })),
            )
            .child(
                div()
                    .id("editor-content")
                    .flex_1()
                    .min_h(px(140.))
                    .w_full()
                    .overflow_hidden()
                    .border_t_1()
                    .border_b_1()
                    .border_color(color_alpha(0x3c3c43, 0.10))
                    .bg(color_alpha(0xffffff, 0.50))
                    .child(
                        Input::new(&self.content)
                            .appearance(false)
                            .bordered(false)
                            .focus_bordered(false)
                            .h_full()
                            .px(px(16.))
                            .py(px(14.))
                            .line_height(relative(1.55))
                            .text_color(color(0x1d1d1f)),
                    ),
            )
            .child(
                h_flex()
                    .gap(px(10.))
                    .w_full()
                    .flex_wrap()
                    .child(
                        Button::new(("add-image", lang_key))
                            .custom(editor_secondary_button(cx))
                            .rounded(px(8.))
                            .h(px(34.))
                            .px(px(10.))
                            .text_sm()
                            .icon(IconName::GalleryVerticalEnd)
                            .label(tr.add_image)
                            .on_click(cx.listener(|this, _, _, cx| this.pick_images(cx))),
                    )
                    .child(
                        h_flex()
                            .id(("editor-media-input", lang_key))
                            .flex_1()
                            .min_w(px(260.))
                            .h(px(34.))
                            .min_w_0()
                            .gap(px(7.))
                            .px(px(10.))
                            .rounded(px(8.))
                            .border_1()
                            .border_color(color_alpha(0x3c3c43, 0.12))
                            .bg(color_alpha(0xffffff, 0.62))
                            .text_color(color_alpha(0x3c3c43, 0.62))
                            .child(Icon::new(IconName::ExternalLink).with_size(px(14.)))
                            .child(
                                div().flex_1().min_w_0().h_full().child(
                                    Input::new(&self.media)
                                        .appearance(false)
                                        .bordered(false)
                                        .focus_bordered(false)
                                        .px_0()
                                        .py_0()
                                        .text_color(color(0x1d1d1f)),
                                ),
                            )
                            .child(Icon::new(IconName::Folder).with_size(px(14.))),
                    )
                    .child(
                        Button::new(("add-media", lang_key))
                            .custom(editor_secondary_button(cx))
                            .rounded(px(8.))
                            .h(px(34.))
                            .px(px(10.))
                            .text_sm()
                            .icon(IconName::GalleryVerticalEnd)
                            .label(tr.add_media)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.add_media_value(window, cx);
                            })),
                    ),
            )
            .when(
                !image_attachments.is_empty() || !file_attachments.is_empty(),
                |this| {
                    this.child(
                        v_flex()
                            .w_full()
                            .max_h(px(180.))
                            .overflow_y_scrollbar()
                            .gap(px(10.))
                            .px(px(12.))
                            .when(!image_attachments.is_empty(), |this| {
                                this.child(h_flex().w_full().flex_wrap().gap_2().children(
                                    image_attachments.iter().filter_map(|attachment| {
                                        let source =
                                            crate::ui::media::attachment_image_source(attachment)?;
                                        let attachment_id = attachment.id.clone();
                                        let remove_id = attachment.id.clone();
                                        Some(
                                            div()
                                                .id(gpui::SharedString::from(format!(
                                                    "editor-image-{}",
                                                    attachment.id
                                                )))
                                                .relative()
                                                .w(px(108.))
                                                .h(px(EDITOR_THUMBNAIL_HEIGHT))
                                                .overflow_hidden()
                                                .rounded(px(8.))
                                                .border_1()
                                                .border_color(color_alpha(0x1d2735, 0.14))
                                                .bg(color_alpha(0xffffff, 0.50))
                                                .child(
                                                    img(source)
                                                        .id(gpui::SharedString::from(format!(
                                                            "editor-image-preview-{}",
                                                            attachment.id
                                                        )))
                                                        .size_full()
                                                        .object_fit(ObjectFit::Cover)
                                                        .cursor_pointer()
                                                        .on_click(cx.listener(
                                                            move |this, _, window, cx| {
                                                                this.open_image_preview(
                                                                    &attachment_id,
                                                                    window,
                                                                    cx,
                                                                );
                                                            },
                                                        )),
                                                )
                                                .child(
                                                    div()
                                                        .id(gpui::SharedString::from(format!(
                                                            "remove-image-{}",
                                                            attachment.id
                                                        )))
                                                        .absolute()
                                                        .top(px(6.))
                                                        .right(px(6.))
                                                        .size(px(22.))
                                                        .rounded_full()
                                                        .flex()
                                                        .items_center()
                                                        .justify_center()
                                                        .cursor_pointer()
                                                        .bg(color_alpha(0xffffff, 0.86))
                                                        .text_color(color(0xa61e4d))
                                                        .hover(|style| {
                                                            style.bg(color_alpha(0xffffff, 0.64))
                                                        })
                                                        .tooltip(move |window, cx| {
                                                            Tooltip::new(tr.remove_attachment)
                                                                .build(window, cx)
                                                        })
                                                        .on_click(cx.listener(
                                                            move |this, _, _, cx| {
                                                                this.attachments.retain(
                                                                    |attachment| {
                                                                        attachment.id != remove_id
                                                                    },
                                                                );
                                                                this.sync_draft(cx);
                                                                cx.notify();
                                                            },
                                                        ))
                                                        .child(
                                                            Icon::new(IconName::Delete)
                                                                .with_size(px(12.)),
                                                        ),
                                                )
                                                .into_any_element(),
                                        )
                                    }),
                                ))
                            })
                            .children(file_attachments.iter().map(|attachment| {
                                let remove_id = attachment.id.clone();
                                h_flex()
                                    .id(gpui::SharedString::from(format!(
                                        "editor-file-{}",
                                        attachment.id
                                    )))
                                    .w_full()
                                    .min_w_0()
                                    .min_h(px(38.))
                                    .gap(px(7.))
                                    .p_2()
                                    .rounded(px(8.))
                                    .border_1()
                                    .border_color(color_alpha(0x3c3c43, 0.12))
                                    .bg(color_alpha(0xffffff, 0.62))
                                    .child(Icon::new(IconName::File).with_size(px(14.)))
                                    .child(
                                        div()
                                            .min_w_0()
                                            .flex_1()
                                            .overflow_hidden()
                                            .text_ellipsis()
                                            .whitespace_nowrap()
                                            .text_xs()
                                            .child(attachment_label(attachment)),
                                    )
                                    .child(
                                        Button::new(gpui::SharedString::from(format!(
                                            "remove-file-{}",
                                            attachment.id
                                        )))
                                        .ghost()
                                        .xsmall()
                                        .icon(IconName::Delete)
                                        .tooltip(tr.remove_attachment)
                                        .on_click(
                                            cx.listener(move |this, _, _, cx| {
                                                this.attachments.retain(|attachment| {
                                                    attachment.id != remove_id
                                                });
                                                this.sync_draft(cx);
                                                cx.notify();
                                            }),
                                        ),
                                    )
                            })),
                    )
                },
            )
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .child({
                        let pinned = self.pinned;
                        h_flex()
                            .id(("pin", lang_key))
                            .gap(px(8.))
                            .items_center()
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.pinned = !this.pinned;
                                this.sync_draft(cx);
                                cx.notify();
                            }))
                            .child(
                                div()
                                    .size(px(16.))
                                    .flex_none()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(2.))
                                    .border_1()
                                    .border_color(if pinned {
                                        color(ACCENT)
                                    } else {
                                        color_alpha(0x3c3c43, 0.34)
                                    })
                                    .bg(if pinned {
                                        color(ACCENT)
                                    } else {
                                        color_alpha(0xffffff, 0.72)
                                    })
                                    .when(pinned, |this| {
                                        this.child(
                                            Icon::new(IconName::Check)
                                                .with_size(px(11.))
                                                .text_color(rgb(0xffffff)),
                                        )
                                    }),
                            )
                            .child(div().text_sm().text_color(color(0x1d1d1f)).child(tr.pin))
                    })
                    .child(
                        h_flex()
                            .gap(px(8.))
                            .child(
                                Button::new(("cancel", lang_key))
                                    .custom(editor_text_button(cx))
                                    .rounded(px(8.))
                                    .h(px(28.))
                                    .px(px(10.))
                                    .text_sm()
                                    .label(tr.cancel)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.close(window, cx);
                                    })),
                            )
                            .child(
                                Button::new(("save", lang_key))
                                    .custom(editor_primary_button(cx))
                                    .rounded(px(8.))
                                    .h(px(28.))
                                    .px(px(10.))
                                    .text_sm()
                                    .text_color(color(0xffffff))
                                    .label(tr.save)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.save(window, cx);
                                    })),
                            ),
                    ),
            )
            .drag_over::<ExternalPaths>(|style, _, _, _| {
                style.border_2().border_color(color_alpha(ACCENT, 0.72))
            })
            .on_drop::<ExternalPaths>(cx.listener(|this, paths, _, cx| {
                this.append_paths(paths, cx);
            }))
            .capture_action(cx.listener(|this, _: &Paste, window, cx| {
                if this.paste_images(window, cx) {
                    cx.stop_propagation();
                }
            }))
            .when(self.image_preview.is_some(), |this| {
                this.child(self.render_image_preview(cx))
            })
    }
}

fn editor_secondary_button(cx: &App) -> ButtonCustomVariant {
    ButtonCustomVariant::new(cx)
        .color(color_alpha(0xffffff, 0.46).into())
        .foreground(color_alpha(0x1d1d1f, 0.82).into())
        .border(color_alpha(0x3c3c43, 0.08).into())
        .hover(color_alpha(0xffffff, 0.64).into())
        .active(color_alpha(0xffffff, 0.64).into())
}

fn editor_text_button(cx: &App) -> ButtonCustomVariant {
    ButtonCustomVariant::new(cx)
        .color(color_alpha(0xffffff, 0.72).into())
        .foreground(color(0x1d1d1f).into())
        .border(color_alpha(0x3c3c43, 0.13).into())
        .hover(color_alpha(0xffffff, 0.64).into())
        .active(color_alpha(0xffffff, 0.64).into())
}

fn editor_primary_button(cx: &App) -> ButtonCustomVariant {
    ButtonCustomVariant::new(cx)
        .color(color_alpha(0x1d1d1f, 0.92).into())
        .foreground(color(0xffffff).into())
        .border(color_alpha(0x1d1d1f, 0.18).into())
        .hover(color_alpha(0x1d1d1f, 0.75).into())
        .active(color_alpha(0x1d1d1f, 0.75).into())
}

fn editor_preview_button(
    id: impl Into<gpui::SharedString>,
    icon: IconName,
    tooltip: impl Into<gpui::SharedString>,
) -> gpui::Stateful<gpui::Div> {
    let tooltip = tooltip.into();
    div()
        .id(id.into())
        .size(px(28.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .cursor_pointer()
        .text_color(rgb(0xffffff))
        .hover(|style| style.bg(color_alpha(0xffffff, 0.20)))
        .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(Icon::new(icon).with_size(px(14.)))
}

fn attachment_label(attachment: &NoteAttachment) -> String {
    attachment
        .name
        .clone()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| attachment.value.clone())
}

fn attachment_name_from_value(value: &str) -> Option<String> {
    value
        .rsplit(['/', '\\'])
        .next()
        .filter(|name| !name.is_empty() && !name.starts_with("data:"))
        .map(str::to_string)
}

pub fn open_editor(
    state: Entity<AppState>,
    note_id: Option<String>,
    from_recovery: bool,
    cx: &mut App,
) {
    // Ensure main is visible when opening editor from dock.
    if state.read(cx).settings.docked {
        crate::ui::main_window::restore_from_dock(state.clone(), cx);
        cx.defer(move |cx| open_editor(state, note_id, from_recovery, cx));
        return;
    }

    let recovery = if from_recovery {
        state.read(cx).editor_draft_recovery.clone()
    } else {
        None
    };
    let note_id = recovery
        .as_ref()
        .and_then(|pending| pending.note_id.clone())
        .or(note_id);
    let always_on_top = state.read(cx).settings.always_on_top;
    let title = {
        let tr = state.read(cx).tr();
        if note_id.is_some() {
            tr.editor_edit_title
        } else {
            tr.editor_new_title
        }
        .to_string()
    };

    let existing = state.read(cx).editor_window;
    if let Some(existing) = existing {
        if state
            .update(cx, |state, _| state.clear_editor_session())
            .is_err()
        {
            let _ = existing.update(cx, |_, window, _| window.activate_window());
            let msg = state.read(cx).tr().save_failed.to_string();
            state.update(cx, |state, cx| state.show_toast(msg, cx));
            return;
        }
        let _ = existing.update(cx, |_, window, _| window.remove_window());
    }

    if from_recovery {
        state.update(cx, |state, _| {
            state.editor_draft_recovery = None;
            state.editor_recovery_active = recovery.is_some();
        });
    }

    let bounds = editor_window_bounds(&state, cx);

    let handle = cx
        .open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some(title.into()),
                    appears_transparent: true,
                    ..Default::default()
                }),
                window_min_size: Some(size(
                    px(EDITOR_WINDOW_MIN_WIDTH),
                    px(EDITOR_WINDOW_MIN_HEIGHT),
                )),
                kind: crate::ui::standard_window_kind(always_on_top),
                is_resizable: true,
                is_movable: true,
                focus: true,
                show: true,
                window_decorations: Some(WindowDecorations::Client),
                ..Default::default()
            },
            {
                let state = state.clone();
                let note_id = note_id.clone();
                let recovery = recovery.clone();
                move |window, cx| {
                    let view = cx.new(|cx| {
                        EditorWindow::new(
                            state.clone(),
                            note_id.clone(),
                            recovery.clone(),
                            window,
                            cx,
                        )
                    });
                    cx.new(|cx| gpui_component::Root::new(view, window, cx))
                }
            },
        )
        .expect("open editor");

    state.update(cx, |s, _| s.editor_window = Some(handle));
    let _ = handle.update(cx, |_, window, _| {
        crate::ui::apply_window_topmost(window, always_on_top);
    });
}

fn editor_window_bounds(state: &Entity<AppState>, cx: &mut App) -> gpui::Bounds<gpui::Pixels> {
    let editor_size = size(px(EDITOR_WINDOW_WIDTH), px(EDITOR_WINDOW_HEIGHT));
    let Some(main) = state.read(cx).main_window else {
        return gpui::Bounds::centered(None, editor_size, cx);
    };
    let Ok((main_bounds, work_area)) = main.update(cx, |_, window, cx| {
        (
            window.bounds(),
            window.display(cx).map(|display| display.bounds()),
        )
    }) else {
        return gpui::Bounds::centered(None, editor_size, cx);
    };

    let target_x = f32::from(main_bounds.origin.x) - EDITOR_WINDOW_WIDTH - EDITOR_WINDOW_GAP;
    let target_y = f32::from(main_bounds.origin.y)
        + (f32::from(main_bounds.size.height) - EDITOR_WINDOW_HEIGHT) / 2.;
    let Some(area) = work_area else {
        return gpui::Bounds {
            origin: gpui::point(px(target_x), px(target_y)),
            size: editor_size,
        };
    };
    let left = f32::from(area.origin.x);
    let top = f32::from(area.origin.y);
    let right = left + f32::from(area.size.width);
    let bottom = top + f32::from(area.size.height);
    let max_x = (right - EDITOR_WINDOW_WIDTH).max(left);
    let max_y = (bottom - EDITOR_WINDOW_HEIGHT).max(top);
    gpui::Bounds {
        origin: gpui::point(
            px(target_x.clamp(left, max_x)),
            px(target_y.clamp(top, max_y)),
        ),
        size: editor_size,
    }
}

trait WhenExt: Sized {
    fn when(self, cond: bool, f: impl FnOnce(Self) -> Self) -> Self {
        if cond { f(self) } else { self }
    }
    fn when_some<T>(self, value: Option<T>, f: impl FnOnce(Self, T) -> Self) -> Self {
        match value {
            Some(value) => f(self, value),
            None => self,
        }
    }
}
impl<T> WhenExt for T {}
