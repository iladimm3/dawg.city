use axum::body::Bytes;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use hmac::{Hmac, Mac};
use serde_json::json;
use sha2::Sha256;
use std::time::Instant;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

// ── Verify Stripe webhook signature ──────────────────────────────
// Validates:
//   1. Signature is present and well-formed (t= and v1= fields)
//   2. Timestamp is within ±300 seconds of now (replay protection)
//   3. HMAC-SHA256 matches using constant-time comparison
fn verify_stripe_signature(payload: &[u8], sig_header: &str, secret: &str) -> bool {
    // sig_header format: t=timestamp,v1=signature,...
    let mut timestamp_str = "";
    let mut v1_sig = "";

    for part in sig_header.split(',') {
        if let Some(ts) = part.strip_prefix("t=") {
            timestamp_str = ts;
        } else if let Some(sig) = part.strip_prefix("v1=") {
            // Only take the first v1= value
            if v1_sig.is_empty() {
                v1_sig = sig;
            }
        }
    }

    if timestamp_str.is_empty() || v1_sig.is_empty() {
        eprintln!("[webhook] missing t= or v1= in Stripe-Signature header");
        return false;
    }

    // ── Replay protection: reject if timestamp is stale ──────────
    let webhook_ts: i64 = match timestamp_str.parse() {
        Ok(t) => t,
        Err(_) => {
            eprintln!("[webhook] non-numeric timestamp in Stripe-Signature");
            return false;
        }
    };
    let now = chrono::Utc::now().timestamp();
    let age_secs = (now - webhook_ts).abs();
    if age_secs > 300 {
        eprintln!("[webhook] timestamp too old or too far in future: age={}s", age_secs);
        return false;
    }

    // ── Build signed payload: "<timestamp>.<raw-body>" ───────────
    let signed_payload = format!("{}.{}", timestamp_str, String::from_utf8_lossy(payload));

    // ── Compute expected HMAC and compare using constant-time ─────
    // HmacSha256::verify_slice uses a timing-safe comparison internally,
    // preventing byte-by-byte timing side-channels.
    let expected_bytes = match hex::decode(v1_sig) {
        Ok(b) => b,
        Err(_) => {
            eprintln!("[webhook] v1 signature is not valid hex");
            return false;
        }
    };

    let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(signed_payload.as_bytes());

    // verify_slice performs constant-time comparison
    mac.verify_slice(&expected_bytes).is_ok()
}

// ── Update Supabase profile ───────────────────────────────────────
async fn update_profile(
    client: &reqwest::Client,
    supabase_url: &str,
    service_role_key: &str,
    stripe_customer_id: &str,
    stripe_subscription_id: &str,
    plan: &str,
) -> Result<(), String> {
    let url = format!("{}/rest/v1/profiles", supabase_url);

    let resp = client
        .patch(&url)
        .header("apikey", service_role_key)
        .header("Authorization", format!("Bearer {}", service_role_key))
        .header("Content-Type", "application/json")
        .header("Prefer", "return=minimal")
        .query(&[("stripe_customer_id", format!("eq.{}", stripe_customer_id))])
        .json(&json!({
            "plan": plan,
            "stripe_subscription_id": stripe_subscription_id,
            "scan_count_month": 0,
            "scan_reset_date": chrono::Utc::now().format("%Y-%m-%d").to_string()
        }))
        .send()
        .await
        .map_err(|e| format!("Supabase request failed: {}", e))?;

    if resp.status().is_success() {
        Ok(())
    } else {
        let body = resp.text().await.unwrap_or_default();
        Err(format!("Supabase update failed: {}", body))
    }
}

// ── Downgrade profile on subscription cancel ──────────────────────
async fn downgrade_profile(
    client: &reqwest::Client,
    supabase_url: &str,
    service_role_key: &str,
    stripe_customer_id: &str,
) -> Result<(), String> {
    let url = format!("{}/rest/v1/profiles", supabase_url);

    let resp = client
        .patch(&url)
        .header("apikey", service_role_key)
        .header("Authorization", format!("Bearer {}", service_role_key))
        .header("Content-Type", "application/json")
        .header("Prefer", "return=minimal")
        .query(&[("stripe_customer_id", format!("eq.{}", stripe_customer_id))])
        .json(&json!({
            "plan": "free",
            "stripe_subscription_id": null
        }))
        .send()
        .await
        .map_err(|e| format!("Supabase request failed: {}", e))?;

    if resp.status().is_success() {
        Ok(())
    } else {
        let body = resp.text().await.unwrap_or_default();
        Err(format!("Supabase downgrade failed: {}", body))
    }
}

