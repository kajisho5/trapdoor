//! Windows output window: a borderless, top-most black popup covering one display,
//! holding one child window per deck. mpv renders into the children (`wid`); a cut is
//! just showing one child and hiding the other, so it is instant.
//!
//! The output raster (e.g. 1920×1080, 3840×2160, 8K or a custom LED size) is placed
//! pixel-for-pixel at the top-left of the display. If it is larger than the display it is
//! scaled down to fit, keeping its aspect ratio.

use std::sync::mpsc;
use std::thread;

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{EnumDisplayMonitors, GetMonitorInfoW, GetStockObject, BLACK_BRUSH, HBRUSH, HDC, HMONITOR, MONITORINFO};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::*;

#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct DisplayInfo {
    pub index: u32,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub primary: bool,
}

unsafe extern "system" fn enum_proc(m: HMONITOR, _: HDC, _: *mut RECT, data: LPARAM) -> windows::core::BOOL {
    let v = &mut *(data.0 as *mut Vec<DisplayInfo>);
    let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
    if GetMonitorInfoW(m, &mut mi).as_bool() {
        let r = mi.rcMonitor;
        v.push(DisplayInfo {
            index: v.len() as u32 + 1,
            x: r.left,
            y: r.top,
            width: r.right - r.left,
            height: r.bottom - r.top,
            primary: mi.dwFlags & 1 == 1,
        });
    }
    true.into()
}

/// Displays in OS enumeration order, 1-based.
pub fn displays() -> Vec<DisplayInfo> {
    let mut v: Vec<DisplayInfo> = vec![];
    unsafe {
        let _ = EnumDisplayMonitors(None, None, Some(enum_proc), LPARAM(&mut v as *mut _ as isize));
    }
    v
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_CLOSE => LRESULT(0), // the output window is closed only by the app
        WM_SETCURSOR => {
            SetCursor(None);
            LRESULT(1)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

/// Handles are passed around as isize (HWND is not Send).
pub struct OutputWindow {
    pub main: isize,
    pub children: [isize; 2],
}

fn fit_raster(raster: (u32, u32), area: (i32, i32)) -> (i32, i32) {
    let (rw, rh) = (raster.0.max(1) as f64, raster.1.max(1) as f64);
    let (aw, ah) = (area.0.max(1) as f64, area.1.max(1) as f64);
    if rw <= aw && rh <= ah {
        return (rw as i32, rh as i32);
    }
    let k = (aw / rw).min(ah / rh);
    ((rw * k).floor() as i32, (rh * k).floor() as i32)
}

impl OutputWindow {
    /// Create the window on its own UI thread and return the handles.
    /// `display`: 1-based; 0 = a normal resizable preview window.
    pub fn spawn(display: u32, raster: (u32, u32)) -> Result<OutputWindow, String> {
        let (tx, rx) = mpsc::channel::<Result<(isize, [isize; 2]), String>>();
        thread::Builder::new()
            .name("trapdoor-output".into())
            .spawn(move || unsafe {
                let inst = match GetModuleHandleW(None) {
                    Ok(h) => h,
                    Err(e) => {
                        let _ = tx.send(Err(e.to_string()));
                        return;
                    }
                };
                let class = w!("TrapdoorOutput");
                let wc = WNDCLASSW {
                    lpfnWndProc: Some(wndproc),
                    hInstance: inst.into(),
                    lpszClassName: class,
                    hbrBackground: HBRUSH(GetStockObject(BLACK_BRUSH).0),
                    ..Default::default()
                };
                RegisterClassW(&wc);
                let ds = displays();
                let (style, ex, x, y, w, h) = match ds.iter().find(|d| d.index == display) {
                    Some(d) if display > 0 => (WS_POPUP | WS_VISIBLE, WS_EX_TOPMOST | WS_EX_TOOLWINDOW, d.x, d.y, d.width, d.height),
                    _ => (WS_OVERLAPPEDWINDOW | WS_VISIBLE, WINDOW_EX_STYLE(0), 80, 80, 960, 540),
                };
                let main = match CreateWindowExW(ex, class, w!("TRAPDOOR OUTPUT"), style, x, y, w, h, None, None, Some(inst.into()), None) {
                    Ok(h) => h,
                    Err(e) => {
                        let _ = tx.send(Err(e.to_string()));
                        return;
                    }
                };
                let (cw, ch) = if display > 0 { fit_raster(raster, (w, h)) } else { (w, h) };
                let mut kids = [0isize; 2];
                for k in kids.iter_mut() {
                    match CreateWindowExW(WINDOW_EX_STYLE(0), class, PCWSTR::null(), WS_CHILD | WS_CLIPSIBLINGS, 0, 0, cw, ch, Some(main), None, Some(inst.into()), None) {
                        Ok(c) => *k = c.0 as isize,
                        Err(e) => {
                            let _ = tx.send(Err(e.to_string()));
                            return;
                        }
                    }
                }
                let _ = tx.send(Ok((main.0 as isize, kids)));
                let mut msg = MSG::default();
                while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            })
            .map_err(|e| e.to_string())?;
        let (main, children) = rx.recv().map_err(|e| e.to_string())??;
        Ok(OutputWindow { main, children })
    }

    pub fn set_child_visible(child: isize, visible: bool) {
        unsafe {
            let h = HWND(child as *mut _);
            let _ = ShowWindowAsync(h, if visible { SW_SHOWNA } else { SW_HIDE });
            if visible {
                let _ = SetWindowPos(h, Some(HWND_TOP), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_ASYNCWINDOWPOS);
            }
        }
    }

    pub fn close(&self) {
        unsafe {
            let _ = PostMessageW(Some(HWND(self.main as *mut _)), WM_QUIT, WPARAM(0), LPARAM(0));
            let _ = DestroyWindow(HWND(self.main as *mut _));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fit_raster;

    #[test]
    fn raster_fit() {
        assert_eq!(fit_raster((1920, 1080), (3840, 2160)), (1920, 1080));
        assert_eq!(fit_raster((7680, 4320), (3840, 2160)), (3840, 2160));
        assert_eq!(fit_raster((8192, 4320), (3840, 2160)), (3840, 2025));
    }
}
