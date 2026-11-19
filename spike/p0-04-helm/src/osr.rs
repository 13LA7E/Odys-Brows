//! P0-04b: the Helm rendered off-screen (Alloy, transparent background) into a per-pixel-alpha
//! layered window owned by the main window. Windows only. We forward mouse, keyboard and IME.
//! Transparent pixels of a layered window are click-through, so the page around the pill and
//! under the shadow stays clickable.

use cef::*;
use std::cell::RefCell;
use std::ffi::c_void;
use std::time::Instant;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, SIZE, WPARAM};
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::HiDpi::GetDpiForWindow;
use windows_sys::Win32::UI::Input::Ime::*;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

/// Room around the 560x52 pill for the drawn shadow (0 12px 32px), in DIP.
pub const MARGIN: i32 = 48;
/// WM_MOUSELEAVE (winuser.h); not in the windows-sys modules we enable.
const WM_MOUSELEAVE_: u32 = 0x02A3;
pub const W: i32 = crate::HELM_W + 2 * MARGIN;
pub const H: i32 = crate::HELM_H + 2 * MARGIN;

#[derive(Default)]
struct Osr {
    hwnd: usize,
    owner: usize,
    browser: Option<Browser>,
    scale: f32,
    tracking: bool,
    /// Time of the last input we forwarded, to measure input-to-paint.
    pending_input: Option<Instant>,
}

thread_local! { static OSR: RefCell<Osr> = RefCell::new(Osr { scale: 1.0, ..Default::default() }); }

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn host() -> Option<BrowserHost> {
    OSR.with(|o| o.borrow().browser.as_ref().and_then(|b| b.host()))
}

fn scale() -> f32 {
    OSR.with(|o| o.borrow().scale)
}

fn modifiers() -> u32 {
    let mut m = 0;
    unsafe {
        if GetKeyState(VK_SHIFT as i32) < 0 {
            m |= 1 << 1; // EVENTFLAG_SHIFT_DOWN
        }
        if GetKeyState(VK_CONTROL as i32) < 0 {
            m |= 1 << 2; // EVENTFLAG_CONTROL_DOWN
        }
        if GetKeyState(VK_MENU as i32) < 0 {
            m |= 1 << 3; // EVENTFLAG_ALT_DOWN
        }
        if GetKeyState(VK_LBUTTON as i32) < 0 {
            m |= 1 << 4; // EVENTFLAG_LEFT_MOUSE_BUTTON
        }
    }
    m
}

fn mouse_at(lp: LPARAM) -> MouseEvent {
    let x = (lp & 0xffff) as i16 as f32;
    let y = ((lp >> 16) & 0xffff) as i16 as f32;
    let s = scale();
    MouseEvent {
        x: (x / s).round() as i32,
        y: (y / s).round() as i32,
        modifiers: modifiers(),
    }
}

