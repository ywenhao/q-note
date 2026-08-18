//! Floating 30px Q icon with native Windows edge movement and snap animation.

use gpui::{
    App, AppContext, Bounds, Context, Entity, InteractiveElement, IntoElement, MouseButton,
    ParentElement, Render, StatefulInteractiveElement, Styled, Window, WindowBackgroundAppearance,
    WindowBounds, WindowDecorations, WindowKind, WindowOptions, div, point, px, size,
};

#[cfg(not(target_os = "windows"))]
use gpui_component::menu::{ContextMenuExt, PopupMenuItem};

#[cfg(target_os = "windows")]
use tray_icon::menu::{ContextMenu as _, Menu, MenuItem};

use crate::app_state::AppState;
use crate::models::{DOCK_WINDOW_SIZE, DockEdge, WindowState};
use crate::ui::main_window::{self, q_mark};
use crate::ui::style::{APP_BG, color, color_alpha};

const SNAP_THRESHOLD: f32 = 28.0;
const DOCK_MARGIN: f32 = 12.0;
const DRAG_THRESHOLD: f32 = 4.0;
const DOCK_SLIDE_DURATION_MS: f32 = 130.0;
const DOCK_ANIMATION_FRAME_MS: u64 = 13;

#[cfg(target_os = "windows")]
struct DockNativeMenu {
    menu: Menu,
    topmost: MenuItem,
    language: MenuItem,
    dock: MenuItem,
    quit: MenuItem,
}

#[cfg(target_os = "windows")]
impl DockNativeMenu {
    fn new(state: &AppState) -> Self {
        let tr = state.tr();
        let menu = Menu::new();
        let topmost = MenuItem::with_id(
            crate::tray::NATIVE_MENU_TOPMOST_ID,
            if state.settings.always_on_top {
                tr.always_off
            } else {
                tr.always_on
            },
            true,
            None,
        );
        let language = MenuItem::with_id(
            crate::tray::NATIVE_MENU_LANGUAGE_ID,
            tr.switch_language,
            true,
            None,
        );
        let dock = MenuItem::with_id(
            crate::tray::NATIVE_MENU_DOCK_ID,
            tr.switch_main_window,
            true,
            None,
        );
        let quit = MenuItem::with_id(crate::tray::NATIVE_MENU_QUIT_ID, tr.quit, true, None);
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

    fn update_labels(&self, state: &AppState) {
        let tr = state.tr();
        self.topmost.set_text(if state.settings.always_on_top {
            tr.always_off
        } else {
            tr.always_on
        });
        self.language.set_text(tr.switch_language);
        self.dock.set_text(tr.switch_main_window);
        self.quit.set_text(tr.quit);
    }

    fn show(&self, window: &Window) -> bool {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};

        let Ok(handle) = HasWindowHandle::window_handle(window) else {
            return false;
        };
        let RawWindowHandle::Win32(handle) = handle.as_raw() else {
            return false;
        };
        unsafe {
            self.menu
                .show_context_menu_for_hwnd(handle.hwnd.get(), None)
        }
    }
}

pub struct DockWindow {
    state: Entity<AppState>,
    pointer_origin: Option<gpui::Point<gpui::Pixels>>,
    moving: bool,
    snap_revision: u64,
    motion_revision: u64,
    revealed: bool,
    #[cfg(target_os = "windows")]
    native_menu: DockNativeMenu,
}

impl DockWindow {
    pub fn new(state: Entity<AppState>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        window.on_window_should_close(cx, |_, _| false);
        cx.observe_window_bounds(window, |this, window, cx| {
            this.window_bounds_changed(window, cx);
        })
        .detach();
        cx.observe(&state, |_, _, cx| cx.notify()).detach();
        let revealed = state.read(cx).settings.dock_edge.is_none() || !cfg!(target_os = "windows");
        #[cfg(target_os = "windows")]
        let native_menu = DockNativeMenu::new(state.read(cx));
        Self {
            state,
            pointer_origin: None,
            moving: false,
            snap_revision: 0,
            motion_revision: 0,
            revealed,
            #[cfg(target_os = "windows")]
            native_menu,
        }
    }

