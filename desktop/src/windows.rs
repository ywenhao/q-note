use crate::host::{self, Dock};
use ::windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
        Graphics::Gdi::{
            CombineRgn, CreateEllipticRgn, CreateRectRgn, DeleteObject, GetMonitorInfoW,
            MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint, MonitorFromWindow, RGN_AND,
            SetWindowRgn,
        },
        System::Ole::{OleInitialize, OleUninitialize},
        UI::{
            HiDpi::{
                DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForWindow,
                SetProcessDpiAwarenessContext,
            },
            Shell::{
                DefSubclassProc, DragAcceptFiles, DragFinish, DragQueryFileW, HDROP,
                SetWindowSubclass,
            },
            WindowsAndMessaging::*,
        },
    },
    core::HSTRING,
};
use nsh_platform_windows::{AppOptions, AppView, Appearance, Limits};
use q_note_native::{
    app_state,
    geometry::{self, Rect},
};
use serde_json::json;
use std::{
    collections::HashMap,
    rc::Rc,
    time::{Duration, Instant},
};
use tray_icon::{
    Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuItem},
};

struct Window {
    view: Rc<AppView>,
}
struct Host {
    windows: HashMap<String, Window>,
    dock: Dock,
    tray: TrayIcon,
    items: Vec<MenuItem>,
    last_frame: Option<Rect>,
    changed: Option<Instant>,
    cursor: Option<POINT>,
    grab: (f64, f64),
    dock_scale: f64,
}

pub fn run() -> anyhow::Result<()> {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        OleInitialize(None)?;
    }
    nsh_platform_windows::set_ui_sets(vec![
        nsh_ui_mobile::registry(),
        q_note_native::__nsh_native_module(),
    ]);
    let menu = Menu::new();
    let items: Vec<_> = host::tray_labels(false)
        .iter()
        .map(|label| MenuItem::new(label, true, None))
        .collect();
    for item in &items {
        menu.append(item)?;
    }
    let image = image::load_from_memory(include_bytes!("../../app-icon.png"))?
        .resize_exact(32, 32, image::imageops::FilterType::Lanczos3)
        .to_rgba8();
    let tray = TrayIconBuilder::new()
        .with_tooltip("Q Note")
        .with_icon(Icon::from_rgba(image.into_raw(), 32, 32)?)
        .with_menu(Box::new(menu))
        .with_menu_on_left_click(false)
        .build()?;
    let mut host = Host {
        windows: HashMap::new(),
        dock: Dock::default(),
        tray,
        items,
        last_frame: None,
        changed: None,
        cursor: None,
        grab: (0.0, 0.0),
        dock_scale: 1.0,
    };
    host.open(
        "main",
        geometry::main_frame(app_state::settings().window.as_ref(), area(None)),
    )?;
    if app_state::has_editor() {
        host.edit()?;
    }
    app_state::start_update_schedule();
    let mut msg = MSG::default();
    let mut quitting = false;
    while !quitting {
        unsafe {
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                if msg.message == WM_QUIT {
                    quitting = true;
                    break;
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if let Some(index) = host.items.iter().position(|item| item.id() == &event.id) {
                host::tray_action(index);
            }
        }
        while let Ok(event) = TrayIconEvent::receiver().try_recv() {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                host.show_main();
            }
        }
        host.tick();
        std::thread::sleep(Duration::from_millis(8));
    }
    for window in host.windows.values() {
        window.view.stop();
        unsafe {
            let _ = DestroyWindow(window.view.hwnd());
        }
    }
    drop(host.tray);
    unsafe {
        OleUninitialize();
    }
    Ok(())
}

