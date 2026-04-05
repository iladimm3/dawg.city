use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub discord_id: String,
    pub username: String,
    pub avatar_url: Option<String>,
    pub coins: i32,
    pub xp: i32,
    pub streak_days: i32,
    pub last_seen: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Session {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Game {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    pub description: Option<String>,
    pub category: String,
    pub embed_url: String,
    pub thumbnail: Option<String>,
    pub tags: Option<Vec<String>>,
    pub play_count: i64,
    pub is_featured: bool,
    pub is_active: bool,
    pub added_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ShopItem {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub item_type: String,
    pub cost_coins: i32,
    pub image_url: Option<String>,
    pub is_active: bool,
    pub stock: Option<i32>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BattlePassSeason {
    pub id: Uuid,
    pub name: String,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BattlePassTier {
    pub id: Uuid,
    pub season_id: Uuid,
    pub tier: i32,
    pub xp_required: i32,
    pub reward_type: String,
    pub reward_meta: serde_json::Value,
    pub is_premium: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserBattlePass {
    pub user_id: Uuid,
    pub season_id: Uuid,
    pub is_premium: bool,
    pub current_tier: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Mirror {
    pub id: Uuid,
    pub url: String,
    pub is_active: bool,
    pub added_at: DateTime<Utc>,
}
