use tokio::time::{sleep, Duration};
use reqwest::Client;
use reqwest::header::CONTENT_TYPE;
use regex::Regex;
use serde_json::{json, Value};
use chrono::Utc;

#[derive(Debug)]
struct Job {
    id: String,
    user_id: Option<String>,
    url: String,
}

async fn fetch_and_claim_job(
    client: &Client,
    supabase_url: &str,
    service_key: &str,
) -> Result<Option<Job>, Box<dyn std::error::Error>> {
    let base = supabase_url.trim_end_matches('/');
    let get_url = format!("{}/rest/v1/jobs?status=eq.queued&select=id,user_id,url&order=created_at.asc&limit=1", base);
    let resp = client
        .get(&get_url)
        .header("apikey", service_key)
        .header("Authorization", format!("Bearer {}", service_key))
        .send()
        .await?;
    if !resp.status().is_success() {
        let t = resp.text().await.unwrap_or_default();
        eprintln!("[worker] fetch jobs failed: {} - {}", resp.status(), t);
        return Ok(None);
    }
    let arr = resp.json::<Value>().await?;
    let job_opt = arr.as_array().and_then(|a| a.get(0)).cloned();
    if job_opt.is_none() {
        return Ok(None);
    }
    let job_json = job_opt.unwrap();
    let id = job_json["id"].as_str().unwrap_or("").to_string();
    if id.is_empty() {
        return Ok(None);
    }

    // Attempt atomic claim: PATCH with both id and status filter
    let patch_url = format!("{}/rest/v1/jobs?id=eq.{}&status=eq.queued", base, id);
    let patch_resp = client
        .patch(&patch_url)
        .header("apikey", service_key)
        .header("Authorization", format!("Bearer {}", service_key))
        .header("Content-Type", "application/json")
        .header("Prefer", "return=representation")
        .json(&json!({"status": "processing", "updated_at": Utc::now().to_rfc3339()}))
        .send()
        .await?;
    if !patch_resp.status().is_success() {
        let t = patch_resp.text().await.unwrap_or_default();
        eprintln!("[worker] claim failed: {} - {}", patch_resp.status(), t);
        return Ok(None);
    }
    let claimed = patch_resp.json::<Value>().await?;
    let claimed_row = claimed.as_array().and_then(|a| a.get(0)).cloned();
    if claimed_row.is_none() {
        // someone else claimed it
        return Ok(None);
    }
    let row = claimed_row.unwrap();
    Ok(Some(Job {
        id: row["id"].as_str().unwrap_or("").to_string(),
        user_id: row.get("user_id").and_then(|v| v.as_str().map(|s| s.to_string())),
        url: row["url"].as_str().unwrap_or("").to_string(),
    }))
}

    // Helper: simple URL encode (works for building oembed queries)
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

    async fn try_fetch_image(client: &Client, url: &str) -> Result<Option<(bytes::Bytes, String)>, Box<dyn std::error::Error>> {
        let ua = "Mozilla/5.0 (compatible; dawg.city/1.0)";
        let resp = client.get(url).header("User-Agent", ua).send().await?;
        if !resp.status().is_success() {
            return Ok(None);
        }
        let content_type = resp
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();
        if !content_type.starts_with("image/") {
            return Ok(None);
        }
        let bytes = resp.bytes().await?;
        if bytes.len() < 64 {
            // too small to be a valid thumbnail
            return Ok(None);
        }
        Ok(Some((bytes, content_type)))
    }

    fn extract_youtube_id(url: &str) -> Option<String> {
        let patterns = [
            r"(?:youtube\.com/watch\?v=)([a-zA-Z0-9_-]{11})",
            r"(?:youtu\.be/)([a-zA-Z0-9_-]{11})",
            r"(?:youtube\.com/shorts/)([a-zA-Z0-9_-]{11})",
            r"(?:youtube\.com/embed/)([a-zA-Z0-9_-]{11})",
            r"(?:youtube\.com/live/)([a-zA-Z0-9_-]{11})",
        ];
        for pat in patterns.iter() {
            if let Ok(re) = Regex::new(pat) {
                if let Some(c) = re.captures(url) {
                    return Some(c[1].to_string());
                }
            }
        }
        None
    }

    async fn fetch_thumbnail_for_url(client: &Client, url: &str) -> Result<(bytes::Bytes, String), Box<dyn std::error::Error>> {
        // 1) Try direct fetch
        if let Some(img) = try_fetch_image(client, url).await? {
            return Ok(img);
        }

        // 2) YouTube heuristic
        if let Some(id) = extract_youtube_id(url) {
            let candidates = vec![
                format!("https://img.youtube.com/vi/{}/maxresdefault.jpg", id),
                format!("https://img.youtube.com/vi/{}/sddefault.jpg", id),
                format!("https://img.youtube.com/vi/{}/hqdefault.jpg", id),
                format!("https://img.youtube.com/vi/{}/default.jpg", id),
            ];
            for c in candidates.iter() {
                if let Some(img) = try_fetch_image(client, c).await? {
                    return Ok(img);
                }
            }
        }

        // 3) TikTok oembed
        if url.contains("tiktok.com") || url.contains("vm.tiktok.com") {
            let oembed = format!("https://www.tiktok.com/oembed?url={}", url_encode(url));
            if let Ok(resp) = client.get(&oembed).header("User-Agent", "Mozilla/5.0 (compatible; dawg.city/1.0)").send().await {
                if resp.status().is_success() {
                    if let Ok(json) = resp.json::<Value>().await {
                        if let Some(th_url) = json["thumbnail_url"].as_str() {
                            if let Some(img) = try_fetch_image(client, th_url).await? {
                                return Ok(img);
                            }
                        }
                    }
                }
            }
        }

        // 4) X / Twitter via api.fxtwitter.com
        if url.contains("twitter.com") || url.contains("x.com") {
            let mut fx = url.to_string().replace("twitter.com", "api.fxtwitter.com");
            fx = fx.replace("x.com", "api.fxtwitter.com");
            if let Ok(resp) = client.get(&fx).header("User-Agent", "Mozilla/5.0 (compatible; dawg.city/1.0)").send().await {
                if resp.status().is_success() {
                    if let Ok(json) = resp.json::<Value>().await {
                        if let Some(img) = json["tweet"]["media"]["photos"].get(0).and_then(|p| p["url"].as_str()) {
                            if let Some(imgb) = try_fetch_image(client, img).await? {
                                return Ok(imgb);
                            }
                        }
                        if let Some(img) = json["tweet"]["media"]["videos"].get(0).and_then(|v| v["thumbnail_url"].as_str()) {
                            if let Some(imgb) = try_fetch_image(client, img).await? {
                                return Ok(imgb);
                            }
                        }
                        if let Some(img) = json["tweet"]["author"]["avatar_url"].as_str() {
                            if let Some(imgb) = try_fetch_image(client, img).await? {
                                return Ok(imgb);
                            }
                        }
                    }
                }
            }
        }

        // 5) Instagram oembed (optional token)
        if url.contains("instagram.com") {
            let ig_token = std::env::var("INSTAGRAM_TOKEN").ok();
            let oembed = if let Some(tok) = ig_token {
                format!("https://graph.facebook.com/v18.0/instagram_oembed?url={}&maxwidth=800&access_token={}", url_encode(url), tok)
            } else {
                format!("https://graph.facebook.com/v18.0/instagram_oembed?url={}&maxwidth=800", url_encode(url))
            };
            if let Ok(resp) = client.get(&oembed).header("User-Agent", "Mozilla/5.0 (compatible; dawg.city/1.0)").send().await {
                if resp.status().is_success() {
                    if let Ok(json) = resp.json::<Value>().await {
                        if let Some(th) = json["thumbnail_url"].as_str() {
                            if let Some(imgb) = try_fetch_image(client, th).await? {
                                return Ok(imgb);
                            }
                        }
                    }
                }
            }
        }

        Err(format!("failed to obtain thumbnail for URL: {}", url).into())
    }

