pub mod auth;
pub mod sync;
pub mod ws;

use axum::Router;
use crate::AppState;

pub fn api_router(state: AppState) -> Router {
    Router::new()
        .nest("/api/auth", auth::router())
        .nest("/api/sync", sync::router())
        .merge(ws::router())
        .with_state(state)
}
