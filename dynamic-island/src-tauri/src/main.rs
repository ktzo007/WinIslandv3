#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod apps;
mod audio;
mod boost;
mod fullscreen;
mod hub;
mod icons;
mod media;
mod netinfo;
mod privacy;
mod procs;
mod stats;
mod sys;
mod tools;
mod trim;
mod window;
mod wm;

use std::sync::atomic::AtomicBool;
use std::time::Duration;

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    Manager, RunEvent, WindowEvent,
};

/// false saat Game Mode / sembunyi manual → thread latar berhenti mengirim event.
pub static VISIBLE: AtomicBool = AtomicBool::new(true);

#[tauri::command]
fn set_island_shape(window: tauri::WebviewWindow, w: f64, h: f64) {
    window::set_shape(&window, w, h);
}

#[tauri::command]
fn trim_memory() {
    trim::trim_all();
}

/// Ikon kecil untuk preview saat file ditarik keluar dari shelf (dibutuhkan plugin drag).
#[tauri::command]
fn drag_icon() -> Option<String> {
    let p = std::env::temp_dir().join("dynamic-island-drag.png");
    if !p.exists() {
        std::fs::write(&p, include_bytes!("../icons/32x32.png")).ok()?;
    }
    Some(p.to_string_lossy().into_owned())
}

fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let settings = MenuItem::with_id(app, "settings", "Pengaturan…", true, None::<&str>)?;
    let toggle = MenuItem::with_id(app, "toggle", "Tampilkan / sembunyikan  (Ctrl+Alt+I)", true, None::<&str>)?;
    let reset = MenuItem::with_id(app, "reset", "Reset posisi", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Keluar", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&settings, &toggle, &reset, &sep, &quit])?;

    let mut builder = TrayIconBuilder::new()
        .tooltip("Dynamic Island")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "quit" => app.exit(0),
            "settings" => {
                let h = app.clone();
                std::thread::spawn(move || wm::open_settings(&h));
            }
            "toggle" => fullscreen::toggle_manual(),
            "reset" => {
                for label in ["main", "dock", "side"] {
                    if let Some(w) = app.get_webview_window(label) {
                        window::position(&w);
                    }
                }
            }
            _ => {}
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

fn main() {
    let app = tauri::Builder::default()
        // Cegah dua kapsul jika exe dijalankan dua kali.
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {}))
        .plugin(tauri_plugin_drag::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            set_island_shape,
            trim_memory,
            drag_icon,
            stats::get_stats,
            privacy::get_privacy,
            media::media_control,
            media::media_art,
            media::media_refresh,
            media::media_seek,
            hub::get_power,
            hub::clipboard_history,
            hub::clipboard_enable,
            hub::clipboard_clear,
            hub::clipboard_remove,
            hub::clipboard_set,
            hub::clipboard_paste,
            hub::hotkeys_apply,
            audio::audio_get,
            audio::audio_set_volume,
            audio::audio_set_mute,
            audio::audio_meter,
            apps::list_windows,
            apps::activate_window,
            apps::close_window,
            apps::launch,
            apps::get_icon,
            apps::taskbar_pins,
            apps::autostart_get,
            apps::autostart_set,
            apps::taskbar_autohide,
            apps::keep_awake,
            apps::sys_action,
            apps::island_focus,
            fullscreen::set_hide_in_fullscreen,
            fullscreen::set_manual_hidden,
            wm::open_settings_window,
            wm::dock_set_enabled,
            wm::list_monitors,
            wm::set_target_monitor,
            wm::side_set_enabled,
            wm::side_set_edge,
            boost::boost_profile,
            boost::boost_state,
            boost::boost_toggle,
            boost::game_sessions,
            tools::power_get,
            tools::power_set,
            tools::quick_state,
            tools::dnd_set,
            tools::game_mode_set,
            tools::clean_ram,
            tools::clean_temp,
            tools::flush_dns,
            tools::restart_explorer,
            netinfo::ping,
            netinfo::net_speed,
            netinfo::gpu_info,
            procs::top_processes,
            procs::kill_process,
        ])
        .on_window_event(|window, event| {
            if let WindowEvent::ScaleFactorChanged { .. } = event {
                if matches!(window.label(), "main" | "dock" | "side") {
                    if let Some(w) = window.app_handle().get_webview_window(window.label()) {
                        window::position(&w);
                    }
                }
            }
        })
        .setup(|app| {
            apps::restore_taskbar(); // pulihkan taskbar jika sesi sebelumnya crash

            let win = app
                .get_webview_window("main")
                .expect("jendela 'main' tidak ditemukan");
            window::init(&win);

            fullscreen::start(app.handle().clone());
            media::start(app.handle().clone());
            privacy::start(app.handle().clone());
            hub::start(app.handle().clone());
            boost::init(app.handle().clone());
            build_tray(app)?;

            // Trim RAM sekali setelah UI selesai dimuat.
            std::thread::spawn(|| {
                std::thread::sleep(Duration::from_secs(6));
                trim::trim_all();
            });
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("gagal membangun aplikasi Tauri");

    app.run(|_handle, event| {
        if let RunEvent::Exit = event {
            // Cleanup: unsubscribe WinRT, lepas hook & hotkey, tutup handle, pulihkan taskbar.
            media::stop();
            privacy::stop();
            fullscreen::stop();
            hub::stop();
            boost::stop(); // pulihkan power plan / notifikasi bila boost masih aktif
            apps::restore_taskbar();
        }
    });
}