    fn window_bounds_changed(&mut self, window: &Window, cx: &mut Context<Self>) {
        if !self.moving {
            return;
        }
        self.snap_revision = self.snap_revision.wrapping_add(1);
        let revision = self.snap_revision;
        cx.spawn_in(window, async move |this, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(220))
                .await;
            let _ = this.update_in(cx, |this, window, cx| {
                if !this.moving || this.snap_revision != revision {
                    return;
                }
                this.moving = false;
                this.finish_dock_move(window, cx);
            });
        })
        .detach();
    }

    fn set_revealed(&mut self, revealed: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.moving || self.revealed == revealed {
            return;
        }
        let Some(edge) = self.state.read(cx).settings.dock_edge else {
            self.revealed = true;
            return;
        };
        self.revealed = revealed;
        let Some(target) = edge_target(window, cx, edge, !revealed) else {
            return;
        };
        self.animate_to(target, window, cx);
    }

    fn animate_to(
        &mut self,
        target: gpui::Point<gpui::Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        #[cfg(target_os = "windows")]
        {
            let start = window.bounds().origin;
            let delta_x = f32::from(target.x - start.x);
            let delta_y = f32::from(target.y - start.y);
            self.motion_revision = self.motion_revision.wrapping_add(1);
            let revision = self.motion_revision;
            if delta_x.abs() < 0.5 && delta_y.abs() < 0.5 {
                let _ = crate::ui::move_window_to(window, target);
                return;
            }
            cx.spawn_in(window, async move |this, cx| {
                let started = std::time::Instant::now();
                loop {
                    cx.background_executor()
                        .timer(std::time::Duration::from_millis(DOCK_ANIMATION_FRAME_MS))
                        .await;
                    let progress = (started.elapsed().as_secs_f32() * 1000.0
                        / DOCK_SLIDE_DURATION_MS)
                        .clamp(0.0, 1.0);
                    let keep_going = this
                        .update_in(cx, |this, window, _| {
                            if this.motion_revision != revision {
                                return false;
                            }
                            let eased = 1.0 - (1.0 - progress).powi(3);
                            let origin =
                                point(start.x + px(delta_x * eased), start.y + px(delta_y * eased));
                            let _ = crate::ui::move_window_to(window, origin);
                            progress < 1.0
                        })
                        .unwrap_or(false);
                    if !keep_going {
                        break;
                    }
                }
            })
            .detach();
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (target, window, cx);
        }
    }

    fn finish_dock_move(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.state.read(cx).settings.docked {
            return;
        }
        let bounds = window.bounds();
        let edge = detect_snap_edge(window, cx);
        let visible_origin = edge
            .and_then(|edge| edge_target(window, cx, edge, false))
            .unwrap_or(bounds.origin);
        self.state.update(cx, |state, _| {
            state.settings.dock_edge = edge;
            state.settings.dock_on_edge = edge.is_some();
            state.dock_position = Some((f32::from(visible_origin.x), f32::from(visible_origin.y)));
            let _ = state.persist_settings();
        });
        self.revealed = true;
        if edge.is_some() {
            self.set_revealed(false, window, cx);
        }
    }
}