async fn process_job(
    client: &Client,
    job: &Job,
    hf_api_key: Option<&str>,
    hf_model: &str,
) -> Result<Value, Box<dyn std::error::Error>> {
    eprintln!("[worker] processing job id={} url={}", job.id, job.url);

    if let Some(hf_key) = hf_api_key {
        // 1) Obtain an image/thumbnail (direct or via platform fallback)
        let (bytes, content_type) = fetch_thumbnail_for_url(client, &job.url).await?;

        // 2) Call Hugging Face Inference API (binary image upload)
        let hf_url = format!("https://api-inference.huggingface.co/models/{}", hf_model);
        let hf_resp = client
            .post(&hf_url)
            .header("Authorization", format!("Bearer {}", hf_key))
            .header(CONTENT_TYPE, content_type.clone())
            .body(bytes)
            .send()
            .await?;

        let status = hf_resp.status();
        let hf_text = hf_resp.text().await?;
        if !status.is_success() {
            return Err(format!("HF inference failed: {} - {}", status, hf_text).into());
        }
        let hf_json: Value = serde_json::from_str(&hf_text)?;

        // Map to a simple verdict if present, otherwise include raw HF output
        let verdict = hf_json.get(0)
            .and_then(|v| v.get("label"))
            .and_then(|l| l.as_str())
            .or_else(|| hf_json.get("label").and_then(|l| l.as_str()))
            .unwrap_or("unknown")
            .to_string();
        let confidence = hf_json.get(0)
            .and_then(|v| v.get("score"))
            .and_then(|s| s.as_f64())
            .or_else(|| hf_json.get("score").and_then(|s| s.as_f64()))
            .unwrap_or(0.0);

        Ok(json!({
            "verdict": verdict,
            "confidence": confidence,
            "hf_raw": hf_json
        }))
    } else {
        // fallback: simulated result when no HF key is configured
        sleep(Duration::from_secs(1)).await;
        Ok(json!({
            "verdict": "unknown",
            "confidence": 0.0,
            "details": "No HF API key; result simulated by worker prototype"
        }))
    }
}

