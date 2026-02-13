use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Auth types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub user_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceRegistrationResponse {
    pub device_id: Uuid,
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: Uuid,
    pub name: String,
    pub device_type: String,
    pub last_seen: String,
    pub created_at: String,
}

// ── Sync types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotResponse {
    pub slot_number: i32,
    pub encrypted_blob: String,
    pub updated_at: String,
    pub updated_by: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSlotRequest {
    pub encrypted_blob: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushHistoryRequest {
    pub id: Uuid,
    pub encrypted_blob: String,
    pub content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryResponse {
    pub id: Uuid,
    pub encrypted_blob: String,
    pub content_hash: String,
    pub device_id: Option<Uuid>,
    pub created_at: String,
}

// ── Status types ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SyncStatus {
    Disconnected,
    Connecting,
    Connected,
    Syncing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncState {
    pub status: SyncStatus,
    pub logged_in: bool,
    pub email: Option<String>,
    pub device_id: Option<Uuid>,
    pub history_sync_enabled: bool,
}

// ── WebSocket messages (mirrors server's WsMessage) ─────────────────────────

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
    Error {
        message: String,
    },
}
