use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use uuid::Uuid;

use crate::models::user::Claims;
use crate::AppState;

/// Extractor for authenticated requests. Extracts user_id and device_id from JWT.
pub struct AuthUser {
    pub user_id: Uuid,
    pub device_id: Option<Uuid>,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = (StatusCode, &'static str);

    fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        let jwt_secret = state.jwt_secret.clone();
        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        async move {
            let header = auth_header
                .ok_or((StatusCode::UNAUTHORIZED, "Missing Authorization header"))?;

            let token = header
                .strip_prefix("Bearer ")
                .ok_or((StatusCode::UNAUTHORIZED, "Invalid Authorization format"))?;

            let token_data = decode::<Claims>(
                token,
                &DecodingKey::from_secret(jwt_secret.as_bytes()),
                &Validation::default(),
            )
            .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid or expired token"))?;

            Ok(AuthUser {
                user_id: token_data.claims.sub,
                device_id: token_data.claims.device_id,
            })
        }
    }
}

pub fn create_token(
    user_id: Uuid,
    device_id: Option<Uuid>,
    secret: &str,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = chrono::Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: user_id,
        device_id,
        exp: now + 7 * 24 * 3600,
        iat: now,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

/// Validate a token string and return claims. Used by WebSocket auth.
pub fn validate_token(token: &str, secret: &str) -> Result<Claims, ()> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|_| ())
}