impl Host {
    fn hwnd(&self, role: &str) -> Option<HWND> {
        self.windows.get(role).map(|w| w.view.hwnd())
    }
    fn open(&mut self, role: &str, frame: Rect) -> anyhow::Result<()> {
        if let Some(hwnd) = self.hwnd(role) {
            self.set_frame(role, frame);
            activate(hwnd);
            return Ok(());
        }
        let view = AppView::new(
            None,
            AppOptions {
                title: host::title(role),
                app_id: "com.win11.q-note".into(),
                size: (frame.width as i32, frame.height as i32),
                appearance: Appearance::Light,
                limits: Limits::standalone_desktop(),
                ..Default::default()
            },
        )?;
        let hwnd = view.hwnd();
        let id = match role {
            "editor" => 2,
            "dock" => 3,
            _ => 1,
        };
        unsafe {
            let style = if role == "dock" {
                WS_POPUP
            } else {
                WS_POPUP | WS_THICKFRAME | WS_MINIMIZEBOX
            };
            SetWindowLongPtrW(hwnd, GWL_STYLE, style.0 as isize);
            if role == "dock" {
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, WS_EX_TOOLWINDOW.0 as isize);
            }
            SetWindowSubclass(hwnd, Some(subclass), id, 0).ok()?;
            DragAcceptFiles(hwnd, role == "editor");
        }
        view.load(host::bundle(role)?.into());
        view.show();
        self.windows.insert(role.into(), Window { view });
        self.set_frame(role, frame);
        self.topmost(role);
        self.clip(role);
        activate(hwnd);
        Ok(())
    }
    fn frame(&self, role: &str) -> Rect {
        let Some(hwnd) = self.hwnd(role) else {
            return Rect {
                x: 0.0,
                y: 0.0,
                width: 300.0,
                height: 450.0,
            };
        };
        let scale = dpi(hwnd);
        let mut rect = RECT::default();
        unsafe {
            let _ = GetWindowRect(hwnd, &mut rect);
        }
        Rect {
            x: rect.left as f64 / scale,
            y: rect.top as f64 / scale,
            width: (rect.right - rect.left) as f64 / scale,
            height: (rect.bottom - rect.top) as f64 / scale,
        }
    }
    fn set_frame(&self, role: &str, frame: Rect) {
        if let Some(hwnd) = self.hwnd(role) {
            let scale = if role == "dock" {
                self.dock_scale
            } else {
                dpi(hwnd)
            };
            unsafe {
                let _ = SetWindowPos(
                    hwnd,
                    None,
                    (frame.x * scale).round() as i32,
                    (frame.y * scale).round() as i32,
                    (frame.width * scale).round() as i32,
                    (frame.height * scale).round() as i32,
                    SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
                );
            }
            self.clip(role);
        }
    }
    fn clip(&self, role: &str) {
        if let Some(hwnd) = self.hwnd(role) {
            unsafe {
                let frame = self.frame(role);
                let scale = dpi(hwnd);
                let w = (frame.width * scale).round() as i32;
                let h = (frame.height * scale).round() as i32;
                if role == "dock" {
                    let round = CreateEllipticRgn(0, 0, w, h);
                    let work = area_at(POINT {
                        x: ((self.dock.full.x + 15.0) * self.dock_scale) as i32,
                        y: ((self.dock.full.y + 15.0) * self.dock_scale) as i32,
                    })
                    .0;
                    let cut = CreateRectRgn(
                        ((work.x - frame.x) * scale).ceil() as i32,
                        ((work.y - frame.y) * scale).ceil() as i32,
                        ((work.x + work.width - frame.x) * scale).floor() as i32,
                        ((work.y + work.height - frame.y) * scale).floor() as i32,
                    );
                    CombineRgn(Some(round), Some(round), Some(cut), RGN_AND);
                    let _ = DeleteObject(cut.into());
                    let _ = SetWindowRgn(hwnd, Some(round), true);
                } else {
                    let radius = (24.0 * scale).round() as i32;
                    let round = ::windows::Win32::Graphics::Gdi::CreateRoundRectRgn(
                        0,
                        0,
                        w + 1,
                        h + 1,
                        radius,
                        radius,
                    );
                    let _ = SetWindowRgn(hwnd, Some(round), true);
                }
            }
        }
    }
    fn topmost(&self, role: &str) {
        if let Some(hwnd) = self.hwnd(role) {
            unsafe {
                let _ = SetWindowPos(
                    hwnd,
                    Some(if app_state::settings().always_on_top {
                        HWND_TOPMOST
                    } else {
                        HWND_NOTOPMOST
                    }),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
                let _ = SetWindowTextW(hwnd, &HSTRING::from(host::title(role)));
            }
        }
    }
    fn show_main(&mut self) {
        self.dock.restore();
        if let Some(hwnd) = self.hwnd("dock") {
            unsafe {
                let _ = ShowWindow(hwnd, SW_HIDE);
            }
        }
        if let Some(hwnd) = self.hwnd("main") {
            activate(hwnd);
        }
        self.update_tray();
    }
    fn edit(&mut self) -> anyhow::Result<()> {
        self.show_main();
        self.open(
            "editor",
            geometry::editor_frame(self.frame("main"), area(self.hwnd("main"))),
        )
    }
    fn toggle_dock(&mut self) -> anyhow::Result<()> {
        if self.dock.active {
            self.show_main();
            return Ok(());
        }
        self.save_window()?;
        let work = if let Some(anchor) = self.dock.anchor() {
            let (work, scale) = area_at(POINT {
                x: ((anchor.x + 15.0) * self.dock_scale) as i32,
                y: ((anchor.y + 15.0) * self.dock_scale) as i32,
            });
            self.dock.full.x *= self.dock_scale / scale;
            self.dock.full.y *= self.dock_scale / scale;
            self.dock_scale = scale;
            work
        } else {
            self.dock_scale = self.hwnd("main").map(dpi).unwrap_or(1.0);
            area(self.hwnd("main"))
        };
        self.dock.open(self.frame("main"), work);
        self.open("dock", self.dock.frame)?;
        if let Some(hwnd) = self.hwnd("main") {
            unsafe {
                let _ = ShowWindow(hwnd, SW_HIDE);
            }
        }
        app_state::event("dock", json!({"kind":"showDock"}));
        self.update_tray();
        Ok(())
    }
    fn save_window(&self) -> anyhow::Result<()> {
        app_state::save_window(self.frame("main").state())
    }
    fn update_tray(&self) {
        for (item, label) in self.items.iter().zip(host::tray_labels(self.dock.active)) {
            item.set_text(label);
        }
    }
    fn tick(&mut self) {
        for command in app_state::take_commands() {
            match command.action.as_str() {
                "show" => self.show_main(),
                "edit" => {
                    if let Err(e) = self.edit() {
                        eprintln!("{e}");
                    }
                }
                "dock" => {
                    if let Err(e) = self.toggle_dock() {
                        eprintln!("{e}");
                    }
                }
                "close" => {
                    if command.window == "main" {
                        app_state::report_save_error(self.save_window());
                    }
                    if let Some(hwnd) = self.hwnd(&command.window) {
                        unsafe {
                            let _ = ShowWindow(hwnd, SW_HIDE);
                        }
                    }
                }
                "minimize" => {
                    if let Some(hwnd) = self.hwnd(&command.window) {
                        unsafe {
                            let _ = ShowWindow(hwnd, SW_MINIMIZE);
                        }
                    }
                }
                "settings" => {
                    for role in self.windows.keys() {
                        self.topmost(role);
                    }
                    self.update_tray();
                }
                "saveWindow" => app_state::window_saved(self.save_window()),
                "quit" => {
                    let saved = self.save_window().and_then(|_| app_state::flush_for_exit());
                    if saved.is_ok() {
                        unsafe {
                            PostQuitMessage(0);
                        }
                    } else {
                        app_state::report_save_error(saved);
                    }
                }
                "hoverDock" => self
                    .dock
                    .hover(command.payload["inside"].as_bool().unwrap_or(false)),
                "beginDockDrag" => {
                    let mut point = POINT::default();
                    unsafe {
                        let _ = GetCursorPos(&mut point);
                    }
                    self.cursor = Some(point);
                    self.grab = (
                        point.x as f64 / self.dock_scale - self.dock.full.x,
                        point.y as f64 / self.dock_scale - self.dock.full.y,
                    );
                }
                "dragDock" => {
                    let mut cursor = POINT::default();
                    unsafe {
                        let _ = GetCursorPos(&mut cursor);
                    }
                    let previous = self.cursor.replace(cursor);
                    if previous.is_some() {
                        let (work, scale) = area_at(cursor);
                        self.dock_scale = scale;
                        self.dock.drag(
                            cursor.x as f64 / scale - self.grab.0 - self.dock.full.x,
                            cursor.y as f64 / scale - self.grab.1 - self.dock.full.y,
                            work,
                        );
                        self.set_frame("dock", self.dock.frame);
                    } else {
                        self.dock.drag(
                            command.payload["dx"].as_f64().unwrap_or(0.0),
                            command.payload["dy"].as_f64().unwrap_or(0.0),
                            area(self.hwnd("dock")),
                        );
                    }
                }
                "endDockDrag" => {
                    self.cursor = None;
                    if command.payload["dragged"].as_bool().unwrap_or(false) {
                        let mut point = POINT::default();
                        unsafe {
                            let _ = GetCursorPos(&mut point);
                        }
                        self.dock.release(area_at(point).0);
                        self.set_frame("dock", self.dock.frame);
                    }
                }
                _ => {}
            }
        }
        if self.dock.tick() {
            self.set_frame("dock", self.dock.frame);
        }
        if let Some(hwnd) = self.hwnd("main") {
            if unsafe { IsWindowVisible(hwnd).as_bool() && !IsIconic(hwnd).as_bool() } {
                let frame = self.frame("main");
                if self.last_frame != Some(frame) {
                    self.last_frame = Some(frame);
                    self.changed = Some(Instant::now());
                    self.clip("main");
                }
                if self
                    .changed
                    .is_some_and(|time| time.elapsed() >= Duration::from_millis(300))
                {
                    app_state::report_save_error(self.save_window());
                    self.changed = None;
                }
            }
        }
    }
}

