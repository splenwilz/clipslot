# Phase 8: Cross-Device Sync (Client Integration)

## Context

ClipSlot is a Tauri 2 clipboard manager. Phase 7 built the backend sync service. This phase integrates the client with the sync service to enable cross-device clipboard sync.

## Prerequisites

- Phases 0-6: Fully functional local client with encryption
- Phase 7: Backend sync service running and tested

## Scope

- User account creation and login from the client
- Device registration
- Sync permanent slots across devices in near real-time
- Sync clipboard history (opt-in)
- Offline support with automatic reconnection
- Conflict resolution on reconnect
- Key exchange for E2EE across devices

## Technical Design

### Sync Flow

```
[App Launch]
    │
    ├── Is user logged in? ──No──→ [Local-only mode]
    │
    Yes
    │
    ├── Connect WebSocket to sync service
    │
    ├── Initial sync:
    │   ├── Pull all remote slots → merge with local
    │   └── Pull remote history → merge with local
    │
    ├── Ongoing:
    │   ├── Local slot change → encrypt → push via WebSocket
    │   ├── Remote slot change received → decrypt → update local
    │   ├── New local clipboard item → encrypt → push (if history sync enabled)
    │   └── New remote clipboard item → decrypt → add to local history
    │
    └── Offline:
        ├── Queue changes locally
        └── Replay queue on reconnect
```

### Key Exchange Flow

For E2EE to work across devices, both devices need the same master key:

1. **Device A (existing):** User goes to Settings → "Link New Device"
2. App shows a one-time code (6 digits) and derives a temporary key from it
3. Master key is encrypted with the temporary key and uploaded to the server
4. **Device B (new):** User enters the 6-digit code during setup
5. Device B downloads the encrypted master key, derives the temporary key from the code, decrypts the master key
6. Server deletes the encrypted key blob after retrieval
7. Both devices now share the same master key

Alternative: User enters their passphrase on Device B → Argon2id derives the same master key.

### Rust Module: `src-tauri/src/sync/`

- `mod.rs` — module exports
- `client.rs` — WebSocket client, connection management
- `auth.rs` — login, registration, token management
- `merge.rs` — conflict resolution, merge logic
- `queue.rs` — offline change queue
- `key_exchange.rs` — cross-device key sharing

### Sync State Machine

```
[Disconnected] ──connect──→ [Connecting] ──success──→ [Syncing]
       ↑                         │                        │
       │                      failure                  disconnect
       │                         │                        │
       └────────────────────────←┘←───────────────────────┘
                    (auto-retry with backoff)
```

### Offline Queue

Changes made while offline are queued:

```rust
pub struct SyncQueueItem {
    pub id: String,
    pub action: SyncAction,    // SlotUpdate, HistoryPush, HistoryDelete
    pub payload: Vec<u8>,      // Encrypted blob
    pub created_at: i64,
    pub retries: u32,
}
```

Queue is stored in SQLite. On reconnect, items are replayed in order.

### Conflict Resolution

**Slots (last-write-wins):**
1. Compare `updated_at` timestamps
2. Most recent wins
3. Loser is preserved in local history (not lost)

**History (merge):**
1. Compare by `content_hash`
2. If same hash exists, skip (dedup)
3. Otherwise, add to local history

### Tauri Commands (New)

```rust
#[tauri::command]
fn login(email: String, password: String) -> Result<UserInfo, String>

#[tauri::command]
fn register(email: String, password: String) -> Result<UserInfo, String>

#[tauri::command]
fn logout() -> bool

#[tauri::command]
fn get_sync_status() -> SyncStatus  // Connected, Disconnected, Syncing

#[tauri::command]
fn get_linked_devices() -> Vec<DeviceInfo>

#[tauri::command]
fn generate_link_code() -> String  // 6-digit code

#[tauri::command]
fn enter_link_code(code: String) -> Result<(), String>

#[tauri::command]
fn toggle_history_sync(enabled: bool) -> bool

#[tauri::command]
fn force_sync() -> Result<(), String>
```

### Frontend: Sync UI

- **Login/Register screen** — shown in settings or on first-time sync setup
- **Sync status indicator** in tray menu (green dot = connected, yellow = syncing, red = disconnected)
- **Linked devices** list in settings
- **Link device** flow (show code / enter code)
- **History sync toggle** in settings

## Acceptance Criteria

- [ ] User can register and login from the client
- [ ] Device is automatically registered on first login
- [ ] Slots sync between two devices within <300ms
- [ ] Saving a slot on Device A updates it on Device B
- [ ] History sync works when enabled (opt-in)
- [ ] New clipboard item on Device A appears in Device B's history
- [ ] Offline changes are queued and synced on reconnect
- [ ] Conflict resolution works (last-write-wins for slots)
- [ ] Key exchange works via 6-digit code
- [ ] Sync status is visible in the tray menu
- [ ] Logout disconnects sync and clears tokens
- [ ] Server cannot read any synced content (E2EE verified)

## Manual Test Steps

1. Start the sync server (Phase 7)
2. Run the client on Device A (`cargo tauri dev`)
3. Open Settings → Account → Register a new account
4. Verify device is registered (check server logs or API)
5. Run the client on Device B
6. Login with the same account on Device B
7. On Device A: Settings → Link Device → note the 6-digit code
8. On Device B: Settings → Enter Link Code → enter the code
9. On Device A: save "Test Sync" to Slot 1
10. On Device B: verify Slot 1 now contains "Test Sync" (should appear within 1 second)
11. On Device B: save "Reply Sync" to Slot 2
12. On Device A: verify Slot 2 updated
13. Disconnect Device B's network
14. On Device B: save "Offline Content" to Slot 3
15. Reconnect Device B's network
16. On Device A: verify Slot 3 updated after reconnect
17. Enable history sync on both devices
18. Copy text on Device A → verify it appears in Device B's history
19. Check the server database — all content should be encrypted blobs

## Notes for Future Context

- Phase 9 adds app exclusion enforcement
- Phase 10 adds polish and packaging
- Rate limiting on the server should prevent abuse
- The 6-digit link code should expire after 5 minutes
- Consider adding a "sync now" button for manual sync trigger
