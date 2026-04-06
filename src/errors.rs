use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("not_found")]
    NotFound(String),

    #[error("unauthorized")]
    Unauthorized(String),

    #[error("internal_error")]
    InternalError(String),

    #[error("invalid_input")]
    InvalidInput(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_code, message) = match &self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, "not_found", msg.clone()),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, "unauthorized", msg.clone()),
            AppError::InternalError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                msg.clone(),
            ),
            AppError::InvalidInput(msg) => (StatusCode::BAD_REQUEST, "invalid_input", msg.clone()),
        };

        (
            status,
            Json(serde_json::json!({
                "error": error_code,
                "message": message,
                "code": status.as_u16()
            })),
        )
            .into_response()
    }
}
