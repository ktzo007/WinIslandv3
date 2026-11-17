//! Volume master, mute, dan level audio (peak meter) via WASAPI/MMDevice.
//!
//! Fitur Cargo `audio` (default aktif). Jika bagian COM ini gagal dikompilasi di versi
//! crate `windows` Anda, bangun dengan:  cargo build --release --no-default-features
//! (island tetap berjalan; slider volume menjadi no-op dan visualizer memakai animasi lembut).

use serde::Serialize;
use tauri::AppHandle;

#[derive(Serialize, Default)]
pub struct AudioState {
    pub volume: f32,
    pub muted: bool,
}

#[tauri::command]
pub fn audio_get() -> AudioState {
    real::get()
}

#[tauri::command]
pub fn audio_set_volume(volume: f32) {
    real::set_volume(volume.clamp(0.0, 1.0));
}

/// Mengatur mute ke keadaan yang diminta (memakai tombol media bila perlu).
#[tauri::command]
pub fn audio_set_mute(muted: bool) {
    let cur = real::get();
    if cur.muted != muted {
        crate::hub::send_keys(&[(crate::sys::VK_VOLUME_MUTE, false), (crate::sys::VK_VOLUME_MUTE, true)]);
    }
}

/// Aktif hanya saat panel Musik terbuka & lagu diputar. Mati → thread tidur total (park).
#[tauri::command]
pub fn audio_meter(app: AppHandle, on: bool) {
    real::meter(app, on);
}

#[cfg(feature = "audio")]
mod real {
    use std::sync::atomic::{AtomicBool, Ordering::SeqCst};
    use std::sync::OnceLock;
    use std::thread::Thread;
    use std::time::Duration;

    use tauri::{AppHandle, Emitter};
    use windows::Win32::Media::Audio::Endpoints::{IAudioEndpointVolume, IAudioMeterInformation};
    use windows::Win32::Media::Audio::{eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator};
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED};

    use super::AudioState;

    static METER_ON: AtomicBool = AtomicBool::new(false);
    static METER_THREAD: OnceLock<Thread> = OnceLock::new();

    struct Endpoint {
        vol: IAudioEndpointVolume,
        meter: IAudioMeterInformation,
    }

    fn open() -> Option<Endpoint> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let en: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
            let dev = en.GetDefaultAudioEndpoint(eRender, eConsole).ok()?;
            let vol: IAudioEndpointVolume = dev.Activate(CLSCTX_ALL, None).ok()?;
            let meter: IAudioMeterInformation = dev.Activate(CLSCTX_ALL, None).ok()?;
            Some(Endpoint { vol, meter })
        }
    }

    pub fn get() -> AudioState {
        let Some(ep) = open() else { return AudioState::default() };
        unsafe {
            let volume = ep.vol.GetMasterVolumeLevelScalar().unwrap_or(0.0);
            let muted: bool = ep.vol.GetMute().map(|m| m.into()).unwrap_or(false);
            AudioState { volume, muted }
        }
    }

    pub fn set_volume(v: f32) {
        if let Some(ep) = open() {
            unsafe {
                let _ = ep.vol.SetMasterVolumeLevelScalar(v, std::ptr::null());
            }
        }
    }

    pub fn meter(app: AppHandle, on: bool) {
        METER_ON.store(on, SeqCst);
        if !on {
            return;
        }
        let t = METER_THREAD.get_or_init(|| {
            std::thread::Builder::new()
                .name("audio-meter".into())
                .spawn(move || {
                    let mut ep = open();
                    let mut tick = 0u32;
                    loop {
                        while !METER_ON.load(SeqCst) {
                            std::thread::park(); // 0% CPU saat meter mati
                            ep = open();
                        }
                        tick += 1;
                        if tick % 200 == 0 {
                            ep = open(); // perangkat output default bisa berganti
                        }
                        let peak = ep
                            .as_ref()
                            .and_then(|e| unsafe { e.meter.GetPeakValue().ok() })
                            .unwrap_or(0.0);
                        let _ = app.emit("audio-level", peak);
                        std::thread::sleep(Duration::from_millis(50));
                    }
                })
                .expect("thread audio-meter")
                .thread()
                .clone()
        });
        t.unpark();
    }
}

#[cfg(not(feature = "audio"))]
mod real {
    use super::AudioState;
    use tauri::AppHandle;

    pub fn get() -> AudioState {
        AudioState::default()
    }
    pub fn set_volume(_v: f32) {}
    pub fn meter(_app: AppHandle, _on: bool) {}
}
