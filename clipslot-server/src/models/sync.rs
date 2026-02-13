use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

// ── Database rows ────────────────────────────────────────────────────────────

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
pub struct SyncedSlot {
    pub user_id: Uuid,
    pub slot_number: i32,
    pub encrypted_blob: Vec<u8>,
    pub updated_at: DateTime<Utc>,
    pub updated_by: Option<Uuid>,
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
pub struct SyncedHistoryItem {
    pub id: Uuid,
    pub user_id: Uuid,
    pub encrypted_blob: Vec<u8>,
    pub content_hash: String,
    pub device_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

// ── API types ────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
pub struct SlotResponse {
    pub slot_number: i32,
    /// Base64-encoded encrypted blob
    pub encrypted_blob: String,
    pub updated_at: DateTime<Utc>,
    pub updated_by: Option<Uuid>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateSlotRequest {
    /// Base64-encoded encrypted blob
    pub encrypted_blob: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PushHistoryRequest {
    pub id: Uuid,
    /// Base64-encoded encrypted blob
    pub encrypted_blob: String,
    /// SHA-256 hash of the plaintext content (for dedup)
    pub content_hash: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct HistoryResponse {
    pub id: Uuid,
    /// Base64-encoded encrypted blob
    pub encrypted_blob: String,
    pub content_hash: String,
    pub device_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct HistoryQuery {
    /// Max items to return (default 50, max 200)
    pub limit: Option<i64>,
    /// Offset for pagination
    pub offset: Option<i64>,
}

// ── WebSocket messages ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsMessage {
    #[serde(rename = "slot_update")]
    SlotUpdate {
        slot_number: i32,
        encrypted_blob: String,
        timestamp: i64,
    },
    #[serde(rename = "slot_updated")]
    SlotUpdated {
        slot_number: i32,
        encrypted_blob: String,
        updated_by: Uuid,
        timestamp: i64,
    },
    #[serde(rename = "history_push")]
    HistoryPush {
        id: Uuid,
        encrypted_blob: String,
        content_hash: String,
    },
    #[serde(rename = "history_new")]
    HistoryNew {
        id: Uuid,
        encrypted_blob: String,
        content_hash: String,
        device_id: Uuid,
    },
    #[serde(rename = "error")]
    Error { message: String },
}
