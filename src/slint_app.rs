use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    io::Read as _,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

use slint::{
    Color, ComponentHandle as _, DataTransfer, LogicalPosition, LogicalSize, ModelRc,
    PhysicalPosition, SharedString, Timer, TimerMode, VecModel,
    winit_030::{EventResult, WinitWindowAccessor as _, winit},
};

use crate::{
    DockWindow, EditorWindow, MainWindow, UiAttachment, UiColor, UiNote, UiStrings,
    app_state::AppState,
    models::{
        AttachmentKind, AttachmentSource, DEFAULT_WINDOW_HEIGHT, DEFAULT_WINDOW_WIDTH,
        DOCK_WINDOW_SIZE, DockEdge, EDITOR_WINDOW_HEIGHT, EDITOR_WINDOW_WIDTH, NOTE_COLORS,
        NOTE_LINE_HEIGHT, Note, NoteAttachment, NoteDraft, PendingUpdateDraft, WindowState,
        parse_hex_color,
    },
};

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .select()?;

    let controller = Controller::new()?;
    controller.start()?;
    slint::run_event_loop_until_quit()?;
    Ok(())
}

const DOCK_DRAG_THRESHOLD: f32 = 4.0;
const DOCK_SNAP_THRESHOLD: f32 = 28.0;
const DOCK_ANIMATION_MS: f32 = 130.0;
const DOCK_ANIMATION_FRAME_MS: u64 = 13;
const DOCK_RETURN_DELAY_MS: u64 = 500;
const DOCK_ANCHOR_MAX_AGE: Duration = Duration::from_secs(5 * 60);

struct Controller {
    ui: MainWindow,
    state: RefCell<AppState>,
    notes: Rc<VecModel<UiNote>>,
    _palette: Rc<VecModel<UiColor>>,
    bounds_timer: Timer,
    startup_timer: Timer,
    toast_timer: Timer,
    editor_position_timer: Timer,
    editor_draft_timer: Timer,
    editor: RefCell<Option<EditorSession>>,
    background_tx: mpsc::Sender<BackgroundEvent>,
    background_rx: RefCell<mpsc::Receiver<BackgroundEvent>>,
    background_timer: Timer,
    daily_update_timer: Timer,
    update_revision: Cell<u64>,
    update_download: RefCell<Option<UpdateDownloadUi>>,
    remote_images: RefCell<HashMap<String, slint::Image>>,
    remote_pending: RefCell<HashSet<String>>,
    tray: RefCell<Option<crate::slint_tray::SlintTray>>,
    tray_timer: Timer,
    dock: RefCell<Option<DockSession>>,
    dock_animation_timer: Timer,
    dock_return_timer: Timer,
    dock_reveal_anchor: RefCell<Option<DockRevealAnchor>>,
}

struct EditorSession {
    ui: EditorWindow,
    note_id: Option<String>,
    attachments: Vec<NoteAttachment>,
    initial_draft: NoteDraft,
    images: Rc<VecModel<UiAttachment>>,
    files: Rc<VecModel<UiAttachment>>,
}

struct DockSession {
    ui: DockWindow,
    edge: Option<DockEdge>,
    area: WorkArea,
    revealed: bool,
    hovered: bool,
    dragging: bool,
    pointer_origin: Option<(f32, f32)>,
    animation: Option<DockAnimation>,
    suppress_click_until: Option<Instant>,
}

