mod clipboard;
mod crypto;
mod slots;
mod storage;
mod sync;

use std::sync::Arc;

use clipboard::item::ClipboardItem;
use clipboard::monitor::ClipboardMonitor;
use crypto::cipher::CryptoEngine;
use slots::SlotInfo;
use storage::database::Database;
use tauri::menu::{Menu, MenuItemBuilder, PredefinedMenuItem};
use tauri::tray::{TrayIcon, TrayIconBuilder};
use tauri::{AppHandle, Listener, Manager, WebviewUrl, WebviewWindowBuilder, Wry};

fn get_or_create_device_id() -> String {
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    let id = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_DNS, hostname.as_bytes());
    id.to_string()
}

/// Stored in Tauri managed state so we can update the tray menu dynamically.
struct TrayIconHandle(TrayIcon);

// ── Tray Menu ────────────────────────────────────────────────────────────────

fn build_tray_menu(app: &AppHandle, slots: &[SlotInfo], is_paused: bool) -> tauri::Result<Menu<Wry>> {
    let mut items: Vec<Box<dyn tauri::menu::IsMenuItem<Wry>>> = Vec::new();

    // Slot items
    for slot in slots {
        let label = if slot.is_empty {
            format!("{}: (empty)", slot.name)
        } else {
            let preview = slot.content_preview.as_deref().unwrap_or("");
            let short: String = preview.chars().take(30).collect();
            if preview.chars().count() > 30 {
                format!("{}: {}...", slot.name, short)
            } else {
                format!("{}: {}", slot.name, short)
            }
        };
        let id = format!("paste_slot_{}", slot.slot_number);
        let item = MenuItemBuilder::with_id(id, label)
            .enabled(!slot.is_empty)
            .build(app)?;
        items.push(Box::new(item));
    }

    items.push(Box::new(PredefinedMenuItem::separator(app)?));

    let show_history = MenuItemBuilder::with_id("show_history", "Show History").build(app)?;
    items.push(Box::new(show_history));

    items.push(Box::new(PredefinedMenuItem::separator(app)?));

    let pause_label = if is_paused { "Resume Monitoring" } else { "Pause Monitoring" };
    let pause = MenuItemBuilder::with_id("pause", pause_label).build(app)?;
    items.push(Box::new(pause));

    let settings = MenuItemBuilder::with_id("settings", "Settings...").build(app)?;
    items.push(Box::new(settings));

    items.push(Box::new(PredefinedMenuItem::separator(app)?));

    let quit = MenuItemBuilder::with_id("quit", "Quit ClipSlot").build(app)?;
    items.push(Box::new(quit));

    let refs: Vec<&dyn tauri::menu::IsMenuItem<Wry>> = items.iter().map(|b| b.as_ref()).collect();
    Menu::with_items(app, &refs)
}

fn refresh_tray_menu(app: &AppHandle) {
    let db = app.state::<Arc<Database>>();
    let monitor = app.state::<Arc<ClipboardMonitor>>();
    let is_paused = monitor.is_paused();

    let slots = db.get_all_slots().unwrap_or_default();
    match build_tray_menu(app, &slots, is_paused) {
        Ok(menu) => {
            let tray = app.state::<TrayIconHandle>();
            let _ = tray.0.set_menu(Some(menu));
        }
        Err(e) => eprintln!("[ClipSlot] Failed to rebuild tray menu: {}", e),
    }
}

fn handle_tray_menu_event(app: &AppHandle, event_id: &str) {
    match event_id {
        "quit" => {
            app.exit(0);
        }
        "show_history" => {
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
            monitor.toggle_pause();
            refresh_tray_menu(app);
        }
        "settings" => {
            if let Some(window) = app.get_webview_window("settings") {
                let _ = window.show();
                let _ = window.set_focus();
            } else {
                let _ = WebviewWindowBuilder::new(
                    app,
                    "settings",
                    WebviewUrl::App("index.html?page=settings".into()),
                )
                .title("ClipSlot Settings")
                .inner_size(560.0, 480.0)
                .resizable(true)
                .center()
                .build();
            }
        }
        id if id.starts_with("paste_slot_") => {
            if let Ok(slot_num) = id.strip_prefix("paste_slot_").unwrap().parse::<u32>() {
                slots::manager::handle_paste_from_slot(app, slot_num);
            }
        }
        _ => {}
    }
}

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
    db.clear_history().map_err(|e| e.to_string())
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
    let result = db
        .save_to_slot(slot_number, &item)
        .map_err(|e| e.to_string())?;
    refresh_tray_menu(&app);
    Ok(result)
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
    app: tauri::AppHandle,
    db: tauri::State<'_, Arc<Database>>,
    slot_number: u32,
) -> Result<bool, String> {
    let result = db.clear_slot(slot_number).map_err(|e| e.to_string())?;
    refresh_tray_menu(&app);
    Ok(result)
}

