pub mod dock_window;
pub mod editor_window;
pub mod main_window;
pub mod media;
pub mod modal;
pub mod style;
pub mod theme;
pub mod widgets;

use gpui::{App, Pixels, Point, Window, WindowKind};

#[cfg(target_os = "windows")]
const MAIN_WINDOW_SUBCLASS_ID: usize = 0x514E_4F54;

pub fn init(cx: &mut App) {
    theme::apply_q_note_theme(cx);
    widgets::init(cx);
}

pub(crate) fn standard_window_kind(always_on_top: bool) -> WindowKind {
    let _ = always_on_top;
    WindowKind::Normal
}

pub(crate) fn apply_window_topmost(window: &Window, enabled: bool) {
    #[cfg(target_os = "windows")]
    {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        use windows_sys::Win32::Foundation::HWND;
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos,
        };

        let Ok(handle) = HasWindowHandle::window_handle(window) else {
            return;
        };
        let RawWindowHandle::Win32(handle) = handle.as_raw() else {
            return;
        };
        let insert_after = if enabled {
            HWND_TOPMOST
        } else {
            HWND_NOTOPMOST
        };
        unsafe {
            let _ = SetWindowPos(
                handle.hwnd.get() as HWND,
                insert_after,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
    }
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::{NSFloatingWindowLevel, NSNormalWindowLevel};

        let _ = with_ns_window(window, |native_window| {
            native_window.setLevel(if enabled {
                NSFloatingWindowLevel
            } else {
                NSNormalWindowLevel
            });
        });
    }
    #[cfg(target_os = "linux")]
    {
        use x11rb::connection::Connection as _;
        use x11rb::protocol::xproto::{ClientMessageEvent, ConnectionExt as _, EventMask};

        let _ = with_x11_window(window, |native, xid| {
            let event = ClientMessageEvent::new(
                32,
                xid,
                native.wm_state,
                [u32::from(enabled), native.wm_state_above, 0, 1, 0],
            );
            native
                .connection
                .send_event(
                    false,
                    native.root,
                    EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
                    event,
                )
                .is_ok()
                && native.connection.flush().is_ok()
        });
    }
}

