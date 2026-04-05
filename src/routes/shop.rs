use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use sqlx::Row;
use uuid::Uuid;

use crate::db::DbPool;
use crate::middleware::auth::Session;
use crate::models::ShopItem;

// ── GET /api/shop ──────────────────────────────────────────────────────────
pub async fn list(State(db): State<DbPool>) -> Response {
    match sqlx::query_as::<_, ShopItem>(
        r#"SELECT id, name, description, type AS item_type, cost_coins,
                  image_url, is_active, stock, created_at
           FROM shop_items
           WHERE is_active = true
           ORDER BY cost_coins ASC"#,
    )
    .fetch_all(&db)
    .await
    {
        Ok(items) => Json(items).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "database error" })),
        )
            .into_response(),
    }
}

// ── POST /api/shop/buy/:item_id ────────────────────────────────────────────
pub async fn buy(
    State(db): State<DbPool>,
    session: Session,
    Path(item_id): Path<Uuid>,
) -> Response {
    // Fetch item
    let item = match sqlx::query_as::<_, ShopItem>(
        r#"SELECT id, name, description, type AS item_type, cost_coins,
                  image_url, is_active, stock, created_at
           FROM shop_items WHERE id = $1 AND is_active = true"#,
    )
    .bind(item_id)
    .fetch_optional(&db)
    .await
    {
        Ok(Some(i)) => i,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "item not found" })),
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

    // Check stock
    if let Some(stock) = item.stock {
        let sold: i64 = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM user_purchases WHERE item_id = $1",
        )
        .bind(item.id)
        .fetch_one(&db)
        .await
        .unwrap_or(0);

        if sold >= stock as i64 {
            return (
                StatusCode::CONFLICT,
                Json(json!({ "error": "item out of stock" })),
            )
                .into_response();
        }
    }

    // Check balance
    let balance: i32 = sqlx::query("SELECT coins FROM users WHERE id = $1")
        .bind(session.user_id)
        .fetch_optional(&db)
        .await
        .ok()
        .flatten()
        .and_then(|r| r.try_get::<i32, _>("coins").ok())
        .unwrap_or(0);

    if balance < item.cost_coins {
        return (
            StatusCode::PAYMENT_REQUIRED,
            Json(json!({ "error": "insufficient coins" })),
        )
            .into_response();
    }

    // Deduct coins via ledger (trigger syncs users.coins)
    if sqlx::query(
        "INSERT INTO coin_transactions (user_id, amount, reason, meta) VALUES ($1, $2, 'purchase', $3)",
    )
    .bind(session.user_id)
    .bind(-item.cost_coins)
    .bind(serde_json::json!({ "item_id": item.id.to_string(), "item_name": item.name }))
    .execute(&db)
    .await
    .is_err()
    {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "failed to process payment" })),
        )
            .into_response();
    }

    // Record purchase
    if sqlx::query("INSERT INTO user_purchases (user_id, item_id) VALUES ($1, $2)")
        .bind(session.user_id)
        .bind(item.id)
        .execute(&db)
        .await
        .is_err()
    {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "failed to record purchase" })),
        )
            .into_response();
    }

    let new_balance = balance - item.cost_coins;
    Json(json!({
        "success": true,
        "item": item.name,
        "new_balance": new_balance,
    }))
    .into_response()
}