#[tauri::command]
fn rename_slot(
    app: tauri::AppHandle,
    db: tauri::State<'_, Arc<Database>>,
    slot_number: u32,
    name: String,
) -> Result<bool, String> {
    let result = db
        .rename_slot(slot_number, &name)
        .map_err(|e| e.to_string())?;
    refresh_tray_menu(&app);
    Ok(result)
}

// ── Settings Commands ────────────────────────────────────────────────────────

#[tauri::command]
fn get_settings(
    db: tauri::State<'_, Arc<Database>>,
) -> Result<std::collections::HashMap<String, String>, String> {
    let keys = ["history_limit", "auto_clear_on_quit", "excluded_apps"];
    let mut map = std::collections::HashMap::new();
    for key in keys {
        if let Some(val) = db.get_setting(key) {
            map.insert(key.to_string(), val);
        }
    }
    Ok(map)
}

const ALLOWED_SETTING_KEYS: &[&str] = &["history_limit", "auto_clear_on_quit", "excluded_apps"];

#[tauri::command]
fn update_setting(
    db: tauri::State<'_, Arc<Database>>,
    key: String,
    value: String,
) -> Result<bool, String> {
    if !ALLOWED_SETTING_KEYS.contains(&key.as_str()) {
        return Err(format!("Unknown setting key: {}", key));
    }
    db.set_setting(&key, &value).map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
fn toggle_monitoring(
    app: tauri::AppHandle,
    monitor: tauri::State<'_, Arc<ClipboardMonitor>>,
) -> Result<bool, String> {
    let is_paused = monitor.toggle_pause();
    refresh_tray_menu(&app);
    Ok(is_paused)
}

#[tauri::command]
fn save_item_to_slot(
    app: tauri::AppHandle,
    db: tauri::State<'_, Arc<Database>>,
    item_id: String,
    slot_number: u32,
) -> Result<SlotInfo, String> {
    let result = db
        .save_existing_item_to_slot(slot_number, &item_id)
        .map_err(|e| e.to_string())?;
    refresh_tray_menu(&app);
    Ok(result)
}

// ── Encryption Commands ──────────────────────────────────────────────────────

#[tauri::command]
fn is_encryption_enabled() -> bool {
    true
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
            get_settings,
            update_setting,
            toggle_monitoring,
            save_item_to_slot,
            is_encryption_enabled,
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            {
                app.set_activation_policy(tauri::ActivationPolicy::Accessory);

                extern "C" {
                    fn AXIsProcessTrusted() -> bool;
                }
                let trusted = unsafe { AXIsProcessTrusted() };
                if !trusted {
                    eprintln!("[ClipSlot] WARNING: Accessibility not granted — shortcuts won't work.");
                    eprintln!("[ClipSlot] Grant access in: System Settings > Privacy & Security > Accessibility");
                }
            }

            // Initialize encryption
            let master_key = crypto::keychain::get_or_create_master_key()
                .expect("failed to initialize encryption key");
            let crypto_engine = Arc::new(CryptoEngine::new(&master_key));

            // Initialize database
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("failed to resolve app data dir");
            let db = Arc::new(
                Database::new(data_dir, crypto_engine).expect("failed to initialize database"),
            );
            app.manage(db.clone());

            // Start clipboard monitoring
            let device_id = get_or_create_device_id();
            println!("[ClipSlot] Device ID: {}", device_id);

            let monitor = Arc::new(ClipboardMonitor::new());
            monitor.start(app.handle().clone(), device_id, db.clone());
            app.manage(monitor);

            // Start keyboard listener for slot shortcuts
            slots::manager::start_shortcut_listener(app.handle().clone());

            // Build initial tray menu with slot previews
            let slots = db.get_all_slots().unwrap_or_default();
            let menu = build_tray_menu(app.handle(), &slots, false)?;

            let tray = TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| {
                    handle_tray_menu_event(app, event.id.as_ref());
                })
                .build(app)?;

            app.manage(TrayIconHandle(tray));

            // Listen for slot changes from the shortcut listener thread
            let handle = app.handle().clone();
            app.listen("slot-changed", move |_| {
                refresh_tray_menu(&handle);
            });

            Ok(())
        })
        .on_window_event(|_window, event| {
            // Prevent app from quitting when windows are closed — it's a tray app
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = _window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running ClipSlot");
}
