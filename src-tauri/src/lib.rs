mod clipboard;
mod crypto;
mod slots;
mod storage;
mod sync;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

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

            // Build the tray menu
            let quit = MenuItem::with_id(app, "quit", "Quit ClipSlot", true, None::<&str>)?;
            let show_history =
                MenuItem::with_id(app, "show_history", "Show History", true, None::<&str>)?;
            let pause =
                MenuItem::with_id(app, "pause", "Pause Monitoring", true, None::<&str>)?;

            let menu = Menu::with_items(app, &[&show_history, &pause, &quit])?;

            // Build the tray icon
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    "show_history" => {
                        // Phase 2: will open history window
                        println!("Show History clicked");
                    }
                    "pause" => {
                        // Phase 1: will toggle clipboard monitoring
                        println!("Pause clicked");
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
