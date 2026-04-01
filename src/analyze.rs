use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::json;
use std::time::Instant;
use uuid::Uuid;

// ── YouTube patterns ──
static YT_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| vec![
    Regex::new(r"(?:youtube\.com/watch\?v=)([a-zA-Z0-9_-]{11})").unwrap(),
    Regex::new(r"(?:youtu\.be/)([a-zA-Z0-9_-]{11})").unwrap(),
    Regex::new(r"(?:youtube\.com/shorts/)([a-zA-Z0-9_-]{11})").unwrap(),
    Regex::new(r"(?:youtube\.com/embed/)([a-zA-Z0-9_-]{11})").unwrap(),
    Regex::new(r"(?:youtube\.com/live/)([a-zA-Z0-9_-]{11})").unwrap(),
]);

// ── TikTok patterns ──
static TT_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| vec![
    Regex::new(r"tiktok\.com/@[^/]+/video/(\d+)").unwrap(),
    Regex::new(r"tiktok\.com/t/([a-zA-Z0-9]+)").unwrap(),
    Regex::new(r"vm\.tiktok\.com/([a-zA-Z0-9]+)").unwrap(),
]);

// ── X / Twitter patterns ──
static X_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| vec![
    Regex::new(r"(?:twitter|x)\.com/\w+/status/(\d+)").unwrap(),
]);

// ── Instagram patterns ──
static IG_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| vec![
    Regex::new(r"instagram\.com/(?:p|reel|tv)/([a-zA-Z0-9_-]+)").unwrap(),
]);

const RECAPTCHA_MIN_SCORE: f64 = 0.5;

#[derive(Debug)]
enum Platform {
    YouTube(String),
    TikTok(String),
    Twitter(String),
    Instagram(String),
}

fn detect_platform(url: &str) -> Option<Platform> {
    if let Some(id) = YT_PATTERNS.iter().find_map(|re| re.captures(url).map(|c| c[1].to_string())) {
        return Some(Platform::YouTube(id));
    }
    if TT_PATTERNS.iter().any(|re| re.is_match(url)) || url.contains("tiktok.com") {
        return Some(Platform::TikTok(url.to_string()));
    }
    if X_PATTERNS.iter().any(|re| re.is_match(url)) {
        return Some(Platform::Twitter(url.to_string()));
    }
    if IG_PATTERNS.iter().any(|re| re.is_match(url)) {
        return Some(Platform::Instagram(url.to_string()));
    }
    None
}

fn url_encode(s: &str) -> String {
    let mut encoded = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' |
            b'-' | b'_' | b'.' | b'~' => encoded.push(b as char),
            _ => encoded.push_str(&format!("%{:02X}", b)),
        }
    }
    encoded
}

