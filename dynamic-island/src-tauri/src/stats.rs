//! RAM, CPU, disk, uptime. Dipanggil frontend HANYA saat panel Sistem terbuka & kursor di atas
//! kapsul (interval 2 dtk). GetSystemTimes + GlobalMemoryStatusEx: murah, tanpa PDH/WMI.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::sys::*;

static PREV: Mutex<Option<(u64, u64, u64, Instant)>> = Mutex::new(None);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    /// None pada sampel pertama (belum ada delta pembanding).
    pub cpu: Option<f32>,
    pub ram_percent: u32,
    pub ram_used_gb: f32,
    pub ram_total_gb: f32,
    pub disk_percent: u32,
    pub disk_free_gb: f32,
    pub uptime_s: u64,
}

fn ft(f: FILETIME) -> u64 {
    ((f.dwHighDateTime as u64) << 32) | f.dwLowDateTime as u64
}

#[tauri::command]
pub fn get_stats() -> Stats {
    unsafe {
        let mut ms: MEMORYSTATUSEX = std::mem::zeroed();
        ms.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
        GlobalMemoryStatusEx(&mut ms);

        let (mut i, mut k, mut u) = (FILETIME::default(), FILETIME::default(), FILETIME::default());
        GetSystemTimes(&mut i, &mut k, &mut u);
        let (i, k, u) = (ft(i), ft(k), ft(u));

        let mut prev = PREV.lock().unwrap();
        let cpu = match *prev {
            Some((pi, pk, pu, t)) if t.elapsed() < Duration::from_secs(6) => {
                let idle = i.wrapping_sub(pi);
                let total = k.wrapping_sub(pk).wrapping_add(u.wrapping_sub(pu)); // kernel sudah termasuk idle
                (total > 0).then(|| (100.0 * (1.0 - idle as f32 / total as f32)).clamp(0.0, 100.0))
            }
            _ => None,
        };
        *prev = Some((i, k, u, Instant::now()));

        let drive = format!("{}\\", std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into()));
        let (mut avail, mut total, mut free) = (0u64, 0u64, 0u64);
        GetDiskFreeSpaceExW(wide(&drive).as_ptr(), &mut avail, &mut total, &mut free);
        let gb = 1024.0 * 1024.0 * 1024.0;

        Stats {
            cpu,
            ram_percent: ms.dwMemoryLoad,
            ram_used_gb: (ms.ullTotalPhys - ms.ullAvailPhys) as f32 / gb,
            ram_total_gb: ms.ullTotalPhys as f32 / gb,
            disk_percent: if total > 0 { (100 - free * 100 / total) as u32 } else { 0 },
            disk_free_gb: free as f32 / gb as f32,
            uptime_s: GetTickCount64() / 1000,
        }
    }
}
