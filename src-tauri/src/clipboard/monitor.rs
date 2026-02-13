use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Runtime};
use tauri_plugin_clipboard_manager::ClipboardExt;

use super::item::ClipboardItem;

const POLL_INTERVAL_MS: u64 = 500;

pub struct ClipboardMonitor {
    paused: Arc<AtomicBool>,
}

impl ClipboardMonitor {
    pub fn new() -> Self {
        Self {
            paused: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }

    pub fn toggle_pause(&self) -> bool {
        let was_paused = self.paused.fetch_xor(true, Ordering::Relaxed);
        let now_paused = !was_paused;
        println!(
            "[ClipSlot] Monitoring {}",
            if now_paused { "PAUSED" } else { "RESUMED" }
        );
        now_paused
    }

    pub fn start<R: Runtime>(&self, app_handle: AppHandle<R>, device_id: String) {
        let paused = self.paused.clone();

        std::thread::spawn(move || {
            let mut last_hash: Option<String> = None;

            // Read initial clipboard content to avoid capturing pre-existing content
            if let Ok(text) = app_handle.clipboard().read_text() {
                if !text.is_empty() {
                    last_hash = Some(ClipboardItem::hash_content(&text));
                    println!("[ClipSlot] Monitor started (existing clipboard content ignored)");
                }
            } else {
                println!("[ClipSlot] Monitor started (clipboard empty)");
            }

            loop {
                std::thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));

                if paused.load(Ordering::Relaxed) {
                    continue;
                }

                let text = match app_handle.clipboard().read_text() {
                    Ok(t) => t,
                    Err(_) => continue,
                };

                if text.is_empty() {
                    continue;
                }

                let hash = ClipboardItem::hash_content(&text);

                if last_hash.as_ref() == Some(&hash) {
                    continue;
                }

                last_hash = Some(hash);

                let item = ClipboardItem::new(text, &device_id);

                println!(
                    "[ClipSlot] Captured: id={} hash={}.. len={} at={}",
                    item.id,
                    &item.content_hash[..12],
                    item.content.len(),
                    item.created_at
                );

                // Emit event to frontend (and for future internal listeners)
                let _ = app_handle.emit("clipboard-changed", &item);
            }
        });
    }
}
