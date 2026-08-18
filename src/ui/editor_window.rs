//! Separate editor window — cream `#fff9df` shell matching original editor.

use gpui::{
    App, AppContext, Context, Entity, InteractiveElement, IntoElement, MouseButton, ParentElement,
    Render, Styled, Window, WindowBounds, WindowDecorations, WindowOptions, div, px, size,
};
use gpui_component::{
    IconName, Sizable as _, StyledExt as _, TitleBar,
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    h_flex,
    input::{Input, InputEvent, InputState},
    v_flex,
};

use crate::app_state::AppState;
use crate::models::{
    EDITOR_WINDOW_HEIGHT, EDITOR_WINDOW_MIN_HEIGHT, EDITOR_WINDOW_MIN_WIDTH, EDITOR_WINDOW_WIDTH,
    NOTE_COLORS, NoteAttachment, NoteDraft, PendingUpdateDraft,
};
use crate::ui::main_window::q_mark;
use crate::ui::style::{EDITOR_BG, TEXT, WINDOW_RADIUS, color, color_alpha, parse_note_color};

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

        window.on_window_should_close(cx, |_, _| false);

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
            content: text,
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
            let path_str = path.to_string_lossy().to_string();
            self.attachments.push(NoteAttachment {
                id: crate::app_state::create_id("asset"),
                kind: crate::models::AttachmentKind::Image,
                source: crate::models::AttachmentSource::Path,
                value: path_str,
                name,
                created_at: crate::app_state::now_ms(),
            });
        }
        self.sync_draft(cx);
        cx.notify();
    }
}

impl Render for EditorWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tr = self.state.read(cx).tr();
        let title = if self.note_id.is_some() {
            tr.editor_edit_title
        } else {
            tr.editor_new_title
        };
        let selected = self.color.clone();

        v_flex()
            .id("editor-shell")
            .size_full()
            .bg(color(EDITOR_BG))
            .rounded(px(WINDOW_RADIUS))
            .p_3()
            .gap_3()
            .text_color(color(TEXT))
            .child(
                TitleBar::new().child(
                    h_flex()
                        .w_full()
                        .items_center()
                        .justify_between()
                        .child(
                            h_flex()
                                .gap_2()
                                .items_center()
                                .child(q_mark(20.))
                                .child(div().text_sm().font_semibold().child(title)),
                        )
                        .child(
                            Button::new("editor-close")
                                .ghost()
                                .icon(IconName::WindowClose)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.close(window, cx);
                                })),
                        ),
                ),
            )
            .child(
                h_flex()
                    .gap_1()
                    .flex_wrap()
                    .children(NOTE_COLORS.iter().map(|c| {
                        let c = (*c).to_string();
                        let selected = selected == c;
                        div()
                            .id(gpui::SharedString::from(format!(
                                "editor-swatch-{}",
                                c.clone()
                            )))
                            .size_6()
                            .rounded_full()
                            .border_2()
                            .border_color(if selected {
                                color(0x007aff)
                            } else {
                                color_alpha(0x1f2328, 0.12)
                            })
                            .bg(parse_note_color(&c))
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
                    .w_full()
                    .child(Input::new(&self.content).h(px(220.))),
            )
            .child(
                h_flex()
                    .gap_2()
                    .w_full()
                    .child(
                        Button::new("add-image")
                            .outline()
                            .icon(IconName::File)
                            .label(tr.add_image)
                            .on_click(cx.listener(|this, _, _, cx| this.pick_images(cx))),
                    )
                    .child(div().flex_1().child(Input::new(&self.media)))
                    .child(
                        Button::new("add-media")
                            .outline()
                            .label(tr.add_media)
                            .on_click(cx.listener(|this, _, _, cx| {
                                let value =
                                    this.media.read(cx).value().to_string().trim().to_string();
                                if value.is_empty() {
                                    return;
                                }
                                let source = if value.starts_with("http://")
                                    || value.starts_with("https://")
                                {
                                    crate::models::AttachmentSource::Url
                                } else if value.starts_with("data:") {
                                    crate::models::AttachmentSource::Data
                                } else {
                                    crate::models::AttachmentSource::Path
                                };
                                let kind = if crate::models::is_likely_image_path(&value) {
                                    crate::models::AttachmentKind::Image
                                } else {
                                    crate::models::AttachmentKind::File
                                };
                                this.attachments.push(NoteAttachment {
                                    id: crate::app_state::create_id("asset"),
                                    kind,
                                    source,
                                    value,
                                    name: None,
                                    created_at: crate::app_state::now_ms(),
                                });
                                this.sync_draft(cx);
                                // Reset media field by replacing entity content via set_value if available
                                cx.notify();
                            })),
                    ),
            )
            .when(!self.attachments.is_empty(), |this| {
                this.child(v_flex().gap_1().w_full().children(
                    self.attachments.iter().enumerate().map(|(idx, att)| {
                        let label = att
                            .name
                            .clone()
                            .unwrap_or_else(|| att.value.chars().take(48).collect());
                        h_flex()
                            .id(gpui::SharedString::from(format!("att-{}", att.id.clone())))
                            .w_full()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .px_2()
                            .py_1()
                            .rounded(px(8.))
                            .bg(color_alpha(0xffffff, 0.55))
                            .child(div().text_xs().flex_1().overflow_hidden().child(label))
                            .child(
                                Button::new(gpui::SharedString::from(format!(
                                    "rm-{}",
                                    att.id.clone()
                                )))
                                .ghost()
                                .xsmall()
                                .icon(IconName::Close)
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        if idx < this.attachments.len() {
                                            this.attachments.remove(idx);
                                            this.sync_draft(cx);
                                            cx.notify();
                                        }
                                    },
                                )),
                            )
                    }),
                ))
            })
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .child(
                        Checkbox::new("pin")
                            .label(tr.pin)
                            .checked(self.pinned)
                            .on_click(cx.listener(|this, checked, _, cx| {
                                this.pinned = *checked;
                                this.sync_draft(cx);
                                cx.notify();
                            })),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(Button::new("cancel").outline().label(tr.cancel).on_click(
                                cx.listener(|this, _, window, cx| {
                                    this.close(window, cx);
                                }),
                            ))
                            .child(Button::new("save").primary().label(tr.save).on_click(
                                cx.listener(|this, _, window, cx| {
                                    this.save(window, cx);
                                }),
                            )),
                    ),
            )
    }
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

    let bounds = gpui::Bounds::centered(
        None,
        size(px(EDITOR_WINDOW_WIDTH), px(EDITOR_WINDOW_HEIGHT)),
        cx,
    );

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

trait WhenExt: Sized {
    fn when(self, cond: bool, f: impl FnOnce(Self) -> Self) -> Self {
        if cond { f(self) } else { self }
    }
}
impl<T> WhenExt for T {}
