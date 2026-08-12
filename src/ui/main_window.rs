//! Main note board window — layout/colors match the original Vue shell.

use gpui::{
    App, AppContext, Context, Entity, InteractiveElement, IntoElement, MouseButton, ParentElement,
    Render, StatefulInteractiveElement, Styled, Window, WindowBounds, WindowDecorations,
    WindowKind, WindowOptions, div, px, rgb, size,
};
use gpui_component::{
    Disableable as _, IconName, Sizable as _, StyledExt as _, TitleBar, WindowExt as _, h_flex,
    v_flex,
    button::{Button, ButtonVariants as _},
    menu::{ContextMenuExt, PopupMenuItem},
    scroll::ScrollableElement as _,
};

use crate::app_state::AppState;
use crate::models::{DEFAULT_WINDOW_HEIGHT, DEFAULT_WINDOW_WIDTH, NOTE_COLORS, Note};
use crate::ui::dock_window;
use crate::ui::editor_window;
use crate::ui::style::{
    APP_BG, CARD_RADIUS, DANGER, LINE_HEIGHT, TEXT, TOOLBAR_RADIUS, WINDOW_RADIUS, card_shadow,
    chrome_chip, color, color_alpha, parse_note_color, toolbar_chip,
};
use crate::updater;

pub struct MainWindow {
    state: Entity<AppState>,
    palette_note_id: Option<String>,
}

impl MainWindow {
    pub fn new(state: Entity<AppState>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        window.on_window_should_close(cx, {
            let state = state.clone();
            move |window, cx| {
                // Close hides to tray instead of quitting (parity with Tauri).
                let _ = window;
                let _ = state;
                let _ = cx;
                false
            }
        });

        // Restore pending editor draft after update if present.
        if state.read(cx).editor_draft_recovery.is_some() {
            let state = state.clone();
            cx.defer(move |cx| {
                editor_window::open_editor(state.clone(), None, true, cx);
                state.update(cx, |s, _| {
                    let _ = s.db.clear_pending_update_draft();
                    s.editor_draft_recovery = None;
                });
            });
        }

        cx.observe(&state, |_, _, cx| cx.notify()).detach();
        Self {
            state,
            palette_note_id: None,
        }
    }

    fn open_new(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        editor_window::open_editor(self.state.clone(), None, false, cx);
    }

    fn open_edit(&mut self, note_id: String, cx: &mut Context<Self>) {
        editor_window::open_editor(self.state.clone(), Some(note_id), false, cx);
    }

    fn confirm_delete_all(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let state = self.state.clone();
        let tr = state.read(cx).tr();
        let title = tr.confirm_delete_all.to_string();
        let body = tr.delete_all_body.to_string();
        let ok = tr.delete.to_string();
        let cancel = tr.cancel.to_string();
        window.open_dialog(cx, move |dialog, _, _| {
            dialog
                .title(title.clone())
                .child(body.clone())
                .confirm()
                .button_props(
                    gpui_component::dialog::DialogButtonProps::default()
                        .ok_text(ok.clone())
                        .cancel_text(cancel.clone())
                        .ok_variant(gpui_component::button::ButtonVariant::Danger),
                )
                .on_ok({
                    let state = state.clone();
                    move |_, _, cx| {
                        let _ = state.update(cx, |s, cx| s.delete_all_notes(cx));
                        true
                    }
                })
        });
    }

    fn open_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let state = self.state.clone();
        let tr = state.read(cx).tr();
        let title = tr.settings_title.to_string();
        let auto_label = tr.startup_setting.to_string();
        let auto_on = state.read(cx).settings.auto_start;
        let export_label = tr.export.to_string();
        let import_label = tr.import.to_string();
        let check_label = tr.check_update.to_string();
        let version = updater::PACKAGE_VERSION.to_string();

