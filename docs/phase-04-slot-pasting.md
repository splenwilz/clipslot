# Phase 4: Slot Pasting via Keyboard Shortcuts

## Context

ClipSlot is a Tauri 2 clipboard manager. Phase 3 added permanent slots with save shortcuts. This phase adds the ability to PASTE directly from slots using keyboard shortcuts, without overwriting the current clipboard.

## Prerequisites

- Phase 0-3 completed: Tray app with clipboard monitoring, history, and slot saving

## Scope

- Global keyboard shortcuts to paste content from a specific slot
- Paste without overwriting the user's current clipboard content
- Handle empty slot gracefully
- Quick-paste popup (optional): show all slots for selection

## Technical Design

### Default Paste Shortcuts

| Action | macOS | Windows |
|---|---|---|
| Paste Slot 1 | `Cmd + Option + 1` | `Ctrl + Alt + 1` |
| Paste Slot 2 | `Cmd + Option + 2` | `Ctrl + Alt + 2` |
| Paste Slot 3 | `Cmd + Option + 3` | `Ctrl + Alt + 3` |
| Paste Slot 4 | `Cmd + Option + 4` | `Ctrl + Alt + 4` |
| Paste Slot 5 | `Cmd + Option + 5` | `Ctrl + Alt + 5` |
| Show Slot Picker | `Cmd + Option + V` | `Ctrl + Alt + V` |

### Paste Flow

1. User presses `Cmd+Option+1`
2. Global shortcut handler fires
3. Read Slot 1 content from database
4. If slot has content:
   a. Save current clipboard content to a temporary variable
   b. Write slot content to system clipboard
   c. Simulate `Cmd+V` paste keystroke
   d. After a short delay (~100ms), restore original clipboard content
5. If slot is empty:
   a. Show notification: "Slot 1 is empty"

### Simulating Paste

This is the trickiest part. Options:

**Option A: Write to clipboard + simulate Cmd+V (Recommended)**
- Use `enigo` crate or CGEvent (macOS) to simulate keystrokes
- Requires accessibility permissions on macOS
- Most reliable for cross-app pasting

**Option B: Write to clipboard only (Simpler fallback)**
- Just write slot content to clipboard
- User manually pastes with Cmd+V
- Less magical but avoids permission complexity

Implement Option A with Option B as fallback if permissions aren't granted.

### Slot Picker Popup

- Triggered by `Cmd+Option+V`
- Small floating window showing all 5 slots
- Each slot shows: number, name, content preview
- Press 1-5 to paste from that slot
- Escape to dismiss
- Window appears near cursor position
- Auto-dismisses after paste

### Rust Dependencies

- `enigo` — cross-platform keyboard/mouse simulation
- Or platform-specific: `core-graphics` (macOS), `winapi` (Windows)

### Clipboard Restoration

Critical: The paste operation temporarily modifies the clipboard. We must:
1. Pause clipboard monitoring during paste (to avoid capturing our own write)
2. Save current clipboard content
3. Write slot content
4. Simulate paste
5. Wait 100-200ms
6. Restore original clipboard content
7. Resume clipboard monitoring

### Rust Module Updates

`src-tauri/src/slots/shortcuts.rs`:
- Add paste shortcut registration
- Add paste handler logic

New: `src-tauri/src/slots/paste.rs`:
- Clipboard save/restore logic
- Keystroke simulation
- Slot picker window management

## Acceptance Criteria

- [ ] `Cmd+Option+1` pastes Slot 1 content into the active application
- [ ] All 5 paste shortcuts work correctly
- [ ] Current clipboard content is preserved after paste (not overwritten by slot content)
- [ ] Pasting from an empty slot shows a notification
- [ ] Slot paste works in: TextEdit, VS Code, browser text fields, Terminal
- [ ] `Cmd+Option+V` opens the slot picker popup
- [ ] Slot picker shows all 5 slots with previews
- [ ] Pressing a number in the picker pastes that slot
- [ ] Escape dismisses the picker
- [ ] Clipboard monitoring does not capture the temporary clipboard write during paste

## Manual Test Steps

1. Run `cargo tauri dev`
2. Save "AAA" to Slot 1 and "BBB" to Slot 2 (using Phase 3 shortcuts)
3. Copy "ORIGINAL" to clipboard
4. Open TextEdit, place cursor in a document
5. Press `Cmd+Option+1` — "AAA" should be pasted into TextEdit
6. Press `Cmd+V` (normal paste) — "ORIGINAL" should paste (clipboard was restored)
7. Press `Cmd+Option+2` — "BBB" should paste
8. Try pasting into VS Code, a browser form, and Terminal
9. Press the shortcut for an empty slot — notification should appear
10. Press `Cmd+Option+V` — slot picker should appear
11. Press `2` — "BBB" should paste and picker should close
12. Press `Cmd+Option+V` then Escape — picker should close without pasting
13. Check clipboard history — it should NOT contain the slot content as a new capture

## Notes for Future Context

- Accessibility permissions (macOS) must be granted for keystroke simulation
- Phase 5 will add a way to configure these shortcuts
- The slot picker UI will be refined in Phase 5
- Cross-device paste is covered in Phase 8 (sync)
