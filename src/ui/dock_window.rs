//! Floating Q icon with native edge movement and snap animation.

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

const SNAP_THRESHOLD: f32 = 28.0;
const DOCK_MARGIN: f32 = 12.0;
const DRAG_THRESHOLD: f32 = 4.0;
const DOCK_SLIDE_DURATION_MS: f32 = 130.0;
const DOCK_ANIMATION_FRAME_MS: u64 = 13;
const DOCK_RETURN_SNAP_DELAY_MS: u64 = 500;
const DOCK_REVEAL_ANCHOR_MAX_AGE: std::time::Duration = std::time::Duration::from_secs(5 * 60);
// Check the final half-circle hit area only after the 130 ms snap animation settles.
const CONCEAL_GUARD_SETTLE_MS: u64 = 160;
const CONCEAL_GUARD_POLL_MS: u64 = 30;
const REVEAL_EXIT_POLL_MS: u64 = 30;

#[derive(Clone, Copy, Debug, PartialEq)]
struct DockClip {
    left: f32,
    top: f32,
    width: f32,
    height: f32,
}

impl DockClip {
    fn full(bounds: Bounds<gpui::Pixels>) -> Self {
        Self {
            left: 0.0,
            top: 0.0,
            width: f32::from(bounds.size.width),
            height: f32::from(bounds.size.height),
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn intersection(bounds: Bounds<gpui::Pixels>, area: Bounds<gpui::Pixels>) -> Self {
        let window_left = f32::from(bounds.origin.x);
        let window_top = f32::from(bounds.origin.y);
        let width = f32::from(bounds.size.width);
        let height = f32::from(bounds.size.height);
        let area_left = f32::from(area.origin.x);
        let area_top = f32::from(area.origin.y);
        let area_right = area_left + f32::from(area.size.width);
        let area_bottom = area_top + f32::from(area.size.height);
        let left = (area_left - window_left).clamp(0.0, width);
        let top = (area_top - window_top).clamp(0.0, height);
        let right = (area_right - window_left).clamp(0.0, width);
        let bottom = (area_bottom - window_top).clamp(0.0, height);

        Self {
            left,
            top,
            width: (right - left).max(0.0),
            height: (bottom - top).max(0.0),
        }
    }
}

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
    manual_move: bool,
    snap_revision: u64,
    motion_revision: u64,
    conceal_revision: u64,
    reveal_revision: u64,
    revealed: bool,
    conceal_until_hover_exit: bool,
    edge_area: Option<Bounds<gpui::Pixels>>,
    clip: DockClip,
    #[cfg(target_os = "windows")]
    native_menu: DockNativeMenu,
}

impl DockWindow {
    pub fn new(state: Entity<AppState>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        window.on_window_should_close(cx, |window, _| {
            if !crate::ui::set_window_visible(window, false) {
                window.minimize_window();
            }
            false
        });
        cx.observe_window_bounds(window, |this, window, cx| {
            this.window_bounds_changed(window, cx);
        })
        .detach();
        cx.observe(&state, |_, _, cx| cx.notify()).detach();
        let revealed =
            state.read(cx).settings.dock_edge.is_none() || !crate::ui::can_position_window(window);
        let edge_area = state
            .read(cx)
            .settings
            .dock_edge
            .and_then(|_| work_area(window, cx));
        #[cfg(target_os = "windows")]
        let native_menu = DockNativeMenu::new(state.read(cx));
        Self {
            state,
            pointer_origin: None,
            moving: false,
            manual_move: false,
            snap_revision: 0,
            motion_revision: 0,
            conceal_revision: 0,
            reveal_revision: 0,
            revealed,
            conceal_until_hover_exit: false,
            edge_area,
            clip: DockClip::full(window.bounds()),
            #[cfg(target_os = "windows")]
            native_menu,
        }
    }

