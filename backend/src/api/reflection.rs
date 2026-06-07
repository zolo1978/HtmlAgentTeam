use std::sync::Arc;
use axum::Router;
use crate::state::AppState;

pub fn router(_state: Arc<AppState>) -> Router {
    Router::new()
    // TODO: implement reflection endpoints
}
