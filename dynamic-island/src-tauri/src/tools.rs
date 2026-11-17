//! Alat sistem untuk optimisasi & utilitas: power plan, Jangan Ganggu, Windows Game Mode,
//! pembersih RAM/temp/DNS, restart Explorer. Semua aksi bersifat eksplisit & dapat dipulihkan.

use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;
use std::ptr::null_mut;
use std::time::{Duration, SystemTime};

use serde::Serialize;

use crate::sys::*;

const NO_WINDOW: u32 = 0x0800_0000;
pub const GUID_BALANCED: &str = "381b4222-f694-41f0-9685-ff5bb260df2e";
pub const GUID_HIGH: &str = "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c";
pub const GUID_SAVER: &str = "a1841308-3541-4fab-bc81-f71556f20b4a";
const TOAST_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\PushNotifications";
const GAMEBAR_KEY: &str = r"Software\Microsoft\GameBar";

fn run(prog: &str, args: &[&str]) -> Option<String> {
    Command::new(prog)
        .args(args)
        .creation_flags(NO_WINDOW)
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
}

fn ok(prog: &str, args: &[&str]) -> bool {
    Command::new(prog).args(args).creation_flags(NO_WINDOW).status().map(|s| s.success()).unwrap_or(false)
}

// ───────── Power plan ─────────
pub fn power_active() -> Option<String> {
    let out = run("powercfg", &["/getactivescheme"])?;
    let i = out.find(':')?;
    out[i + 1..].split_whitespace().next().map(|s| s.to_lowercase())
}

fn valid_guid(g: &str) -> bool {
    g.len() == 36 && g.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
}

pub fn power_apply(g: &str) -> bool {
    if !valid_guid(g) {
        return false;
    }
    if ok("powercfg", &["/setactive", g]) {
        return true;
    }
    // Skema "High performance" kadang tersembunyi di laptop modern → buat lalu aktifkan.
    if g == GUID_HIGH {
        let _ = ok("powercfg", &["/duplicatescheme", GUID_HIGH]);
        return ok("powercfg", &["/setactive", g]);
    }
    false
}

#[tauri::command(async)]
pub fn power_get() -> Option<String> {
    power_active()
}

#[tauri::command(async)]
pub fn power_set(guid: String) -> bool {
    power_apply(&guid)
}

// ───────── Registry DWORD ─────────
pub fn reg_get(sub: &str, name: &str) -> Option<u32> {
    unsafe {
        let mut key: HKEY = 0;
        if RegOpenKeyExW(HKEY_CURRENT_USER, wide(sub).as_ptr(), 0, KEY_READ, &mut key) != 0 {
            return None;
        }
        let (mut ty, mut data, mut len) = (0u32, 0u32, 4u32);
        let rc = RegQueryValueExW(key, wide(name).as_ptr(), null_mut(), &mut ty, &mut data as *mut u32 as *mut u8, &mut len);
        RegCloseKey(key);
        (rc == 0 && ty == REG_DWORD).then_some(data)
    }
}

pub fn reg_set(sub: &str, name: &str, v: u32) -> bool {
    unsafe {
        let mut key: HKEY = 0;
        let rc = RegCreateKeyExW(
            HKEY_CURRENT_USER,
            wide(sub).as_ptr(),
            0,
            std::ptr::null(),
            0,
            KEY_SET_VALUE | KEY_READ,
            std::ptr::null(),
            &mut key,
            null_mut(),
        );
        if rc != 0 {
            return false;
        }
        let ok = RegSetValueExW(key, wide(name).as_ptr(), 0, REG_DWORD, &v as *const u32 as *const u8, 4) == 0;
        RegCloseKey(key);
        ok
    }
}

