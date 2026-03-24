use vercel_runtime::{run, Body, Error, Request, Response, StatusCode};
use serde_json::json;
use once_cell::sync::Lazy;
use regex::Regex;

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

const RECAPTCHA_SECRET: &str = "6LdrpZIsAAAAAK3PDZSvYxYhF09-oB28hhpalscV";
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

        Platform::Instagram(_url) => {
            Err("Instagram scanning is coming soon. Due to platform restrictions, Instagram thumbnails can't be fetched automatically. Try a YouTube, TikTok, or X link instead!".to_string())
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(handler).await
}

pub async fn handler(req: Request) -> Result<Response<Body>, Error> {
    if req.method() == "OPTIONS" {
        return Ok(Response::builder()
            .status(StatusCode::NO_CONTENT)
            .header("Access-Control-Allow-Origin", "https://dawg.city")
            .header("Access-Control-Allow-Methods", "POST, OPTIONS")
            .header("Access-Control-Allow-Headers", "Content-Type")
            .body(Body::Empty)?);
    }

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

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default();

    let recaptcha_resp = client
        .post("https://www.google.com/recaptcha/api/siteverify")
        .form(&[("secret", RECAPTCHA_SECRET), ("response", &recaptcha_token)])
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    let success = recaptcha_resp["success"].as_bool().unwrap_or(false);
    let score   = recaptcha_resp["score"].as_f64().unwrap_or(0.0);

    if !success || score < RECAPTCHA_MIN_SCORE {
        return error_response("reCAPTCHA verification failed. Please try again.");
    }

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

    if result["status"] != "success" {
        let err = result.get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
            .unwrap_or("Unknown Sightengine error");
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

fn error_response(msg: &str) -> Result<Response<Body>, Error> {
    Ok(Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", "https://dawg.city")
        .body(Body::Text(json!({ "error": msg }).to_string()))?)
}
