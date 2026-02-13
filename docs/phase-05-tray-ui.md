# Phase 5: System Tray UI & Settings

## Context

ClipSlot is a Tauri 2 clipboard manager. Phases 1-4 built the core engine: clipboard monitoring, history, slots, and pasting. This phase builds the user-facing tray UI and settings interface.

## Prerequisites

- Phases 0-4 completed: Full clipboard engine with monitoring, history, slots, save/paste shortcuts

## Scope

- Refined system tray menu with live slot previews
- History popup window (improved from Phase 2's basic version)
- Settings window for configuration
- Shortcut customization
- Slot management UI (rename, clear, reorder)
- Pause/Resume toggle
- Visual polish

## Technical Design

### Tray Menu Structure

```
ClipSlot (tray icon)
├── [Slot 1: "Hello Wor..."]     ← Click to paste
├── [Slot 2: "https://e..."]     ← Click to paste
├── [Slot 3: (empty)]
├── [Slot 4: (empty)]
├── [Slot 5: (empty)]
├── ─────────────────
├── Show History          Cmd+Option+H
├── ─────────────────
├── ⏸ Pause Monitoring   Cmd+Option+P
├── Settings...
├── ─────────────────
├── About ClipSlot
└── Quit                  Cmd+Q
```

### History Window (Refined)

- Floating window, appears centered on screen
- Invoked via tray menu or `Cmd+Option+H`
- Features:
  - Search bar (auto-focused on open)
  - Scrollable list of clipboard items
  - Each item: content preview, timestamp, source app badge
  - Click to copy, double-click to paste
  - Right-click context menu: Copy, Save to Slot, Delete
  - Keyboard navigation: arrow keys, Enter to select, Escape to close
  - Promoted items shown with a pin/star icon
- Styling: clean, minimal, dark/light mode following system preference

### Settings Window

- Standard window with tabbed interface
- Tabs:
  1. **General**
     - History limit (default: 500)
     - Auto-start on login (toggle)
     - Launch minimized (toggle)
  2. **Shortcuts**
     - Save shortcuts (Slot 1-5)
     - Paste shortcuts (Slot 1-5)
     - Show History shortcut
     - Show Slot Picker shortcut
     - Pause/Resume shortcut
     - Each shortcut has a "Record" button to capture new key combo
  3. **Privacy**
     - App exclusion list (add/remove apps)
     - Auto-clear history on quit (toggle)
  4. **Slots**
     - List of all 5 slots
     - Rename each slot
     - Clear individual slots
     - Preview slot content

### Frontend Components

```
src/
├── components/
│   ├── HistoryWindow/
│   │   ├── HistoryWindow.tsx
│   │   ├── HistoryItem.tsx
│   │   ├── SearchBar.tsx
│   │   └── history.css
│   ├── Settings/
│   │   ├── SettingsWindow.tsx
│   │   ├── GeneralTab.tsx
│   │   ├── ShortcutsTab.tsx
│   │   ├── PrivacyTab.tsx
│   │   ├── SlotsTab.tsx
│   │   └── settings.css
│   ├── SlotPicker/
│   │   ├── SlotPicker.tsx
│   │   └── slotpicker.css
│   └── shared/
│       ├── Notification.tsx
│       └── Badge.tsx
├── hooks/
│   ├── useClipboardHistory.ts
│   ├── useSlots.ts
│   └── useSettings.ts
├── App.tsx
└── main.tsx
```

### Settings Storage

- Settings stored in `app_config` table (from Phase 2)
- Keys: `history_limit`, `auto_start`, `launch_minimized`, `shortcuts_config`, `excluded_apps`, `auto_clear_on_quit`
- Shortcuts config is JSON: `{ "save_slot_1": "CmdOrCtrl+Shift+1", ... }`

### Tauri Commands (New)

```rust
#[tauri::command]
fn get_settings() -> AppSettings

#[tauri::command]
fn update_setting(key: String, value: String) -> bool

#[tauri::command]
fn get_excluded_apps() -> Vec<String>

#[tauri::command]
fn add_excluded_app(app_id: String) -> bool

#[tauri::command]
fn remove_excluded_app(app_id: String) -> bool

#[tauri::command]
fn toggle_monitoring() -> bool  // Returns new state

#[tauri::command]
fn update_shortcut(action: String, shortcut: String) -> Result<(), String>
```

## Acceptance Criteria

- [ ] Tray menu shows all 5 slots with content previews
- [ ] Clicking a slot in the tray pastes its content
- [ ] "Show History" opens the history window
- [ ] History window supports search, keyboard navigation, click-to-copy
- [ ] Right-click on history item shows context menu with "Save to Slot"
- [ ] Settings window opens with all 4 tabs functional
- [ ] Shortcuts can be customized and changes take effect immediately
- [ ] Pause/Resume toggle works from tray and shortcut
- [ ] App exclusion list can be edited in settings
- [ ] Settings persist across restart
- [ ] UI respects system dark/light mode

## Manual Test Steps

1. Run `cargo tauri dev`
2. Click tray icon — menu should show all 5 slots (some empty, some filled if saved in Phase 3)
3. Click a filled slot — content should paste into active app
4. Click "Show History" — history window should open
5. Type in search bar — list should filter
6. Use arrow keys + Enter to select an item — content should be copied
7. Right-click an item → "Save to Slot 1" — item should be saved
8. Press Escape — history window should close
9. Open Settings → Shortcuts tab → change Slot 1 save shortcut
10. Verify the new shortcut works
11. Settings → General → change history limit → verify it takes effect
12. Toggle Pause from tray — monitoring should stop (copy text, check no new history items)
13. Toggle Resume — monitoring resumes
14. Switch between light and dark mode — UI should adapt

## Notes for Future Context

- This phase creates the main user-facing experience
- Phase 6 adds encryption (stored data will be encrypted)
- Phase 9 adds app exclusion enforcement
- The slot picker (from Phase 4) should also use the refined UI styles from this phase