async fn get_thumbnail(client: &reqwest::Client, platform: &Platform) -> Result<String, String> {
    match platform {
        Platform::YouTube(id) => {
            let max = format!("https://img.youtube.com/vi/{}/maxresdefault.jpg", id);
            let hq  = format!("https://img.youtube.com/vi/{}/hqdefault.jpg", id);
            match client.get(&max).send().await {
                Ok(r) if r.status().is_success() => {
                    let bytes = r.bytes().await.map_err(|e| e.to_string())?;
                    if bytes.len() > 2000 { Ok(max) } else { Ok(hq) }
                }
                _ => Ok(hq),
            }
        }

        Platform::TikTok(url) => {
            let oembed_url = format!(
                "https://www.tiktok.com/oembed?url={}",
                url_encode(url)
            );
            let resp = client
                .get(&oembed_url)
                .header("User-Agent", "Mozilla/5.0 (compatible; dawg.city/1.0)")
                .send()
                .await
                .map_err(|e| format!("TikTok request failed: {}", e))?
                .json::<serde_json::Value>()
                .await
                .map_err(|e| format!("TikTok parse failed: {}", e))?;

            resp["thumbnail_url"]
                .as_str()
                .map(|s| s.to_string())
                .ok_or_else(|| "Could not get TikTok thumbnail. The video may be private or unavailable.".to_string())
        }

        Platform::Twitter(url) => {
            let fx_url = url
                .replace("twitter.com", "api.fxtwitter.com")
                .replace("x.com", "api.fxtwitter.com");

            let fx_resp = client
                .get(&fx_url)
                .header("User-Agent", "Mozilla/5.0 (compatible; dawg.city/1.0)")
                .send()
                .await
                .map_err(|e| format!("X/Twitter request failed: {}", e))?
                .json::<serde_json::Value>()
                .await
                .map_err(|e| format!("X/Twitter parse failed: {}", e))?;

            if let Some(img) = fx_resp["tweet"]["media"]["photos"]
                .get(0)
                .and_then(|p| p["url"].as_str())
            {
                return Ok(img.to_string());
            }
            if let Some(img) = fx_resp["tweet"]["media"]["videos"]
                .get(0)
                .and_then(|v| v["thumbnail_url"].as_str())
            {
                return Ok(img.to_string());
            }
            if let Some(img) = fx_resp["tweet"]["author"]["avatar_url"].as_str() {
                return Ok(img.to_string());
            }

            Err("Could not extract X/Twitter media. The post may be private or text-only.".to_string())
        }

        Platform::Instagram(url) => {
            let ig_token = std::env::var("INSTAGRAM_TOKEN").unwrap_or_default();
            let oembed_url = if !ig_token.is_empty() {
                format!(
                    "https://graph.facebook.com/v18.0/instagram_oembed?url={}&maxwidth=800&access_token={}",
                    url_encode(url), ig_token
                )
            } else {
                format!(
                    "https://graph.facebook.com/v18.0/instagram_oembed?url={}&maxwidth=800",
                    url_encode(url)
                )
            };

            let resp = client
                .get(&oembed_url)
                .header("User-Agent", "Mozilla/5.0 (compatible; dawg.city/1.0)")
                .send()
                .await
                .map_err(|e| format!("Instagram request failed: {}", e))?
                .json::<serde_json::Value>()
                .await
                .map_err(|e| format!("Instagram parse failed: {}", e))?;

            resp["thumbnail_url"]
                .as_str()
                .map(|s| s.to_string())
                .ok_or_else(|| "Could not get Instagram thumbnail. Make sure the post is public and paste the full URL.".to_string())
        }
    }
}

