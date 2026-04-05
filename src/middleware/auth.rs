use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use serde_json::json;
use sqlx::Row;
use uuid::Uuid;

use crate::db::DbPool;

/// Authenticated session extracted from the `Authorization: Bearer <token>` header.
#[derive(Debug, Clone)]
pub struct Session {
    pub user_id: Uuid,
    pub token: String,
}

/// Axum extractor: validates the bearer token against the sessions table.
#[axum::async_trait]
impl FromRequestParts<DbPool> for Session {
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, db: &DbPool) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(json!({ "error": "missing authorization header" })),
                )
                    .into_response()
            })?
            .to_string();

        let row = sqlx::query(
            "SELECT user_id, expires_at FROM sessions WHERE token = $1 LIMIT 1",
        )
        .bind(&token)
        .fetch_optional(db)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "database error" })),
            )
                .into_response()
        })?
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "invalid or expired token" })),
            )
                .into_response()
        })?;

        let expires_at: DateTime<Utc> = row.try_get("expires_at").map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "database error" })),
            )
                .into_response()
        })?;

        if expires_at < Utc::now() {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "session expired" })),
            )
                .into_response());
        }

        let user_id: Uuid = row.try_get("user_id").map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "database error" })),
            )
                .into_response()
        })?;

        Ok(Session { user_id, token })
    }
}
