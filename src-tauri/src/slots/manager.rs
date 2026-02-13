use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Manager, Wry};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_notification::NotificationExt;

use crate::clipboard::item::ClipboardItem;
use crate::clipboard::monitor::ClipboardMonitor;
use crate::storage::database::Database;

/// Start keyboard polling for slot shortcuts.
/// Uses Cmd+Ctrl+1 through Cmd+Ctrl+5 on macOS.
/// Polling-based approach (no CGEventTap) to avoid interfering with app windows.
pub fn start_shortcut_listener(app_handle: AppHandle<Wry>) {
    std::thread::spawn(move || {
        use device_query::{DeviceQuery, DeviceState, Keycode};

        let device_state = DeviceState::new();
        let mut last_slot: Option<u32> = None;

        println!("[ClipSlot] Shortcut listener started (polling)");

        loop {
            std::thread::sleep(Duration::from_millis(50));

            let keys = device_state.get_keys();

            // Check if Cmd and Ctrl are both held
            let cmd_held = keys.contains(&Keycode::Command);
            let ctrl_held = keys.contains(&Keycode::LControl) || keys.contains(&Keycode::RControl);

            if cmd_held && ctrl_held {
                let slot = if keys.contains(&Keycode::Key1) {
                    Some(1u32)
                } else if keys.contains(&Keycode::Key2) {
                    Some(2)
                } else if keys.contains(&Keycode::Key3) {
                    Some(3)
                } else if keys.contains(&Keycode::Key4) {
                    Some(4)
                } else if keys.contains(&Keycode::Key5) {
                    Some(5)
                } else {
                    None
                };

                // Debounce: only fire once per key press
                if slot != last_slot {
                    if let Some(slot_number) = slot {
                        println!("[ClipSlot] Detected Cmd+Ctrl+{}", slot_number);
                        handle_save_to_slot(&app_handle, slot_number);
                    }
                    last_slot = slot;
                }
            } else {
                last_slot = None;
            }
        }
    });
}

fn handle_save_to_slot(app: &AppHandle<Wry>, slot_number: u32) {
    // Read current clipboard content
    let text = match app.clipboard().read_text() {
        Ok(t) if !t.is_empty() => t,
        Ok(_) => {
            println!("[ClipSlot] Clipboard is empty, nothing to save");
            let _ = app
                .notification()
                .builder()
                .title("ClipSlot")
                .body("Clipboard is empty")
                .show();
            return;
        }
        Err(e) => {
            eprintln!("[ClipSlot] Failed to read clipboard: {}", e);
            return;
        }
    };

    let db = app.state::<Arc<Database>>();
    let device_id = {
        let hostname = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string());
        uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_DNS, hostname.as_bytes()).to_string()
    };

    let item = ClipboardItem::new(text, &device_id);

    // Tell the monitor to skip the next change
    if let Some(monitor) = app.try_state::<Arc<ClipboardMonitor>>() {
        monitor.set_skip_next();
    }

    match db.save_to_slot(slot_number, &item) {
        Ok(slot_info) => {
            let preview = slot_info
                .content_preview
                .as_deref()
                .unwrap_or("(empty)");
            let body = format!("Saved to {}: {}", slot_info.name, truncate(preview, 50));

            println!("[ClipSlot] {}", body);

            match app.notification().builder().title("ClipSlot").body(&body).show() {
                Ok(_) => println!("[ClipSlot] Notification sent"),
                Err(e) => eprintln!("[ClipSlot] Notification failed: {}", e),
            }
        }
        Err(e) => {
            eprintln!("[ClipSlot] Failed to save to slot {}: {}", slot_number, e);
            let _ = app
                .notification()
                .builder()
                .title("ClipSlot")
                .body(&format!("Failed to save to Slot {}", slot_number))
                .show();
        }
    }
}

fn truncate(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        s
    } else {
        &s[..max_len]
    }
}
