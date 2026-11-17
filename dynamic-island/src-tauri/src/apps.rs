//! Aplikasi & utilitas sistem: daftar jendela, aktivasi, peluncur, ikon, autostart,
//! auto-hide taskbar, keep-awake, dan aksi cepat.

use std::ffi::c_void;
use std::ptr::{null, null_mut};

use serde::Serialize;
use tauri::{AppHandle, Manager, WebviewWindow};

use crate::sys::*;

// ───────────────────────── Jendela berjalan ─────────────────────────
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WinInfo {
    hwnd: isize,
    title: String,
    exe: String,
    pid: u32,
}

pub unsafe fn exe_of(pid: u32) -> String {
    let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
    if h == 0 {
        return String::new();
    }
    let mut buf = [0u16; 520];
    let mut len = buf.len() as u32;
    let ok = QueryFullProcessImageNameW(h, 0, buf.as_mut_ptr(), &mut len);
    CloseHandle(h);
    if ok != 0 {
        String::from_utf16_lossy(&buf[..len as usize])
    } else {
        String::new()
    }
}

unsafe extern "system" fn enum_cb(hwnd: HWND, lparam: isize) -> BOOL {
    let out = &mut *(lparam as *mut Vec<WinInfo>);
    if IsWindowVisible(hwnd) == 0 {
        return 1;
    }
    let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
    let owner = GetWindow(hwnd, GW_OWNER);
    if (ex & WS_EX_TOOLWINDOW != 0 && ex & WS_EX_APPWINDOW == 0) || (owner != 0 && ex & WS_EX_APPWINDOW == 0) {
        return 1;
    }
    let mut cloaked = 0u32;
    DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED, &mut cloaked as *mut u32 as *mut c_void, 4);
    if cloaked != 0 {
        return 1; // jendela di desktop virtual lain / UWP tersembunyi
    }
    let mut tb = [0u16; 256];
    let n = GetWindowTextW(hwnd, tb.as_mut_ptr(), 256).max(0) as usize;
    if n == 0 {
        return 1;
    }
    let mut pid = 0u32;
    GetWindowThreadProcessId(hwnd, &mut pid);
    if pid == GetCurrentProcessId() {
        return 1;
    }
    let mut cb = [0u16; 64];
    let cn = GetClassNameW(hwnd, cb.as_mut_ptr(), 64).max(0) as usize;
    let class = String::from_utf16_lossy(&cb[..cn]);
    if matches!(class.as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd") {
        return 1;
    }
    out.push(WinInfo { hwnd, title: String::from_utf16_lossy(&tb[..n]), exe: exe_of(pid), pid });
    1
}

/// Urutan = z-order (paling atas dulu). Dipanggil hanya saat dock di-hover.
#[tauri::command(async)]
pub fn list_windows() -> Vec<WinInfo> {
    let mut v: Vec<WinInfo> = Vec::new();
    unsafe {
        EnumWindows(enum_cb, &mut v as *mut Vec<WinInfo> as isize);
    }
    v
}

#[tauri::command]
pub fn activate_window(hwnd: isize) {
    unsafe {
        if IsIconic(hwnd) != 0 {
            ShowWindow(hwnd, SW_RESTORE);
        }
        SetForegroundWindow(hwnd);
    }
}

#[tauri::command]
pub fn close_window(hwnd: isize) {
    unsafe {
        PostMessageW(hwnd, WM_CLOSE, 0, 0);
    }
}

// ───────────────────────── Peluncur & ikon ─────────────────────────
#[tauri::command]
pub fn launch(path: String) {
    unsafe {
        ShellExecuteW(0, wide("open").as_ptr(), wide(&path).as_ptr(), null(), null(), SW_SHOWNORMAL);
    }
}

#[tauri::command(async)]
pub fn get_icon(path: String) -> Option<String> {
    crate::icons::icon_data_url(&path)
}

#[derive(Serialize)]
pub struct Pin {
    name: String,
    path: String,
}

/// Aplikasi yang disematkan ke taskbar Windows (impor satu kali ke dock).
#[tauri::command]
pub fn taskbar_pins() -> Vec<Pin> {
    let Ok(appdata) = std::env::var("APPDATA") else { return vec![] };
    let dir = std::path::Path::new(&appdata).join(r"Microsoft\Internet Explorer\Quick Launch\User Pinned\TaskBar");
    let Ok(rd) = std::fs::read_dir(dir) else { return vec![] };
    let mut v: Vec<Pin> = rd
        .flatten()
        .filter(|e| e.path().extension().map(|x| x.eq_ignore_ascii_case("lnk")).unwrap_or(false))
        .map(|e| Pin {
            name: e.path().file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
            path: e.path().to_string_lossy().into_owned(),
        })
        .collect();
    v.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    v
}

