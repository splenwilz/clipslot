# Phase 9: App Exclusion Rules

## Context

ClipSlot is a Tauri 2 clipboard manager with cross-device sync. This phase adds privacy controls that prevent ClipSlot from capturing clipboard content from sensitive applications (password managers, banking apps, etc.).

## Prerequisites

- Phases 0-8 completed: Full local + sync functionality

## Scope

- Detect the source application of clipboard events
- Maintain a configurable exclusion list
- Skip clipboard capture when the source app is excluded
- Include sensible defaults (common password managers, banking apps)
- "Private mode" toggle to pause all capture temporarily

## Technical Design

### Source App Detection

**macOS:**
- Use `NSWorkspace.shared.frontmostApplication` to detect the active app
- Bundle identifier (e.g., `com.1password.1password`) is the most reliable identifier
- Check the active app at the moment a clipboard change is detected

**Windows:**
- Use `GetForegroundWindow` + `GetWindowThreadProcessId` to get the active process
- Get the executable name from the process ID

### Default Exclusion List

```json
{
    "excluded_apps": [
        "com.1password.1password",
        "com.agilebits.onepassword7",
        "com.lastpass.LastPass",
        "com.bitwarden.desktop",
        "com.dashlane.Dashlane",
        "org.keepassxc.keepassxc",
        "com.apple.keychainaccess"
    ]
}
```

Windows equivalents:
```json
{
    "excluded_apps": [
        "1Password.exe",
        "LastPass.exe",
        "Bitwarden.exe",
        "Dashlane.exe",
        "KeePassXC.exe"
    ]
}
```

### Exclusion Check Flow

```
[Clipboard Change Detected]
        │
        ▼
[Get Active App ID]
        │
        ▼
[Is app in exclusion list?] ──Yes──→ [Skip capture, log "excluded"]
        │
        No
        │
        ▼
[Proceed with normal capture]
```

### Private Mode

- Toggle via keyboard shortcut (`Cmd+Option+P`) or tray menu
- When active:
  - All clipboard capture is paused
  - Tray icon changes to indicate private mode (e.g., muted/gray icon)
  - A subtle notification confirms "Private mode ON"
- Toggle off to resume

### Rust Module Updates

`src-tauri/src/clipboard/monitor.rs`:
- Add active app detection before capture
- Check exclusion list

New: `src-tauri/src/clipboard/exclusion.rs`:
- Exclusion list management
- Default exclusion list
- Active app detection (platform-specific)

### Tauri Commands (Updated)

```rust
#[tauri::command]
fn get_excluded_apps() -> Vec<AppExclusion>

#[tauri::command]
fn add_excluded_app(app_id: String, display_name: String) -> bool

#[tauri::command]
fn remove_excluded_app(app_id: String) -> bool

#[tauri::command]
fn is_private_mode() -> bool

#[tauri::command]
fn toggle_private_mode() -> bool  // Returns new state

#[tauri::command]
fn get_running_apps() -> Vec<AppInfo>
// Helper to let users pick from running apps to add to exclusion list
```

### AppExclusion Model

```rust
pub struct AppExclusion {
    pub app_id: String,         // Bundle ID (macOS) or exe name (Windows)
    pub display_name: String,   // Human-readable name
    pub is_default: bool,       // Whether it's a built-in default
}
```

### Settings UI Update (Phase 5 Privacy Tab)

- Show list of excluded apps with toggle switches
- "Add App" button that shows currently running apps to pick from
- Manual entry field for app bundle ID / exe name
- Default apps are shown but can be disabled
- Private mode toggle with current status

## Acceptance Criteria

- [ ] Clipboard content from excluded apps is NOT captured
- [ ] Default exclusion list includes common password managers
- [ ] Users can add/remove apps from the exclusion list in settings
- [ ] "Add App" picker shows currently running applications
- [ ] Private mode pauses ALL capture regardless of source app
- [ ] Private mode is toggleable via shortcut and tray menu
- [ ] Tray icon reflects private mode status
- [ ] Excluded captures are not synced (nothing leaves the device)
- [ ] Exclusion settings persist across restart
- [ ] Exclusion works on both macOS and Windows

## Manual Test Steps

1. Run `cargo tauri dev`
2. Open 1Password (or another excluded app)
3. Copy a password from 1Password
4. Check clipboard history — the password should NOT appear
5. Copy text from a non-excluded app (e.g., TextEdit)
6. Check clipboard history — the text should appear
7. Open Settings → Privacy → verify exclusion list shows defaults
8. Add a new app to the exclusion list (e.g., TextEdit)
9. Copy from TextEdit — should NOT be captured now
10. Remove TextEdit from exclusion list — capturing resumes
11. Toggle private mode via tray menu
12. Copy text from any app — should NOT be captured
13. Verify tray icon changed to indicate private mode
14. Toggle private mode off — capturing resumes
15. Test the `Cmd+Option+P` shortcut for private mode toggle

## Notes for Future Context

- Phase 10 adds final polish and packaging
- Private browser window detection is best-effort (may not be reliable on all browsers)
- The exclusion list should sync across devices (it's not sensitive data)
- Consider adding "auto-clear" option: automatically clear clipboard after pasting from an excluded app
