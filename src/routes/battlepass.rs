use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use sqlx::Row;

use crate::db::DbPool;
use crate::middleware::auth::Session;
use crate::models::{BattlePassSeason, BattlePassTier, UserBattlePass};

// ── GET /api/battlepass ────────────────────────────────────────────────────
pub async fn status(State(db): State<DbPool>, session: Session) -> Response {
    let season = match sqlx::query_as::<_, BattlePassSeason>(
        "SELECT * FROM battle_pass_seasons WHERE is_active = true LIMIT 1",
    )
    .fetch_optional(&db)
    .await
    {
        Ok(Some(s)) => s,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "no active season" })),
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

    let tiers = sqlx::query_as::<_, BattlePassTier>(
        "SELECT * FROM battle_pass_tiers WHERE season_id = $1 ORDER BY tier ASC",
    )
    .bind(season.id)
    .fetch_all(&db)
    .await
    .unwrap_or_default();

    let progress: Option<UserBattlePass> = sqlx::query_as::<_, UserBattlePass>(
        "SELECT * FROM user_battle_pass WHERE user_id = $1 AND season_id = $2",
    )
    .bind(session.user_id)
    .bind(season.id)
    .fetch_optional(&db)
    .await
    .unwrap_or(None);

    let xp: i32 = sqlx::query("SELECT xp FROM users WHERE id = $1")
        .bind(session.user_id)
        .fetch_optional(&db)
        .await
        .ok()
        .flatten()
        .and_then(|r| r.try_get::<i32, _>("xp").ok())
        .unwrap_or(0);

    let claimed: Vec<i32> = sqlx::query_scalar::<_, i32>(
        "SELECT tier FROM claimed_tiers WHERE user_id = $1 AND season_id = $2",
    )
    .bind(session.user_id)
    .bind(season.id)
    .fetch_all(&db)
    .await
    .unwrap_or_default();

    Json(json!({
        "season": season,
        "tiers": tiers,
        "progress": progress,
        "xp": xp,
        "claimed_tiers": claimed,
    }))
    .into_response()
}

// ── POST /api/battlepass/claim/:tier ──────────────────────────────────────
pub async fn claim(
    State(db): State<DbPool>,
    session: Session,
    Path(tier_num): Path<i32>,
) -> Response {
    if !(1..=30).contains(&tier_num) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "tier must be between 1 and 30" })),
        )
            .into_response();
    }

    // Active season
    let season_id: uuid::Uuid = match sqlx::query(
        "SELECT id FROM battle_pass_seasons WHERE is_active = true LIMIT 1",
    )
    .fetch_optional(&db)
    .await
    {
        Ok(Some(r)) => r.try_get("id").unwrap(),
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "no active season" })),
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

    // Tier definition
    let tier = match sqlx::query_as::<_, BattlePassTier>(
        "SELECT * FROM battle_pass_tiers WHERE season_id = $1 AND tier = $2",
    )
    .bind(season_id)
    .bind(tier_num)
    .fetch_optional(&db)
    .await
    {
        Ok(Some(t)) => t,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "tier not found" })),
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

    // Check user's XP
    let xp: i32 = sqlx::query("SELECT xp FROM users WHERE id = $1")
        .bind(session.user_id)
        .fetch_optional(&db)
        .await
        .ok()
        .flatten()
        .and_then(|r| r.try_get::<i32, _>("xp").ok())
        .unwrap_or(0);

    if xp < tier.xp_required {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "error": "not enough XP",
                "required": tier.xp_required,
                "have": xp,
            })),
        )
            .into_response();
    }

    // Premium gate
    if tier.is_premium {
        let is_premium: bool = sqlx::query(
            "SELECT is_premium FROM user_battle_pass WHERE user_id = $1 AND season_id = $2",
        )
        .bind(session.user_id)
        .bind(season_id)
        .fetch_optional(&db)
        .await
        .ok()
        .flatten()
        .and_then(|r| r.try_get::<bool, _>("is_premium").ok())
        .unwrap_or(false);

        if !is_premium {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({ "error": "premium pass required for this tier" })),
            )
                .into_response();
        }
    }

    // Mark as claimed (idempotent)
    let rows_affected = sqlx::query(
        "INSERT INTO claimed_tiers (user_id, season_id, tier) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
    )
    .bind(session.user_id)
    .bind(season_id)
    .bind(tier_num)
    .execute(&db)
    .await
    .map(|r| r.rows_affected())
    .unwrap_or(0);

    if rows_affected == 0 {
        return (
            StatusCode::CONFLICT,
            Json(json!({ "error": "tier already claimed" })),
        )
            .into_response();
    }

    // Grant coin reward automatically
    if tier.reward_type == "coins" {
        let amount = tier
            .reward_meta
            .as_object()
            .and_then(|o| o.get("amount"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;

        if amount > 0 {
            let _ = sqlx::query(
                "INSERT INTO coin_transactions (user_id, amount, reason, meta) VALUES ($1, $2, 'battle_pass', $3)",
            )
            .bind(session.user_id)
            .bind(amount)
            .bind(serde_json::json!({ "tier": tier_num }))
            .execute(&db)
            .await;
        }
    }

    Json(json!({
        "claimed": true,
        "tier": tier_num,
        "reward_type": tier.reward_type,
        "reward_meta": tier.reward_meta,
    }))
    .into_response()
}
