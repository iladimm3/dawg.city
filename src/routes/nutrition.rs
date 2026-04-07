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
        .route("/plan", post(generate_nutrition_plan))
        .route("/history", get(get_nutrition_history))
        .route("/stats", get(get_nutrition_stats))
        .route_layer(middleware::from_fn_with_state(state, require_auth))
}

#[derive(Deserialize)]
pub struct HistoryParams {
    pub dog_id: Uuid,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct NutritionPlanRow {
    pub id: Uuid,
    pub dog_id: Uuid,
    pub daily_calories: i32,
    pub meals_per_day: i32,
    pub portion_per_meal_grams: f64,
    pub feeding_schedule: Vec<String>,
    pub recommended_foods: Vec<String>,
    pub foods_to_avoid: Vec<String>,
    pub supplements: Vec<String>,
    pub notes: String,
    pub next_review_weeks: i32,
    pub goal: Option<String>,
    pub food_brand: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize)]
pub struct NutritionRequest {
    pub dog_id: Uuid,
    pub food_brand: Option<String>,
    pub dietary_restrictions: Option<Vec<String>>,  // e.g. ["grain-free", "no chicken"]
    pub goal: Option<String>,   // "maintain" | "lose_weight" | "gain_muscle" | "puppy_growth"
    pub current_issues: Option<Vec<String>>,        // e.g. ["loose stool", "low energy"]
}

#[derive(Serialize, Deserialize)]
pub struct NutritionPlan {
    pub daily_calories: i32,
    pub meals_per_day: i32,
    pub portion_per_meal_grams: f64,
    pub feeding_schedule: Vec<String>,
    pub recommended_foods: Vec<String>,
    pub foods_to_avoid: Vec<String>,
    pub supplements: Vec<String>,
    pub notes: String,
    pub next_review_weeks: i32,
}

