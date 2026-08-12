use std::sync::{Mutex, OnceLock};

use gpui::{AsyncApp, Entity};
use tray_icon::{
    TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};

use crate::app_state::AppState;
use crate::i18n::t;
use crate::ui::{dock_window, main_window};

static LABEL_CACHE: OnceLock<Mutex<TrayLabels>> = OnceLock::new();

#[derive(Clone)]
struct TrayLabels {
    topmost: String,
    language: String,
    dock: String,
    quit: String,
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
    let _ = LABEL_CACHE.set(Mutex::new(labels.clone()));

    let (tx, rx) = std::sync::mpsc::channel::<TrayCommand>();

    std::thread::Builder::new()
        .name("q-note-tray".into())
        .spawn(move || {
            #[cfg(target_os = "linux")]
            {
                let _ = gtk::init();
            }

            let icon = load_tray_icon();
            let menu = Menu::new();
            let topmost = MenuItem::new(&labels.topmost, true, None);
            let language = MenuItem::new(&labels.language, true, None);
            let dock = MenuItem::new(&labels.dock, true, None);
            let quit = MenuItem::new(&labels.quit, true, None);
            let _ = menu.append(&topmost);
            let _ = menu.append(&language);
            let _ = menu.append(&dock);
            let _ = menu.append(&PredefinedMenuItem::separator());
            let _ = menu.append(&quit);

            let tray = TrayIconBuilder::new()
                .with_menu(Box::new(menu))
                .with_tooltip("Q Note")
                .with_icon(icon)
                .build();

            // Keep tray alive on this thread (TrayIcon is !Send on Linux).
            let _tray = tray;

            let topmost_id = topmost.id().clone();
            let language_id = language.id().clone();
            let dock_id = dock.id().clone();
            let quit_id = quit.id().clone();

            let menu_channel = MenuEvent::receiver();
            let tray_channel = TrayIconEvent::receiver();

            loop {
                if let Ok(event) = menu_channel.try_recv() {
                    if event.id == topmost_id {
                        let _ = tx.send(TrayCommand::ToggleTopmost);
                    } else if event.id == language_id {
                        let _ = tx.send(TrayCommand::ToggleLanguage);
                    } else if event.id == dock_id {
                        let _ = tx.send(TrayCommand::ToggleDock);
                    } else if event.id == quit_id {
                        let _ = tx.send(TrayCommand::Quit);
                    }
                }

                if let Ok(event) = tray_channel.try_recv() {
                    if let TrayIconEvent::Click {
                        button: tray_icon::MouseButton::Left,
                        button_state: tray_icon::MouseButtonState::Up,
                        ..
                    } = event
                    {
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
            cx.update(|cx| cx.quit());
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
    if let Some(cache) = LABEL_CACHE.get() {
        if let Ok(mut guard) = cache.lock() {
            *guard = labels;
        }
    }
    // tray-icon MenuItem text updates require holding item handles;
    // labels apply on next tray rebuild / process restart. Tooltip stays "Q Note".
}

fn load_tray_icon() -> tray_icon::Icon {
    let bytes = include_bytes!("../assets/tray-icon.png");
    let img = image::load_from_memory(bytes)
        .expect("tray icon")
        .into_rgba8();
    let (w, h) = img.dimensions();
    tray_icon::Icon::from_rgba(img.into_raw(), w, h).expect("tray rgba")
}
