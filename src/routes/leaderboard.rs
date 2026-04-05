use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::json;
use sqlx::Row;
use uuid::Uuid;

use crate::db::DbPool;
use crate::middleware::auth::Session;

// ── GET /api/games/:slug/leaderboard ──────────────────────────────────────
pub async fn get_leaderboard(State(db): State<DbPool>, Path(slug): Path<String>) -> Response {
    let game_id = match resolve_game_id(&db, &slug).await {
        Some(id) => id,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "game not found" })),
            )
                .into_response()
        }
    };

    #[derive(serde::Serialize, sqlx::FromRow)]
    struct Row {
        rank: Option<i64>,
        username: String,
        score: i64,
        posted_at: chrono::DateTime<chrono::Utc>,
    }

    match sqlx::query_as::<_, Row>(
        r#"
        SELECT
            ROW_NUMBER() OVER (ORDER BY l.score DESC) AS rank,
            u.username,
            l.score,
            l.posted_at
        FROM leaderboard l
        JOIN users u ON u.id = l.user_id
        WHERE l.game_id = $1
        ORDER BY l.score DESC
        LIMIT 100
        "#,
    )
    .bind(game_id)
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

// ── POST /api/games/:slug/score ────────────────────────────────────────────
#[derive(Deserialize)]
pub struct ScoreBody {
    pub score: i64,
}

pub async fn submit_score(
    State(db): State<DbPool>,
    session: Session,
    Path(slug): Path<String>,
    Json(body): Json<ScoreBody>,
) -> Response {
    if body.score < 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "score must be non-negative" })),
        )
            .into_response();
    }

    let game_id = match resolve_game_id(&db, &slug).await {
        Some(id) => id,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "game not found" })),
            )
                .into_response()
        }
    };

    // Upsert: only keep the higher score
    let row = match sqlx::query(
        r#"
        INSERT INTO leaderboard (game_id, user_id, score)
        VALUES ($1, $2, $3)
        ON CONFLICT (game_id, user_id) DO UPDATE
          SET score     = GREATEST(leaderboard.score, EXCLUDED.score),
              posted_at = CASE WHEN EXCLUDED.score > leaderboard.score
                               THEN now() ELSE leaderboard.posted_at END
        RETURNING score
        "#,
    )
    .bind(game_id)
    .bind(session.user_id)
    .bind(body.score)
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

    let stored_score: i64 = row.try_get("score").unwrap_or(0);
    let is_new_best = stored_score == body.score;

    award_score_coins(&db, session.user_id, &slug, is_new_best).await;

    Json(json!({
        "recorded": true,
        "new_best": is_new_best,
        "score": stored_score,
    }))
    .into_response()
}

// ── Helpers ────────────────────────────────────────────────────────────────

async fn resolve_game_id(db: &DbPool, slug: &str) -> Option<Uuid> {
    sqlx::query("SELECT id FROM games WHERE slug = $1 AND is_active = true")
        .bind(slug)
        .fetch_optional(db)
        .await
        .ok()
        .flatten()
        .and_then(|r| r.try_get("id").ok())
}

async fn award_score_coins(db: &DbPool, user_id: Uuid, slug: &str, is_new_best: bool) {
    // Only award once per game per day
    let already: i64 = sqlx::query_scalar::<_, i64>(
        r#"SELECT COUNT(*) FROM coin_transactions
           WHERE user_id = $1
             AND reason IN ('score_post', 'personal_best')
             AND meta ->> 'game' = $2
             AND created_at > now() - INTERVAL '1 day'"#,
    )
    .bind(user_id)
    .bind(slug)
    .fetch_one(db)
    .await
    .unwrap_or(0);

    if already > 0 {
        return;
    }

    let (coins, xp, reason) = if is_new_best {
        (25_i32, 50_i32, "personal_best")
    } else {
        (15_i32, 30_i32, "score_post")
    };

    let _ = sqlx::query(
        "INSERT INTO coin_transactions (user_id, amount, reason, meta) VALUES ($1, $2, $3, $4)",
    )
    .bind(user_id)
    .bind(coins)
    .bind(reason)
    .bind(serde_json::json!({ "game": slug }))
    .execute(db)
    .await;

    let _ = sqlx::query("UPDATE users SET xp = xp + $1 WHERE id = $2")
        .bind(xp)
        .bind(user_id)
        .execute(db)
        .await;
}
