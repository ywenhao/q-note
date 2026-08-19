use std::sync::{Mutex, OnceLock};

use gpui::{AsyncApp, Entity};
use tray_icon::{
    TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuItem},
};

use crate::app_state::AppState;
use crate::i18n::t;
use crate::ui::{dock_window, main_window};

static TRAY_CONTROL: OnceLock<Mutex<std::sync::mpsc::Sender<TrayControl>>> = OnceLock::new();

pub(crate) const NATIVE_MENU_TOPMOST_ID: &str = "q-note-toggle-topmost";
pub(crate) const NATIVE_MENU_LANGUAGE_ID: &str = "q-note-toggle-language";
pub(crate) const NATIVE_MENU_DOCK_ID: &str = "q-note-toggle-dock";
pub(crate) const NATIVE_MENU_QUIT_ID: &str = "q-note-quit";

#[derive(Clone, PartialEq)]
struct TrayLabels {
    topmost: String,
    language: String,
    dock: String,
    quit: String,
}

enum TrayControl {
    UpdateLabels(TrayLabels),
}

struct NativeMenu {
    menu: Menu,
    topmost: MenuItem,
    language: MenuItem,
    dock: MenuItem,
    quit: MenuItem,
}

#[cfg(target_os = "macos")]
struct MacTray {
    _tray: tray_icon::TrayIcon,
    menu: NativeMenu,
}

#[cfg(target_os = "macos")]
impl gpui::Global for MacTray {}

impl NativeMenu {
    fn new(labels: &TrayLabels) -> Self {
        let menu = Menu::new();
        let topmost = MenuItem::with_id(NATIVE_MENU_TOPMOST_ID, &labels.topmost, true, None);
        let language = MenuItem::with_id(NATIVE_MENU_LANGUAGE_ID, &labels.language, true, None);
        let dock = MenuItem::with_id(NATIVE_MENU_DOCK_ID, &labels.dock, true, None);
        let quit = MenuItem::with_id(NATIVE_MENU_QUIT_ID, &labels.quit, true, None);
        let _ = menu.append(&topmost);
        let _ = menu.append(&language);
        let _ = menu.append(&dock);
        let _ = menu.append(&quit);
        Self {
            menu,
            topmost,
            language,
            dock,
            quit,
        }
    }

    fn update_labels(&self, labels: &TrayLabels) {
        self.topmost.set_text(&labels.topmost);
        self.language.set_text(&labels.language);
        self.dock.set_text(&labels.dock);
        self.quit.set_text(&labels.quit);
    }

    fn command_for(&self, event: &MenuEvent) -> Option<TrayCommand> {
        command_for_menu_id(&event.id)
    }
}

fn command_for_menu_id(id: &tray_icon::menu::MenuId) -> Option<TrayCommand> {
    match id.as_ref() {
        NATIVE_MENU_TOPMOST_ID => Some(TrayCommand::ToggleTopmost),
        NATIVE_MENU_LANGUAGE_ID => Some(TrayCommand::ToggleLanguage),
        NATIVE_MENU_DOCK_ID => Some(TrayCommand::ToggleDock),
        NATIVE_MENU_QUIT_ID => Some(TrayCommand::Quit),
        _ => None,
    }
}

#[derive(Debug, Clone)]
pub enum TrayCommand {
    ShowMain,
    ToggleTopmost,
    ToggleLanguage,
    ToggleDock,
    Quit,
}

