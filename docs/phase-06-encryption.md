# Phase 6: Local Encryption (E2EE Foundation)

## Context

ClipSlot is a Tauri 2 clipboard manager. Phases 0-5 built the full local experience: monitoring, history, slots, pasting, and UI. This phase adds end-to-end encryption for all stored clipboard data, forming the foundation for secure cross-device sync.

## Prerequisites

- Phases 0-5 completed: Fully functional local clipboard manager with UI

## Scope

- Encrypt all clipboard content stored in SQLite
- Generate and manage encryption keys locally
- Key derivation from user passphrase (for cross-device key sharing)
- Encrypt data at rest — database content is unreadable without the key
- Decrypt on-the-fly when displaying content
- Key never leaves the device unencrypted

## Technical Design

### Encryption Scheme

- **Algorithm:** AES-256-GCM (authenticated encryption)
- **Key Derivation:** Argon2id (passphrase → encryption key)
- **Key Storage:** OS keychain (macOS Keychain, Windows Credential Manager)
- **Per-item encryption:** Each clipboard item's `content` field is individually encrypted
- **Nonce:** Random 96-bit nonce per encryption operation, stored alongside ciphertext

### Key Management

```
User Passphrase (optional)
        │
        ▼
  Argon2id KDF ──→ Master Key (256-bit)
        │
        ▼
  Stored in OS Keychain
        │
        ▼
  Used for AES-256-GCM encrypt/decrypt
```

**First launch flow:**
1. Generate a random 256-bit master key
2. Store in OS keychain
3. All encryption uses this key

**Cross-device flow (Phase 8):**
1. User sets a passphrase
2. Derive key from passphrase using Argon2id
3. This derived key encrypts the master key for transport
4. On new device: enter passphrase → derive key → decrypt master key

### Rust Module: `src-tauri/src/crypto/`

- `mod.rs` — module exports
- `keys.rs` — key generation, derivation, keychain storage
- `cipher.rs` — encrypt/decrypt functions
- `keychain.rs` — OS keychain integration (macOS Keychain API, Windows DPAPI)

### Rust Dependencies

- `aes-gcm` — AES-256-GCM encryption
- `argon2` — Argon2id key derivation
- `rand` — cryptographic random number generation
- `keyring` — cross-platform OS keychain access

### Encrypted Storage Format

```rust
pub struct EncryptedContent {
    pub nonce: [u8; 12],    // 96-bit random nonce
    pub ciphertext: Vec<u8>, // AES-256-GCM ciphertext + auth tag
}
// Serialized as base64 string for SQLite storage
```

### Database Changes

- `clipboard_items.content` now stores base64-encoded `EncryptedContent`
- `content_hash` remains plaintext (SHA-256 of original content, used for dedup)
- Add migration to encrypt existing unencrypted content

### Performance Considerations

- AES-256-GCM is hardware-accelerated on modern CPUs (AES-NI)
- Encryption/decryption per item should be <1ms
- Decrypt only when displaying content (lazy decryption)
- Cache decrypted content in memory for the current session (clear on lock/quit)

### Tauri Commands

```rust
#[tauri::command]
fn set_encryption_passphrase(passphrase: String) -> Result<(), String>

#[tauri::command]
fn verify_passphrase(passphrase: String) -> bool

#[tauri::command]
fn is_encryption_enabled() -> bool

#[tauri::command]
fn export_encrypted_key(passphrase: String) -> Result<String, String>
// For cross-device key transfer (Phase 8)
```

## Acceptance Criteria

- [ ] Master encryption key is generated on first launch
- [ ] Key is stored in the OS keychain (not in a file)
- [ ] All new clipboard items are encrypted before storage
- [ ] Existing items are migrated to encrypted format on upgrade
- [ ] Decryption is transparent — history and slots display plaintext as before
- [ ] Database file is unreadable without the key (verify with `sqlite3` CLI)
- [ ] Encryption does not noticeably impact performance (<1ms per item)
- [ ] User can set a passphrase for cross-device key derivation
- [ ] App data is secure if the database file is copied without keychain access
- [ ] Key is cleared from memory on app quit

## Manual Test Steps

1. Run `cargo tauri dev`
2. Copy some text — verify it appears normally in history (encryption is transparent)
3. Open the SQLite database directly with `sqlite3` CLI
4. Query `SELECT content FROM clipboard_items` — content should be base64 gibberish, NOT plaintext
5. Verify `content_hash` is still readable (used for dedup, not sensitive)
6. Restart the app — history should still be readable (key retrieved from keychain)
7. Check macOS Keychain Access app — a "ClipSlot" entry should exist
8. Open Settings → set a passphrase
9. Verify passphrase verification works (enter correct/incorrect)
10. Performance check: copy text rapidly — no noticeable delay compared to pre-encryption

## Notes for Future Context

- Phase 7 builds the backend sync service
- Phase 8 uses the exported encrypted key for cross-device sync
- The passphrase is optional for local-only use (key is in keychain)
- The passphrase is required for cross-device sync (key derivation)
- Never log or display the encryption key