fn dpi(hwnd: HWND) -> f64 {
    (unsafe { GetDpiForWindow(hwnd) } as f64 / 96.0).max(1.0)
}
fn area_at(point: POINT) -> (Rect, f64) {
    unsafe {
        let monitor = MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let _ = GetMonitorInfoW(monitor, &mut info);
        let (mut x, mut y) = (96, 96);
        let _ = ::windows::Win32::UI::HiDpi::GetDpiForMonitor(
            monitor,
            ::windows::Win32::UI::HiDpi::MDT_EFFECTIVE_DPI,
            &mut x,
            &mut y,
        );
        let scale = (x as f64 / 96.0).max(1.0);
        let r = info.rcWork;
        (
            Rect {
                x: r.left as f64 / scale,
                y: r.top as f64 / scale,
                width: (r.right - r.left) as f64 / scale,
                height: (r.bottom - r.top) as f64 / scale,
            },
            scale,
        )
    }
}
fn area(hwnd: Option<HWND>) -> Rect {
    unsafe {
        let monitor = if let Some(hwnd) = hwnd {
            MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST)
        } else {
            MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTONEAREST)
        };
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let _ = GetMonitorInfoW(monitor, &mut info);
        let scale = hwnd.map(dpi).unwrap_or_else(|| {
            let mut x = 96;
            let mut y = 96;
            let _ = ::windows::Win32::UI::HiDpi::GetDpiForMonitor(
                monitor,
                ::windows::Win32::UI::HiDpi::MDT_EFFECTIVE_DPI,
                &mut x,
                &mut y,
            );
            x as f64 / 96.0
        });
        let r = info.rcWork;
        Rect {
            x: r.left as f64 / scale,
            y: r.top as f64 / scale,
            width: (r.right - r.left) as f64 / scale,
            height: (r.bottom - r.top) as f64 / scale,
        }
    }
}
fn activate(hwnd: HWND) {
    unsafe {
        let _ = ShowWindow(hwnd, SW_RESTORE);
        let _ = SetForegroundWindow(hwnd);
    }
}