pub async fn handler(headers: HeaderMap, body: String) -> impl IntoResponse {
    let req_start = Instant::now();
    let request_id = Uuid::new_v4().to_string();

    let parsed: serde_json::Value = serde_json::from_str(&body).unwrap_or(json!({}));

    let recaptcha_token = match parsed["recaptcha_token"].as_str() {
        Some(t) if !t.is_empty() => t.to_string(),
        _ => return error_response("Missing reCAPTCHA token"),
    };

    // ── Read secrets from env (never hardcode) ────────────────────
    let recaptcha_secret = match std::env::var("RECAPTCHA_SECRET") {
        Ok(s) => s,
        Err(_) => return error_response("Server misconfiguration: missing RECAPTCHA_SECRET"),
    };
    let supabase_url = match std::env::var("SUPABASE_URL") {
        Ok(s) => s,
        Err(_) => return error_response("Server misconfiguration: missing SUPABASE_URL"),
    };
    let supabase_service_key = match std::env::var("SUPABASE_SERVICE_ROLE_KEY") {
        Ok(s) => s,
        Err(_) => return error_response("Server misconfiguration: missing SUPABASE_SERVICE_ROLE_KEY"),
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(7))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    // ── reCAPTCHA verification ────────────────────────────────────
    let recaptcha_resp = match client
        .post("https://www.google.com/recaptcha/api/siteverify")
        .form(&[("secret", recaptcha_secret.as_str()), ("response", recaptcha_token.as_str())])
        .send()
        .await
    {
        Ok(r) => match r.json::<serde_json::Value>().await {
            Ok(v) => v,
            Err(_) => return error_response("reCAPTCHA verification failed. Please try again."),
        },
        Err(_) => return error_response("reCAPTCHA verification failed. Please try again."),
    };

    let success = recaptcha_resp["success"].as_bool().unwrap_or(false);
    let score   = recaptcha_resp["score"].as_f64().unwrap_or(0.0);

    if !success || score < RECAPTCHA_MIN_SCORE {
        return error_response("reCAPTCHA verification failed. Please try again.");
    }

    // ── Authenticate caller (optional — guests may scan without a token) ──
    let user_jwt = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.to_string())
        .filter(|t| !t.is_empty());

    let user_id: String = if let Some(jwt) = user_jwt {
        let auth_result = client
            .get(format!("{}/auth/v1/user", supabase_url))
            .header("apikey", &supabase_service_key)
            .header("Authorization", format!("Bearer {}", jwt))
            .send()
            .await;

        let id = match auth_result {
            Ok(r) if r.status().is_success() => {
                let user: serde_json::Value = r.json().await.unwrap_or(json!({}));
                match user["id"].as_str().map(|s| s.to_string()) {
                    Some(id) => id,
                    None => return error_response("Could not resolve user identity"),
                }
            }
            Ok(r) if r.status().as_u16() == 401 => {
                return error_response("Invalid or expired session. Please log in again.");
            }
            Ok(_) => return error_response("Authentication failed"),
            Err(_) => return error_response("Authentication check failed. Please try again."),
        };

        // ── Atomic quota enforcement (server-authoritative) ───────
        let quota_resp = client
            .post(format!("{}/rest/v1/rpc/increment_scan_quota", supabase_url))
            .header("apikey", &supabase_service_key)
            .header("Authorization", format!("Bearer {}", supabase_service_key))
            .header("Content-Type", "application/json")
            .json(&json!({ "p_user_id": id }))
            .send()
            .await;

        match quota_resp {
            Ok(r) if r.status().is_success() => {
                let body = r.json::<serde_json::Value>().await.unwrap_or(serde_json::Value::Null);
                if body.is_null() {
                    eprintln!("{}", json!({"event":"quota_exceeded","request_id":request_id,"user_id":id,"ms":req_start.elapsed().as_millis() as u64}));
                    return error_response(
                        "Scan quota exceeded for this billing period. Please upgrade your plan."
                    );
                }
            }
            Ok(r) => {
                eprintln!("[analyze] quota RPC failed: status={}", r.status());
                return error_response("Could not verify scan quota. Please try again.");
            }
            Err(e) => {
                eprintln!("[analyze] quota RPC error: {}", e);
                return error_response("Could not verify scan quota. Please try again.");
            }
        }

        id
    } else {
        eprintln!("{}", json!({"event":"guest_scan","request_id":request_id,"ms":req_start.elapsed().as_millis() as u64}));
        "guest".to_string()
    };

    // ── Extract + validate URL ────────────────────────────────────
    let url = match parsed["url"].as_str() {
        Some(u) if !u.is_empty() => u.to_string(),
        _ => return error_response("Missing URL"),
    };

    let platform = match detect_platform(&url) {
        Some(p) => p,
        None => return error_response(
            "Unsupported platform. Paste a YouTube, TikTok, X (Twitter), or Instagram URL."
        ),
    };

    let thumbnail = match get_thumbnail(&client, &platform).await {
        Ok(t) => t,
        Err(e) => return error_response(&e),
    };

    let api_user = match std::env::var("SIGHTENGINE_API_USER") {
        Ok(v) => v,
        Err(_) => return error_response("Missing SIGHTENGINE_API_USER"),
    };
    let api_secret = match std::env::var("SIGHTENGINE_API_SECRET") {
        Ok(v) => v,
        Err(_) => return error_response("Missing SIGHTENGINE_API_SECRET"),
    };

    let deepware_token = std::env::var("DEEPWARE_AUTH_TOKEN").ok();
    let hf_token = std::env::var("HF_TOKEN").ok();

    let platform_name = match &platform {
        Platform::YouTube(_)   => "YouTube",
        Platform::TikTok(_)    => "TikTok",
        Platform::Twitter(_)   => "X/Twitter",
        Platform::Instagram(_) => "Instagram",
    };

    // ── Longer-timeout client for external AI calls ───────────────
    let ai_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .unwrap_or_else(|_| client.clone());

    // ── Parallel multi-tool analysis ──────────────────────────────
    let se_fut = call_sightengine(&ai_client, &thumbnail, &api_user, &api_secret);
    let dw_fut = call_deepware(&ai_client, &url, deepware_token.as_deref());
    let hf_fut = call_huggingface(&ai_client, &thumbnail, hf_token.as_deref());

    let (se_result, dw_result, hf_result) = tokio::join!(se_fut, dw_fut, hf_fut);

    // ── Collect scores and tool names ─────────────────────────────
    let mut scores: Vec<(f64, f64)> = Vec::new(); // (score, weight)
    let mut tools_used: Vec<&str> = Vec::new();
    let mut tool_errors: Vec<String> = Vec::new();

    match se_result {
        Ok(score) => {
            scores.push((score, 0.4));
            tools_used.push("Sightengine");
        }
        Err(e) => tool_errors.push(format!("Sightengine: {}", e)),
    }

    match dw_result {
        Ok(score) => {
            scores.push((score, 0.4));
            tools_used.push("Deepware");
        }
        Err(e) => tool_errors.push(format!("Deepware: {}", e)),
    }

    match hf_result {
        Ok(Some(score)) => {
            scores.push((score, 0.2));
            tools_used.push("HF");
        }
        Ok(None) => {} // HF not configured — skip silently
        Err(e) => tool_errors.push(format!("HF: {}", e)),
    }

    if scores.is_empty() {
        let err_detail = tool_errors.join("; ");
        eprintln!("{}", json!({"event":"all_tools_failed","request_id":request_id,"errors":err_detail}));
        return error_response("All detection tools failed. Please try again later.");
    }

    // ── Weighted average (re-normalize to available tools) ────────
    let total_weight: f64 = scores.iter().map(|(_, w)| w).sum();
    let combined_score: f64 = scores.iter().map(|(s, w)| s * w).sum::<f64>() / total_weight;

    let tool_count = tools_used.len();
    let plural = if tool_count > 1 { "s" } else { "" };

    let (verdict, confidence, details) = if combined_score > 0.65 {
        (
            "ai_generated",
            combined_score,
            format!(
                "Fake — Combined AI probability: {:.0}% across {} tool{}. Scanned {} thumbnail.",
                combined_score * 100.0, tool_count, plural, platform_name
            ),
        )
    } else if combined_score < 0.35 {
        (
            "likely_real",
            1.0 - combined_score,
            format!(
                "Real — Combined AI probability: {:.0}% across {} tool{}. Scanned {} thumbnail.",
                combined_score * 100.0, tool_count, plural, platform_name
            ),
        )
    } else {
        (
            "unsure",
            combined_score,
            format!(
                "Inconclusive — Combined AI probability: {:.0}% across {} tool{}. Scanned {} thumbnail. The result is uncertain; try a different link or check manually.",
                combined_score * 100.0, tool_count, plural, platform_name
            ),
        )
    };

    if !tool_errors.is_empty() {
        eprintln!("{}", json!({"event":"partial_tool_failure","request_id":request_id,"errors":tool_errors.join("; ")}));
    }

    eprintln!("{}", json!({
        "event":          "scan_complete",
        "request_id":     request_id,
        "user_id":        user_id,
        "platform":       platform_name,
        "verdict":        verdict,
        "combined_score": combined_score,
        "tools_used":     tools_used,
        "ms":             req_start.elapsed().as_millis() as u64
    }));

    (
        StatusCode::OK,
        Json(json!({
            "verdict":    verdict,
            "confidence": confidence,
            "details":    details,
            "thumbnail":  thumbnail,
            "platform":   platform_name,
            "tools_used": tools_used
        })),
    ).into_response()
}