// ───────────────────────── Autostart (HKCU\...\Run) ─────────────────────────
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_NAME: &str = "DynamicIsland";

#[tauri::command]
pub fn autostart_get() -> bool {
    unsafe {
        let mut key: HKEY = 0;
        if RegOpenKeyExW(HKEY_CURRENT_USER, wide(RUN_KEY).as_ptr(), 0, KEY_READ, &mut key) != 0 {
            return false;
        }
        let mut ty = 0u32;
        let mut len = 0u32;
        let rc = RegQueryValueExW(key, wide(RUN_NAME).as_ptr(), null_mut(), &mut ty, null_mut(), &mut len);
        RegCloseKey(key);
        rc == 0
    }
}

#[tauri::command]
pub fn autostart_set(on: bool) -> bool {
    unsafe {
        let mut key: HKEY = 0;
        let rc = RegCreateKeyExW(
            HKEY_CURRENT_USER,
            wide(RUN_KEY).as_ptr(),
            0,
            null(),
            0,
            KEY_SET_VALUE | KEY_READ,
            null(),
            &mut key,
            null_mut(),
        );
        if rc != 0 {
            return false;
        }
        let name = wide(RUN_NAME);
        let ok = if on {
            let exe = std::env::current_exe().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
            let val = wide(&format!("\"{exe}\""));
            RegSetValueExW(key, name.as_ptr(), 0, REG_SZ, val.as_ptr() as *const u8, (val.len() * 2) as u32) == 0
        } else {
            RegDeleteValueW(key, name.as_ptr());
            true
        };
        RegCloseKey(key);
        ok
    }
}

// ───────────────────────── Taskbar Windows: auto-hide ─────────────────────────
fn state_file() -> std::path::PathBuf {
    std::env::temp_dir().join("dynamic-island-taskbar.state")
}

unsafe fn abm(msg: u32, lparam: isize) -> usize {
    let mut d: APPBARDATA = std::mem::zeroed();
    d.cbSize = std::mem::size_of::<APPBARDATA>() as u32;
    d.lParam = lparam;
    SHAppBarMessage(msg, &mut d)
}

/// Aktifkan/nonaktifkan auto-hide bawaan Windows (fitur resmi OS, aman).
#[tauri::command]
pub fn taskbar_autohide(app: AppHandle, on: bool) {
    unsafe {
        let state = abm(ABM_GETSTATE, 0) as isize;
        if on {
            if !state_file().exists() {
                let _ = std::fs::write(state_file(), if state & ABS_AUTOHIDE != 0 { "1" } else { "0" });
            }
            abm(ABM_SETSTATE, state | ABS_AUTOHIDE);
        } else {
            restore_taskbar_inner();
        }
    }
    // Area kerja berubah → dock menyesuaikan posisi.
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(900));
        if let Some(w) = app.get_webview_window("dock") {
            crate::window::position(&w);
        }
    });
}

unsafe fn restore_taskbar_inner() {
    if let Ok(orig) = std::fs::read_to_string(state_file()) {
        let state = abm(ABM_GETSTATE, 0) as isize;
        let new_state = if orig.trim() == "1" { state | ABS_AUTOHIDE } else { state & !ABS_AUTOHIDE };
        abm(ABM_SETSTATE, new_state);
        let _ = std::fs::remove_file(state_file());
    }
}

/// Dipanggil saat startup (pulih dari crash) dan saat exit.
pub fn restore_taskbar() {
    unsafe { restore_taskbar_inner() }
}

// ───────────────────────── Aksi cepat ─────────────────────────
/// SetThreadExecutionState berlaku per-thread → command sinkron (thread utama persisten).
#[tauri::command]
pub fn keep_awake(on: bool) {
    unsafe {
        SetThreadExecutionState(if on {
            ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED
        } else {
            ES_CONTINUOUS
        });
    }
}

#[tauri::command]
pub fn sys_action(name: String) {
    unsafe {
        match name.as_str() {
            "lock" => {
                LockWorkStation();
            }
            "screenshot" => crate::hub::send_keys(&[
                (VK_LWIN, false),
                (VK_SHIFT, false),
                (0x53, false),
                (0x53, true),
                (VK_SHIFT, true),
                (VK_LWIN, true),
            ]),
            "taskmgr" => {
                ShellExecuteW(0, wide("open").as_ptr(), wide("taskmgr.exe").as_ptr(), null(), null(), SW_SHOWNORMAL);
            }
            "settings" => {
                ShellExecuteW(0, wide("open").as_ptr(), wide("ms-settings:").as_ptr(), null(), null(), SW_SHOWNORMAL);
            }
            _ => {}
        }
    }
}

/// Aktifkan fokus keyboard sementara untuk jendela island (input teks).
#[tauri::command]
pub fn island_focus(window: WebviewWindow, on: bool) {
    crate::window::set_focusable(&window, on);
}
