use axum::{
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Response},
};
use axum_extra::extract::cookie::SignedCookieJar;
use uuid::Uuid;

use crate::{models::user::User, AppState};

/// Middleware that injects the current user into request extensions.
/// Returns 401 if the session cookie is missing or invalid.
pub async fn require_auth(
    State(state): State<AppState>,
    jar: SignedCookieJar,
    mut req: Request,
    next: Next,
) -> Response {
    let user_id = jar
        .get("session_user_id")
        .and_then(|c| Uuid::parse_str(c.value()).ok());

    match user_id {
        Some(id) => match User::find_by_id(&state.db, id).await {
            Ok(Some(user)) => {
                req.extensions_mut().insert(user);
                next.run(req).await
            }
            _ => unauthorized(),
        },
        None => unauthorized(),
    }
}

fn unauthorized() -> Response {
    (
        axum::http::StatusCode::UNAUTHORIZED,
        axum::Json(serde_json::json!({"error": "Authentication required"})),
    )
        .into_response()
}
