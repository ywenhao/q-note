//! Main note board window — layout/colors match the original Vue shell.

use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use gpui::StyledImage as _;
use gpui::{
    Animation, AnimationExt as _, App, AppContext, Context, Entity, FocusHandle,
    InteractiveElement, IntoElement, KeyDownEvent, MouseButton, ObjectFit, ParentElement, Render,
    ScrollHandle, Stateful, StatefulInteractiveElement, Styled, Window, WindowBounds,
    WindowControlArea, WindowDecorations, WindowOptions, deferred, div, img, px, relative, rgb,
    size, svg,
};
use gpui_component::{
    Disableable as _, Icon, IconName, IconNamed as _, Sizable as _, StyledExt as _,
    animation::cubic_bezier,
    badge::Badge,
    button::{Button, ButtonVariants as _},
    h_flex,
    menu::{ContextMenuExt, PopupMenuItem},
    scroll::ScrollableElement as _,
    tooltip::Tooltip,
    v_flex,
};

use crate::app_state::AppState;
use crate::models::{
    AttachmentKind, DEFAULT_WINDOW_HEIGHT, DEFAULT_WINDOW_WIDTH, MAX_WINDOW_HEIGHT,
    MAX_WINDOW_WIDTH, NOTE_COLORS, Note, NoteAttachment, WindowState,
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
const NOTE_THUMBNAIL_SIZE: f32 = 58.;
const NOTE_ACTION_TRIGGER_SIZE: f32 = 20.;
const NOTE_ACTION_BUTTON_SIZE: f32 = 22.;
const NOTE_ACTION_ICON_SIZE: f32 = 14.;
const PREVIEW_SCALE_STEP: f32 = 0.25;
const PREVIEW_KEY_PAN_STEP: f32 = 36.;

#[derive(Clone)]
struct NoteDrag {
    note_id: String,
}

impl Render for NoteDrag {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_3()
            .py_2()
            .rounded(px(CARD_RADIUS))
            .bg(color_alpha(0xffffff, 0.88))
            .shadow_lg()
            .text_sm()
            .child("Q Note")
    }
}

struct ResizeState {
    note_id: String,
    start_y: f32,
    start_height: i64,
    min_height: i64,
    max_height: i64,
}

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

#[derive(Clone, Copy, PartialEq, Eq)]
enum UpdatePhase {
    Downloading,
    Preparing,
    Installing,
}

struct UpdateDownloadState {
    info: updater::UpdateInfo,
    phase: UpdatePhase,
    progress: updater::DownloadProgress,
    cancelled: Arc<AtomicBool>,
    revision: u64,
}

pub struct MainWindow {
    state: Entity<AppState>,
    actions_note_id: Option<String>,
    note_scroll_handle: ScrollHandle,
    palette_note_id: Option<String>,
    window_state_revision: u64,
    modal: Option<AppModal>,
    modal_closing: bool,
    modal_generation: u64,
    update_confirm: Option<updater::UpdateInfo>,
    update_confirm_closing: bool,
    update_confirm_generation: u64,
    update_schedule_generation: u64,
    update_download: Option<UpdateDownloadState>,
    update_download_revision: u64,
    drop_target: Option<(String, bool)>,
    resize_state: Option<ResizeState>,
    suppress_copy_until: Option<Instant>,
    image_preview: Option<ImagePreviewState>,
    preview_drag: Option<PreviewDrag>,
    preview_focus: FocusHandle,
}

