use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use uuid::Uuid;

use crate::middleware::auth::AuthUser;
use crate::models::sync::{
    HistoryQuery, HistoryResponse, PushHistoryRequest, SlotResponse, SyncedHistoryItem,
    SyncedSlot, UpdateSlotRequest, WsMessage,
};
use crate::AppState;

#[derive(serde::Serialize, utoipa::ToSchema)]
pub(crate) struct ApiError {
    error: String,
}

fn err(status: StatusCode, msg: &str) -> (StatusCode, Json<ApiError>) {
    (
        status,
        Json(ApiError {
            error: msg.to_string(),
        }),
    )
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/slots", get(get_slots))
        .route("/slots/{number}", put(update_slot))
        .route("/history", get(get_history))
        .route("/history", post(push_history))
        .route("/history/{id}", delete(delete_history))
}

#[utoipa::path(
    get,
    path = "/api/sync/slots",
    responses(
        (status = 200, description = "All encrypted slots", body = Vec<SlotResponse>),
    ),
    security(("bearer" = [])),
    tag = "Sync"
)]
pub(crate) async fn get_slots(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<SlotResponse>>, (StatusCode, Json<ApiError>)> {
    let slots = sqlx::query_as::<_, SyncedSlot>(
        "SELECT user_id, slot_number, encrypted_blob, updated_at, updated_by
         FROM synced_slots WHERE user_id = $1 ORDER BY slot_number",
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    let response: Vec<SlotResponse> = slots
        .into_iter()
        .map(|s| SlotResponse {
            slot_number: s.slot_number,
            encrypted_blob: BASE64.encode(&s.encrypted_blob),
            updated_at: s.updated_at,
            updated_by: s.updated_by,
        })
        .collect();

    Ok(Json(response))
}

#[utoipa::path(
    put,
    path = "/api/sync/slots/{number}",
    params(("number" = i32, Path, description = "Slot number (1-10)")),
    request_body = UpdateSlotRequest,
    responses(
        (status = 200, description = "Slot updated"),
        (status = 400, description = "Invalid slot number or blob"),
    ),
    security(("bearer" = [])),
    tag = "Sync"
)]
pub(crate) async fn update_slot(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(slot_number): Path<i32>,
    Json(req): Json<UpdateSlotRequest>,
) -> Result<StatusCode, (StatusCode, Json<ApiError>)> {
    if !(1..=10).contains(&slot_number) {
        return Err(err(StatusCode::BAD_REQUEST, "Invalid slot number (1-10)"));
    }

    let blob = BASE64
        .decode(&req.encrypted_blob)
        .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid base64 blob"))?;

    let device_id = auth.device_id;

    sqlx::query(
        "INSERT INTO synced_slots (user_id, slot_number, encrypted_blob, updated_at, updated_by)
         VALUES ($1, $2, $3, NOW(), $4)
         ON CONFLICT (user_id, slot_number)
         DO UPDATE SET encrypted_blob = $3, updated_at = NOW(), updated_by = $4",
    )
    .bind(auth.user_id)
    .bind(slot_number)
    .bind(&blob)
    .bind(device_id)
    .execute(&state.db)
    .await
    .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "Failed to update slot"))?;

    if let Some(device_id) = device_id {
        if let Some(tx) = state.user_channels.get(&auth.user_id) {
            let msg = WsMessage::SlotUpdated {
                slot_number,
                encrypted_blob: req.encrypted_blob,
                updated_by: device_id,
                timestamp: chrono::Utc::now().timestamp_millis(),
            };
            let _ = tx.send((device_id, serde_json::to_string(&msg).unwrap()));
        }
    }

    Ok(StatusCode::OK)
}

#[utoipa::path(
    get,
    path = "/api/sync/history",
    params(HistoryQuery),
    responses(
        (status = 200, description = "Paginated encrypted history", body = Vec<HistoryResponse>),
    ),
    security(("bearer" = [])),
    tag = "Sync"
)]
pub(crate) async fn get_history(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<Vec<HistoryResponse>>, (StatusCode, Json<ApiError>)> {
    let limit = query.limit.unwrap_or(50).min(200);
    let offset = query.offset.unwrap_or(0);

    let items = sqlx::query_as::<_, SyncedHistoryItem>(
        "SELECT id, user_id, encrypted_blob, content_hash, device_id, created_at
         FROM synced_history WHERE user_id = $1
         ORDER BY created_at DESC
         LIMIT $2 OFFSET $3",
    )
    .bind(auth.user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    let response: Vec<HistoryResponse> = items
        .into_iter()
        .map(|i| HistoryResponse {
            id: i.id,
            encrypted_blob: BASE64.encode(&i.encrypted_blob),
            content_hash: i.content_hash,
            device_id: i.device_id,
            created_at: i.created_at,
        })
        .collect();

    Ok(Json(response))
}

#[utoipa::path(
    post,
    path = "/api/sync/history",
    request_body = PushHistoryRequest,
    responses(
        (status = 201, description = "History item created"),
        (status = 400, description = "Invalid blob"),
    ),
    security(("bearer" = [])),
    tag = "Sync"
)]
pub(crate) async fn push_history(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<PushHistoryRequest>,
) -> Result<StatusCode, (StatusCode, Json<ApiError>)> {
    let blob = BASE64
        .decode(&req.encrypted_blob)
        .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid base64 blob"))?;

    let device_id = auth.device_id;

    let result = sqlx::query(
        "INSERT INTO synced_history (id, user_id, encrypted_blob, content_hash, device_id, created_at)
         VALUES ($1, $2, $3, $4, $5, NOW())
         ON CONFLICT (user_id, content_hash) DO NOTHING",
    )
    .bind(req.id)
    .bind(auth.user_id)
    .bind(&blob)
    .bind(&req.content_hash)
    .bind(device_id)
    .execute(&state.db)
    .await
    .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "Failed to push history"))?;

    if result.rows_affected() > 0 {
        if let Some(device_id) = device_id {
            if let Some(tx) = state.user_channels.get(&auth.user_id) {
                let msg = WsMessage::HistoryNew {
                    id: req.id,
                    encrypted_blob: req.encrypted_blob,
                    content_hash: req.content_hash,
                    device_id,
                };
                let _ = tx.send((device_id, serde_json::to_string(&msg).unwrap()));
            }
        }
    }

    Ok(StatusCode::CREATED)
}

#[utoipa::path(
    delete,
    path = "/api/sync/history/{id}",
    params(("id" = Uuid, Path, description = "History item UUID")),
    responses(
        (status = 204, description = "History item deleted"),
        (status = 404, description = "Not found"),
    ),
    security(("bearer" = [])),
    tag = "Sync"
)]
pub(crate) async fn delete_history(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(item_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ApiError>)> {
    let result = sqlx::query("DELETE FROM synced_history WHERE id = $1 AND user_id = $2")
        .bind(item_id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await
        .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    if result.rows_affected() == 0 {
        return Err(err(StatusCode::NOT_FOUND, "History item not found"));
    }

    Ok(StatusCode::NO_CONTENT)
}