// ── Sightengine ───────────────────────────────────────────────────
async fn call_sightengine(
    client: &reqwest::Client,
    thumbnail_url: &str,
    api_user: &str,
    api_secret: &str,
) -> Result<f64, String> {
    let result = client
        .get("https://api.sightengine.com/1.0/check.json")
        .query(&[
            ("url",        thumbnail_url),
            ("models",     "genai"),
            ("api_user",   api_user),
            ("api_secret", api_secret),
        ])
        .send()
        .await
        .map_err(|e| format!("request failed: {}", e))?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("parse failed: {}", e))?;

    if result["status"] != "success" {
        let err = result.get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
            .unwrap_or("unknown error");
        return Err(err.to_string());
    }

    Ok(result["type"]["ai_generated"].as_f64().unwrap_or(0.0))
}

// ── Deepware (multipart/form-data with field "videourl") ──────────
async fn call_deepware(
    client: &reqwest::Client,
    video_url: &str,
    token: Option<&str>,
) -> Result<f64, String> {
    let token = match token {
        Some(t) if !t.is_empty() => t,
        _ => return Err("not configured".to_string()),
    };

    let form = reqwest::multipart::Form::new()
        .text("videourl", video_url.to_string());

    let resp = client
        .post("https://api.deepware.ai/api/v1/url/scan")
        .header("X-Deepware-Authentication", token)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("request failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("HTTP {} — {}", status, body));
    }

    let data = resp
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("parse failed: {}", e))?;

    // Deepware returns various result shapes — try common paths
    if let Some(score) = data["data"]["deepfake_probability"].as_f64() {
        return Ok(score);
    }
    if let Some(score) = data["result"]["deepfake_probability"].as_f64() {
        return Ok(score);
    }
    if let Some(score) = data["deepfake_probability"].as_f64() {
        return Ok(score);
    }
    if let Some(label) = data["result"]["label"].as_str() {
        let conf = data["result"]["confidence"].as_f64().unwrap_or(0.5);
        return match label.to_lowercase().as_str() {
            "fake" | "deepfake" => Ok(conf),
            "real" | "authentic" => Ok(1.0 - conf),
            _ => Ok(0.5),
        };
    }

    Err("unexpected response format".to_string())
}

