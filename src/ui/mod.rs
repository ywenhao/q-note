pub mod dock_window;
pub mod editor_window;
pub mod main_window;
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
    #[cfg(target_os = "windows")]
    {
        let _ = always_on_top;
        WindowKind::Normal
    }
    #[cfg(not(target_os = "windows"))]
    {
        if always_on_top {
            WindowKind::PopUp
        } else {
            WindowKind::Normal
        }
    }
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
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (window, enabled);
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
    #[cfg(not(target_os = "windows"))]
    {
        let _ = window;
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
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (window, origin);
        false
    }
}
