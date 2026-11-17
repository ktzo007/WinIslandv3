//! Kosongkan working set proses ini + seluruh turunannya (msedgewebview2.exe).
//! Dipanggil hanya saat kapsul sudah idle beberapa detik. Halaman yang jarang
//! dipakai pindah ke standby list (RAM fisik bebas untuk game/aplikasi lain)
//! dan dimuat lagi otomatis jika dibutuhkan.

use crate::sys::*;

/// (pid, parent_pid, nama_exe) seluruh proses.
pub fn snapshot() -> Vec<(u32, u32, String)> {
    let mut out = Vec::with_capacity(256);
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE || snap == 0 {
            return out;
        }
        let mut pe: PROCESSENTRY32W = std::mem::zeroed();
        pe.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        if Process32FirstW(snap, &mut pe) != 0 {
            loop {
                let n = pe.szExeFile.iter().position(|&c| c == 0).unwrap_or(260);
                out.push((pe.th32ProcessID, pe.th32ParentProcessID, String::from_utf16_lossy(&pe.szExeFile[..n])));
                if Process32NextW(snap, &mut pe) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snap);
    }
    out
}

pub fn trim_all() {
    let _ = std::thread::Builder::new().name("trim".into()).spawn(|| unsafe {
        let me = GetCurrentProcessId();
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE || snap == 0 {
            return;
        }

        let mut procs: Vec<(u32, u32)> = Vec::with_capacity(256);
        let mut pe: PROCESSENTRY32W = std::mem::zeroed();
        pe.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        if Process32FirstW(snap, &mut pe) != 0 {
            loop {
                procs.push((pe.th32ProcessID, pe.th32ParentProcessID));
                if Process32NextW(snap, &mut pe) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snap);

        // BFS pohon proses: kita → msedgewebview2 (browser) → renderer/gpu/utility.
        let mut tree = vec![me];
        let mut i = 0;
        while i < tree.len() {
            let parent = tree[i];
            i += 1;
            for &(pid, ppid) in &procs {
                if ppid == parent && !tree.contains(&pid) {
                    tree.push(pid);
                }
            }
        }

        for pid in tree {
            let h = OpenProcess(PROCESS_SET_QUOTA | PROCESS_QUERY_INFORMATION, 0, pid);
            if h != 0 {
                K32EmptyWorkingSet(h);
                CloseHandle(h);
            }
        }
    });
}
