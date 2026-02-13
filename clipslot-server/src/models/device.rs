use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
pub struct Device {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub device_type: String,
    pub last_seen: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterDeviceRequest {
    /// Device name (e.g., "MacBook Pro")
    pub name: String,
    /// Device type: "macos", "windows", "linux"
    pub device_type: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DeviceResponse {
    pub id: Uuid,
    pub name: String,
    pub device_type: String,
    pub last_seen: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl From<Device> for DeviceResponse {
    fn from(d: Device) -> Self {
        Self {
            id: d.id,
            name: d.name,
            device_type: d.device_type,
            last_seen: d.last_seen,
            created_at: d.created_at,
        }
    }
}
