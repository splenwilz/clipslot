mod clipboard;
mod crypto;
mod slots;
mod storage;
mod sync;

use std::sync::Arc;

use clipboard::item::ClipboardItem;
use clipboard::monitor::ClipboardMonitor;
use slots::SlotInfo;
use storage::database::Database;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

fn get_or_create_device_id() -> String {
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    let id = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_DNS, hostname.as_bytes());
    id.to_string()
}

/// Stored in Tauri managed state so tray menu events can update the pause label.
struct PauseMenuItem(MenuItem<tauri::Wry>);

// ── Tauri Commands ──────────────────────────────────────────────────────────

#[tauri::command]
fn get_clipboard_history(
    db: tauri::State<'_, Arc<Database>>,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<Vec<ClipboardItem>, String> {
    db.get_history(limit.unwrap_or(50), offset.unwrap_or(0))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn search_history(
    db: tauri::State<'_, Arc<Database>>,
    query: String,
) -> Result<Vec<ClipboardItem>, String> {
    db.search(&query).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_history_item(
    db: tauri::State<'_, Arc<Database>>,
    id: String,
) -> Result<bool, String> {
    db.delete_item(&id).map_err(|e| e.to_string())
}

#[tauri::command]
fn clear_history(db: tauri::State<'_, Arc<Database>>) -> Result<u32, String> {
    let result = db.clear_history().map_err(|e| e.to_string());
    println!("[ClipSlot] clear_history called, result: {:?}", result);
    result
}

#[tauri::command]
fn get_history_count(db: tauri::State<'_, Arc<Database>>) -> Result<u32, String> {
    db.get_count().map_err(|e| e.to_string())
}

#[tauri::command]
fn copy_to_clipboard(
    app: tauri::AppHandle,
    monitor: tauri::State<'_, Arc<ClipboardMonitor>>,
    text: String,
) -> Result<(), String> {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    monitor.set_skip_next();
    app.clipboard()
        .write_text(&text)
        .map_err(|e| e.to_string())
}

// ── Slot Commands ────────────────────────────────────────────────────────────

#[tauri::command]
fn save_to_slot(
    app: tauri::AppHandle,
    db: tauri::State<'_, Arc<Database>>,
    slot_number: u32,
) -> Result<SlotInfo, String> {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    let text = app
        .clipboard()
        .read_text()
        .map_err(|e| e.to_string())?;
    if text.is_empty() {
        return Err("Clipboard is empty".to_string());
    }
    let device_id = get_or_create_device_id();
    let item = ClipboardItem::new(text, &device_id);
    db.save_to_slot(slot_number, &item)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_slot(
    db: tauri::State<'_, Arc<Database>>,
    slot_number: u32,
) -> Result<SlotInfo, String> {
    db.get_slot(slot_number).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_all_slots(db: tauri::State<'_, Arc<Database>>) -> Result<Vec<SlotInfo>, String> {
    db.get_all_slots().map_err(|e| e.to_string())
}

#[tauri::command]
fn clear_slot(
    db: tauri::State<'_, Arc<Database>>,
    slot_number: u32,
) -> Result<bool, String> {
    db.clear_slot(slot_number).map_err(|e| e.to_string())
}

#[tauri::command]
fn rename_slot(
    db: tauri::State<'_, Arc<Database>>,
    slot_number: u32,
    name: String,
) -> Result<bool, String> {
    db.rename_slot(slot_number, &name)
        .map_err(|e| e.to_string())
}

// ── App Entry ───────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![
            get_clipboard_history,
            search_history,
            delete_history_item,
            clear_history,
            get_history_count,
            copy_to_clipboard,
            save_to_slot,
            get_slot,
            get_all_slots,
            clear_slot,
            rename_slot,
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            {
                app.set_activation_policy(tauri::ActivationPolicy::Accessory);

                // Check macOS Accessibility permissions (needed for global shortcuts)
                extern "C" {
                    fn AXIsProcessTrusted() -> bool;
                }
                let trusted = unsafe { AXIsProcessTrusted() };
                if !trusted {
                    eprintln!("[ClipSlot] WARNING: Accessibility not granted — shortcuts won't work.");
                    eprintln!("[ClipSlot] Grant access in: System Settings > Privacy & Security > Accessibility");
                }
            }

            // Initialize database
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("failed to resolve app data dir");
            let db = Arc::new(
                Database::new(data_dir).expect("failed to initialize database"),
            );
            app.manage(db.clone());

            // Start clipboard monitoring
            let device_id = get_or_create_device_id();
            println!("[ClipSlot] Device ID: {}", device_id);

            let monitor = Arc::new(ClipboardMonitor::new());
            monitor.start(app.handle().clone(), device_id, db);
            app.manage(monitor);

            // Start keyboard listener for slot shortcuts (Cmd+Ctrl+1-5)
            slots::manager::start_shortcut_listener(app.handle().clone());

            // Build the tray menu
            let quit = MenuItem::with_id(app, "quit", "Quit ClipSlot", true, None::<&str>)?;
            let show_history =
                MenuItem::with_id(app, "show_history", "Show History", true, None::<&str>)?;
            let pause =
                MenuItem::with_id(app, "pause", "Pause Monitoring", true, None::<&str>)?;

            app.manage(PauseMenuItem(pause.clone()));

            let menu = Menu::with_items(app, &[&show_history, &pause, &quit])?;

            let _tray = TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    "show_history" => {
                        // Show or focus the history window
                        if let Some(window) = app.get_webview_window("history") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        } else {
                            let _ = WebviewWindowBuilder::new(
                                app,
                                "history",
                                WebviewUrl::App("index.html".into()),
                            )
                            .title("ClipSlot History")
                            .inner_size(480.0, 600.0)
                            .resizable(true)
                            .center()
                            .build();
                        }
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
