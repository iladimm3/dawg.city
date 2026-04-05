use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::json;

use crate::db::DbPool;
use crate::middleware::auth::Session;
use crate::models::Game;

// ── GET /api/games ─────────────────────────────────────────────────────────
#[derive(Deserialize)]
pub struct GamesQuery {
    pub category: Option<String>,
    pub search: Option<String>,
    pub featured: Option<bool>,
}

pub async fn list(State(db): State<DbPool>, Query(q): Query<GamesQuery>) -> Response {
    // Build a dynamic query with up to two optional filters.
    let mut sql = String::from(
        "SELECT id, slug, title, description, category, embed_url, thumbnail, tags, \
         play_count, is_featured, is_active, added_at \
         FROM games WHERE is_active = true",
    );

    let mut category_bind: Option<String> = None;
    let mut search_bind: Option<String> = None;

    if let Some(cat) = q.category {
        category_bind = Some(cat);
        sql.push_str(" AND category = $1");
    }
    if let Some(search) = q.search {
        search_bind = Some(format!("%{}%", search.to_lowercase()));
        let n = if category_bind.is_some() { 2 } else { 1 };
        sql.push_str(&format!(
            " AND (lower(title) LIKE ${n} OR lower(description) LIKE ${n})"
        ));
    }
    if let Some(true) = q.featured {
        sql.push_str(" AND is_featured = true");
    }
    sql.push_str(" ORDER BY is_featured DESC, play_count DESC, added_at DESC");

    let mut qb = sqlx::query_as::<_, Game>(&sql);
    if let Some(ref cat) = category_bind {
        qb = qb.bind(cat);
    }
    if let Some(ref s) = search_bind {
        qb = qb.bind(s);
    }

    match qb.fetch_all(&db).await {
        Ok(games) => Json(games).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "database error" })),
        )
            .into_response(),
    }
}

// ── GET /api/games/:slug ───────────────────────────────────────────────────
pub async fn get_game(State(db): State<DbPool>, Path(slug): Path<String>) -> Response {
    match sqlx::query_as::<_, Game>(
        "SELECT * FROM games WHERE slug = $1 AND is_active = true",
    )
    .bind(&slug)
    .fetch_optional(&db)
    .await
    {
        Ok(Some(game)) => Json(game).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "game not found" })),
        )
            .into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "database error" })),
        )
            .into_response(),
    }
}

// ── POST /api/games/:slug/ping ─────────────────────────────────────────────
// Playtime heartbeat — awards 5 coins + 10 XP per ping, capped at 30/day/game.
pub async fn ping(
    State(db): State<DbPool>,
    session: Session,
    Path(slug): Path<String>,
) -> Response {
    use sqlx::Row;

    // Resolve game id
    let game_id: uuid::Uuid = match sqlx::query(
        "SELECT id FROM games WHERE slug = $1 AND is_active = true",
    )
    .bind(&slug)
    .fetch_optional(&db)
    .await
    {
        Ok(Some(r)) => r.try_get("id").unwrap(),
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "game not found" })),
            )
                .into_response()
        }
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "database error" })),
            )
                .into_response()
        }
    };

    // Count today's pings for this user + game
    let ping_count: i64 = sqlx::query_scalar::<_, i64>(
        r#"SELECT COUNT(*) FROM playtime_pings
           WHERE user_id = $1 AND game_id = $2
             AND pinged_at > now() - INTERVAL '1 day'"#,
    )
    .bind(session.user_id)
    .bind(game_id)
    .fetch_one(&db)
    .await
    .unwrap_or(0);

    if ping_count >= 30 {
        return (
            StatusCode::OK,
            Json(json!({ "awarded": false, "reason": "daily cap reached" })),
        )
            .into_response();
    }

    // Record ping
    if sqlx::query("INSERT INTO playtime_pings (user_id, game_id) VALUES ($1, $2)")
        .bind(session.user_id)
        .bind(game_id)
        .execute(&db)
        .await
        .is_err()
    {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "database error" })),
        )
            .into_response();
    }

    // Award coins via ledger (trigger keeps users.coins in sync)
    let _ = sqlx::query(
        "INSERT INTO coin_transactions (user_id, amount, reason, meta) VALUES ($1, 5, 'playtime', $2)",
    )
    .bind(session.user_id)
    .bind(serde_json::json!({ "game": slug }))
    .execute(&db)
    .await;

    // Award XP
    let _ = sqlx::query("UPDATE users SET xp = xp + 10 WHERE id = $1")
        .bind(session.user_id)
        .execute(&db)
        .await;

    // Increment play count
    let _ = sqlx::query("UPDATE games SET play_count = play_count + 1 WHERE id = $1")
        .bind(game_id)
        .execute(&db)
        .await;

    (
        StatusCode::OK,
        Json(json!({ "awarded": true, "coins": 5, "xp": 10 })),
    )
        .into_response()
}
