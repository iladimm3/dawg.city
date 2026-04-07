use axum::{
    body::Bytes,
    extract::{Extension, State},
    http::HeaderMap,
    middleware,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{errors::AppError, middleware::auth::require_auth, models::user::User, AppState};

/// Routes behind session auth (/api/billing/...)
pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/checkout", post(create_checkout_session))
        .route("/portal", post(create_portal_session))
        .route("/status", get(get_subscription_status))
        .route_layer(middleware::from_fn_with_state(state, require_auth))
}

/// Stripe webhooks — NOT behind auth middleware (verified via HMAC signature)
pub fn webhook_router() -> Router<AppState> {
    Router::new().route("/stripe/webhook", post(handle_webhook))
}

// ─── Helpers ────────────────────────────────────────────────────────────────

/// POST form-encoded data to the Stripe API and return the JSON body.
async fn stripe_post(
    path: &str,
    secret_key: &str,
    params: &[(&str, &str)],
) -> Result<serde_json::Value, AppError> {
    let form_body = params
        .iter()
        .map(|(k, v)| format!("{}={}", urlencoding(k), urlencoding(v)))
        .collect::<Vec<_>>()
        .join("&");

    let response = reqwest::Client::new()
        .post(format!("https://api.stripe.com/v1/{}", path))
        .header("Authorization", format!("Bearer {}", secret_key))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(form_body)
        .send()
        .await
        .map_err(|e| AppError::InternalError(format!("Stripe request failed: {}", e)))?;

    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| AppError::InternalError(format!("Stripe response invalid JSON: {}", e)))?;

    if let Some(err) = body.get("error") {
        let msg = err["message"].as_str().unwrap_or("Stripe error");
        return Err(AppError::InternalError(format!("Stripe error: {}", msg)));
    }

    Ok(body)
}

fn urlencoding(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => vec![c],
            _ => format!("%{:02X}", c as u32).chars().collect(),
        })
        .collect()
}

/// Verify a Stripe webhook signature.
/// See https://stripe.com/docs/webhooks/signatures
fn verify_stripe_signature(
    payload: &[u8],
    sig_header: &str,
    secret: &str,
) -> Result<(), AppError> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    let parts: HashMap<&str, &str> = sig_header
        .split(',')
        .filter_map(|part| {
            let mut kv = part.splitn(2, '=');
            Some((kv.next()?, kv.next()?))
        })
        .collect();

    let timestamp = parts
        .get("t")
        .ok_or_else(|| AppError::InvalidInput("Missing timestamp in Stripe signature".into()))?;

    let expected_sig = parts
        .get("v1")
        .ok_or_else(|| AppError::InvalidInput("Missing v1 in Stripe signature".into()))?;

    // Enforce freshness: reject events older than 5 minutes
    let ts: i64 = timestamp
        .parse()
        .map_err(|_| AppError::InvalidInput("Invalid timestamp in Stripe signature".into()))?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    if (now - ts).abs() > 300 {
        return Err(AppError::InvalidInput("Stripe webhook timestamp too old".into()));
    }

    let signed_payload = format!("{}.{}", timestamp, String::from_utf8_lossy(payload));

    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .map_err(|_| AppError::InternalError("HMAC key error".into()))?;
    mac.update(signed_payload.as_bytes());
    let computed = hex::encode(mac.finalize().into_bytes());

    if computed != *expected_sig {
        return Err(AppError::Unauthorized("Stripe signature mismatch".into()));
    }

    Ok(())
}

// ─── Route handlers ──────────────────────────────────────────────────────────

#[derive(Serialize)]
struct CheckoutResponse {
    url: String,
}

async fn create_checkout_session(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
) -> Result<Json<CheckoutResponse>, AppError> {
    let secret_key = std::env::var("STRIPE_SECRET_KEY")
        .map_err(|_| AppError::InternalError("STRIPE_SECRET_KEY not configured".into()))?;
    let price_id = std::env::var("STRIPE_PRICE_ID")
        .map_err(|_| AppError::InternalError("STRIPE_PRICE_ID not configured".into()))?;
    let app_url = std::env::var("APP_URL").unwrap_or_else(|_| "http://localhost:5173".to_string());

    // Ensure the user has a Stripe customer — create one if not
    let customer_id = ensure_stripe_customer(&state, &user, &secret_key).await?;

    let session = stripe_post(
        "checkout/sessions",
        &secret_key,
        &[
            ("customer", &customer_id),
            ("mode", "subscription"),
            ("line_items[0][price]", &price_id),
            ("line_items[0][quantity]", "1"),
            ("success_url", &format!("{}/billing?success=1", app_url)),
            ("cancel_url", &format!("{}/billing", app_url)),
            ("allow_promotion_codes", "true"),
        ],
    )
    .await?;

    let url = session["url"]
        .as_str()
        .ok_or_else(|| AppError::InternalError("No checkout URL in Stripe response".into()))?
        .to_string();

    Ok(Json(CheckoutResponse { url }))
}

