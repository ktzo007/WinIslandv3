//! Utilitas jendela: posisi (multi-monitor), gaya window, dan "hit-region".
//!
//! Click-through TANPA polling: jendela dibuat lebih besar dari kapsul (transparan),
//! tetapi `SetWindowRgn` membatasi area jendela hanya pada kapsul yang terlihat.
//! Piksel di luar region bukan bagian jendela → klik langsung menembus ke aplikasi
//! di bawahnya, sementara kapsul tetap menerima hover/klik.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::SeqCst};
use std::sync::Mutex;

use tauri::{Monitor, PhysicalPosition, WebviewWindow};

use crate::sys::*;

static TARGET_IDX: AtomicUsize = AtomicUsize::new(0);
/// Sisi layar untuk sidebar: true = kanan, false = kiri.
pub static SIDE_RIGHT: AtomicBool = AtomicBool::new(true);
/// Rect (px fisik) monitor tujuan — dipakai Game Mode untuk membandingkan.
pub static TARGET_RECT: Mutex<(i32, i32, i32, i32)> = Mutex::new((0, 0, 0, 0));

pub fn hwnd(w: &WebviewWindow) -> Option<HWND> {
    w.hwnd().ok().map(|h| h.0 as isize)
}

fn is_dock(w: &WebviewWindow) -> bool {
    w.label() == "dock"
}

fn is_side(w: &WebviewWindow) -> bool {
    w.label() == "side"
}

/// Dipanggil sekali per jendela overlay (island & dock).
pub fn init(w: &WebviewWindow) {
    if let Some(h) = hwnd(w) {
        unsafe {
            // NOACTIVATE: klik tidak mencuri fokus. TOOLWINDOW: tidak muncul di Alt-Tab.
            let ex = GetWindowLongPtrW(h, GWL_EXSTYLE);
            SetWindowLongPtrW(h, GWL_EXSTYLE, ex | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW);
        }
    }
    position(w);
    if is_dock(w) {
        set_shape(w, 120.0, 8.0);
    } else if is_side(w) {
        set_shape(w, 56.0, 300.0);
    } else {
        set_shape(w, 180.0, 34.0);
    }
}

pub fn set_target_monitor(w: &WebviewWindow, idx: usize) {
    TARGET_IDX.store(idx, SeqCst);
    position(w);
}

fn monitor_for(w: &WebviewWindow) -> Option<Monitor> {
    let idx = TARGET_IDX.load(SeqCst);
    w.available_monitors()
        .ok()
        .and_then(|v| v.get(idx).cloned())
        .or_else(|| w.primary_monitor().ok().flatten())
}

/// Island: tengah-atas (8 px). Dock: tengah-bawah, tepat di atas taskbar bila taskbar terlihat.
pub fn position(w: &WebviewWindow) {
    let (Some(mon), Ok(size)) = (monitor_for(w), w.outer_size()) else {
        return;
    };
    let sf = mon.scale_factor();
    let mp = *mon.position();
    let ms = *mon.size();
    *TARGET_RECT.lock().unwrap() = (mp.x, mp.y, mp.x + ms.width as i32, mp.y + ms.height as i32);

    if is_side(w) {
        let edge = (6.0 * sf) as i32;
        let x = if SIDE_RIGHT.load(SeqCst) { mp.x + ms.width as i32 - size.width as i32 - edge } else { mp.x + edge };
        let y = mp.y + (ms.height as i32 - size.height as i32) / 2;
        let _ = w.set_position(PhysicalPosition::new(x, y));
        return;
    }
    let x = mp.x + (ms.width as i32 - size.width as i32) / 2;
    let y = if is_dock(w) {
        let work_bottom = unsafe {
            let m = MonitorFromPoint(POINT { x: mp.x + 1, y: mp.y + 1 }, MONITOR_DEFAULTTONEAREST);
            let mut mi: MONITORINFO = std::mem::zeroed();
            mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
            if GetMonitorInfoW(m, &mut mi) != 0 {
                mi.rcWork.bottom
            } else {
                mp.y + ms.height as i32
            }
        };
        work_bottom - size.height as i32 - (4.0 * sf) as i32
    } else {
        mp.y + (8.0 * sf).round() as i32
    };
    let _ = w.set_position(PhysicalPosition::new(x, y));
}

/// Batasi area jendela yang "hidup" (ukuran logis, rata tengah; menempel atas/bawah sesuai jenis).
pub fn set_shape(w: &WebviewWindow, logical_w: f64, logical_h: f64) {
    let (Some(h), Ok(size)) = (hwnd(w), w.outer_size()) else {
        return;
    };
    let sf = w.scale_factor().unwrap_or(1.0);
    let pad = 6.0; // ruang untuk overshoot spring + glow border
    let pw = ((logical_w + pad * 2.0) * sf).ceil() as i32;
    let ph = ((logical_h + pad) * sf).ceil() as i32;
    let (ww, wh) = (size.width as i32, size.height as i32);
    if is_side(w) {
        // Sidebar: menempel ke tepi layar (kanan/kiri), terpusat vertikal.
        let pw = ((logical_w + pad) * sf).ceil() as i32;
        let ph = ((logical_h + pad * 2.0) * sf).ceil() as i32;
        let (l, r) = if SIDE_RIGHT.load(SeqCst) { ((ww - pw).max(0), ww) } else { (0, pw.min(ww)) };
        let t = ((wh - ph) / 2).max(0);
        unsafe {
            let rgn = CreateRectRgn(l, t, r, (t + ph).min(wh));
            SetWindowRgn(h, rgn, 1);
        }
        return;
    }
    let left = ((ww - pw) / 2).max(0);
    let right = (left + pw).min(ww);
    let (top, bottom) = if is_dock(w) { ((wh - ph).max(0), wh) } else { (0, ph.min(wh)) };
    unsafe {
        // Kepemilikan region berpindah ke sistem → jangan DeleteObject.
        let rgn = CreateRectRgn(left, top, right, bottom);
        SetWindowRgn(h, rgn, 1);
    }
}

/// Tampilkan tanpa aktivasi + pastikan tetap topmost.
pub fn show_noactivate(w: &WebviewWindow) {
    if let Some(h) = hwnd(w) {
        unsafe {
            ShowWindow(h, SW_SHOWNOACTIVATE);
            SetWindowPos(h, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
        }
    }
}

/// `ShowWindow(hwnd, SW_HIDE)` — dipakai Game Mode.
pub fn hide(w: &WebviewWindow) {
    if let Some(h) = hwnd(w) {
        unsafe {
            ShowWindow(h, SW_HIDE);
        }
    }
}

/// Izinkan fokus keyboard sementara (mis. mengetik acara baru di kalender), lalu kembalikan NOACTIVATE.
pub fn set_focusable(w: &WebviewWindow, on: bool) {
    if let Some(h) = hwnd(w) {
        unsafe {
            let ex = GetWindowLongPtrW(h, GWL_EXSTYLE);
            if on {
                SetWindowLongPtrW(h, GWL_EXSTYLE, ex & !WS_EX_NOACTIVATE);
                SetForegroundWindow(h);
            } else {
                SetWindowLongPtrW(h, GWL_EXSTYLE, ex | WS_EX_NOACTIVATE);
            }
        }
        if on {
            let _ = w.set_focus();
        }
    }
}
