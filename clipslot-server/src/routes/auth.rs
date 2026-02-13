use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use uuid::Uuid;

use crate::middleware::auth::{create_token, AuthUser};
use crate::models::device::{DeviceResponse, RegisterDeviceRequest};
use crate::models::user::{AuthResponse, LoginRequest, RegisterRequest};
use crate::AppState;

use argon2::Argon2;
use password_hash::rand_core::OsRng;
use password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};

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
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/device", post(register_device))
        .route("/device/{id}", delete(delete_device))
        .route("/devices", get(list_devices))
}

#[utoipa::path(
    post,
    path = "/api/auth/register",
    request_body = RegisterRequest,
    responses(
        (status = 200, description = "Account created", body = AuthResponse),
        (status = 400, description = "Invalid input", body = ApiError),
        (status = 409, description = "Email already registered", body = ApiError),
    ),
    tag = "Auth"
)]
pub(crate) async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<ApiError>)> {
    let email = req.email.trim().to_lowercase();
    if !email.contains('@') || email.len() < 5 {
        return Err(err(StatusCode::BAD_REQUEST, "Invalid email"));
    }
    if req.password.len() < 8 {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "Password must be at least 8 characters",
        ));
    }

    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(req.password.as_bytes(), &salt)
        .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "Failed to hash password"))?
        .to_string();

    let user_id: Uuid = sqlx::query_scalar(
        "INSERT INTO users (email, password_hash) VALUES ($1, $2) RETURNING id",
    )
    .bind(&email)
    .bind(&hash)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        if e.to_string().contains("unique") || e.to_string().contains("duplicate") {
            err(StatusCode::CONFLICT, "Email already registered")
        } else {
            err(StatusCode::INTERNAL_SERVER_ERROR, "Failed to create user")
        }
    })?;

    let token = create_token(user_id, None, &state.jwt_secret)
        .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "Failed to create token"))?;

    Ok(Json(AuthResponse { token, user_id }))
}

#[utoipa::path(
    post,
    path = "/api/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = AuthResponse),
        (status = 401, description = "Invalid credentials", body = ApiError),
    ),
    tag = "Auth"
)]
pub(crate) async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<ApiError>)> {
    let email = req.email.trim().to_lowercase();

    let row =
        sqlx::query_as::<_, (Uuid, String)>("SELECT id, password_hash FROM users WHERE email = $1")
            .bind(&email)
            .fetch_optional(&state.db)
            .await
            .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?
            .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "Invalid credentials"))?;

    let (user_id, password_hash) = row;

    let parsed_hash = PasswordHash::new(&password_hash)
        .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "Invalid stored hash"))?;

    Argon2::default()
        .verify_password(req.password.as_bytes(), &parsed_hash)
        .map_err(|_| err(StatusCode::UNAUTHORIZED, "Invalid credentials"))?;

    let token = create_token(user_id, None, &state.jwt_secret)
        .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "Failed to create token"))?;

    Ok(Json(AuthResponse { token, user_id }))
}

#[utoipa::path(
    post,
    path = "/api/auth/device",
    request_body = RegisterDeviceRequest,
    responses(
        (status = 200, description = "Device registered, returns new JWT with device_id"),
    ),
    security(("bearer" = [])),
    tag = "Auth"
)]
pub(crate) async fn register_device(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<RegisterDeviceRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let device_id: Uuid = sqlx::query_scalar(
        "INSERT INTO devices (user_id, name, device_type) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(auth.user_id)
    .bind(&req.name)
    .bind(&req.device_type)
    .fetch_one(&state.db)
    .await
    .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "Failed to register device"))?;

    let token = create_token(auth.user_id, Some(device_id), &state.jwt_secret)
        .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "Failed to create token"))?;

    Ok(Json(serde_json::json!({
        "device_id": device_id,
        "token": token,
    })))
}

#[utoipa::path(
    delete,
    path = "/api/auth/device/{id}",
    params(("id" = Uuid, Path, description = "Device UUID")),
    responses(
        (status = 204, description = "Device deleted"),
        (status = 404, description = "Device not found", body = ApiError),
    ),
    security(("bearer" = [])),
    tag = "Auth"
)]
pub(crate) async fn delete_device(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(device_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ApiError>)> {
    let result = sqlx::query("DELETE FROM devices WHERE id = $1 AND user_id = $2")
        .bind(device_id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await
        .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    if result.rows_affected() == 0 {
        return Err(err(StatusCode::NOT_FOUND, "Device not found"));
    }

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/api/auth/devices",
    responses(
        (status = 200, description = "List of registered devices", body = Vec<DeviceResponse>),
    ),
    security(("bearer" = [])),
    tag = "Auth"
)]
pub(crate) async fn list_devices(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<DeviceResponse>>, (StatusCode, Json<ApiError>)> {
    let devices = sqlx::query_as::<_, crate::models::device::Device>(
        "SELECT id, user_id, name, device_type, last_seen, created_at
         FROM devices WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    Ok(Json(devices.into_iter().map(DeviceResponse::from).collect()))
}
