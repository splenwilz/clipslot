mod clipboard;
mod crypto;
mod slots;
mod storage;
mod sync;

use std::sync::Arc;

use clipboard::monitor::ClipboardMonitor;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::Manager;

fn get_or_create_device_id() -> String {
    // Generate a stable device ID based on hostname + a fixed namespace.
    // Phase 2 will persist this in the database.
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    let id = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_DNS, hostname.as_bytes());
    id.to_string()
}

/// Stored in Tauri managed state so tray menu events can update the pause label.
struct PauseMenuItem(MenuItem<tauri::Wry>);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            // Make this a tray-only app (no dock icon, no activation on click)
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            // Start clipboard monitoring
            let device_id = get_or_create_device_id();
            println!("[ClipSlot] Device ID: {}", device_id);

            let monitor = Arc::new(ClipboardMonitor::new());
            monitor.start(app.handle().clone(), device_id);
            app.manage(monitor);

            // Build the tray menu
            let quit = MenuItem::with_id(app, "quit", "Quit ClipSlot", true, None::<&str>)?;
            let show_history =
                MenuItem::with_id(app, "show_history", "Show History", true, None::<&str>)?;
            let pause =
                MenuItem::with_id(app, "pause", "Pause Monitoring", true, None::<&str>)?;

            // Store the pause menu item so we can update its text later
            app.manage(PauseMenuItem(pause.clone()));

            let menu = Menu::with_items(app, &[&show_history, &pause, &quit])?;

            // Build the tray icon
            let _tray = TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    "show_history" => {
                        println!("[ClipSlot] Show History clicked");
                    }
                    "pause" => {
                        let monitor = app.state::<Arc<ClipboardMonitor>>();
                        let is_paused = monitor.toggle_pause();

                        let pause_item = app.state::<PauseMenuItem>();
                        let label = if is_paused {
                            "Resume Monitoring"
                        } else {
                            "Pause Monitoring"
                        };
                        let _ = pause_item.0.set_text(label);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|_tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        // Left click shows menu (handled by show_menu_on_left_click)
                    }
                })
                .build(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running ClipSlot");
}
