//! PROTOTYPE: click-through overlay windows and monitor discovery.

use anyhow::Result;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, LWA_ALPHA, RegisterClassExW, SW_SHOWNOACTIVATE,
    SetLayeredWindowAttributes, ShowWindow, WINDOW_EX_STYLE, WNDCLASSEXW, WS_EX_LAYERED,
    WS_EX_NOACTIVATE, WS_EX_NOREDIRECTIONBITMAP, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_EX_TRANSPARENT, WS_POPUP,
};
use windows::core::{BOOL, w};

extern "system" fn wndproc(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    // SAFETY: forwarding the window's own message.
    unsafe { DefWindowProcW(h, m, w, l) }
}

/// A topmost, click-through, never-activated window with no redirection
/// surface, so only what is composed into it (DirectComposition visuals or a
/// WebView2 child) shows.
pub fn overlay_window(x: i32, y: i32, w: i32, h: i32) -> Result<HWND> {
    // SAFETY: standard window registration and creation on this thread.
    unsafe {
        let instance = GetModuleHandleW(None)?;
        let class = w!("reo-spike-overlay");
        let wc = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(wndproc),
            hInstance: instance.into(),
            lpszClassName: class,
            ..Default::default()
        };
        RegisterClassExW(&wc); // fails harmlessly after the first window
        let ex: WINDOW_EX_STYLE = WS_EX_NOREDIRECTIONBITMAP
            | WS_EX_TOPMOST
            | WS_EX_TOOLWINDOW
            | WS_EX_NOACTIVATE
            | WS_EX_LAYERED
            | WS_EX_TRANSPARENT;
        let hwnd = CreateWindowExW(
            ex,
            class,
            w!("reo spike"),
            WS_POPUP,
            x,
            y,
            w,
            h,
            None,
            None,
            Some(instance.into()),
            None,
        )?;
        SetLayeredWindowAttributes(hwnd, COLORREF(0), 255, LWA_ALPHA)?;
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        Ok(hwnd)
    }
}

pub fn monitors() -> Vec<RECT> {
    extern "system" fn each(m: HMONITOR, _: HDC, _: *mut RECT, data: LPARAM) -> BOOL {
        // SAFETY: `data` is the Vec passed below, alive for the enumeration.
        unsafe {
            let out = &mut *(data.0 as *mut Vec<RECT>);
            let mut info = MONITORINFO {
                cbSize: size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            if GetMonitorInfoW(m, &mut info).as_bool() {
                out.push(info.rcMonitor);
            }
        }
        true.into()
    }
    let mut out: Vec<RECT> = Vec::new();
    // SAFETY: the callback only touches `out`.
    unsafe {
        let _ = EnumDisplayMonitors(None, None, Some(each), LPARAM(&raw mut out as isize));
    }
    out.sort_by_key(|r| (r.left, r.top));
    out
}