    fn window_bounds_changed(&mut self, window: &Window, cx: &mut Context<Self>) {
        if !self.moving || self.manual_move {
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
        if !revealed {
            self.cancel_reveal_watch();
        }
        if !crate::ui::can_position_window(window) {
            self.revealed = true;
            return;
        }
        if self.moving || self.revealed == revealed {
            return;
        }
        let Some(edge) = self.state.read(cx).settings.dock_edge else {
            self.revealed = true;
            self.clear_edge_clip(window, cx);
            return;
        };
        let area = if revealed {
            self.edge_area.or_else(|| work_area(window, cx))
        } else {
            work_area(window, cx).or(self.edge_area)
        };
        let Some(area) = area else {
            self.revealed = true;
            self.clear_edge_clip(window, cx);
            return;
        };
        self.edge_area = Some(area);
        self.revealed = revealed;
        let target = edge_window_origin(window, edge, !revealed, area);
        self.animate_to(target, area, edge, window, cx);
        if revealed {
            self.watch_revealed_pointer_exit(window, cx);
        }
    }

    fn animate_to(
        &mut self,
        target: gpui::Point<gpui::Pixels>,
        area: Bounds<gpui::Pixels>,
        edge: DockEdge,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !crate::ui::can_position_window(window) {
            return;
        }
        let start = window.bounds().origin;
        let delta_x = f32::from(target.x - start.x);
        let delta_y = f32::from(target.y - start.y);
        self.motion_revision = self.motion_revision.wrapping_add(1);
        let revision = self.motion_revision;
        if delta_x.abs() < 0.5 && delta_y.abs() < 0.5 {
            let _ = crate::ui::move_window_to(window, target);
            self.apply_edge_clip(target, area, edge, window, cx);
            return;
        }
        cx.spawn_in(window, async move |this, cx| {
            let started = std::time::Instant::now();
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(DOCK_ANIMATION_FRAME_MS))
                    .await;
                let progress = (started.elapsed().as_secs_f32() * 1000.0 / DOCK_SLIDE_DURATION_MS)
                    .clamp(0.0, 1.0);
                let keep_going = this
                    .update_in(cx, |this, window, cx| {
                        if this.motion_revision != revision {
                            return false;
                        }
                        let eased = 1.0 - (1.0 - progress).powi(3);
                        let origin =
                            point(start.x + px(delta_x * eased), start.y + px(delta_y * eased));
                        let _ = crate::ui::move_window_to(window, origin);
                        this.apply_edge_clip(origin, area, edge, window, cx);
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

    fn apply_edge_clip(
        &mut self,
        origin: gpui::Point<gpui::Pixels>,
        area: Bounds<gpui::Pixels>,
        edge: DockEdge,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        let bounds = Bounds {
            origin,
            size: window.bounds().size,
        };
        #[cfg(target_os = "windows")]
        let clip = {
            let _ = area;
            DockClip::full(bounds)
        };
        #[cfg(not(target_os = "windows"))]
        let clip = DockClip::intersection(bounds, area);
        if self.clip != clip {
            self.clip = clip;
            cx.notify();
        }
        #[cfg(target_os = "windows")]
        apply_native_edge_clip(window, edge);
    }

    fn clear_edge_clip(&mut self, window: &Window, cx: &mut Context<Self>) {
        let clip = DockClip::full(window.bounds());
        if self.clip != clip {
            self.clip = clip;
            cx.notify();
        }
        #[cfg(target_os = "windows")]
        apply_native_content_clip(window);
    }

    fn prepare_drag(&mut self, window: &Window, cx: &mut Context<Self>) {
        self.cancel_conceal_guard();
        self.cancel_reveal_watch();
        let clip = DockClip::full(window.bounds());
        if self.clip != clip {
            self.clip = clip;
            cx.notify();
        }
        #[cfg(target_os = "windows")]
        clear_native_clip(window);
    }

    fn cancel_conceal_guard(&mut self) {
        self.conceal_revision = self.conceal_revision.wrapping_add(1);
        self.conceal_until_hover_exit = false;
    }

    fn cancel_reveal_watch(&mut self) {
        self.reveal_revision = self.reveal_revision.wrapping_add(1);
    }

    fn watch_revealed_pointer_exit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.reveal_revision = self.reveal_revision.wrapping_add(1);
        let revision = self.reveal_revision;
        cx.spawn_in(window, async move |this, cx| {
            // Let the reveal animation settle before checking the final window bounds.
            cx.background_executor()
                .timer(std::time::Duration::from_millis(CONCEAL_GUARD_SETTLE_MS))
                .await;
            loop {
                let keep_waiting = this
                    .update_in(cx, |this, window, cx| {
                        if this.reveal_revision != revision
                            || !this.revealed
                            || this.moving
                            || !this.state.read(cx).settings.docked
                            || this.state.read(cx).settings.dock_edge.is_none()
                        {
                            return false;
                        }
                        if pointer_inside_dock(window) {
                            return true;
                        }
                        this.set_revealed(false, window, cx);
                        false
                    })
                    .unwrap_or(false);
                if !keep_waiting {
                    break;
                }
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(REVEAL_EXIT_POLL_MS))
                    .await;
            }
        })
        .detach();
    }

    fn guard_conceal_until_pointer_exit(
        &mut self,
        edge: DockEdge,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.conceal_revision = self.conceal_revision.wrapping_add(1);
        let revision = self.conceal_revision;
        self.conceal_until_hover_exit = pointer_inside_dock(window);
        if !self.conceal_until_hover_exit {
            return;
        }

        cx.spawn_in(window, async move |this, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(CONCEAL_GUARD_SETTLE_MS))
                .await;
            loop {
                let keep_waiting = this
                    .update_in(cx, |this, window, _| {
                        if this.conceal_revision != revision || !this.conceal_until_hover_exit {
                            return false;
                        }
                        if pointer_inside_concealed_dock(window, edge) {
                            return true;
                        }
                        this.conceal_until_hover_exit = false;
                        false
                    })
                    .unwrap_or(false);
                if !keep_waiting {
                    break;
                }
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(CONCEAL_GUARD_POLL_MS))
                    .await;
            }
        })
        .detach();
    }

