use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect},
    routing::get,
    Router,
};
use axum_extra::extract::cookie::{Cookie, SignedCookieJar};
use serde::Deserialize;
use uuid::Uuid;

use crate::{errors::AppError, models::user::User, AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/google", get(google_login))
        .route("/google/callback", get(google_callback))
        .route("/logout", get(logout))
        .route("/me", get(me))
}

/// Step 1: Redirect user to Google
async fn google_login(
    State(state): State<AppState>,
    jar: SignedCookieJar,
) -> impl IntoResponse {
    let (auth_url, csrf_token) = state.oauth.auth_url();

    // Store CSRF token in a short-lived cookie to verify on callback
    let csrf_cookie = Cookie::build(("oauth_csrf", csrf_token.secret().clone()))
        .http_only(true)
        .max_age(time::Duration::minutes(10))
        .build();

    (jar.add(csrf_cookie), Redirect::to(&auth_url))
}

#[derive(Deserialize)]
struct CallbackParams {
    code: String,
    state: String, // CSRF token from Google
}

/// Step 2: Google redirects back here with a code
async fn google_callback(
    State(state): State<AppState>,
    Query(params): Query<CallbackParams>,
    jar: SignedCookieJar,
) -> impl IntoResponse {
    // Verify CSRF
    let csrf_cookie = jar.get("oauth_csrf");
    if csrf_cookie.map(|c| c.value().to_string()) != Some(params.state.clone()) {
        return (
            jar,
            Redirect::to("/?error=csrf_mismatch"),
        );
    }

    // Exchange code for user info
    let google_user = match state.oauth.exchange_code(params.code).await {
        Ok(u) => u,
        Err(e) => {
            tracing::error!("OAuth exchange failed: {}", e);
            return (jar, Redirect::to("/?error=oauth_failed"));
        }
    };

    // Upsert user in DB
    let user = User::upsert_from_google(&state.db, &google_user).await;
    if let Err(e) = &user {
        tracing::error!("DB upsert failed: {}", e);
        return (jar, Redirect::to("/?error=db_error"));
    }
    let user = user.unwrap();

    // Create session cookie with user ID
    let session_cookie = Cookie::build(("session_user_id", user.id.to_string()))
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .max_age(time::Duration::days(30))
        .path("/")
        .build();

    let jar = jar.remove(Cookie::from("oauth_csrf")).add(session_cookie);
    (jar, Redirect::to("/dashboard"))
}

/// Logout: clear session cookie
async fn logout(jar: SignedCookieJar) -> impl IntoResponse {
    let jar = jar.remove(Cookie::from("session_user_id"));
    (jar, Redirect::to("/"))
}

/// GET /auth/me — returns current user info as JSON
async fn me(
    State(state): State<AppState>,
    jar: SignedCookieJar,
) -> Result<axum::Json<serde_json::Value>, AppError> {
    let user_id = jar
        .get("session_user_id")
        .and_then(|c| Uuid::parse_str(c.value()).ok())
        .ok_or_else(|| AppError::Unauthorized("Not authenticated".into()))?;

    let user = User::find_by_id(&state.db, user_id)
        .await
        .map_err(|e| AppError::InternalError(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("User not found".into()))?;

    Ok(axum::Json(serde_json::json!({
        "id": user.id,
        "email": user.email,
        "name": user.name,
        "avatar": user.avatar_url,
    })))
}