// ── Store stripe_customer_id on first checkout ────────────────────
async fn link_customer_to_user(
    client: &reqwest::Client,
    supabase_url: &str,
    service_role_key: &str,
    customer_email: &str,
    stripe_customer_id: &str,
) -> Result<(), String> {
    // Find profile by email (Supabase auth users table)
    let auth_url = format!("{}/auth/v1/admin/users", supabase_url);
    let resp = client
        .get(&auth_url)
        .header("apikey", service_role_key)
        .header("Authorization", format!("Bearer {}", service_role_key))
        .query(&[("email", customer_email)])
        .send()
        .await
        .map_err(|e| format!("Auth lookup failed: {}", e))?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("Auth parse failed: {}", e))?;

    let user_id = resp["users"]
        .get(0)
        .and_then(|u| u["id"].as_str())
        .ok_or_else(|| format!("No user found for email: {}", customer_email))?;

    // Update their profile with the Stripe customer ID
    let profile_url = format!("{}/rest/v1/profiles", supabase_url);
    client
        .patch(&profile_url)
        .header("apikey", service_role_key)
        .header("Authorization", format!("Bearer {}", service_role_key))
        .header("Content-Type", "application/json")
        .header("Prefer", "return=minimal")
        .query(&[("id", format!("eq.{}", user_id))])
        .json(&json!({ "stripe_customer_id": stripe_customer_id }))
        .send()
        .await
        .map_err(|e| format!("Profile link failed: {}", e))?;

    Ok(())
}

pub async fn handler(headers: HeaderMap, body: Bytes) -> impl IntoResponse {
    let req_start = Instant::now();
    let request_id = Uuid::new_v4().to_string();

    let webhook_secret = match std::env::var("STRIPE_WEBHOOK_SECRET") {
        Ok(s) => s,
        Err(_) => return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Missing STRIPE_WEBHOOK_SECRET"})),
        ).into_response(),
    };

    let supabase_url = match std::env::var("SUPABASE_URL") {
        Ok(s) => s,
        Err(_) => return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Missing SUPABASE_URL"})),
        ).into_response(),
    };

    let service_role_key = match std::env::var("SUPABASE_SERVICE_ROLE_KEY") {
        Ok(s) => s,
        Err(_) => return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Missing SUPABASE_SERVICE_ROLE_KEY"})),
        ).into_response(),
    };

    // Get Stripe signature header
    let sig_header = headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    // Verify signature against raw bytes
    if !verify_stripe_signature(&body, &sig_header, &webhook_secret) {
        eprintln!("{}", json!({
            "event":      "webhook_sig_invalid",
            "request_id": request_id,
            "ms":         req_start.elapsed().as_millis() as u64
        }));
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Invalid signature"})),
        ).into_response();
    }

    // Parse event
    let event: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(e) => e,
        Err(_) => return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid JSON"})),
        ).into_response(),
    };

    let event_type = event["type"].as_str().unwrap_or("");
    let client = reqwest::Client::new();

    match event_type {
        // ── New subscription / payment completed ──────────────────
        "checkout.session.completed" => {
            eprintln!("{}", json!({"event":"stripe_checkout_completed","request_id":request_id}));
            let session = &event["data"]["object"];
            let customer_id = session["customer"].as_str().unwrap_or("");
            let customer_email = session["customer_details"]["email"]
                .as_str()
                .or_else(|| session["customer_email"].as_str())
                .unwrap_or("");
            let subscription_id = session["subscription"].as_str().unwrap_or("");

            // Determine plan from price ID or amount
            let amount = session["amount_total"].as_i64().unwrap_or(0);
            let plan = if amount >= 1900 { "pro" } else { "starter" };

            // Link customer email → Supabase user if not already linked
            if !customer_email.is_empty() && !customer_id.is_empty() {
                let _ = link_customer_to_user(
                    &client, &supabase_url, &service_role_key,
                    customer_email, customer_id
                ).await;
            }

            // Update plan
            if !customer_id.is_empty() {
                if let Err(e) = update_profile(
                    &client, &supabase_url, &service_role_key,
                    customer_id, subscription_id, plan
                ).await {
                    eprintln!("Failed to update profile: {}", e);
                }
            }
        }

        // ── Subscription cancelled / expired ─────────────────────
        "customer.subscription.deleted" => {
            eprintln!("{}", json!({"event":"stripe_subscription_deleted","request_id":request_id}));
            let subscription = &event["data"]["object"];
            let customer_id = subscription["customer"].as_str().unwrap_or("");

            if !customer_id.is_empty() {
                if let Err(e) = downgrade_profile(
                    &client, &supabase_url, &service_role_key, customer_id
                ).await {
                    eprintln!("Failed to downgrade profile: {}", e);
                }
            }
        }

        // ── Ignore all other events ───────────────────────────────
        _ => {}
    }

    // Always return 200 to Stripe so it doesn't retry
    eprintln!("{}", json!({
        "event":      "webhook_ok",
        "request_id": request_id,
        "type":       event_type,
        "ms":         req_start.elapsed().as_millis() as u64
    }));

    (StatusCode::OK, Json(json!({"received": true}))).into_response()
}