    fn schedule_return_snap(
        &mut self,
        edge: DockEdge,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.snap_revision = self.snap_revision.wrapping_add(1);
        let revision = self.snap_revision;
        cx.spawn_in(window, async move |this, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(DOCK_RETURN_SNAP_DELAY_MS))
                .await;
            let _ = this.update_in(cx, |this, window, cx| {
                if this.snap_revision != revision
                    || this.moving
                    || !this.state.read(cx).settings.docked
                    || this.state.read(cx).settings.dock_edge != Some(edge)
                {
                    return;
                }
                this.guard_conceal_until_pointer_exit(edge, window, cx);
                this.revealed = true;
                this.set_revealed(false, window, cx);
            });
        })
        .detach();
    }

    fn finish_dock_move(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.state.read(cx).settings.docked {
            return;
        }
        let bounds = window.bounds();
        let edge = crate::ui::can_position_window(window)
            .then(|| detect_snap_edge(window, cx))
            .flatten();
        let geometry = edge.and_then(|edge| {
            let area = work_area(window, cx)?;
            let origin = edge_window_origin(window, edge, false, area);
            Some((area, origin))
        });
        self.edge_area = geometry.map(|(area, _)| area);
        let visible_origin = geometry.map_or(bounds.origin, |(_, origin)| origin);
        self.state.update(cx, |state, _| {
            state.settings.dock_edge = edge;
            state.settings.dock_on_edge = edge.is_some();
            state.dock_position = Some((f32::from(visible_origin.x), f32::from(visible_origin.y)));
            let _ = state.persist_settings();
        });
        self.revealed = true;
        if let Some(edge) = edge {
            self.guard_conceal_until_pointer_exit(edge, window, cx);
            self.set_revealed(false, window, cx);
        } else {
            self.cancel_conceal_guard();
            self.clear_edge_clip(window, cx);
        }
    }
}

