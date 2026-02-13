use std::sync::Arc;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

use crate::storage::database::Database;

use super::api_client::ApiClient;
use super::types::PushHistoryRequest;

/// Perform initial history sync between local and remote.
/// Pulls remote items missing locally, pushes local items missing remotely.
/// Returns (pulled, pushed) counts.
pub async fn perform_initial_history_sync(
    api: &ApiClient,
    token: &str,
    db: &Arc<Database>,
    device_id: &str,
) -> Result<(u32, u32), String> {
    let mut pulled = 0u32;
    let mut pushed = 0u32;

    // Pull remote history (first 200 items)
    let remote_items = api.get_history(token, 200, 0).await?;

    for item in &remote_items {
        // Check if we already have this item locally (by content_hash)
        let exists = db
            .has_item_with_hash(&item.content_hash)
            .map_err(|e| format!("DB error: {}", e))?;

        if !exists {
            // Decode base64 blob to get the encrypted string
            let blob_bytes = BASE64
                .decode(&item.encrypted_blob)
                .map_err(|e| format!("Base64 decode error: {}", e))?;
            let enc_str = String::from_utf8(blob_bytes)
                .map_err(|e| format!("UTF-8 error: {}", e))?;

            let created_at = parse_timestamp(&item.created_at);

            db.insert_synced_item(
                &item.id.to_string(),
                &enc_str,
                &item.content_hash,
                device_id,
                created_at,
            )
            .map_err(|e| format!("DB error: {}", e))?;
            pulled += 1;
        }
    }

    // Push local history items that the server might not have
    // We collect content hashes from remote for quick lookup
    let remote_hashes: std::collections::HashSet<&str> =
        remote_items.iter().map(|r| r.content_hash.as_str()).collect();

    let local_items = db
        .get_unpromoted_encrypted_items(200)
        .map_err(|e| format!("DB error: {}", e))?;

    for (id, encrypted, content_hash) in &local_items {
        if remote_hashes.contains(content_hash.as_str()) {
            continue;
        }

        // Base64-encode the encrypted content for the server
        let blob = BASE64.encode(encrypted.as_bytes());

        let req = PushHistoryRequest {
            id: uuid::Uuid::parse_str(id).unwrap_or_else(|_| uuid::Uuid::new_v4()),
            encrypted_blob: blob,
            content_hash: content_hash.clone(),
        };

        if let Err(e) = api.push_history(token, &req).await {
            eprintln!("[ClipSlot] Failed to push history item {}: {}", id, e);
        } else {
            pushed += 1;
        }
    }

    println!(
        "[ClipSlot] History sync: pulled {}, pushed {}",
        pulled, pushed
    );
    Ok((pulled, pushed))
}

/// Parse an ISO 8601 timestamp string to epoch millis, falling back to 0.
fn parse_timestamp(ts: &str) -> i64 {
    chrono::DateTime::parse_from_rfc3339(ts)
        .map(|dt| dt.timestamp_millis())
        .unwrap_or(0)
}