impl MainWindow {
    pub fn new(state: Entity<AppState>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        window.on_window_should_close(cx, {
            let state = state.clone();
            move |window, cx| {
                // Closing the board hides it while the tray keeps the process alive.
                hide_main_window(&state, window, cx);
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
        let mut this = Self {
            state,
            actions_note_id: None,
            note_scroll_handle: ScrollHandle::default(),
            palette_note_id: None,
            window_state_revision: 0,
            modal: None,
            modal_closing: false,
            modal_generation: 0,
            update_confirm: None,
            update_confirm_closing: false,
            update_confirm_generation: 0,
            update_schedule_generation: 0,
            update_download: None,
            update_download_revision: 0,
            drop_target: None,
            resize_state: None,
            suppress_copy_until: None,
            image_preview: None,
            preview_drag: None,
            preview_focus: cx.focus_handle().tab_stop(true),
        };
        this.start_update_scheduler(window, cx);
        this
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

    fn open_image_preview(
        &mut self,
        images: Vec<NoteAttachment>,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if images.is_empty() {
            return;
        }
        self.image_preview = Some(ImagePreviewState {
            index: index.min(images.len() - 1),
            images,
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
        let Some(preview) = self.image_preview.as_mut() else {
            return;
        };
        preview.scale = (preview.scale + delta).clamp(0.5, 6.);
        if preview.scale <= 1. {
            preview.offset_x = 0.;
            preview.offset_y = 0.;
        }
        cx.notify();
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

    fn begin_note_resize(&mut self, note: &Note, window: &Window, cx: &mut Context<Self>) {
        let min_height = default_note_height(note);
        let max_height = full_note_height(note).max(min_height);
        self.resize_state = Some(ResizeState {
            note_id: note.id.clone(),
            start_y: f32::from(window.mouse_position().y),
            start_height: note.text_height.unwrap_or(min_height),
            min_height,
            max_height,
        });
        self.suppress_copy_until = Some(Instant::now() + Duration::from_millis(800));
        cx.notify();
    }

    fn resize_note(&mut self, y: f32, cx: &mut Context<Self>) {
        let Some(resize) = self.resize_state.as_ref() else {
            return;
        };
        let next = (resize.start_height as f32 + y - resize.start_y)
            .clamp(resize.min_height as f32, resize.max_height as f32);
        let snapped = ((next / LINE_HEIGHT).ceil() * LINE_HEIGHT) as i64;
        let id = resize.note_id.clone();
        let _ = self
            .state
            .update(cx, |state, cx| state.set_note_height(&id, snapped, cx));
    }

    fn finish_note_resize(&mut self, cx: &mut Context<Self>) {
        self.resize_state = None;
        self.suppress_copy_until = Some(Instant::now() + Duration::from_millis(300));
        cx.notify();
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

    fn start_update_scheduler(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.update_schedule_generation = self.update_schedule_generation.wrapping_add(1);
        self.run_update_check(false, window, cx);
        self.schedule_daily_check(window, cx);
    }

    fn schedule_daily_check(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let generation = self.update_schedule_generation;
        let wait = updater::duration_until_next_daily_check();
        cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(wait).await;
            let _ = this.update_in(cx, |this, window, cx| {
                if this.update_schedule_generation != generation {
                    return;
                }
                this.run_update_check(false, window, cx);
                this.schedule_daily_check(window, cx);
            });
        })
        .detach();
    }

    fn update_confirm_open(&self) -> bool {
        self.update_confirm.is_some() && !self.update_confirm_closing
    }

    fn open_update_confirm(&mut self, cx: &mut Context<Self>) {
        let Some(info) = self.state.read(cx).update.available.clone() else {
            return;
        };
        if self.update_confirm_open() {
            return;
        }
        self.update_confirm = Some(info);
        self.update_confirm_closing = false;
        self.update_confirm_generation = self.update_confirm_generation.wrapping_add(1);
        cx.notify();
    }

    fn request_close_update_confirm(&mut self, cx: &mut Context<Self>) {
        if self.update_confirm.is_none() || self.update_confirm_closing {
            return;
        }
        self.update_confirm_closing = true;
        let generation = self.update_confirm_generation;
        cx.notify();
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(modal::PANEL_MS))
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.update_confirm_generation == generation && this.update_confirm_closing {
                    this.update_confirm = None;
                    this.update_confirm_closing = false;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn handle_check_update(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.state.read(cx).update.checking || self.update_confirm_open() {
            return;
        }
        if self.state.read(cx).update.available.is_some() {
            self.open_update_confirm(cx);
            return;
        }
        self.run_update_check(true, window, cx);
    }

    fn confirm_update(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.update_confirm_closing || self.update_download.is_some() {
            return;
        }
        let info = self.update_confirm.clone();
        self.request_close_update_confirm(cx);
        let Some(info) = info else {
            return;
        };

        if info.current_binary().is_none() {
            updater::open_release_page(Some(&info.version));
            self.state.update(cx, |state, cx| {
                let msg = state.tr().update_open_release.to_string();
                state.show_toast(msg, cx);
            });
            return;
        }

        let snapshot = capture_window_state(window);
        let prepared = self.state.update(cx, |s, _| {
            s.settings.window = Some(snapshot);
            s.persist_settings()?;
            s.db.flush()
        });
        if let Err(error) = prepared {
            self.state.update(cx, |s, cx| {
                s.update.error = Some(error.to_string());
                s.update.available = None;
                let msg = s.tr().update_prepare_failed.to_string();
                s.show_toast(msg, cx);
                cx.notify();
            });
            return;
        }
        self.start_update_download(info, window, cx);
    }

    fn start_update_download(
        &mut self,
        info: updater::UpdateInfo,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.update_download_revision = self.update_download_revision.wrapping_add(1);
        let revision = self.update_download_revision;
        let cancelled = Arc::new(AtomicBool::new(false));
        let progress = Arc::new(Mutex::new(updater::DownloadProgress {
            downloaded: 0,
            total: info.current_binary().and_then(|artifact| artifact.size),
        }));
        self.update_download = Some(UpdateDownloadState {
            info: info.clone(),
            phase: UpdatePhase::Downloading,
            progress: progress
                .lock()
                .map(|progress| *progress)
                .unwrap_or_default(),
            cancelled: cancelled.clone(),
            revision,
        });
        cx.notify();
        self.poll_update_progress(revision, progress.clone(), window, cx);

        cx.spawn_in(window, async move |this, cx| {
            let shared_progress = progress;
            let download_cancelled = cancelled;
            let result = cx
                .background_executor()
                .spawn(async move {
                    updater::download_update(&info, &download_cancelled, |next| {
                        if let Ok(mut progress) = shared_progress.lock() {
                            *progress = next;
                        }
                    })
                })
                .await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.finish_update_download(revision, result, window, cx);
            });
        })
        .detach();
    }

    fn poll_update_progress(
        &mut self,
        revision: u64,
        progress: Arc<Mutex<updater::DownloadProgress>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.spawn_in(window, async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(80))
                    .await;
                let next = progress.lock().map(|progress| *progress).ok();
                let keep_polling = this
                    .update_in(cx, |this, _, cx| {
                        let Some(download) = this.update_download.as_mut() else {
                            return false;
                        };
                        if download.revision != revision
                            || download.phase != UpdatePhase::Downloading
                        {
                            return false;
                        }
                        if let Some(next) = next
                            && (next.downloaded != download.progress.downloaded
                                || next.total != download.progress.total)
                        {
                            download.progress = next;
                            cx.notify();
                        }
                        true
                    })
                    .unwrap_or(false);
                if !keep_polling {
                    break;
                }
            }
        })
        .detach();
    }

    fn finish_update_download(
        &mut self,
        revision: u64,
        result: anyhow::Result<updater::DownloadedUpdate>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(download) = self.update_download.as_mut() else {
            return;
        };
        if download.revision != revision || download.cancelled.load(Ordering::Relaxed) {
            return;
        }
        let downloaded = match result {
            Ok(downloaded) => downloaded,
            Err(error) => {
                self.update_download = None;
                self.state.update(cx, |state, cx| {
                    state.update.error = Some(error.to_string());
                    state.update.available = None;
                    let msg = state.tr().update_download_failed.to_string();
                    state.show_toast(msg, cx);
                    cx.notify();
                });
                cx.notify();
                return;
            }
        };

        download.phase = UpdatePhase::Preparing;
        download.progress.downloaded = download
            .progress
            .total
            .unwrap_or(download.progress.downloaded);
        cx.notify();
        let snapshot = capture_window_state(window);
        let prepared = self.state.update(cx, |state, _| {
            state.settings.window = Some(snapshot);
            state.prepare_for_update()
        });
        if let Err(error) = prepared {
            self.update_download = None;
            self.state.update(cx, |state, cx| {
                state.update.error = Some(error.to_string());
                state.update.available = None;
                let msg = state.tr().update_prepare_failed.to_string();
                state.show_toast(msg, cx);
                cx.notify();
            });
            cx.notify();
            return;
        }
        let Some(download) = self.update_download.as_mut() else {
            return;
        };
        if download.revision != revision || download.cancelled.load(Ordering::Relaxed) {
            return;
        }
        download.phase = UpdatePhase::Installing;
        cx.notify();

        cx.spawn_in(window, async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { updater::install_and_relaunch(downloaded) })
                .await;
            let _ = this.update_in(cx, |this, _, cx| match result {
                Ok(()) => {
                    let _ = prepare_for_shutdown(&this.state, cx);
                    cx.quit();
                }
                Err(error) => {
                    this.update_download = None;
                    this.state.update(cx, |state, cx| {
                        state.update.error = Some(error.to_string());
                        state.update.available = None;
                        let msg = state.tr().update_install_failed.to_string();
                        state.show_toast(msg, cx);
                        cx.notify();
                    });
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn cancel_update_download(&mut self, cx: &mut Context<Self>) {
        let Some(download) = self.update_download.as_ref() else {
            return;
        };
        if download.phase == UpdatePhase::Installing {
            return;
        }
        download.cancelled.store(true, Ordering::Relaxed);
        self.update_download_revision = self.update_download_revision.wrapping_add(1);
        self.update_download = None;
        self.state.update(cx, |state, cx| {
            state.update.available = None;
            cx.notify();
        });
        cx.notify();
    }

    fn run_update_check(&mut self, manual: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.state.read(cx).update.checking {
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
            let _ = this.update_in(cx, |this, _, cx| match result {
                Ok(Some(info)) => {
                    state.update(cx, |s, cx| {
                        s.update.checking = false;
                        s.update.available = Some(info);
                        s.update.error = None;
                        cx.notify();
                    });
                    if manual {
                        this.open_update_confirm(cx);
                    }
                }
                Ok(None) => {
                    state.update(cx, |s, cx| {
                        s.update.checking = false;
                        s.update.available = None;
                        s.update.error = None;
                        if manual {
                            let msg = s.tr().update_none.to_string();
                            s.show_toast(msg, cx);
                        }
                        cx.notify();
                    });
                    this.request_close_update_confirm(cx);
                }
                Err(error) => {
                    state.update(cx, |s, cx| {
                        s.update.checking = false;
                        s.update.available = None;
                        s.update.error = Some(error.to_string());
                        if manual {
                            let msg = s.tr().update_check_failed.to_string();
                            s.show_toast(msg, cx);
                        }
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
                "app-modal",
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
        let lang_key = self.state.read(cx).settings.language.element_key();
        let state = self.state.clone();

        modal::animate_panel(
            "settings",
            modal::settings_shell(lang_key)
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
                                    this.handle_check_update(window, cx);
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
        let lang_key = self.state.read(cx).settings.language.element_key();
        let state = self.state.clone();

        modal::animate_panel(
            "confirm-delete-all",
            modal::confirm_shell("confirm-dialog", lang_key)
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

    fn render_update_confirm(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let tr = self.state.read(cx).tr();
        let closing = self.update_confirm_closing;
        let generation = self.update_confirm_generation;
        let lang_key = self.state.read(cx).settings.language.element_key();
        let info = self.update_confirm.clone();
        let version = info
            .as_ref()
            .map(|item| item.version.clone())
            .unwrap_or_default();
        let notes = info
            .as_ref()
            .and_then(|item| item.notes.clone())
            .filter(|notes| !notes.trim().is_empty());
        let title = tr.update_available_title(&version);

        modal::overlay_layer("update-confirm-layer")
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| this.request_close_update_confirm(cx)),
            )
            .child(modal::animate_overlay(
                "update-confirm",
                modal::overlay_backdrop("update-confirm-backdrop").on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| this.request_close_update_confirm(cx)),
                ),
                closing,
                generation,
            ))
            .child(modal::animate_panel(
                "update-confirm",
                modal::confirm_shell("update-confirm-dialog", lang_key)
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .pb(px(8.))
                            .text_lg()
                            .font_semibold()
                            .text_color(color(0x1d1d1f))
                            .child(title),
                    )
                    .child(
                        div()
                            .pb(if notes.is_some() { px(10.) } else { px(18.) })
                            .text_sm()
                            .text_color(color_alpha(0x3c3c43, 0.78))
                            .child(tr.update_confirm_default),
                    )
                    .when_some(notes, |this, notes| {
                        this.child(
                            div()
                                .pb(px(18.))
                                .text_sm()
                                .text_color(color_alpha(0x3c3c43, 0.62))
                                .child(notes),
                        )
                    })
                    .child(
                        h_flex()
                            .w_full()
                            .justify_end()
                            .gap(px(8.))
                            .child(
                                modal::text_button("update-confirm-cancel")
                                    .child(tr.cancel)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.request_close_update_confirm(cx);
                                    })),
                            )
                            .child(
                                modal::primary_button("update-confirm-ok")
                                    .child(tr.update_confirm)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.confirm_update(window, cx);
                                    })),
                            ),
                    ),
                closing,
                generation,
            ))
    }

    fn render_update_download(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let tr = self.state.read(cx).tr();
        let lang_key = self.state.read(cx).settings.language.element_key();
        let download = self.update_download.as_ref();
        let phase = download
            .map(|download| download.phase)
            .unwrap_or(UpdatePhase::Downloading);
        let progress = download
            .map(|download| download.progress)
            .unwrap_or_default();
        let version = download
            .map(|download| download.info.version.clone())
            .unwrap_or_default();
        let notes = download
            .and_then(|download| download.info.notes.clone())
            .filter(|notes| !notes.trim().is_empty());
        let status = match phase {
            UpdatePhase::Downloading => tr.update_downloading,
            UpdatePhase::Preparing => tr.update_preparing,
            UpdatePhase::Installing => tr.update_installing,
        };
        let percent = if phase == UpdatePhase::Downloading {
            progress
                .total
                .filter(|total| *total > 0)
                .map(|total| (progress.downloaded as f32 / total as f32).clamp(0., 1.))
                .unwrap_or(0.)
        } else {
            1.
        };

        modal::overlay_layer("update-download-layer")
            .child(modal::overlay_backdrop("update-download-backdrop"))
            .child(
                modal::confirm_shell("update-download-dialog", lang_key)
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .gap_2()
                            .pb_3()
                            .font_semibold()
                            .child(Icon::new(IconName::ArrowDown).with_size(px(15.)))
                            .child(status),
                    )
                    .child(
                        v_flex()
                            .w_full()
                            .gap_1()
                            .pb_3()
                            .child(div().font_semibold().child(format!("Q Note v{version}")))
                            .when_some(notes, |this, notes| {
                                this.child(
                                    div()
                                        .text_sm()
                                        .text_color(color_alpha(0x3c3c43, 0.68))
                                        .child(notes),
                                )
                            }),
                    )
                    .child(
                        div()
                            .w_full()
                            .h(px(8.))
                            .overflow_hidden()
                            .rounded_full()
                            .bg(color_alpha(0x3c3c43, 0.12))
                            .child(
                                div()
                                    .h_full()
                                    .w(relative(percent.max(
                                        if phase == UpdatePhase::Downloading {
                                            0.
                                        } else {
                                            0.35
                                        },
                                    )))
                                    .rounded_full()
                                    .bg(color(ACCENT)),
                            ),
                    )
                    .child(
                        h_flex()
                            .w_full()
                            .justify_between()
                            .pt_2()
                            .text_xs()
                            .text_color(color_alpha(0x3c3c43, 0.72))
                            .child(if phase == UpdatePhase::Downloading {
                                format!("{}%", (percent * 100.).round() as u32)
                            } else {
                                status.to_string()
                            })
                            .when(
                                phase == UpdatePhase::Downloading && progress.total.is_some(),
                                |this| {
                                    this.child(format!(
                                        "{} / {}",
                                        format_bytes(progress.downloaded),
                                        format_bytes(progress.total.unwrap_or_default())
                                    ))
                                },
                            ),
                    )
                    .when(phase != UpdatePhase::Installing, |this| {
                        this.child(
                            h_flex().w_full().justify_end().pt_3().child(
                                modal::text_button("update-download-cancel")
                                    .child(tr.cancel)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.cancel_update_download(cx);
                                    })),
                            ),
                        )
                    }),
            )
    }

