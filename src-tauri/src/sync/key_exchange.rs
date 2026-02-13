use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

use super::api_client::ApiClient;

/// Read the master key from the OS keychain and upload it to the server,
/// receiving a 6-digit link code in return.
pub async fn generate_link_code(api: &ApiClient, token: &str) -> Result<String, String> {
    let master_key = crate::crypto::keychain::get_or_create_master_key()?;
    let encoded = BASE64.encode(master_key);
    api.generate_link_code(token, &encoded).await
}

/// Redeem a 6-digit link code, receive the master key, and store it in the OS keychain.
/// After this, the app must be restarted to pick up the new key.
pub async fn redeem_link_code(api: &ApiClient, token: &str, code: &str) -> Result<(), String> {
    let encoded = api.redeem_link_code(token, code).await?;

    let key_bytes = BASE64
        .decode(&encoded)
        .map_err(|e| format!("Failed to decode key: {}", e))?;

    if key_bytes.len() != 32 {
        return Err(format!(
            "Invalid key length: {} (expected 32)",
            key_bytes.len()
        ));
    }

    // Store in OS keychain (overwrites existing key)
    let entry = keyring::Entry::new("clipslot", "master-key")
        .map_err(|e| format!("Keyring error: {}", e))?;
    entry
        .set_password(&encoded)
        .map_err(|e| format!("Failed to store key in keychain: {}", e))?;

    println!("[ClipSlot] Master key imported from link code — restart required");
    Ok(())
}
