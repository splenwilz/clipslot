pub mod manager;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotInfo {
    pub slot_number: u32,
    pub name: String,
    pub content: Option<String>,
    pub content_preview: Option<String>,
    pub updated_at: i64,
    pub is_empty: bool,
}
