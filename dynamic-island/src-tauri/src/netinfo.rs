//! Metrik jaringan & GPU untuk gamer: ping (ICMP), kecepatan unduh/unggah, GPU NVIDIA.
//! Semua dipanggil frontend HANYA saat panel terkait terbuka (interval 2 dtk).

use std::ffi::c_void;
use std::net::{IpAddr, ToSocketAddrs};
use std::os::windows::process::CommandExt;
use std::process::Command;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicBool, Ordering::SeqCst};
use std::sync::Mutex;
use std::time::Instant;

use serde::Serialize;

use crate::sys::*;

// ───────── Ping ─────────
/// Round-trip time (ms) ke host IPv4, atau None jika timeout (1 dtk).
#[tauri::command(async)]
pub fn ping(host: String) -> Option<u32> {
    if host.is_empty() || host.len() > 80 {
        return None;
    }
    let ip4 = match host.parse::<IpAddr>() {
        Ok(IpAddr::V4(v)) => v,
        Ok(_) => return None,
        Err(_) => (host.as_str(), 0u16).to_socket_addrs().ok()?.find_map(|a| match a.ip() {
            IpAddr::V4(v) => Some(v),
            _ => None,
        })?,
    };
    let dest = u32::from_ne_bytes(ip4.octets());
    unsafe {
        let h = IcmpCreateFile();
        if h == INVALID_HANDLE_VALUE || h == 0 {
            return None;
        }
        let data = [0x61u8; 32];
        let mut reply = [0u8; 160];
        let n = IcmpSendEcho(h, dest, data.as_ptr() as *const c_void, 32, null(), reply.as_mut_ptr() as *mut c_void, 160, 1000);
        IcmpCloseHandle(h);
        if n == 0 {
            return None;
        }
        // ICMP_ECHO_REPLY: Address(0..4) Status(4..8) RoundTripTime(8..12)
        let status = u32::from_ne_bytes(reply[4..8].try_into().ok()?);
        if status != 0 {
            return None;
        }
        Some(u32::from_ne_bytes(reply[8..12].try_into().ok()?))
    }
}

// ───────── Kecepatan jaringan ─────────
static NET_PREV: Mutex<Option<(Instant, u64, u64)>> = Mutex::new(None);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetSpeed {
    pub down_bps: u64,
    pub up_bps: u64,
}

/// Total oktet masuk/keluar semua antarmuka aktif (non-loopback) dari MIB_IFTABLE (baris 860 byte).
fn if_totals() -> Option<(u64, u64)> {
    unsafe {
        let mut size = 0u32;
        GetIfTable(null_mut(), &mut size, 0);
        if size < 4 {
            return None;
        }
        let mut buf = vec![0u8; size as usize];
        if GetIfTable(buf.as_mut_ptr(), &mut size, 0) != 0 {
            return None;
        }
        let n = u32::from_ne_bytes(buf[0..4].try_into().ok()?) as usize;
        let (mut i, mut o) = (0u64, 0u64);
        for k in 0..n {
            let base = 4 + k * 860;
            if base + 860 > buf.len() {
                break;
            }
            let rd = |off: usize| u32::from_ne_bytes(buf[base + off..base + off + 4].try_into().unwrap());
            let (ty, oper) = (rd(516), rd(544));
            if ty == 24 || (oper != 4 && oper != 5) {
                continue; // loopback / tidak aktif
            }
            i += rd(552) as u64;
            o += rd(576) as u64;
        }
        Some((i, o))
    }
}

#[tauri::command]
pub fn net_speed() -> NetSpeed {
    let Some((i, o)) = if_totals() else { return NetSpeed { down_bps: 0, up_bps: 0 } };
    let mut prev = NET_PREV.lock().unwrap();
    let res = match *prev {
        Some((t, pi, po)) => {
            let dt = t.elapsed().as_secs_f64().max(0.2);
            NetSpeed { down_bps: (i.saturating_sub(pi) as f64 / dt) as u64, up_bps: (o.saturating_sub(po) as f64 / dt) as u64 }
        }
        None => NetSpeed { down_bps: 0, up_bps: 0 },
    };
    *prev = Some((Instant::now(), i, o));
    res
}

// ───────── GPU NVIDIA (nvidia-smi) ─────────
static NO_SMI: AtomicBool = AtomicBool::new(false);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Gpu {
    pub name: String,
    pub util: u32,
    pub temp: u32,
    pub mem_used: u32,
    pub mem_total: u32,
}

/// Hanya GPU NVIDIA dengan driver terpasang (nvidia-smi). AMD/Intel → None.
#[tauri::command(async)]
pub fn gpu_info() -> Option<Gpu> {
    if NO_SMI.load(SeqCst) {
        return None;
    }
    let out = Command::new("nvidia-smi")
        .args(["--query-gpu=name,utilization.gpu,temperature.gpu,memory.used,memory.total", "--format=csv,noheader,nounits"])
        .creation_flags(0x0800_0000)
        .output();
    let out = match out {
        Ok(o) if o.status.success() => o,
        Ok(_) => return None,
        Err(_) => {
            NO_SMI.store(true, SeqCst);
            return None;
        }
    };
    let txt = String::from_utf8_lossy(&out.stdout);
    let f: Vec<&str> = txt.lines().next()?.split(',').map(|s| s.trim()).collect();
    if f.len() < 5 {
        return None;
    }
    Some(Gpu {
        name: f[0].to_string(),
        util: f[1].parse().ok()?,
        temp: f[2].parse().ok()?,
        mem_used: f[3].parse().ok()?,
        mem_total: f[4].parse().ok()?,
    })
}
