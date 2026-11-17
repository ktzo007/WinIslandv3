//! Game Boost: profil optimisasi yang diterapkan otomatis saat game fullscreen terdeteksi
//! (atau manual lewat tombol/hotkey) dan DIPULIHKAN sepenuhnya saat selesai.
//!
//! Yang dilakukan (semua opsional di profil): ganti power plan ke High performance, aktifkan
//! Jangan Ganggu (matikan toast), aktifkan Windows Game Mode, naikkan prioritas proses game,
//! bersihkan working set proses lain, dan akhiri daftar aplikasi latar yang Anda pilih sendiri.
//! Tidak ada injeksi/hook ke game → aman untuk anti-cheat.

use std::io::Write;
use std::sync::atomic::{AtomicU32, Ordering::SeqCst};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use crate::apps::exe_of;
use crate::sys::*;
use crate::tools;

static APP: OnceLock<AppHandle> = OnceLock::new();
static PROFILE: Mutex<Option<Profile>> = Mutex::new(None);
static SAVED: Mutex<Option<Saved>> = Mutex::new(None);
static END_GEN: AtomicU32 = AtomicU32::new(0);

/// Aplikasi fullscreen yang BUKAN game (mode "semua fullscreen" mengabaikannya).
const NON_GAME: &[&str] = &[
    "chrome.exe", "msedge.exe", "firefox.exe", "brave.exe", "opera.exe", "vivaldi.exe", "vlc.exe", "mpc-hc64.exe",
    "mpc-hc.exe", "potplayermini64.exe", "spotify.exe", "powerpnt.exe", "winword.exe", "excel.exe", "acrord32.exe",
    "mpv.exe", "netflix.exe", "wmplayer.exe", "code.exe", "devenv.exe", "obs64.exe", "teams.exe", "zoom.exe",
];

#[derive(Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub auto: bool,
    /// "all" = semua fullscreen non-browser/pemutar video · "list" = hanya exe di `games`
    pub scope: String,
    pub games: Vec<String>,
    pub power: bool,
    pub dnd: bool,
    pub priority: bool,
    pub clean_ram: bool,
    pub game_mode: bool,
    pub close: Vec<String>,
}

struct Saved {
    guid: Option<String>,
    toast: Option<u32>,
    wgm: Option<u32>,
    exe: String,
    since: u64,
    auto: bool,
    pid: u32,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BoostState {
    pub active: bool,
    pub exe: String,
    pub since: u64,
    pub auto: bool,
}

fn now_s() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

pub fn init(app: AppHandle) {
    let _ = APP.set(app);
}

fn state() -> BoostState {
    match SAVED.lock().unwrap().as_ref() {
        Some(s) => BoostState { active: true, exe: s.exe.clone(), since: s.since, auto: s.auto },
        None => BoostState { active: false, exe: String::new(), since: 0, auto: false },
    }
}

fn announce() {
    if let Some(app) = APP.get() {
        let _ = app.emit("boost-state", state());
    }
}

#[tauri::command]
pub fn boost_profile(profile: Profile) {
    *PROFILE.lock().unwrap() = Some(profile);
}

#[tauri::command]
pub fn boost_state() -> BoostState {
    state()
}

#[tauri::command(async)]
pub fn boost_toggle() -> BoostState {
    if SAVED.lock().unwrap().is_some() {
        stop();
    } else {
        start(0, false);
    }
    state()
}

fn base_name(path: &str) -> String {
    path.rsplit(|c: char| c == '\\' || c == '/').next().unwrap_or("").to_lowercase()
}

/// Dipanggil fullscreen.rs saat game fullscreen terdeteksi.
pub fn on_game(pid: u32) {
    END_GEN.fetch_add(1, SeqCst); // batalkan penghentian yang tertunda (mis. setelah Alt-Tab)
    let Some(p) = PROFILE.lock().unwrap().clone() else { return };
    if !p.auto || SAVED.lock().unwrap().is_some() {
        return;
    }
    let exe = unsafe { exe_of(pid) };
    let name = base_name(&exe);
    let listed = p.games.iter().any(|g| g.eq_ignore_ascii_case(&name));
    let allowed = if p.scope == "list" { listed } else { listed || !NON_GAME.contains(&name.as_str()) };
    if allowed {
        start(pid, true);
    }
}

/// Dipanggil saat tidak lagi fullscreen. Penghentian ditunda 8 dtk agar Alt-Tab sesaat tidak
/// membuat power plan & pengaturan lain bolak-balik.
pub fn on_game_end() {
    let gen = END_GEN.fetch_add(1, SeqCst) + 1;
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(8));
        let auto = SAVED.lock().unwrap().as_ref().map(|s| s.auto).unwrap_or(false);
        if END_GEN.load(SeqCst) == gen && auto {
            stop();
        }
    });
}