impl Render for DockWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        #[cfg(target_os = "windows")]
        self.native_menu.update_labels(self.state.read(cx));
        let clip = self.clip;
        let shell = div()
            .id("dock-shell")
            .absolute()
            .left(px(clip.left))
            .top(px(clip.top))
            .w(px(clip.width))
            .h(px(clip.height))
            .overflow_hidden()
            .cursor_pointer()
            .child(
                div()
                    .absolute()
                    .left(px(-clip.left))
                    .top(px(-clip.top))
                    .size(px(DOCK_WINDOW_SIZE))
                    .rounded_full()
                    .overflow_hidden()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(q_mark(DOCK_WINDOW_SIZE).border_0()),
            )
            .on_hover(cx.listener(|this, hovered: &bool, window, cx| {
                if this.conceal_until_hover_exit {
                    if !*hovered {
                        this.cancel_conceal_guard();
                    }
                    return;
                }
                this.set_revealed(*hovered, window, cx);
            }))
            .on_mouse_down_out(cx.listener(|this, _, window, cx| {
                if this.manual_move {
                    this.manual_move = false;
                    this.moving = false;
                    this.finish_dock_move(window, cx);
                }
                this.pointer_origin = None;
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &gpui::MouseDownEvent, _, _| {
                    this.snap_revision = this.snap_revision.wrapping_add(1);
                    this.motion_revision = this.motion_revision.wrapping_add(1);
                    this.moving = false;
                    this.manual_move = false;
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
                        if this.manual_move {
                            this.manual_move = false;
                            this.moving = false;
                            this.finish_dock_move(window, cx);
                        }
                        return;
                    }
                    let dx = f32::from(event.position.x - origin.x);
                    let dy = f32::from(event.position.y - origin.y);
                    if this.manual_move {
                        let bounds = window.bounds();
                        let target = point(bounds.origin.x + px(dx), bounds.origin.y + px(dy));
                        let _ = crate::ui::move_window_to(window, target);
                        return;
                    }
                    if dx * dx + dy * dy < DRAG_THRESHOLD * DRAG_THRESHOLD {
                        return;
                    }

                    this.prepare_drag(window, cx);
                    this.moving = true;
                    #[cfg(target_os = "windows")]
                    {
                        this.pointer_origin = None;
                        if !start_window_move(window) {
                            this.moving = false;
                            this.finish_dock_move(window, cx);
                            return;
                        }
                        // WM_NCLBUTTONDOWN returns after the native move loop ends.
                        this.moving = false;
                        this.finish_dock_move(window, cx);
                    }
                    #[cfg(not(target_os = "windows"))]
                    {
                        if crate::ui::can_position_window(window) {
                            this.manual_move = true;
                            let bounds = window.bounds();
                            let target = point(bounds.origin.x + px(dx), bounds.origin.y + px(dy));
                            let _ = crate::ui::move_window_to(window, target);
                        } else {
                            this.pointer_origin = None;
                            window.start_window_move();
                        }
                    }
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| {
                    if this.manual_move {
                        this.pointer_origin = None;
                        this.manual_move = false;
                        this.moving = false;
                        this.finish_dock_move(window, cx);
                        return;
                    }
                    if this.pointer_origin.take().is_some() {
                        main_window::restore_from_dock(this.state.clone(), cx);
                    }
                }),
            );

        #[cfg(target_os = "windows")]
        let shell = shell.on_mouse_up(
            MouseButton::Right,
            cx.listener(|this, _, window, _| {
                let _ = this.native_menu.show(window);
            }),
        );

        #[cfg(not(target_os = "windows"))]
        let shell = shell.context_menu({
            let state = self.state.clone();
            move |menu, _, cx| {
                let app = state.read(cx);
                let tr = app.tr();
                let topmost_label = if app.settings.always_on_top {
                    tr.always_off
                } else {
                    tr.always_on
                };
                menu.item(PopupMenuItem::new(topmost_label).on_click({
                    let state = state.clone();
                    move |_, _, cx| {
                        state.update(cx, |s, cx| {
                            let next = !s.settings.always_on_top;
                            s.set_always_on_top(next, cx);
                        });
                    }
                }))
                .item(PopupMenuItem::new(tr.switch_language).on_click({
                    let state = state.clone();
                    move |_, window, cx| {
                        state.update(cx, |s, cx| s.toggle_language(cx));
                        window.refresh();
                    }
                }))
                .item(PopupMenuItem::new(tr.switch_main_window).on_click({
                    let state = state.clone();
                    move |_, _, cx| {
                        main_window::restore_from_dock(state.clone(), cx);
                    }
                }))
                .item(PopupMenuItem::new(tr.quit).on_click({
                    let state = state.clone();
                    move |_, _, cx| {
                        let _ = main_window::prepare_for_shutdown(&state, cx);
                        cx.quit();
                    }
                }))
            }
        });

        div().relative().size_full().child(shell)
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

#[cfg(target_os = "windows")]
fn disable_native_frame(window: &Window) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::Graphics::Dwm::{
        DWMNCRP_DISABLED, DWMWA_BORDER_COLOR, DWMWA_COLOR_NONE, DWMWA_NCRENDERING_POLICY,
        DwmSetWindowAttribute,
    };

    let Ok(handle) = HasWindowHandle::window_handle(window) else {
        return;
    };
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return;
    };
    let color = DWMWA_COLOR_NONE;
    let rendering_policy = DWMNCRP_DISABLED;
    unsafe {
        let _ = DwmSetWindowAttribute(
            handle.hwnd.get() as HWND,
            DWMWA_BORDER_COLOR as u32,
            &color as *const _ as *const std::ffi::c_void,
            std::mem::size_of_val(&color) as u32,
        );
        let _ = DwmSetWindowAttribute(
            handle.hwnd.get() as HWND,
            DWMWA_NCRENDERING_POLICY as u32,
            &rendering_policy as *const _ as *const std::ffi::c_void,
            std::mem::size_of_val(&rendering_policy) as u32,
        );
    }
}

