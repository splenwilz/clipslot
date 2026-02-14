use std::sync::Arc;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::storage::database::Database;

use super::api_client::ApiClient;
use super::offline_queue::OfflineQueue;
use super::types::*;
use super::ws_client::WsClient;

struct AuthState {
    token: String,
    user_id: Uuid,
    device_id: Uuid,
    email: String,
}

pub struct SyncManager {
    api: RwLock<ApiClient>,
    db: Arc<Database>,
    auth: RwLock<Option<AuthState>>,
    ws: RwLock<Option<WsClient>>,
    status: RwLock<SyncStatus>,
    offline_queue: OfflineQueue,
}

impl SyncManager {
    pub fn new(db: Arc<Database>) -> Self {
        let server_url = db
            .get_setting("sync_server_url")
            .unwrap_or_else(|| crate::config::SYNC_SERVER_URL.to_string());

        let manager = Self {
            api: RwLock::new(ApiClient::new(&server_url)),
            db,
            auth: RwLock::new(None),
            ws: RwLock::new(None),
            status: RwLock::new(SyncStatus::Disconnected),
            offline_queue: OfflineQueue::new(),
        };

        // Try to restore auth from persisted settings
        manager.try_restore_auth();
        manager
    }

    fn try_restore_auth(&self) {
        let token = self.db.get_setting("auth_token");
        let user_id = self.db.get_setting("auth_user_id");
        let device_id = self.db.get_setting("auth_device_id");
        let email = self.db.get_setting("auth_email");

        if let (Some(token), Some(user_id_str), Some(device_id_str), Some(email)) =
            (token, user_id, device_id, email)
        {
            // Skip empty values (cleared by logout)
            if token.is_empty() || user_id_str.is_empty() {
                return;
            }
            if let (Ok(user_id), Ok(device_id)) = (
                Uuid::parse_str(&user_id_str),
                Uuid::parse_str(&device_id_str),
            ) {
                let auth = AuthState {
                    token,
                    user_id,
                    device_id,
                    email: email.clone(),
                };
                *self.auth.blocking_write() = Some(auth);
                println!("[ClipSlot] Restored auth session for {}", email);
            }
        }
    }

    fn persist_auth(&self, state: &AuthState) {
        let _ = self.db.set_setting("auth_token", &state.token);
        let _ = self
            .db
            .set_setting("auth_user_id", &state.user_id.to_string());
        let _ = self
            .db
            .set_setting("auth_device_id", &state.device_id.to_string());
        let _ = self.db.set_setting("auth_email", &state.email);
    }

    fn clear_auth_settings(&self) {
        let _ = self.db.set_setting("auth_token", "");
        let _ = self.db.set_setting("auth_user_id", "");
        let _ = self.db.set_setting("auth_device_id", "");
        let _ = self.db.set_setting("auth_email", "");
    }

