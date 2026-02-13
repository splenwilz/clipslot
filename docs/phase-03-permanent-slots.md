# Phase 3: Permanent Slots & Keyboard Shortcuts (Save)

## Context

ClipSlot is a Tauri 2 clipboard manager. Phase 2 added local storage and clipboard history. This phase introduces the core differentiating feature: permanent slots that persist important clipboard items, accessible via global keyboard shortcuts.

## Prerequisites

- Phase 0: Project builds as a tray app
- Phase 1: Clipboard monitoring working
- Phase 2: Local storage and history working

## Scope

- Create 5 permanent slots for saving clipboard items
- Global keyboard shortcuts to SAVE current clipboard content to a slot
- Slots persist across app restarts and never auto-delete
- Slots are stored in the local database
- Slot metadata (name, position) is manageable
- Visual/audio feedback when a slot is saved

## Technical Design

### Database Schema Addition

```sql
CREATE TABLE slots (
    slot_number INTEGER PRIMARY KEY,  -- 1 through 5
    item_id TEXT REFERENCES clipboard_items(id),
    name TEXT,                         -- User-defined label
    updated_at INTEGER NOT NULL
);

-- Pre-populate 5 empty slots
INSERT INTO slots (slot_number, name, updated_at) VALUES (1, 'Slot 1', 0);
INSERT INTO slots (slot_number, name, updated_at) VALUES (2, 'Slot 2', 0);
INSERT INTO slots (slot_number, name, updated_at) VALUES (3, 'Slot 3', 0);
INSERT INTO slots (slot_number, name, updated_at) VALUES (4, 'Slot 4', 0);
INSERT INTO slots (slot_number, name, updated_at) VALUES (5, 'Slot 5', 0);
```

### Default Save Shortcuts

| Action | macOS | Windows |
|---|---|---|
| Save to Slot 1 | `Cmd + Shift + 1` | `Ctrl + Shift + 1` |
| Save to Slot 2 | `Cmd + Shift + 2` | `Ctrl + Shift + 2` |
| Save to Slot 3 | `Cmd + Shift + 3` | `Ctrl + Shift + 3` |
| Save to Slot 4 | `Cmd + Shift + 4` | `Ctrl + Shift + 4` |
| Save to Slot 5 | `Cmd + Shift + 5` | `Ctrl + Shift + 5` |

**Note:** `Cmd+Shift+3/4/5` conflict with macOS screenshot shortcuts. We may need alternative defaults like `Cmd+Ctrl+1-5` or make them fully configurable. Test during implementation and adjust.

### Rust Module: `src-tauri/src/slots/`

- `mod.rs` — module exports
- `manager.rs` — slot CRUD operations
- `shortcuts.rs` — global shortcut registration for save actions

### Tauri Commands

```rust
#[tauri::command]
fn save_to_slot(slot_number: u32) -> Result<SlotInfo, String>
// Takes current clipboard content and saves it to the specified slot

#[tauri::command]
fn get_slot(slot_number: u32) -> Option<SlotInfo>

#[tauri::command]
fn get_all_slots() -> Vec<SlotInfo>

#[tauri::command]
fn clear_slot(slot_number: u32) -> bool

#[tauri::command]
fn rename_slot(slot_number: u32, name: String) -> bool
```

### SlotInfo Model

```rust
pub struct SlotInfo {
    pub slot_number: u32,
    pub name: String,
    pub content: Option<String>,       // The saved text
    pub content_preview: Option<String>, // First 100 chars
    pub updated_at: i64,
    pub is_empty: bool,
}
```

### Save Flow

1. User presses `Cmd+Shift+1`
2. Global shortcut handler fires
3. Read current clipboard content
4. If clipboard has content:
   a. Find or create a `ClipboardItem` for this content
   b. Mark the item as `is_promoted = true`
   c. Update slot 1 to point to this item
   d. Show system notification: "Saved to Slot 1"
5. If clipboard is empty:
   a. Show notification: "Clipboard is empty"

### Feedback

- Use macOS native notifications (`tauri-plugin-notification`) for save confirmation
- Notification should be brief: "Saved to Slot 1: [first 50 chars...]"

## Acceptance Criteria

- [ ] 5 permanent slots exist in the database
- [ ] Global shortcut `Cmd+Ctrl+1` (or chosen alternative) saves clipboard to Slot 1
- [ ] All 5 save shortcuts work correctly
- [ ] Saving shows a system notification with slot name and content preview
- [ ] Saved slot content persists across app restart
- [ ] Saving to an occupied slot overwrites it (previous content remains in history)
- [ ] Promoted items are NOT auto-deleted by history expiry
- [ ] `get_all_slots` returns all 5 slots with their current content
- [ ] Slots can be renamed
- [ ] Shortcuts do not conflict with common OS shortcuts

## Manual Test Steps

1. Run `cargo tauri dev`
2. Copy some text (e.g., "Hello World")
3. Press the Slot 1 save shortcut — a notification should confirm the save
4. Copy different text (e.g., "Test 123")
5. Press the Slot 2 save shortcut — notification confirms
6. Quit and restart the app
7. Check that Slot 1 still contains "Hello World" and Slot 2 contains "Test 123"
8. Save new content to Slot 1 — it should overwrite
9. Verify the old content ("Hello World") is still in clipboard history
10. Try saving with an empty clipboard — should show "Clipboard is empty" notification
11. Test all 5 slot shortcuts

## Notes for Future Context

- Phase 4 will add PASTE shortcuts (reading from slots)
- Phase 5 will show slots in the tray UI with management options
- Phase 8 will sync slots across devices
- Shortcut configuration will be added in Phase 5 (settings UI)