// ── Hugging Face (optional — labels "Deepfake" / "Real") ─────────
async fn call_huggingface(
    client: &reqwest::Client,
    thumbnail_url: &str,
    token: Option<&str>,
) -> Result<Option<f64>, String> {
    let token = match token {
        Some(t) if !t.is_empty() => t,
        _ => return Ok(None), // not configured — not an error
    };

    // Fetch thumbnail bytes for binary upload
    let img_bytes = client
        .get(thumbnail_url)
        .send()
        .await
        .map_err(|e| format!("thumbnail fetch failed: {}", e))?
        .bytes()
        .await
        .map_err(|e| format!("thumbnail read failed: {}", e))?;

    let resp = client
        .post("https://api-inference.huggingface.co/models/prithivMLmods/Deep-Fake-Detector-v2-Model")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/octet-stream")
        .body(img_bytes)
        .send()
        .await
        .map_err(|e| format!("request failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("HTTP {} — {}", status, body));
    }

    let data = resp
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("parse failed: {}", e))?;

    // HF image-classification: [{label, score}, ...] or [[{label, score}, ...]]
    // Model returns "Deepfake" / "Real" labels
    if let Some(arr) = data.as_array() {
        for item in arr {
            let label = item["label"].as_str().unwrap_or("").to_lowercase();
            let score = item["score"].as_f64().unwrap_or(0.0);
            if label.contains("fake") || label.contains("ai") || label.contains("synthetic") || label.contains("deepfake") {
                return Ok(Some(score));
            }
        }
        for item in arr {
            let label = item["label"].as_str().unwrap_or("").to_lowercase();
            let score = item["score"].as_f64().unwrap_or(0.0);
            if label.contains("real") || label.contains("authentic") || label.contains("human") {
                return Ok(Some(1.0 - score));
            }
        }
    }
    // Nested array format [[{label, score}]]
    if let Some(outer) = data.get(0).and_then(|v| v.as_array()) {
        for item in outer {
            let label = item["label"].as_str().unwrap_or("").to_lowercase();
            let score = item["score"].as_f64().unwrap_or(0.0);
            if label.contains("fake") || label.contains("deepfake") {
                return Ok(Some(score));
            }
        }
        for item in outer {
            let label = item["label"].as_str().unwrap_or("").to_lowercase();
            let score = item["score"].as_f64().unwrap_or(0.0);
            if label.contains("real") {
                return Ok(Some(1.0 - score));
            }
        }
    }

    Err("could not parse classification result".to_string())
}

fn error_response(msg: &str) -> axum::response::Response {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({ "error": msg })),
    ).into_response()
}

