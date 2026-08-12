//! Floating 30px Q icon with edge snap via window recreate (GPUI has no set_bounds).

use gpui::{
    App, AppContext, Bounds, Context, Entity, InteractiveElement, IntoElement, MouseButton,
    ParentElement, Render, Styled, Window, WindowBackgroundAppearance, WindowBounds,
    WindowDecorations, WindowKind, WindowOptions, div, point, px, size,
};
use gpui_component::menu::{ContextMenuExt, PopupMenuItem};

use crate::app_state::AppState;
use crate::models::{DOCK_WINDOW_SIZE, DockEdge, WindowState};
use crate::ui::main_window::{self, q_mark};
use crate::ui::style::{APP_BG, color, color_alpha};

const SNAP_THRESHOLD: f32 = 28.0;
const DOCK_MARGIN: f32 = 12.0;

pub struct DockWindow {
    state: Entity<AppState>,
    moved: bool,
}

impl DockWindow {
    pub fn new(state: Entity<AppState>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        window.on_window_should_close(cx, |_, _| false);
        cx.observe(&state, |_, _, cx| cx.notify()).detach();
        Self {
            state,
            moved: false,
        }
    }
}

impl Render for DockWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tr = self.state.read(cx).tr();
        let always_on_top = self.state.read(cx).settings.always_on_top;

        div()
            .id("dock-shell")
            .size_full()
            .rounded_full()
            .bg(color(APP_BG))
            .border_1()
            .border_color(color_alpha(0x18212f, 0.15))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .shadow(vec![gpui::BoxShadow {
                color: color_alpha(0x1f2328, 0.18).into(),
                offset: point(px(0.), px(6.)),
                blur_radius: px(14.),
                spread_radius: px(0.),
            }])
            .child(q_mark(22.))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| {
                    this.moved = false;
                    window.start_window_move();
                    // Mark as moved after a short delay via notify cycle
                    this.moved = true;
                    cx.notify();
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| {
                    if this.moved {
                        // Attempt edge snap by recreating at snapped origin.
                        if let Some(edge) = detect_snap_edge(window, cx) {
                            let state = this.state.clone();
                            state.update(cx, |s, _| {
                                s.settings.dock_edge = Some(edge);
                                s.settings.dock_on_edge = true;
                                let _ = s.persist_settings();
                            });
                            // Recreate dock at snapped position.
                            window.remove_window();
                            open_dock_window(state, cx);
                        }
                        this.moved = false;
                    } else {
                        main_window::restore_from_dock(this.state.clone(), cx);
                    }
                    cx.notify();
                }),
            )
            .context_menu({
                let state = self.state.clone();
                let topmost_label = if always_on_top {
                    tr.always_off.to_string()
                } else {
                    tr.always_on.to_string()
                };
                let lang_label = tr.switch_language.to_string();
                let main_label = tr.switch_main_window.to_string();
                let quit_label = tr.quit.to_string();
                move |menu, _, _| {
                    menu.item(PopupMenuItem::new(main_label.clone()).on_click({
                        let state = state.clone();
                        move |_, _, cx| {
                            main_window::restore_from_dock(state.clone(), cx);
                        }
                    }))
                    .item(PopupMenuItem::new(topmost_label.clone()).on_click({
                        let state = state.clone();
                        move |_, _, cx| {
                            state.update(cx, |s, cx| {
                                let next = !s.settings.always_on_top;
                                s.set_always_on_top(next, cx);
                            });
                        }
                    }))
                    .item(PopupMenuItem::new(lang_label.clone()).on_click({
                        let state = state.clone();
                        move |_, _, cx| {
                            state.update(cx, |s, cx| s.toggle_language(cx));
                        }
                    }))
                    .separator()
                    .item(PopupMenuItem::new(quit_label.clone()).on_click(|_, _, cx| cx.quit()))
                }
            })
    }
}

fn detect_snap_edge(window: &Window, cx: &App) -> Option<DockEdge> {
    let bounds = window.bounds();
    let display = window.display(cx)?;
    let area = display.bounds();
    let threshold = px(SNAP_THRESHOLD);

    let dist_left = bounds.origin.x - area.origin.x;
    let dist_right = (area.origin.x + area.size.width) - (bounds.origin.x + bounds.size.width);
    let dist_top = bounds.origin.y - area.origin.y;
    let dist_bottom = (area.origin.y + area.size.height) - (bounds.origin.y + bounds.size.height);

    let mut edge = None;
    let mut min = threshold;
    if dist_left <= min {
        min = dist_left;
        edge = Some(DockEdge::Left);
    }
    if dist_right <= min {
        min = dist_right;
        edge = Some(DockEdge::Right);
    }
    if dist_top <= min {
        min = dist_top;
        edge = Some(DockEdge::Top);
    }
    if dist_bottom <= min {
        let _ = min;
        edge = Some(DockEdge::Bottom);
    }
    edge
}

pub fn collapse_to_dock(state: Entity<AppState>, cx: &mut App) {
    if let Some(main) = state.read(cx).main_window.clone() {
        let _ = main.update(cx, |_, window, cx| {
            let b = window.bounds();
            state.update(cx, |s, _| {
                s.settings.window = Some(WindowState {
                    width: f32::from(b.size.width),
                    height: f32::from(b.size.height),
                    x: f32::from(b.origin.x),
                    y: f32::from(b.origin.y),
                });
                s.settings.docked = true;
                let _ = s.persist_settings();
                crate::tray::update_labels(s);
            });
            window.remove_window();
        });
    }
    state.update(cx, |s, _| s.main_window = None);
    open_dock_window(state, cx);
}

pub fn open_dock_window(state: Entity<AppState>, cx: &mut App) {
    let size_px = DOCK_WINDOW_SIZE;
    let edge = state.read(cx).settings.dock_edge;
    let bounds = dock_bounds(edge, size_px, cx);

    let handle = cx
        .open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: None,
                kind: WindowKind::PopUp,
                is_resizable: false,
                is_movable: true,
                is_minimizable: false,
                focus: false,
                show: true,
                window_background: WindowBackgroundAppearance::Transparent,
                window_decorations: Some(WindowDecorations::Client),
                window_min_size: Some(size(px(size_px), px(size_px))),
                ..Default::default()
            },
            {
                let state = state.clone();
                move |window, cx| {
                    let view = cx.new(|cx| DockWindow::new(state.clone(), window, cx));
                    cx.new(|cx| gpui_component::Root::new(view, window, cx))
                }
            },
        )
        .expect("open dock");

    state.update(cx, |s, _| s.dock_window = Some(handle));
}

fn dock_bounds(edge: Option<DockEdge>, size_px: f32, cx: &App) -> Bounds<gpui::Pixels> {
    let displays = cx.displays();
    let Some(display) = displays.first() else {
        return Bounds::centered(None, size(px(size_px), px(size_px)), cx);
    };
    let area = display.bounds();
    let half = px(size_px / 2.);
    let origin = match edge {
        Some(DockEdge::Left) => point(area.origin.x - half, area.origin.y + area.size.height / 2.),
        Some(DockEdge::Right) => point(
            area.origin.x + area.size.width - half,
            area.origin.y + area.size.height / 2.,
        ),
        Some(DockEdge::Top) => point(area.origin.x + area.size.width / 2., area.origin.y - half),
        Some(DockEdge::Bottom) | None => point(
            area.origin.x + area.size.width - px(DOCK_MARGIN) - px(size_px),
            area.origin.y + area.size.height - px(DOCK_MARGIN) - px(size_px),
        ),
    };
    Bounds {
        origin,
        size: size(px(size_px), px(size_px)),
    }
}
