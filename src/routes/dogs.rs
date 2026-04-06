use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    middleware,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{errors::AppError, middleware::auth::require_auth, models::user::User, AppState};

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(list_dogs).post(create_dog))
        .route("/:id", get(get_dog).put(update_dog).delete(delete_dog))
        .route_layer(middleware::from_fn_with_state(state, require_auth))
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Dog {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub name: String,
    pub breed: String,
    pub age_months: i32,
    pub weight_kg: f64,
    pub sex: String,           // "male" | "female"
    pub neutered: bool,
    pub activity_level: String, // "low" | "medium" | "high"
    pub health_notes: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize)]
pub struct CreateDogPayload {
    pub name: String,
    pub breed: String,
    pub age_months: i32,
    pub weight_kg: f64,
    pub sex: String,
    pub neutered: bool,
    pub activity_level: String,
    pub health_notes: Option<String>,
}

async fn list_dogs(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
) -> Result<Json<serde_json::Value>, AppError> {
    let dogs = sqlx::query_as!(
        Dog,
        "SELECT * FROM dogs WHERE owner_id = $1 ORDER BY created_at DESC",
        user.id
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::InternalError(e.to_string()))?;

    Ok(Json(serde_json::json!(dogs)))
}

async fn create_dog(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(payload): Json<CreateDogPayload>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    let dog = sqlx::query_as!(
        Dog,
        r#"
        INSERT INTO dogs (id, owner_id, name, breed, age_months, weight_kg, sex, neutered, activity_level, health_notes, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW())
        RETURNING *
        "#,
        Uuid::new_v4(), user.id, payload.name, payload.breed,
        payload.age_months, payload.weight_kg, payload.sex,
        payload.neutered, payload.activity_level, payload.health_notes,
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| AppError::InternalError(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(serde_json::json!(dog))))
}

async fn get_dog(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let dog = sqlx::query_as!(
        Dog,
        "SELECT * FROM dogs WHERE id = $1 AND owner_id = $2",
        id, user.id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::InternalError(e.to_string()))?
    .ok_or_else(|| AppError::NotFound("Dog not found".into()))?;

    Ok(Json(serde_json::json!(dog)))
}

async fn update_dog(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(id): Path<Uuid>,
    Json(payload): Json<CreateDogPayload>,
) -> Result<Json<serde_json::Value>, AppError> {
    let dog = sqlx::query_as!(
        Dog,
        r#"
        UPDATE dogs SET name=$1, breed=$2, age_months=$3, weight_kg=$4,
        sex=$5, neutered=$6, activity_level=$7, health_notes=$8
        WHERE id=$9 AND owner_id=$10
        RETURNING *
        "#,
        payload.name, payload.breed, payload.age_months, payload.weight_kg,
        payload.sex, payload.neutered, payload.activity_level, payload.health_notes,
        id, user.id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::InternalError(e.to_string()))?
    .ok_or_else(|| AppError::NotFound("Dog not found".into()))?;

    Ok(Json(serde_json::json!(dog)))
}

async fn delete_dog(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = sqlx::query!(
        "DELETE FROM dogs WHERE id = $1 AND owner_id = $2",
        id, user.id
    )
    .execute(&state.db)
    .await
    .map_err(|e| AppError::InternalError(e.to_string()))?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Dog not found".into()));
    }

    Ok(Json(serde_json::json!({"success": true})))
}
