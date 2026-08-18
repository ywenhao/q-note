//! Main note board window — layout/colors match the original Vue shell.

use std::time::Duration;

use gpui::{
    Animation, AnimationExt as _, App, AppContext, Context, Entity, InteractiveElement,
    IntoElement, MouseButton, ParentElement, Render, ScrollHandle, Stateful,
    StatefulInteractiveElement, Styled, Window, WindowBounds, WindowControlArea, WindowDecorations,
    WindowOptions, deferred, div, px, relative, rgb, size, svg,
};
use gpui_component::{
    Disableable as _, Icon, IconName, IconNamed as _, Sizable as _, StyledExt as _, WindowExt as _,
    animation::cubic_bezier,
    button::{Button, ButtonVariants as _},
    h_flex,
    menu::{ContextMenuExt, PopupMenuItem},
    scroll::ScrollableElement as _,
    tooltip::Tooltip,
    v_flex,
};

use crate::app_state::AppState;
use crate::models::{
    DEFAULT_WINDOW_HEIGHT, DEFAULT_WINDOW_WIDTH, MAX_WINDOW_HEIGHT, MAX_WINDOW_WIDTH, NOTE_COLORS,
    Note, WindowState,
};
use crate::ui::dock_window;
use crate::ui::editor_window;
use crate::ui::modal::{self, AppModal};
use crate::ui::style::{
    ACCENT, APP_BG, CARD_RADIUS, DANGER, LINE_HEIGHT, TEXT, TOOLBAR_RADIUS, WINDOW_RADIUS,
    card_shadow, chrome_chip, color, color_alpha, parse_note_color, toolbar_chip,
};
use crate::updater;

const NOTE_SCROLLBAR_GUTTER: f32 = 16.;
const MAIN_CONTENT_MAX_WIDTH: f32 = 980.;
const MAIN_TITLE_BAR_HEIGHT: f32 = 34.;
const TOOLBAR_PAD: f32 = 4.;
const PIN_BADGE_SIZE: f32 = 14.;
const PIN_ICON_SIZE: f32 = 10.;
const WINDOW_CONTROL_SIZE: f32 = 18.;
const WINDOW_CONTROL_PAD: f32 = 2.;
const COLOR_SWATCH_SIZE: f32 = 20.;
const COLOR_POPOVER_GAP: f32 = 5.;
const COLOR_POPOVER_PAD: f32 = 6.;
const COLOR_POPOVER_COLS: f32 = 4.;

pub struct MainWindow {
    state: Entity<AppState>,
    actions_note_id: Option<String>,
    note_scroll_handle: ScrollHandle,
    palette_note_id: Option<String>,
    window_state_revision: u64,
    modal: Option<AppModal>,
    modal_closing: bool,
    modal_generation: u64,
}

impl MainWindow {
    pub fn new(state: Entity<AppState>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        window.on_window_should_close(cx, {
            move |window, cx| {
                // Close hides to tray instead of quitting (parity with Tauri).
                let _ = cx;
                window.minimize_window();
                false
            }
        });

        // Restore pending editor draft after update if present.
        if state.read(cx).editor_draft_recovery.is_some() {
            let state = state.clone();
            cx.defer(move |cx| {
                editor_window::open_editor(state.clone(), None, true, cx);
            });
        }

        cx.observe_window_bounds(window, |this, window, cx| {
            this.window_bounds_changed(window, cx);
        })
        .detach();
        cx.observe(&state, |_, _, cx| cx.notify()).detach();
        Self {
            state,
            actions_note_id: None,
            note_scroll_handle: ScrollHandle::default(),
            palette_note_id: None,
            window_state_revision: 0,
            modal: None,
            modal_closing: false,
            modal_generation: 0,
        }
    }