#[cfg(target_os = "windows")]
#[derive(Clone, Copy)]
struct NativeDockGeometry {
    hwnd: windows_sys::Win32::Foundation::HWND,
    window_left: i32,
    window_top: i32,
    client_left: i32,
    client_top: i32,
    client_width: i32,
    client_height: i32,
}

#[cfg(target_os = "windows")]
fn native_dock_geometry(window: &Window) -> Option<NativeDockGeometry> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::Foundation::{HWND, POINT, RECT};
    use windows_sys::Win32::Graphics::Gdi::ClientToScreen;
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetClientRect, GetWindowRect};

    let handle = HasWindowHandle::window_handle(window).ok()?;
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return None;
    };
    let hwnd = handle.hwnd.get() as HWND;
    let mut window_rect = RECT::default();
    let mut client_rect = RECT::default();
    let mut client_origin = POINT::default();
    if unsafe { GetWindowRect(hwnd, &mut window_rect) } == 0 {
        return None;
    }
    if unsafe { GetClientRect(hwnd, &mut client_rect) } == 0 {
        return None;
    }
    if unsafe { ClientToScreen(hwnd, &mut client_origin) } == 0 {
        return None;
    }
    let client_width = client_rect.right - client_rect.left;
    let client_height = client_rect.bottom - client_rect.top;
    if client_width <= 0 || client_height <= 0 {
        return None;
    }

    Some(NativeDockGeometry {
        hwnd,
        window_left: window_rect.left,
        window_top: window_rect.top,
        client_left: client_origin.x,
        client_top: client_origin.y,
        client_width,
        client_height,
    })
}

#[cfg(target_os = "windows")]
fn native_edge_work_area(
    geometry: NativeDockGeometry,
    edge: DockEdge,
) -> Option<windows_sys::Win32::Foundation::RECT> {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint,
    };

    // Probe the half that belongs to the docked display. This avoids an
    // ambiguous monitor choice when the hidden window straddles two screens.
    let probe = POINT {
        x: match edge {
            DockEdge::Left => geometry.client_left + geometry.client_width * 3 / 4,
            DockEdge::Right => geometry.client_left + geometry.client_width / 4,
            DockEdge::Top | DockEdge::Bottom => geometry.client_left + geometry.client_width / 2,
        },
        y: match edge {
            DockEdge::Top => geometry.client_top + geometry.client_height * 3 / 4,
            DockEdge::Bottom => geometry.client_top + geometry.client_height / 4,
            DockEdge::Left | DockEdge::Right => geometry.client_top + geometry.client_height / 2,
        },
    };
    let monitor = unsafe { MonitorFromPoint(probe, MONITOR_DEFAULTTONEAREST) };
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
    Some(info.rcWork)
}

#[cfg(target_os = "windows")]
fn set_native_clip_region(
    geometry: NativeDockGeometry,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
) {
    use windows_sys::Win32::Graphics::Gdi::{CreateRectRgn, DeleteObject, SetWindowRgn};

    if right <= left || bottom <= top {
        return;
    }
    let region = unsafe { CreateRectRgn(left, top, right, bottom) };
    if region.is_null() {
        return;
    }
    // After a successful SetWindowRgn call, Windows owns the region handle.
    if unsafe { SetWindowRgn(geometry.hwnd, region, 1) } == 0 {
        let _ = unsafe { DeleteObject(region) };
    }
}

#[cfg(target_os = "windows")]
fn apply_native_edge_clip(window: &Window, edge: DockEdge) {
    let Some(geometry) = native_dock_geometry(window) else {
        return;
    };
    let Some(work) = native_edge_work_area(geometry, edge) else {
        return;
    };
    let client_right = geometry.client_left + geometry.client_width;
    let client_bottom = geometry.client_top + geometry.client_height;
    let left = work.left.max(geometry.client_left) - geometry.window_left;
    let top = work.top.max(geometry.client_top) - geometry.window_top;
    let right = work.right.min(client_right) - geometry.window_left;
    let bottom = work.bottom.min(client_bottom) - geometry.window_top;
    if right <= left || bottom <= top {
        apply_native_content_clip(window);
        return;
    }
    set_native_clip_region(geometry, left, top, right, bottom);
}

