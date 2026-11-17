//! Manajer jendela dinamis: Pengaturan & Dock dibuat saat dibutuhkan dan DIHANCURKAN saat
//! tidak dipakai → proses renderer WebView2-nya ikut bebas (hemat RAM).
//! Catatan Windows: pembuatan window harus dari thread lain (command `async`/thread), bukan
//! command sinkron, agar tidak deadlock.

use serde::Serialize;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

/// HARUS identik dengan `additionalBrowserArgs` di tauri.conf.json: semua jendela berbagi
/// satu environment WebView2, argumen berbeda menyebabkan gagal membuat webview.
pub const BROWSER_ARGS: &str = "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection,Translate --disable-background-networking --disable-component-update --disable-sync --js-flags=--max-old-space-size=96";

pub fn open_settings(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("settings") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
        return;
    }
    let _ = WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("index.html".into()))
        .title("Pengaturan — Dynamic Island")
        .inner_size(880.0, 640.0)
        .min_inner_size(780.0, 560.0)
        .center()
        .decorations(true)
        .transparent(false)
        .always_on_top(false)
        .skip_taskbar(false)
        .theme(Some(tauri::Theme::Dark))
        .additional_browser_args(BROWSER_ARGS)
        .build();
}

pub fn set_dock(app: &AppHandle, on: bool) {
    let existing = app.get_webview_window("dock");
    match (existing, on) {
        (None, true) => {
            let built = WebviewWindowBuilder::new(app, "dock", WebviewUrl::App("index.html".into()))
                .title("Dock")
                .inner_size(1040.0, 320.0)
                .resizable(false)
                .decorations(false)
                .transparent(true)
                .shadow(false)
                .always_on_top(true)
                .skip_taskbar(true)
                .focused(false)
                .additional_browser_args(BROWSER_ARGS)
                .build();
            if let Ok(w) = built {
                crate::window::init(&w);
            }
        }
        (Some(w), false) => {
            let _ = w.destroy();
        }
        _ => {}
    }
}

pub fn set_side(app: &AppHandle, on: bool) {
    let existing = app.get_webview_window("side");
    match (existing, on) {
        (None, true) => {
            // Tinggi menyesuaikan monitor (laptop 768p tetap muat).
            let h = app
                .primary_monitor()
                .ok()
                .flatten()
                .map(|m| (m.size().height as f64 / m.scale_factor() - 40.0).clamp(480.0, 780.0))
                .unwrap_or(720.0);
            let built = WebviewWindowBuilder::new(app, "side", WebviewUrl::App("index.html".into()))
                .title("Sidebar")
                .inner_size(452.0, h)
                .resizable(false)
                .decorations(false)
                .transparent(true)
                .shadow(false)
                .always_on_top(true)
                .skip_taskbar(true)
                .focused(false)
                .additional_browser_args(BROWSER_ARGS)
                .build();
            if let Ok(w) = built {
                crate::window::init(&w);
            }
        }
        (Some(w), false) => {
            let _ = w.destroy();
        }
        _ => {}
    }
}

#[tauri::command(async)]
pub fn side_set_enabled(app: AppHandle, on: bool) {
    set_side(&app, on);
}

#[tauri::command]
pub fn side_set_edge(app: AppHandle, right: bool) {
    crate::window::SIDE_RIGHT.store(right, std::sync::atomic::Ordering::SeqCst);
    if let Some(w) = app.get_webview_window("side") {
        crate::window::position(&w);
    }
}

#[tauri::command(async)]
pub fn open_settings_window(app: AppHandle) {
    open_settings(&app);
}

#[tauri::command(async)]
pub fn dock_set_enabled(app: AppHandle, on: bool) {
    set_dock(&app, on);
}

#[derive(Serialize)]
pub struct MonitorInfo {
    index: usize,
    name: String,
    width: u32,
    height: u32,
    primary: bool,
}

#[tauri::command]
pub fn list_monitors(app: AppHandle) -> Vec<MonitorInfo> {
    let primary = app.primary_monitor().ok().flatten();
    app.available_monitors()
        .unwrap_or_default()
        .iter()
        .enumerate()
        .map(|(i, m)| MonitorInfo {
            index: i,
            name: m.name().cloned().unwrap_or_else(|| format!("Monitor {}", i + 1)),
            width: m.size().width,
            height: m.size().height,
            primary: primary.as_ref().map(|p| p.position() == m.position()).unwrap_or(i == 0),
        })
        .collect()
}

#[tauri::command]
pub fn set_target_monitor(app: AppHandle, index: usize) {
    for label in ["main", "dock", "side"] {
        if let Some(w) = app.get_webview_window(label) {
            crate::window::set_target_monitor(&w, index);
        }
    }
}
