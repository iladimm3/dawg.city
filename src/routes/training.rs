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
        .route("/stats", get(get_training_stats))
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
    pub log_id: Option<Uuid>, // If provided, update existing auto-saved log
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

    // Tier gating: free users limited to 3 AI generations per day across training + nutrition
    if user.subscription_tier == "free" {
        let today_count: i64 = sqlx::query_scalar!(
            r#"SELECT COUNT(*) FROM training_logs WHERE owner_id = $1 AND logged_at >= NOW() - INTERVAL '1 day'"#,
            user.id
        )
        .fetch_one(&state.db)
        .await
        .map_err(|e| AppError::InternalError(e.to_string()))?
        .unwrap_or(0);

        let nutrition_count: i64 = sqlx::query_scalar!(
            r#"SELECT COUNT(*) FROM nutrition_plans WHERE owner_id = $1 AND created_at >= NOW() - INTERVAL '1 day'"#,
            user.id
        )
        .fetch_one(&state.db)
        .await
        .map_err(|e| AppError::InternalError(e.to_string()))?
        .unwrap_or(0);

        if today_count + nutrition_count >= 3 {
            return Err(AppError::InvalidInput(
                "Free plan limit reached (3 AI sessions/day). Upgrade to Pro for unlimited access.".into()
            ));
        }
    }

    // Call Anthropic API
    let content = crate::services::anthropic::call(&state.anthropic_api_key, &state.anthropic_model, &prompt, 1024).await?;

    let session = serde_json::from_str::<TrainingSession>(&content)
        .map_err(|e| AppError::InternalError(format!("AI returned invalid JSON for training session: {}", e)))?;

    // Auto-save the generated session as a pending training log
    let log_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO training_logs (id, owner_id, dog_id, session_title, completed, logged_at)
        VALUES ($1, $2, $3, $4, false, NOW())
        "#,
        log_id, user.id, req.dog_id, session.title
    )
    .execute(&state.db)
    .await
    .map_err(|e| AppError::InternalError(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "log_id": log_id,
        "title": session.title,
        "duration_minutes": session.duration_minutes,
        "exercises": session.exercises,
        "tips": session.tips,
        "encouragement": session.encouragement,
    })))
}

async fn log_session_result(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(log): Json<SessionLog>,
) -> Result<Json<serde_json::Value>, AppError> {
    if let Some(id) = log.log_id {
        // Update the existing auto-saved log
        sqlx::query!(
            r#"
            UPDATE training_logs
            SET completed = $1, notes = $2, rating = $3, logged_at = NOW()
            WHERE id = $4 AND owner_id = $5
            "#,
            log.completed, log.notes, log.rating, id, user.id
        )
        .execute(&state.db)
        .await
        .map_err(|e| AppError::InternalError(e.to_string()))?;
    } else {
        // Create a new log entry (fallback / manual log)
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
    }

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

async fn get_training_stats(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Query(params): Query<HistoryParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Weekly session counts + avg rating for the last 8 weeks
    let weekly = sqlx::query!(
        r#"
        SELECT
            DATE_TRUNC('week', logged_at)::TEXT AS week,
            COUNT(*)::INT AS sessions,
            COUNT(*) FILTER (WHERE completed = true)::INT AS completed,
            ROUND(AVG(rating)::NUMERIC, 1)::FLOAT8 AS avg_rating
        FROM training_logs
        WHERE owner_id = $1
          AND dog_id   = $2
          AND logged_at >= NOW() - INTERVAL '8 weeks'
        GROUP BY 1
        ORDER BY 1
        "#,
        user.id,
        params.dog_id
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::InternalError(e.to_string()))?;

    let stats: Vec<serde_json::Value> = weekly
        .into_iter()
        .map(|row| serde_json::json!({
            "week":       row.week,
            "sessions":   row.sessions,
            "completed":  row.completed,
            "avg_rating": row.avg_rating,
        }))
        .collect();

    // All-time totals
    let total_completed: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM training_logs WHERE owner_id = $1 AND dog_id = $2 AND completed = true",
        user.id, params.dog_id
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| AppError::InternalError(e.to_string()))?
    .unwrap_or(0);

    let overall_avg: Option<f64> = sqlx::query_scalar!(
        "SELECT ROUND(AVG(rating)::NUMERIC, 1)::FLOAT8 FROM training_logs WHERE owner_id = $1 AND dog_id = $2 AND rating IS NOT NULL",
        user.id, params.dog_id
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| AppError::InternalError(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "weekly":           stats,
        "total_completed":  total_completed,
        "overall_avg_rating": overall_avg,
    })))
}