fn start(pid: u32, auto: bool) {
    if SAVED.lock().unwrap().is_some() {
        return;
    }
    let p = PROFILE.lock().unwrap().clone().unwrap_or_default();
    let exe = if pid != 0 { unsafe { exe_of(pid) } } else { String::new() };
    let saved = Saved {
        guid: tools::power_active(),
        toast: tools::toast_raw(),
        wgm: tools::wgm_raw(),
        exe,
        since: now_s(),
        auto,
        pid,
    };

    if p.power {
        tools::power_apply(tools::GUID_HIGH);
    }
    if p.dnd {
        tools::toast_set(false);
    }
    if p.game_mode {
        tools::wgm_set(true);
    }
    if p.priority && pid != 0 {
        unsafe {
            let h = OpenProcess(PROCESS_SET_INFORMATION | PROCESS_QUERY_INFORMATION, 0, pid);
            if h != 0 {
                SetPriorityClass(h, HIGH_PRIORITY_CLASS);
                CloseHandle(h);
            }
        }
    }
    if p.clean_ram {
        tools::clean_ram_inner();
    }
    for name in &p.close {
        crate::procs::kill_by_name(name);
    }
    *SAVED.lock().unwrap() = Some(saved);
    announce();
}

pub fn stop() {
    let Some(s) = SAVED.lock().unwrap().take() else { return };
    let p = PROFILE.lock().unwrap().clone().unwrap_or_default();
    if p.power {
        if let Some(g) = &s.guid {
            tools::power_apply(g);
        }
    }
    if p.dnd {
        tools::toast_set(s.toast.unwrap_or(1) != 0);
    }
    if p.game_mode {
        tools::wgm_set(s.wgm.unwrap_or(1) != 0);
    }
    if s.pid != 0 {
        log_session(&s.exe, s.since, now_s());
    }
    announce();
}

// ───────── Riwayat bermain ─────────
fn log_path() -> Option<std::path::PathBuf> {
    let dir = std::path::Path::new(&std::env::var("APPDATA").ok()?).join("DynamicIsland");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join("sessions.log"))
}

fn log_session(exe: &str, start: u64, end: u64) {
    if end.saturating_sub(start) < 60 || exe.is_empty() {
        return; // abaikan sesi < 1 menit
    }
    if let Some(p) = log_path() {
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(p) {
            let _ = writeln!(f, "{}|{}|{}", exe.replace('|', "_"), start, end);
        }
    }
}

#[derive(Serialize)]
pub struct Session {
    pub exe: String,
    pub start: u64,
    pub end: u64,
}

#[tauri::command]
pub fn game_sessions() -> Vec<Session> {
    let Some(p) = log_path() else { return vec![] };
    let Ok(txt) = std::fs::read_to_string(p) else { return vec![] };
    let mut v: Vec<Session> = txt
        .lines()
        .filter_map(|l| {
            let mut it = l.split('|');
            Some(Session { exe: it.next()?.to_string(), start: it.next()?.parse().ok()?, end: it.next()?.parse().ok()? })
        })
        .collect();
    let n = v.len();
    if n > 400 {
        v.drain(..n - 400);
    }
    v
}