async fn finalize_job(
    client: &Client,
    supabase_url: &str,
    service_key: &str,
    job_id: &str,
    result: Result<Value, Box<dyn std::error::Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let base = supabase_url.trim_end_matches('/');
    let (status_str, payload) = match result {
        Ok(v) => ("done", json!({"status": "done", "result": v, "updated_at": Utc::now().to_rfc3339()})),
        Err(e) => {
            eprintln!("[worker] job {} failed: {}", job_id, e);
            ("failed", json!({"status": "failed", "error": format!("{}", e), "updated_at": Utc::now().to_rfc3339()}))
        }
    };
    let patch_url = format!("{}/rest/v1/jobs?id=eq.{}", base, job_id);
    let resp = client
        .patch(&patch_url)
        .header("apikey", service_key)
        .header("Authorization", format!("Bearer {}", service_key))
        .header("Content-Type", "application/json")
        .header("Prefer", "return=representation")
        .json(&payload)
        .send()
        .await?;
    if !resp.status().is_success() {
        let t = resp.text().await.unwrap_or_default();
        eprintln!("[worker] finalize failed: {} - {}", resp.status(), t);
    } else {
        eprintln!("[worker] job {} finalized as {}", job_id, status_str);
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let supabase_url = std::env::var("SUPABASE_URL").expect("SUPABASE_URL required");
    let service_key = std::env::var("SUPABASE_SERVICE_ROLE_KEY").expect("SUPABASE_SERVICE_ROLE_KEY required");
    let hf_api_key = std::env::var("HF_API_KEY").ok();
    let hf_model = std::env::var("HF_MODEL").unwrap_or_else(|_| "naman712/seedance".to_string());

    let client = Client::builder().timeout(std::time::Duration::from_secs(30)).build()?;

    eprintln!("[worker] started, polling {}/rest/v1/jobs", supabase_url);

    loop {
        match fetch_and_claim_job(&client, &supabase_url, &service_key).await {
            Ok(Some(job)) => {
                eprintln!("[worker] claimed job {}", job.id);
                let res = process_job(&client, &job, hf_api_key.as_deref(), &hf_model).await;
                if let Err(e) = finalize_job(&client, &supabase_url, &service_key, &job.id, res).await {
                    eprintln!("[worker] failed to finalize job {}: {}", job.id, e);
                }
            }
            Ok(None) => {
                // nothing to do
                sleep(Duration::from_secs(5)).await;
            }
            Err(e) => {
                eprintln!("[worker] error fetching/claiming job: {}", e);
                sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_unreserved_chars_unchanged() {
        // RFC 3986 unreserved chars: A-Z a-z 0-9 - _ . ~
        let s = "ABCabc123-_.~";
        assert_eq!(url_encode(s), s);
    }

    #[test]
    fn url_encode_reserved_chars() {
        assert_eq!(url_encode("a b"), "a%20b");
        assert_eq!(url_encode("foo@bar"), "foo%40bar");
    }

    #[test]
    fn extract_youtube_id_variants() {
        let id = "dQw4w9WgXcQ";
        let urls = vec![
            format!("https://www.youtube.com/watch?v={}", id),
            format!("https://youtu.be/{}", id),
            format!("https://www.youtube.com/shorts/{}", id),
            format!("https://www.youtube.com/embed/{}", id),
            format!("https://www.youtube.com/live/{}", id),
        ];
        for u in urls {
            assert_eq!(extract_youtube_id(&u), Some(id.to_string()));
        }
        assert_eq!(extract_youtube_id("https://example.com/"), None);
    }
}
