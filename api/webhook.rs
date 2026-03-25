use vercel_runtime::{run, Body, Error, Request, Response, StatusCode};
use serde_json::json;
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(handler).await
}

// ── Verify Stripe webhook signature ──────────────────────────────
fn verify_stripe_signature(payload: &[u8], sig_header: &str, secret: &str) -> bool {
    // sig_header format: t=timestamp,v1=signature,...
    let mut timestamp = "";
    let mut v1_sig = "";

    for part in sig_header.split(',') {
        if let Some(ts) = part.strip_prefix("t=") {
            timestamp = ts;
        } else if let Some(sig) = part.strip_prefix("v1=") {
            v1_sig = sig;
        }
    }

    if timestamp.is_empty() || v1_sig.is_empty() {
        return false;
    }

    // signed_payload = timestamp + "." + payload
    let signed_payload = format!("{}.{}", timestamp, String::from_utf8_lossy(payload));

    let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(signed_payload.as_bytes());
    let result = mac.finalize().into_bytes();
    let computed = hex::encode(result);

    // Constant-time comparison
    computed == v1_sig
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

pub async fn handler(req: Request) -> Result<Response<Body>, Error> {
    // Only accept POST
    if req.method() != "POST" {
        return Ok(Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body(Body::Text(json!({"error": "Method not allowed"}).to_string()))?);
    }

    let webhook_secret = match std::env::var("STRIPE_WEBHOOK_SECRET") {
        Ok(s) => s,
        Err(_) => return Ok(Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::Text(json!({"error": "Missing STRIPE_WEBHOOK_SECRET"}).to_string()))?),
    };

    let supabase_url = match std::env::var("SUPABASE_URL") {
        Ok(s) => s,
        Err(_) => return Ok(Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::Text(json!({"error": "Missing SUPABASE_URL"}).to_string()))?),
    };

    let service_role_key = match std::env::var("SUPABASE_SERVICE_ROLE_KEY") {
        Ok(s) => s,
        Err(_) => return Ok(Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::Text(json!({"error": "Missing SUPABASE_SERVICE_ROLE_KEY"}).to_string()))?),
    };

    // Get Stripe signature header
    let sig_header = req.headers()
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    // Get raw body bytes for signature verification
    let raw_body = match req.body() {
        Body::Text(s) => s.as_bytes().to_vec(),
        Body::Binary(b) => b.clone(),
        Body::Empty => vec![],
    };

    // Verify signature
    if !verify_stripe_signature(&raw_body, &sig_header, &webhook_secret) {
        return Ok(Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Body::Text(json!({"error": "Invalid signature"}).to_string()))?);
    }

    // Parse event
    let event: serde_json::Value = match serde_json::from_slice(&raw_body) {
        Ok(e) => e,
        Err(_) => return Ok(Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::Text(json!({"error": "Invalid JSON"}).to_string()))?),
    };

    let event_type = event["type"].as_str().unwrap_or("");
    let client = reqwest::Client::new();

    match event_type {
        // ── New subscription / payment completed ──────────────────
        "checkout.session.completed" => {
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
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::Text(json!({"received": true}).to_string()))?)
}
