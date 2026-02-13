pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub listen_addr: String,
    /// Comma-separated allowed CORS origins. If empty or "*", allows all origins (dev mode).
    pub cors_origins: String,
}

impl Config {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();
        Self {
            database_url: std::env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            jwt_secret: std::env::var("JWT_SECRET").expect("JWT_SECRET must be set"),
            listen_addr: std::env::var("LISTEN_ADDR")
                .unwrap_or_else(|_| "0.0.0.0:3000".to_string()),
            cors_origins: std::env::var("CORS_ORIGINS").unwrap_or_else(|_| "*".to_string()),
        }
    }
}
