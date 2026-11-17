//! Game Mode: sembunyikan island & dock saat ada aplikasi/game fullscreen di monitor tujuan.
//!
//! 100% event-driven (tanpa timer/polling):
//!   • EVENT_SYSTEM_FOREGROUND     → jendela depan berganti.
//!   • EVENT_OBJECT_LOCATIONCHANGE → hanya untuk PROSES jendela depan (hook di-retarget
//!     tiap foreground berubah): menangkap game yang pindah ke fullscreen/borderless
//!     atau F11 di browser/pemutar video tanpa pergantian foreground.
//! Saat keluar dari fullscreen, tampil kembali ditunda ±450 ms (anti-strobe untuk aplikasi
//! yang overlay-nya sempat mencuri fokus).

use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, Ordering::SeqCst};
use std::sync::OnceLock;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};

use crate::sys::*;
use crate::window;

static APP: OnceLock<AppHandle> = OnceLock::new();
static GAME_MODE: AtomicBool = AtomicBool::new(false);
static HIDE_ENABLED: AtomicBool = AtomicBool::new(true);
static MANUAL_HIDDEN: AtomicBool = AtomicBool::new(false);
static CURRENT_HIDDEN: AtomicBool = AtomicBool::new(false);
static PENDING: AtomicBool = AtomicBool::new(false);
static LOC_HOOK: AtomicIsize = AtomicIsize::new(0);
static THREAD_ID: AtomicU32 = AtomicU32::new(0);
static GAME_PID: AtomicU32 = AtomicU32::new(0);

const IGNORED_CLASSES: &[&str] = &[
    "Progman",
    "WorkerW",
    "Shell_TrayWnd",
    "Shell_SecondaryTrayWnd",
    "MultitaskingViewFrame",
    "XamlExplorerHostIslandWindow",
    "ForegroundStaging",
    "TaskSwitcherWnd",
    "Windows.UI.Core.CoreWindow",
];

pub fn start(app: AppHandle) {
    let _ = APP.set(app);
    let _ = std::thread::Builder::new()
        .name("fullscreen-watch".into())
        .spawn(|| unsafe {
            THREAD_ID.store(GetCurrentThreadId(), SeqCst);
            let fg = SetWinEventHook(
                EVENT_SYSTEM_FOREGROUND,
                EVENT_SYSTEM_FOREGROUND,
                0,
                Some(on_event),
                0,
                0,
                WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
            );
            retarget();
            evaluate();

            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, 0, 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            UnhookWinEvent(fg);
            let loc = LOC_HOOK.swap(0, SeqCst);
            if loc != 0 {
                UnhookWinEvent(loc);
            }
        });
}

pub fn stop() {
    let tid = THREAD_ID.load(SeqCst);
    if tid != 0 {
        unsafe {
            PostThreadMessageW(tid, WM_QUIT, 0, 0);
        }
    }
}

#[tauri::command]
pub fn set_hide_in_fullscreen(on: bool) {
    HIDE_ENABLED.store(on, SeqCst);
    refresh();
}

#[tauri::command]
pub fn set_manual_hidden(hidden: bool) {
    MANUAL_HIDDEN.store(hidden, SeqCst);
    refresh();
}

pub fn toggle_manual() {
    let now = !MANUAL_HIDDEN.load(SeqCst);
    MANUAL_HIDDEN.store(now, SeqCst);
    refresh();
}

unsafe extern "system" fn on_event(
    _hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    id_object: i32,
    _id_child: i32,
    _thread: u32,
    _time: u32,
) {
    match event {
        EVENT_SYSTEM_FOREGROUND => {
            retarget();
            evaluate();
            // Dock menyegarkan indikator "sedang berjalan" hanya saat foreground berganti.
            if let Some(app) = APP.get() {
                if crate::VISIBLE.load(SeqCst) && app.get_webview_window("dock").is_some() {
                    let _ = app.emit("foreground-changed", ());
                }
            }
        }
        EVENT_OBJECT_LOCATIONCHANGE => {
            if id_object == OBJID_WINDOW && hwnd == GetForegroundWindow() {
                evaluate();
            }
        }
        _ => {}
    }
}