#[cfg(target_os = "windows")]
fn apply_native_content_clip(window: &Window) {
    let Some(geometry) = native_dock_geometry(window) else {
        return;
    };
    let left = geometry.client_left - geometry.window_left;
    let top = geometry.client_top - geometry.window_top;
    set_native_clip_region(
        geometry,
        left,
        top,
        left + geometry.client_width,
        top + geometry.client_height,
    );
}

#[cfg(target_os = "windows")]
fn clear_native_clip(window: &Window) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::Graphics::Gdi::SetWindowRgn;

    let Ok(handle) = HasWindowHandle::window_handle(window) else {
        return;
    };
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return;
    };
    unsafe {
        let _ = SetWindowRgn(handle.hwnd.get() as HWND, std::ptr::null_mut(), 1);
    }
}

fn pointer_inside_dock(window: &Window) -> bool {
    let pointer = window.mouse_position();
    let x = f32::from(pointer.x);
    let y = f32::from(pointer.y);
    x >= 0.0 && x < DOCK_WINDOW_SIZE && y >= 0.0 && y < DOCK_WINDOW_SIZE
}

fn pointer_inside_concealed_dock(window: &Window, edge: DockEdge) -> bool {
    let pointer = window.mouse_position();
    let x = f32::from(pointer.x);
    let y = f32::from(pointer.y);
    let half = DOCK_WINDOW_SIZE / 2.0;
    let inside_cross_axis = match edge {
        DockEdge::Left | DockEdge::Right => y >= 0.0 && y < DOCK_WINDOW_SIZE,
        DockEdge::Top | DockEdge::Bottom => x >= 0.0 && x < DOCK_WINDOW_SIZE,
    };
    let inside_visible_half = match edge {
        DockEdge::Left => x >= half && x < DOCK_WINDOW_SIZE,
        DockEdge::Right => x >= 0.0 && x < half,
        DockEdge::Top => y >= half && y < DOCK_WINDOW_SIZE,
        DockEdge::Bottom => y >= 0.0 && y < half,
    };
    inside_cross_axis && inside_visible_half
}

fn detect_snap_edge(window: &Window, cx: &App) -> Option<DockEdge> {
    let bounds = window.bounds();
    let area = work_area(window, cx)?;
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

fn work_area(window: &Window, cx: &App) -> Option<Bounds<gpui::Pixels>> {
    #[cfg(target_os = "windows")]
    if let Some(area) = windows_work_area(window) {
        return Some(area);
    }

    window.display(cx).map(|display| display.bounds())
}

#[cfg(target_os = "windows")]
fn windows_work_area(window: &Window) -> Option<Bounds<gpui::Pixels>> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow,
    };

    let handle = HasWindowHandle::window_handle(window).ok()?;
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

    let scale = window.scale_factor();
    if !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    let rect = info.rcWork;
    if rect.right <= rect.left || rect.bottom <= rect.top {
        return None;
    }

    Some(Bounds {
        origin: point(px(rect.left as f32 / scale), px(rect.top as f32 / scale)),
        size: size(
            px((rect.right - rect.left) as f32 / scale),
            px((rect.bottom - rect.top) as f32 / scale),
        ),
    })
}

fn edge_window_origin(
    window: &Window,
    edge: DockEdge,
    hidden: bool,
    area: Bounds<gpui::Pixels>,
) -> gpui::Point<gpui::Pixels> {
    #[cfg(target_os = "windows")]
    if let Some(origin) = windows_edge_origin(window, edge, hidden) {
        return origin;
    }

    edge_origin(edge, hidden, window.bounds(), area)
}