        window.open_dialog(cx, move |dialog, _, cx| {
            let auto_on = auto_on;
            dialog.title(title.clone()).child(
                v_flex()
                    .gap_3()
                    .w(px(320.))
                    .child(
                        h_flex()
                            .items_center()
                            .justify_between()
                            .child(auto_label.clone())
                            .child(
                                gpui_component::switch::Switch::new("auto-start")
                                    .checked(auto_on)
                                    .on_click({
                                        let state = state.clone();
                                        move |checked, _, cx| {
                                            state.update(cx, |s, cx| {
                                                s.set_auto_start(*checked, cx);
                                            });
                                        }
                                    }),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("export")
                                    .outline()
                                    .label(export_label.clone())
                                    .on_click({
                                        let state = state.clone();
                                        move |_, _, cx| {
                                            let _ = state.update(cx, |s, cx| s.export_json(cx));
                                        }
                                    }),
                            )
                            .child(
                                Button::new("import")
                                    .outline()
                                    .label(import_label.clone())
                                    .on_click({
                                        let state = state.clone();
                                        move |_, _, cx| {
                                            let _ = state.update(cx, |s, cx| {
                                                if s.import_json(cx).is_err() {
                                                    let msg = s.tr().import_failed.to_string();
                                                    s.show_toast(msg, cx);
                                                }
                                            });
                                        }
                                    }),
                            ),
                    )
                    .child(
                        Button::new("check-update")
                            .outline()
                            .label(check_label.clone())
                            .on_click({
                                let state = state.clone();
                                move |_, window, cx| {
                                    state.update(cx, |s, cx| {
                                        s.update.checking = true;
                                        cx.notify();
                                    });
                                    match updater::check_for_update() {
                                        Ok(Some(info)) => {
                                            let ver = info.version.clone();
                                            state.update(cx, |s, cx| {
                                                s.update.checking = false;
                                                s.update.available = Some(info);
                                                let title = s.tr().update_available_title(&ver);
                                                window.push_notification(
                                                    gpui_component::notification::Notification::info(
                                                        title,
                                                    ),
                                                    cx,
                                                );
                                                cx.notify();
                                            });
                                            updater::open_release_page(Some(&ver));
                                        }
                                        Ok(None) => {
                                            state.update(cx, |s, cx| {
                                                s.update.checking = false;
                                                let msg = s.tr().update_none.to_string();
                                                window.push_notification(
                                                    gpui_component::notification::Notification::info(
                                                        msg,
                                                    ),
                                                    cx,
                                                );
                                                cx.notify();
                                            });
                                        }
                                        Err(_) => {
                                            state.update(cx, |s, cx| {
                                                s.update.checking = false;
                                                let msg = s.tr().update_check_failed.to_string();
                                                window.push_notification(
                                                    gpui_component::notification::Notification::error(
                                                        msg,
                                                    ),
                                                    cx,
                                                );
                                                cx.notify();
                                            });
                                        }
                                    }
                                }
                            }),
                    )
                    .child(
                        Button::new("version")
                            .ghost()
                            .label(format!("v{version}"))
                            .on_click(move |_, _, _| {
                                updater::open_release_page(Some(updater::PACKAGE_VERSION));
                            }),
                    ),
            )
        });
    }

    fn render_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let tr = self.state.read(cx).tr();
        let empty = self.state.read(cx).notes.is_empty();
        let lang = tr.language_toggle;

        h_flex()
            .id("toolbar")
            .gap_2()
            .w_full()
            .px_1()
            .py_1()
            .rounded(px(TOOLBAR_RADIUS))
            .bg(toolbar_chip())
            .child(
                Button::new("new")
                    .ghost()
                    .icon(IconName::Plus)
                    .tooltip(tr.new_note)
                    .on_click(cx.listener(|this, _, window, cx| this.open_new(window, cx))),
            )
            .child(
                Button::new("delete-all")
                    .ghost()
                    .icon(IconName::Delete)
                    .text_color(color(DANGER))
                    .disabled(empty)
                    .tooltip(tr.delete_all)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.confirm_delete_all(window, cx);
                    })),
            )
            .child(
                Button::new("settings")
                    .ghost()
                    .icon(IconName::Settings)
                    .tooltip(tr.settings)
                    .on_click(cx.listener(|this, _, window, cx| this.open_settings(window, cx))),
            )
            .child(
                Button::new("lang")
                    .ghost()
                    .icon(IconName::Globe)
                    .label(lang)
                    .tooltip(tr.switch_language)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.state.update(cx, |s, cx| s.toggle_language(cx));
                    })),
            )
    }

    fn render_note_card(
        &self,
        note: &Note,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let tr = self.state.read(cx).tr();
        let note_id = note.id.clone();
        let pinned = note.pinned;
        let bg = parse_note_color(&note.color);
        let lines = note.display_lines() as f32;
        let content = if note.content.trim().is_empty() {
            if note.attachments.iter().any(|a| {
                matches!(a.kind, crate::models::AttachmentKind::Image)
            }) {
                tr.image_only.to_string()
            } else {
                String::new()
            }
        } else {
            note.content.clone()
        };
        let muted = note.content.trim().is_empty();
        let show_palette = self.palette_note_id.as_deref() == Some(note.id.as_str());

        let card = v_flex()
            .id(gpui::SharedString::from(format!("note-card-{}", note.id.clone())))
            .w_full()
            .gap_2()
            .p_3()
            .rounded(px(CARD_RADIUS))
            .bg(bg)
            .shadow(card_shadow())
            .cursor_pointer()
            .hover(|s| s.shadow(crate::ui::style::card_shadow_hover()))
            .on_click({
                let state = self.state.clone();
                let id = note_id.clone();
                move |_, _, cx| {
                    state.update(cx, |s, cx| s.copy_note(&id, cx));
                }
            })
            .context_menu({
                let state = self.state.clone();
                let id = note_id.clone();
                let pin_label = if pinned {
                    tr.unpin.to_string()
                } else {
                    tr.pin.to_string()
                };
                let copy_label = tr.copy.to_string();
                let edit_label = tr.edit.to_string();
                let delete_label = tr.delete.to_string();
                move |menu, window, _cx| {
                    menu.item(PopupMenuItem::new(copy_label.clone()).on_click({
                        let state = state.clone();
                        let id = id.clone();
                        move |_, _, cx| {
                            state.update(cx, |s, cx| s.copy_note(&id, cx));
                        }
                    }))
                    .item(PopupMenuItem::new(edit_label.clone()).on_click({
                        let state = state.clone();
                        let id = id.clone();
                        move |_, _, cx| {
                            editor_window::open_editor(state.clone(), Some(id.clone()), false, cx);
                        }
                    }))
                    .item(PopupMenuItem::new(pin_label.clone()).on_click({
                        let state = state.clone();
                        let id = id.clone();
                        move |_, _, cx| {
                            let _ = state.update(cx, |s, cx| s.toggle_pin(&id, cx));
                        }
                    }))
                    .separator()
                    .item(PopupMenuItem::new(delete_label.clone()).on_click({
                        let state = state.clone();
                        let id = id.clone();
                        move |_, _, cx| {
                            let _ = state.update(cx, |s, cx| s.delete_note(&id, cx));
                        }
                    }))
                }
            })
            .when(pinned, |this| {
                this.child(
                    h_flex()
                        .w_full()
                        .justify_start()
                        .child(
                            div()
                                .px_2()
                                .py_1()
                                .rounded_full()
                                .bg(color_alpha(0xffffff, 0.42))
                                .text_xs()
                                .text_color(color(TEXT))
                                .child(tr.pinned),
                        ),
                )
            })
            .child(
                div()
                    .w_full()
                    .text_sm()
                    .text_color(if muted {
                        color_alpha(TEXT, 0.45)
                    } else {
                        color(TEXT)
                    })
                    .line_height(px(LINE_HEIGHT))
                    .h(px(LINE_HEIGHT * lines))
                    .overflow_hidden()
                    .child(content),
            )
            .child(
                h_flex()
                    .id(gpui::SharedString::from(format!("note-actions-{}", note.id.clone())))
                    .w_full()
                    .gap_1()
                    .justify_end()
                    .opacity(0.7)
                    .hover(|s| s.opacity(1.))
                    .child(
                        Button::new(gpui::SharedString::from(format!("pin-{}", note.id.clone())))
                            .ghost()
                            .xsmall()
                            .icon(if pinned {
                                IconName::Star
                            } else {
                                IconName::StarOff
                            })
                            .tooltip(if pinned { tr.unpin } else { tr.pin })
                            .on_click({
                                let state = self.state.clone();
                                let id = note_id.clone();
                                move |_, _, cx| {
                                    let _ = state.update(cx, |s, cx| s.toggle_pin(&id, cx));
                                }
                            }),
                    )
                    .child(
                        Button::new(gpui::SharedString::from(format!("edit-{}", note.id.clone())))
                            .ghost()
                            .xsmall()
                            .icon(IconName::SquareTerminal)
                            .tooltip(tr.edit)
                            .on_click(cx.listener({
                                let id = note_id.clone();
                                move |this, _, _, cx| this.open_edit(id.clone(), cx)
                            })),
                    )
                    .child(
                        Button::new(gpui::SharedString::from(format!("color-{}", note.id.clone())))
                            .ghost()
                            .xsmall()
                            .icon(IconName::Palette)
                            .tooltip(tr.color)
                            .on_click(cx.listener({
                                let id = note_id.clone();
                                move |this, _, _, cx| {
                                    this.palette_note_id = if this.palette_note_id.as_deref()
                                        == Some(id.as_str())
                                    {
                                        None
                                    } else {
                                        Some(id.clone())
                                    };
                                    cx.notify();
                                }
                            })),
                    )
                    .child(
                        Button::new(gpui::SharedString::from(format!("copy-{}", note.id.clone())))
                            .ghost()
                            .xsmall()
                            .icon(IconName::Copy)
                            .tooltip(tr.copy)
                            .on_click({
                                let state = self.state.clone();
                                let id = note_id.clone();
                                move |_, _, cx| {
                                    state.update(cx, |s, cx| s.copy_note(&id, cx));
                                }
                            }),
                    )
                    .child(
                        Button::new(gpui::SharedString::from(format!("delete-{}", note.id.clone())))
                            .ghost()
                            .xsmall()
                            .icon(IconName::Delete)
                            .text_color(color(DANGER))
                            .tooltip(tr.delete)
                            .on_click({
                                let state = self.state.clone();
                                let id = note_id.clone();
                                move |_, _, cx| {
                                    let _ = state.update(cx, |s, cx| s.delete_note(&id, cx));
                                }
                            }),
                    ),
            );

        if show_palette {
            card.child(self.render_color_palette(&note_id, cx))
        } else {
            card
        }
    }

    fn render_color_palette(&self, note_id: &str, cx: &mut Context<Self>) -> impl IntoElement {
        let note_id = note_id.to_string();
        h_flex()
            .id(gpui::SharedString::from(format!("palette-{}", note_id.clone())))
            .w_full()
            .gap_1()
            .flex_wrap()
            .p_1()
            .rounded(px(8.))
            .bg(color_alpha(0xffffff, 0.55))
            .children(NOTE_COLORS.iter().map(|c| {
                let color_str = (*c).to_string();
                let id = note_id.clone();
                div()
                    .id(gpui::SharedString::from(format!("swatch-{}", color_str.clone())))
                    .size_5()
                    .rounded_full()
                    .border_1()
                    .border_color(color_alpha(0x1f2328, 0.12))
                    .bg(parse_note_color(&color_str))
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, {
                        let state = self.state.clone();
                        move |_, _, cx| {
                            let _ = state.update(cx, |s, cx| {
                                s.patch_note(&id, cx, |n| n.color = color_str.clone())
                            });
                        }
                    })
            }))
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                this.palette_note_id = None;
                cx.notify();
            }))
    }

    fn render_empty(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let tr = self.state.read(cx).tr();
        v_flex()
            .id("empty")
            .flex_1()
            .w_full()
            .items_center()
            .justify_center()
            .gap_3()
            .child(q_mark(52.))
            .child(
                div()
                    .text_base()
                    .font_semibold()
                    .text_color(color(TEXT))
                    .child(tr.empty_title),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(color_alpha(TEXT, 0.65))
                    .child(tr.no_notes_body),
            )
            .child(
                Button::new("empty-new")
                    .primary()
                    .label(tr.empty_action)
                    .on_click(cx.listener(|this, _, window, cx| this.open_new(window, cx))),
            )
    }
}

