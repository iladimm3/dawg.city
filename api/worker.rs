use tokio::time::{sleep, Duration};
use reqwest::Client;
use reqwest::header::CONTENT_TYPE;
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

async fn process_job(
    client: &Client,
    job: &Job,
    hf_api_key: Option<&str>,
    hf_model: &str,
) -> Result<Value, Box<dyn std::error::Error>> {
    eprintln!("[worker] processing job id={} url={}", job.id, job.url);

    if let Some(hf_key) = hf_api_key {
        // 1) Fetch the target URL (expected to be an image/thumbnail)
        let fetched = client.get(&job.url).send().await?;
        if !fetched.status().is_success() {
            return Err(format!("failed to fetch url {}: status {}", job.url, fetched.status()).into());
        }
        let content_type = fetched
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();
        if !content_type.starts_with("image/") {
            return Err(format!("unsupported content-type for HF inference: {}", content_type).into());
        }
        let bytes = fetched.bytes().await?;

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
