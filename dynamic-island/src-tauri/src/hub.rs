//! "Hub": satu jendela tersembunyi (thread sendiri) untuk tiga sumber event Windows:
//!   • Clipboard   — AddClipboardFormatListener  → WM_CLIPBOARDUPDATE
//!   • Daya/baterai — RegisterPowerSettingNotification → WM_POWERBROADCAST
//!   • Hotkey global — RegisterHotKey → WM_HOTKEY
//! Thread ini tidur di GetMessageW; tidak ada timer/polling sama sekali.

use std::ptr::null;
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicUsize, Ordering::SeqCst};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use crate::sys::*;

static APP: OnceLock<AppHandle> = OnceLock::new();
static HUB: AtomicIsize = AtomicIsize::new(0);
static CLIP_ON: AtomicBool = AtomicBool::new(true);
static CLIP_MAX: AtomicUsize = AtomicUsize::new(25);
static HISTORY: Mutex<Vec<String>> = Mutex::new(Vec::new());
static LAST_POWER: Mutex<Option<Power>> = Mutex::new(None);

const WM_APP_HOTKEYS: u32 = 0x8001;

const GUID_ACDC_POWER_SOURCE: GUID = GUID {
    data1: 0x5d3e9a59,
    data2: 0xe9d5,
    data3: 0x4b00,
    data4: [0xa6, 0xbd, 0xff, 0x34, 0xff, 0x51, 0x65, 0x48],
};
const GUID_BATTERY_PERCENTAGE_REMAINING: GUID = GUID {
    data1: 0xa7ad8041,
    data2: 0xb45a,
    data3: 0x4cae,
    data4: [0x87, 0xa3, 0xee, 0xcb, 0xb4, 0x68, 0xa9, 0xe1],
};

// ───────────────────────── Power ─────────────────────────
#[derive(Serialize, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Power {
    pub has_battery: bool,
    pub ac: bool,
    pub charging: bool,
    pub percent: Option<u8>,
    pub minutes_left: Option<u32>,
}

pub fn power_status() -> Power {
    unsafe {
        let mut s: SYSTEM_POWER_STATUS = std::mem::zeroed();
        if GetSystemPowerStatus(&mut s) == 0 {
            return Power::default();
        }
        Power {
            has_battery: s.BatteryFlag != 128 && s.BatteryFlag != 255,
            ac: s.ACLineStatus == 1,
            charging: s.BatteryFlag & 8 != 0,
            percent: (s.BatteryLifePercent <= 100).then_some(s.BatteryLifePercent),
            minutes_left: (s.BatteryLifeTime != 0xFFFF_FFFF).then(|| s.BatteryLifeTime / 60),
        }
    }
}

#[tauri::command]
pub fn get_power() -> Power {
    power_status()
}

fn on_power() {
    let now = power_status();
    let mut last = LAST_POWER.lock().unwrap();
    if last.as_ref() != Some(&now) {
        *last = Some(now.clone());
        if crate::VISIBLE.load(SeqCst) {
            if let Some(app) = APP.get() {
                let _ = app.emit("power-update", now);
            }
        }
    }
}

// ───────────────────────── Clipboard ─────────────────────────
unsafe fn read_text(hwnd: HWND) -> Option<String> {
    // Hormati format "jangan simpan" yang dikirim password manager.
    let excl = RegisterClipboardFormatW(wide("ExcludeClipboardContentFromMonitorProcessing").as_ptr());
    if excl != 0 && IsClipboardFormatAvailable(excl) != 0 {
        return None;
    }
    let mut opened = false;
    for _ in 0..8 {
        if OpenClipboard(hwnd) != 0 {
            opened = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(15));
    }
    if !opened {
        return None;
    }
    let mut out = None;
    if IsClipboardFormatAvailable(CF_UNICODETEXT) != 0 {
        let h = GetClipboardData(CF_UNICODETEXT);
        if h != 0 {
            let p = GlobalLock(h) as *const u16;
            if !p.is_null() {
                let mut len = 0usize;
                while len < 4000 && *p.add(len) != 0 {
                    len += 1;
                }
                out = Some(String::from_utf16_lossy(std::slice::from_raw_parts(p, len)));
                GlobalUnlock(h);
            }
        }
    }
    CloseClipboard();
    out
}