pub(crate) fn apply_main_window_constraints(window: &Window) {
    #[cfg(target_os = "windows")]
    {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        use windows_sys::Win32::Foundation::HWND;
        use windows_sys::Win32::UI::Shell::SetWindowSubclass;
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GWL_STYLE, GetWindowLongPtrW, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
            SWP_NOZORDER, SetWindowLongPtrW, SetWindowPos, WS_MAXIMIZEBOX,
        };

        let Ok(handle) = HasWindowHandle::window_handle(window) else {
            return;
        };
        let RawWindowHandle::Win32(handle) = handle.as_raw() else {
            return;
        };
        let hwnd = handle.hwnd.get() as HWND;
        unsafe {
            let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
            if style & WS_MAXIMIZEBOX != 0 {
                let _ = SetWindowLongPtrW(hwnd, GWL_STYLE, (style & !WS_MAXIMIZEBOX) as isize);
                let _ = SetWindowPos(
                    hwnd,
                    std::ptr::null_mut(),
                    0,
                    0,
                    0,
                    0,
                    SWP_FRAMECHANGED | SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER,
                );
            }
            let _ = SetWindowSubclass(
                hwnd,
                Some(main_window_subclass_proc),
                MAIN_WINDOW_SUBCLASS_ID,
                0,
            );
        }
    }
    #[cfg(target_os = "macos")]
    {
        use objc2_foundation::NSSize;

        let _ = with_ns_window(window, |native_window| {
            native_window.setContentMaxSize(NSSize::new(
                crate::models::MAX_WINDOW_WIDTH as f64,
                crate::models::MAX_WINDOW_HEIGHT as f64,
            ));
        });
    }
    #[cfg(target_os = "linux")]
    {
        use x11rb::connection::Connection as _;
        use x11rb::properties::WmSizeHints;

        let scale = window.scale_factor();
        let max_width = (crate::models::MAX_WINDOW_WIDTH * scale).round() as i32;
        let max_height = (crate::models::MAX_WINDOW_HEIGHT * scale).round() as i32;
        let _ = with_x11_window(window, |native, xid| {
            let mut hints = WmSizeHints::get_normal_hints(&native.connection, xid)
                .ok()
                .and_then(|cookie| cookie.reply().ok())
                .flatten()
                .unwrap_or_default();
            hints.max_size = Some((max_width, max_height));
            hints.set_normal_hints(&native.connection, xid).is_ok()
                && native.connection.flush().is_ok()
        });
    }
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn main_window_subclass_proc(
    hwnd: windows_sys::Win32::Foundation::HWND,
    message: u32,
    wparam: windows_sys::Win32::Foundation::WPARAM,
    lparam: windows_sys::Win32::Foundation::LPARAM,
    _subclass_id: usize,
    _reference_data: usize,
) -> windows_sys::Win32::Foundation::LRESULT {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::UI::HiDpi::GetDpiForWindow;
    use windows_sys::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetClientRect, GetWindowRect, MINMAXINFO, WM_GETMINMAXINFO, WM_NCDESTROY,
    };

    let result = unsafe { DefSubclassProc(hwnd, message, wparam, lparam) };

    if message == WM_GETMINMAXINFO && lparam != 0 {
        let dpi_scale = unsafe { GetDpiForWindow(hwnd) }.max(96) as f32 / 96.0;
        let mut window_rect = RECT::default();
        let mut client_rect = RECT::default();
        let has_window_rect = unsafe { GetWindowRect(hwnd, &mut window_rect) } != 0;
        let has_client_rect = unsafe { GetClientRect(hwnd, &mut client_rect) } != 0;
        let (frame_width, frame_height) = if has_window_rect && has_client_rect {
            let window_width = window_rect.right - window_rect.left;
            let window_height = window_rect.bottom - window_rect.top;
            let client_width = client_rect.right - client_rect.left;
            let client_height = client_rect.bottom - client_rect.top;
            (
                (window_width - client_width).max(0),
                (window_height - client_height).max(0),
            )
        } else {
            (0, 0)
        };
        let max_width = (crate::models::MAX_WINDOW_WIDTH * dpi_scale).round() as i32 + frame_width;
        let max_height =
            (crate::models::MAX_WINDOW_HEIGHT * dpi_scale).round() as i32 + frame_height;
        let minmax = unsafe { &mut *(lparam as *mut MINMAXINFO) };
        minmax.ptMaxTrackSize.x = max_width;
        minmax.ptMaxTrackSize.y = max_height;
        minmax.ptMaxSize.x = if minmax.ptMaxSize.x > 0 {
            minmax.ptMaxSize.x.min(max_width)
        } else {
            max_width
        };
        minmax.ptMaxSize.y = if minmax.ptMaxSize.y > 0 {
            minmax.ptMaxSize.y.min(max_height)
        } else {
            max_height
        };
    }

    if message == WM_NCDESTROY {
        let _ = unsafe {
            RemoveWindowSubclass(
                hwnd,
                Some(main_window_subclass_proc),
                MAIN_WINDOW_SUBCLASS_ID,
            )
        };
    }

    result
}

pub(crate) fn move_window_to(window: &Window, origin: Point<Pixels>) -> bool {
    #[cfg(target_os = "windows")]
    {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        use windows_sys::Win32::Foundation::HWND;
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER, SetWindowPos,
        };

        let Ok(handle) = HasWindowHandle::window_handle(window) else {
            return false;
        };
        let RawWindowHandle::Win32(handle) = handle.as_raw() else {
            return false;
        };
        let scale = window.scale_factor();
        let x = (f32::from(origin.x) * scale).round() as i32;
        let y = (f32::from(origin.y) * scale).round() as i32;
        unsafe {
            SetWindowPos(
                handle.hwnd.get() as HWND,
                std::ptr::null_mut(),
                x,
                y,
                0,
                0,
                SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOZORDER,
            ) != 0
        }
    }
    #[cfg(target_os = "macos")]
    {
        use objc2_foundation::NSPoint;

        with_ns_window(window, |native_window| {
            let Some(screen) = native_window.screen() else {
                return false;
            };
            let frame = screen.frame();
            native_window.setFrameTopLeftPoint(NSPoint::new(
                frame.origin.x + f32::from(origin.x) as f64,
                frame.origin.y + frame.size.height - f32::from(origin.y) as f64,
            ));
            true
        })
        .unwrap_or(false)
    }
    #[cfg(target_os = "linux")]
    {
        use x11rb::connection::Connection as _;
        use x11rb::protocol::xproto::{ConfigureWindowAux, ConnectionExt as _};

        let scale = window.scale_factor();
        let x = (f32::from(origin.x) * scale).round() as i32;
        let y = (f32::from(origin.y) * scale).round() as i32;
        with_x11_window(window, |native, xid| {
            native
                .connection
                .configure_window(xid, &ConfigureWindowAux::new().x(x).y(y))
                .is_ok()
                && native.connection.flush().is_ok()
        })
        .unwrap_or(false)
    }
}

