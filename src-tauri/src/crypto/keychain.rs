use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::RngCore;
use std::path::PathBuf;

const SERVICE: &str = "clipslot";
const USER: &str = "master-key";
const KEY_FILE_NAME: &str = ".master_key";

/// Set the app data directory so the key file fallback knows where to write.
/// Must be called before `get_or_create_master_key`.
static APP_DATA_DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

pub fn set_app_data_dir(dir: PathBuf) {
    let _ = APP_DATA_DIR.set(dir);
}

fn key_file_path() -> Option<PathBuf> {
    APP_DATA_DIR.get().map(|dir| dir.join(KEY_FILE_NAME))
}

/// Try to load the key from the file-based fallback.
fn load_from_file() -> Option<[u8; 32]> {
    let path = key_file_path()?;
    let encoded = std::fs::read_to_string(&path).ok()?;
    let bytes = BASE64.decode(encoded.trim()).ok()?;
    if bytes.len() != 32 {
        return None;
    }
    let mut key = [0u8; 32];
    key.copy_from_slice(&bytes);
    Some(key)
}

/// Save key to the file-based fallback (best-effort).
fn save_to_file(key: &[u8; 32]) {
    if let Some(path) = key_file_path() {
        let encoded = BASE64.encode(key);
        let _ = std::fs::write(&path, &encoded);
    }
}

/// Import a key received from link code — saves to both keychain and file.
pub fn import_master_key(key: &[u8; 32]) -> Result<(), String> {
    let encoded = BASE64.encode(key);

    // Best-effort keychain store
    if let Ok(entry) = keyring::Entry::new(SERVICE, USER) {
        let _ = entry.set_password(&encoded);
    }
    // Always save to file fallback
    save_to_file(key);

    println!("[ClipSlot] Master key imported and stored");
    Ok(())
}

/// Retrieve the master encryption key from the OS keychain,
/// falling back to a key file in the app data directory.
/// If neither exists, generate and store a new key in both.
pub fn get_or_create_master_key() -> Result<[u8; 32], String> {
    let entry =
        keyring::Entry::new(SERVICE, USER).map_err(|e| format!("Keyring entry error: {}", e))?;

    // Try keychain first
    match entry.get_password() {
        Ok(encoded) => {
            let bytes = BASE64
                .decode(&encoded)
                .map_err(|e| format!("Failed to decode key from keychain: {}", e))?;
            if bytes.len() != 32 {
                return Err(format!(
                    "Invalid key length in keychain: {} (expected 32)",
                    bytes.len()
                ));
            }
            let mut key = [0u8; 32];
            key.copy_from_slice(&bytes);
            // Keep file in sync
            save_to_file(&key);
            println!("[ClipSlot] Encryption key loaded from keychain");
            Ok(key)
        }
        Err(keyring::Error::NoEntry) | Err(_) => {
            // Keychain failed — try file-based fallback
            if let Some(key) = load_from_file() {
                // Restore to keychain (best-effort)
                let encoded = BASE64.encode(&key);
                let _ = entry.set_password(&encoded);
                println!("[ClipSlot] Encryption key loaded from file fallback");
                return Ok(key);
            }

            // No key anywhere — generate a new one
            let mut key = [0u8; 32];
            rand::thread_rng().fill_bytes(&mut key);

            let encoded = BASE64.encode(&key);
            let _ = entry.set_password(&encoded);
            save_to_file(&key);

            println!("[ClipSlot] New encryption key generated and stored");
            Ok(key)
        }
    }
}
