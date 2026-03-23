use vercel_runtime::{run, Body, Error, Request, Response, StatusCode};
use serde_json::json;
use once_cell::sync::Lazy;
use regex::Regex;

static YT_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| vec![
    Regex::new(r"(?:youtube\.com/watch\?v=)([a-zA-Z0-9_-]{11})").unwrap(),
    Regex::new(r"(?:youtu\.be/)([a-zA-Z0-9_-]{11})").unwrap(),
    Regex::new(r"(?:youtube\.com/shorts/)([a-zA-Z0-9_-]{11})").unwrap(),
    Regex::new(r"(?:youtube\.com/embed/)([a-zA-Z0-9_-]{11})").unwrap(),
    Regex::new(r"(?:youtube\.com/live/)([a-zA-Z0-9_-]{11})").unwrap(),
]);

// Known AI video generation tools — checked against title, description, tags, channel name
const AI_VIDEO_KEYWORDS: &[&str] = &[
    "seedance", "seed dance",
    "kling", "kling ai",
    "sora", "openai sora",
    "runway", "runway ml", "runway gen",
    "pika", "pika labs",
    "hailuo", "minimax hailuo",
    "luma", "luma dream machine", "luma ai",
    "cogvideo", "cogvideox",
    "stable video", "svd",
    "gen-2", "gen-3",
    "vidu", "vidu ai",
    "mochi", "genmo mochi",
    "wan", "wan2", "wanvideo",
    "hunyuan video",
    "bytedance", "byte dance",
    "ai generated", "ai-generated", "made with ai",
    "ai video", "ai shorts", "ai film",
    "deepfake", "deep fake",
    "synthesia",
    "heygen", "hey gen",
    "d-id",
];

const RECAPTCHA_SECRET: &str = "6LdrpZIsAAAAAK3PDZSvYxYhF09-oB28hhpalscV";
const RECAPTCHA_MIN_SCORE: f64 = 0.5;

// HuggingFace model: ViT-based deepfake/AI image detector (~92% accuracy)
const HF_MODEL: &str = "prithivMLmods/Deep-Fake-Detector-v2-Model";

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(handler).await
}

pub async fn handler(req: Request) -> Result<Response<Body>, Error> {
    // Handle CORS preflight
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

    let parsed: serde_json::Value = serde_json::from_str(&body)
        .unwrap_or(json!({}));

    // Verify reCAPTCHA token
    let recaptcha_token = match parsed["recaptcha_token"].as_str() {
        Some(t) if !t.is_empty() => t.to_string(),
        _ => return error_response("Missing reCAPTCHA token"),
    };

    let client = reqwest::Client::new();

    let recaptcha_resp = client
        .post("https://www.google.com/recaptcha/api/siteverify")
        .form(&[
            ("secret", RECAPTCHA_SECRET),
            ("response", &recaptcha_token),
        ])
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    let success = recaptcha_resp["success"].as_bool().unwrap_or(false);
    let score = recaptcha_resp["score"].as_f64().unwrap_or(0.0);

    if !success || score < RECAPTCHA_MIN_SCORE {
        return error_response("reCAPTCHA verification failed. Please try again.");
    }

    // Parse URL
    let url = match parsed["url"].as_str() {
        Some(u) if !u.is_empty() => u.to_string(),
        _ => return error_response("Missing URL"),
    };

    let video_id = YT_PATTERNS.iter()
        .find_map(|re| re.captures(&url).map(|caps| caps[1].to_string()));

    let video_id = match video_id {
        Some(id) => id,
        None => return error_response("Unsupported platform. YouTube links only for now."),
    };

    let thumbnail = format!("https://img.youtube.com/vi/{}/maxresdefault.jpg", video_id);

    // ── Env vars ──
    let api_user = match std::env::var("SIGHTENGINE_API_USER") {
        Ok(v) => v,
        Err(_) => return error_response("Missing SIGHTENGINE_API_USER"),
    };
    let api_secret = match std::env::var("SIGHTENGINE_API_SECRET") {
        Ok(v) => v,
        Err(_) => return error_response("Missing SIGHTENGINE_API_SECRET"),
    };
    let yt_api_key = std::env::var("YOUTUBE_API_KEY").ok();
    let hf_api_key = std::env::var("HF_API_KEY").ok();

    // ── Run all 3 tasks concurrently ──
    let sightengine_fut = call_sightengine(&client, &thumbnail, &api_user, &api_secret);
    let yt_meta_fut = fetch_yt_metadata(&client, &video_id, yt_api_key.as_deref());
    let hf_fut = call_huggingface(&client, &thumbnail, hf_api_key.as_deref());

    let (sightengine_result, yt_meta, hf_result) =
        tokio::join!(sightengine_fut, yt_meta_fut, hf_fut);

    // ── Sightengine score (required) ──
    let se_score = match sightengine_result {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    // ── HuggingFace score (optional — graceful fallback) ──
    let hf_score: Option<f32> = hf_result.ok().flatten();

    // ── YouTube metadata keyword scan ──
    let (keyword_hit, keyword_tool) = match &yt_meta {
        Ok(meta) => scan_keywords(meta),
        Err(_) => (false, None),
    };

    // ── Blend scores ──
    // HF available: 50% Sightengine + 50% HF
    // HF unavailable: 100% Sightengine
    let blended_score: f32 = match hf_score {
        Some(hf) => se_score * 0.5 + hf * 0.5,
        None => se_score,
    };

    // Keyword hit: force score to at least 0.85 (strong AI signal from metadata)
    let final_score: f32 = if keyword_hit {
        blended_score.max(0.85)
    } else {
        blended_score
    };

    // ── Verdict ──
    let (verdict, confidence, details) = build_verdict(
        final_score,
        se_score,
        hf_score,
        keyword_hit,
        keyword_tool.as_deref(),
    );

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", "https://dawg.city")
        .body(Body::Text(json!({
            "verdict": verdict,
            "confidence": confidence,
            "details": details,
            "thumbnail": thumbnail,
            "keyword_hit": keyword_hit,
            "keyword_tool": keyword_tool,
            "hf_available": hf_score.is_some(),
        }).to_string()))?)
}