// ────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    // ── helpers ──────────────────────────────────────────────────

    fn is_youtube(url: &str) -> bool {
        matches!(detect_platform(url), Some(Platform::YouTube(_)))
    }
    fn is_tiktok(url: &str) -> bool {
        matches!(detect_platform(url), Some(Platform::TikTok(_)))
    }
    fn is_twitter(url: &str) -> bool {
        matches!(detect_platform(url), Some(Platform::Twitter(_)))
    }
    fn is_instagram(url: &str) -> bool {
        matches!(detect_platform(url), Some(Platform::Instagram(_)))
    }
    fn is_none(url: &str) -> bool {
        detect_platform(url).is_none()
    }

    // ── YouTube detection ─────────────────────────────────────────

    #[test]
    fn youtube_watch_url() {
        assert!(is_youtube("https://www.youtube.com/watch?v=dQw4w9WgXcQ"));
    }

    #[test]
    fn youtube_short_url() {
        assert!(is_youtube("https://youtu.be/dQw4w9WgXcQ"));
    }

    #[test]
    fn youtube_shorts_url() {
        assert!(is_youtube("https://www.youtube.com/shorts/dQw4w9WgXcQ"));
    }

    #[test]
    fn youtube_embed_url() {
        assert!(is_youtube("https://www.youtube.com/embed/dQw4w9WgXcQ"));
    }

    #[test]
    fn youtube_live_url() {
        assert!(is_youtube("https://www.youtube.com/live/dQw4w9WgXcQ"));
    }

    #[test]
    fn youtube_extracts_correct_id() {
        match detect_platform("https://www.youtube.com/watch?v=dQw4w9WgXcQ") {
            Some(Platform::YouTube(id)) => assert_eq!(id, "dQw4w9WgXcQ"),
            _ => panic!("Expected YouTube platform with ID"),
        }
    }

    #[test]
    fn youtube_watch_with_extra_params() {
        assert!(is_youtube(
            "https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=42s&list=PLxxx"
        ));
    }

    // ── TikTok detection ──────────────────────────────────────────

    #[test]
    fn tiktok_long_url() {
        assert!(is_tiktok(
            "https://www.tiktok.com/@user/video/7123456789012345678"
        ));
    }

    #[test]
    fn tiktok_short_url() {
        assert!(is_tiktok("https://vm.tiktok.com/ZMeABCDEF/"));
    }

    #[test]
    fn tiktok_t_url() {
        assert!(is_tiktok("https://www.tiktok.com/t/ZTRabc123/"));
    }

    #[test]
    fn tiktok_any_tiktok_domain() {
        assert!(is_tiktok("https://www.tiktok.com/@someone/video/999"));
    }

    // ── X / Twitter detection ─────────────────────────────────────

    #[test]
    fn twitter_status_url() {
        assert!(is_twitter(
            "https://twitter.com/elonmusk/status/1234567890123456789"
        ));
    }

    #[test]
    fn x_status_url() {
        assert!(is_twitter(
            "https://x.com/elonmusk/status/1234567890123456789"
        ));
    }

    #[test]
    fn x_status_with_query_params() {
        assert!(is_twitter(
            "https://x.com/user/status/1234567890123456789?s=20"
        ));
    }

    // ── Instagram detection ───────────────────────────────────────

    #[test]
    fn instagram_post_url() {
        assert!(is_instagram("https://www.instagram.com/p/ABC123xyz/"));
    }

    #[test]
    fn instagram_reel_url() {
        assert!(is_instagram("https://www.instagram.com/reel/ABC123xyz/"));
    }

    #[test]
    fn instagram_tv_url() {
        assert!(is_instagram("https://www.instagram.com/tv/ABC123xyz/"));
    }

    // ── Unsupported / garbage URLs ────────────────────────────────

    #[test]
    fn random_url_returns_none() {
        assert!(is_none("https://example.com/some/page"));
    }

    #[test]
    fn empty_string_returns_none() {
        assert!(is_none(""));
    }

    #[test]
    fn facebook_url_returns_none() {
        assert!(is_none("https://www.facebook.com/video/12345"));
    }

    #[test]
    fn youtube_homepage_returns_none() {
        assert!(is_none("https://www.youtube.com/"));
    }

    // ── url_encode ────────────────────────────────────────────────

    #[test]
    fn encode_plain_ascii_unchanged() {
        assert_eq!(url_encode("hello"), "hello");
    }

    #[test]
    fn encode_space() {
        assert_eq!(url_encode("hello world"), "hello%20world");
    }

    #[test]
    fn encode_ampersand_and_equals() {
        assert_eq!(url_encode("a=1&b=2"), "a%3D1%26b%3D2");
    }

    #[test]
    fn encode_full_tiktok_url() {
        let url = "https://www.tiktok.com/@user/video/123";
        let encoded = url_encode(url);
        assert!(encoded.contains("%3A"));
        assert!(encoded.contains("%2F"));
        assert!(!encoded.contains(':'));
        assert!(!encoded.contains('/'));
    }

    #[test]
    fn encode_unreserved_chars_unchanged() {
        let s = "ABCabc123-_.~";
        assert_eq!(url_encode(s), s);
    }
}
