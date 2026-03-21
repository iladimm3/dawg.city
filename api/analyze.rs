use serde::{Deserialize, Serialize}; 
use vercel_runtime::{run, service_fn, Body, Error, Request, Response, StatusCode}; 
use serde_json::json; 
use once_cell::sync::Lazy; 
use regex::Regex; 
  
#[derive(Deserialize)] 
struct RequestBody { 
    url: String, 
} 
  
#[derive(Serialize)] 
struct ResponseBody { 
    verdict: String, 
    confidence: f32, 
    details: String, 
    thumbnail: Option<String>, 
} 
  
static YT_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| vec![ 
    Regex::new(r"(?:youtube\.com/watch\?v=)([a-zA-Z0-9_-]{11})").unwrap(), 
    Regex::new(r"(?:youtu\.be/)([a-zA-Z0-9_-]{11})").unwrap(), 
    Regex::new(r"(?:youtube\.com/shorts/)([a-zA-Z0-9_-]{11})").unwrap(), 
    Regex::new(r"(?:youtube\.com/embed/)([a-zA-Z0-9_-]{11})").unwrap(), 
    Regex::new(r"(?:youtube\.com/live/)([a-zA-Z0-9_-]{11})").unwrap(), 
]); 
  
const ALLOWED_ORIGIN: &str = "https://dawg.city"; 
  
#[tokio::main] 
async fn main() -> Result<(), Error> { 
    run(service_fn(analyze_handler)).await 
} 
  
async fn analyze_handler(req: Request) -> Result<Response<Body>, Error> { 
    if req.method() == "OPTIONS" { return preflight_response(); } 
    if req.method() != "POST" { 
        return json_error(StatusCode::METHOD_NOT_ALLOWED, "Only POST method allowed"); 
    } 
  
    let body_bytes = match req.body() { 
        Body::Text(s) => s.as_bytes().to_vec(), 
        Body::Binary(b) => b.to_vec(), 
        Body::Empty => return json_error(StatusCode::BAD_REQUEST, "Empty request body"), 
    }; 
  
    let req_json: RequestBody = match serde_json::from_slice(&body_bytes) { 
        Ok(r) => r, 
        Err(_) => return json_error( 
            StatusCode::BAD_REQUEST, 
            "Invalid JSON. Expected: {\"url\": \"https://...\"}" 
        ), 
    }; 
  
    if !req_json.url.starts_with("http://") && !req_json.url.starts_with("https://") { 
        return json_error(StatusCode::BAD_REQUEST, "URL must start with http:// or https://"); 
    } 
  
    let video_id = YT_PATTERNS.iter() 
        .find_map(|re| re.captures(&req_json.url).map(|caps| caps[1].to_string())); 
  
    let thumbnail = match video_id { 
        Some(id) => format!("https://img.youtube.com/vi/{}/maxresdefault.jpg", id), 
        None => return json_error( 
            StatusCode::BAD_REQUEST, 
            "Unsupported platform. YouTube links only for now (youtube.com, youtu.be, shorts)." 
        ), 
    }; 
  
    let (verdict, confidence, details) = match call_sightengine(&thumbnail).await { 
        Ok(score) => { 
            if score > 0.65 { 
                ("ai_generated", score, format!("AI generation probability: {:.0}% — high confidence deepfake/AI artifact detected.", score * 100.0)) 
            } else if score < 0.35 { 
                ("likely_real", 1.0 - score, format!("Real image detected (AI probability only {:.0}%).", score * 100.0)) 
            } else { 
                ("unsure", 0.5, format!("Borderline result — AI probability {:.0}% (needs human review).", score * 100.0)) 
            } 
        } 
        Err(e) => ("unsure", 0.4, format!("Analysis error: {}", e)), 
    }; 
  
    let response_body = ResponseBody { 
        verdict: verdict.to_string(), 
        confidence, 
        details, 
        thumbnail: Some(thumbnail), 
    }; 
  
    let json_bytes = match serde_json::to_vec(&response_body) { 
        Ok(b) => b, 
        Err(_) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to serialize response"), 
    }; 
  
    Ok(Response::builder() 
        .status(StatusCode::OK) 
        .header("Content-Type", "application/json") 
        .header("Access-Control-Allow-Origin", ALLOWED_ORIGIN) 
        .body(json_bytes.into())?) 
} 
  
 fn preflight_response() -> Result<Response<Body>, Error> { 
     Ok(Response::builder() 
         .status(StatusCode::OK) 
         .header("Access-Control-Allow-Origin", ALLOWED_ORIGIN) 
         .header("Access-Control-Allow-Methods", "POST, OPTIONS") 
         .header("Access-Control-Allow-Headers", "Content-Type") 
         .body(vec![].into())?) 
 } 
  
 fn json_error(status: StatusCode, msg: &str) -> Result<Response<Body>, Error> { 
     let json = json!({"error": msg}); 
     Ok(Response::builder() 
         .status(status) 
         .header("Content-Type", "application/json") 
         .header("Access-Control-Allow-Origin", ALLOWED_ORIGIN) 
         .body(serde_json::to_vec(&json).unwrap().into())?) 
 } 
  
 async fn call_sightengine(image_url: &str) -> Result<f32, String> { 
     let client = reqwest::Client::new(); 
     let api_user = std::env::var("SIGHTENGINE_API_USER") 
         .map_err(|_| "SIGHTENGINE_API_USER env var missing".to_string())?; 
     let api_secret = std::env::var("SIGHTENGINE_API_SECRET") 
         .map_err(|_| "SIGHTENGINE_API_SECRET env var missing".to_string())?; 
  
     let resp = client 
         .get("https://api.sightengine.com/1.0/check.json") 
         .query(&[ 
             ("models", "genai"), 
             ("url", image_url), 
             ("api_user", &api_user), 
             ("api_secret", &api_secret), 
         ]) 
         .send() 
         .await 
         .map_err(|e| e.to_string())?; 
  
     let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?; 
  
     if json["status"] != "success" { 
         let err = json.get("error") 
             .and_then(|e| e.get("message")) 
             .and_then(|m| m.as_str()) 
             .unwrap_or("unknown Sightengine error"); 
         return Err(err.to_string()); 
     } 
  
     let score = json["type"]["ai_generated"] 
         .as_f64() 
         .unwrap_or(0.0) as f32; 
     Ok(score) 
 } 
