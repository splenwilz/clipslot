use std::collections::VecDeque;
use std::sync::Mutex;

use super::types::WsMessage;

/// In-memory queue for messages that couldn't be sent while offline.
/// Deduplicates slot updates by keeping only the latest per slot_number.
pub struct OfflineQueue {
    queue: Mutex<VecDeque<WsMessage>>,
}

impl OfflineQueue {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
        }
    }

    /// Enqueue a message. For SlotUpdate messages, replaces any existing
    /// entry for the same slot_number (keeping only the latest).
    pub fn enqueue(&self, msg: WsMessage) {
        let mut q = self.queue.lock().unwrap();

        // Dedup slot updates — remove older entry for the same slot
        if let WsMessage::SlotUpdate { slot_number, .. } = &msg {
            q.retain(|existing| {
                !matches!(existing, WsMessage::SlotUpdate { slot_number: n, .. } if n == slot_number)
            });
        }

        q.push_back(msg);
    }

    /// Drain all queued messages for sending.
    pub fn drain(&self) -> Vec<WsMessage> {
        let mut q = self.queue.lock().unwrap();
        q.drain(..).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.lock().unwrap().is_empty()
    }
}
