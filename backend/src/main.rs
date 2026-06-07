use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod api;
mod agent;
mod auth;
mod db;
mod error;
mod event;
mod growth;
mod llm;
mod memory;
mod prolog;
mod reflection;
mod skill;
mod sop;
mod state;
mod task;

pub use error::AppError;
pub use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "harp_backend=debug,tower_http=debug".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("🚀 HARP Backend starting...");

    // 初始化 AppState
    let state = AppState::from_env().await?;

    // 运行 DB 迁移
    db::migrate(&state.db).await?;

    // 构建路由
    let app = api::router(state.clone());

    // 启动服务器
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr: SocketAddr = format!("0.0.0.0:{port}").parse()?;
    tracing::info!("Listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