impl Render for MainWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tr = self.state.read(cx).tr();
        let notes = self.state.read(cx).notes.clone();
        let count = notes.len();
        let always_on_top = self.state.read(cx).settings.always_on_top;
        let toast = self.state.read(cx).toast.clone();

        // Position main window on first paint using saved size when available.
        let _ = window;

        v_flex()
            .id("app-shell")
            .size_full()
            .bg(color(APP_BG))
            .rounded(px(WINDOW_RADIUS))
            .pt_2()
            .px_2()
            .pb_7()
            .text_color(color(TEXT))
            .child(
                TitleBar::new()
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .justify_between()
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_2()
                                    .child(q_mark(22.))
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_semibold()
                                            .child(tr.app_title),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .gap_1()
                                    .rounded_full()
                                    .px_1()
                                    .py_1()
                                    .bg(chrome_chip())
                                    .child(
                                        Button::new("topmost")
                                            .ghost()
                                            .xsmall()
                                            .icon(IconName::Star)
                                            .tooltip(if always_on_top {
                                                tr.always_off
                                            } else {
                                                tr.always_on
                                            })
                                            .on_click({
                                                let state = self.state.clone();
                                                move |_, _, cx| {
                                                    state.update(cx, |s, cx| {
                                                        let next = !s.settings.always_on_top;
                                                        s.set_always_on_top(next, cx);
                                                    });
                                                }
                                            }),
                                    )
                                    .child(
                                        Button::new("min")
                                            .ghost()
                                            .xsmall()
                                            .icon(IconName::WindowMinimize)
                                            .tooltip(tr.minimize)
                                            .on_click(|_, window, _| window.minimize_window()),
                                    )
                                    .child(
                                        Button::new("close")
                                            .ghost()
                                            .xsmall()
                                            .icon(IconName::WindowClose)
                                            .tooltip(tr.close_panel)
                                            .on_click(|_, window, _| {
                                                // Hide behavior: remove focus / minimize as close-to-tray
                                                window.minimize_window();
                                            }),
                                    ),
                            ),
                    ),
            )
            .child(self.render_toolbar(cx))
            .child(
                div()
                    .id("note-list")
                    .flex_1()
                    .w_full()
                    .mt_2()
                    .overflow_y_scrollbar()
                    .when(notes.is_empty(), |this| this.child(self.render_empty(cx)))
                    .when(!notes.is_empty(), |this| {
                        let mut cards = Vec::new();
                        for note in &notes {
                            cards.push(self.render_note_card(note, cx).into_any_element());
                        }
                        this.child(v_flex().w_full().gap_2().children(cards))
                    }),
            )
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_center()
                    .pt_1()
                    .child(
                        div()
                            .text_xs()
                            .text_color(color_alpha(TEXT, 0.7))
                            .child(tr.status_summary(count)),
                    ),
            )
            .child(
                div()
                    .absolute()
                    .right_3()
                    .bottom_3()
                    .child(
                        div()
                            .id("dock-fab")
                            .size_8()
                            .rounded_full()
                            .bg(color_alpha(0xffffff, 0.55))
                            .shadow(card_shadow())
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .hover(|s| s.bg(color_alpha(0xffffff, 0.75)))
                            .child(q_mark(22.))
                            .on_mouse_down(MouseButton::Left, {
                                let state = self.state.clone();
                                move |_, _, cx| {
                                    dock_window::collapse_to_dock(state.clone(), cx);
                                }
                            }),
                    ),
            )
            .when_some(toast, |this, toast| {
                this.child(
                    div()
                        .absolute()
                        .bottom_10()
                        .left_0()
                        .right_0()
                        .flex()
                        .justify_center()
                        .child(
                            div()
                                .px_3()
                                .py_1p5()
                                .rounded_full()
                                .bg(color_alpha(0x1d1d1f, 0.88))
                                .text_color(rgb(0xffffff))
                                .text_xs()
                                .child(toast.text),
                        ),
                )
            })
    }
}