    fn render_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let tr = self.state.read(cx).tr();
        let empty = self.state.read(cx).notes.is_empty();
        let lang = tr.language_toggle;
        let lang_key = self.state.read(cx).settings.language.element_key();
        let has_update = self.state.read(cx).update.available.is_some();
        let settings_tooltip = match self.state.read(cx).update.available.as_ref() {
            Some(info) => tr.update_available_title(&info.version),
            None => tr.settings.to_string(),
        };
        let settings_button = Button::new(("settings", lang_key))
            .ghost()
            .icon(IconName::Settings)
            .tooltip(settings_tooltip)
            .on_click(cx.listener(|this, _, window, cx| this.open_settings(window, cx)));

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
                Button::new(("new", lang_key))
                    .ghost()
                    .icon(IconName::Plus)
                    .tooltip(tr.new_note)
                    .on_click(cx.listener(|this, _, window, cx| this.open_new(window, cx))),
            )
            .child(
                Button::new(("delete-all", lang_key))
                    .ghost()
                    .icon(IconName::Delete)
                    .text_color(color(DANGER))
                    .disabled(empty)
                    .tooltip(tr.delete_all)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.confirm_delete_all(window, cx);
                    })),
            )
            .child(if has_update {
                Badge::new()
                    .dot()
                    .color(color(DANGER))
                    .child(settings_button)
                    .into_any_element()
            } else {
                settings_button.into_any_element()
            })
            .child(
                Button::new(("lang", lang_key))
                    .ghost()
                    .icon(IconName::Globe)
                    .label(lang)
                    .tooltip(tr.switch_language)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.state.update(cx, |s, cx| s.toggle_language(cx));
                        cx.notify();
                        window.refresh();
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
        let action_icon = color_alpha(0x1d1d1f, 0.82);
        let action_hover = color_alpha(0xffffff, 0.64);
        let lang_key = self.state.read(cx).settings.language.element_key();
        let state = self.state.clone();
        let view = cx.entity();
        let drag_style_id = note_id.clone();
        let image_attachments: Vec<_> = note
            .attachments
            .iter()
            .filter(|attachment| attachment.kind == AttachmentKind::Image)
            .cloned()
            .collect();
        let file_attachments: Vec<_> = note
            .attachments
            .iter()
            .filter(|attachment| attachment.kind == AttachmentKind::File)
            .cloned()
            .collect();
        let resizing = self
            .resize_state
            .as_ref()
            .is_some_and(|resize| resize.note_id == note.id);

        let action_trigger_group =
            gpui::SharedString::from(format!("more-actions-{}-{lang_key}", note.id));
        let action_trigger = h_flex()
            .id(action_trigger_group.clone())
            .group(action_trigger_group.clone())
            .size(px(NOTE_ACTION_TRIGGER_SIZE))
            .justify_center()
            .rounded_full()
            .bg(color_alpha(0xffffff, 0.46))
            .line_height(relative(1.))
            .cursor_pointer()
            .tab_stop(false)
            .text_color(color_alpha(0x1d1d1f, 0.58))
            .hover(|style| {
                style
                    .bg(color_alpha(0xffffff, 0.78))
                    .text_color(color_alpha(ACCENT, 0.86))
            })
            .tooltip({
                let state = state.clone();
                move |window, cx| Tooltip::new(state.read(cx).tr().more_actions).build(window, cx)
            })
            .on_hover(cx.listener({
                let id = note_id.clone();
                move |this, hovered: &bool, _, cx| {
                    if *hovered && this.actions_note_id.as_deref() != Some(id.as_str()) {
                        this.actions_note_id = Some(id.clone());
                        cx.notify();
                    }
                }
            }))
            .child(
                svg()
                    .size(px(12.))
                    .flex_none()
                    .line_height(relative(1.))
                    .text_color(color_alpha(0x1d1d1f, 0.58))
                    .group_hover(action_trigger_group, |style| {
                        style.text_color(color_alpha(ACCENT, 0.86))
                    })
                    .path(IconName::ChevronLeft.path()),
            );

        let action_dock = h_flex()
            .id(gpui::SharedString::from(format!(
                "note-action-dock-{}",
                note.id
            )))
            .absolute()
            .top_1_2()
            .right(px(2.))
            .mt(px(-15.))
            .h(px(30.))
            .w(if actions_open {
                px(162.)
            } else {
                px(NOTE_ACTION_TRIGGER_SIZE)
            })
            .items_center()
            .justify_end()
            .gap(px(2.))
            .invisible()
            .group_hover(card_group.clone(), |style| style.visible())
            .when(actions_open, |this| this.visible())
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
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
                            note_action_icon_button(
                                gpui::SharedString::from(format!("pin-{}", note.id)),
                                if pinned {
                                    crate::PIN_OFF_ICON_PATH
                                } else {
                                    crate::PIN_ICON_PATH
                                },
                                {
                                    let id = note_id.clone();
                                    move |s| {
                                        if s.notes.iter().any(|n| n.id == id && n.pinned) {
                                            s.tr().unpin
                                        } else {
                                            s.tr().pin
                                        }
                                    }
                                },
                                state.clone(),
                                lang_key,
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
                            note_action_icon_button(
                                gpui::SharedString::from(format!("edit-{}", note.id)),
                                IconName::SquareTerminal.path(),
                                |s| s.tr().edit,
                                state.clone(),
                                lang_key,
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
                                    note_action_icon_button(
                                        gpui::SharedString::from(format!("color-{}", note.id)),
                                        IconName::Palette.path(),
                                        |s| s.tr().color,
                                        state.clone(),
                                        lang_key,
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
                            note_action_icon_button(
                                gpui::SharedString::from(format!("copy-{}", note.id)),
                                IconName::Copy.path(),
                                |s| s.tr().copy,
                                state.clone(),
                                lang_key,
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
                            note_action_icon_button(
                                gpui::SharedString::from(format!("delete-{}", note.id)),
                                IconName::Delete.path(),
                                |s| s.tr().delete,
                                state.clone(),
                                lang_key,
                                color(DANGER),
                                color(DANGER),
                                action_hover,
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
            .child(action_trigger);

        let card = v_flex()
            .id(gpui::SharedString::from(format!(
                "note-card-{}",
                note.id.clone()
            )))
            .group(card_group.clone())
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
            .drag_over::<NoteDrag>(move |style, dragged, _, _| {
                if dragged.note_id == drag_style_id {
                    style
                } else {
                    style.border_color(color_alpha(ACCENT, 0.72))
                }
            })
            .on_drag(
                NoteDrag {
                    note_id: note_id.clone(),
                },
                {
                    let view = view.clone();
                    move |drag, _, _, cx| {
                        let _ = view.update(cx, |this, cx| {
                            this.suppress_copy_until =
                                Some(Instant::now() + Duration::from_millis(800));
                            this.drop_target = None;
                            cx.notify();
                        });
                        cx.new(|_| drag.clone())
                    }
                },
            )
            .on_drag_move::<NoteDrag>(cx.listener({
                let target_id = note_id.clone();
                move |this, event: &gpui::DragMoveEvent<NoteDrag>, _, cx| {
                    let dragged = event.drag(cx);
                    if dragged.note_id == target_id {
                        return;
                    }
                    let center_y =
                        f32::from(event.bounds.origin.y) + f32::from(event.bounds.size.height) / 2.;
                    let after = f32::from(event.event.position.y) >= center_y;
                    if this.drop_target.as_ref() != Some(&(target_id.clone(), after)) {
                        this.drop_target = Some((target_id.clone(), after));
                        cx.notify();
                    }
                }
            }))
            .on_drop::<NoteDrag>(cx.listener({
                let state = self.state.clone();
                let target_id = note_id.clone();
                move |this, dragged: &NoteDrag, _, cx| {
                    let after = this
                        .drop_target
                        .as_ref()
                        .filter(|(id, _)| id == &target_id)
                        .map(|(_, after)| *after)
                        .unwrap_or(false);
                    let _ = state.update(cx, |state, cx| {
                        state.reorder_note(&dragged.note_id, &target_id, after, cx)
                    });
                    this.drop_target = None;
                    this.suppress_copy_until = Some(Instant::now() + Duration::from_millis(400));
                    cx.notify();
                }
            }))
            .on_click(cx.listener({
                let id = note_id.clone();
                move |this, _, _, cx| {
                    if this
                        .suppress_copy_until
                        .is_some_and(|until| Instant::now() < until)
                    {
                        return;
                    }
                    this.state.update(cx, |state, cx| state.copy_note(&id, cx));
                }
            }))
            .context_menu({
                let state = self.state.clone();
                let id = note_id.clone();
                move |menu, _window, cx| {
                    let app = state.read(cx);
                    let tr = app.tr();
                    let pinned = app
                        .notes
                        .iter()
                        .find(|note| note.id == id)
                        .is_some_and(|note| note.pinned);
                    let pin_label = if pinned { tr.unpin } else { tr.pin };
                    menu.item(PopupMenuItem::new(tr.copy).on_click({
                        let state = state.clone();
                        let id = id.clone();
                        move |_, _, cx| {
                            state.update(cx, |s, cx| s.copy_note(&id, cx));
                        }
                    }))
                    .item(PopupMenuItem::new(tr.edit).on_click({
                        let state = state.clone();
                        let id = id.clone();
                        move |_, _, cx| {
                            editor_window::open_editor(state.clone(), Some(id.clone()), false, cx);
                        }
                    }))
                    .item(PopupMenuItem::new(pin_label).on_click({
                        let state = state.clone();
                        let id = id.clone();
                        move |_, _, cx| {
                            let _ = state.update(cx, |s, cx| s.toggle_pin(&id, cx));
                        }
                    }))
                    .separator()
                    .item(PopupMenuItem::new(tr.delete).on_click({
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
            .when(!image_attachments.is_empty(), |this| {
                let preview_images = image_attachments.clone();
                this.child(
                    h_flex()
                        .id(gpui::SharedString::from(format!("note-images-{}", note.id)))
                        .w_full()
                        .flex_wrap()
                        .gap_2()
                        .mt_2p5()
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .on_click(|_, _, cx| cx.stop_propagation())
                        .children(image_attachments.iter().take(4).enumerate().filter_map(
                            |(index, attachment)| {
                                let source = crate::ui::media::attachment_image_source(attachment)?;
                                let images = preview_images.clone();
                                Some(
                                    div()
                                        .id(gpui::SharedString::from(format!(
                                            "note-image-{}-{}",
                                            note.id, attachment.id
                                        )))
                                        .size(px(NOTE_THUMBNAIL_SIZE))
                                        .flex_none()
                                        .overflow_hidden()
                                        .rounded(px(7.))
                                        .border_1()
                                        .border_color(color_alpha(0x1d2735, 0.14))
                                        .bg(color_alpha(0xffffff, 0.46))
                                        .cursor_pointer()
                                        .hover(|style| {
                                            style
                                                .border_color(color_alpha(ACCENT, 0.36))
                                                .shadow_md()
                                        })
                                        .child(img(source).size_full().object_fit(ObjectFit::Cover))
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.open_image_preview(
                                                images.clone(),
                                                index,
                                                window,
                                                cx,
                                            );
                                            cx.stop_propagation();
                                        }))
                                        .into_any_element(),
                                )
                            },
                        )),
                )
            })
            .when(!file_attachments.is_empty(), |this| {
                this.child(
                    v_flex()
                        .id(gpui::SharedString::from(format!("note-files-{}", note.id)))
                        .w_full()
                        .gap_1p5()
                        .mt_2p5()
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .on_click(|_, _, cx| cx.stop_propagation())
                        .children(file_attachments.iter().take(3).map(|attachment| {
                            h_flex()
                                .id(gpui::SharedString::from(format!(
                                    "note-file-{}-{}",
                                    note.id, attachment.id
                                )))
                                .w_full()
                                .min_w_0()
                                .gap_1p5()
                                .px_2()
                                .py_1p5()
                                .rounded(px(8.))
                                .border_1()
                                .border_color(color_alpha(0x3c3c43, 0.12))
                                .bg(color_alpha(0xffffff, 0.48))
                                .text_xs()
                                .text_color(color_alpha(TEXT, 0.76))
                                .child(Icon::new(IconName::File).with_size(px(13.)))
                                .child(
                                    div()
                                        .min_w_0()
                                        .flex_1()
                                        .overflow_hidden()
                                        .text_ellipsis()
                                        .whitespace_nowrap()
                                        .child(attachment_label(attachment)),
                                )
                        })),
                )
            })
            .child(
                div()
                    .id(gpui::SharedString::from(format!("resize-{}", note.id)))
                    .absolute()
                    .right(px(12.))
                    .bottom(px(1.))
                    .w_6()
                    .h_3()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(8.))
                    .text_color(color_alpha(0x1f2937, 0.52))
                    .cursor_ns_resize()
                    .invisible()
                    .group_hover(card_group.clone(), |style| style.visible())
                    .when(resizing, |style| style.visible())
                    .hover(|style| {
                        style
                            .bg(color_alpha(0xffffff, 0.50))
                            .text_color(color(0x1f2937))
                    })
                    .tooltip(move |window, cx| Tooltip::new(tr.resize).build(window, cx))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener({
                            let note = note.clone();
                            move |this, _, window, cx| {
                                this.begin_note_resize(&note, window, cx);
                                cx.stop_propagation();
                            }
                        }),
                    )
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(
                        Icon::empty()
                            .path(crate::GRIP_HORIZONTAL_ICON_PATH)
                            .with_size(px(12.)),
                    ),
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
        let lang_key = self.state.read(cx).settings.language.element_key();
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
                Button::new(("empty-new", lang_key))
                    .primary()
                    .label(tr.empty_action)
                    .on_click(cx.listener(|this, _, window, cx| this.open_new(window, cx))),
            )
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
        let current = preview.images.get(preview.index).cloned();
        let source = current
            .as_ref()
            .and_then(crate::ui::media::attachment_image_source);
        let tr = self.state.read(cx).tr();

        div()
            .id("image-preview")
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
                                        .id("preview-image")
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
                    preview_icon_button(
                        "preview-previous",
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
                    preview_icon_button("preview-next", IconName::ChevronRight, tr.next_image)
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
                    .text_color(rgb(0xffffff))
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
                        preview_icon_button("preview-zoom-out", IconName::Minus, tr.zoom_out)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.zoom_image_preview(-PREVIEW_SCALE_STEP, cx);
                                cx.stop_propagation();
                            })),
                    )
                    .child(
                        preview_icon_button("preview-zoom-in", IconName::Plus, tr.zoom_in)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.zoom_image_preview(PREVIEW_SCALE_STEP, cx);
                                cx.stop_propagation();
                            })),
                    )
                    .child(
                        preview_icon_button("preview-reset", IconName::Undo2, tr.reset_view)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.reset_image_preview(cx);
                                cx.stop_propagation();
                            })),
                    )
                    .child(
                        preview_icon_button("preview-close", IconName::Close, tr.cancel).on_click(
                            cx.listener(|this, _, _, cx| {
                                this.image_preview = None;
                                cx.notify();
                                cx.stop_propagation();
                            }),
                        ),
                    ),
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
        let lang_key = self.state.read(cx).settings.language.element_key();
        let state = self.state.clone();

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
                                    "close",
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
                                        hide_main_window(&this.state, window, cx);
                                    },
                                )),
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
            .when(self.update_confirm.is_some(), |this| {
                this.child(self.render_update_confirm(cx))
            })
            .when(self.update_download.is_some(), |this| {
                this.child(self.render_update_download(cx))
            })
            .when(self.image_preview.is_some(), |this| {
                this.child(self.render_image_preview(cx))
            })
            .on_mouse_move(cx.listener(|this, event: &gpui::MouseMoveEvent, _, cx| {
                if this.resize_state.is_some() && event.pressed_button == Some(MouseButton::Left) {
                    this.resize_note(f32::from(event.position.y), cx);
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    if this.resize_state.is_some() {
                        this.finish_note_resize(cx);
                    }
                }),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    if this.resize_state.is_some() {
                        this.finish_note_resize(cx);
                    }
                }),
            )
            .children(dialog_layer)
            .children(notification_layer)
    }
}