async fn create_portal_session(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
) -> Result<Json<serde_json::Value>, AppError> {
    let secret_key = std::env::var("STRIPE_SECRET_KEY")
        .map_err(|_| AppError::InternalError("STRIPE_SECRET_KEY not configured".into()))?;
    let app_url = std::env::var("APP_URL").unwrap_or_else(|_| "http://localhost:5173".to_string());

    let customer_id = ensure_stripe_customer(&state, &user, &secret_key).await?;

    let session = stripe_post(
        "billing_portal/sessions",
        &secret_key,
        &[
            ("customer", &customer_id),
            ("return_url", &format!("{}/billing", app_url)),
        ],
    )
    .await?;

    let url = session["url"]
        .as_str()
        .ok_or_else(|| AppError::InternalError("No portal URL in Stripe response".into()))?
        .to_string();

    Ok(Json(serde_json::json!({ "url": url })))
}

#[derive(Serialize)]
struct SubscriptionStatus {
    tier: String,
    has_active_subscription: bool,
}

async fn get_subscription_status(
    Extension(user): Extension<User>,
) -> Result<Json<SubscriptionStatus>, AppError> {
    Ok(Json(SubscriptionStatus {
        has_active_subscription: user.subscription_tier == "pro",
        tier: user.subscription_tier,
    }))
}

async fn handle_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<serde_json::Value>, AppError> {
    let secret = std::env::var("STRIPE_WEBHOOK_SECRET")
        .map_err(|_| AppError::InternalError("STRIPE_WEBHOOK_SECRET not configured".into()))?;

    let sig = headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::InvalidInput("Missing Stripe-Signature header".into()))?;

    verify_stripe_signature(&body, sig, &secret)?;

    let event: serde_json::Value = serde_json::from_slice(&body)
        .map_err(|_| AppError::InvalidInput("Invalid webhook JSON body".into()))?;

    let event_type = event["type"].as_str().unwrap_or("");

    match event_type {
        "customer.subscription.created" | "customer.subscription.updated" => {
            let subscription = &event["data"]["object"];
            let customer_id = subscription["customer"].as_str().unwrap_or("");
            let status = subscription["status"].as_str().unwrap_or("inactive");

            let new_tier = if status == "active" || status == "trialing" {
                "pro"
            } else {
                "free"
            };

            sqlx::query!(
                "UPDATE users SET subscription_tier = $1, updated_at = NOW() WHERE stripe_customer_id = $2",
                new_tier,
                customer_id,
            )
            .execute(&state.db)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

            tracing::info!(customer_id, new_tier, "Subscription tier updated via webhook");
        }
        "customer.subscription.deleted" => {
            let subscription = &event["data"]["object"];
            let customer_id = subscription["customer"].as_str().unwrap_or("");

            sqlx::query!(
                "UPDATE users SET subscription_tier = 'free', updated_at = NOW() WHERE stripe_customer_id = $1",
                customer_id,
            )
            .execute(&state.db)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

            tracing::info!(customer_id, "Subscription cancelled, tier reset to free");
        }
        _ => {
            tracing::debug!(event_type, "Unhandled Stripe webhook event");
        }
    }

    Ok(Json(serde_json::json!({ "received": true })))
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Look up or create a Stripe customer for the given user.
/// Stores the customer ID in the DB on first creation.
async fn ensure_stripe_customer(
    state: &AppState,
    user: &User,
    secret_key: &str,
) -> Result<String, AppError> {
    // Check if already stored
    let row = sqlx::query!(
        "SELECT stripe_customer_id FROM users WHERE id = $1",
        user.id
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| AppError::InternalError(e.to_string()))?;

    if let Some(cid) = row.stripe_customer_id {
        return Ok(cid);
    }

    // Create new Stripe customer
    let customer = stripe_post(
        "customers",
        secret_key,
        &[("email", &user.email), ("name", &user.name)],
    )
    .await?;

    let customer_id = customer["id"]
        .as_str()
        .ok_or_else(|| AppError::InternalError("No customer ID in Stripe response".into()))?
        .to_string();

    // Persist
    sqlx::query!(
        "UPDATE users SET stripe_customer_id = $1 WHERE id = $2",
        customer_id,
        user.id,
    )
    .execute(&state.db)
    .await
    .map_err(|e| AppError::InternalError(e.to_string()))?;

    Ok(customer_id)
}