pub fn q_mark(size_px: f32) -> impl IntoElement {
    div()
        .size(px(size_px))
        .rounded_full()
        .bg(color(APP_BG))
        .border_1()
        .border_color(color_alpha(0x18212f, 0.12))
        .flex()
        .items_center()
        .justify_center()
        .text_color(color(0x18212f))
        .font_bold()
        .text_sm()
        .child("Q")
}

pub fn show_main_from_tray(state: Entity<AppState>, cx: &mut App) {
    if state.read(cx).settings.docked {
        restore_from_dock(state, cx);
        return;
    }
    if let Some(handle) = state.read(cx).main_window.clone() {
        let _ = handle.update(cx, |_, window, _| {
            window.activate_window();
        });
        return;
    }
    open_main_window(state, cx);
}

pub fn restore_from_dock(state: Entity<AppState>, cx: &mut App) {
    state.update(cx, |s, _| {
        s.settings.docked = false;
        let _ = s.persist_settings();
        crate::tray::update_labels(s);
    });
    let dock = state.update(cx, |s, _| s.dock_window.take());
    if let Some(dock) = dock {
        let _ = dock.update(cx, |_, window, _| {
            window.remove_window();
        });
    }
    open_main_window(state, cx);
}

pub fn open_main_window(state: Entity<AppState>, cx: &mut App) {
    let settings = state.read(cx).settings.clone();
    let (w, h) = settings
        .window
        .as_ref()
        .map(|w| (w.width, w.height))
        .unwrap_or((DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT));
    let bounds = gpui::Bounds::centered(None, size(px(w), px(h)), cx);
    let always_on_top = settings.always_on_top;

    let handle = cx
        .open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some("Q Note".into()),
                    appears_transparent: true,
                    ..Default::default()
                }),
                window_min_size: Some(size(px(DEFAULT_WINDOW_WIDTH), px(DEFAULT_WINDOW_HEIGHT))),
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
                move |window, cx| {
                    let view = cx.new(|cx| MainWindow::new(state.clone(), window, cx));
                    cx.new(|cx| gpui_component::Root::new(view, window, cx))
                }
            },
        )
        .expect("open main");

    state.update(cx, |s, _| s.main_window = Some(handle));
}

// Fluent helpers used above
trait WhenExt: Sized {
    fn when(self, cond: bool, f: impl FnOnce(Self) -> Self) -> Self {
        if cond { f(self) } else { self }
    }
    fn when_some<T>(self, opt: Option<T>, f: impl FnOnce(Self, T) -> Self) -> Self {
        match opt {
            Some(v) => f(self, v),
            None => self,
        }
    }
}
impl<T> WhenExt for T {}