fn preview_icon_button(
    id: impl Into<gpui::SharedString>,
    icon: IconName,
    tooltip: impl Into<gpui::SharedString>,
) -> Stateful<gpui::Div> {
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

fn default_note_height(note: &Note) -> i64 {
    if note.content.trim().is_empty()
        || (note.content.chars().count() <= 34 && !note.content.contains('\n'))
    {
        LINE_HEIGHT as i64
    } else {
        (LINE_HEIGHT * 2.) as i64
    }
}

fn full_note_height(note: &Note) -> i64 {
    let lines = note
        .content
        .lines()
        .map(|line| line.chars().count().max(1).div_ceil(34))
        .sum::<usize>()
        .max(1);
    (lines as f32 * LINE_HEIGHT) as i64
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024. && unit < UNITS.len() - 1 {
        value /= 1024.;
        unit += 1;
    }
    if unit == 0 {
        format!("{} {}", bytes, UNITS[unit])
    } else if value >= 10. {
        format!("{value:.0} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

fn pin_icon(off: bool) -> Icon {
    Icon::empty().path(if off {
        crate::PIN_OFF_ICON_PATH
    } else {
        crate::PIN_ICON_PATH
    })
}

fn note_action_icon_button(
    id: impl Into<gpui::SharedString>,
    path: impl Into<gpui::SharedString>,
    tooltip: impl Fn(&AppState) -> &'static str + 'static,
    state: Entity<AppState>,
    lang_key: u64,
    idle_fg: gpui::Rgba,
    hover_fg: gpui::Rgba,
    hover_bg: gpui::Rgba,
) -> Stateful<gpui::Div> {
    let id = gpui::SharedString::from(format!("{}-{lang_key}", id.into()));
    let group = id.clone();
    h_flex()
        .id(id)
        .group(group.clone())
        .min_w(px(NOTE_ACTION_BUTTON_SIZE))
        .h(px(NOTE_ACTION_BUTTON_SIZE))
        .px(px(4.))
        .flex_none()
        .justify_center()
        .rounded(px(8.))
        .border_1()
        .border_color(color_alpha(0x3c3c43, 0.08))
        .bg(color_alpha(0xffffff, 0.34))
        .line_height(relative(1.))
        .cursor_pointer()
        .tab_stop(false)
        .text_color(idle_fg)
        .hover(move |style| style.bg(hover_bg).text_color(hover_fg))
        .tooltip(move |window, cx| Tooltip::new(tooltip(state.read(cx))).build(window, cx))
        .child(
            svg()
                .size(px(NOTE_ACTION_ICON_SIZE))
                .flex_none()
                .line_height(relative(1.))
                .text_color(idle_fg)
                .group_hover(group, move |style| style.text_color(hover_fg))
                .path(path),
        )
}

pub(crate) fn centered_icon_button(
    id: impl Into<gpui::SharedString>,
    path: impl Into<gpui::SharedString>,
    tooltip: impl Fn(&AppState) -> &'static str + 'static,
    state: Entity<AppState>,
    lang_key: u64,
    idle_fg: gpui::Rgba,
    hover_fg: gpui::Rgba,
    hover_bg: gpui::Rgba,
) -> Stateful<gpui::Div> {
    let id = gpui::SharedString::from(format!("{}-{lang_key}", id.into()));
    let group = id.clone();
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
        .tooltip(move |window, cx| Tooltip::new(tooltip(state.read(cx))).build(window, cx))
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

/// Persist the current board bounds and hide the native window without destroying its handle.
/// Keeping the handle alive lets the tray restore the same GPUI window on all platforms.
pub fn hide_main_window(state: &Entity<AppState>, window: &mut Window, cx: &mut App) {
    let snapshot = capture_window_state(window);
    state.update(cx, |state, _| state.settings.window = Some(snapshot));
    state.update(cx, |state, _| {
        let _ = state.persist_settings();
    });
    if !crate::ui::set_window_visible(window, false) {
        // Wayland does not expose a generic hide operation; minimizing is the safest fallback.
        window.minimize_window();
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

pub fn q_mark(size_px: f32) -> gpui::Div {
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
        let always_on_top = state.read(cx).settings.always_on_top;
        let shown = handle
            .update(cx, |_, window, _| {
                if !crate::ui::set_window_visible(window, true) {
                    return false;
                }
                crate::ui::apply_window_topmost(window, always_on_top);
                window.activate_window();
                true
            })
            .unwrap_or(false);
        if shown {
            return;
        }
        let _ = handle.update(cx, |_, window, _| window.remove_window());
        state.update(cx, |s, _| s.main_window = None);
        open_main_window(state, cx);
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
        s.settings.dock_edge = None;
        s.settings.dock_on_edge = false;
        s.settings.keep_full_main = true;
        let _ = s.persist_settings();
        crate::tray::update_labels(s);
    });
    let dock = state.update(cx, |s, _| s.dock_window.take());
    if let Some(dock) = dock {
        let _ = dock.update(cx, |_, window, _| {
            window.remove_window();
        });
    }
    if let Some(main) = state.read(cx).main_window {
        let always_on_top = state.read(cx).settings.always_on_top;
        let shown = main
            .update(cx, |_, window, _| {
                if !crate::ui::set_window_visible(window, true) {
                    return false;
                }
                crate::ui::apply_window_topmost(window, always_on_top);
                window.activate_window();
                true
            })
            .unwrap_or(false);
        if shown {
            return;
        }
        let _ = main.update(cx, |_, window, _| window.remove_window());
        state.update(cx, |s, _| s.main_window = None);
    }
    // Recover gracefully if the native window disappeared unexpectedly or the
    // platform only supports recreating a hidden window (for example Wayland).
    open_main_window(state, cx);
}

pub fn open_main_window_at_startup(state: Entity<AppState>, cx: &mut App) {
    open_main_window_inner(state, cx, true);
}

pub fn open_main_window(state: Entity<AppState>, cx: &mut App) {
    open_main_window_inner(state, cx, false);
}

fn open_main_window_inner(state: Entity<AppState>, cx: &mut App, startup: bool) {
    let settings = state.read(cx).settings.clone();
    let bounds = if startup {
        startup_main_bounds(&settings, cx)
    } else {
        restored_main_bounds(&settings, cx)
    };
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
    let Some(area) = display_area_for_bounds(candidate, cx) else {
        return gpui::Bounds::centered(None, size, cx);
    };
    let area_left = f32::from(area.origin.x);
    let area_top = f32::from(area.origin.y);
    let area_right = area_left + f32::from(area.size.width);
    let area_bottom = area_top + f32::from(area.size.height);
    let width = f32::from(size.width);
    let height = f32::from(size.height);
    let x = saved
        .x
        .clamp(area_left, (area_right - width).max(area_left));
    let y = saved
        .y
        .clamp(area_top, (area_bottom - height).max(area_top));
    gpui::Bounds {
        origin: gpui::point(px(x), px(y)),
        size,
    }
}

fn startup_main_bounds(
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
    let Some(display) = cx
        .primary_display()
        .or_else(|| cx.displays().into_iter().next())
    else {
        return gpui::Bounds::centered(None, size, cx);
    };
    let area = display.bounds();
    let left = f32::from(area.origin.x);
    let top = f32::from(area.origin.y);
    let right = left + f32::from(area.size.width);
    let bottom = top + f32::from(area.size.height);
    let width = f32::from(size.width);
    let height = f32::from(size.height);
    let x = (right - width - 40.0).clamp(left, (right - width).max(left));
    let y = (top + (bottom - top - height) / 2.0).clamp(top, (bottom - height).max(top));
    gpui::Bounds {
        origin: gpui::point(px(x), px(y)),
        size,
    }
}

fn display_area_for_bounds(
    bounds: gpui::Bounds<gpui::Pixels>,
    cx: &App,
) -> Option<gpui::Bounds<gpui::Pixels>> {
    let left = f32::from(bounds.origin.x);
    let top = f32::from(bounds.origin.y);
    let right = left + f32::from(bounds.size.width);
    let bottom = top + f32::from(bounds.size.height);
    let displays = cx.displays();
    let center_x = (left + right) / 2.;
    let center_y = (top + bottom) / 2.;
    displays
        .iter()
        .find(|display| {
            let area = display.bounds();
            let area_left = f32::from(area.origin.x);
            let area_top = f32::from(area.origin.y);
            let area_right = area_left + f32::from(area.size.width);
            let area_bottom = area_top + f32::from(area.size.height);
            center_x >= area_left
                && center_x <= area_right
                && center_y >= area_top
                && center_y <= area_bottom
        })
        .map(|display| display.bounds())
        .or_else(|| {
            displays.iter().find_map(|display| {
                let area = display.bounds();
                let area_left = f32::from(area.origin.x);
                let area_top = f32::from(area.origin.y);
                let area_right = area_left + f32::from(area.size.width);
                let area_bottom = area_top + f32::from(area.size.height);
                let visible_width = right.min(area_right) - left.max(area_left);
                let visible_height = bottom.min(area_bottom) - top.max(area_top);
                (visible_width >= 48.0 && visible_height >= 32.0).then_some(area)
            })
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