    fn window_bounds_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.state.read(cx).settings.docked {
            return;
        }
        let bounds = window.bounds();
        let width = f32::from(bounds.size.width);
        let height = f32::from(bounds.size.height);
        if width > MAX_WINDOW_WIDTH || height > MAX_WINDOW_HEIGHT {
            window.resize(size(
                px(width.min(MAX_WINDOW_WIDTH)),
                px(height.min(MAX_WINDOW_HEIGHT)),
            ));
            return;
        }
        let snapshot = capture_window_state(window);
        self.window_state_revision = self.window_state_revision.wrapping_add(1);
        let revision = self.window_state_revision;
        let state = self.state.clone();
        state.update(cx, |state, _| {
            state.settings.window = Some(snapshot);
        });
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(300))
                .await;
            let is_latest = this
                .update(cx, |this, _| this.window_state_revision == revision)
                .unwrap_or(false);
            if is_latest {
                let _ = state.update(cx, |state, _| {
                    if !state.settings.docked {
                        let _ = state.persist_settings();
                    }
                });
            }
        })
        .detach();
    }

    fn open_new(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        editor_window::open_editor(self.state.clone(), None, false, cx);
    }

    fn confirm_delete_all(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.open_modal(AppModal::ConfirmDeleteAll, cx);
    }

    fn open_settings(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.open_modal(AppModal::Settings, cx);
    }

    fn open_modal(&mut self, kind: AppModal, cx: &mut Context<Self>) {
        if self.modal == Some(kind) && !self.modal_closing {
            return;
        }
        self.modal = Some(kind);
        self.modal_closing = false;
        self.modal_generation = self.modal_generation.wrapping_add(1);
        cx.notify();
    }

    fn request_close_modal(&mut self, cx: &mut Context<Self>) {
        if self.modal.is_none() || self.modal_closing {
            return;
        }
        self.modal_closing = true;
        let generation = self.modal_generation;
        cx.notify();
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(modal::PANEL_MS))
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.modal_generation == generation && this.modal_closing {
                    this.modal = None;
                    this.modal_closing = false;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn run_update_check(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.modal_closing || self.state.read(cx).update.checking {
            return;
        }
        let state = self.state.clone();
        state.update(cx, |s, cx| {
            s.update.checking = true;
            s.update.error = None;
            cx.notify();
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async { updater::check_for_update() })
                .await;
            let _ = this.update_in(cx, |_, window, cx| match result {
                Ok(Some(info)) => {
                    let ver = info.version.clone();
                    state.update(cx, |s, cx| {
                        s.update.checking = false;
                        s.update.available = Some(info);
                        s.update.error = None;
                        let title = s.tr().update_available_title(&ver);
                        window.push_notification(
                            gpui_component::notification::Notification::info(title),
                            cx,
                        );
                        cx.notify();
                    });
                    let snapshot = capture_window_state(window);
                    let prepared = state.update(cx, |s, _| {
                        s.settings.window = Some(snapshot);
                        s.prepare_for_update()
                    });
                    if let Err(error) = prepared {
                        state.update(cx, |s, cx| {
                            s.update.error = Some(error.to_string());
                            let msg = s.tr().update_prepare_failed.to_string();
                            window.push_notification(
                                gpui_component::notification::Notification::error(msg),
                                cx,
                            );
                            cx.notify();
                        });
                        return;
                    }
                    updater::open_release_page(Some(&ver));
                }
                Ok(None) => {
                    state.update(cx, |s, cx| {
                        s.update.checking = false;
                        s.update.available = None;
                        s.update.error = None;
                        let msg = s.tr().update_none.to_string();
                        window.push_notification(
                            gpui_component::notification::Notification::info(msg),
                            cx,
                        );
                        cx.notify();
                    });
                }
                Err(error) => {
                    state.update(cx, |s, cx| {
                        s.update.checking = false;
                        s.update.error = Some(error.to_string());
                        let msg = s.tr().update_check_failed.to_string();
                        window.push_notification(
                            gpui_component::notification::Notification::error(msg),
                            cx,
                        );
                        cx.notify();
                    });
                }
            });
        })
        .detach();
    }

    fn render_modal(&self, kind: AppModal, cx: &mut Context<Self>) -> impl IntoElement {
        let closing = self.modal_closing;
        let generation = self.modal_generation;
        let panel = match kind {
            AppModal::Settings => self.render_settings_dialog(cx).into_any_element(),
            AppModal::ConfirmDeleteAll => self.render_confirm_dialog(cx).into_any_element(),
        };
        modal::modal_layer()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| this.request_close_modal(cx)),
            )
            .child(modal::animate_overlay(
                modal::modal_backdrop().on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| this.request_close_modal(cx)),
                ),
                closing,
                generation,
            ))
            .child(panel)
    }

    fn render_settings_dialog(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let tr = self.state.read(cx).tr();
        let auto_on = self.state.read(cx).settings.auto_start;
        let checking = self.state.read(cx).update.checking;
        let has_update = self.state.read(cx).update.available.is_some();
        let version = format!("v{}", updater::PACKAGE_VERSION);
        let closing = self.modal_closing;
        let generation = self.modal_generation;
        let state = self.state.clone();

        modal::animate_panel(
            modal::settings_shell()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(
                    h_flex()
                        .w_full()
                        .items_center()
                        .justify_between()
                        .gap(px(8.))
                        .child(
                            div()
                                .text_sm()
                                .font_semibold()
                                .text_color(color(0x1d1d1f))
                                .child(tr.settings_title),
                        )
                        .child(modal::settings_close_button().on_click(cx.listener(
                            |this, _, _, cx| {
                                this.request_close_modal(cx);
                            },
                        ))),
                )
                .child(
                    modal::settings_group()
                        .child(
                            modal::settings_row("settings-autostart")
                                .on_click(cx.listener({
                                    let state = state.clone();
                                    move |this, _, _, cx| {
                                        if this.modal_closing {
                                            return;
                                        }
                                        let next = !state.read(cx).settings.auto_start;
                                        state.update(cx, |s, cx| s.set_auto_start(next, cx));
                                    }
                                }))
                                .child(modal::settings_label(
                                    crate::POWER_ICON_PATH,
                                    tr.startup_setting,
                                ))
                                .child(modal::launch_switch(auto_on)),
                        )
                        .child(
                            h_flex()
                                .w_full()
                                .gap(px(1.))
                                .child(
                                    modal::settings_action("settings-import")
                                        .on_click(cx.listener({
                                            let state = state.clone();
                                            move |this, _, _, cx| {
                                                if this.modal_closing {
                                                    return;
                                                }
                                                state.update(cx, |s, cx| {
                                                    if s.import_json(cx).is_err() {
                                                        let msg = s.tr().import_failed.to_string();
                                                        s.show_toast(msg, cx);
                                                    }
                                                });
                                            }
                                        }))
                                        .child(modal::settings_icon(crate::UPLOAD_ICON_PATH))
                                        .child(tr.import),
                                )
                                .child(
                                    modal::settings_action("settings-export")
                                        .on_click(cx.listener({
                                            let state = state.clone();
                                            move |this, _, _, cx| {
                                                if this.modal_closing {
                                                    return;
                                                }
                                                let _ = state.update(cx, |s, cx| s.export_json(cx));
                                            }
                                        }))
                                        .child(modal::settings_icon(crate::DOWNLOAD_ICON_PATH))
                                        .child(tr.export),
                                ),
                        )
                        .child(
                            modal::settings_row("settings-check-update")
                                .justify_center()
                                .when(has_update, |this| this.justify_between())
                                .when(checking, |this| this.opacity(0.72))
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.run_update_check(window, cx);
                                }))
                                .child(
                                    h_flex()
                                        .items_center()
                                        .gap(px(6.))
                                        .child(modal::check_update_icon(checking))
                                        .when(has_update, |this| this.child(modal::update_dot()))
                                        .child(tr.check_update),
                                )
                                .when(has_update, |this| {
                                    this.child(
                                        div()
                                            .text_xs()
                                            .text_color(color_alpha(0x3c3c43, 0.68))
                                            .child(tr.update_available),
                                    )
                                }),
                        ),
                )
                .child(
                    h_flex()
                        .w_full()
                        .min_h(px(16.))
                        .mt(px(1.))
                        .items_center()
                        .justify_center()
                        .child(modal::version_button().child(version).on_click(|_, _, _| {
                            updater::open_release_page(Some(updater::PACKAGE_VERSION));
                        })),
                ),
            closing,
            generation,
        )
    }

    fn render_confirm_dialog(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let tr = self.state.read(cx).tr();
        let closing = self.modal_closing;
        let generation = self.modal_generation;
        let state = self.state.clone();

        modal::animate_panel(
            modal::confirm_shell()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(
                    div()
                        .pb(px(8.))
                        .text_lg()
                        .font_semibold()
                        .text_color(color(0x1d1d1f))
                        .child(tr.confirm_delete_all),
                )
                .child(
                    div()
                        .pb(px(18.))
                        .text_sm()
                        .text_color(color_alpha(0x3c3c43, 0.78))
                        .child(tr.delete_all_body),
                )
                .child(
                    h_flex()
                        .w_full()
                        .justify_end()
                        .gap(px(8.))
                        .child(
                            modal::text_button("confirm-cancel")
                                .child(tr.cancel)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.request_close_modal(cx);
                                })),
                        )
                        .child(
                            modal::danger_button("confirm-delete")
                                .child(tr.delete)
                                .on_click(cx.listener({
                                    let state = state.clone();
                                    move |this, _, _, cx| {
                                        if this.modal_closing {
                                            return;
                                        }
                                        let _ = state.update(cx, |s, cx| s.delete_all_notes(cx));
                                        this.request_close_modal(cx);
                                    }
                                })),
                        ),
                ),
            closing,
            generation,
        )
    }

    fn render_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let tr = self.state.read(cx).tr();
        let empty = self.state.read(cx).notes.is_empty();
        let lang = tr.language_toggle;

        h_flex()
            .id("toolbar")
            .gap_2()
            .w_full()
            .max_w(px(MAIN_CONTENT_MAX_WIDTH))
            .mx_auto()
            .px(px(TOOLBAR_PAD))
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

    fn render_note_card(&self, note: &Note, cx: &mut Context<Self>) -> impl IntoElement {
        let tr = self.state.read(cx).tr();
        let note_id = note.id.clone();
        let pinned = note.pinned;
        let bg = parse_note_color(&note.color);
        let lines = note.display_lines() as f32;
        let content = if note.content.trim().is_empty() {
            if note
                .attachments
                .iter()
                .any(|a| matches!(a.kind, crate::models::AttachmentKind::Image))
            {
                tr.image_only.to_string()
            } else {
                String::new()
            }
        } else {
            note.content.clone()
        };
        let muted = note.content.trim().is_empty();
        let show_palette = self.palette_note_id.as_deref() == Some(note.id.as_str());
        let actions_open = self.actions_note_id.as_deref() == Some(note.id.as_str());
        let card_group = gpui::SharedString::from(format!("note-card-group-{}", note.id));
        let action_icon = color_alpha(0x3c3c43, 0.72);
        let action_hover = color_alpha(0x3c3c43, 0.10);

        let action_dock = h_flex()
            .id(gpui::SharedString::from(format!(
                "note-action-dock-{}",
                note.id
            )))
            .absolute()
            .top_1_2()
            .right(px(WINDOW_CONTROL_PAD - NOTE_SCROLLBAR_GUTTER))
            .mt(px(-15.))
            .h(px(30.))
            .w(if actions_open {
                px(148.)
            } else {
                px(WINDOW_CONTROL_SIZE)
            })
            .items_center()
            .justify_end()
            .gap(px(2.))
            .invisible()
            .group_hover(card_group.clone(), |style| style.visible())
            .when(actions_open, |this| this.visible())
            .on_click(|_, _, cx| cx.stop_propagation())
            .on_hover(cx.listener({
                let id = note_id.clone();
                move |this, hovered: &bool, _, cx| {
                    if !*hovered
                        && this.palette_note_id.as_deref() != Some(id.as_str())
                        && this.actions_note_id.as_deref() == Some(id.as_str())
                    {
                        this.actions_note_id = None;
                        cx.notify();
                    }
                }
            }))
            .when(actions_open, |this| {
                this.child(
                    h_flex()
                        .items_center()
                        .gap(px(5.))
                        .p_1()
                        .rounded(px(12.))
                        .border_1()
                        .border_color(color_alpha(0x3c3c43, 0.08))
                        .bg(color_alpha(0xffffff, 0.86))
                        .shadow_lg()
                        .child(
                            centered_icon_button(
                                gpui::SharedString::from(format!("pin-{}", note.id)),
                                if pinned {
                                    crate::PIN_OFF_ICON_PATH
                                } else {
                                    crate::PIN_ICON_PATH
                                },
                                if pinned { tr.unpin } else { tr.pin },
                                action_icon,
                                action_icon,
                                action_hover,
                            )
                            .on_click(cx.listener({
                                let state = self.state.clone();
                                let id = note_id.clone();
                                move |this, _, _, cx| {
                                    let _ = state.update(cx, |s, cx| s.toggle_pin(&id, cx));
                                    this.actions_note_id = None;
                                    this.palette_note_id = None;
                                    cx.notify();
                                }
                            })),
                        )
                        .child(
                            centered_icon_button(
                                gpui::SharedString::from(format!("edit-{}", note.id)),
                                IconName::SquareTerminal.path(),
                                tr.edit,
                                action_icon,
                                action_icon,
                                action_hover,
                            )
                            .on_click(cx.listener({
                                let state = self.state.clone();
                                let id = note_id.clone();
                                move |this, _, _, cx| {
                                    editor_window::open_editor(
                                        state.clone(),
                                        Some(id.clone()),
                                        false,
                                        cx,
                                    );
                                    this.actions_note_id = None;
                                    this.palette_note_id = None;
                                    cx.notify();
                                }
                            })),
                        )
                        .child(
                            div()
                                .id(gpui::SharedString::from(format!(
                                    "color-popover-wrap-{}",
                                    note.id
                                )))
                                .relative()
                                .flex_none()
                                .child(
                                    centered_icon_button(
                                        gpui::SharedString::from(format!("color-{}", note.id)),
                                        IconName::Palette.path(),
                                        tr.color,
                                        action_icon,
                                        action_icon,
                                        action_hover,
                                    )
                                    .on_click(cx.listener({
                                        let id = note_id.clone();
                                        move |this, _, _, cx| {
                                            this.palette_note_id =
                                                if this.palette_note_id.as_deref()
                                                    == Some(id.as_str())
                                                {
                                                    None
                                                } else {
                                                    Some(id.clone())
                                                };
                                            this.actions_note_id = Some(id.clone());
                                            cx.notify();
                                        }
                                    })),
                                )
                                .when(show_palette, |this| {
                                    this.child(self.render_color_palette(&note_id, &note.color, cx))
                                }),
                        )
                        .child(
                            centered_icon_button(
                                gpui::SharedString::from(format!("copy-{}", note.id)),
                                IconName::Copy.path(),
                                tr.copy,
                                action_icon,
                                action_icon,
                                action_hover,
                            )
                            .on_click(cx.listener({
                                let state = self.state.clone();
                                let id = note_id.clone();
                                move |this, _, _, cx| {
                                    state.update(cx, |s, cx| s.copy_note(&id, cx));
                                    this.actions_note_id = None;
                                    this.palette_note_id = None;
                                    cx.notify();
                                }
                            })),
                        )
                        .child(
                            centered_icon_button(
                                gpui::SharedString::from(format!("delete-{}", note.id)),
                                IconName::Delete.path(),
                                tr.delete,
                                color(DANGER),
                                color(0xffffff),
                                color(DANGER),
                            )
                            .on_click(cx.listener({
                                let state = self.state.clone();
                                let id = note_id.clone();
                                move |this, _, _, cx| {
                                    let _ = state.update(cx, |s, cx| s.delete_note(&id, cx));
                                    this.actions_note_id = None;
                                    this.palette_note_id = None;
                                    cx.notify();
                                }
                            })),
                        )
                        .with_animation(
                            gpui::SharedString::from(format!("note-actions-open-{}", note.id)),
                            Animation::new(Duration::from_millis(140))
                                .with_easing(cubic_bezier(0.25, 0.1, 0.25, 1.)),
                            |this, delta| {
                                this.relative()
                                    .left(px(6.) - delta * px(6.))
                                    .opacity(0.94 * delta)
                            },
                        ),
                )
            })
            .child(
                centered_icon_button(
                    gpui::SharedString::from(format!("more-actions-{}", note.id)),
                    IconName::ChevronLeft.path(),
                    tr.more_actions,
                    action_icon,
                    action_icon,
                    action_hover,
                )
                .on_hover(cx.listener({
                    let id = note_id.clone();
                    move |this, hovered: &bool, _, cx| {
                        if *hovered && this.actions_note_id.as_deref() != Some(id.as_str()) {
                            this.actions_note_id = Some(id.clone());
                            cx.notify();
                        }
                    }
                })),
            );

        let card = v_flex()
            .id(gpui::SharedString::from(format!(
                "note-card-{}",
                note.id.clone()
            )))
            .group(card_group)
            .relative()
            .w_full()
            .min_w_full()
            .flex_none()
            .pt(px(10.))
            .pr(px(28.))
            .pb(px(9.))
            .pl(px(10.))
            .rounded(px(CARD_RADIUS))
            .border_1()
            .border_color(if pinned {
                color_alpha(ACCENT, 0.28)
            } else {
                color_alpha(0xffffff, 0.50)
            })
            .bg(bg)
            .shadow(card_shadow())
            .cursor_pointer()
            .hover(|s| {
                s.border_color(color_alpha(ACCENT, 0.36))
                    .shadow(crate::ui::style::card_shadow_hover())
            })
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
                move |menu, _window, _cx| {
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
                    div()
                        .absolute()
                        .top(px(2.))
                        .left(px(TOOLBAR_PAD))
                        .size(px(PIN_BADGE_SIZE))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded_full()
                        .bg(color_alpha(0xffffff, 0.42))
                        .child(
                            pin_icon(false)
                                .with_size(px(8.))
                                .text_color(color_alpha(ACCENT, 0.76)),
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
            .child(action_dock);

        card
    }

    fn render_color_palette(
        &self,
        note_id: &str,
        selected: &str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let note_id = note_id.to_string();
        let selected = selected.to_string();
        let width = COLOR_POPOVER_COLS * COLOR_SWATCH_SIZE
            + (COLOR_POPOVER_COLS - 1.) * COLOR_POPOVER_GAP
            + COLOR_POPOVER_PAD * 2.;
        let left = (WINDOW_CONTROL_SIZE - width) / 2.;

        deferred(
            div()
                .id(gpui::SharedString::from(format!(
                    "palette-{}",
                    note_id.clone()
                )))
                .absolute()
                .top(px(36.))
                .left(px(left))
                .w(px(width))
                .p(px(COLOR_POPOVER_PAD))
                .rounded(px(8.))
                .border_1()
                .border_color(color_alpha(0x1d2735, 0.14))
                .bg(color_alpha(0xffffff, 0.92))
                .shadow(vec![gpui::BoxShadow {
                    color: color_alpha(0x1f2328, 0.16).into(),
                    offset: gpui::point(px(0.), px(16.)),
                    blur_radius: px(34.),
                    spread_radius: px(0.),
                }])
                .occlude()
                .child(
                    h_flex()
                        .w_full()
                        .flex_wrap()
                        .gap(px(COLOR_POPOVER_GAP))
                        .children(NOTE_COLORS.iter().map(|c| {
                            let color_str = (*c).to_string();
                            let is_selected = color_str == selected;
                            let id = note_id.clone();
                            div()
                                .id(gpui::SharedString::from(format!(
                                    "swatch-{}",
                                    color_str.clone()
                                )))
                                .size(px(COLOR_SWATCH_SIZE))
                                .flex()
                                .flex_none()
                                .items_center()
                                .justify_center()
                                .rounded_full()
                                .border_1()
                                .border_color(if is_selected {
                                    color_alpha(0x1d2735, 0.45)
                                } else {
                                    color_alpha(0x1d2735, 0.16)
                                })
                                .bg(parse_note_color(&color_str))
                                .cursor_pointer()
                                .hover(|style| style.border_color(color_alpha(0x1d2735, 0.45)))
                                .when(is_selected, |this| {
                                    this.child(
                                        Icon::new(IconName::Check)
                                            .with_size(px(11.))
                                            .text_color(color(0x1d2735)),
                                    )
                                })
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener({
                                        let state = self.state.clone();
                                        move |this, _, _, cx| {
                                            let _ = state.update(cx, |s, cx| {
                                                s.patch_note(&id, cx, |n| {
                                                    n.color = color_str.clone()
                                                })
                                            });
                                            this.palette_note_id = None;
                                            cx.notify();
                                        }
                                    }),
                                )
                        })),
                )
                .on_click(|_, _, cx| cx.stop_propagation())
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                    this.palette_note_id = None;
                    cx.notify();
                })),
        )
        .with_priority(1)
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
        let dialog_layer = gpui_component::Root::render_dialog_layer(window, cx);
        let notification_layer = gpui_component::Root::render_notification_layer(window, cx);
        let tr = self.state.read(cx).tr();
        let notes = self.state.read(cx).notes.clone();
        let count = notes.len();
        let always_on_top = self.state.read(cx).settings.always_on_top;
        let toast = self.state.read(cx).toast.clone();
        let note_scroll_handle = self.note_scroll_handle.clone();
        let window_control_foreground = color_alpha(0x3c3c43, 0.72);

        v_flex()
            .id("app-shell")
            .size_full()
            .bg(color(APP_BG))
            .rounded(px(WINDOW_RADIUS))
            .pt_2()
            .px_2p5()
            .pb_7()
            .text_color(color(TEXT))
            .child(
                h_flex()
                    .id("main-title-bar")
                    .h(px(MAIN_TITLE_BAR_HEIGHT))
                    .w_full()
                    .max_w(px(MAIN_CONTENT_MAX_WIDTH))
                    .mx_auto()
                    .items_center()
                    .justify_between()
                    .child(
                        h_flex()
                            .id("main-title-drag-area")
                            .h_full()
                            .min_w_0()
                            .flex_1()
                            .items_center()
                            .gap_2()
                            .window_control_area(WindowControlArea::Drag)
                            .on_mouse_down(MouseButton::Left, |_, window, _| {
                                window.start_window_move();
                            })
                            .child(q_mark(22.))
                            .child(div().text_sm().font_semibold().child(tr.app_title)),
                    )
                    .child(
                        h_flex()
                            .flex_shrink_0()
                            .gap(px(2.))
                            .rounded_full()
                            .p(px(WINDOW_CONTROL_PAD))
                            .border_1()
                            .border_color(color_alpha(0xffffff, 0.42))
                            .bg(chrome_chip())
                            .shadow(vec![gpui::BoxShadow {
                                color: color_alpha(0x1f2328, 0.06).into(),
                                offset: gpui::point(px(0.), px(4.)),
                                blur_radius: px(12.),
                                spread_radius: px(0.),
                            }])
                            .child(
                                centered_icon_button(
                                    "topmost",
                                    if always_on_top {
                                        crate::PIN_OFF_ICON_PATH
                                    } else {
                                        crate::PIN_ICON_PATH
                                    },
                                    if always_on_top {
                                        tr.always_off
                                    } else {
                                        tr.always_on
                                    },
                                    if always_on_top {
                                        color(ACCENT)
                                    } else {
                                        window_control_foreground
                                    },
                                    color(0xffffff),
                                    if always_on_top {
                                        color_alpha(0xffffff, 0.)
                                    } else {
                                        color(ACCENT)
                                    },
                                )
                                .on_click({
                                    let state = self.state.clone();
                                    move |_, window, cx| {
                                        let next = !state.read(cx).settings.always_on_top;
                                        crate::ui::apply_window_topmost(window, next);
                                        state.update(cx, |s, cx| {
                                            s.set_always_on_top(next, cx);
                                        });
                                    }
                                }),
                            )
                            .child(
                                centered_icon_button(
                                    "min",
                                    IconName::Minus.path(),
                                    tr.minimize,
                                    window_control_foreground,
                                    color(0xffffff),
                                    color(0xffcc00),
                                )
                                .on_click(|_, window, _| window.minimize_window()),
                            )
                            .child(
                                centered_icon_button(
                                    "close",
                                    IconName::Close.path(),
                                    tr.close_panel,
                                    window_control_foreground,
                                    color(0xffffff),
                                    color(DANGER),
                                )
                                .on_click(|_, window, _| window.minimize_window()),
                            ),
                    ),
            )
            .child(self.render_toolbar(cx))
            .child(
                div()
                    .id("note-list-frame")
                    .relative()
                    .flex_1()
                    .w_full()
                    .max_w(px(MAIN_CONTENT_MAX_WIDTH))
                    .mx_auto()
                    .min_w_0()
                    .min_h_0()
                    .mt_2()
                    .child(
                        div()
                            .id("note-list")
                            .absolute()
                            .top_0()
                            .bottom_0()
                            .left_0()
                            .right(px(-NOTE_SCROLLBAR_GUTTER))
                            .child(
                                div()
                                    .id("note-list-scroll-area")
                                    .size_full()
                                    .track_scroll(&note_scroll_handle)
                                    .overflow_y_scroll()
                                    .when(notes.is_empty(), |this| {
                                        this.child(self.render_empty(cx))
                                    })
                                    .when(!notes.is_empty(), |this| {
                                        let mut cards = Vec::new();
                                        for note in &notes {
                                            cards.push(
                                                self.render_note_card(note, cx).into_any_element(),
                                            );
                                        }
                                        this.child(
                                            v_flex()
                                                .w_full()
                                                .min_w_full()
                                                .gap(px(10.))
                                                .pr(px(NOTE_SCROLLBAR_GUTTER))
                                                .pb_2()
                                                .children(cards),
                                        )
                                    }),
                            )
                            .vertical_scrollbar(&note_scroll_handle),
                    ),
            )
            .child(
                h_flex()
                    .w_full()
                    .max_w(px(MAIN_CONTENT_MAX_WIDTH))
                    .mx_auto()
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
                div().absolute().right_3().bottom_3().child(
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
            .when_some(self.modal, |this, kind| {
                this.child(self.render_modal(kind, cx))
            })
            .children(dialog_layer)
            .children(notification_layer)
    }
}

