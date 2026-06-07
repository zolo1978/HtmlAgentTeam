use std::sync::Arc;
use axum::{Router, routing::get, Json};
use tower_http::cors::{CorsLayer, Any};
use tower_http::trace::TraceLayer;
use crate::state::AppState;
use crate::error::ApiResponse;

pub mod auth;
pub mod agents;
pub mod tasks;
pub mod skills;
pub mod memories;
pub mod reflection;
pub mod growth;
pub mod cost;

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
        // Auth
        .nest("/api/v1/auth", auth::router(state.clone()))
        // Agents
        .nest("/api/v1/agents", agents::router(state.clone()))
        // Tasks
        .nest("/api/v1/tasks", tasks::router(state.clone()))
        // Skills
        .nest("/api/v1/skills", skills::router(state.clone()))
        // Memories
        .nest("/api/v1/memories", memories::router(state.clone()))
        // Reflection
        .nest("/api/v1/reflection", reflection::router(state.clone()))
        // Growth
        .nest("/api/v1/growth", growth::router(state.clone()))
        // Cost
        .nest("/api/v1/cost", cost::router(state.clone()))
        // Middleware
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(Any) // TODO: 生产环境改为具体域名
                .allow_methods(Any)
                .allow_headers(Any),
        )
}

async fn health() -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::success(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
    })))
}
