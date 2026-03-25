use vercel_runtime::{run, Body, Error, Request, Response, StatusCode};
use serde_json::json;
use once_cell::sync::Lazy;
use regex::Regex;
use std::time::Instant;

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
            // Use fxtwitter API to extract tweet media
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

            // Try tweet photos first
            if let Some(img) = fx_resp["tweet"]["media"]["photos"]
                .get(0)
                .and_then(|p| p["url"].as_str())
            {
                return Ok(img.to_string());
            }
            // Try video thumbnail
            if let Some(img) = fx_resp["tweet"]["media"]["videos"]
                .get(0)
                .and_then(|v| v["thumbnail_url"].as_str())
            {
                return Ok(img.to_string());
            }
            // Fall back to author avatar
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

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(handler).await
}

pub async fn handler(req: Request) -> Result<Response<Body>, Error> {
    if req.method() == "OPTIONS" {
pub async fn handler(req: Request) -> Result<Response<Body>, Error> {
    if req.method() == "OPTIONS" {
        return Ok(Response::builder()
            .status(StatusCode::NO_CONTENT)
            .header("Access-Control-Allow-Origin", "https://dawg.city")
            .header("Access-Control-Allow-Methods", "POST, OPTIONS")
            .header("Access-Control-Allow-Headers", "Content-Type, Authorization")
            .body(Body::Empty)?);
    }

    // ── Request tracing ──────────────────────────────────────────
    let req_start = Instant::now();
    let request_id = req.headers()
        .get("x-vercel-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    let body = match req.body() {
        Body::Text(s) => s.clone(),
        Body::Binary(b) => String::from_utf8_lossy(b).to_string(),
        Body::Empty => String::new(),
    };

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
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default();

    // ── reCAPTCHA verification ────────────────────────────────────
    let recaptcha_resp = client
        .post("https://www.google.com/recaptcha/api/siteverify")
        .form(&[("secret", recaptcha_secret.as_str()), ("response", recaptcha_token.as_str())])
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    let success = recaptcha_resp["success"].as_bool().unwrap_or(false);
    let score   = recaptcha_resp["score"].as_f64().unwrap_or(0.0);

    if !success || score < RECAPTCHA_MIN_SCORE {
        return error_response("reCAPTCHA verification failed. Please try again.");
    }

    // ── Authenticate caller via Supabase JWT ──────────────────────
    // Frontend must send:  Authorization: Bearer <supabase-session-token>
    let user_jwt = req.headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.to_string());

    let user_jwt = match user_jwt {
        Some(t) if !t.is_empty() => t,
        _ => {
            eprintln!("{}", json!({"event":"auth_missing","request_id":request_id,"ms":req_start.elapsed().as_millis() as u64}));
            return error_response("Authentication required");
        }
    };

    // Validate JWT with Supabase and retrieve the user record
    let auth_result = client
        .get(format!("{}/auth/v1/user", supabase_url))
        .header("apikey", &supabase_service_key)
        .header("Authorization", format!("Bearer {}", user_jwt))
        .send()
        .await;

    let user_id = match auth_result {
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

    // ── Atomic quota enforcement (server-authoritative) ───────────
    // Calls a Supabase SQL function that does:
    //   UPDATE profiles
    //   SET scan_count_month = scan_count_month + 1
    //   WHERE id = p_user_id AND scan_count_month < quota_limit
    //   RETURNING scan_count_month;
    // Returns NULL when quota is exhausted → no TOCTOU race possible.
    let quota_resp = client
        .post(format!("{}/rest/v1/rpc/increment_scan_quota", supabase_url))
        .header("apikey", &supabase_service_key)
        .header("Authorization", format!("Bearer {}", supabase_service_key))
        .header("Content-Type", "application/json")
        .json(&json!({ "p_user_id": user_id }))
        .send()
        .await;

    match quota_resp {
        Ok(r) if r.status().is_success() => {
            let body = r.json::<serde_json::Value>().await.unwrap_or(serde_json::Value::Null);
            if body.is_null() {
                eprintln!("{}", json!({"event":"quota_exceeded","request_id":request_id,"user_id":user_id,"ms":req_start.elapsed().as_millis() as u64}));
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

    let se_start = Instant::now();
    let result = client
        .get("https://api.sightengine.com/1.0/check.json")
        .query(&[
            ("url",        thumbnail.as_str()),
            ("models",     "genai"),
            ("api_user",   api_user.as_str()),
            ("api_secret", api_secret.as_str()),
        ])
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;
    let se_ms = se_start.elapsed().as_millis() as u64;

    if result["status"] != "success" {
        let err = result.get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
            .unwrap_or("Unknown Sightengine error");
        eprintln!("{}", json!({"event":"sightengine_error","request_id":request_id,"error":err,"sightengine_ms":se_ms,"ms":req_start.elapsed().as_millis() as u64}));
        return error_response(err);
    }

    let ai_score = result["type"]["ai_generated"].as_f64().unwrap_or(0.0) as f32;

    let platform_name = match &platform {
        Platform::YouTube(_)   => "YouTube",
        Platform::TikTok(_)    => "TikTok",
        Platform::Twitter(_)   => "X/Twitter",
        Platform::Instagram(_) => "Instagram",
    };

    let (verdict, confidence, details) = if ai_score >= 0.5 {
        (
            "ai_generated",
            ai_score,
            format!("Fake — AI generation probability: {:.0}% (scanned {} thumbnail).", ai_score * 100.0, platform_name),
        )
    } else {
        (
            "likely_real",
            1.0 - ai_score,
            format!("Real — AI generation probability only {:.0}% (scanned {} thumbnail).", ai_score * 100.0, platform_name),
        )
    };

    eprintln!("{}", json!({
        "event":          "scan_complete",
        "request_id":     request_id,
        "user_id":        user_id,
        "platform":       platform_name,
        "verdict":        verdict,
        "ai_score":       ai_score,
        "sightengine_ms": se_ms,
        "ms":             req_start.elapsed().as_millis() as u64
    }));

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", "https://dawg.city")
        .body(Body::Text(json!({
            "verdict":    verdict,
            "confidence": confidence,
            "details":    details,
            "thumbnail":  thumbnail,
            "platform":   platform_name
        }).to_string()))?)
}

    Ok(Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", "https://dawg.city")
        .body(Body::Text(json!({ "error": msg }).to_string()))?)
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
        // Catches any tiktok.com URL even if pattern doesn't match exactly
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
        // No video ID — should not match
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
        // Colons and slashes must be encoded
        assert!(encoded.contains("%3A")); // :
        assert!(encoded.contains("%2F")); // /
        assert!(!encoded.contains(':'));
        assert!(!encoded.contains('/'));
    }

    #[test]
    fn encode_unreserved_chars_unchanged() {
        // RFC 3986 unreserved chars: A-Z a-z 0-9 - _ . ~
        let s = "ABCabc123-_.~";
        assert_eq!(url_encode(s), s);
    }
}