fn pin_icon(off: bool) -> Icon {
    Icon::empty().path(if off {
        crate::PIN_OFF_ICON_PATH
    } else {
        crate::PIN_ICON_PATH
    })
}

fn centered_icon_button(
    id: impl Into<gpui::SharedString>,
    path: impl Into<gpui::SharedString>,
    tooltip: impl Into<gpui::SharedString>,
    idle_fg: gpui::Rgba,
    hover_fg: gpui::Rgba,
    hover_bg: gpui::Rgba,
) -> Stateful<gpui::Div> {
    let id = id.into();
    let group = id.clone();
    let tooltip = tooltip.into();
    h_flex()
        .id(id)
        .group(group.clone())
        .size(px(WINDOW_CONTROL_SIZE))
        .flex_none()
        .justify_center()
        .overflow_hidden()
        .rounded_full()
        .line_height(relative(1.))
        .cursor_pointer()
        .tab_stop(false)
        .text_color(idle_fg)
        .hover(move |style| style.bg(hover_bg).text_color(hover_fg))
        .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
        .child(
            svg()
                .size(px(PIN_ICON_SIZE))
                .flex_none()
                .line_height(relative(1.))
                .text_color(idle_fg)
                .group_hover(group, move |style| style.text_color(hover_fg))
                .path(path),
        )
}

