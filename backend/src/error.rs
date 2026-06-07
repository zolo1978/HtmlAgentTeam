use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use uuid::Uuid;
use chrono::Utc;

/// 请求元信息
#[derive(Serialize)]
pub struct ApiMeta {
    pub request_id: Uuid,
    pub timestamp: i64,
    pub page: Option<PageMeta>,
}

#[derive(Serialize)]
pub struct PageMeta {
    pub next_cursor: Option<String>,
    pub has_more: bool,
    pub total: Option<i64>,
}

/// 成功响应信封
#[derive(Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    pub data: T,
    pub meta: ApiMeta,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data,
            meta: ApiMeta {
                request_id: Uuid::now_v7(),
                timestamp: Utc::now().timestamp_millis(),
                page: None,
            },
        }
    }

    pub fn paged(data: T, next_cursor: Option<String>, has_more: bool) -> Self {
        Self {
            success: true,
            data,
            meta: ApiMeta {
                request_id: Uuid::now_v7(),
                timestamp: Utc::now().timestamp_millis(),
                page: Some(PageMeta { next_cursor, has_more, total: None }),
            },
        }
    }
}

/// 字段级错误
#[derive(Debug, Serialize)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

/// 统一 AppError 枚举
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    // 4xx
    #[error("not found: {resource}")]
    NotFound { resource: String },
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("token expired")]
    TokenExpired,
    #[error("token invalid")]
    TokenInvalid,
    #[error("unauthorized: {action}")]
    Unauthorized { action: String },
    #[error("forbidden")]
    Forbidden,
    #[error("validation failed")]
    ValidationFailed { fields: Vec<FieldError> },
    #[error("invalid state transition: {from} -> {to}")]
    InvalidStateTransition { from: String, to: String },
    #[error("rate limit exceeded")]
    RateLimitExceeded,
    #[error("conflict: {resource}")]
    Conflict { resource: String },
    // 5xx
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("llm provider error: {provider}")]
    LlmProvider { provider: String, message: String },
    #[error("prolog engine error")]
    PrologEngine(String),
    #[error("event bus error")]
    EventBus(String),
    #[error("internal error")]
    Internal(#[from] anyhow::Error),
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            AppError::NotFound { .. }             => "E4040",
            AppError::InvalidCredentials          => "E4010",
            AppError::TokenExpired                => "E4011",
            AppError::TokenInvalid                => "E4012",
            AppError::Unauthorized { .. }         => "E4013",
            AppError::Forbidden                   => "E4030",
            AppError::ValidationFailed { .. }     => "E4220",
            AppError::InvalidStateTransition { .. }=> "E4221",
            AppError::RateLimitExceeded           => "E4290",
            AppError::Conflict { .. }             => "E4090",
            AppError::Database(_)                 => "E5000",
            AppError::LlmProvider { .. }          => "E5010",
            AppError::PrologEngine(_)             => "E5020",
            AppError::EventBus(_)                 => "E5030",
            AppError::Internal(_)                 => "E5040",
        }
    }

    pub fn http_status(&self) -> StatusCode {
        match self {
            AppError::NotFound { .. }             => StatusCode::NOT_FOUND,
            AppError::InvalidCredentials          => StatusCode::UNAUTHORIZED,
            AppError::TokenExpired                => StatusCode::UNAUTHORIZED,
            AppError::TokenInvalid                => StatusCode::UNAUTHORIZED,
            AppError::Unauthorized { .. }         => StatusCode::UNAUTHORIZED,
            AppError::Forbidden                   => StatusCode::FORBIDDEN,
            AppError::ValidationFailed { .. }     => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::InvalidStateTransition { .. }=> StatusCode::UNPROCESSABLE_ENTITY,
            AppError::RateLimitExceeded           => StatusCode::TOO_MANY_REQUESTS,
            AppError::Conflict { .. }             => StatusCode::CONFLICT,
            _                                     => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[derive(Serialize)]
struct ErrorBody {
    code: &'static str,
    message: String,
    details: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct ApiError {
    success: bool,
    error: ErrorBody,
    meta: ApiMeta,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.http_status();
        let details = match &self {
            AppError::ValidationFailed { fields } =>
                Some(serde_json::to_value(fields).unwrap_or_default()),
            AppError::InvalidStateTransition { from, to } =>
                Some(serde_json::json!({ "from": from, "to": to })),
            AppError::LlmProvider { provider, message } =>
                Some(serde_json::json!({ "provider": provider, "message": message })),
            _ => None,
        };
        let body = ApiError {
            success: false,
            error: ErrorBody {
                code: self.code(),
                message: self.to_string(),
                details,
            },
            meta: ApiMeta {
                request_id: Uuid::now_v7(),
                timestamp: Utc::now().timestamp_millis(),
                page: None,
            },
        };
        (status, Json(body)).into_response()
    }
}
