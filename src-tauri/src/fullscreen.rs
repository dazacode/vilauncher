//! Borderless fullscreen for a game's window: the projects open a normal
//! window with a title bar; once it shows, its frame goes and it covers its
//! whole monitor. Done again for a minute in case the game remakes it.

#[cfg(windows)]
pub fn keep_borderless(pid: u32) {
    std::thread::spawn(move || {
        let started = std::time::Instant::now();
        let mut done_once = false;
        while started.elapsed() < std::time::Duration::from_secs(if done_once { 60 } else { 600 }) {
            if let Some(hwnd) = find_window(pid) {
                if !is_borderless(hwnd) {
                    make_borderless(hwnd);
                    done_once = true;
                }
            } else if done_once {
                // The window went: the game ended.
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(if done_once {
                2000
            } else {
                400
            }));
        }
    });
}

#[cfg(not(windows))]
pub fn keep_borderless(_pid: u32) {}

#[cfg(windows)]
use windows_sys::Win32::{Foundation::*, Graphics::Gdi::*, UI::WindowsAndMessaging::*};

/// The biggest visible top-level window of process `pid`.
#[cfg(windows)]
fn find_window(pid: u32) -> Option<HWND> {
    struct Search {
        pid: u32,
        best: HWND,
        area: i64,
    }
    unsafe extern "system" fn each(hwnd: HWND, data: LPARAM) -> BOOL {
        // SAFETY: `data` is the `Search` below, alive during the enumeration.
        let s = unsafe { &mut *(data as *mut Search) };
        let mut owner = 0u32;
        // SAFETY: a window handle from the enumeration.
        unsafe { GetWindowThreadProcessId(hwnd, &mut owner) };
        if owner == s.pid && unsafe { IsWindowVisible(hwnd) } != 0 {
            let mut r = RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            // SAFETY: as above, and an out pointer.
            unsafe { GetWindowRect(hwnd, &mut r) };
            let area = i64::from(r.right - r.left) * i64::from(r.bottom - r.top);
            if area > s.area {
                s.area = area;
                s.best = hwnd;
            }
        }
        1
    }
    let mut s = Search {
        pid,
        best: std::ptr::null_mut(),
        area: 0,
    };
    // SAFETY: the callback reads `s` only during this call.
    unsafe { EnumWindows(Some(each), &mut s as *mut Search as LPARAM) };
    (!s.best.is_null() && s.area > 200 * 200).then_some(s.best)
}

#[cfg(windows)]
fn is_borderless(hwnd: HWND) -> bool {
    // SAFETY: a live window handle.
    let style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) } as u32;
    style & WS_CAPTION == 0
}

#[cfg(windows)]
fn make_borderless(hwnd: HWND) {
    // SAFETY: a live window of the game's; only its frame and place change.
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
        let frame = WS_CAPTION | WS_THICKFRAME | WS_MINIMIZEBOX | WS_MAXIMIZEBOX | WS_SYSMENU;
        SetWindowLongPtrW(
            hwnd,
            GWL_STYLE,
            ((style & !frame) | WS_POPUP | WS_VISIBLE) as isize,
        );
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        let edges = WS_EX_DLGMODALFRAME | WS_EX_CLIENTEDGE | WS_EX_STATICEDGE | WS_EX_WINDOWEDGE;
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, (ex & !edges) as isize);
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(monitor, &mut info) != 0 {
            let r = info.rcMonitor;
            SetWindowPos(
                hwnd,
                HWND_TOP,
                r.left,
                r.top,
                r.right - r.left,
                r.bottom - r.top,
                SWP_FRAMECHANGED | SWP_SHOWWINDOW,
            );
        }
        SetForegroundWindow(hwnd);
    }
}