unsafe fn on_clipboard(hwnd: HWND) {
    if !CLIP_ON.load(SeqCst) {
        return;
    }
    let Some(text) = read_text(hwnd) else { return };
    if text.trim().is_empty() {
        return;
    }
    {
        let mut h = HISTORY.lock().unwrap();
        h.retain(|x| x != &text);
        h.insert(0, text.clone());
        let max = CLIP_MAX.load(SeqCst);
        h.truncate(max);
    }
    if crate::VISIBLE.load(SeqCst) {
        if let Some(app) = APP.get() {
            let _ = app.emit("clipboard-update", text);
        }
    }
}

#[tauri::command]
pub fn clipboard_history() -> Vec<String> {
    HISTORY.lock().unwrap().clone()
}

#[tauri::command]
pub fn clipboard_enable(on: bool, max: usize) {
    CLIP_ON.store(on, SeqCst);
    CLIP_MAX.store(max.clamp(5, 100), SeqCst);
    if !on {
        HISTORY.lock().unwrap().clear(); // privasi: matikan = hapus semua
    }
}

#[tauri::command]
pub fn clipboard_remove(text: String) {
    HISTORY.lock().unwrap().retain(|x| x != &text);
}

#[tauri::command]
pub fn clipboard_clear() {
    HISTORY.lock().unwrap().clear();
}

/// Tulis teks ke clipboard sistem.
pub fn set_clipboard_text(text: &str) -> bool {
    unsafe {
        if OpenClipboard(0) == 0 {
            return false;
        }
        EmptyClipboard();
        let w = wide(text);
        let bytes = w.len() * 2;
        let h = GlobalAlloc(GMEM_MOVEABLE, bytes);
        let mut ok = false;
        if h != 0 {
            let p = GlobalLock(h) as *mut u16;
            if !p.is_null() {
                std::ptr::copy_nonoverlapping(w.as_ptr(), p, w.len());
                GlobalUnlock(h);
                ok = SetClipboardData(CF_UNICODETEXT, h) != 0;
            }
        }
        CloseClipboard();
        ok
    }
}

#[tauri::command]
pub fn clipboard_set(text: String) -> bool {
    set_clipboard_text(&text)
}

/// Salin lalu tempel (Ctrl+V) ke aplikasi yang sedang aktif — island tidak pernah mengambil fokus.
#[tauri::command]
pub fn clipboard_paste(text: String) {
    if set_clipboard_text(&text) {
        std::thread::spawn(|| {
            std::thread::sleep(Duration::from_millis(70));
            send_keys(&[(VK_CONTROL, false), (0x56, false), (0x56, true), (VK_CONTROL, true)]);
        });
    }
}

/// Kirim urutan tombol (vk, key_up).
pub fn send_keys(seq: &[(u16, bool)]) {
    let inputs: Vec<INPUT> = seq
        .iter()
        .map(|&(vk, up)| INPUT {
            typ: INPUT_KEYBOARD,
            u: INPUT_U {
                ki: KEYBDINPUT { wVk: vk, wScan: 0, dwFlags: if up { KEYEVENTF_KEYUP } else { 0 }, time: 0, dwExtraInfo: 0 },
            },
        })
        .collect();
    unsafe {
        SendInput(inputs.len() as u32, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32);
    }
}

// ───────────────────────── Hotkey (dapat dikonfigurasi) ─────────────────────────
#[derive(Deserialize, Clone)]
pub struct HkDef {
    pub name: String,
    pub mods: u32,
    pub vk: u32,
}

static HK: Mutex<Vec<(i32, String)>> = Mutex::new(Vec::new());
static HK_PENDING: Mutex<Vec<HkDef>> = Mutex::new(Vec::new());
static HK_FAILED: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn apply_hotkeys(hwnd: HWND, list: Vec<HkDef>) {
    let mut reg = HK.lock().unwrap();
    for (id, _) in reg.drain(..) {
        unsafe {
            UnregisterHotKey(hwnd, id);
        }
    }
    let mut failed = Vec::new();
    for (i, d) in list.into_iter().enumerate() {
        let id = 100 + i as i32;
        if unsafe { RegisterHotKey(hwnd, id, d.mods | MOD_NOREPEAT, d.vk) } != 0 {
            reg.push((id, d.name));
        } else {
            failed.push(d.name); // kombinasi sudah dipakai aplikasi lain
        }
    }
    *HK_FAILED.lock().unwrap() = failed;
}

