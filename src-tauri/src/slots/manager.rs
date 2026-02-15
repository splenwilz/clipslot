use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager, Wry};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_notification::NotificationExt;

use crate::clipboard::item::ClipboardItem;
use crate::clipboard::monitor::ClipboardMonitor;
use crate::storage::database::Database;
use crate::sync::manager::SyncManager;

/// Start keyboard polling for slot shortcuts.
/// macOS:   Save = Cmd+Ctrl+1-5,    Paste = Cmd+Option+1-5
/// Windows: Save = Ctrl+Shift+1-5,  Paste = Alt+Shift+1-5
pub fn start_shortcut_listener(app_handle: AppHandle<Wry>) {
    std::thread::spawn(move || {
        use device_query::{DeviceQuery, DeviceState, Keycode};

        let device_state = DeviceState::new();
        let mut last_save_slot: Option<u32> = None;
        let mut last_paste_slot: Option<u32> = None;

        clog!("Shortcut listener started (polling)");
        #[cfg(target_os = "macos")]
        clog!("Shortcuts: Save=Cmd+Ctrl+1-5, Paste=Cmd+Option+1-5");
        #[cfg(not(target_os = "macos"))]
        clog!("Shortcuts: Save=Ctrl+Shift+1-5, Paste=Alt+Shift+1-5");

        let mut log_keys_once = true;

        loop {
            std::thread::sleep(Duration::from_millis(50));

            let keys = device_state.get_keys();

            // Log detected keys once when any modifier is held (for debugging)
            if !keys.is_empty() && log_keys_once {
                let has_modifier = keys.iter().any(|k| matches!(k,
                    Keycode::LControl | Keycode::RControl |
                    Keycode::LShift | Keycode::RShift |
                    Keycode::LAlt | Keycode::RAlt |
                    Keycode::Command | Keycode::LOption
                ));
                if has_modifier {
                    clog!("Keys detected: {:?}", keys);
                    log_keys_once = false;
                }
            }
            if keys.is_empty() {
                log_keys_once = true;
            }

            let ctrl_held =
                keys.contains(&Keycode::LControl) || keys.contains(&Keycode::RControl);
            #[allow(unused_variables)]
            let shift_held =
                keys.contains(&Keycode::LShift) || keys.contains(&Keycode::RShift);
            #[allow(unused_variables)]
            let alt_held = keys.contains(&Keycode::LAlt) || keys.contains(&Keycode::RAlt);

            // Determine which number key (1-5) is pressed
            let slot_number = if keys.contains(&Keycode::Key1) {
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

            // Platform-specific modifier detection
            #[cfg(target_os = "macos")]
            let (save_combo, paste_combo) = {
                let cmd_held = keys.contains(&Keycode::Command);
                let option_held = keys.contains(&Keycode::LOption) || keys.contains(&Keycode::RAlt);
                // Save: Cmd+Ctrl+N (without Option)
                let save = cmd_held && ctrl_held && !option_held;
                // Paste: Cmd+Option+N (without Ctrl)
                let paste = cmd_held && option_held && !ctrl_held;
                (save, paste)
            };

            #[cfg(not(target_os = "macos"))]
            let (save_combo, paste_combo) = {
                // Save: Ctrl+Shift+N (without Alt)
                let save = ctrl_held && shift_held && !alt_held;
                // Paste: Alt+Shift+N (without Ctrl)
                let paste = alt_held && shift_held && !ctrl_held;
                (save, paste)
            };

            // Save to slot
            if save_combo {
                if slot_number != last_save_slot {
                    if let Some(n) = slot_number {
                        clog!("Shortcut: SAVE to slot {}", n);
                        handle_save_to_slot(&app_handle, n);
                    }
                    last_save_slot = slot_number;
                }
            } else {
                last_save_slot = None;
            }

            // Paste from slot
            if paste_combo {
                if slot_number != last_paste_slot {
                    if let Some(n) = slot_number {
                        clog!("Shortcut: PASTE from slot {}", n);
                        handle_paste_from_slot(&app_handle, n);
                    }
                    last_paste_slot = slot_number;
                }
            } else {
                last_paste_slot = None;
            }
        }
    });
}

pub fn handle_save_to_slot(app: &AppHandle<Wry>, slot_number: u32) {
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

            match app
                .notification()
                .builder()
                .title("ClipSlot")
                .body(&body)
                .show()
            {
                Ok(_) => println!("[ClipSlot] Notification sent"),
                Err(e) => eprintln!("[ClipSlot] Notification failed: {}", e),
            }

            // Signal tray menu to refresh
            let _ = app.emit("slot-changed", ());

            // Notify sync manager so the slot change reaches other devices
            if let Some(sync) = app.try_state::<Arc<SyncManager>>() {
                let sync = sync.inner().clone();
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("Failed to create runtime for sync notify");
                    rt.block_on(sync.notify_slot_changed(slot_number));
                });
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

pub fn handle_paste_from_slot(app: &AppHandle<Wry>, slot_number: u32) {
    clog!("handle_paste_from_slot: slot {}", slot_number);
    let db = app.state::<Arc<Database>>();

    // Read slot content from DB
    let slot_info = match db.get_slot(slot_number) {
        Ok(info) => {
            clog!("handle_paste_from_slot: got slot info, is_empty={}, has_content={}, name={}",
                info.is_empty, info.content.is_some(), info.name);
            info
        }
        Err(e) => {
            clog!("ERROR: handle_paste_from_slot: get_slot failed: {}", e);
            return;
        }
    };

    if slot_info.is_empty {
        clog!("handle_paste_from_slot: slot {} is empty", slot_number);
        let _ = app
            .notification()
            .builder()
            .title("ClipSlot")
            .body(&format!("{} is empty", slot_info.name))
            .show();
        return;
    }

    let slot_content = match slot_info.content {
        Some(c) => c,
        None => {
            clog!("ERROR: Slot {} content is None despite not being empty", slot_number);
            return;
        }
    };
    clog!(
        "Pasting from {} ({} chars)",
        slot_info.name,
        slot_content.len()
    );

    // 1. Pause clipboard monitoring
    if let Some(monitor) = app.try_state::<Arc<ClipboardMonitor>>() {
        monitor.pause();
    }

    // 2. Save current clipboard content
    let original_clipboard = app.clipboard().read_text().ok();

    // 3. Write slot content to system clipboard
    if let Err(e) = app.clipboard().write_text(&slot_content) {
        clog!("ERROR: Failed to write slot content to clipboard: {}", e);
        if let Some(monitor) = app.try_state::<Arc<ClipboardMonitor>>() {
            monitor.resume();
        }
        return;
    }

    // 4. Small delay for clipboard to propagate
    std::thread::sleep(Duration::from_millis(50));

    // 5. Simulate Cmd+V paste keystroke (uses CGEvent with explicit flags,
    //    so physical Cmd+Option being held won't interfere)
    clog!("Simulating Ctrl+V paste...");
    if let Err(e) = simulate_paste() {
        clog!("ERROR: Failed to simulate paste: {}", e);
    } else {
        clog!("Paste simulation sent");
    }

    // 6. Wait for the target app to process the paste
    std::thread::sleep(Duration::from_millis(200));

    // 7. Restore original clipboard content
    if let Some(original) = original_clipboard {
        let _ = app.clipboard().write_text(&original);
    }

    // 8. Resume clipboard monitoring
    if let Some(monitor) = app.try_state::<Arc<ClipboardMonitor>>() {
        monitor.resume();
    }

    clog!("Paste from {} complete", slot_info.name);
}

/// Simulate Cmd+V using CoreGraphics CGEvent with explicit flags.
/// This works even while physical modifier keys are held because we use
/// a private event source and set only the Command flag on the event.
#[cfg(target_os = "macos")]
fn simulate_paste() -> Result<(), String> {
    extern "C" {
        fn CGEventSourceCreate(state_id: i32) -> *mut std::ffi::c_void;
        fn CGEventCreateKeyboardEvent(
            source: *mut std::ffi::c_void,
            virtual_key: u16,
            key_down: bool,
        ) -> *mut std::ffi::c_void;
        fn CGEventSetFlags(event: *mut std::ffi::c_void, flags: u64);
        fn CGEventPost(tap_location: u32, event: *mut std::ffi::c_void);
        fn CFRelease(cf: *mut std::ffi::c_void);
    }

    unsafe {
        // kCGEventSourceStatePrivate = -1 (isolated from physical key state)
        let source = CGEventSourceCreate(-1);
        if source.is_null() {
            return Err("Failed to create CGEventSource".to_string());
        }

        // Virtual key code 9 = 'v' on macOS
        // kCGEventFlagMaskCommand = 0x00100000
        let cmd_flag: u64 = 0x00100000;

        // Key down
        let key_down = CGEventCreateKeyboardEvent(source, 9, true);
        if key_down.is_null() {
            CFRelease(source);
            return Err("Failed to create key down event".to_string());
        }
        CGEventSetFlags(key_down, cmd_flag);
        CGEventPost(0, key_down); // kCGHIDEventTap = 0
        CFRelease(key_down);

        std::thread::sleep(Duration::from_millis(10));

        // Key up
        let key_up = CGEventCreateKeyboardEvent(source, 9, false);
        if key_up.is_null() {
            CFRelease(source);
            return Err("Failed to create key up event".to_string());
        }
        CGEventSetFlags(key_up, cmd_flag);
        CGEventPost(0, key_up);
        CFRelease(key_up);

        CFRelease(source);
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn simulate_paste() -> Result<(), String> {
    // Windows INPUT struct layout on 64-bit:
    //   offset 0:  type (u32, 4 bytes)
    //   offset 4:  _align (u32, 4 bytes padding for union alignment)
    //   offset 8:  union start — KEYBDINPUT fields:
    //     offset 8:  wVk (u16)
    //     offset 10: wScan (u16)
    //     offset 12: dwFlags (u32)
    //     offset 16: time (u32)
    //     offset 20: [4 bytes padding]
    //     offset 24: dwExtraInfo (usize, 8 bytes)
    //     offset 32: [8 bytes padding to fill union to MOUSEINPUT size]
    //   offset 40: end — total sizeof(INPUT) = 40 bytes
    #[repr(C)]
    struct Input {
        type_: u32,
        _align: u32,       // padding between type and union
        vk: u16,
        scan: u16,
        flags: u32,
        time: u32,
        extra_info: usize,
        _union_pad: [u8; 8], // pad union to MOUSEINPUT size
    }

    extern "system" {
        fn SendInput(count: u32, inputs: *const Input, size: i32) -> u32;
    }

    const INPUT_KEYBOARD: u32 = 1;
    const KEYEVENTF_KEYUP: u32 = 0x0002;
    const VK_CONTROL: u16 = 0x11;
    const VK_V: u16 = 0x56;

    const VK_SHIFT: u16 = 0x10;
    const VK_MENU: u16 = 0x12; // Alt

    let size = std::mem::size_of::<Input>() as i32;

    let make = |vk: u16, flags: u32| -> Input {
        Input { type_: INPUT_KEYBOARD, _align: 0, vk, scan: 0, flags, time: 0, extra_info: 0, _union_pad: [0; 8] }
    };

    // Release physical Alt+Shift first (they're still held from the shortcut),
    // then send Ctrl+V, so the target app sees only Ctrl+V.
    let inputs = [
        make(VK_MENU, KEYEVENTF_KEYUP),     // Alt up
        make(VK_SHIFT, KEYEVENTF_KEYUP),     // Shift up
        make(VK_CONTROL, 0),                  // Ctrl down
        make(VK_V, 0),                        // V down
        make(VK_V, KEYEVENTF_KEYUP),          // V up
        make(VK_CONTROL, KEYEVENTF_KEYUP),    // Ctrl up
    ];

    let sent = unsafe {
        SendInput(6, inputs.as_ptr(), size)
    };

    if sent == 6 {
        Ok(())
    } else {
        Err(format!("SendInput returned {} (expected 6), size={}", sent, size))
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn simulate_paste() -> Result<(), String> {
    // Linux: xdotool or similar would be needed
    Ok(())
}

fn truncate(s: &str, max_len: usize) -> &str {
    match s.char_indices().nth(max_len) {
        Some((byte_idx, _)) => &s[..byte_idx],
        None => s,
    }
}
