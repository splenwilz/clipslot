# Phase 2: Local Storage & Clipboard History

## Context

ClipSlot is a Tauri 2 clipboard manager. Phase 1 added clipboard monitoring that captures items and logs them. This phase persists captured items to a local SQLite database and exposes clipboard history to the frontend.

## Prerequisites

- Phase 0 completed: Project builds as a tray app
- Phase 1 completed: Clipboard monitoring captures items and emits events

## Scope

- Create a local SQLite database for clipboard history
- Persist every captured clipboard item
- Implement configurable history limit (default: 500 items)
- Auto-expire old items (FIFO, oldest non-promoted items deleted first)
- Expose history to the frontend via Tauri commands
- Implement search across clipboard history
- Display clipboard history in a basic popup window

## Technical Design

### Database Schema

```sql
CREATE TABLE clipboard_items (
    id TEXT PRIMARY KEY,
    content TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    content_type TEXT NOT NULL DEFAULT 'text/plain',
    source_app TEXT,
    device_id TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    is_promoted INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_created_at ON clipboard_items(created_at DESC);
CREATE INDEX idx_content_hash ON clipboard_items(content_hash);
CREATE INDEX idx_is_promoted ON clipboard_items(is_promoted);

CREATE TABLE app_config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
```

### Rust Module: `src-tauri/src/storage/`

- `mod.rs` — module exports
- `database.rs` — SQLite connection, migrations, CRUD operations
- `history.rs` — history management (limit enforcement, expiry, search)

### Tauri Commands (Rust → Frontend)

```rust
#[tauri::command]
fn get_clipboard_history(limit: u32, offset: u32) -> Vec<ClipboardItem>

#[tauri::command]
fn search_history(query: String) -> Vec<ClipboardItem>

#[tauri::command]
fn delete_history_item(id: String) -> bool

#[tauri::command]
fn clear_history() -> bool

#[tauri::command]
fn get_history_count() -> u32
```

### History Management

- **Max items:** 500 (configurable via `app_config` table)
- **Auto-expiry:** After each insert, if count > max, delete oldest non-promoted items
- **Search:** SQLite `LIKE` query on `content` field (full-text search is post-MVP)

### Frontend: History Window

- A minimal popup window triggered from the tray menu ("Show History")
- Displays a scrollable list of clipboard items (newest first)
- Each item shows: truncated content (first 100 chars), timestamp, source app
- Search bar at the top for filtering
- Click an item to copy it back to clipboard
- Basic styling — functional, not polished (Phase 5 will refine UI)

### Database Location

- macOS: `~/Library/Application Support/com.clipslot.app/clipslot.db`
- Windows: `%APPDATA%/com.clipslot.app/clipslot.db`
- Use Tauri's `app_data_dir()` to resolve the path

## Acceptance Criteria

- [ ] SQLite database is created on first launch
- [ ] Every captured clipboard item is persisted to the database
- [ ] Clipboard history survives app restart
- [ ] History is capped at 500 items (oldest non-promoted items removed)
- [ ] Search returns matching items by content substring
- [ ] Tray menu has "Show History" option that opens the history window
- [ ] History window displays items in reverse chronological order
- [ ] Clicking an item in the history copies it to the clipboard
- [ ] "Clear History" removes all non-promoted items
- [ ] Database file is in the correct platform-specific location

## Manual Test Steps

1. Run `cargo tauri dev`
2. Copy 5 different pieces of text from various apps
3. Click tray icon → "Show History" — all 5 items should appear
4. Verify items show content preview, timestamp
5. Click an item — it should be copied to clipboard (paste to verify)
6. Type in the search bar — list should filter in real-time
7. Quit and restart the app — history should persist
8. Verify the database file exists at the expected location
9. Copy many items (50+) to verify performance stays snappy
10. Test "Clear History" — all items should be removed

## Notes for Future Context

- Phase 3 will add `is_promoted` functionality (permanent slots)
- Phase 6 will encrypt the database contents
- The history window UI will be refined in Phase 5 (system tray UI)
- Pagination (`limit`/`offset`) is included for future large history support