impl Render for DockWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        #[cfg(target_os = "windows")]
        self.native_menu.update_labels(self.state.read(cx));
        let shell = div()
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
            .on_hover(cx.listener(|this, hovered: &bool, window, cx| {
                this.set_revealed(*hovered, window, cx);
            }))
            .on_mouse_down_out(cx.listener(|this, _, _, _| {
                this.pointer_origin = None;
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &gpui::MouseDownEvent, _, _| {
                    this.snap_revision = this.snap_revision.wrapping_add(1);
                    this.motion_revision = this.motion_revision.wrapping_add(1);
                    this.moving = false;
                    this.pointer_origin = Some(event.position);
                }),
            )
            .on_mouse_move(
                cx.listener(|this, event: &gpui::MouseMoveEvent, window, cx| {
                    let Some(origin) = this.pointer_origin else {
                        return;
                    };
                    if event.pressed_button != Some(MouseButton::Left) {
                        this.pointer_origin = None;
                        return;
                    }
                    let dx = f32::from(event.position.x - origin.x);
                    let dy = f32::from(event.position.y - origin.y);
                    if dx * dx + dy * dy < DRAG_THRESHOLD * DRAG_THRESHOLD {
                        return;
                    }

                    this.pointer_origin = None;
                    this.moving = true;
                    if !start_window_move(window) {
                        this.moving = false;
                        return;
                    }
                    #[cfg(target_os = "windows")]
                    {
                        // WM_NCLBUTTONDOWN returns after the native move loop ends.
                        this.moving = false;
                        this.finish_dock_move(window, cx);
                    }
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    if this.pointer_origin.take().is_some() {
                        main_window::restore_from_dock(this.state.clone(), cx);
                    }
                }),
            );

        #[cfg(target_os = "windows")]
        {
            shell.on_mouse_up(
                MouseButton::Right,
                cx.listener(|this, _, window, _| {
                    let _ = this.native_menu.show(window);
                }),
            )
        }

        #[cfg(not(target_os = "windows"))]
        {
            let tr = self.state.read(cx).tr();
            let always_on_top = self.state.read(cx).settings.always_on_top;
            shell.context_menu({
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
                    menu.item(PopupMenuItem::new(topmost_label.clone()).on_click({
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
                    .item(PopupMenuItem::new(main_label.clone()).on_click({
                        let state = state.clone();
                        move |_, _, cx| {
                            main_window::restore_from_dock(state.clone(), cx);
                        }
                    }))
                    .item(PopupMenuItem::new(quit_label.clone()).on_click({
                        let state = state.clone();
                        move |_, _, cx| {
                            let _ = main_window::prepare_for_shutdown(&state, cx);
                            cx.quit();
                        }
                    }))
                }
            })
        }
    }
}

#[cfg(target_os = "windows")]
fn start_window_move(window: &Window) -> bool {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture;
    use windows_sys::Win32::UI::WindowsAndMessaging::{HTCAPTION, SendMessageW, WM_NCLBUTTONDOWN};

    let Ok(handle) = HasWindowHandle::window_handle(window) else {
        return false;
    };
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return false;
    };
    let hwnd = handle.hwnd.get() as HWND;

    unsafe {
        let _ = ReleaseCapture();
        let _ = SendMessageW(hwnd, WM_NCLBUTTONDOWN, HTCAPTION as usize, 0);
    }
    true
}

#[cfg(not(target_os = "windows"))]
fn start_window_move(window: &Window) -> bool {
    window.start_window_move();
    true
}

