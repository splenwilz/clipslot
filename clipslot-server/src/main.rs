mod config;
mod db;
mod middleware;
mod models;
mod routes;

use std::sync::Arc;

use dashmap::DashMap;
use sqlx::postgres::PgPoolOptions;
use tokio::sync::broadcast;
use axum::http::{HeaderValue, Method};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub jwt_secret: String,
    /// Per-user broadcast channels for WebSocket relay.
    /// Key: user_id, Value: sender that broadcasts (origin_device_id, json_payload).
    pub user_channels: Arc<DashMap<Uuid, broadcast::Sender<(Uuid, String)>>>,
}

#[derive(OpenApi)]
#[openapi(
    paths(
        routes::auth::register,
        routes::auth::login,
        routes::auth::register_device,
        routes::auth::delete_device,
        routes::auth::list_devices,
        routes::sync::get_slots,
        routes::sync::update_slot,
        routes::sync::get_history,
        routes::sync::push_history,
        routes::sync::delete_history,
    ),
    components(schemas(
        models::user::RegisterRequest,
        models::user::LoginRequest,
        models::user::AuthResponse,
        models::device::RegisterDeviceRequest,
        models::device::DeviceResponse,
        models::sync::SlotResponse,
        models::sync::UpdateSlotRequest,
        models::sync::PushHistoryRequest,
        models::sync::HistoryResponse,
        models::sync::HistoryQuery,
    )),
    modifiers(&SecurityAddon),
    tags(
        (name = "Auth", description = "Authentication & device management"),
        (name = "Sync", description = "Encrypted clipboard sync (slots & history)")
    ),
    security(("bearer" = []))
)]
struct ApiDoc;

struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "bearer",
            utoipa::openapi::security::SecurityScheme::Http(
                utoipa::openapi::security::Http::new(
                    utoipa::openapi::security::HttpAuthScheme::Bearer,
                ),
            ),
        );
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("clipslot_server=debug,tower_http=debug")
        .init();

    let config = config::Config::from_env();

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await
        .expect("Failed to connect to database");

    sqlx::migrate!("./src/db/migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let cors = if config.cors_origins == "*" {
        CorsLayer::permissive()
    } else {
        let origins: Vec<HeaderValue> = config
            .cors_origins
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();
        CorsLayer::new()
            .allow_origin(origins)
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::DELETE,
                Method::OPTIONS,
            ])
            .allow_headers(tower_http::cors::Any)
            .allow_credentials(true)
    };

    let state = AppState {
        db: pool,
        jwt_secret: config.jwt_secret,
        user_channels: Arc::new(DashMap::new()),
    };

    let app = routes::api_router(state)
        .merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(&config.listen_addr)
        .await
        .unwrap();
    tracing::info!("Listening on {}", config.listen_addr);
    tracing::info!("Swagger UI at http://{}/docs/", config.listen_addr);
    axum::serve(listener, app).await.unwrap();
}