fn capture_window_state(window: &Window) -> WindowState {
    let bounds = window.window_bounds().get_bounds();
    WindowState {
        width: f32::from(bounds.size.width),
        height: f32::from(bounds.size.height),
        x: f32::from(bounds.origin.x),
        y: f32::from(bounds.origin.y),
    }
}

pub fn prepare_for_shutdown(state: &Entity<AppState>, cx: &mut App) -> anyhow::Result<()> {
    let main_window = state.read(cx).main_window;
    if let Some(main_window) = main_window
        && let Ok(snapshot) = main_window.update(cx, |_, window, _| capture_window_state(window))
    {
        state.update(cx, |state, _| state.settings.window = Some(snapshot));
    }
    state.update(cx, |state, _| state.prepare_for_shutdown())
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
    if let Some(handle) = state.read(cx).main_window {
        let _ = handle.update(cx, |_, window, _| {
            window.activate_window();
        });
        return;
    }
    open_main_window(state, cx);
}

pub fn restore_from_dock(state: Entity<AppState>, cx: &mut App) {
    cx.defer(move |cx| restore_from_dock_now(state, cx));
}

fn restore_from_dock_now(state: Entity<AppState>, cx: &mut App) {
    if !state.read(cx).settings.docked {
        return;
    }
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
    let bounds = restored_main_bounds(&settings, cx);
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
                move |window, cx| {
                    let view = cx.new(|cx| MainWindow::new(state.clone(), window, cx));
                    cx.new(|cx| gpui_component::Root::new(view, window, cx))
                }
            },
        )
        .expect("open main");

    state.update(cx, |s, _| s.main_window = Some(handle));
    let _ = handle.update(cx, |_, window, _| {
        crate::ui::apply_main_window_constraints(window);
        crate::ui::apply_window_topmost(window, always_on_top);
    });
}