#[cfg(target_os = "windows")]
fn windows_edge_origin(
    window: &Window,
    edge: DockEdge,
    hidden: bool,
) -> Option<gpui::Point<gpui::Pixels>> {
    let geometry = native_dock_geometry(window)?;
    let work = native_edge_work_area(geometry, edge)?;
    let max_client_left = (work.right - geometry.client_width).max(work.left);
    let max_client_top = (work.bottom - geometry.client_height).max(work.top);
    let current_client_left = geometry.client_left.clamp(work.left, max_client_left);
    let current_client_top = geometry.client_top.clamp(work.top, max_client_top);
    let hidden_left = geometry.client_width / 2;
    let hidden_right = (geometry.client_width + 1) / 2;
    let hidden_top = geometry.client_height / 2;
    let hidden_bottom = (geometry.client_height + 1) / 2;

    let (client_left, client_top) = match edge {
        DockEdge::Left => (
            work.left - if hidden { hidden_left } else { 0 },
            current_client_top,
        ),
        DockEdge::Right => (
            work.right
                - if hidden {
                    hidden_right
                } else {
                    geometry.client_width
                },
            current_client_top,
        ),
        DockEdge::Top => (
            current_client_left,
            work.top - if hidden { hidden_top } else { 0 },
        ),
        DockEdge::Bottom => (
            current_client_left,
            work.bottom
                - if hidden {
                    hidden_bottom
                } else {
                    geometry.client_height
                },
        ),
    };
    let client_offset_x = geometry.client_left - geometry.window_left;
    let client_offset_y = geometry.client_top - geometry.window_top;
    let scale = window.scale_factor();
    if !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    Some(point(
        px((client_left - client_offset_x) as f32 / scale),
        px((client_top - client_offset_y) as f32 / scale),
    ))
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
    let reveal_anchor = state.update(cx, |s, _| {
        s.dock_reveal_anchor
            .take()
            .filter(|anchor| anchor.saved_at.elapsed() <= DOCK_REVEAL_ANCHOR_MAX_AGE)
    });
    let main = state.read(cx).main_window;
    let snapshot = main.and_then(|main| {
        main.update(cx, |_, window, _| {
            let b = window.bounds();
            WindowState {
                width: f32::from(b.size.width),
                height: f32::from(b.size.height),
                x: f32::from(b.origin.x),
                y: f32::from(b.origin.y),
            }
        })
        .ok()
    });

    state.update(cx, |s, _| {
        if let Some(snapshot) = snapshot {
            s.settings.window = Some(snapshot);
        }
        if let Some(anchor) = reveal_anchor {
            s.settings.dock_edge = Some(anchor.edge);
            s.settings.dock_on_edge = true;
            s.dock_position = Some(anchor.position);
        } else if s.settings.dock_edge.is_none() {
            s.dock_position = None;
        }
        s.settings.docked = true;
        s.settings.keep_full_main = false;
        let _ = s.persist_settings();
        crate::tray::update_labels(s);
    });

    // Keep the GPUI main window alive, matching the Tauri lifecycle. This
    // preserves its view state and lets restore simply reveal the same window.
    if let Some(main) = main {
        let hidden = main
            .update(cx, |_, window, _| {
                crate::ui::set_window_visible(window, false)
            })
            .unwrap_or(false);
        if !hidden {
            // Wayland does not provide a generic unhide operation in GPUI.
            // Recreate the main window on restore instead of leaving it
            // permanently minimized.
            let _ = main.update(cx, |_, window, _| window.remove_window());
            state.update(cx, |s, _| s.main_window = None);
        }
    }
    open_dock_window(state, reveal_anchor.map(|anchor| anchor.edge), cx);
}

fn open_dock_window(state: Entity<AppState>, return_snap_edge: Option<DockEdge>, cx: &mut App) {
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
                move |window, cx| cx.new(|cx| DockWindow::new(state.clone(), window, cx))
            },
        )
        .expect("open dock");

    state.update(cx, |s, _| s.dock_window = Some(handle.into()));
    let always_on_top = state.read(cx).settings.always_on_top;
    let dock_edge = state.read(cx).settings.dock_edge;
    let _ = handle.update(cx, |dock, window, cx| {
        #[cfg(target_os = "windows")]
        disable_native_frame(window);
        crate::ui::apply_window_topmost(window, always_on_top);
        if crate::ui::can_position_window(window)
            && let Some(edge) = dock_edge
            && let Some(area) = dock.edge_area.or_else(|| work_area(window, cx))
        {
            dock.edge_area = Some(area);
            let returning_to_edge = return_snap_edge == Some(edge);
            dock.revealed = returning_to_edge;
            let target = edge_window_origin(window, edge, !returning_to_edge, area);
            let _ = crate::ui::move_window_to(window, target);
            dock.apply_edge_clip(target, area, edge, window, cx);
            if returning_to_edge {
                dock.schedule_return_snap(edge, window, cx);
            }
            return;
        }
        dock.clear_edge_clip(window, cx);
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
    let origin = state
        .settings
        .dock_edge
        .map_or(base_origin, |edge| edge_origin(edge, false, base, area));
    Bounds { origin, size }
}
