use axum::{
    extract::State,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use sqlx::Row;

use crate::db::DbPool;
use crate::middleware::auth::Session;

// ── GET /api/me/coins ──────────────────────────────────────────────────────
pub async fn balance(State(db): State<DbPool>, session: Session) -> Response {
    #[derive(serde::Serialize, sqlx::FromRow)]
    struct TxRow {
        amount: i32,
        reason: String,
        meta: Option<serde_json::Value>,
        created_at: chrono::DateTime<chrono::Utc>,
    }

    #[derive(serde::Serialize, sqlx::FromRow)]
    struct PurchaseRow {
        item_name: String,
        item_type: String,
        cost_coins: i32,
        image_url: Option<String>,
        purchased_at: chrono::DateTime<chrono::Utc>,
    }

    let balance: i32 = sqlx::query("SELECT coins FROM users WHERE id = $1")
        .bind(session.user_id)
        .fetch_optional(&db)
        .await
        .ok()
        .flatten()
        .and_then(|r| r.try_get::<i32, _>("coins").ok())
        .unwrap_or(0);

    let history = sqlx::query_as::<_, TxRow>(
        r#"SELECT amount, reason, meta, created_at
           FROM coin_transactions
           WHERE user_id = $1
           ORDER BY created_at DESC
           LIMIT 50"#,
    )
    .bind(session.user_id)
    .fetch_all(&db)
    .await
    .unwrap_or_default();

    let purchases = sqlx::query_as::<_, PurchaseRow>(
        r#"SELECT s.name AS item_name, s.type AS item_type,
                  s.cost_coins, s.image_url, p.purchased_at
           FROM user_purchases p
           JOIN shop_items s ON s.id = p.item_id
           WHERE p.user_id = $1
           ORDER BY p.purchased_at DESC
           LIMIT 50"#,
    )
    .bind(session.user_id)
    .fetch_all(&db)
    .await
    .unwrap_or_default();

    Json(json!({
        "balance": balance,
        "history": history,
        "purchases": purchases,
    }))
    .into_response()
}
