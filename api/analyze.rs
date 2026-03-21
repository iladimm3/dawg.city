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
  
     let url = match parsed["url"].as_str() { 
         Some(u) if !u.is_empty() => u.to_string(), 
         _ => return error_response("Missing URL"), 
     }; 
  
     let video_id = YT_PATTERNS.iter() 
         .find_map(|re| re.captures(&url).map(|caps| caps[1].to_string())); 
  
     let thumbnail = match video_id { 
         Some(id) => format!("https://img.youtube.com/vi/{}/maxresdefault.jpg", id), 
         None => return error_response("Unsupported platform. YouTube links only for now."), 
     }; 
  
     let api_user = match std::env::var("SIGHTENGINE_API_USER") { 
         Ok(v) => v, 
         Err(_) => return error_response("Missing SIGHTENGINE_API_USER"), 
     }; 
  
     let api_secret = match std::env::var("SIGHTENGINE_API_SECRET") { 
         Ok(v) => v, 
         Err(_) => return error_response("Missing SIGHTENGINE_API_SECRET"), 
     }; 
  
     let client = reqwest::Client::new(); 
     let result = client 
         .get("https://api.sightengine.com/1.0/check.json") 
         .query(&[ 
             ("url", thumbnail.as_str()), 
             ("models", "genai"), 
             ("api_user", api_user.as_str()), 
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
  
     let score = result["type"]["ai_generated"].as_f64().unwrap_or(0.0) as f32; 
     let (verdict, confidence, details) = if score > 0.65 { 
         ("ai_generated", score, format!("AI generation probability: {:.0}% — high confidence deepfake/AI artifact detected.", score * 100.0)) 
     } else if score < 0.35 { 
         ("likely_real", 1.0 - score, format!("Real image detected (AI probability only {:.0}%).", score * 100.0)) 
     } else { 
         ("unsure", 0.5f32, format!("Borderline result — AI probability {:.0}% (needs human review).", score * 100.0)) 
     }; 
  
     Ok(Response::builder() 
         .status(StatusCode::OK) 
         .header("Content-Type", "application/json") 
         .header("Access-Control-Allow-Origin", "https://dawg.city") 
         .body(Body::Text(json!({ 
             "verdict": verdict, 
             "confidence": confidence, 
             "details": details, 
             "thumbnail": thumbnail 
         }).to_string()))?) 
 } 
  
 fn error_response(msg: &str) -> Result<Response<Body>, Error> { 
     Ok(Response::builder() 
         .status(StatusCode::BAD_REQUEST) 
         .header("Content-Type", "application/json") 
         .header("Access-Control-Allow-Origin", "https://dawg.city") 
         .body(Body::Text(json!({ "error": msg }).to_string()))?) 
 }
