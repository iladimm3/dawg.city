use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::services::oauth::GoogleUserInfo;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub google_sub: String,
    pub email: String,
    pub name: String,
    pub avatar_url: Option<String>,
    pub subscription_tier: String, // "free" | "pro"
    pub stripe_customer_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    /// Insert or update a user from Google OAuth info
    pub async fn upsert_from_google(db: &PgPool, google: &GoogleUserInfo) -> Result<Self> {
        let user = sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (id, google_sub, email, name, avatar_url, subscription_tier, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, 'free', NOW(), NOW())
            ON CONFLICT (google_sub) DO UPDATE SET
                email = EXCLUDED.email,
                name = EXCLUDED.name,
                avatar_url = EXCLUDED.avatar_url,
                updated_at = NOW()
            RETURNING *
            "#,
            Uuid::new_v4(),
            google.sub,
            google.email,
            google.name,
            google.picture,
        )
        .fetch_one(db)
        .await?;

        Ok(user)
    }

    pub async fn find_by_id(db: &PgPool, id: Uuid) -> Result<Option<Self>> {
        let user = sqlx::query_as!(
            User,
            "SELECT * FROM users WHERE id = $1",
            id
        )
        .fetch_optional(db)
        .await?;

        Ok(user)
    }
}