/// Pasang ulang hook LOCATIONCHANGE khusus untuk proses jendela depan.
unsafe fn retarget() {
    let old = LOC_HOOK.swap(0, SeqCst);
    if old != 0 {
        UnhookWinEvent(old);
    }
    let fg = GetForegroundWindow();
    if fg == 0 {
        return;
    }
    let mut pid = 0u32;
    GetWindowThreadProcessId(fg, &mut pid);
    if pid == 0 || pid == GetCurrentProcessId() {
        return;
    }
    let h = SetWinEventHook(
        EVENT_OBJECT_LOCATIONCHANGE,
        EVENT_OBJECT_LOCATIONCHANGE,
        0,
        Some(on_event),
        pid,
        0,
        WINEVENT_OUTOFCONTEXT,
    );
    LOC_HOOK.store(h, SeqCst);
}

fn evaluate() {
    if foreground_is_fullscreen() {
        let was = GAME_MODE.swap(true, SeqCst);
        refresh();
        if !was {
            let pid = GAME_PID.load(SeqCst);
            std::thread::spawn(move || crate::boost::on_game(pid)); // Game Boost (profil opsional)
        }
    } else if GAME_MODE.load(SeqCst) && !PENDING.swap(true, SeqCst) {
        // Tunda tampil kembali; cek ulang setelah 450 ms agar tidak berkedip.
        std::thread::spawn(|| {
            std::thread::sleep(Duration::from_millis(450));
            let fs = foreground_is_fullscreen();
            GAME_MODE.store(fs, SeqCst);
            PENDING.store(false, SeqCst);
            refresh();
            if !fs {
                crate::boost::on_game_end();
            }
        });
    }
}

fn refresh() {
    let hidden = (GAME_MODE.load(SeqCst) && HIDE_ENABLED.load(SeqCst)) || MANUAL_HIDDEN.load(SeqCst);
    if CURRENT_HIDDEN.swap(hidden, SeqCst) != hidden {
        apply(hidden);
    }
}

fn apply(hidden: bool) {
    let Some(app) = APP.get() else { return };
    crate::VISIBLE.store(!hidden, SeqCst);

    if hidden {
        // 1) lepas listener WinRT & matikan meter, 2) frontend berhenti render, 3) SW_HIDE.
        crate::media::suspend();
        let _ = app.emit("island-visibility", false);
        for label in ["main", "dock", "side"] {
            if let Some(w) = app.get_webview_window(label) {
                window::hide(&w);
            }
        }
    } else {
        for label in ["main", "dock", "side"] {
            if let Some(w) = app.get_webview_window(label) {
                window::position(&w); // resolusi bisa berubah setelah keluar dari game
                window::show_noactivate(&w);
            }
        }
        crate::media::resume();
        let _ = app.emit("island-visibility", true);
    }
}

/// Fullscreen = jendela depan menutupi seluruh monitor TUJUAN, tanpa title-bar
/// (WS_CAPTION) → membedakan game/borderless dari jendela maximized biasa.
pub fn foreground_is_fullscreen() -> bool {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd == 0 || IsWindowVisible(hwnd) == 0 || IsIconic(hwnd) != 0 {
            return false;
        }

        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == GetCurrentProcessId() {
            return false;
        }

        if (GetWindowLongW(hwnd, GWL_STYLE) as u32 & WS_CAPTION) == WS_CAPTION {
            return false;
        }

        let mut buf = [0u16; 64];
        let n = GetClassNameW(hwnd, buf.as_mut_ptr(), buf.len() as i32).max(0) as usize;
        let class = String::from_utf16_lossy(&buf[..n]);
        if IGNORED_CLASSES.contains(&class.as_str()) {
            return false;
        }

        let mut r = RECT::default();
        if GetWindowRect(hwnd, &mut r) == 0 {
            return false;
        }
        let mon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut mi: MONITORINFO = std::mem::zeroed();
        mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(mon, &mut mi) == 0 {
            return false;
        }
        let m = mi.rcMonitor;
        // Hanya relevan bila game berada di monitor tempat island/dock hidup.
        if (m.left, m.top, m.right, m.bottom) != *window::TARGET_RECT.lock().unwrap() {
            return false;
        }
        let fs = r.left <= m.left && r.top <= m.top && r.right >= m.right && r.bottom >= m.bottom;
        if fs {
            GAME_PID.store(pid, SeqCst);
        }
        fs
    }
}