fn detect_snap_edge(window: &Window, cx: &App) -> Option<DockEdge> {
    let bounds = window.bounds();
    let display = window.display(cx)?;
    let area = display.bounds();
    let dist_left = f32::from(bounds.origin.x - area.origin.x).abs();
    let dist_right =
        f32::from((area.origin.x + area.size.width) - (bounds.origin.x + bounds.size.width)).abs();
    let dist_top = f32::from(bounds.origin.y - area.origin.y).abs();
    let dist_bottom =
        f32::from((area.origin.y + area.size.height) - (bounds.origin.y + bounds.size.height))
            .abs();

    let mut edge = None;
    let mut min = SNAP_THRESHOLD;
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

fn edge_target(
    window: &Window,
    cx: &App,
    edge: DockEdge,
    hidden: bool,
) -> Option<gpui::Point<gpui::Pixels>> {
    let area = window.display(cx)?.bounds();
    Some(edge_origin(edge, hidden, window.bounds(), area))
}

fn edge_origin(
    edge: DockEdge,
    hidden: bool,
    bounds: Bounds<gpui::Pixels>,
    area: Bounds<gpui::Pixels>,
) -> gpui::Point<gpui::Pixels> {
    let area_left = f32::from(area.origin.x);
    let area_top = f32::from(area.origin.y);
    let area_right = area_left + f32::from(area.size.width);
    let area_bottom = area_top + f32::from(area.size.height);
    let width = f32::from(bounds.size.width);
    let height = f32::from(bounds.size.height);
    let center_x = f32::from(bounds.origin.x) + width / 2.0;
    let center_y = f32::from(bounds.origin.y) + height / 2.0;
    let hidden_x = if hidden { width / 2.0 } else { 0.0 };
    let hidden_y = if hidden { height / 2.0 } else { 0.0 };

    let (x, y) = match edge {
        DockEdge::Left => (
            area_left - hidden_x,
            (center_y - height / 2.0).clamp(area_top, area_bottom - height),
        ),
        DockEdge::Right => (
            area_right - width + hidden_x,
            (center_y - height / 2.0).clamp(area_top, area_bottom - height),
        ),
        DockEdge::Top => (
            (center_x - width / 2.0).clamp(area_left, area_right - width),
            area_top - hidden_y,
        ),
        DockEdge::Bottom => (
            (center_x - width / 2.0).clamp(area_left, area_right - width),
            area_bottom - height + hidden_y,
        ),
    };
    point(px(x), px(y))
}

pub fn collapse_to_dock(state: Entity<AppState>, cx: &mut App) {
    cx.defer(move |cx| collapse_to_dock_now(state, cx));
}

fn collapse_to_dock_now(state: Entity<AppState>, cx: &mut App) {
    if state.read(cx).settings.docked {
        return;
    }
    if let Some(main) = state.read(cx).main_window {
        let _ = main.update(cx, |_, window, cx| {
            let b = window.bounds();
            state.update(cx, |s, _| {
                s.settings.window = Some(WindowState {
                    width: f32::from(b.size.width),
                    height: f32::from(b.size.height),
                    x: f32::from(b.origin.x),
                    y: f32::from(b.origin.y),
                });
                if s.settings.dock_edge.is_none() {
                    s.dock_position = None;
                }
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
    let bounds = dock_bounds(state.read(cx), size_px, cx);

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
    let always_on_top = state.read(cx).settings.always_on_top;
    let _ = handle.update(cx, |_, window, _| {
        crate::ui::apply_window_topmost(window, always_on_top);
    });
}

fn dock_bounds(state: &AppState, size_px: f32, cx: &App) -> Bounds<gpui::Pixels> {
    let displays = cx.displays();
    let size = size(px(size_px), px(size_px));
    let anchor = state
        .dock_position
        .map(|(x, y)| (x + size_px / 2.0, y + size_px / 2.0))
        .or_else(|| {
            state.settings.window.as_ref().map(|window| {
                (
                    window.x + window.width / 2.0,
                    window.y + window.height / 2.0,
                )
            })
        });
    let display = anchor
        .and_then(|(x, y)| {
            displays.iter().find(|display| {
                let area = display.bounds();
                let left = f32::from(area.origin.x);
                let top = f32::from(area.origin.y);
                x >= left
                    && x <= left + f32::from(area.size.width)
                    && y >= top
                    && y <= top + f32::from(area.size.height)
            })
        })
        .or_else(|| displays.first());
    let Some(display) = display else {
        return Bounds::centered(None, size, cx);
    };
    let area = display.bounds();
    let area_left = f32::from(area.origin.x);
    let area_top = f32::from(area.origin.y);
    let area_right = area_left + f32::from(area.size.width);
    let area_bottom = area_top + f32::from(area.size.height);
    let base_origin = if let Some((center_x, center_y)) = anchor {
        point(
            px((center_x - size_px / 2.0).clamp(area_left, area_right - size_px)),
            px((center_y - size_px / 2.0).clamp(area_top, area_bottom - size_px)),
        )
    } else {
        point(
            px(area_right - DOCK_MARGIN - size_px),
            px(area_bottom - DOCK_MARGIN - size_px),
        )
    };
    let base = Bounds {
        origin: base_origin,
        size,
    };
    let origin = state.settings.dock_edge.map_or(base_origin, |edge| {
        edge_origin(edge, cfg!(target_os = "windows"), base, area)
    });
    Bounds { origin, size }
}