/// Daftarkan ulang seluruh hotkey (mods: ALT=1, CTRL=2, SHIFT=4, WIN=8). Mengembalikan nama yang gagal.
#[tauri::command]
pub fn hotkeys_apply(items: Vec<HkDef>) -> Vec<String> {
    let h = HUB.load(SeqCst);
    if h == 0 {
        return vec![];
    }
    *HK_PENDING.lock().unwrap() = items;
    unsafe {
        SendMessageW(h, WM_APP_HOTKEYS, 0, 0); // sinkron: selesai di thread hub sebelum return
    }
    HK_FAILED.lock().unwrap().clone()
}

fn on_hotkey(id: i32) {
    let name = HK.lock().unwrap().iter().find(|(i, _)| *i == id).map(|(_, n)| n.clone());
    if let Some(n) = name {
        dispatch(&n);
    }
}

/// Aksi yang harus tetap bekerja saat game (UI tersembunyi) ditangani langsung di Rust.
fn dispatch(name: &str) {
    let key = |vk: u16| send_keys(&[(vk, false), (vk, true)]);
    match name {
        "toggle" => crate::fullscreen::toggle_manual(),
        "media_play" => key(0xB3),
        "media_next" => key(0xB0),
        "media_prev" => key(0xB1),
        "vol_up" => key(0xAF),
        "vol_down" => key(0xAE),
        "mute" => key(0xAD),
        "screenshot" => crate::apps::sys_action("screenshot".to_string()),
        "lock" => crate::apps::sys_action("lock".to_string()),
        "boost" => {
            std::thread::spawn(|| {
                crate::boost::boost_toggle();
            });
        }
        other => {
            if let Some(app) = APP.get() {
                let _ = app.emit("hotkey", other.to_string()); // diteruskan ke UI (panel, sidebar, dock, dll.)
            }
        }
    }
}

// ───────────────────────── Jendela & message loop ─────────────────────────
unsafe extern "system" fn hub_proc(hwnd: HWND, msg: u32, w: usize, l: isize) -> isize {
    match msg {
        WM_CLIPBOARDUPDATE => {
            on_clipboard(hwnd);
            return 0;
        }
        WM_POWERBROADCAST => {
            if w == PBT_POWERSETTINGCHANGE {
                on_power();
            }
            return 1;
        }
        WM_HOTKEY => {
            on_hotkey(w as i32);
            return 0;
        }
        WM_APP_HOTKEYS => {
            let list = std::mem::take(&mut *HK_PENDING.lock().unwrap());
            apply_hotkeys(hwnd, list);
            return 0;
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            return 0;
        }
        _ => {}
    }
    DefWindowProcW(hwnd, msg, w, l)
}

pub fn start(app: AppHandle) {
    let _ = APP.set(app);
    let _ = std::thread::Builder::new().name("hub".into()).spawn(|| unsafe {
        let hinst = GetModuleHandleW(null());
        let class = wide("DynamicIslandHub");
        let wc = WNDCLASSW {
            style: 0,
            lpfnWndProc: Some(hub_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinst,
            hIcon: 0,
            hCursor: 0,
            hbrBackground: 0,
            lpszMenuName: null(),
            lpszClassName: class.as_ptr(),
        };
        if RegisterClassW(&wc) == 0 {
            return;
        }
        let hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW as u32,
            class.as_ptr(),
            class.as_ptr(),
            WS_POPUP,
            0,
            0,
            0,
            0,
            0,
            0,
            hinst,
            null(),
        );
        if hwnd == 0 {
            return;
        }
        HUB.store(hwnd, SeqCst);

        AddClipboardFormatListener(hwnd);
        let p1 = RegisterPowerSettingNotification(hwnd, &GUID_ACDC_POWER_SOURCE, 0);
        let p2 = RegisterPowerSettingNotification(hwnd, &GUID_BATTERY_PERCENTAGE_REMAINING, 0);
        *LAST_POWER.lock().unwrap() = Some(power_status());

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, 0, 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        // Cleanup
        apply_hotkeys(hwnd, Vec::new());
        RemoveClipboardFormatListener(hwnd);
        if p1 != 0 {
            UnregisterPowerSettingNotification(p1);
        }
        if p2 != 0 {
            UnregisterPowerSettingNotification(p2);
        }
    });
}

pub fn stop() {
    let h = HUB.swap(0, SeqCst);
    if h != 0 {
        unsafe {
            PostMessageW(h, WM_CLOSE, 0, 0);
        }
    }
}
