//! Separate editor window — cream `#fff9df` shell matching original editor.

use gpui::{
    App, AppContext, Context, Entity, InteractiveElement, IntoElement, MouseButton, ParentElement,
    Render, Styled, Window, WindowBounds, WindowDecorations, WindowKind, WindowOptions, div, px,
    size,
};
use gpui_component::{
    IconName, Sizable as _, StyledExt as _, TitleBar, h_flex, v_flex,
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    input::{Input, InputState},
};

use crate::app_state::AppState;
use crate::models::{
    DEFAULT_NOTE_COLOR, EDITOR_WINDOW_HEIGHT, EDITOR_WINDOW_MIN_HEIGHT, EDITOR_WINDOW_MIN_WIDTH,
    EDITOR_WINDOW_WIDTH, NOTE_COLORS, NoteAttachment, NoteDraft,
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
}

impl EditorWindow {
    pub fn new(
        state: Entity<AppState>,
        note_id: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let (draft_content, draft_color, draft_pinned, attachments) = {
            let app = state.read(cx);
            if let Some(recovery) = app.editor_draft_recovery.as_ref() {
                (
                    recovery.draft.content.clone(),
                    recovery.draft.color.clone(),
                    recovery.draft.pinned,
                    recovery.draft.attachments.clone(),
                )
            } else if let Some(id) = note_id.as_ref() {
                if let Some(note) = app.note_by_id(id) {
                    (
                        note.content.clone(),
                        note.color.clone(),
                        note.pinned,
                        note.attachments.clone(),
                    )
                } else {
                    (String::new(), DEFAULT_NOTE_COLOR.to_string(), false, Vec::new())
                }
            } else {
                (String::new(), DEFAULT_NOTE_COLOR.to_string(), false, Vec::new())
            }
        };

        let tr = state.read(cx).tr();
        let content = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .placeholder(tr.content_placeholder)
                .default_value(draft_content)
        });
        let media = cx.new(|cx| {
            InputState::new(window, cx).placeholder(tr.media_placeholder)
        });

        window.on_window_should_close(cx, |_, _| false);

        Self {
            state,
            note_id,
            content,
            media,
            color: draft_color,
            pinned: draft_pinned,
            attachments,
        }
    }

    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.content.read(cx).value().to_string();
        if text.trim().is_empty() && self.attachments.is_empty() {
            window.remove_window();
            return;
        }
        let draft = NoteDraft {
            attachments: self.attachments.clone(),
            color: self.color.clone(),
            content: text,
            pinned: self.pinned,
        };
        let note_id = self.note_id.clone();
        let result = self
            .state
            .update(cx, |s, cx| s.upsert_from_draft(note_id, draft, cx));
        if result.is_ok() {
            let msg = self.state.read(cx).tr().saved.to_string();
            self.state.update(cx, |s, cx| s.show_toast(msg, cx));
        }
        window.remove_window();
    }

    fn pick_images(&mut self, cx: &mut Context<Self>) {
        let files = rfd::FileDialog::new()
            .add_filter("Images", &["png", "jpg", "jpeg", "gif", "webp", "bmp", "svg"])
            .pick_files();
        let Some(files) = files else {
            return;
        };
        for path in files {
            let name = path
                .file_name()
                .map(|s| s.to_string_lossy().to_string());
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
                                .on_click(|_, window, _| window.remove_window()),
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
                            .id(gpui::SharedString::from(format!("editor-swatch-{}", c.clone())))
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
                            .on_mouse_down(MouseButton::Left, cx.listener({
                                let c = c.clone();
                                move |this, _, _, cx| {
                                    this.color = c.clone();
                                    cx.notify();
                                }
                            }))
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
                                let value = this.media.read(cx).value().to_string().trim().to_string();
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
                                // Reset media field by replacing entity content via set_value if available
                                cx.notify();
                            })),
                    ),
            )
            .when(!self.attachments.is_empty(), |this| {
                this.child(
                    v_flex()
                        .gap_1()
                        .w_full()
                        .children(self.attachments.iter().enumerate().map(|(idx, att)| {
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
                                    Button::new(gpui::SharedString::from(format!("rm-{}", att.id.clone())))
                                        .ghost()
                                        .xsmall()
                                        .icon(IconName::Close)
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            if idx < this.attachments.len() {
                                                this.attachments.remove(idx);
                                                cx.notify();
                                            }
                                        })),
                                )
                        })),
                )
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
                                cx.notify();
                            })),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("cancel")
                                    .outline()
                                    .label(tr.cancel)
                                    .on_click(|_, window, _| window.remove_window()),
                            )
                            .child(
                                Button::new("save")
                                    .primary()
                                    .label(tr.save)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.save(window, cx);
                                    })),
                            ),
                    ),
            )
    }
}

pub fn open_editor(
    state: Entity<AppState>,
    note_id: Option<String>,
    _from_recovery: bool,
    cx: &mut App,
) {
    // Ensure main is visible when opening editor from dock.
    if state.read(cx).settings.docked {
        crate::ui::main_window::restore_from_dock(state.clone(), cx);
    }

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

    if let Some(existing) = state.read(cx).editor_window.clone() {
        let _ = existing.update(cx, |_, window, _| {
            window.set_window_title(&title);
            window.activate_window();
        });
        // For simplicity, recreate with fresh draft by removing and reopening.
        let _ = existing.update(cx, |_, window, _| window.remove_window());
        state.update(cx, |s, _| s.editor_window = None);
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
                kind: if always_on_top {
                    WindowKind::PopUp
                } else {
                    WindowKind::Normal
                },
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
                move |window, cx| {
                    let view =
                        cx.new(|cx| EditorWindow::new(state.clone(), note_id.clone(), window, cx));
                    cx.new(|cx| gpui_component::Root::new(view, window, cx))
                }
            },
        )
        .expect("open editor");

    state.update(cx, |s, _| s.editor_window = Some(handle));
}

trait WhenExt: Sized {
    fn when(self, cond: bool, f: impl FnOnce(Self) -> Self) -> Self {
        if cond { f(self) } else { self }
    }
}
impl<T> WhenExt for T {}
