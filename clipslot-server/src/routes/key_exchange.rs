use axum::{
    extract::State,
    http::StatusCode,
    routing::post,
    Json, Router,
};
use rand::Rng as _;

use crate::middleware::auth::AuthUser;
use crate::AppState;

#[derive(serde::Deserialize)]
pub struct GenerateCodeRequest {
    pub encrypted_key: String,
}

#[derive(serde::Serialize)]
pub struct GenerateCodeResponse {
    pub code: String,
}

#[derive(serde::Deserialize)]
pub struct RedeemCodeRequest {
    pub code: String,
}

#[derive(serde::Serialize)]
pub struct RedeemCodeResponse {
    pub encrypted_key: String,
}

#[derive(serde::Serialize)]
struct ApiError {
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
        .route("/link-code", post(generate_link_code))
        .route("/redeem-code", post(redeem_link_code))
}

/// Generate a 6-digit link code that holds the encrypted master key for 5 minutes.
async fn generate_link_code(
    State(state): State<AppState>,
    _auth: AuthUser,
    Json(req): Json<GenerateCodeRequest>,
) -> Result<Json<GenerateCodeResponse>, (StatusCode, Json<ApiError>)> {
    if req.encrypted_key.is_empty() {
        return Err(err(StatusCode::BAD_REQUEST, "encrypted_key is required"));
    }

    // Generate a random 6-digit code
    let code: String = {
        let mut rng = rand::thread_rng();
        format!("{:06}", rng.gen_range(0..1_000_000u32))
    };

    // Store with TTL (cleanup handled by background task)
    let now = std::time::Instant::now();
    state
        .link_codes
        .insert(code.clone(), (req.encrypted_key, now));

    Ok(Json(GenerateCodeResponse { code }))
}

/// Redeem a 6-digit link code to retrieve the encrypted master key.
/// The code is deleted after retrieval (one-time use).
async fn redeem_link_code(
    State(state): State<AppState>,
    _auth: AuthUser,
    Json(req): Json<RedeemCodeRequest>,
) -> Result<Json<RedeemCodeResponse>, (StatusCode, Json<ApiError>)> {
    let code = req.code.trim().to_string();

    if code.len() != 6 || !code.chars().all(|c| c.is_ascii_digit()) {
        return Err(err(StatusCode::BAD_REQUEST, "Code must be a 6-digit number"));
    }

    let entry = state.link_codes.remove(&code);

    match entry {
        Some((_, (encrypted_key, created_at))) => {
            // Check TTL (5 minutes)
            if created_at.elapsed() > std::time::Duration::from_secs(300) {
                return Err(err(StatusCode::GONE, "Code has expired"));
            }
            Ok(Json(RedeemCodeResponse { encrypted_key }))
        }
        None => Err(err(StatusCode::NOT_FOUND, "Invalid or expired code")),
    }
}
