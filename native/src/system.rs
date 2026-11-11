use base64::Engine;

pub fn autostart() -> Result<auto_launch::AutoLaunch, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    auto_launch::AutoLaunchBuilder::new()
        .set_app_name("Q Note")
        .set_app_path(&exe.to_string_lossy())
        .set_use_launch_agent(true)
        .build()
        .map_err(|e| e.to_string())
}

pub fn image(path: &str) -> Result<String, String> {
    let path = if let Some(path) = path.strip_prefix("file:///") {
        #[cfg(target_os = "windows")]
        {
            path.to_string()
        }
        #[cfg(not(target_os = "windows"))]
        {
            format!("/{path}")
        }
    } else {
        path.to_string()
    };
    let extension = std::path::Path::new(&path)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    let mime = match extension.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "svg" => "image/svg+xml",
        _ => return Err("Unsupported image type".into()),
    };
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    Ok(format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

#[cfg(windows)]
pub fn clip_dock(edge: &str, hidden: f64) {
    use windows_sys::Win32::{
        Foundation::{HWND, LPARAM, RECT},
        Graphics::Gdi::{CreateRectRgn, DeleteObject, SetWindowRgn},
        UI::{
            HiDpi::GetDpiForWindow,
            WindowsAndMessaging::{
                EnumWindows, GetWindowRect, GetWindowTextW, GetWindowThreadProcessId,
            },
        },
    };
    unsafe extern "system" fn find(hwnd: HWND, data: LPARAM) -> i32 {
        let mut process = 0;
        unsafe {
            GetWindowThreadProcessId(hwnd, &mut process);
        }
        if process != std::process::id() {
            return 1;
        }
        let mut title = [0u16; 128];
        let length = unsafe { GetWindowTextW(hwnd, title.as_mut_ptr(), title.len() as i32) };
        if String::from_utf16_lossy(&title[..length.max(0) as usize]) == "Q Note - Dock" {
            unsafe {
                *(data as *mut HWND) = hwnd;
            }
            return 0;
        }
        1
    }
    let mut hwnd: HWND = std::ptr::null_mut();
    // The callback filters both title and process; other applications are never touched.
    unsafe {
        EnumWindows(Some(find), &mut hwnd as *mut HWND as LPARAM);
        if hwnd.is_null() {
            return;
        }
        if hidden <= 0.0 {
            SetWindowRgn(hwnd, std::ptr::null_mut(), 1);
            return;
        }
        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect) == 0 {
            return;
        }
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        let amount =
            (hidden.clamp(0.0, 15.0) * GetDpiForWindow(hwnd).max(96) as f64 / 96.0).round() as i32;
        let region = CreateRectRgn(
            if edge == "left" { amount } else { 0 },
            if edge == "top" { amount } else { 0 },
            width - if edge == "right" { amount } else { 0 },
            height - if edge == "bottom" { amount } else { 0 },
        );
        if !region.is_null() && SetWindowRgn(hwnd, region, 1) == 0 {
            DeleteObject(region);
        }
    }
}