fn mark_input() {
    OSR.with(|o| o.borrow_mut().pending_input = Some(Instant::now()));
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let Some(h) = host() else {
        return unsafe { DefWindowProcW(hwnd, msg, wp, lp) };
    };
    match msg {
        // The shadow has alpha > 0, which would swallow clicks. Outside the pill, let the click go
        // to the window below (HTTRANSPARENT works because the main window is on this thread).
        WM_NCHITTEST => {
            let mut p = POINT {
                x: (lp & 0xffff) as i16 as i32,
                y: ((lp >> 16) & 0xffff) as i16 as i32,
            };
            unsafe { ScreenToClient(hwnd, &mut p) };
            let s = scale();
            let (x, y) = (p.x as f32 / s, p.y as f32 / s);
            let r = crate::HELM_H as f32 / 2.0;
            let (l, t) = (MARGIN as f32, MARGIN as f32);
            let (rr, b) = (l + crate::HELM_W as f32, t + crate::HELM_H as f32);
            // Inside the pill: the central rectangle or one of the two end circles.
            let cy = t + r;
            let inside = (y >= t && y < b && x >= l + r && x < rr - r)
                || (x - (l + r)).hypot(y - cy) <= r
                || (x - (rr - r)).hypot(y - cy) <= r;
            if inside {
                HTCLIENT as isize
            } else {
                HTTRANSPARENT as isize
            }
        }
        WM_MOUSEMOVE => {
            let start = OSR.with(|o| !std::mem::replace(&mut o.borrow_mut().tracking, true));
            if start {
                let mut t = TRACKMOUSEEVENT {
                    cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                    dwFlags: TME_LEAVE,
                    hwndTrack: hwnd,
                    dwHoverTime: 0,
                };
                unsafe { TrackMouseEvent(&mut t) };
            }
            h.send_mouse_move_event(Some(&mouse_at(lp)), 0);
            0
        }
        WM_MOUSELEAVE_ => {
            OSR.with(|o| o.borrow_mut().tracking = false);
            h.send_mouse_move_event(Some(&MouseEvent::default()), 1);
            0
        }
        WM_LBUTTONDOWN | WM_LBUTTONUP | WM_RBUTTONDOWN | WM_RBUTTONUP => {
            let up = (msg == WM_LBUTTONUP || msg == WM_RBUTTONUP) as i32;
            let button = if msg == WM_RBUTTONDOWN || msg == WM_RBUTTONUP {
                MouseButtonType::RIGHT
            } else {
                MouseButtonType::LEFT
            };
            if up == 0 {
                mark_input();
                unsafe { SetFocus(hwnd) };
            }
            h.send_mouse_click_event(Some(&mouse_at(lp)), button, up, 1);
            0
        }
        WM_MOUSEWHEEL => {
            let mut p = POINT {
                x: (lp & 0xffff) as i16 as i32,
                y: ((lp >> 16) & 0xffff) as i16 as i32,
            };
            unsafe { ScreenToClient(hwnd, &mut p) };
            let s = scale();
            let ev = MouseEvent {
                x: (p.x as f32 / s) as i32,
                y: (p.y as f32 / s) as i32,
                modifiers: modifiers(),
            };
            h.send_mouse_wheel_event(Some(&ev), 0, ((wp >> 16) & 0xffff) as i16 as i32);
            0
        }
        WM_KEYDOWN | WM_KEYUP | WM_CHAR | WM_SYSKEYDOWN | WM_SYSKEYUP | WM_SYSCHAR => {
            let type_ = match msg {
                WM_KEYDOWN | WM_SYSKEYDOWN => KeyEventType::RAWKEYDOWN,
                WM_KEYUP | WM_SYSKEYUP => KeyEventType::KEYUP,
                _ => KeyEventType::CHAR,
            };
            if msg == WM_KEYDOWN {
                mark_input();
            }
            let ev = KeyEvent {
                size: std::mem::size_of::<KeyEvent>(),
                type_,
                modifiers: modifiers(),
                windows_key_code: wp as i32,
                native_key_code: lp as i32,
                is_system_key: matches!(msg, WM_SYSKEYDOWN | WM_SYSKEYUP | WM_SYSCHAR) as i32,
                character: wp as u16,
                unmodified_character: wp as u16,
                focus_on_editable_field: 0,
            };
            h.send_key_event(Some(&ev));
            0
        }
        WM_SETFOCUS => {
            h.set_focus(1);
            0
        }
        WM_KILLFOCUS => {
            h.set_focus(0);
            0
        }
        // IME: draw the composition inside our page instead of the system window.
        WM_IME_STARTCOMPOSITION => 0,
        WM_IME_COMPOSITION => {
            let himc = unsafe { ImmGetContext(hwnd) };
            let read = |kind: u32| -> Option<String> {
                let bytes =
                    unsafe { ImmGetCompositionStringW(himc, kind, std::ptr::null_mut(), 0) };
                if bytes <= 0 {
                    return None;
                }
                let mut buf = vec![0u16; bytes as usize / 2];
                unsafe {
                    ImmGetCompositionStringW(
                        himc,
                        kind,
                        buf.as_mut_ptr() as *mut c_void,
                        bytes as u32,
                    )
                };
                Some(String::from_utf16_lossy(&buf))
            };
            if lp as u32 & GCS_RESULTSTR != 0 {
                if let Some(t) = read(GCS_RESULTSTR) {
                    h.ime_commit_text(Some(&CefString::from(t.as_str())), None, 0);
                }
            } else if lp as u32 & GCS_COMPSTR != 0
                && let Some(t) = read(GCS_COMPSTR)
            {
                let n = t.encode_utf16().count() as u32;
                let sel = Range { from: n, to: n };
                h.ime_set_composition(Some(&CefString::from(t.as_str())), None, None, Some(&sel));
            }
            unsafe { ImmReleaseContext(hwnd, himc) };
            0
        }
        WM_IME_ENDCOMPOSITION => {
            h.ime_finish_composing_text(0);
            unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wp, lp) },
    }
}

