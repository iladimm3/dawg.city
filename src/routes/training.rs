use axum::{
    extract::{Extension, Query, State},
    middleware,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{errors::AppError, middleware::auth::require_auth, models::user::User, AppState};

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/session", post(generate_training_session))
        .route("/log", post(log_session_result))
        .route("/history", get(get_training_history))
        .route_layer(middleware::from_fn_with_state(state, require_auth))
}

#[derive(Deserialize)]
pub struct TrainingRequest {
    pub dog_id: Uuid,
    pub focus_areas: Vec<String>,              // e.g. ["recall", "anxiety", "leash walking"]
    pub session_length_minutes: i32,           // duration in minutes
    pub last_session_notes: Option<String>,    // What went well/poorly last time
    pub difficulty: Option<String>,            // "beginner" | "intermediate" | "advanced"
}

#[derive(Deserialize)]
pub struct HistoryParams {
    pub dog_id: Uuid,
    pub limit: Option<i32>,   // default 20
    pub offset: Option<i32>,  // default 0
}

#[derive(Serialize, sqlx::FromRow)]
pub struct TrainingLogRow {
    pub id: Uuid,
    pub dog_id: Uuid,
    pub session_title: String,
    pub completed: bool,
    pub notes: Option<String>,
    pub rating: Option<i32>,
    pub logged_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, Deserialize)]
pub struct TrainingSession {
    pub title: String,
    pub duration_minutes: i32,
    pub exercises: Vec<Exercise>,
    pub tips: Vec<String>,
    pub encouragement: String,
}

#[derive(Serialize, Deserialize)]
pub struct Exercise {
    pub name: String,
    pub description: String,
    pub repetitions: String,
    pub success_criteria: String,
}

#[derive(Deserialize)]
pub struct SessionLog {
    pub dog_id: Uuid,
    pub session_title: String,
    pub completed: bool,
    pub notes: Option<String>,
    pub rating: Option<i32>, // 1-5
}

async fn generate_training_session(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(req): Json<TrainingRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let dog = sqlx::query!(
        "SELECT name, breed, age_months, weight_kg, activity_level, health_notes FROM dogs WHERE id = $1 AND owner_id = $2",
        req.dog_id, user.id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::InternalError(e.to_string()))?
    .ok_or_else(|| AppError::NotFound("Dog not found".into()))?;

    // Build AI prompt with dog context
    let age_description = match dog.age_months {
        0..=6 => "young puppy",
        7..=18 => "adolescent",
        19..=84 => "adult",
        _ => "senior",
    };

    let focus_list = if req.focus_areas.is_empty() {
        "general obedience".to_string()
    } else {
        req.focus_areas.join(", ")
    };
    let duration = req.session_length_minutes;
    let difficulty = req.difficulty.as_deref().unwrap_or("beginner");
    let last_notes = req.last_session_notes.as_deref()
        .unwrap_or("No previous session data");

    let prompt = format!(
        r#"You are Dawg City's expert AI dog trainer. Generate a highly personalized, science-backed training session.

DOG PROFILE:
- Name: {}
- Breed: {} (consider breed-specific traits, energy level, and learning style)
- Age: {} months ({} — tailor exercises accordingly)
- Weight: {} kg
- Activity level: {}
- Health notes: {}

SESSION PARAMETERS:
- Focus areas: {}
- Difficulty level: {} (adjust complexity, repetitions, and expectations accordingly)
- Duration: {} minutes (plan exercise timing to fit exactly within this window)
- Last session feedback: {}

TRAINER INSTRUCTIONS:
- For a {} dog, adapt vocabulary, patience requirements, and reward frequency
- For {} difficulty, {} pacing and complexity
- Prioritize the focus areas in order listed
- Include rest periods between exercises for the dog's age/health
- Each exercise should build on the previous one where possible

Respond ONLY with valid JSON in this exact format:
{{
  "title": "Descriptive session title reflecting the focus areas",
  "duration_minutes": {},
  "exercises": [
    {{
      "name": "Exercise name",
      "description": "Step-by-step instruction for the owner, including body language and tone tips",
      "repetitions": "e.g. 5 repetitions, 3 sets with 30s rest",
      "success_criteria": "Specific observable behavior that shows the dog understood"
    }}
  ],
  "tips": [
    "Breed-specific tip for {}",
    "Age-appropriate tip for a {} dog",
    "Tip specific to {} difficulty"
  ],
  "encouragement": "A warm, specific motivating message mentioning the dog's name and focus areas"
}}"#,
        dog.name, dog.breed, dog.age_months, age_description,
        dog.weight_kg, dog.activity_level,
        dog.health_notes.as_deref().unwrap_or("None"),
        focus_list, difficulty, duration, last_notes,
        age_description,
        difficulty,
        match difficulty { "beginner" => "use slower", "advanced" => "use faster", _ => "use moderate" },
        duration,
        dog.breed, age_description, difficulty
    );

    // Call Anthropic API
    let api_key = std::env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY must be set");
    let model = std::env::var("ANTHROPIC_MODEL")
        .unwrap_or_else(|_| "claude-sonnet-4-20250514".to_string());

    let response = reqwest::Client::new()
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "model": model,
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": prompt}]
        }))
        .send()
        .await;

    let body: serde_json::Value = response
        .map_err(|e| AppError::InternalError(format!("AI request failed: {}", e)))?
        .json()
        .await
        .unwrap_or_default();
    let content = body["content"][0]["text"].as_str().unwrap_or("{}");

    let session = serde_json::from_str::<TrainingSession>(content)
        .map_err(|_| AppError::InternalError("AI response parsing failed".into()))?;

    Ok(Json(serde_json::json!(session)))
}

async fn log_session_result(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(log): Json<SessionLog>,
) -> Result<Json<serde_json::Value>, AppError> {
    sqlx::query!(
        r#"
        INSERT INTO training_logs (id, owner_id, dog_id, session_title, completed, notes, rating, logged_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())
        "#,
        Uuid::new_v4(), user.id, log.dog_id,
        log.session_title, log.completed, log.notes, log.rating
    )
    .execute(&state.db)
    .await
    .map_err(|e| AppError::InternalError(e.to_string()))?;

    Ok(Json(serde_json::json!({"success": true})))
}

async fn get_training_history(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Query(params): Query<HistoryParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let limit = params.limit.unwrap_or(20).clamp(1, 100) as i64;
    let offset = params.offset.unwrap_or(0).max(0) as i64;

    let total: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM training_logs WHERE owner_id = $1 AND dog_id = $2",
        user.id, params.dog_id
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| AppError::InternalError(e.to_string()))?
    .unwrap_or(0);

    let logs = sqlx::query_as!(
        TrainingLogRow,
        r#"
        SELECT id, dog_id, session_title, completed, notes, rating, logged_at
        FROM training_logs
        WHERE owner_id = $1 AND dog_id = $2
        ORDER BY logged_at DESC
        LIMIT $3 OFFSET $4
        "#,
        user.id, params.dog_id, limit, offset
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::InternalError(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "data": logs,
        "total": total,
        "limit": limit,
        "offset": offset
    })))
}
