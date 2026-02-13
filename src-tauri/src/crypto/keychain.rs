use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::RngCore;

const SERVICE: &str = "clipslot";
const USER: &str = "master-key";

/// Retrieve the master encryption key from the OS keychain,
/// or generate and store a new one if none exists.
pub fn get_or_create_master_key() -> Result<[u8; 32], String> {
    let entry =
        keyring::Entry::new(SERVICE, USER).map_err(|e| format!("Keyring entry error: {}", e))?;

    // Try to load existing key
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
            println!("[ClipSlot] Encryption key loaded from keychain");
            Ok(key)
        }
        Err(keyring::Error::NoEntry) => {
            // Generate a new random key
            let mut key = [0u8; 32];
            rand::thread_rng().fill_bytes(&mut key);

            let encoded = BASE64.encode(&key);
            entry
                .set_password(&encoded)
                .map_err(|e| format!("Failed to store key in keychain: {}", e))?;

            println!("[ClipSlot] New encryption key generated and stored in keychain");
            Ok(key)
        }
        Err(e) => Err(format!("Failed to access keychain: {}", e)),
    }
}
