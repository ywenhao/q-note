//! Q Note — GPUI desktop note board.

mod app_state;
mod autostart;
mod i18n;
mod models;
mod note_ordering;
mod storage;
mod tray;
mod ui;
mod updater;

use gpui::{
    App, AppContext, Application, Bounds, TitlebarOptions, WindowBounds, WindowKind, WindowOptions,
    px, size,
};
use gpui_component::Root;
use gpui_component_assets::Assets;

use crate::app_state::AppState;
use crate::models::{DEFAULT_WINDOW_HEIGHT, DEFAULT_WINDOW_WIDTH};
use crate::ui::main_window::MainWindow;

fn main() {
    Application::new().with_assets(Assets).run(|cx: &mut App| {
        gpui_component::init(cx);
        ui::init(cx);

        let state = cx.new(AppState::new);
        tray::spawn_tray(state.clone(), cx);

        let docked = state.read(cx).settings.docked;
        if docked {
            ui::dock_window::open_dock_window(state.clone(), cx);
        } else {
            let always_on_top = state.read(cx).settings.always_on_top;
            let (w, h) = state
                .read(cx)
                .settings
                .window
                .as_ref()
                .map(|win| (win.width, win.height))
                .unwrap_or((DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT));

            let bounds = Bounds::centered(None, size(px(w.max(DEFAULT_WINDOW_WIDTH)), px(h.max(DEFAULT_WINDOW_HEIGHT))), cx);

            let window_handle = cx
                .open_window(
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(bounds)),
                        titlebar: Some(TitlebarOptions {
                            title: Some("Q Note".into()),
                            appears_transparent: true,
                            ..Default::default()
                        }),
                        window_min_size: Some(size(
                            px(DEFAULT_WINDOW_WIDTH),
                            px(DEFAULT_WINDOW_HEIGHT),
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
                        window_decorations: Some(gpui::WindowDecorations::Client),
                        ..Default::default()
                    },
                    {
                        let state = state.clone();
                        move |window, cx| {
                            let view = cx.new(|cx| MainWindow::new(state.clone(), window, cx));
                            cx.new(|cx| Root::new(view, window, cx))
                        }
                    },
                )
                .expect("failed to open main window");

            state.update(cx, |state, _| {
                state.main_window = Some(window_handle);
            });
        }

        cx.activate(true);
    });
}
