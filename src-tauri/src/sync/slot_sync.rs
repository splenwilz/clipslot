use std::sync::Arc;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

use crate::storage::database::Database;

use super::api_client::ApiClient;

/// Perform a full slot sync between local and remote.
/// Returns the number of slots synced.
pub async fn perform_full_slot_sync(
    api: &ApiClient,
    token: &str,
    db: &Arc<Database>,
    device_id: &str,
) -> Result<u32, String> {
    let remote_slots = api.get_slots(token).await?;
    let mut synced = 0u32;

    for slot_num in 1..=10 {
        let (local_encrypted, local_updated_at) = db
            .get_slot_raw(slot_num)
            .map_err(|e| format!("DB error: {}", e))?;

        // Find the matching remote slot
        let remote = remote_slots
            .iter()
            .find(|s| s.slot_number == slot_num as i32);

        match (local_encrypted.as_deref(), remote) {
            // Both exist — compare timestamps
            (Some(local_enc), Some(remote_slot)) => {
                let remote_ts = parse_timestamp(&remote_slot.updated_at);

                if remote_ts > local_updated_at {
                    // Remote is newer — pull
                    let blob_bytes = BASE64
                        .decode(&remote_slot.encrypted_blob)
                        .map_err(|e| format!("Base64 decode error: {}", e))?;
                    let enc_str = String::from_utf8(blob_bytes)
                        .map_err(|e| format!("UTF-8 error: {}", e))?;

                    db.save_encrypted_to_slot(slot_num, &enc_str, remote_ts, device_id)
                        .map_err(|e| format!("DB error: {}", e))?;
                    synced += 1;
                    println!(
                        "[ClipSlot] Slot {} pulled from server (remote newer)",
                        slot_num
                    );
                } else if local_updated_at > remote_ts {
                    // Local is newer — push
                    let blob = BASE64.encode(local_enc.as_bytes());
                    api.update_slot(token, slot_num as i32, &blob).await?;
                    synced += 1;
                    println!(
                        "[ClipSlot] Slot {} pushed to server (local newer)",
                        slot_num
                    );
                }
                // Equal timestamps — skip
            }

            // Only local exists — push to server
            (Some(local_enc), None) => {
                let blob = BASE64.encode(local_enc.as_bytes());
                api.update_slot(token, slot_num as i32, &blob).await?;
                synced += 1;
                println!("[ClipSlot] Slot {} pushed to server (new)", slot_num);
            }

            // Only remote exists — pull to local
            (None, Some(remote_slot)) => {
                let blob_bytes = BASE64
                    .decode(&remote_slot.encrypted_blob)
                    .map_err(|e| format!("Base64 decode error: {}", e))?;
                let enc_str = String::from_utf8(blob_bytes)
                    .map_err(|e| format!("UTF-8 error: {}", e))?;

                let remote_ts = parse_timestamp(&remote_slot.updated_at);
                db.save_encrypted_to_slot(slot_num, &enc_str, remote_ts, device_id)
                    .map_err(|e| format!("DB error: {}", e))?;
                synced += 1;
                println!("[ClipSlot] Slot {} pulled from server (new)", slot_num);
            }

            // Neither exists — nothing to do
            (None, None) => {}
        }
    }

    Ok(synced)
}

/// Parse an ISO 8601 timestamp string to epoch millis, falling back to 0.
fn parse_timestamp(ts: &str) -> i64 {
    chrono::DateTime::parse_from_rfc3339(ts)
        .map(|dt| dt.timestamp_millis())
        .unwrap_or(0)
}