pub(crate) fn can_position_window(window: &Window) -> bool {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let Ok(handle) = HasWindowHandle::window_handle(window) else {
        return false;
    };
    matches!(
        handle.as_raw(),
        RawWindowHandle::Win32(_) | RawWindowHandle::AppKit(_) | RawWindowHandle::Xcb(_)
    )
}

pub(crate) fn set_window_visible(window: &Window, visible: bool) -> bool {
    #[cfg(target_os = "windows")]
    {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        use windows_sys::Win32::Foundation::HWND;
        use windows_sys::Win32::UI::WindowsAndMessaging::{SW_HIDE, SW_SHOW, ShowWindow};

        let Ok(handle) = HasWindowHandle::window_handle(window) else {
            return false;
        };
        let RawWindowHandle::Win32(handle) = handle.as_raw() else {
            return false;
        };
        unsafe {
            ShowWindow(
                handle.hwnd.get() as HWND,
                if visible { SW_SHOW } else { SW_HIDE },
            );
        }
        true
    }
    #[cfg(target_os = "macos")]
    {
        with_ns_window(window, |native_window| {
            if visible {
                native_window.orderFrontRegardless();
            } else {
                native_window.orderOut(None);
            }
        })
        .is_some()
    }
    #[cfg(target_os = "linux")]
    {
        use x11rb::connection::Connection as _;
        use x11rb::protocol::xproto::ConnectionExt as _;

        with_x11_window(window, |native, xid| {
            let result = if visible {
                native.connection.map_window(xid)
            } else {
                native.connection.unmap_window(xid)
            };
            result.is_ok() && native.connection.flush().is_ok()
        })
        .unwrap_or(false)
    }
}

#[cfg(target_os = "macos")]
fn with_ns_window<T>(
    window: &Window,
    operation: impl FnOnce(&objc2_app_kit::NSWindow) -> T,
) -> Option<T> {
    use objc2_app_kit::NSView;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let handle = HasWindowHandle::window_handle(window).ok()?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return None;
    };
    let view = unsafe { handle.ns_view.cast::<NSView>().as_ref() };
    let native_window = view.window()?;
    Some(operation(&native_window))
}

#[cfg(target_os = "linux")]
struct X11Native {
    connection: x11rb::rust_connection::RustConnection,
    root: u32,
    wm_state: u32,
    wm_state_above: u32,
}

#[cfg(target_os = "linux")]
impl X11Native {
    fn connect() -> Option<Self> {
        use x11rb::connection::Connection as _;
        use x11rb::protocol::xproto::ConnectionExt as _;

        let (connection, screen_index) = x11rb::connect(None).ok()?;
        let root = connection.setup().roots.get(screen_index)?.root;
        let wm_state = connection
            .intern_atom(false, b"_NET_WM_STATE")
            .ok()?
            .reply()
            .ok()?
            .atom;
        let wm_state_above = connection
            .intern_atom(false, b"_NET_WM_STATE_ABOVE")
            .ok()?
            .reply()
            .ok()?
            .atom;
        Some(Self {
            connection,
            root,
            wm_state,
            wm_state_above,
        })
    }
}

#[cfg(target_os = "linux")]
fn with_x11_window<T>(window: &Window, operation: impl FnOnce(&X11Native, u32) -> T) -> Option<T> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use std::sync::OnceLock;

    static X11_NATIVE: OnceLock<Option<X11Native>> = OnceLock::new();

    let handle = HasWindowHandle::window_handle(window).ok()?;
    let RawWindowHandle::Xcb(handle) = handle.as_raw() else {
        return None;
    };
    let native = X11_NATIVE.get_or_init(X11Native::connect).as_ref()?;
    Some(operation(native, handle.window.get()))
}