pub fn spawn_tray(state: Entity<AppState>, cx: &mut gpui::App) {
    let labels = {
        let s = state.read(cx);
        let tr = s.tr();
        TrayLabels {
            topmost: if s.settings.always_on_top {
                tr.always_off.to_string()
            } else {
                tr.always_on.to_string()
            },
            language: tr.switch_language.to_string(),
            dock: if s.settings.docked {
                tr.switch_main_window.to_string()
            } else {
                tr.switch_floating_ball.to_string()
            },
            quit: tr.quit.to_string(),
        }
    };
    let (tx, rx) = std::sync::mpsc::channel::<TrayCommand>();
    let (control_tx, control_rx) = std::sync::mpsc::channel::<TrayControl>();
    let _ = TRAY_CONTROL.set(Mutex::new(control_tx));

    #[cfg(target_os = "macos")]
    {
        // AppKit requires NSStatusItem creation and mutation on the main thread.
        // GPUI's foreground executor is the application main thread, so retain
        // the tray and its menu in a GPUI global and poll both event channels
        // from there.
        let tray_menu = NativeMenu::new(&labels);
        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu.menu.clone()))
            .with_menu_on_left_click(false)
            .with_tooltip("Q Note")
            .with_icon(load_tray_icon())
            .build();

        if let Ok(tray) = tray {
            cx.set_global(MacTray {
                _tray: tray,
                menu: tray_menu,
            });
        }

        let menu_channel = MenuEvent::receiver();
        let tray_channel = TrayIconEvent::receiver();
        cx.spawn(async move |cx: &mut AsyncApp| {
            let mut current_labels = labels;
            loop {
                while let Ok(control) = control_rx.try_recv() {
                    match control {
                        TrayControl::UpdateLabels(next_labels) => {
                            if next_labels != current_labels {
                                let _ = cx.update(|cx| {
                                    if let Some(tray) = cx.try_global::<MacTray>() {
                                        tray.menu.update_labels(&next_labels);
                                    }
                                });
                                current_labels = next_labels;
                            }
                        }
                    }
                }

                while let Ok(event) = menu_channel.try_recv() {
                    if let Some(command) = command_for_menu_id(&event.id) {
                        let _ = tx.send(command);
                    }
                }

                while let Ok(event) = tray_channel.try_recv() {
                    if matches!(
                        event,
                        TrayIconEvent::Click {
                            button: tray_icon::MouseButton::Left,
                            button_state: tray_icon::MouseButtonState::Up,
                            ..
                        }
                    ) {
                        let _ = tx.send(TrayCommand::ShowMain);
                    }
                }

                cx.background_executor()
                    .timer(std::time::Duration::from_millis(50))
                    .await;
            }
        })
        .detach();
    }

    #[cfg(not(target_os = "macos"))]
    std::thread::Builder::new()
        .name("q-note-tray".into())
        .spawn(move || {
            #[cfg(target_os = "linux")]
            {
                let _ = gtk::init();
            }

            let icon = load_tray_icon();
            let tray_menu = NativeMenu::new(&labels);

            let tray = TrayIconBuilder::new()
                .with_menu(Box::new(tray_menu.menu.clone()))
                .with_menu_on_left_click(false)
                .with_tooltip("Q Note")
                .with_icon(icon)
                .build();

            // Keep tray alive on this thread (TrayIcon is !Send on Linux).
            let _tray = tray;

            let menu_channel = MenuEvent::receiver();
            let tray_channel = TrayIconEvent::receiver();
            let mut current_labels = labels;

            loop {
                #[cfg(target_os = "windows")]
                pump_windows_messages();

                while let Ok(control) = control_rx.try_recv() {
                    match control {
                        TrayControl::UpdateLabels(labels) => {
                            if labels != current_labels {
                                tray_menu.update_labels(&labels);
                                current_labels = labels;
                            }
                        }
                    }
                }

                while let Ok(event) = menu_channel.try_recv() {
                    let command = tray_menu.command_for(&event);
                    if let Some(command) = command {
                        let _ = tx.send(command);
                    }
                }

                while let Ok(event) = tray_channel.try_recv() {
                    if matches!(
                        event,
                        TrayIconEvent::Click {
                            button: tray_icon::MouseButton::Left,
                            button_state: tray_icon::MouseButtonState::Up,
                            ..
                        }
                    ) {
                        let _ = tx.send(TrayCommand::ShowMain);
                    }
                }

                #[cfg(target_os = "linux")]
                while gtk::events_pending() {
                    gtk::main_iteration_do(false);
                }

                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        })
        .ok();

    // Poll tray commands on the GPUI executor.
    cx.spawn(async move |cx: &mut AsyncApp| {
        loop {
            while let Ok(cmd) = rx.try_recv() {
                handle_command(cmd, &state, cx);
            }
            cx.background_executor()
                .timer(std::time::Duration::from_millis(50))
                .await;
        }
    })
    .detach();
}

fn handle_command(cmd: TrayCommand, state: &Entity<AppState>, cx: &mut AsyncApp) {
    match cmd {
        TrayCommand::Quit => {
            let _ = cx.update(|cx| {
                let _ = main_window::prepare_for_shutdown(state, cx);
                cx.quit();
            });
        }
        TrayCommand::ShowMain => {
            let _ = cx.update(|cx| {
                main_window::show_main_from_tray(state.clone(), cx);
            });
        }
        TrayCommand::ToggleTopmost => {
            let _ = state.update(cx, |state, cx| {
                let next = !state.settings.always_on_top;
                state.set_always_on_top(next, cx);
            });
        }
        TrayCommand::ToggleLanguage => {
            let _ = state.update(cx, |state, cx| {
                state.toggle_language(cx);
            });
        }
        TrayCommand::ToggleDock => {
            let _ = cx.update(|cx| {
                let docked = state.read(cx).settings.docked;
                if docked {
                    main_window::restore_from_dock(state.clone(), cx);
                } else {
                    dock_window::collapse_to_dock(state.clone(), cx);
                }
            });
        }
    }
}

pub fn update_labels(state: &AppState) {
    let tr = t(state.settings.language);
    let labels = TrayLabels {
        topmost: if state.settings.always_on_top {
            tr.always_off.to_string()
        } else {
            tr.always_on.to_string()
        },
        language: tr.switch_language.to_string(),
        dock: if state.settings.docked {
            tr.switch_main_window.to_string()
        } else {
            tr.switch_floating_ball.to_string()
        },
        quit: tr.quit.to_string(),
    };
    send_control(TrayControl::UpdateLabels(labels));
}

#[cfg(target_os = "windows")]
fn pump_windows_messages() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, MSG, PM_REMOVE, PeekMessageW, TranslateMessage,
    };

    // tray-icon owns a hidden HWND on this thread, so its callbacks only run while this queue is pumped.
    let mut message = MSG::default();
    unsafe {
        while PeekMessageW(&mut message, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

fn send_control(control: TrayControl) {
    if let Some(sender) = TRAY_CONTROL.get()
        && let Ok(sender) = sender.lock()
    {
        let _ = sender.send(control);
    }
}

fn load_tray_icon() -> tray_icon::Icon {
    let bytes = include_bytes!("../assets/tray-icon.png");
    let img = image::load_from_memory(bytes)
        .expect("tray icon")
        .into_rgba8();
    let (w, h) = img.dimensions();
    tray_icon::Icon::from_rgba(img.into_raw(), w, h).expect("tray rgba")
}
