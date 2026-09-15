use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub listen_addr: String,
    pub admin_username: String,
    pub admin_password: String,
    pub cors_origin: String,
    /// gaode | embedded | opensource；空则：有 AMAP_WEB_KEY → gaode，否则 embedded
    pub routing_provider: Option<String>,
    pub amap_web_key: Option<String>,
    pub amap_js_key: Option<String>,
    pub amap_security_js_code: Option<String>,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:///workspace/map-stop/data/map-stop.db".into()),
            jwt_secret: env::var("JWT_SECRET")
                .unwrap_or_else(|_| "dev-secret-change-me-in-production".into()),
            listen_addr: env::var("LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".into()),
            admin_username: env::var("ADMIN_USERNAME").unwrap_or_else(|_| "admin".into()),
            admin_password: env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "admin123".into()),
            cors_origin: env::var("CORS_ORIGIN").unwrap_or_else(|_| "http://localhost:5173".into()),
            routing_provider: env::var("ROUTING_PROVIDER").ok().filter(|s| !s.trim().is_empty()),
            amap_web_key: env::var("AMAP_WEB_KEY").ok(),
            amap_js_key: env::var("AMAP_JS_KEY").ok(),
            amap_security_js_code: env::var("AMAP_SECURITY_JS_CODE").ok(),
        }
    }
}