// ── Sightengine call ──
async fn call_sightengine(
    client: &reqwest::Client,
    thumbnail: &str,
    api_user: &str,
    api_secret: &str,
) -> Result<f32, String> {
    let result = client
        .get("https://api.sightengine.com/1.0/check.json")
        .query(&[
            ("url", thumbnail),
            ("models", "genai"),
            ("api_user", api_user),
            ("api_secret", api_secret),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| e.to_string())?;

    if result["status"] != "success" {
        let err = result.get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
            .unwrap_or("Unknown Sightengine error");
        return Err(err.to_string());
    }

    Ok(result["type"]["ai_generated"].as_f64().unwrap_or(0.0) as f32)
}

// ── YouTube Data API v3 metadata fetch ──
async fn fetch_yt_metadata(
    client: &reqwest::Client,
    video_id: &str,
    api_key: Option<&str>,
) -> Result<serde_json::Value, String> {
    let key = match api_key {
        Some(k) if !k.is_empty() => k,
        _ => return Err("No YouTube API key configured".to_string()),
    };

    let resp = client
        .get("https://www.googleapis.com/youtube/v3/videos")
        .query(&[
            ("part", "snippet"),
            ("id", video_id),
            ("key", key),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| e.to_string())?;

    let item = resp["items"]
        .as_array()
        .and_then(|a| a.first())
        .ok_or_else(|| "Video not found".to_string())?;

    Ok(item["snippet"].clone())
}

// ── HuggingFace Inference API (image classification on thumbnail) ──
async fn call_huggingface(
    client: &reqwest::Client,
    thumbnail_url: &str,
    api_key: Option<&str>,
) -> Result<Option<f32>, String> {
    let key = match api_key {
        Some(k) if !k.is_empty() => k,
        _ => return Ok(None), // No key — skip gracefully
    };

    // Download thumbnail image bytes
    let img_bytes = client
        .get(thumbnail_url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;

    // POST raw image bytes to HF Inference API
    let hf_resp = client
        .post(format!(
            "https://api-inference.huggingface.co/models/{}",
            HF_MODEL
        ))
        .header("Authorization", format!("Bearer {}", key))
        .header("Content-Type", "image/jpeg")
        .body(img_bytes)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| e.to_string())?;

    // Response: [{label: "Deepfake", score: 0.93}, {label: "Realism", score: 0.07}]
    let scores = match hf_resp.as_array() {
        Some(arr) => arr,
        None => return Ok(None),
    };

    let deepfake_score = scores.iter()
        .find(|item| {
            item["label"].as_str()
                .map(|l| l.to_lowercase().contains("deepfake") || l.to_lowercase().contains("fake"))
                .unwrap_or(false)
        })
        .and_then(|item| item["score"].as_f64())
        .map(|s| s as f32);

    Ok(deepfake_score)
}

// ── Keyword scan across title, description, tags, channelTitle ──
fn scan_keywords(snippet: &serde_json::Value) -> (bool, Option<String>) {
    let mut haystack = String::new();

    if let Some(title) = snippet["title"].as_str() {
        haystack.push_str(&title.to_lowercase());
        haystack.push(' ');
    }
    if let Some(desc) = snippet["description"].as_str() {
        // Only first 500 chars of description — fast and enough
        let end = desc.char_indices().nth(500).map(|(i, _)| i).unwrap_or(desc.len());
        haystack.push_str(&desc[..end].to_lowercase());
        haystack.push(' ');
    }
    if let Some(channel) = snippet["channelTitle"].as_str() {
        haystack.push_str(&channel.to_lowercase());
        haystack.push(' ');
    }
    if let Some(tags) = snippet["tags"].as_array() {
        for tag in tags {
            if let Some(t) = tag.as_str() {
                haystack.push_str(&t.to_lowercase());
                haystack.push(' ');
            }
        }
    }

    for keyword in AI_VIDEO_KEYWORDS {
        if haystack.contains(keyword) {
            let tool_name = keyword_to_tool_name(keyword);
            return (true, Some(tool_name.to_string()));
        }
    }

    (false, None)
}

fn keyword_to_tool_name(keyword: &str) -> &'static str {
    match keyword {
        "seedance" | "seed dance" => "Seedance",
        "kling" | "kling ai" => "Kling AI",
        "sora" | "openai sora" => "OpenAI Sora",
        "runway" | "runway ml" | "runway gen" => "Runway",
        "pika" | "pika labs" => "Pika Labs",
        "hailuo" | "minimax hailuo" => "Hailuo (MiniMax)",
        "luma" | "luma dream machine" | "luma ai" => "Luma AI",
        "cogvideo" | "cogvideox" => "CogVideoX",
        "stable video" | "svd" => "Stable Video Diffusion",
        "gen-2" | "gen-3" => "Runway Gen",
        "vidu" | "vidu ai" => "Vidu AI",
        "mochi" | "genmo mochi" => "Genmo Mochi",
        "wan" | "wan2" | "wanvideo" => "WanVideo",
        "hunyuan video" => "HunyuanVideo",
        "bytedance" | "byte dance" => "ByteDance",
        "heygen" | "hey gen" => "HeyGen",
        "synthesia" => "Synthesia",
        "d-id" => "D-ID",
        _ => "Known AI Video Tool",
    }
}

// ── Build final verdict + details string ──
fn build_verdict(
    final_score: f32,
    se_score: f32,
    hf_score: Option<f32>,
    keyword_hit: bool,
    keyword_tool: Option<&str>,
) -> (&'static str, f32, String) {
    let is_fake = final_score >= 0.5;
    let verdict = if is_fake { "ai_generated" } else { "likely_real" };
    let confidence = if is_fake { final_score } else { 1.0 - final_score };

    let mut details = if is_fake {
        format!("AI-generated — {:.0}% confidence.", final_score * 100.0)
    } else {
        format!("Likely real — only {:.0}% AI probability.", final_score * 100.0)
    };

    if keyword_hit {
        if let Some(tool) = keyword_tool {
            details.push_str(&format!(
                " ⚠️ Metadata mentions {}, a known AI video generator.",
                tool
            ));
        }
    }

    if let Some(hf) = hf_score {
        details.push_str(&format!(
            " (Sightengine: {:.0}% · HF detector: {:.0}%)",
            se_score * 100.0,
            hf * 100.0
        ));
    }

    (verdict, confidence, details)
}

fn error_response(msg: &str) -> Result<Response<Body>, Error> {
    Ok(Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", "https://dawg.city")
        .body(Body::Text(json!({ "error": msg }).to_string()))?)
}
