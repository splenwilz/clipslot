# Phase 1: Clipboard Monitoring

## Context

ClipSlot is a Tauri 2 (Rust + React/TypeScript) clipboard manager. Phase 0 set up the project skeleton with a system tray app. This phase adds the core clipboard monitoring engine.

## Prerequisites

- Phase 0 completed: Tauri 2 project builds and runs as a tray app

## Scope

- Monitor the system clipboard for changes in the background
- Detect new clipboard content (plain text only for MVP)
- Emit clipboard events with metadata
- Log captured clipboard items to the console (storage comes in Phase 2)

## Technical Design

### Clipboard Polling

Tauri 2's `clipboard-manager` plugin provides read access. Since there's no native "clipboard changed" event on all platforms, we poll:

- Poll interval: **500ms** (configurable)
- Compare content hash to detect changes
- Only capture if content differs from last capture

### Clipboard Item Model

```rust
pub struct ClipboardItem {
    pub id: String,           // UUID v4
    pub content: String,      // The copied text
    pub content_hash: String, // SHA-256 hash for dedup
    pub content_type: String, // "text/plain" for MVP
    pub source_app: Option<String>, // Source app if detectable
    pub device_id: String,    // This device's ID
    pub created_at: i64,      // Unix timestamp ms
    pub is_promoted: bool,    // Whether saved to a slot
}
```

### Architecture

```
[OS Clipboard] → [Polling Loop (Rust)] → [ClipboardItem] → [Event Emitter]
                      ↓
              [Content Hash Check]
              (skip if unchanged)
```

### Rust Module: `src-tauri/src/clipboard/`

- `mod.rs` — module exports
- `monitor.rs` — polling loop, change detection
- `item.rs` — ClipboardItem struct and helpers

### Key Implementation Details

1. **Polling loop** runs in a background thread using `tokio::spawn` or `std::thread`
2. **Content hash** uses SHA-256 to compare clipboard content without storing duplicates
3. **Device ID** generated once on first launch, stored locally
4. **Event emission** — use Tauri's event system (`app_handle.emit(...)`) to notify the frontend
5. The monitor should be **startable/stoppable** (for the "Pause" feature later)

## Acceptance Criteria

- [ ] App starts and begins monitoring clipboard in the background
- [ ] Copying text anywhere on the system is detected within 1 second
- [ ] Duplicate consecutive copies of the same text are NOT captured twice
- [ ] Each captured item has: id, content, content_hash, timestamp, device_id
- [ ] Clipboard events are logged to the Tauri console / stdout
- [ ] Monitoring does not noticeably impact system performance
- [ ] Monitor can be paused and resumed programmatically

## Manual Test Steps

1. Run `cargo tauri dev`
2. Open any app (TextEdit, browser, terminal)
3. Copy some text (Cmd+C)
4. Check the Tauri dev console — a log entry should appear with the clipboard content and metadata
5. Copy the SAME text again — no new log entry should appear (dedup working)
6. Copy DIFFERENT text — a new log entry should appear
7. Copy text rapidly 5 times — all 5 unique items should be captured
8. Verify timestamps are accurate
9. Check Activity Monitor — the app should use minimal CPU (<2% idle, brief spikes on capture)

## Notes for Future Context

- Phase 2 will persist these items to SQLite
- Phase 3 will add the ability to promote items to permanent slots
- The `source_app` field is best-effort — may not be available on all platforms
- The polling interval may need tuning based on testing