unsafe extern "system" fn subclass(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    id: usize,
    _: usize,
) -> LRESULT {
    let role = match id {
        2 => "editor",
        3 => "dock",
        _ => "main",
    };
    match msg {
        WM_CLOSE => {
            let _ = app_state::invoke("close", json!({"window":role}));
            return LRESULT(0);
        }
        WM_NCCALCSIZE => return LRESULT(0),
        WM_NCHITTEST if id != 3 => {
            let mut rect = RECT::default();
            unsafe {
                let _ = GetWindowRect(hwnd, &mut rect);
            }
            let x = lp.0 as u16 as i16 as i32;
            let y = (lp.0 >> 16) as u16 as i16 as i32;
            let margin = (6.0 * dpi(hwnd)).round() as i32;
            let left = x < rect.left + margin;
            let right = x >= rect.right - margin;
            let top = y < rect.top + margin;
            let bottom = y >= rect.bottom - margin;
            let hit = match (left, right, top, bottom) {
                (true, _, true, _) => HTTOPLEFT,
                (_, true, true, _) => HTTOPRIGHT,
                (true, _, _, true) => HTBOTTOMLEFT,
                (_, true, _, true) => HTBOTTOMRIGHT,
                (true, _, _, _) => HTLEFT,
                (_, true, _, _) => HTRIGHT,
                (_, _, true, _) => HTTOP,
                (_, _, _, true) => HTBOTTOM,
                _ => HTCLIENT,
            };
            if hit != HTCLIENT {
                return LRESULT(hit as isize);
            }
        }
        WM_SYSCOMMAND if wp.0 & 0xfff0 == SC_MAXIMIZE as usize => return LRESULT(0),
        WM_NCLBUTTONDBLCLK | WM_LBUTTONDBLCLK if wp.0 == HTCAPTION as usize => return LRESULT(0),
        WM_GETMINMAXINFO => {
            unsafe {
                let info = &mut *(lp.0 as *mut MINMAXINFO);
                let scale = dpi(hwnd);
                let (minw, minh, maxw, maxh) = match id {
                    2 => (420, 520, 1000, 800),
                    3 => (30, 30, 30, 30),
                    _ => (300, 450, 1000, 800),
                };
                info.ptMinTrackSize = POINT {
                    x: (minw as f64 * scale) as i32,
                    y: (minh as f64 * scale) as i32,
                };
                info.ptMaxTrackSize = POINT {
                    x: (maxw as f64 * scale) as i32,
                    y: (maxh as f64 * scale) as i32,
                };
            }
            return LRESULT(0);
        }
        WM_RBUTTONUP => {
            let scale = dpi(hwnd);
            app_state::event(
                role,
                json!({"kind":"context","x":(lp.0 as u16 as i16)as f64/scale,"y":((lp.0>>16)as u16 as i16)as f64/scale}),
            );
            return LRESULT(0);
        }
        WM_MOUSEWHEEL if app_state::preview_open(role) => {
            app_state::event(
                role,
                json!({"kind":"wheel","delta":-((wp.0>>16)as u16 as i16)as f64/120.0}),
            );
            return LRESULT(0);
        }
        WM_DROPFILES if id == 2 => {
            unsafe {
                let drop = HDROP(wp.0 as *mut _);
                let count = DragQueryFileW(drop, u32::MAX, None);
                let mut paths = Vec::new();
                for i in 0..count {
                    let len = DragQueryFileW(drop, i, None);
                    let mut buffer = vec![0; len as usize + 1];
                    DragQueryFileW(drop, i, Some(&mut buffer));
                    paths.push(String::from_utf16_lossy(&buffer[..len as usize]));
                }
                DragFinish(drop);
                app_state::append_paths(paths);
            }
            return LRESULT(0);
        }
        WM_WINDOWPOSCHANGED if id != 3 => {}
        _ => {}
    }
    unsafe { DefSubclassProc(hwnd, msg, wp, lp) }
}
