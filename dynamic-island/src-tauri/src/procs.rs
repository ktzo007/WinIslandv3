//! Daftar proses teratas (RAM) dan penghenti proses. Proses sistem inti dilindungi.

use serde::Serialize;

use crate::sys::*;

const PROTECTED: &[&str] = &[
    "system", "registry", "memcompression", "system idle process", "smss.exe", "csrss.exe", "wininit.exe",
    "winlogon.exe", "services.exe", "lsass.exe", "svchost.exe", "dwm.exe", "explorer.exe", "fontdrvhost.exe",
    "sihost.exe", "taskhostw.exe", "ctfmon.exe", "searchhost.exe", "startmenuexperiencehost.exe",
    "shellexperiencehost.exe", "runtimebroker.exe", "audiodg.exe", "dynamic-island.exe", "msedgewebview2.exe",
    "conhost.exe", "wudfhost.exe", "spoolsv.exe", "securityhealthservice.exe", "msmpeng.exe",
];

fn is_protected(name: &str) -> bool {
    PROTECTED.contains(&name.to_lowercase().as_str())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Proc {
    pub name: String,
    pub count: u32,
    pub mem_mb: u64,
    pub protected: bool,
}

#[tauri::command(async)]
pub fn top_processes(n: usize) -> Vec<Proc> {
    let me = unsafe { GetCurrentProcessId() };
    let mut map: std::collections::HashMap<String, (u32, u64)> = std::collections::HashMap::new();
    for (pid, _, name) in crate::trim::snapshot() {
        if pid <= 4 || pid == me {
            continue;
        }
        let ws = unsafe {
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if h == 0 {
                continue;
            }
            let mut c: PROCESS_MEMORY_COUNTERS = std::mem::zeroed();
            c.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
            let ok = K32GetProcessMemoryInfo(h, &mut c, c.cb);
            CloseHandle(h);
            if ok == 0 {
                continue;
            }
            c.WorkingSetSize as u64
        };
        let e = map.entry(name).or_insert((0, 0));
        e.0 += 1;
        e.1 += ws;
    }
    let mut v: Vec<Proc> = map
        .into_iter()
        .map(|(name, (count, ws))| Proc { protected: is_protected(&name), name, count, mem_mb: ws / (1024 * 1024) })
        .collect();
    v.sort_by(|a, b| b.mem_mb.cmp(&a.mem_mb));
    v.truncate(n.clamp(1, 30));
    v
}

/// Akhiri semua proses dengan nama exe tertentu. Mengembalikan jumlah proses yang dihentikan.
pub fn kill_by_name(name: &str) -> u32 {
    if is_protected(name) {
        return 0;
    }
    let me = unsafe { GetCurrentProcessId() };
    let mut killed = 0;
    for (pid, _, n) in crate::trim::snapshot() {
        if pid <= 4 || pid == me || !n.eq_ignore_ascii_case(name) {
            continue;
        }
        unsafe {
            let h = OpenProcess(PROCESS_TERMINATE, 0, pid);
            if h != 0 {
                if TerminateProcess(h, 1) != 0 {
                    killed += 1;
                }
                CloseHandle(h);
            }
        }
    }
    killed
}

#[tauri::command(async)]
pub fn kill_process(name: String) -> u32 {
    kill_by_name(&name)
}