async fn generate_nutrition_plan(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(req): Json<NutritionRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let dog = sqlx::query!(
        "SELECT name, breed, age_months, weight_kg, activity_level, health_notes, neutered FROM dogs WHERE id = $1 AND owner_id = $2",
        req.dog_id, user.id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::InternalError(e.to_string()))?
    .ok_or_else(|| AppError::NotFound("Dog not found".into()))?;

    let goal = req.goal.as_deref().unwrap_or("maintain");
    let food_brand = req.food_brand.as_deref().unwrap_or("any quality brand");
    let restrictions = req
        .dietary_restrictions
        .as_ref()
        .filter(|v| !v.is_empty())
        .map(|v| v.join(", "))
        .unwrap_or_else(|| "none".to_string());
    let issues = req
        .current_issues
        .as_ref()
        .filter(|v| !v.is_empty())
        .map(|v| v.join(", "))
        .unwrap_or_else(|| "none reported".to_string());

    let prompt = format!(
        r#"You are Dawg City's expert AI canine nutritionist. Create a medically-informed, personalized nutrition plan.

DOG PROFILE:
- Name: {}
- Breed: {} (consider breed-specific metabolic rates and common health predispositions)
- Age: {} months
- Weight: {} kg
- Activity level: {}
- Neutered: {} (affects caloric needs significantly)
- Health notes: {}

NUTRITION REQUEST:
- Goal: {} (optimize all recommendations toward this goal)
- Current food brand: {}
- Dietary restrictions: {} (strictly exclude these)
- Current health issues: {} (address these in your recommendations)

NUTRITIONIST INSTRUCTIONS:
- Calculate calories using the dog's weight, age, neutered status, and activity level
- Account for breed size (toy/small/medium/large/giant have different metabolic rates)
- Address each current health issue with specific food/supplement recommendations
- Ensure all food recommendations comply with the dietary restrictions

Respond ONLY with valid JSON in this exact format:
{{
  "daily_calories": 1200,
  "meals_per_day": 2,
  "portion_per_meal_grams": 150.0,
  "feeding_schedule": ["7:00 AM - Morning meal", "6:00 PM - Evening meal"],
  "recommended_foods": ["Specific food 1 with reason", "Specific food 2 with reason"],
  "foods_to_avoid": ["Food to avoid with reason", "Another food to avoid"],
  "supplements": ["Supplement if needed with dosage"],
  "notes": "Specific actionable advice addressing the dog's breed, age, health issues, and goal",
  "next_review_weeks": 4
}}"#,
        dog.name, dog.breed, dog.age_months, dog.weight_kg,
        dog.activity_level, dog.neutered,
        dog.health_notes.as_deref().unwrap_or("None"),
        goal, food_brand, restrictions, issues
    );

    // Tier gating: free users limited to 3 AI generations per day across training + nutrition
    if user.subscription_tier == "free" {
        let training_count: i64 = sqlx::query_scalar!(
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

        if training_count + nutrition_count >= 3 {
            return Err(AppError::InvalidInput(
                "Free plan limit reached (3 AI sessions/day). Upgrade to Pro for unlimited access.".into()
            ));
        }
    }

    let content = crate::services::anthropic::call(&state.anthropic_api_key, &state.anthropic_model, &prompt, 1024).await?;

    let plan = serde_json::from_str::<NutritionPlan>(&content)
        .map_err(|e| AppError::InternalError(format!("AI returned invalid JSON for nutrition plan: {}", e)))?;

    // Auto-save the plan to the database
    sqlx::query!(
        r#"
        INSERT INTO nutrition_plans (
            id, owner_id, dog_id,
            daily_calories, meals_per_day, portion_per_meal_grams,
            feeding_schedule, recommended_foods, foods_to_avoid,
            supplements, notes, next_review_weeks, goal, food_brand,
            created_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, NOW())
        "#,
        Uuid::new_v4(),
        user.id,
        req.dog_id,
        plan.daily_calories,
        plan.meals_per_day,
        plan.portion_per_meal_grams,
        &plan.feeding_schedule,
        &plan.recommended_foods,
        &plan.foods_to_avoid,
        &plan.supplements,
        plan.notes,
        plan.next_review_weeks,
        goal,
        food_brand,
    )
    .execute(&state.db)
    .await
    .map_err(|e| AppError::InternalError(e.to_string()))?;

    Ok(Json(serde_json::json!(plan)))
}

async fn get_nutrition_history(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Query(params): Query<HistoryParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let limit = params.limit.unwrap_or(10).min(100);
    let offset = params.offset.unwrap_or(0);

    // Verify dog belongs to user
    let dog_exists = sqlx::query!(
        "SELECT id FROM dogs WHERE id = $1 AND owner_id = $2",
        params.dog_id, user.id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::InternalError(e.to_string()))?;

    if dog_exists.is_none() {
        return Err(AppError::NotFound("Dog not found".into()));
    }

    let rows = sqlx::query_as!(
        NutritionPlanRow,
        r#"
        SELECT id, dog_id, daily_calories, meals_per_day, portion_per_meal_grams,
               feeding_schedule, recommended_foods, foods_to_avoid,
               supplements, notes, next_review_weeks, goal, food_brand, created_at
        FROM nutrition_plans
        WHERE dog_id = $1 AND owner_id = $2
        ORDER BY created_at DESC
        LIMIT $3 OFFSET $4
        "#,
        params.dog_id, user.id, limit, offset
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::InternalError(e.to_string()))?;

    let total: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM nutrition_plans WHERE dog_id = $1 AND owner_id = $2",
        params.dog_id, user.id
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| AppError::InternalError(e.to_string()))?
    .unwrap_or(0);

    Ok(Json(serde_json::json!({
        "data": rows,
        "total": total,
        "limit": limit,
        "offset": offset
    })))
}

async fn get_nutrition_stats(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Query(params): Query<HistoryParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Calorie history over the last 12 plans
    let rows = sqlx::query!(
        r#"
        SELECT
            created_at::TEXT AS date,
            daily_calories,
            goal
        FROM nutrition_plans
        WHERE dog_id = $1 AND owner_id = $2
        ORDER BY created_at DESC
        LIMIT 12
        "#,
        params.dog_id,
        user.id
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::InternalError(e.to_string()))?;

    let history: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| serde_json::json!({
            "date":           r.date,
            "daily_calories": r.daily_calories,
            "goal":           r.goal,
        }))
        .collect();

    Ok(Json(serde_json::json!({ "calorie_history": history })))
}
