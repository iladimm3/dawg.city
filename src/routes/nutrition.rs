use axum::{
    extract::{Extension, State},
    middleware,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{errors::AppError, middleware::auth::require_auth, models::user::User, AppState};

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/plan", post(generate_nutrition_plan))
        .route_layer(middleware::from_fn_with_state(state, require_auth))
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

    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| AppError::InternalError("ANTHROPIC_API_KEY is not configured".into()))?;
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

    let plan = serde_json::from_str::<NutritionPlan>(content)
        .map_err(|_| AppError::InternalError("AI response parsing failed".into()))?;

    Ok(Json(serde_json::json!(plan)))
}
