use reqwest::Client;
use uuid::Uuid;

use super::types::*;

#[derive(Clone)]
pub struct ApiClient {
    client: Client,
    base_url: String,
}

impl ApiClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    // ── Auth ────────────────────────────────────────────────────────────

    pub async fn register(&self, email: &str, password: &str) -> Result<AuthResponse, String> {
        let resp = self
            .client
            .post(format!("{}/api/auth/register", self.base_url))
            .json(&serde_json::json!({ "email": email, "password": password }))
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(extract_error(&body));
        }

        resp.json::<AuthResponse>()
            .await
            .map_err(|e| format!("Parse error: {}", e))
    }

    pub async fn login(&self, email: &str, password: &str) -> Result<AuthResponse, String> {
        let resp = self
            .client
            .post(format!("{}/api/auth/login", self.base_url))
            .json(&serde_json::json!({ "email": email, "password": password }))
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(extract_error(&body));
        }

        resp.json::<AuthResponse>()
            .await
            .map_err(|e| format!("Parse error: {}", e))
    }

    pub async fn register_device(
        &self,
        token: &str,
        name: &str,
        device_type: &str,
    ) -> Result<DeviceRegistrationResponse, String> {
        let resp = self
            .client
            .post(format!("{}/api/auth/device", self.base_url))
            .bearer_auth(token)
            .json(&serde_json::json!({ "name": name, "device_type": device_type }))
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(extract_error(&body));
        }

        resp.json::<DeviceRegistrationResponse>()
            .await
            .map_err(|e| format!("Parse error: {}", e))
    }

    pub async fn list_devices(&self, token: &str) -> Result<Vec<DeviceInfo>, String> {
        let resp = self
            .client
            .get(format!("{}/api/auth/devices", self.base_url))
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(extract_error(&body));
        }

        resp.json::<Vec<DeviceInfo>>()
            .await
            .map_err(|e| format!("Parse error: {}", e))
    }

    pub async fn delete_device(&self, token: &str, device_id: Uuid) -> Result<(), String> {
        let resp = self
            .client
            .delete(format!("{}/api/auth/device/{}", self.base_url, device_id))
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(extract_error(&body));
        }

        Ok(())
    }

    // ── Slots ───────────────────────────────────────────────────────────

    pub async fn get_slots(&self, token: &str) -> Result<Vec<SlotResponse>, String> {
        let resp = self
            .client
            .get(format!("{}/api/sync/slots", self.base_url))
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(extract_error(&body));
        }

        resp.json::<Vec<SlotResponse>>()
            .await
            .map_err(|e| format!("Parse error: {}", e))
    }

    pub async fn update_slot(
        &self,
        token: &str,
        slot_number: i32,
        encrypted_blob: &str,
    ) -> Result<(), String> {
        let resp = self
            .client
            .put(format!(
                "{}/api/sync/slots/{}",
                self.base_url, slot_number
            ))
            .bearer_auth(token)
            .json(&UpdateSlotRequest {
                encrypted_blob: encrypted_blob.to_string(),
            })
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(extract_error(&body));
        }

        Ok(())
    }

    // ── History ─────────────────────────────────────────────────────────

    pub async fn get_history(
        &self,
        token: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<HistoryResponse>, String> {
        let resp = self
            .client
            .get(format!("{}/api/sync/history", self.base_url))
            .bearer_auth(token)
            .query(&[("limit", limit), ("offset", offset)])
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(extract_error(&body));
        }

        resp.json::<Vec<HistoryResponse>>()
            .await
            .map_err(|e| format!("Parse error: {}", e))
    }

    pub async fn push_history(&self, token: &str, req: &PushHistoryRequest) -> Result<(), String> {
        let resp = self
            .client
            .post(format!("{}/api/sync/history", self.base_url))
            .bearer_auth(token)
            .json(req)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(extract_error(&body));
        }

        Ok(())
    }

    // ── WebSocket ───────────────────────────────────────────────────────

    pub fn ws_url(&self, token: &str) -> String {
        let ws_base = self
            .base_url
            .replace("http://", "ws://")
            .replace("https://", "wss://");
        format!("{}/api/sync/ws?token={}", ws_base, token)
    }

    // ── Key Exchange ─────────────────────────────────────────────────────

    pub async fn generate_link_code(
        &self,
        token: &str,
        encrypted_key: &str,
    ) -> Result<String, String> {
        let resp = self
            .client
            .post(format!("{}/api/auth/link-code", self.base_url))
            .bearer_auth(token)
            .json(&serde_json::json!({ "encrypted_key": encrypted_key }))
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(extract_error(&body));
        }

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Parse error: {}", e))?;

        data.get("code")
            .and_then(|c| c.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| "Missing code in response".to_string())
    }

    pub async fn redeem_link_code(
        &self,
        token: &str,
        code: &str,
    ) -> Result<String, String> {
        let resp = self
            .client
            .post(format!("{}/api/auth/redeem-code", self.base_url))
            .bearer_auth(token)
            .json(&serde_json::json!({ "code": code }))
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(extract_error(&body));
        }

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Parse error: {}", e))?;

        data.get("encrypted_key")
            .and_then(|k| k.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| "Missing encrypted_key in response".to_string())
    }
}

fn extract_error(body: &str) -> String {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| v.get("error")?.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| body.to_string())
}