/// Create the layered window (owned by `owner`) and the off-screen Helm browser in it.
pub fn create(owner: *mut c_void, url: &str, client: &mut Client) {
    unsafe {
        let class = wide("ConstellationsSpikeHelm");
        let inst = GetModuleHandleW(std::ptr::null());
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: 0,
            lpfnWndProc: Some(wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: inst,
            hIcon: std::ptr::null_mut(),
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: class.as_ptr(),
            hIconSm: std::ptr::null_mut(),
        };
        RegisterClassExW(&wc);
        let hwnd = CreateWindowExW(
            WS_EX_LAYERED,
            class.as_ptr(),
            wide("Helm").as_ptr(),
            WS_POPUP,
            0,
            0,
            1,
            1,
            owner as HWND,
            std::ptr::null_mut(),
            inst,
            std::ptr::null(),
        );
        let s = GetDpiForWindow(owner as HWND).max(96) as f32 / 96.0;
        OSR.with(|o| {
            let mut o = o.borrow_mut();
            o.hwnd = hwnd as usize;
            o.owner = owner as usize;
            o.scale = s;
        });
        let info = WindowInfo::default().set_as_windowless(sys::HWND(hwnd as *mut _));
        let settings = BrowserSettings {
            background_color: 0x00000000, // transparent painting (windowless only)
            // `HELM_FPS` overrides the off-screen frame rate for measurements (default 60).
            windowless_frame_rate: std::env::var("HELM_FPS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(60),
            ..Default::default()
        };
        browser_host_create_browser(
            Some(&info),
            Some(client),
            Some(&CefString::from(url)),
            Some(&settings),
            None,
            None,
        );
    }
}

pub fn set_browser(b: Option<Browser>) {
    OSR.with(|o| o.borrow_mut().browser = b);
}

pub fn exists() -> bool {
    OSR.with(|o| o.borrow().hwnd != 0)
}

/// Place the layered window so the pill sits at (hx, hy) DIP in the owner's client area.
pub fn place(hx: i32, hy: i32) {
    let (hwnd, owner) = OSR.with(|o| (o.borrow().hwnd, o.borrow().owner));
    if hwnd == 0 {
        return;
    }
    unsafe {
        // While the owner is minimised Windows hides owned windows; don't show it again.
        if IsIconic(owner as HWND) != 0 {
            return;
        }
        let s = GetDpiForWindow(owner as HWND).max(96) as f32 / 96.0;
        let changed = OSR.with(|o| {
            let mut o = o.borrow_mut();
            let c = (o.scale - s).abs() > 0.001;
            o.scale = s;
            c
        });
        if changed && let Some(h) = host() {
            h.notify_screen_info_changed();
            h.was_resized();
        }
        let mut p = POINT { x: 0, y: 0 };
        ClientToScreen(owner as HWND, &mut p);
        let x = p.x + ((hx - MARGIN) as f32 * s).round() as i32;
        let y = p.y + ((hy - MARGIN) as f32 * s).round() as i32;
        let w = (W as f32 * s).round() as i32;
        let h = (H as f32 * s).round() as i32;
        SetWindowPos(
            hwnd as HWND,
            std::ptr::null_mut(),
            x,
            y,
            w,
            h,
            SWP_NOZORDER | SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
    }
}

pub fn focus_owner() {
    let owner = OSR.with(|o| o.borrow().owner);
    if owner != 0 {
        unsafe { SetForegroundWindow(owner as HWND) };
    }
}

pub fn destroy() {
    let hwnd = OSR.with(|o| std::mem::take(&mut o.borrow_mut().hwnd));
    if let Some(h) = host() {
        h.close_browser(1);
    }
    if hwnd != 0 {
        unsafe { DestroyWindow(hwnd as HWND) };
    }
}

/// Copy a CEF paint buffer (BGRA, premultiplied, top-down) into the layered window.
fn paint(buffer: *const u8, width: i32, height: i32) {
    let hwnd = OSR.with(|o| o.borrow().hwnd);
    if hwnd == 0 || buffer.is_null() || width <= 0 || height <= 0 {
        return;
    }
    unsafe {
        let screen = GetDC(std::ptr::null_mut());
        let mem = CreateCompatibleDC(screen);
        let mut bmi: BITMAPINFO = std::mem::zeroed();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = width;
        bmi.bmiHeader.biHeight = -height; // top-down
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB;
        let mut bits: *mut c_void = std::ptr::null_mut();
        let dib = CreateDIBSection(
            mem,
            &bmi,
            DIB_RGB_COLORS,
            &mut bits,
            std::ptr::null_mut(),
            0,
        );
        if !dib.is_null() && !bits.is_null() {
            std::ptr::copy_nonoverlapping(buffer, bits as *mut u8, (width * height * 4) as usize);
            let old = SelectObject(mem, dib);
            let size = SIZE {
                cx: width,
                cy: height,
            };
            let src = POINT { x: 0, y: 0 };
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };
            UpdateLayeredWindow(
                hwnd as HWND,
                screen,
                std::ptr::null(),
                &size,
                mem,
                &src,
                0,
                &blend,
                ULW_ALPHA,
            );
            SelectObject(mem, old);
            DeleteObject(dib);
        }
        DeleteDC(mem);
        ReleaseDC(std::ptr::null_mut(), screen);
    }
    // Spike evidence: keep the last frame so a screenshot can composite it with real alpha
    // (PrintWindow can't read a layered window's per-pixel alpha).
    if let Some(path) = std::env::var_os("HELM_DUMP") {
        let mut data = Vec::with_capacity(8 + (width * height * 4) as usize);
        data.extend_from_slice(&width.to_le_bytes());
        data.extend_from_slice(&height.to_le_bytes());
        data.extend_from_slice(unsafe {
            std::slice::from_raw_parts(buffer, (width * height * 4) as usize)
        });
        let _ = std::fs::write(path, data);
    }
    if let Some(t) = OSR.with(|o| o.borrow_mut().pending_input.take()) {
        println!("METRIC osrpaint {:.2}", t.elapsed().as_secs_f64() * 1000.0);
    }
}

wrap_render_handler! {
    pub struct HelmRender {}
    impl RenderHandler {
        fn view_rect(&self, _browser: Option<&mut Browser>, rect: Option<&mut Rect>) {
            if let Some(r) = rect {
                *r = Rect { x: 0, y: 0, width: W, height: H };
            }
        }
        fn screen_info(&self, _browser: Option<&mut Browser>, info: Option<&mut ScreenInfo>) -> i32 {
            let Some(i) = info else { return 0 };
            i.device_scale_factor = scale();
            1
        }
        fn on_paint(&self, _browser: Option<&mut Browser>, type_: PaintElementType, _dirty: Option<&[Rect]>, buffer: *const u8, width: i32, height: i32) {
            if type_ == PaintElementType::VIEW {
                paint(buffer, width, height);
            }
        }
    }
}

/// Apply the cursor CEF asks for (off-screen browsers don't set it themselves).
pub fn set_cursor(cursor: *mut c_void) {
    unsafe { SetCursor(cursor as HCURSOR) };
}
