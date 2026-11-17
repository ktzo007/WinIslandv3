//! Indikator Kamera/Mikrofon dari Registry CapabilityAccess.
//!
//! Event-driven: RegNotifyChangeKeyValue + WaitForMultipleObjects (thread tidur
//! sampai Windows menulis LastUsedTimeStart/Stop). Tidak ada polling.
//! Aplikasi sedang memakai perangkat ⇔ LastUsedTimeStart > LastUsedTimeStop.

use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicIsize, Ordering::SeqCst};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::sys::*;

const BASE: &str = r"Software\Microsoft\Windows\CurrentVersion\CapabilityAccess\ConsentStore";
static STOP_EVENT: AtomicIsize = AtomicIsize::new(0);

#[derive(Serialize, Clone, Copy, PartialEq, Default)]
pub struct Privacy {
    pub cam: bool,
    pub mic: bool,
}

pub fn scan() -> Privacy {
    Privacy { cam: in_use("webcam"), mic: in_use("microphone") }
}

#[tauri::command]
pub fn get_privacy() -> Privacy {
    scan()
}

fn in_use(capability: &str) -> bool {
    unsafe {
        let path = wide(&format!(r"{BASE}\{capability}"));
        let mut root: HKEY = 0;
        if RegOpenKeyExW(HKEY_CURRENT_USER, path.as_ptr(), 0, KEY_READ, &mut root) != 0 {
            return false;
        }
        let hit = scan_key(root, 0);
        RegCloseKey(root);
        hit
    }
}

/// Struktur: <cap>\<PackageFamilyName>  dan  <cap>\NonPackaged\<app-exe>
unsafe fn scan_key(key: HKEY, depth: u32) -> bool {
    if qword(key, "LastUsedTimeStart") > qword(key, "LastUsedTimeStop") {
        return true;
    }
    if depth >= 2 {
        return false;
    }
    let mut index = 0u32;
    loop {
        let mut name = [0u16; 512];
        let mut len = name.len() as u32;
        let rc = RegEnumKeyExW(key, index, name.as_mut_ptr(), &mut len, null_mut(), null_mut(), null_mut(), null_mut());
        if rc != 0 {
            break;
        }
        index += 1;
        let mut sub: HKEY = 0;
        if RegOpenKeyExW(key, name.as_ptr(), 0, KEY_READ, &mut sub) == 0 {
            let hit = scan_key(sub, depth + 1);
            RegCloseKey(sub);
            if hit {
                return true;
            }
        }
    }
    false
}

unsafe fn qword(key: HKEY, value: &str) -> u64 {
    let name = wide(value);
    let mut ty = 0u32;
    let mut data = 0u64;
    let mut len = 8u32;
    let rc = RegQueryValueExW(key, name.as_ptr(), null_mut(), &mut ty, &mut data as *mut u64 as *mut u8, &mut len);
    if rc == 0 && ty == REG_QWORD {
        data
    } else {
        0
    }
}

pub fn start(app: AppHandle) {
    let _ = std::thread::Builder::new().name("privacy-watch".into()).spawn(move || unsafe {
        let ev = [
            CreateEventW(null(), 0, 0, null()), // webcam
            CreateEventW(null(), 0, 0, null()), // microphone
            CreateEventW(null(), 0, 0, null()), // stop
        ];
        STOP_EVENT.store(ev[2], SeqCst);

        let caps = ["webcam", "microphone"];
        let mut keys: [HKEY; 2] = [0; 2];
        let filter = REG_NOTIFY_CHANGE_NAME | REG_NOTIFY_CHANGE_LAST_SET;
        for i in 0..2 {
            let path = wide(&format!(r"{BASE}\{}", caps[i]));
            if RegOpenKeyExW(HKEY_CURRENT_USER, path.as_ptr(), 0, KEY_READ, &mut keys[i]) == 0 {
                RegNotifyChangeKeyValue(keys[i], 1, filter, ev[i], 1);
            } else {
                keys[i] = 0;
            }
        }

        let mut last = scan();
        loop {
            let r = WaitForMultipleObjects(3, ev.as_ptr(), 0, INFINITE);
            if r >= 2 {
                break; // sinyal stop atau error
            }
            let i = r as usize;
            if keys[i] != 0 {
                RegNotifyChangeKeyValue(keys[i], 1, filter, ev[i], 1); // re-arm
            }
            std::thread::sleep(Duration::from_millis(150)); // debounce rentetan penulisan registry
            let now = scan();
            if now != last {
                last = now;
                if crate::VISIBLE.load(SeqCst) {
                    let _ = app.emit("privacy-update", now);
                }
            }
        }

        for k in keys {
            if k != 0 {
                RegCloseKey(k);
            }
        }
        for e in ev {
            CloseHandle(e);
        }
    });
}

pub fn stop() {
    let e = STOP_EVENT.load(SeqCst);
    if e != 0 {
        unsafe {
            SetEvent(e);
        }
    }
}
