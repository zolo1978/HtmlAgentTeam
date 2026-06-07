use std::sync::Arc;
use sqlx::PgPool;

/// 全局应用状态，注入所有 Axum handlers
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Arc<AppConfig>,
}

pub struct AppConfig {
    pub database_url: String,
    pub redis_url: String,
    pub nats_url: String,
    pub jwt_secret: String,
    pub jwt_access_ttl_minutes: u64,
    pub jwt_refresh_ttl_days: u64,
    pub openai_api_key: String,
    pub anthropic_api_key: String,
    pub semantic_cache_threshold: f32,
    pub memory_injection_budget_tokens: u32,
    pub port: u16,
    pub frontend_url: String,
}

impl AppState {
    pub async fn from_env() -> anyhow::Result<Arc<Self>> {
        let config = AppConfig {
            database_url: std::env::var("DATABASE_URL")?,
            redis_url: std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".into()),
            nats_url: std::env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".into()),
            jwt_secret: std::env::var("JWT_SECRET")?,
            jwt_access_ttl_minutes: std::env::var("JWT_ACCESS_TTL_MINUTES")
                .ok().and_then(|v| v.parse().ok()).unwrap_or(15),
            jwt_refresh_ttl_days: std::env::var("JWT_REFRESH_TTL_DAYS")
                .ok().and_then(|v| v.parse().ok()).unwrap_or(7),
            openai_api_key: std::env::var("OPENAI_API_KEY").unwrap_or_default(),
            anthropic_api_key: std::env::var("ANTHROPIC_API_KEY").unwrap_or_default(),
            semantic_cache_threshold: std::env::var("SEMANTIC_CACHE_THRESHOLD")
                .ok().and_then(|v| v.parse().ok()).unwrap_or(0.92),
            memory_injection_budget_tokens: std::env::var("MEMORY_INJECTION_BUDGET_TOKENS")
                .ok().and_then(|v| v.parse().ok()).unwrap_or(2000),
            port: std::env::var("PORT").ok().and_then(|v| v.parse().ok()).unwrap_or(8080),
            frontend_url: std::env::var("FRONTEND_URL")
                .unwrap_or_else(|_| "http://localhost:3000".into()),
        };

        let db = sqlx::PgPool::connect(&config.database_url).await?;

        Ok(Arc::new(Self { db, config: Arc::new(config) }))
    }
}