pub fn toast_raw() -> Option<u32> {
    reg_get(TOAST_KEY, "ToastEnabled")
}
pub fn toast_set(enabled: bool) -> bool {
    reg_set(TOAST_KEY, "ToastEnabled", enabled as u32)
}
pub fn wgm_raw() -> Option<u32> {
    reg_get(GAMEBAR_KEY, "AutoGameModeEnabled")
}
pub fn wgm_set(on: bool) -> bool {
    reg_set(GAMEBAR_KEY, "AutoGameModeEnabled", on as u32) && reg_set(GAMEBAR_KEY, "AllowAutoGameMode", on as u32)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickState {
    pub dnd: bool,
    pub game_mode: bool,
    pub power: Option<String>,
}

#[tauri::command(async)]
pub fn quick_state() -> QuickState {
    QuickState {
        dnd: toast_raw() == Some(0),
        game_mode: wgm_raw() != Some(0),
        power: power_active(),
    }
}

#[tauri::command]
pub fn dnd_set(on: bool) -> bool {
    toast_set(!on)
}

#[tauri::command]
pub fn game_mode_set(on: bool) -> bool {
    wgm_set(on)
}

// ───────── Pembersih ─────────
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RamClean {
    pub freed_mb: u64,
    pub procs: u32,
}

unsafe fn avail_phys() -> u64 {
    let mut ms: MEMORYSTATUSEX = std::mem::zeroed();
    ms.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
    GlobalMemoryStatusEx(&mut ms);
    ms.ullAvailPhys
}

/// Kosongkan working set semua proses yang boleh diakses. Efeknya sementara: halaman yang masih
/// dipakai dimuat ulang otomatis. Berguna sebelum membuka game besar saat RAM hampir penuh.
pub fn clean_ram_inner() -> RamClean {
    let me = unsafe { GetCurrentProcessId() };
    let mut n = 0u32;
    let before = unsafe { avail_phys() };
    for (pid, _, _) in crate::trim::snapshot() {
        if pid <= 4 || pid == me {
            continue;
        }
        unsafe {
            let h = OpenProcess(PROCESS_SET_QUOTA | PROCESS_QUERY_INFORMATION, 0, pid);
            if h != 0 {
                if K32EmptyWorkingSet(h) != 0 {
                    n += 1;
                }
                CloseHandle(h);
            }
        }
    }
    std::thread::sleep(Duration::from_millis(450));
    let after = unsafe { avail_phys() };
    RamClean { freed_mb: after.saturating_sub(before) / (1024 * 1024), procs: n }
}

#[tauri::command(async)]
pub fn clean_ram() -> RamClean {
    clean_ram_inner()
}

fn clean_dir(dir: &Path, cutoff: SystemTime, depth: u32) -> u64 {
    let mut freed = 0u64;
    let Ok(rd) = std::fs::read_dir(dir) else { return 0 };
    for e in rd.flatten() {
        let p = e.path();
        if e.file_name().to_string_lossy().to_lowercase().starts_with("dynamic-island") {
            continue; // jangan hapus berkas milik aplikasi ini (mis. state taskbar)
        }
        let Ok(md) = e.metadata() else { continue };
        if md.is_dir() {
            if depth < 6 {
                freed += clean_dir(&p, cutoff, depth + 1);
                let _ = std::fs::remove_dir(&p); // hanya berhasil jika sudah kosong
            }
        } else if md.modified().map(|m| m < cutoff).unwrap_or(false) {
            let sz = md.len();
            if std::fs::remove_file(&p).is_ok() {
                freed += sz;
            }
        }
    }
    freed
}

/// Hapus berkas di %TEMP% yang lebih tua dari 3 hari. Mengembalikan byte yang dibebaskan.
#[tauri::command(async)]
pub fn clean_temp() -> u64 {
    clean_dir(&std::env::temp_dir(), SystemTime::now() - Duration::from_secs(3 * 86_400), 0)
}

#[tauri::command(async)]
pub fn flush_dns() -> bool {
    ok("ipconfig", &["/flushdns"])
}

#[tauri::command(async)]
pub fn restart_explorer() -> bool {
    let _ = ok("taskkill", &["/f", "/im", "explorer.exe"]);
    std::thread::sleep(Duration::from_millis(600));
    Command::new("explorer.exe").spawn().is_ok()
}
