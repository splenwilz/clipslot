use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardItem {
    pub id: String,
    pub content: String,
    pub content_hash: String,
    pub content_type: String,
    pub source_app: Option<String>,
    pub device_id: String,
    pub created_at: i64,
    pub is_promoted: bool,
}

impl ClipboardItem {
    pub fn new(content: String, device_id: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            content_hash: Self::hash_content(&content),
            content,
            content_type: "text/plain".to_string(),
            source_app: None,
            device_id: device_id.to_string(),
            created_at: Utc::now().timestamp_millis(),
            is_promoted: false,
        }
    }

    pub fn hash_content(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}
