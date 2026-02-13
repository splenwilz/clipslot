use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Router,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use futures::{SinkExt, StreamExt};
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;

use crate::middleware::auth::validate_token;
use crate::models::sync::WsMessage;
use crate::AppState;

#[derive(serde::Deserialize)]
struct WsQuery {
    token: String,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/api/sync/ws", get(ws_handler))
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(query): Query<WsQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    let claims =
        validate_token(&query.token, &state.jwt_secret).map_err(|_| StatusCode::UNAUTHORIZED)?;

    let user_id = claims.sub;
    let device_id = claims.device_id.ok_or(StatusCode::UNAUTHORIZED)?;

    Ok(ws.on_upgrade(move |socket| handle_socket(socket, state, user_id, device_id)))
}

fn get_or_create_channel(
    state: &AppState,
    user_id: Uuid,
) -> broadcast::Sender<(Uuid, String)> {
    state
        .user_channels
        .entry(user_id)
        .or_insert_with(|| broadcast::channel(100).0)
        .clone()
}

async fn handle_socket(socket: WebSocket, state: AppState, user_id: Uuid, device_id: Uuid) {
    let (mut sender, mut receiver) = socket.split();

    let tx = get_or_create_channel(&state, user_id);
    let mut rx = tx.subscribe();

    // Direct channel for messages targeted at this specific connection (errors, acks)
    let (direct_tx, mut direct_rx) = mpsc::channel::<String>(32);

    // Update device last_seen
    let _ = sqlx::query("UPDATE devices SET last_seen = NOW() WHERE id = $1")
        .bind(device_id)
        .execute(&state.db)
        .await;

    tracing::info!("WebSocket connected: user={}, device={}", user_id, device_id);

    // Task: forward broadcast messages and direct messages to this client
    let send_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                result = rx.recv() => {
                    match result {
                        Ok((origin_device, payload)) => {
                            if origin_device == device_id {
                                continue;
                            }
                            if sender.send(Message::Text(payload.into())).await.is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
                Some(payload) = direct_rx.recv() => {
                    if sender.send(Message::Text(payload.into())).await.is_err() {
                        break;
                    }
                }
            }
        }
    });

    // Task: process incoming messages from this client
    let state_clone = state.clone();
    let tx_clone = tx.clone();
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    handle_ws_message(&state_clone, user_id, device_id, &text, &tx_clone, &direct_tx).await;
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }

    tracing::info!(
        "WebSocket disconnected: user={}, device={}",
        user_id,
        device_id
    );
}

async fn handle_ws_message(
    state: &AppState,
    user_id: Uuid,
    device_id: Uuid,
    text: &str,
    tx: &broadcast::Sender<(Uuid, String)>,
    direct_tx: &mpsc::Sender<String>,
) {
    let msg: WsMessage = match serde_json::from_str(text) {
        Ok(m) => m,
        Err(e) => {
            let err_msg = WsMessage::Error {
                message: format!("Invalid message: {}", e),
            };
            let _ = direct_tx.send(serde_json::to_string(&err_msg).unwrap()).await;
            return;
        }
    };

    match msg {
        WsMessage::SlotUpdate {
            slot_number,
            encrypted_blob,
            timestamp,
        } => {
            if !(1..=10).contains(&slot_number) {
                let err_msg = WsMessage::Error {
                    message: "Invalid slot number".to_string(),
                };
                let _ = direct_tx.send(serde_json::to_string(&err_msg).unwrap()).await;
                return;
            }

            let blob = match BASE64.decode(&encrypted_blob) {
                Ok(b) => b,
                Err(_) => {
                    let err_msg = WsMessage::Error {
                        message: "Invalid base64 blob".to_string(),
                    };
                    let _ = direct_tx.send(serde_json::to_string(&err_msg).unwrap()).await;
                    return;
                }
            };

            let result = sqlx::query(
                "INSERT INTO synced_slots (user_id, slot_number, encrypted_blob, updated_at, updated_by)
                 VALUES ($1, $2, $3, NOW(), $4)
                 ON CONFLICT (user_id, slot_number)
                 DO UPDATE SET encrypted_blob = $3, updated_at = NOW(), updated_by = $4",
            )
            .bind(user_id)
            .bind(slot_number)
            .bind(&blob)
            .bind(device_id)
            .execute(&state.db)
            .await;

            if let Err(e) = result {
                tracing::error!("Failed to save slot update: {}", e);
                let err_msg = WsMessage::Error {
                    message: format!("Failed to save slot update: {}", e),
                };
                let _ = direct_tx.send(serde_json::to_string(&err_msg).unwrap()).await;
                return;
            }

            let response = WsMessage::SlotUpdated {
                slot_number,
                encrypted_blob,
                updated_by: device_id,
                timestamp,
            };
            let _ = tx.send((device_id, serde_json::to_string(&response).unwrap()));
        }

        WsMessage::HistoryPush {
            id,
            encrypted_blob,
            content_hash,
        } => {
            let blob = match BASE64.decode(&encrypted_blob) {
                Ok(b) => b,
                Err(_) => {
                    let err_msg = WsMessage::Error {
                        message: "Invalid base64 blob".to_string(),
                    };
                    let _ = direct_tx.send(serde_json::to_string(&err_msg).unwrap()).await;
                    return;
                }
            };

            let result = sqlx::query(
                "INSERT INTO synced_history (id, user_id, encrypted_blob, content_hash, device_id, created_at)
                 VALUES ($1, $2, $3, $4, $5, NOW())
                 ON CONFLICT (user_id, content_hash) DO NOTHING",
            )
            .bind(id)
            .bind(user_id)
            .bind(&blob)
            .bind(&content_hash)
            .bind(device_id)
            .execute(&state.db)
            .await;

            match result {
                Ok(r) if r.rows_affected() > 0 => {
                    let response = WsMessage::HistoryNew {
                        id,
                        encrypted_blob,
                        content_hash,
                        device_id,
                    };
                    let _ = tx.send((device_id, serde_json::to_string(&response).unwrap()));
                }
                Err(e) => {
                    tracing::error!("Failed to save history push: {}", e);
                    let err_msg = WsMessage::Error {
                        message: format!("Failed to save history: {}", e),
                    };
                    let _ = direct_tx.send(serde_json::to_string(&err_msg).unwrap()).await;
                }
                _ => {} // Dedup — item already exists
            }
        }

        // Ignore server-to-client message types
        _ => {}
    }
}
