use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    Json,
};
use chrono::{Duration, Utc};
use rand::Rng;
use serde::Deserialize;
use serde_json::json;
use sqlx::Row;

use crate::db::DbPool;

// ── Discord OAuth constants ────────────────────────────────────────────────

fn discord_client_id() -> String {
    std::env::var("DISCORD_CLIENT_ID").expect("DISCORD_CLIENT_ID must be set")
}
fn discord_client_secret() -> String {
    std::env::var("DISCORD_CLIENT_SECRET").expect("DISCORD_CLIENT_SECRET must be set")
}
fn discord_redirect_uri() -> String {
    std::env::var("DISCORD_REDIRECT_URI")
        .unwrap_or_else(|_| "https://api.dawg.city/api/auth/callback".to_string())
}
fn frontend_url() -> String {
    std::env::var("FRONTEND_URL").unwrap_or_else(|_| "https://dawg.city".to_string())
}

// ── GET /api/auth/discord ──────────────────────────────────────────────────
/// Redirects the browser to Discord's OAuth authorisation page.
pub async fn login() -> impl IntoResponse {
    let url = format!(
        "https://discord.com/api/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope=identify",
        discord_client_id(),
        urlencoded(&discord_redirect_uri()),
    );
    Redirect::temporary(&url)
}

// ── GET /api/auth/callback ─────────────────────────────────────────────────
#[derive(Deserialize)]
pub struct CallbackParams {
    code: Option<String>,
    error: Option<String>,
}

#[derive(Deserialize)]
struct DiscordTokenResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct DiscordUser {
    id: String,
    username: String,
    avatar: Option<String>,
}

pub async fn callback(
    State(db): State<DbPool>,
    Query(params): Query<CallbackParams>,
) -> Response {
    if let Some(err) = params.error {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": err }))).into_response();
    }

    let code = match params.code {
        Some(c) => c,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "missing code" })),
            )
                .into_response()
        }
    };

    let client = reqwest::Client::new();

    // Exchange code for access token
    let token_resp = match client
        .post("https://discord.com/api/oauth2/token")
        .form(&[
            ("client_id", discord_client_id()),
            ("client_secret", discord_client_secret()),
            ("grant_type", "authorization_code".to_string()),
            ("code", code),
            ("redirect_uri", discord_redirect_uri()),
        ])
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(json!({ "error": "discord token exchange failed" })),
            )
                .into_response()
        }
    };

    let token_data: DiscordTokenResponse = match token_resp.json().await {
        Ok(t) => t,
        Err(_) => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(json!({ "error": "failed to parse discord token response" })),
            )
                .into_response()
        }
    };

    // Fetch user profile from Discord
    let discord_user: DiscordUser = match client
        .get("https://discord.com/api/users/@me")
        .bearer_auth(&token_data.access_token)
        .send()
        .await
    {
        Ok(r) => match r.json().await {
            Ok(u) => u,
            Err(_) => {
                return (
                    StatusCode::BAD_GATEWAY,
                    Json(json!({ "error": "failed to parse discord user" })),
                )
                    .into_response()
            }
        },
        Err(_) => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(json!({ "error": "discord user fetch failed" })),
            )
                .into_response()
        }
    };

    let avatar_url = discord_user.avatar.as_ref().map(|hash| {
        format!(
            "https://cdn.discordapp.com/avatars/{}/{}.png",
            discord_user.id, hash
        )
    });

    // Upsert user
    let row = match sqlx::query(
        r#"
        INSERT INTO users (discord_id, username, avatar_url)
        VALUES ($1, $2, $3)
        ON CONFLICT (discord_id) DO UPDATE
          SET username   = EXCLUDED.username,
              avatar_url = EXCLUDED.avatar_url
        RETURNING id
        "#,
    )
    .bind(&discord_user.id)
    .bind(&discord_user.username)
    .bind(&avatar_url)
    .fetch_one(&db)
    .await
    {
        Ok(r) => r,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "database error" })),
            )
                .into_response()
        }
    };

    let user_id: uuid::Uuid = row.try_get("id").unwrap();

    // Update streak (best-effort)
    let _ = sqlx::query("SELECT update_streak($1)")
        .bind(user_id)
        .execute(&db)
        .await;

    // Issue session token (30-day expiry)
    let token = generate_token();
    let expires_at = Utc::now() + Duration::days(30);

    if sqlx::query("INSERT INTO sessions (user_id, token, expires_at) VALUES ($1, $2, $3)")
        .bind(user_id)
        .bind(&token)
        .bind(expires_at)
        .execute(&db)
        .await
        .is_err()
    {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "failed to create session" })),
        )
            .into_response();
    }

    // Redirect to frontend; JS reads the token from the URL fragment and
    // stores it in localStorage. It never appears in server logs.
    let redirect_url = format!("{}/?token={}", frontend_url(), token);
    Redirect::temporary(&redirect_url).into_response()
}

// ── GET /api/me ────────────────────────────────────────────────────────────
pub async fn me(
    State(db): State<DbPool>,
    session: crate::middleware::auth::Session,
) -> Response {
    match sqlx::query_as::<_, crate::models::User>(
        "SELECT * FROM users WHERE id = $1",
    )
    .bind(session.user_id)
    .fetch_optional(&db)
    .await
    {
        Ok(Some(user)) => Json(user).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "user not found" })),
        )
            .into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "database error" })),
        )
            .into_response(),
    }
}

// ── GET /api/mirrors ───────────────────────────────────────────────────────
pub async fn mirrors(State(db): State<DbPool>) -> Response {
    match sqlx::query_as::<_, crate::models::Mirror>(
        "SELECT * FROM mirrors WHERE is_active = true ORDER BY added_at DESC",
    )
    .fetch_all(&db)
    .await
    {
        Ok(rows) => Json(rows).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "database error" })),
        )
            .into_response(),
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────

fn generate_token() -> String {
    let bytes: [u8; 32] = rand::thread_rng().gen();
    hex::encode(bytes)
}

fn urlencoded(s: &str) -> String {
    s.replace(':', "%3A").replace('/', "%2F")
}