#[derive(Clone, Copy)]
struct WorkArea {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[derive(Clone, Copy)]
struct DockAnimation {
    start: PhysicalPosition,
    target: PhysicalPosition,
    started: Instant,
    area: WorkArea,
    revealed: bool,
}

#[derive(Clone, Copy)]
struct DockRevealAnchor {
    edge: DockEdge,
    position: PhysicalPosition,
    saved_at: Instant,
}

enum BackgroundEvent {
    UpdateChecked {
        manual: bool,
        result: Result<Option<crate::updater::UpdateInfo>, String>,
    },
    UpdateProgress {
        revision: u64,
        progress: crate::updater::DownloadProgress,
    },
    UpdateDownloaded {
        revision: u64,
        result: Result<crate::updater::DownloadedUpdate, String>,
    },
    UpdateInstalled {
        revision: u64,
        result: Result<(), String>,
    },
    RemoteImage {
        url: String,
        bytes: Option<Vec<u8>>,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum UpdatePhase {
    Downloading,
    Preparing,
    Installing,
}

struct UpdateDownloadUi {
    info: crate::updater::UpdateInfo,
    phase: UpdatePhase,
    progress: crate::updater::DownloadProgress,
    cancelled: Arc<AtomicBool>,
    revision: u64,
}

impl Controller {
    fn new() -> anyhow::Result<Rc<Self>> {
        let ui = MainWindow::new().map_err(|error| anyhow::anyhow!(error.to_string()))?;
        let notes = Rc::new(VecModel::default());
        let palette = Rc::new(VecModel::from(
            NOTE_COLORS
                .iter()
                .map(|hex| UiColor {
                    hex: (*hex).into(),
                    value: color_from_hex(hex),
                })
                .collect::<Vec<_>>(),
        ));
        ui.set_notes(ModelRc::from(notes.clone()));
        ui.set_palette(ModelRc::from(palette.clone()));
        let (background_tx, background_rx) = mpsc::channel();

        let controller = Rc::new(Self {
            ui,
            state: RefCell::new(AppState::new()?),
            notes,
            _palette: palette,
            bounds_timer: Timer::default(),
            startup_timer: Timer::default(),
            toast_timer: Timer::default(),
            editor_position_timer: Timer::default(),
            editor_draft_timer: Timer::default(),
            editor: RefCell::new(None),
            background_tx,
            background_rx: RefCell::new(background_rx),
            background_timer: Timer::default(),
            daily_update_timer: Timer::default(),
            update_revision: Cell::new(0),
            update_download: RefCell::new(None),
            remote_images: RefCell::new(HashMap::new()),
            remote_pending: RefCell::new(HashSet::new()),
            tray: RefCell::new(None),
            tray_timer: Timer::default(),
            dock: RefCell::new(None),
            dock_animation_timer: Timer::default(),
            dock_return_timer: Timer::default(),
            dock_reveal_anchor: RefCell::new(None),
        });
        controller.refresh();
        controller.wire_callbacks();
        Ok(controller)
    }

    fn start(self: &Rc<Self>) -> Result<(), slint::PlatformError> {
        self.ui.show()?;
        self.install_window_hooks();
        let labels = tray_labels(&self.state.borrow());
        *self.tray.borrow_mut() = crate::slint_tray::SlintTray::new(labels).ok();
        let weak = Rc::downgrade(self);
        self.tray_timer
            .start(TimerMode::Repeated, Duration::from_millis(50), move || {
                if let Some(controller) = weak.upgrade() {
                    controller.poll_tray();
                }
            });

        let weak = Rc::downgrade(self);
        self.startup_timer
            .start(TimerMode::SingleShot, Duration::from_millis(0), move || {
                if let Some(controller) = weak.upgrade() {
                    controller.place_main_at_startup();
                }
            });
        if self.state.borrow().editor_draft_recovery.is_some() {
            self.open_editor(None, true);
        }
        let weak = Rc::downgrade(self);
        self.background_timer
            .start(TimerMode::Repeated, Duration::from_millis(50), move || {
                if let Some(controller) = weak.upgrade() {
                    controller.process_background_events();
                }
            });
        self.run_update_check(false);
        self.schedule_daily_update();
        Ok(())
    }

    fn wire_callbacks(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        self.ui.on_start_window_drag(move || {
            if let Some(controller) = weak.upgrade() {
                controller.ui.window().with_winit_window(|window| {
                    let _ = window.drag_window();
                });
            }
        });

        let weak = Rc::downgrade(self);
        self.ui.on_toggle_topmost(move || {
            let Some(controller) = weak.upgrade() else {
                return;
            };
            let next = !controller.state.borrow().settings.always_on_top;
            if controller
                .state
                .borrow_mut()
                .set_always_on_top(next)
                .is_ok()
            {
                controller.refresh();
            }
        });

        let weak = Rc::downgrade(self);
        self.ui.on_minimize(move || {
            if let Some(controller) = weak.upgrade() {
                controller.ui.window().set_minimized(true);
            }
        });

        let weak = Rc::downgrade(self);
        self.ui.on_hide_window(move || {
            if let Some(controller) = weak.upgrade() {
                controller.hide_main();
            }
        });

        let weak = Rc::downgrade(self);
        self.ui.on_toggle_language(move || {
            let Some(controller) = weak.upgrade() else {
                return;
            };
            if controller.state.borrow_mut().toggle_language().is_ok() {
                controller.refresh();
            }
        });

        let weak = Rc::downgrade(self);
        self.ui.on_copy_note(move |id| {
            if let Some(controller) = weak.upgrade() {
                controller.copy_note(id.as_str());
            }
        });

        self.ui
            .on_make_note_drag_data(|id| DataTransfer::from(SharedString::from(id.as_str())));

        let weak = Rc::downgrade(self);
        self.ui.on_reorder_note(move |data, target_id, after| {
            let Some(controller) = weak.upgrade() else {
                return;
            };
            let Ok(dragged_id) = data.plain_text() else {
                return;
            };
            if controller
                .state
                .borrow_mut()
                .reorder_note(dragged_id.as_str(), target_id.as_str(), after)
                .is_ok()
            {
                controller.refresh();
            }
        });

        let weak = Rc::downgrade(self);
        self.ui.on_toggle_pin(move |id| {
            let Some(controller) = weak.upgrade() else {
                return;
            };
            if controller
                .state
                .borrow_mut()
                .toggle_pin(id.as_str())
                .is_ok()
            {
                controller.refresh();
            }
        });

        let weak = Rc::downgrade(self);
        self.ui.on_change_color(move |id, color| {
            let Some(controller) = weak.upgrade() else {
                return;
            };
            if controller
                .state
                .borrow_mut()
                .patch_note(id.as_str(), |note| note.color = color.to_string())
                .is_ok()
            {
                controller.refresh();
            }
        });

        let weak = Rc::downgrade(self);
        self.ui.on_delete_note(move |id| {
            let Some(controller) = weak.upgrade() else {
                return;
            };
            if controller
                .state
                .borrow_mut()
                .delete_note(id.as_str())
                .is_ok()
            {
                controller.refresh();
            }
        });

        let weak = Rc::downgrade(self);
        self.ui.on_resize_note(move |id, requested| {
            let Some(controller) = weak.upgrade() else {
                return;
            };
            let (minimum, maximum) = controller
                .state
                .borrow()
                .note_by_id(id.as_str())
                .map(|note| (default_note_height(note), full_note_height(note)))
                .unwrap_or((NOTE_LINE_HEIGHT as i64, NOTE_LINE_HEIGHT as i64));
            let snapped = ((requested as f32 / NOTE_LINE_HEIGHT).ceil() * NOTE_LINE_HEIGHT) as i64;
            if controller
                .state
                .borrow_mut()
                .set_note_height(id.as_str(), snapped.clamp(minimum, maximum.max(minimum)))
                .is_ok()
            {
                controller.refresh();
            }
        });

        let weak = Rc::downgrade(self);
        self.ui.on_request_delete_all(move || {
            if let Some(controller) = weak.upgrade() {
                controller.ui.set_modal_kind(1);
            }
        });

        let weak = Rc::downgrade(self);
        self.ui.on_confirm_delete_all(move || {
            let Some(controller) = weak.upgrade() else {
                return;
            };
            if controller.state.borrow_mut().delete_all_notes().is_ok() {
                controller.refresh();
            }
        });

        let weak = Rc::downgrade(self);
        self.ui.on_new_note(move || {
            if let Some(controller) = weak.upgrade() {
                controller.open_editor(None, false);
            }
        });

        let weak = Rc::downgrade(self);
        self.ui.on_edit_note(move |id| {
            if let Some(controller) = weak.upgrade() {
                controller.open_editor(Some(id.to_string()), false);
            }
        });

        let weak = Rc::downgrade(self);
        self.ui.on_open_settings(move || {
            if let Some(controller) = weak.upgrade() {
                controller.ui.set_modal_kind(2);
            }
        });

        let weak = Rc::downgrade(self);
        self.ui.on_toggle_auto_start(move || {
            let Some(controller) = weak.upgrade() else {
                return;
            };
            let enabled = !controller.state.borrow().settings.auto_start;
            let result = controller.state.borrow_mut().set_auto_start(enabled);
            let message = if result.is_ok() {
                controller.state.borrow().tr().auto_start_updated
            } else {
                controller.state.borrow().tr().auto_start_failed
            };
            controller.refresh();
            controller.show_toast(message.to_string());
        });

        let weak = Rc::downgrade(self);
        self.ui.on_export_data(move || {
            if let Some(controller) = weak.upgrade() {
                controller.export_data();
            }
        });

        let weak = Rc::downgrade(self);
        self.ui.on_import_data(move || {
            if let Some(controller) = weak.upgrade() {
                controller.import_data();
            }
        });

        self.ui.on_open_version(|| {
            crate::updater::open_release_page(Some(crate::updater::PACKAGE_VERSION));
        });

        let weak = Rc::downgrade(self);
        self.ui.on_check_update(move || {
            if let Some(controller) = weak.upgrade() {
                controller.handle_manual_update_check();
            }
        });

        let weak = Rc::downgrade(self);
        self.ui.on_confirm_update(move || {
            if let Some(controller) = weak.upgrade() {
                controller.confirm_update();
            }
        });

        let weak = Rc::downgrade(self);
        self.ui.on_cancel_update_download(move || {
            if let Some(controller) = weak.upgrade() {
                controller.cancel_update_download();
            }
        });

        let weak = Rc::downgrade(self);
        self.ui.on_collapse_to_dock(move || {
            if let Some(controller) = weak.upgrade() {
                controller.collapse_to_dock();
            }
        });
        let weak = Rc::downgrade(self);
        self.ui.on_preview_image(move |note_id, attachment_id| {
            if let Some(controller) = weak.upgrade() {
                controller.open_main_preview(note_id.as_str(), attachment_id.as_str());
            }
        });

        let weak = Rc::downgrade(self);
        self.ui.window().on_close_requested(move || {
            if let Some(controller) = weak.upgrade() {
                controller.hide_main();
            }
            slint::CloseRequestResponse::KeepWindowShown
        });
    }

    fn open_editor(self: &Rc<Self>, requested_note_id: Option<String>, from_recovery: bool) {
        if self.state.borrow().settings.docked {
            self.restore_from_dock();
        }
        if self.editor.borrow().is_some() && !self.discard_editor() {
            return;
        }

        let (recovery, note_id, initial_draft, draft) = {
            let state = self.state.borrow();
            let recovery = from_recovery
                .then(|| state.editor_draft_recovery.clone())
                .flatten();
            let note_id = recovery
                .as_ref()
                .and_then(|pending| pending.note_id.clone())
                .or(requested_note_id);
            let initial = note_id
                .as_ref()
                .and_then(|id| state.note_by_id(id))
                .map(|note| NoteDraft {
                    attachments: note.attachments.clone(),
                    color: note.color.clone(),
                    content: note.content.clone(),
                    pinned: note.pinned,
                })
                .unwrap_or_default();
            let draft = recovery
                .as_ref()
                .map(|pending| pending.draft.clone())
                .unwrap_or_else(|| initial.clone());
            (recovery, note_id, initial, draft)
        };

        if from_recovery {
            let mut state = self.state.borrow_mut();
            state.editor_draft_recovery = None;
            state.editor_recovery_active = recovery.is_some();
        }

        let Ok(ui) = EditorWindow::new() else {
            self.show_toast(self.state.borrow().tr().save_failed.to_string());
            return;
        };
        let images = Rc::new(VecModel::default());
        let files = Rc::new(VecModel::default());
        ui.set_palette(ModelRc::from(self._palette.clone()));
        ui.set_images(ModelRc::from(images.clone()));
        ui.set_files(ModelRc::from(files.clone()));
        ui.set_note_id(note_id.clone().unwrap_or_default().into());
        ui.set_content_text(draft.content.clone().into());
        ui.set_selected_color(draft.color.clone().into());
        ui.set_pinned(draft.pinned);
        ui.set_topmost(self.state.borrow().settings.always_on_top);
        ui.set_strings(strings_to_ui(self.state.borrow().tr()));

        *self.editor.borrow_mut() = Some(EditorSession {
            ui,
            note_id,
            attachments: draft.attachments,
            initial_draft,
            images,
            files,
        });
        self.refresh_editor_models();
        self.wire_editor_callbacks();

        let show_result = self
            .editor
            .borrow()
            .as_ref()
            .expect("editor session")
            .ui
            .show();
        if show_result.is_err() {
            self.editor.borrow_mut().take();
            self.show_toast(self.state.borrow().tr().save_failed.to_string());
            return;
        }

        let weak = Rc::downgrade(self);
        self.editor_position_timer.start(
            TimerMode::SingleShot,
            Duration::from_millis(0),
            move || {
                if let Some(controller) = weak.upgrade() {
                    controller.place_editor_window();
                }
            },
        );
        self.sync_editor_draft();
    }

    fn wire_editor_callbacks(self: &Rc<Self>) {
        let editor = self.editor.borrow();
        let Some(session) = editor.as_ref() else {
            return;
        };
        let ui = &session.ui;

        let weak = Rc::downgrade(self);
        ui.on_start_window_drag(move || {
            if let Some(controller) = weak.upgrade()
                && let Some(editor) = controller.editor.borrow().as_ref()
            {
                editor.ui.window().with_winit_window(|window| {
                    let _ = window.drag_window();
                });
            }
        });

        let weak = Rc::downgrade(self);
        ui.on_toggle_topmost(move || {
            let Some(controller) = weak.upgrade() else {
                return;
            };
            let next = !controller.state.borrow().settings.always_on_top;
            if controller
                .state
                .borrow_mut()
                .set_always_on_top(next)
                .is_ok()
            {
                controller.refresh();
            }
        });

        let weak = Rc::downgrade(self);
        ui.on_minimize(move || {
            if let Some(controller) = weak.upgrade()
                && let Some(editor) = controller.editor.borrow().as_ref()
            {
                editor.ui.window().set_minimized(true);
            }
        });

        let weak = Rc::downgrade(self);
        ui.on_close_editor(move || {
            if let Some(controller) = weak.upgrade() {
                controller.discard_editor();
            }
        });

        let weak = Rc::downgrade(self);
        ui.on_save_editor(move || {
            if let Some(controller) = weak.upgrade() {
                controller.save_editor();
            }
        });

        let weak = Rc::downgrade(self);
        ui.on_pick_images(move || {
            if let Some(controller) = weak.upgrade() {
                controller.pick_editor_images();
            }
        });

        let weak = Rc::downgrade(self);
        ui.on_add_media(move |value| {
            if let Some(controller) = weak.upgrade() {
                controller.add_editor_media(value.as_str());
            }
        });

        let weak = Rc::downgrade(self);
        ui.on_remove_attachment(move |id| {
            if let Some(controller) = weak.upgrade() {
                controller.remove_editor_attachment(id.as_str());
            }
        });

        let weak = Rc::downgrade(self);
        ui.on_paste_image(move || {
            weak.upgrade()
                .is_some_and(|controller| controller.paste_editor_image())
        });

        let weak = Rc::downgrade(self);
        ui.on_draft_changed(move || {
            if let Some(controller) = weak.upgrade() {
                controller.sync_editor_draft();
            }
        });

        let weak = Rc::downgrade(self);
        ui.on_preview_image(move |attachment_id| {
            if let Some(controller) = weak.upgrade() {
                controller.open_editor_preview(attachment_id.as_str());
            }
        });

        let weak = Rc::downgrade(self);
        ui.window().on_close_requested(move || {
            if let Some(controller) = weak.upgrade() {
                controller.discard_editor();
            }
            slint::CloseRequestResponse::KeepWindowShown
        });

        let weak = Rc::downgrade(self);
        ui.window().on_winit_window_event(move |_, event| {
            let Some(controller) = weak.upgrade() else {
                return EventResult::Propagate;
            };
            match event {
                winit::event::WindowEvent::HoveredFile(_) => {
                    if let Some(editor) = controller.editor.borrow().as_ref() {
                        editor.ui.set_drop_active(true);
                    }
                }
                winit::event::WindowEvent::HoveredFileCancelled => {
                    if let Some(editor) = controller.editor.borrow().as_ref() {
                        editor.ui.set_drop_active(false);
                    }
                }
                winit::event::WindowEvent::DroppedFile(path) => {
                    if let Some(editor) = controller.editor.borrow().as_ref() {
                        editor.ui.set_drop_active(false);
                    }
                    controller.append_editor_paths(std::slice::from_ref(path));
                }
                _ => {}
            }
            EventResult::Propagate
        });
    }

    fn current_editor_draft(&self) -> Option<NoteDraft> {
        let editor = self.editor.borrow();
        let session = editor.as_ref()?;
        Some(NoteDraft {
            attachments: session.attachments.clone(),
            color: session.ui.get_selected_color().to_string(),
            content: session.ui.get_content_text().to_string(),
            pinned: session.ui.get_pinned(),
        })
    }

    fn sync_editor_draft(self: &Rc<Self>) {
        let Some(draft) = self.current_editor_draft() else {
            return;
        };
        let (note_id, initial) = {
            let editor = self.editor.borrow();
            let Some(session) = editor.as_ref() else {
                return;
            };
            (session.note_id.clone(), session.initial_draft.clone())
        };
        let pending = (draft != initial).then(|| PendingUpdateDraft {
            note_id,
            draft,
            saved_at: crate::app_state::now_ms(),
        });
        self.state.borrow_mut().set_editor_draft(pending);

        if !self.state.borrow().editor_recovery_active {
            return;
        }
        let weak = Rc::downgrade(self);
        self.editor_draft_timer.start(
            TimerMode::SingleShot,
            Duration::from_millis(250),
            move || {
                if let Some(controller) = weak.upgrade() {
                    let _ = controller.state.borrow().persist_editor_draft();
                }
            },
        );
    }

    fn save_editor(&self) {
        let Some(mut draft) = self.current_editor_draft() else {
            return;
        };
        draft.content = draft.content.trim().to_string();
        if draft.content.is_empty() && draft.attachments.is_empty() {
            self.discard_editor();
            return;
        }
        let note_id = self
            .editor
            .borrow()
            .as_ref()
            .and_then(|session| session.note_id.clone());
        let result = {
            let mut state = self.state.borrow_mut();
            state.upsert_from_draft(note_id, draft).and_then(|_| {
                state.set_editor_draft(None);
                state.clear_editor_session()
            })
        };
        match result {
            Ok(()) => {
                self.take_editor_window();
                self.refresh();
                self.show_toast(self.state.borrow().tr().saved.to_string());
            }
            Err(_) => self.show_toast(self.state.borrow().tr().save_failed.to_string()),
        }
    }

    fn discard_editor(&self) -> bool {
        if self.editor.borrow().is_none() {
            return true;
        }
        if self.state.borrow_mut().clear_editor_session().is_err() {
            self.show_toast(self.state.borrow().tr().save_failed.to_string());
            return false;
        }
        self.take_editor_window();
        true
    }

    fn take_editor_window(&self) {
        if let Some(session) = self.editor.borrow_mut().take() {
            let _ = session.ui.hide();
        }
    }

    fn pick_editor_images(self: &Rc<Self>) {
        let Some(paths) = rfd::FileDialog::new()
            .add_filter(
                "Images",
                &["png", "jpg", "jpeg", "gif", "webp", "bmp", "svg"],
            )
            .pick_files()
        else {
            return;
        };
        let attachments = paths
            .into_iter()
            .filter_map(|path| {
                let value = crate::slint_media::file_to_data_url(&path).ok()?;
                Some(NoteAttachment {
                    id: crate::app_state::create_id("asset"),
                    kind: AttachmentKind::Image,
                    source: AttachmentSource::Data,
                    value,
                    name: path
                        .file_name()
                        .map(|name| name.to_string_lossy().to_string()),
                    created_at: crate::app_state::now_ms(),
                })
            })
            .collect::<Vec<_>>();
        if attachments.is_empty() {
            return;
        }
        if let Some(editor) = self.editor.borrow_mut().as_mut() {
            editor.attachments.extend(attachments);
        }
        self.refresh_editor_models();
        self.sync_editor_draft();
    }

    fn add_editor_media(self: &Rc<Self>, raw: &str) {
        let value = raw.trim();
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
        let kind = if crate::models::is_likely_image_path(value) {
            AttachmentKind::Image
        } else {
            AttachmentKind::File
        };
        let attachment = NoteAttachment {
            id: crate::app_state::create_id("asset"),
            kind,
            source,
            value: value.to_string(),
            name: attachment_name_from_value(value),
            created_at: crate::app_state::now_ms(),
        };
        if let Some(editor) = self.editor.borrow_mut().as_mut() {
            editor.attachments.push(attachment);
            editor.ui.set_media_value("".into());
        }
        self.refresh_editor_models();
        self.sync_editor_draft();
    }

    fn append_editor_paths(self: &Rc<Self>, paths: &[std::path::PathBuf]) {
        let attachments = paths
            .iter()
            .map(|path| {
                let value = path.to_string_lossy().to_string();
                NoteAttachment {
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
                }
            })
            .collect::<Vec<_>>();
        if let Some(editor) = self.editor.borrow_mut().as_mut() {
            editor.attachments.extend(attachments);
        }
        self.refresh_editor_models();
        self.sync_editor_draft();
    }

    fn remove_editor_attachment(self: &Rc<Self>, id: &str) {
        if let Some(editor) = self.editor.borrow_mut().as_mut() {
            editor.attachments.retain(|attachment| attachment.id != id);
        }
        self.refresh_editor_models();
        self.sync_editor_draft();
    }

    fn paste_editor_image(self: &Rc<Self>) -> bool {
        let Ok(mut clipboard) = arboard::Clipboard::new() else {
            return false;
        };
        let Ok(image) = clipboard.get_image() else {
            return false;
        };
        let Ok(value) = crate::slint_media::rgba_bytes_to_data_url(
            image.width as u32,
            image.height as u32,
            image.bytes.as_ref(),
        ) else {
            return false;
        };
        let attachment = NoteAttachment {
            id: crate::app_state::create_id("asset"),
            kind: AttachmentKind::Image,
            source: AttachmentSource::Data,
            value,
            name: Some(self.state.borrow().tr().add_image.to_string()),
            created_at: crate::app_state::now_ms(),
        };
        if let Some(editor) = self.editor.borrow_mut().as_mut() {
            editor.attachments.push(attachment);
        }
        self.refresh_editor_models();
        self.sync_editor_draft();
        true
    }

    fn ensure_remote_images(&self, urls: Vec<String>) {
        for url in urls {
            if self.remote_images.borrow().contains_key(&url)
                || self.remote_pending.borrow().contains(&url)
            {
                continue;
            }
            self.remote_pending.borrow_mut().insert(url.clone());
            let sender = self.background_tx.clone();
            let _ = std::thread::Builder::new()
                .name("q-note-image".into())
                .spawn(move || {
                    let bytes = (|| -> Option<Vec<u8>> {
                        let mut response = ureq::get(&url).call().ok()?;
                        let mut reader = response.body_mut().as_reader().take(24 * 1024 * 1024);
                        let mut bytes = Vec::new();
                        reader.read_to_end(&mut bytes).ok()?;
                        Some(bytes)
                    })();
                    let _ = sender.send(BackgroundEvent::RemoteImage { url, bytes });
                });
        }
    }

    fn refresh_editor_models(&self) {
        let remote_urls = self
            .editor
            .borrow()
            .as_ref()
            .map(|session| {
                session
                    .attachments
                    .iter()
                    .filter(|attachment| {
                        attachment.kind == AttachmentKind::Image
                            && attachment.source == AttachmentSource::Url
                    })
                    .map(|attachment| attachment.value.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        self.ensure_remote_images(remote_urls);
        let cache = self.remote_images.borrow();
        let editor = self.editor.borrow();
        let Some(session) = editor.as_ref() else {
            return;
        };
        session.images.set_vec(
            session
                .attachments
                .iter()
                .filter(|attachment| attachment.kind == AttachmentKind::Image)
                .map(|attachment| attachment_to_ui(attachment, &cache))
                .collect::<Vec<_>>(),
        );
        session.files.set_vec(
            session
                .attachments
                .iter()
                .filter(|attachment| attachment.kind == AttachmentKind::File)
                .map(|attachment| attachment_to_ui(attachment, &cache))
                .collect::<Vec<_>>(),
        );
    }

    fn open_main_preview(&self, note_id: &str, attachment_id: &str) {
        let state = self.state.borrow();
        let Some(note) = state.note_by_id(note_id) else {
            return;
        };
        let images = note
            .attachments
            .iter()
            .filter(|attachment| attachment.kind == AttachmentKind::Image)
            .collect::<Vec<_>>();
        let Some(index) = images
            .iter()
            .position(|attachment| attachment.id == attachment_id)
        else {
            return;
        };
        let cache = self.remote_images.borrow();
        let model = images
            .into_iter()
            .map(|attachment| attachment_to_ui(attachment, &cache))
            .collect::<Vec<_>>();
        self.ui
            .set_preview_images(ModelRc::new(VecModel::from(model)));
        self.ui.set_preview_index(index as i32);
        self.ui.set_preview_open(true);
    }

    fn open_editor_preview(&self, attachment_id: &str) {
        let editor = self.editor.borrow();
        let Some(session) = editor.as_ref() else {
            return;
        };
        let images = session
            .attachments
            .iter()
            .filter(|attachment| attachment.kind == AttachmentKind::Image)
            .collect::<Vec<_>>();
        let Some(index) = images
            .iter()
            .position(|attachment| attachment.id == attachment_id)
        else {
            return;
        };
        session.ui.set_preview_index(index as i32);
        session.ui.set_preview_open(true);
    }

    fn place_editor_window(&self) {
        let editor = self.editor.borrow();
        let Some(session) = editor.as_ref() else {
            return;
        };
        session
            .ui
            .window()
            .set_size(LogicalSize::new(EDITOR_WINDOW_WIDTH, EDITOR_WINDOW_HEIGHT));
        let main_position = self.ui.window().position();
        let main_size = self.ui.window().size();
        let scale = session.ui.window().scale_factor().max(1.0);
        let target_x = main_position.x as f32 - EDITOR_WINDOW_WIDTH * scale - 12.0 * scale;
        let target_y =
            main_position.y as f32 + (main_size.height as f32 - EDITOR_WINDOW_HEIGHT * scale) / 2.0;
        let (x, y) = session
            .ui
            .window()
            .with_winit_window(|window| {
                let Some(monitor) = window
                    .current_monitor()
                    .or_else(|| window.primary_monitor())
                else {
                    return (target_x, target_y);
                };
                let origin = monitor.position();
                let size = monitor.size();
                let min_x = origin.x as f32;
                let min_y = origin.y as f32;
                let max_x = min_x + size.width as f32 - EDITOR_WINDOW_WIDTH * scale;
                let max_y = min_y + size.height as f32 - EDITOR_WINDOW_HEIGHT * scale;
                (
                    target_x.clamp(min_x, max_x.max(min_x)),
                    target_y.clamp(min_y, max_y.max(min_y)),
                )
            })
            .unwrap_or((target_x, target_y));
        session
            .ui
            .window()
            .set_position(PhysicalPosition::new(x.round() as i32, y.round() as i32));
    }

    fn install_window_hooks(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        self.ui.window().on_winit_window_event(move |_, event| {
            if matches!(
                event,
                winit::event::WindowEvent::Moved(_) | winit::event::WindowEvent::Resized(_)
            ) && let Some(controller) = weak.upgrade()
            {
                controller.schedule_bounds_save();
            }
            EventResult::Propagate
        });
    }

    fn poll_tray(self: &Rc<Self>) {
        let commands = self
            .tray
            .borrow()
            .as_ref()
            .map(crate::slint_tray::SlintTray::poll)
            .unwrap_or_default();
        for command in commands {
            match command {
                crate::slint_tray::TrayCommand::ShowMain => self.show_main(),
                crate::slint_tray::TrayCommand::ToggleTopmost => {
                    let next = !self.state.borrow().settings.always_on_top;
                    if self.state.borrow_mut().set_always_on_top(next).is_ok() {
                        self.refresh();
                    }
                }
                crate::slint_tray::TrayCommand::ToggleLanguage => {
                    if self.state.borrow_mut().toggle_language().is_ok() {
                        self.refresh();
                    }
                }
                crate::slint_tray::TrayCommand::ToggleDock => {
                    if self.state.borrow().settings.docked {
                        self.restore_from_dock();
                    } else {
                        self.collapse_to_dock();
                    }
                }
                crate::slint_tray::TrayCommand::Quit => self.quit(),
            }
        }
    }

    fn show_main(&self) {
        if self.state.borrow().settings.docked {
            self.restore_from_dock();
            return;
        }
        let _ = self.ui.show();
        self.ui.window().set_minimized(false);
        self.ui.window().with_winit_window(|window| {
            window.focus_window();
        });
    }

    fn collapse_to_dock(self: &Rc<Self>) {
        if self.state.borrow().settings.docked {
            return;
        }
        self.persist_main_bounds();
        let area = window_work_area(self.ui.window()).unwrap_or(WorkArea {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        });
        let scale = self.ui.window().scale_factor().max(1.0);
        let size = (DOCK_WINDOW_SIZE * scale).round().max(1.0) as i32;
        let main_position = self.ui.window().position();
        let main_size = self.ui.window().size();
        let center = PhysicalPosition::new(
            main_position.x + main_size.width as i32 / 2 - size / 2,
            main_position.y + main_size.height as i32 / 2 - size / 2,
        );
        let anchor = self
            .dock_reveal_anchor
            .borrow_mut()
            .take()
            .filter(|anchor| anchor.saved_at.elapsed() <= DOCK_ANCHOR_MAX_AGE);
        let (edge, origin) = if let Some(anchor) = anchor {
            (
                Some(anchor.edge),
                visible_edge_origin(anchor.edge, anchor.position, area, size),
            )
        } else {
            (None, clamp_free_dock_origin(center, area, size))
        };

        let Ok(ui) = DockWindow::new() else {
            return;
        };
        ui.set_topmost(self.state.borrow().settings.always_on_top);
        ui.set_strings(strings_to_ui(self.state.borrow().tr()));
        ui.window()
            .set_size(LogicalSize::new(DOCK_WINDOW_SIZE, DOCK_WINDOW_SIZE));
        ui.window().set_position(origin);
        *self.dock.borrow_mut() = Some(DockSession {
            ui,
            edge,
            area,
            revealed: true,
            hovered: false,
            dragging: false,
            pointer_origin: None,
            animation: None,
            suppress_click_until: None,
        });
        self.wire_dock_callbacks();
        if let Some(dock) = self.dock.borrow().as_ref() {
            let _ = dock.ui.show();
            dock.ui.window().set_position(origin);
            clear_dock_clip(&dock.ui);
        }

        {
            let mut state = self.state.borrow_mut();
            state.settings.docked = true;
            state.settings.dock_edge = edge;
            state.settings.dock_on_edge = edge.is_some();
            state.settings.keep_full_main = false;
            let _ = state.persist_settings();
        }
        let _ = self.ui.hide();
        self.refresh();

        if edge.is_some() {
            let weak = Rc::downgrade(self);
            self.dock_return_timer.start(
                TimerMode::SingleShot,
                Duration::from_millis(DOCK_RETURN_DELAY_MS),
                move || {
                    if let Some(controller) = weak.upgrade() {
                        let hovered = controller
                            .dock
                            .borrow()
                            .as_ref()
                            .is_some_and(|dock| dock.hovered);
                        if !hovered {
                            controller.set_dock_revealed(false);
                        }
                    }
                },
            );
        }
    }

    fn wire_dock_callbacks(self: &Rc<Self>) {
        let dock = self.dock.borrow();
        let Some(session) = dock.as_ref() else {
            return;
        };
        let ui = &session.ui;

        let weak = Rc::downgrade(self);
        ui.on_pointer_down(move |x, y| {
            if let Some(controller) = weak.upgrade() {
                controller.dock_pointer_down(x, y);
            }
        });
        let weak = Rc::downgrade(self);
        ui.on_pointer_move(move |x, y| {
            if let Some(controller) = weak.upgrade() {
                controller.dock_pointer_move(x, y);
            }
        });
        let weak = Rc::downgrade(self);
        ui.on_pointer_up(move || {
            if let Some(controller) = weak.upgrade() {
                controller.dock_pointer_up();
            }
        });
        let weak = Rc::downgrade(self);
        ui.on_clicked(move || {
            if let Some(controller) = weak.upgrade() {
                let suppressed = controller
                    .dock
                    .borrow()
                    .as_ref()
                    .and_then(|dock| dock.suppress_click_until)
                    .is_some_and(|until| Instant::now() < until);
                if !suppressed {
                    controller.restore_from_dock();
                }
            }
        });
        let weak = Rc::downgrade(self);
        ui.on_hover_changed(move |hovered| {
            if let Some(controller) = weak.upgrade() {
                controller.dock_hover_changed(hovered);
            }
        });
        let weak = Rc::downgrade(self);
        ui.on_toggle_topmost(move || {
            let Some(controller) = weak.upgrade() else {
                return;
            };
            let next = !controller.state.borrow().settings.always_on_top;
            if controller
                .state
                .borrow_mut()
                .set_always_on_top(next)
                .is_ok()
            {
                controller.refresh();
            }
        });
        let weak = Rc::downgrade(self);
        ui.on_toggle_language(move || {
            if let Some(controller) = weak.upgrade()
                && controller.state.borrow_mut().toggle_language().is_ok()
            {
                controller.refresh();
            }
        });
        let weak = Rc::downgrade(self);
        ui.on_restore_main(move || {
            if let Some(controller) = weak.upgrade() {
                controller.restore_from_dock();
            }
        });
        let weak = Rc::downgrade(self);
        ui.on_quit(move || {
            if let Some(controller) = weak.upgrade() {
                controller.quit();
            }
        });
        let weak = Rc::downgrade(self);
        ui.window().on_close_requested(move || {
            if let Some(controller) = weak.upgrade()
                && let Some(dock) = controller.dock.borrow().as_ref()
            {
                let _ = dock.ui.hide();
            }
            slint::CloseRequestResponse::KeepWindowShown
        });
        let weak = Rc::downgrade(self);
        ui.window().on_winit_window_event(move |_, event| {
            if let winit::event::WindowEvent::MouseInput {
                state: winit::event::ElementState::Released,
                button: winit::event::MouseButton::Left,
                ..
            } = event
                && let Some(controller) = weak.upgrade()
            {
                controller.dock_pointer_up();
            }
            EventResult::Propagate
        });
    }

    fn dock_pointer_down(&self, x: f32, y: f32) {
        self.dock_animation_timer.stop();
        let mut dock = self.dock.borrow_mut();
        let Some(session) = dock.as_mut() else {
            return;
        };
        session.animation = None;
        session.pointer_origin = Some((x, y));
        session.dragging = false;
        session.suppress_click_until = None;
        if let Some(edge) = session.edge {
            let size = session.ui.window().size().width as i32;
            let visible =
                visible_edge_origin(edge, session.ui.window().position(), session.area, size);
            session.ui.window().set_position(visible);
            session.revealed = true;
        }
        clear_dock_clip(&session.ui);
    }

    fn dock_pointer_move(&self, x: f32, y: f32) {
        let mut dock = self.dock.borrow_mut();
        let Some(session) = dock.as_mut() else {
            return;
        };
        let Some((start_x, start_y)) = session.pointer_origin else {
            return;
        };
        if session.dragging
            || (x - start_x).powi(2) + (y - start_y).powi(2) < DOCK_DRAG_THRESHOLD.powi(2)
        {
            return;
        }
        session.dragging = true;
        session.suppress_click_until = Some(Instant::now() + Duration::from_millis(600));
        session.edge = None;
        clear_dock_clip(&session.ui);
        session.ui.window().with_winit_window(|window| {
            let _ = window.drag_window();
        });
    }

    fn dock_pointer_up(self: &Rc<Self>) {
        let was_dragging = {
            let mut dock = self.dock.borrow_mut();
            let Some(session) = dock.as_mut() else {
                return;
            };
            session.pointer_origin = None;
            let dragging = session.dragging;
            session.dragging = false;
            dragging
        };
        if was_dragging {
            self.finish_dock_move();
        }
    }

    fn finish_dock_move(self: &Rc<Self>) {
        let (edge, hovered) = {
            let mut dock = self.dock.borrow_mut();
            let Some(session) = dock.as_mut() else {
                return;
            };
            session.area = window_work_area(session.ui.window()).unwrap_or(session.area);
            let position = session.ui.window().position();
            let size = session.ui.window().size().width as i32;
            let scale = session.ui.window().scale_factor().max(1.0);
            let edge = detect_dock_edge(position, session.area, size, scale);
            session.edge = edge;
            session.revealed = true;
            session.animation = None;
            if let Some(edge) = edge {
                let visible = visible_edge_origin(edge, position, session.area, size);
                session.ui.window().set_position(visible);
                clear_dock_clip(&session.ui);
            } else {
                let origin = clamp_free_dock_origin(position, session.area, size);
                session.ui.window().set_position(origin);
                clear_dock_clip(&session.ui);
            }
            (edge, session.hovered)
        };
        {
            let mut state = self.state.borrow_mut();
            state.settings.dock_edge = edge;
            state.settings.dock_on_edge = edge.is_some();
            let _ = state.persist_settings();
        }
        self.refresh();
        if edge.is_some() && !hovered {
            self.set_dock_revealed(false);
        }
    }

    fn dock_hover_changed(self: &Rc<Self>, hovered: bool) {
        let should_move = {
            let mut dock = self.dock.borrow_mut();
            let Some(session) = dock.as_mut() else {
                return;
            };
            session.hovered = hovered;
            !session.dragging && session.edge.is_some()
        };
        if should_move {
            self.set_dock_revealed(hovered);
        }
    }

    fn set_dock_revealed(self: &Rc<Self>, revealed: bool) {
        let animation = {
            let mut dock = self.dock.borrow_mut();
            let Some(session) = dock.as_mut() else {
                return;
            };
            if session.dragging || session.edge.is_none() {
                return;
            }
            if session.revealed == revealed && session.animation.is_none() {
                return;
            }
            let edge = session.edge.expect("checked edge");
            let start = session.ui.window().position();
            let size = session.ui.window().size().width as i32;
            let visible = visible_edge_origin(edge, start, session.area, size);
            let target = if revealed {
                visible
            } else {
                hidden_edge_origin(edge, visible, size)
            };
            session.revealed = revealed;
            DockAnimation {
                start,
                target,
                started: Instant::now(),
                area: session.area,
                revealed,
            }
        };
        if animation.start == animation.target {
            if let Some(session) = self.dock.borrow_mut().as_mut() {
                session.animation = None;
                apply_dock_clip(
                    &session.ui,
                    animation.target,
                    animation.area,
                    session.ui.window().size().width as i32,
                );
            }
            return;
        }
        if let Some(session) = self.dock.borrow_mut().as_mut() {
            session.animation = Some(animation);
        }
        let weak = Rc::downgrade(self);
        self.dock_animation_timer.start(
            TimerMode::Repeated,
            Duration::from_millis(DOCK_ANIMATION_FRAME_MS),
            move || {
                if let Some(controller) = weak.upgrade() {
                    controller.step_dock_animation();
                }
            },
        );
    }

    fn step_dock_animation(&self) {
        let mut finished = false;
        {
            let mut dock = self.dock.borrow_mut();
            let Some(session) = dock.as_mut() else {
                self.dock_animation_timer.stop();
                return;
            };
            let Some(animation) = session.animation else {
                self.dock_animation_timer.stop();
                return;
            };
            let progress = (animation.started.elapsed().as_secs_f32() * 1000.0 / DOCK_ANIMATION_MS)
                .clamp(0.0, 1.0);
            let eased = 1.0 - (1.0 - progress).powi(3);
            let origin = PhysicalPosition::new(
                animation.start.x
                    + ((animation.target.x - animation.start.x) as f32 * eased).round() as i32,
                animation.start.y
                    + ((animation.target.y - animation.start.y) as f32 * eased).round() as i32,
            );
            session.ui.window().set_position(origin);
            apply_dock_clip(
                &session.ui,
                origin,
                animation.area,
                session.ui.window().size().width as i32,
            );
            if progress >= 1.0 {
                session.animation = None;
                finished = true;
                if animation.revealed {
                    clear_dock_clip(&session.ui);
                }
            }
        }
        if finished {
            self.dock_animation_timer.stop();
        }
    }

    fn restore_from_dock(&self) {
        if !self.state.borrow().settings.docked {
            return;
        }
        self.dock_animation_timer.stop();
        self.dock_return_timer.stop();
        if let Some(session) = self.dock.borrow_mut().take() {
            if let Some(edge) = session.edge {
                let size = session.ui.window().size().width as i32;
                let visible =
                    visible_edge_origin(edge, session.ui.window().position(), session.area, size);
                *self.dock_reveal_anchor.borrow_mut() = Some(DockRevealAnchor {
                    edge,
                    position: visible,
                    saved_at: Instant::now(),
                });
            }
            let _ = session.ui.hide();
        }
        {
            let mut state = self.state.borrow_mut();
            state.settings.docked = false;
            state.settings.dock_edge = None;
            state.settings.dock_on_edge = false;
            state.settings.keep_full_main = true;
            let _ = state.persist_settings();
        }
        self.refresh();
        let _ = self.ui.show();
        self.ui.window().set_minimized(false);
        self.ui
            .window()
            .with_winit_window(|window| window.focus_window());
    }

    fn quit(&self) {
        self.persist_main_bounds();
        let editor_open = self.editor.borrow().is_some();
        let _ = self.state.borrow().prepare_for_shutdown(editor_open);
        if let Some(tray) = self.tray.borrow().as_ref() {
            tray.hide();
        }
        if let Some(dock) = self.dock.borrow().as_ref() {
            let _ = dock.ui.hide();
        }
        let _ = slint::quit_event_loop();
    }

    fn schedule_bounds_save(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        self.bounds_timer.start(
            TimerMode::SingleShot,
            Duration::from_millis(300),
            move || {
                if let Some(controller) = weak.upgrade() {
                    controller.persist_main_bounds();
                }
            },
        );
    }

    fn persist_main_bounds(&self) {
        let position = self.ui.window().position();
        let size = self.ui.window().size();
        let scale = self.ui.window().scale_factor().max(1.0);
        let snapshot = WindowState {
            width: size.width as f32 / scale,
            height: size.height as f32 / scale,
            x: position.x as f32 / scale,
            y: position.y as f32 / scale,
        };
        let mut state = self.state.borrow_mut();
        state.settings.window = Some(snapshot);
        let _ = state.persist_settings();
    }

    fn place_main_at_startup(&self) {
        let saved_size = self
            .state
            .borrow()
            .settings
            .window
            .as_ref()
            .map(|window| (window.width, window.height))
            .unwrap_or((DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT));
        self.ui
            .window()
            .set_size(LogicalSize::new(saved_size.0, saved_size.1));

        self.ui.window().with_winit_window(|window| {
            let Some(monitor) = window
                .current_monitor()
                .or_else(|| window.primary_monitor())
            else {
                return;
            };
            let scale = monitor.scale_factor() as f32;
            let area_position = monitor.position();
            let area_size = monitor.size();
            let width = saved_size.0 * scale;
            let height = saved_size.1 * scale;
            let (x, y) = (
                area_position.x as f32 + area_size.width as f32 - width - 40.0 * scale,
                area_position.y as f32 + (area_size.height as f32 - height) / 2.0,
            );
            let min_x = area_position.x as f32;
            let min_y = area_position.y as f32;
            let max_x = min_x + area_size.width as f32 - width;
            let max_y = min_y + area_size.height as f32 - height;
            self.ui.window().set_position(LogicalPosition::new(
                x.clamp(min_x, max_x.max(min_x)) / scale,
                y.clamp(min_y, max_y.max(min_y)) / scale,
            ));
        });
    }

    fn hide_main(&self) {
        self.persist_main_bounds();
        let _ = self.ui.hide();
    }

    fn copy_note(&self, id: &str) {
        let Some(text) = self.state.borrow().copy_text(id) else {
            return;
        };
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            if clipboard.set_text(text).is_ok() {
                let message = self.state.borrow().tr().copied.to_string();
                self.show_toast(message);
            }
        }
    }

    fn export_data(&self) {
        let stamp = chrono::Local::now().format("%Y%m%d%H%M");
        let filename = format!("q-note_{stamp}.json");
        let Some(path) = rfd::FileDialog::new()
            .set_file_name(&filename)
            .add_filter("JSON", &["json"])
            .save_file()
        else {
            return;
        };
        match self.state.borrow().export_json_to(&path) {
            Ok(()) => self.show_toast(self.state.borrow().tr().exported.to_string()),
            Err(_) => self.show_toast(self.state.borrow().tr().save_failed.to_string()),
        }
    }

    fn import_data(&self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("JSON", &["json"])
            .pick_file()
        else {
            return;
        };
        let result = self.state.borrow_mut().import_json_from(&path);
        match result {
            Ok(()) => {
                self.refresh();
                self.show_toast(self.state.borrow().tr().imported.to_string());
            }
            Err(_) => self.show_toast(self.state.borrow().tr().import_failed.to_string()),
        }
    }

    fn handle_manual_update_check(&self) {
        if self.state.borrow().update.checking {
            return;
        }
        if let Some(info) = self.state.borrow().update.available.clone() {
            self.open_update_confirm(&info);
            return;
        }
        self.run_update_check(true);
    }

    fn run_update_check(&self, manual: bool) {
        {
            let mut state = self.state.borrow_mut();
            if state.update.checking {
                return;
            }
            state.update.checking = true;
            state.update.error = None;
        }
        self.refresh();
        let sender = self.background_tx.clone();
        let _ = std::thread::Builder::new()
            .name("q-note-update-check".into())
            .spawn(move || {
                let result = crate::updater::check_for_update().map_err(|error| error.to_string());
                let _ = sender.send(BackgroundEvent::UpdateChecked { manual, result });
            });
    }

    fn schedule_daily_update(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        self.daily_update_timer.start(
            TimerMode::SingleShot,
            crate::updater::duration_until_next_daily_check(),
            move || {
                if let Some(controller) = weak.upgrade() {
                    controller.run_update_check(false);
                    controller.schedule_daily_update();
                }
            },
        );
    }

    fn process_background_events(&self) {
        let events = self.background_rx.borrow().try_iter().collect::<Vec<_>>();
        for event in events {
            self.handle_background_event(event);
        }
    }

    fn handle_background_event(&self, event: BackgroundEvent) {
        match event {
            BackgroundEvent::UpdateChecked { manual, result } => match result {
                Ok(Some(info)) => {
                    {
                        let mut state = self.state.borrow_mut();
                        state.update.checking = false;
                        state.update.available = Some(info.clone());
                        state.update.error = None;
                    }
                    self.refresh();
                    if manual {
                        self.open_update_confirm(&info);
                    }
                }
                Ok(None) => {
                    {
                        let mut state = self.state.borrow_mut();
                        state.update.checking = false;
                        state.update.available = None;
                        state.update.error = None;
                    }
                    self.refresh();
                    if manual {
                        self.show_toast(self.state.borrow().tr().update_none.to_string());
                    }
                }
                Err(error) => {
                    {
                        let mut state = self.state.borrow_mut();
                        state.update.checking = false;
                        state.update.available = None;
                        state.update.error = Some(error);
                    }
                    self.refresh();
                    if manual {
                        self.show_toast(self.state.borrow().tr().update_check_failed.to_string());
                    }
                }
            },
            BackgroundEvent::UpdateProgress { revision, progress } => {
                {
                    let mut active = self.update_download.borrow_mut();
                    let Some(download) = active.as_mut() else {
                        return;
                    };
                    if download.revision != revision || download.phase != UpdatePhase::Downloading {
                        return;
                    }
                    download.progress = progress;
                }
                self.refresh_update_download();
            }
            BackgroundEvent::UpdateDownloaded { revision, result } => {
                let is_current = self
                    .update_download
                    .borrow()
                    .as_ref()
                    .is_some_and(|download| {
                        download.revision == revision && !download.cancelled.load(Ordering::Relaxed)
                    });
                if !is_current {
                    return;
                }
                let downloaded = match result {
                    Ok(downloaded) => downloaded,
                    Err(error) => {
                        let message = self.state.borrow().tr().update_download_failed.to_string();
                        self.fail_update(error, message);
                        return;
                    }
                };
                if let Some(download) = self.update_download.borrow_mut().as_mut() {
                    download.phase = UpdatePhase::Preparing;
                    if let Some(total) = download.progress.total {
                        download.progress.downloaded = total;
                    }
                }
                self.refresh_update_download();
                self.persist_main_bounds();
                let editor_open = self.editor.borrow().is_some();
                let prepare_result = self.state.borrow_mut().prepare_for_update(editor_open);
                if let Err(error) = prepare_result {
                    let message = self.state.borrow().tr().update_prepare_failed.to_string();
                    self.fail_update(error.to_string(), message);
                    return;
                }
                if let Some(download) = self.update_download.borrow_mut().as_mut() {
                    download.phase = UpdatePhase::Installing;
                }
                self.refresh_update_download();
                let sender = self.background_tx.clone();
                let _ = std::thread::Builder::new()
                    .name("q-note-update-install".into())
                    .spawn(move || {
                        let result = crate::updater::install_and_relaunch(downloaded)
                            .map_err(|error| error.to_string());
                        let _ = sender.send(BackgroundEvent::UpdateInstalled { revision, result });
                    });
            }
            BackgroundEvent::UpdateInstalled { revision, result } => {
                if !self
                    .update_download
                    .borrow()
                    .as_ref()
                    .is_some_and(|download| download.revision == revision)
                {
                    return;
                }
                match result {
                    Ok(()) => {
                        let editor_open = self.editor.borrow().is_some();
                        let _ = self.state.borrow().prepare_for_shutdown(editor_open);
                        let _ = slint::quit_event_loop();
                    }
                    Err(error) => {
                        let message = self.state.borrow().tr().update_install_failed.to_string();
                        self.fail_update(error, message);
                    }
                }
            }
            BackgroundEvent::RemoteImage { url, bytes } => {
                self.remote_pending.borrow_mut().remove(&url);
                let image = bytes
                    .as_deref()
                    .and_then(|bytes| crate::slint_media::downloaded_image(&url, bytes))
                    .unwrap_or_default();
                self.remote_images.borrow_mut().insert(url, image);
                self.refresh();
                self.refresh_editor_models();
            }
        }
    }

    fn open_update_confirm(&self, info: &crate::updater::UpdateInfo) {
        self.ui
            .set_update_version(format!("v{}", info.version).into());
        self.ui
            .set_update_notes(info.notes.clone().unwrap_or_default().into());
        self.ui.set_modal_kind(3);
    }

    fn confirm_update(&self) {
        let Some(info) = self.state.borrow().update.available.clone() else {
            return;
        };
        if info.current_binary().is_none() {
            crate::updater::open_release_page(Some(&info.version));
            self.ui.set_modal_kind(0);
            self.show_toast(self.state.borrow().tr().update_open_release.to_string());
            return;
        }
        self.start_update_download(info);
    }

    fn start_update_download(&self, info: crate::updater::UpdateInfo) {
        let revision = self.update_revision.get().wrapping_add(1);
        self.update_revision.set(revision);
        let cancelled = Arc::new(AtomicBool::new(false));
        let progress = crate::updater::DownloadProgress {
            downloaded: 0,
            total: info.current_binary().and_then(|artifact| artifact.size),
        };
        *self.update_download.borrow_mut() = Some(UpdateDownloadUi {
            info: info.clone(),
            phase: UpdatePhase::Downloading,
            progress,
            cancelled: cancelled.clone(),
            revision,
        });
        self.ui.set_modal_kind(4);
        self.refresh_update_download();

        let sender = self.background_tx.clone();
        let progress_sender = sender.clone();
        let _ = std::thread::Builder::new()
            .name("q-note-update-download".into())
            .spawn(move || {
                let result = crate::updater::download_update(&info, &cancelled, |progress| {
                    let _ = progress_sender
                        .send(BackgroundEvent::UpdateProgress { revision, progress });
                })
                .map_err(|error| error.to_string());
                let _ = sender.send(BackgroundEvent::UpdateDownloaded { revision, result });
            });
    }

    fn cancel_update_download(&self) {
        let mut download = self.update_download.borrow_mut();
        let Some(active) = download.as_ref() else {
            return;
        };
        if active.phase == UpdatePhase::Installing {
            return;
        }
        active.cancelled.store(true, Ordering::Relaxed);
        self.update_revision
            .set(self.update_revision.get().wrapping_add(1));
        *download = None;
        drop(download);
        self.state.borrow_mut().update.available = None;
        self.ui.set_modal_kind(0);
        self.refresh();
    }

    fn fail_update(&self, error: String, message: String) {
        self.update_download.borrow_mut().take();
        {
            let mut state = self.state.borrow_mut();
            state.update.error = Some(error);
            state.update.available = None;
            state.update.checking = false;
        }
        self.ui.set_modal_kind(0);
        self.refresh();
        self.show_toast(message);
    }

    fn refresh_update_download(&self) {
        let download = self.update_download.borrow();
        let Some(download) = download.as_ref() else {
            return;
        };
        let tr = self.state.borrow().tr();
        let status = match download.phase {
            UpdatePhase::Downloading => tr.update_downloading,
            UpdatePhase::Preparing => tr.update_preparing,
            UpdatePhase::Installing => tr.update_installing,
        };
        let progress = if download.phase == UpdatePhase::Downloading {
            download
                .progress
                .total
                .filter(|total| *total > 0)
                .map(|total| download.progress.downloaded as f32 / total as f32)
                .unwrap_or(0.0)
                .clamp(0.0, 1.0)
        } else {
            1.0
        };
        self.ui.set_update_status(status.into());
        self.ui
            .set_update_version(format!("v{}", download.info.version).into());
        self.ui
            .set_update_notes(download.info.notes.clone().unwrap_or_default().into());
        self.ui.set_update_progress(progress);
        self.ui
            .set_update_progress_label(if download.phase == UpdatePhase::Downloading {
                format!("{}%", (progress * 100.0).round() as u32).into()
            } else {
                status.into()
            });
        self.ui.set_update_size_label(
            download
                .progress
                .total
                .map(|total| {
                    format!(
                        "{} / {}",
                        format_bytes(download.progress.downloaded),
                        format_bytes(total)
                    )
                })
                .unwrap_or_default()
                .into(),
        );
        self.ui
            .set_update_can_cancel(download.phase != UpdatePhase::Installing);
    }

    fn show_toast(&self, message: String) {
        self.ui.set_toast_text(message.into());
        self.ui.set_toast_visible(true);
        let ui = self.ui.as_weak();
        self.toast_timer.start(
            TimerMode::SingleShot,
            Duration::from_millis(1700),
            move || {
                if let Some(ui) = ui.upgrade() {
                    ui.set_toast_visible(false);
                }
            },
        );
    }

    fn refresh(&self) {
        let state = self.state.borrow();
        let remote_urls = state
            .notes
            .iter()
            .flat_map(|note| note.attachments.iter())
            .filter(|attachment| {
                attachment.kind == AttachmentKind::Image
                    && attachment.source == AttachmentSource::Url
            })
            .map(|attachment| attachment.value.clone())
            .collect::<Vec<_>>();
        self.ensure_remote_images(remote_urls);
        let cache = self.remote_images.borrow();
        let image_only = state.tr().image_only;
        self.notes.set_vec(
            state
                .notes
                .iter()
                .map(|note| note_to_ui(note, image_only, &cache))
                .collect::<Vec<_>>(),
        );
        let strings = strings_to_ui(state.tr());
        self.ui.set_strings(strings.clone());
        self.ui
            .set_total_label(state.tr().status_summary(state.notes.len()).into());
        self.ui.set_topmost(state.settings.always_on_top);
        self.ui.set_auto_start(state.settings.auto_start);
        self.ui.set_checking_update(state.update.checking);
        self.ui
            .set_version_label(format!("v{}", crate::updater::PACKAGE_VERSION).into());
        self.ui.set_has_update(state.update.available.is_some());
        if let Some(info) = state.update.available.as_ref() {
            self.ui
                .set_update_version(format!("v{}", info.version).into());
        }
        if let Some(tray) = self.tray.borrow().as_ref() {
            tray.update_labels(tray_labels(&state));
        }
        if let Some(editor) = self.editor.borrow().as_ref() {
            editor.ui.set_strings(strings);
            editor.ui.set_topmost(state.settings.always_on_top);
        }
        if let Some(dock) = self.dock.borrow().as_ref() {
            dock.ui.set_strings(strings_to_ui(state.tr()));
            dock.ui.set_topmost(state.settings.always_on_top);
        }
    }
}

fn window_work_area(window: &slint::Window) -> Option<WorkArea> {
    window.with_winit_window(|native| {
        #[cfg(target_os = "windows")]
        if let Some(area) = windows_work_area(native) {
            return area;
        }

        let monitor = native
            .current_monitor()
            .or_else(|| native.primary_monitor());
        let Some(monitor) = monitor else {
            return WorkArea {
                left: 0,
                top: 0,
                right: 1920,
                bottom: 1080,
            };
        };
        let origin = monitor.position();
        let size = monitor.size();
        WorkArea {
            left: origin.x,
            top: origin.y,
            right: origin.x + size.width as i32,
            bottom: origin.y + size.height as i32,
        }
    })
}

#[cfg(target_os = "windows")]
fn windows_work_area(window: &winit::window::Window) -> Option<WorkArea> {
    use raw_window_handle::{HasWindowHandle as _, RawWindowHandle};
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow,
    };

    let handle = window.window_handle().ok()?;
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return None;
    };
    let monitor = unsafe { MonitorFromWindow(handle.hwnd.get() as HWND, MONITOR_DEFAULTTONEAREST) };
    if monitor.is_null() {
        return None;
    }
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if unsafe { GetMonitorInfoW(monitor, &mut info) } == 0 {
        return None;
    }
    Some(WorkArea {
        left: info.rcWork.left,
        top: info.rcWork.top,
        right: info.rcWork.right,
        bottom: info.rcWork.bottom,
    })
}

fn clamp_free_dock_origin(
    position: PhysicalPosition,
    area: WorkArea,
    size: i32,
) -> PhysicalPosition {
    PhysicalPosition::new(
        position
            .x
            .clamp(area.left, (area.right - size).max(area.left)),
        position
            .y
            .clamp(area.top, (area.bottom - size).max(area.top)),
    )
}

fn visible_edge_origin(
    edge: DockEdge,
    position: PhysicalPosition,
    area: WorkArea,
    size: i32,
) -> PhysicalPosition {
    match edge {
        DockEdge::Left => PhysicalPosition::new(
            area.left,
            position
                .y
                .clamp(area.top, (area.bottom - size).max(area.top)),
        ),
        DockEdge::Right => PhysicalPosition::new(
            area.right - size,
            position
                .y
                .clamp(area.top, (area.bottom - size).max(area.top)),
        ),
        DockEdge::Top => PhysicalPosition::new(
            position
                .x
                .clamp(area.left, (area.right - size).max(area.left)),
            area.top,
        ),
        DockEdge::Bottom => PhysicalPosition::new(
            position
                .x
                .clamp(area.left, (area.right - size).max(area.left)),
            area.bottom - size,
        ),
    }
}

fn hidden_edge_origin(edge: DockEdge, visible: PhysicalPosition, size: i32) -> PhysicalPosition {
    let hidden = size / 2;
    match edge {
        DockEdge::Left => PhysicalPosition::new(visible.x - hidden, visible.y),
        DockEdge::Right => PhysicalPosition::new(visible.x + hidden, visible.y),
        DockEdge::Top => PhysicalPosition::new(visible.x, visible.y - hidden),
        DockEdge::Bottom => PhysicalPosition::new(visible.x, visible.y + hidden),
    }
}

fn detect_dock_edge(
    position: PhysicalPosition,
    area: WorkArea,
    size: i32,
    scale: f32,
) -> Option<DockEdge> {
    let threshold = (DOCK_SNAP_THRESHOLD * scale).round() as i32;
    let distances = [
        (DockEdge::Left, (position.x - area.left).abs()),
        (DockEdge::Right, (area.right - (position.x + size)).abs()),
        (DockEdge::Top, (position.y - area.top).abs()),
        (DockEdge::Bottom, (area.bottom - (position.y + size)).abs()),
    ];
    distances
        .into_iter()
        .filter(|(_, distance)| *distance <= threshold)
        .min_by_key(|(_, distance)| *distance)
        .map(|(edge, _)| edge)
}

fn apply_dock_clip(ui: &DockWindow, origin: PhysicalPosition, area: WorkArea, size: i32) {
    let scale = ui.window().scale_factor().max(1.0);
    let clip_left = (area.left - origin.x).clamp(0, size);
    let clip_top = (area.top - origin.y).clamp(0, size);
    let clip_right = (area.right - origin.x).clamp(0, size);
    let clip_bottom = (area.bottom - origin.y).clamp(0, size);
    ui.set_clip_left(clip_left as f32 / scale);
    ui.set_clip_top(clip_top as f32 / scale);
    ui.set_clip_width((clip_right - clip_left).max(0) as f32 / scale);
    ui.set_clip_height((clip_bottom - clip_top).max(0) as f32 / scale);

    #[cfg(target_os = "windows")]
    ui.window().with_winit_window(|native| {
        use raw_window_handle::{HasWindowHandle as _, RawWindowHandle};
        use windows_sys::Win32::Foundation::HWND;
        use windows_sys::Win32::Graphics::Gdi::{CreateRectRgn, DeleteObject, SetWindowRgn};

        let Ok(handle) = native.window_handle() else {
            return;
        };
        let RawWindowHandle::Win32(handle) = handle.as_raw() else {
            return;
        };
        let left = clip_left;
        let top = clip_top;
        let right = clip_right;
        let bottom = clip_bottom;
        if right <= left || bottom <= top {
            return;
        }
        let region = unsafe { CreateRectRgn(left, top, right, bottom) };
        if region.is_null() {
            return;
        }
        if unsafe { SetWindowRgn(handle.hwnd.get() as HWND, region, 1) } == 0 {
            let _ = unsafe { DeleteObject(region) };
        }
    });

    #[cfg(not(target_os = "windows"))]
    let _ = (origin, area, size);
}

fn clear_dock_clip(ui: &DockWindow) {
    let scale = ui.window().scale_factor().max(1.0);
    let size = ui.window().size().width as f32 / scale;
    ui.set_clip_left(0.0);
    ui.set_clip_top(0.0);
    ui.set_clip_width(size);
    ui.set_clip_height(size);

    #[cfg(target_os = "windows")]
    ui.window().with_winit_window(|native| {
        use raw_window_handle::{HasWindowHandle as _, RawWindowHandle};
        use windows_sys::Win32::Foundation::HWND;
        use windows_sys::Win32::Graphics::Gdi::SetWindowRgn;

        let Ok(handle) = native.window_handle() else {
            return;
        };
        let RawWindowHandle::Win32(handle) = handle.as_raw() else {
            return;
        };
        unsafe {
            let _ = SetWindowRgn(handle.hwnd.get() as HWND, std::ptr::null_mut(), 1);
        }
    });

    #[cfg(not(target_os = "windows"))]
    let _ = ui;
}

fn note_to_ui(
    note: &Note,
    image_only_label: &str,
    remote_images: &HashMap<String, slint::Image>,
) -> UiNote {
    let minimum = default_note_height(note);
    let maximum = full_note_height(note).max(minimum);
    let text_height = note.text_height.unwrap_or(minimum).clamp(minimum, maximum);
    let image_only = note.content.trim().is_empty()
        && note
            .attachments
            .iter()
            .any(|attachment| attachment.kind == AttachmentKind::Image);
    let display_content = if image_only {
        image_only_label.to_string()
    } else {
        note.content.clone()
    };
    let images = note
        .attachments
        .iter()
        .filter(|attachment| attachment.kind == AttachmentKind::Image)
        .take(4)
        .map(|attachment| attachment_to_ui(attachment, remote_images))
        .collect::<Vec<_>>();
    let files = note
        .attachments
        .iter()
        .filter(|attachment| attachment.kind == AttachmentKind::File)
        .take(3)
        .map(|attachment| attachment_to_ui(attachment, remote_images))
        .collect::<Vec<_>>();

    UiNote {
        id: note.id.clone().into(),
        content: note.content.clone().into(),
        display_content: display_content.into(),
        color: color_from_hex(&note.color),
        color_hex: note.color.clone().into(),
        pinned: note.pinned,
        muted: note.content.trim().is_empty(),
        text_height: text_height as i32,
        min_text_height: minimum as i32,
        max_text_height: maximum as i32,
        images: ModelRc::new(VecModel::from(images)),
        files: ModelRc::new(VecModel::from(files)),
    }
}

fn attachment_to_ui(
    attachment: &NoteAttachment,
    remote_images: &HashMap<String, slint::Image>,
) -> UiAttachment {
    UiAttachment {
        id: attachment.id.clone().into(),
        label: attachment_label(attachment).into(),
        value: attachment.value.clone().into(),
        image: if attachment.source == AttachmentSource::Url {
            remote_images
                .get(&attachment.value)
                .cloned()
                .unwrap_or_default()
        } else {
            crate::slint_media::attachment_image(attachment)
        },
        is_image: attachment.kind == AttachmentKind::Image,
    }
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

fn default_note_height(note: &Note) -> i64 {
    if note.content.trim().is_empty()
        || (note.content.chars().count() <= 34 && !note.content.contains('\n'))
    {
        NOTE_LINE_HEIGHT as i64
    } else {
        (NOTE_LINE_HEIGHT * 2.0) as i64
    }
}

fn full_note_height(note: &Note) -> i64 {
    let lines = note
        .content
        .lines()
        .map(|line| line.chars().count().max(1).div_ceil(34))
        .sum::<usize>()
        .max(1);
    (lines as f32 * NOTE_LINE_HEIGHT) as i64
}

fn color_from_hex(hex: &str) -> Color {
    let value = parse_hex_color(hex);
    Color::from_rgb_u8(
        ((value >> 16) & 0xff) as u8,
        ((value >> 8) & 0xff) as u8,
        (value & 0xff) as u8,
    )
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} {}", bytes, UNITS[unit])
    } else if value >= 10.0 {
        format!("{value:.0} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

fn tray_labels(state: &AppState) -> crate::slint_tray::TrayLabels {
    let tr = state.tr();
    crate::slint_tray::TrayLabels {
        topmost: if state.settings.always_on_top {
            tr.always_off
        } else {
            tr.always_on
        }
        .to_string(),
        language: tr.switch_language.to_string(),
        dock: if state.settings.docked {
            tr.switch_main_window
        } else {
            tr.switch_floating_ball
        }
        .to_string(),
        quit: tr.quit.to_string(),
    }
}

fn strings_to_ui(tr: crate::i18n::Translation) -> UiStrings {
    UiStrings {
        add_image: tr.add_image.into(),
        add_media: tr.add_media.into(),
        always_off: tr.always_off.into(),
        always_on: tr.always_on.into(),
        app_title: tr.app_title.into(),
        auto_start_off: tr.auto_start_off.into(),
        auto_start_on: tr.auto_start_on.into(),
        auto_start_failed: tr.auto_start_failed.into(),
        auto_start_updated: tr.auto_start_updated.into(),
        cancel: tr.cancel.into(),
        check_update: tr.check_update.into(),
        checking_update: tr.checking_update.into(),
        close_panel: tr.close_panel.into(),
        color: tr.color.into(),
        confirm_delete_all: tr.confirm_delete_all.into(),
        content_placeholder: tr.content_placeholder.into(),
        copied: tr.copied.into(),
        copy: tr.copy.into(),
        delete: tr.delete.into(),
        delete_all: tr.delete_all.into(),
        delete_all_body: tr.delete_all_body.into(),
        edit: tr.edit.into(),
        editor_edit_title: tr.editor_edit_title.into(),
        editor_new_title: tr.editor_new_title.into(),
        empty_action: tr.empty_action.into(),
        empty_title: tr.empty_title.into(),
        export: tr.export.into(),
        exported: tr.exported.into(),
        image_only: tr.image_only.into(),
        import: tr.import.into(),
        imported: tr.imported.into(),
        import_failed: tr.import_failed.into(),
        language_toggle: tr.language_toggle.into(),
        media_placeholder: tr.media_placeholder.into(),
        minimize: tr.minimize.into(),
        more_actions: tr.more_actions.into(),
        new_note: tr.new_note.into(),
        next_image: tr.next_image.into(),
        no_notes_body: tr.no_notes_body.into(),
        path: tr.path.into(),
        pin: tr.pin.into(),
        pinned: tr.pinned.into(),
        previous_image: tr.previous_image.into(),
        quit: tr.quit.into(),
        remove_attachment: tr.remove_attachment.into(),
        resize: tr.resize.into(),
        reset_view: tr.reset_view.into(),
        save: tr.save.into(),
        save_failed: tr.save_failed.into(),
        saved: tr.saved.into(),
        settings: tr.settings.into(),
        settings_title: tr.settings_title.into(),
        startup_setting: tr.startup_setting.into(),
        switch_language: tr.switch_language.into(),
        switch_floating_ball: tr.switch_floating_ball.into(),
        switch_main_window: tr.switch_main_window.into(),
        unpin: tr.unpin.into(),
        update_available: tr.update_available.into(),
        update_check_failed: tr.update_check_failed.into(),
        update_confirm: tr.update_confirm.into(),
        update_confirm_default: tr.update_confirm_default.into(),
        update_download_failed: tr.update_download_failed.into(),
        update_download_progress: tr.update_download_progress.into(),
        update_downloading: tr.update_downloading.into(),
        update_install_failed: tr.update_install_failed.into(),
        update_installing: tr.update_installing.into(),
        update_none: tr.update_none.into(),
        update_open_release: tr.update_open_release.into(),
        update_preparing: tr.update_preparing.into(),
        update_prepare_failed: tr.update_prepare_failed.into(),
        url: tr.url.into(),
        zoom_in: tr.zoom_in.into(),
        zoom_out: tr.zoom_out.into(),
    }
}