fn restored_main_bounds(
    settings: &crate::models::AppSettings,
    cx: &App,
) -> gpui::Bounds<gpui::Pixels> {
    let (width, height) = settings
        .window
        .as_ref()
        .map(|window| (window.width, window.height))
        .unwrap_or((DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT));
    let size = size(
        px(width.clamp(DEFAULT_WINDOW_WIDTH, MAX_WINDOW_WIDTH)),
        px(height.clamp(DEFAULT_WINDOW_HEIGHT, MAX_WINDOW_HEIGHT)),
    );
    let Some(saved) = settings.window.as_ref() else {
        return gpui::Bounds::centered(None, size, cx);
    };
    let candidate = gpui::Bounds {
        origin: gpui::point(px(saved.x), px(saved.y)),
        size,
    };
    if bounds_overlap_a_display(candidate, cx) {
        candidate
    } else {
        gpui::Bounds::centered(None, size, cx)
    }
}

fn bounds_overlap_a_display(bounds: gpui::Bounds<gpui::Pixels>, cx: &App) -> bool {
    let left = f32::from(bounds.origin.x);
    let top = f32::from(bounds.origin.y);
    let right = left + f32::from(bounds.size.width);
    let bottom = top + f32::from(bounds.size.height);
    cx.displays().iter().any(|display| {
        let area = display.bounds();
        let area_left = f32::from(area.origin.x);
        let area_top = f32::from(area.origin.y);
        let area_right = area_left + f32::from(area.size.width);
        let area_bottom = area_top + f32::from(area.size.height);
        let visible_width = right.min(area_right) - left.max(area_left);
        let visible_height = bottom.min(area_bottom) - top.max(area_top);
        visible_width >= 48.0 && visible_height >= 32.0
    })
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