    fn get_device_name() -> String {
        hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "Unknown Device".to_string())
    }

    fn get_device_type() -> String {
        if cfg!(target_os = "macos") {
            "macos".to_string()
        } else if cfg!(target_os = "windows") {
            "windows".to_string()
        } else {
            "linux".to_string()
        }
    }

    pub async fn login(&self, email: &str, password: &str) -> Result<SyncState, String> {
        let api = self.api.read().await;

        let auth_resp = api.login(email, password).await?;

        let device_resp = api
            .register_device(
                &auth_resp.token,
                &Self::get_device_name(),
                &Self::get_device_type(),
            )
            .await?;

        let state = AuthState {
            token: device_resp.token,
            user_id: auth_resp.user_id,
            device_id: device_resp.device_id,
            email: email.to_string(),
        };

        self.persist_auth(&state);
        *self.auth.write().await = Some(state);

        Ok(self.build_sync_state().await)
    }

    pub async fn register(&self, email: &str, password: &str) -> Result<SyncState, String> {
        let api = self.api.read().await;

        let auth_resp = api.register(email, password).await?;

        let device_resp = api
            .register_device(
                &auth_resp.token,
                &Self::get_device_name(),
                &Self::get_device_type(),
            )
            .await?;

        let state = AuthState {
            token: device_resp.token,
            user_id: auth_resp.user_id,
            device_id: device_resp.device_id,
            email: email.to_string(),
        };

        self.persist_auth(&state);
        *self.auth.write().await = Some(state);

        Ok(self.build_sync_state().await)
    }

    pub async fn logout(&self) -> Result<(), String> {
        // Disconnect WebSocket
        if let Some(ws) = self.ws.write().await.take() {
            ws.disconnect().await;
        }
        *self.status.write().await = SyncStatus::Disconnected;
        self.clear_auth_settings();
        *self.auth.write().await = None;
        println!("[ClipSlot] Logged out");
        Ok(())
    }

    pub async fn get_sync_status(&self) -> SyncState {
        self.build_sync_state().await
    }

    pub async fn get_linked_devices(&self) -> Result<Vec<DeviceInfo>, String> {
        let auth = self.auth.read().await;
        let auth = auth.as_ref().ok_or("Not logged in")?;
        let api = self.api.read().await;
        api.list_devices(&auth.token).await
    }

    pub async fn start_sync(&self) -> Result<String, String> {
        clog!("start_sync: beginning...");
        let auth_guard = self.auth.read().await;
        let auth = auth_guard.as_ref().ok_or("Not logged in")?;
        let token = auth.token.clone();
        let device_id = auth.device_id.to_string();
        clog!("start_sync: device_id={}", device_id);
        drop(auth_guard);
        let api = self.api.read().await;
        clog!("start_sync: API base_url={}", api.base_url());

        *self.status.write().await = SyncStatus::Syncing;

        clog!("start_sync: performing slot sync...");
        let slot_synced = super::slot_sync::perform_full_slot_sync(
            &api,
            &token,
            &self.db,
            &device_id,
        )
        .await?;
        clog!("start_sync: slot sync done, synced {} slots", slot_synced);

        // History sync (opt-in)
        let history_sync_enabled = self
            .db
            .get_setting("history_sync_enabled")
            .map(|v| v == "true")
            .unwrap_or(false);
        clog!("start_sync: history_sync_enabled={}", history_sync_enabled);

        let mut history_msg = String::new();
        if history_sync_enabled {
            match super::history_sync::perform_initial_history_sync(
                &api,
                &token,
                &self.db,
                &device_id,
            )
            .await
            {
                Ok((pulled, pushed)) => {
                    history_msg = format!(", history: pulled {}, pushed {}", pulled, pushed);
                    clog!("start_sync: history pulled={}, pushed={}", pulled, pushed);
                }
                Err(e) => {
                    clog!("ERROR: History sync failed: {}", e);
                }
            }
        }

        *self.status.write().await = SyncStatus::Connected;

        Ok(format!("Synced {} slots{}", slot_synced, history_msg))
    }

    // ── WebSocket ───────────────────────────────────────────────────────

    pub async fn connect_ws(&self) -> Result<(), String> {
        clog!("connect_ws: starting...");

        // Disconnect any existing WS connection first
        if let Some(old_ws) = self.ws.write().await.take() {
            clog!("connect_ws: disconnecting old WS connection");
            old_ws.disconnect().await;
        }

        let auth_guard = self.auth.read().await;
        let auth = auth_guard.as_ref().ok_or("Not logged in")?;
        let api = self.api.read().await;

        let ws_url = api.ws_url(&auth.token);
        clog!("connect_ws: URL={}", ws_url.split('?').next().unwrap_or(&ws_url));
        drop(api);
        drop(auth_guard);

        *self.status.write().await = SyncStatus::Connecting;

        let client = WsClient::connect(&ws_url).await?;
        clog!("connect_ws: WebSocket connected successfully");

        // Spawn a task to handle incoming WS messages
        let mut rx = client.subscribe();
        let db = self.db.clone();
        let device_id_str = self
            .auth
            .read()
            .await
            .as_ref()
            .map(|a| a.device_id.to_string())
            .unwrap_or_default();

        tokio::spawn(async move {
            clog!("WS message handler started, listening for broadcasts...");
            while let Ok(msg) = rx.recv().await {
                clog!("WS handler: received broadcast message");
                match msg {
                    WsMessage::SlotUpdated {
                        slot_number,
                        encrypted_blob,
                        timestamp,
                        ..
                    } => {
                        clog!("WS handler: SlotUpdated slot={}", slot_number);
                        if let Ok(blob_bytes) = BASE64.decode(&encrypted_blob) {
                            if let Ok(enc_str) = String::from_utf8(blob_bytes) {
                                if let Err(e) = db.save_encrypted_to_slot(
                                    slot_number as u32,
                                    &enc_str,
                                    timestamp,
                                    &device_id_str,
                                ) {
                                    clog!(
                                        "ERROR: Failed to save synced slot {}: {}",
                                        slot_number, e
                                    );
                                } else {
                                    clog!("Slot {} updated from remote", slot_number);
                                }
                            } else {
                                clog!("ERROR: SlotUpdated blob is not valid UTF-8");
                            }
                        } else {
                            clog!("ERROR: SlotUpdated blob is not valid base64");
                        }
                    }
                    WsMessage::HistoryNew {
                        id,
                        encrypted_blob,
                        content_hash,
                        device_id,
                    } => {
                        clog!("WS handler: HistoryNew id={}", id);
                        if let Ok(blob_bytes) = BASE64.decode(&encrypted_blob) {
                            if let Ok(enc_str) = String::from_utf8(blob_bytes) {
                                let now = chrono::Utc::now().timestamp_millis();
                                if let Err(e) = db.insert_synced_item(
                                    &id.to_string(),
                                    &enc_str,
                                    &content_hash,
                                    &device_id.to_string(),
                                    now,
                                ) {
                                    clog!("ERROR: Failed to save synced history item: {}", e);
                                } else {
                                    clog!("History item received from remote");
                                }
                            }
                        }
                    }
                    WsMessage::Error { message } => {
                        clog!("WS handler: server error: {}", message);
                    }
                    _ => {
                        clog!("WS handler: ignoring message type");
                    }
                }
            }
            clog!("WS message handler ended (broadcast channel closed)");
        });

        *self.ws.write().await = Some(client);
        *self.status.write().await = SyncStatus::Connected;
        println!("[ClipSlot] WebSocket connected and listening");

        // Flush any messages queued while offline
        self.flush_offline_queue().await;

        Ok(())
    }

    /// Notify the server of a local slot change via WebSocket.
    /// If WS is disconnected, queues the message for later.
    pub async fn notify_slot_changed(&self, slot_number: u32) {
        clog!("notify_slot_changed: slot {}", slot_number);
        let auth = self.auth.read().await;
        if auth.is_none() {
            clog!("notify_slot_changed: no auth, skipping");
            return;
        }
        drop(auth);

        // Get the raw encrypted content for this slot
        let (encrypted, _) = match self.db.get_slot_raw(slot_number) {
            Ok(r) => r,
            Err(_) => return,
        };

        let encrypted = match encrypted {
            Some(e) => e,
            None => return,
        };

        // Encode as base64 for the server
        let blob = BASE64.encode(encrypted.as_bytes());
        let timestamp = chrono::Utc::now().timestamp_millis();

        let msg = WsMessage::SlotUpdate {
            slot_number: slot_number as i32,
            encrypted_blob: blob,
            timestamp,
        };

        self.send_or_queue(msg).await;
    }

    /// Notify the server of a new history item via WebSocket.
    /// If WS is disconnected, queues the message for later.
    pub async fn notify_history_push(&self, id: &str, encrypted: &str, content_hash: &str) {
        let auth = self.auth.read().await;
        if auth.is_none() {
            return;
        }
        drop(auth);

        let history_enabled = self
            .db
            .get_setting("history_sync_enabled")
            .map(|v| v == "true")
            .unwrap_or(false);

        if !history_enabled {
            return;
        }

        let blob = BASE64.encode(encrypted.as_bytes());

        let msg = WsMessage::HistoryPush {
            id: uuid::Uuid::parse_str(id).unwrap_or_else(|_| uuid::Uuid::new_v4()),
            encrypted_blob: blob,
            content_hash: content_hash.to_string(),
        };

        self.send_or_queue(msg).await;
    }

    /// Send a message via WS if connected, otherwise enqueue for later.
    async fn send_or_queue(&self, msg: WsMessage) {
        let ws = self.ws.read().await;
        if let Some(client) = ws.as_ref() {
            clog!("send_or_queue: sending via WS");
            if let Err(e) = client.send(&msg).await {
                clog!("ERROR: WS send failed, queuing: {}", e);
                self.offline_queue.enqueue(msg);
            }
        } else {
            clog!("send_or_queue: WS not connected, queuing message");
            self.offline_queue.enqueue(msg);
        }
    }

    /// Flush any queued messages through the WS connection.
    async fn flush_offline_queue(&self) {
        let messages = self.offline_queue.drain();
        if messages.is_empty() {
            return;
        }

        println!("[ClipSlot] Flushing {} queued messages", messages.len());
        let ws = self.ws.read().await;
        if let Some(client) = ws.as_ref() {
            for msg in messages {
                if let Err(e) = client.send(&msg).await {
                    eprintln!("[ClipSlot] Failed to flush queued message: {}", e);
                    // Re-queue failed messages
                    self.offline_queue.enqueue(msg);
                    break;
                }
            }
        }
    }

    pub async fn get_token(&self) -> Option<String> {
        self.auth.read().await.as_ref().map(|a| a.token.clone())
    }

    /// Get a clone of the API client for use by commands.
    pub async fn get_api(&self) -> ApiClient {
        self.api.read().await.clone()
    }

    pub async fn is_logged_in(&self) -> bool {
        self.auth.read().await.is_some()
    }

    /// Synchronous check for auth state (for tray menu).
    pub fn has_auth(&self) -> bool {
        self.auth.blocking_read().is_some()
    }

    /// Synchronous status read (for tray menu).
    pub fn get_status_blocking(&self) -> SyncStatus {
        self.status.blocking_read().clone()
    }

    async fn build_sync_state(&self) -> SyncState {
        let auth = self.auth.read().await;
        let status = self.status.read().await.clone();
        let history_sync = self
            .db
            .get_setting("history_sync_enabled")
            .map(|v| v == "true")
            .unwrap_or(false);

        match auth.as_ref() {
            Some(a) => SyncState {
                status,
                logged_in: true,
                email: Some(a.email.clone()),
                device_id: Some(a.device_id),
                history_sync_enabled: history_sync,
            },
            None => SyncState {
                status: SyncStatus::Disconnected,
                logged_in: false,
                email: None,
                device_id: None,
                history_sync_enabled: history_sync,
            },
        }
    }
}
