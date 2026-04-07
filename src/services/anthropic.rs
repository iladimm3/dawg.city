use std::time::Duration;

use crate::errors::AppError;

const MAX_RETRIES: u32 = 3;
const BASE_DELAY_MS: u64 = 750;

/// Errors that should not trigger a retry
fn is_permanent_error(e: &AppError) -> bool {
    match e {
        AppError::InternalError(msg) => {
            msg.contains("authentication") || msg.contains("invalid") || msg.contains("JSON")
        }
        _ => true,
    }
}

/// Call the Anthropic Messages API and return the raw text content of the first message block.
///
/// Handles:
/// - 30-second request timeout per attempt
/// - Exponential backoff retry (up to 3 attempts) for transient errors (rate limit, overload)
/// - Non-2xx HTTP status codes from Anthropic with actionable error messages
/// - Anthropic API-level error objects embedded in the JSON body
/// - Missing or malformed `content[0].text` field
pub async fn call(
    api_key: &str,
    model: &str,
    prompt: &str,
    max_tokens: u32,
) -> Result<String, AppError> {
    let mut last_err = AppError::InternalError("Unknown Anthropic error".into());

    for attempt in 0..=MAX_RETRIES {
        if attempt > 0 {
            let delay = BASE_DELAY_MS * (2u64.pow(attempt - 1));
            tracing::warn!(attempt, delay_ms = delay, "Retrying Anthropic call after error");
            tokio::time::sleep(Duration::from_millis(delay)).await;
        }

        match call_once(api_key, model, prompt, max_tokens).await {
            Ok(content) => return Ok(content),
            Err(e) => {
                if is_permanent_error(&e) {
                    return Err(e);
                }
                tracing::warn!(attempt, "Transient Anthropic error, will retry: {:?}", e);
                last_err = e;
            }
        }
    }

    Err(last_err)
}

async fn call_once(
    api_key: &str,
    model: &str,
    prompt: &str,
    max_tokens: u32,
) -> Result<String, AppError> {
    let client = reqwest::ClientBuilder::new()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| AppError::InternalError(format!("HTTP client build failed: {}", e)))?;

    let http_response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "model": model,
            "max_tokens": max_tokens,
            "messages": [{"role": "user", "content": prompt}]
        }))
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                AppError::InternalError("AI request timed out after 30 seconds".into())
            } else {
                AppError::InternalError(format!("AI request failed: {}", e))
            }
        })?;

    let status = http_response.status();

    let body: serde_json::Value = http_response.json().await.map_err(|e| {
        AppError::InternalError(format!("AI response was not valid JSON: {}", e))
    })?;

    if !status.is_success() {
        let error_type = body["error"]["type"].as_str().unwrap_or("unknown_error");
        let error_msg = body["error"]["message"]
            .as_str()
            .unwrap_or("No error details provided");

        tracing::error!(
            status = status.as_u16(),
            error_type,
            error_msg,
            "Anthropic API returned non-2xx response"
        );

        return Err(match error_type {
            "authentication_error" => AppError::InternalError(
                "Anthropic authentication failed — check ANTHROPIC_API_KEY".into(),
            ),
            "rate_limit_error" => AppError::InternalError(
                "Anthropic rate limit reached — please try again shortly".into(),
            ),
            "overloaded_error" => AppError::InternalError(
                "Anthropic is temporarily overloaded — please try again".into(),
            ),
            _ => AppError::InternalError(format!(
                "Anthropic API error ({}): {}",
                error_type, error_msg
            )),
        });
    }

    if let Some(err) = body.get("error") {
        let error_type = err["type"].as_str().unwrap_or("unknown_error");
        let error_msg = err["message"].as_str().unwrap_or("No error details provided");
        tracing::error!(error_type, error_msg, "Anthropic returned error in 200 body");
        return Err(AppError::InternalError(format!(
            "Anthropic API error ({}): {}",
            error_type, error_msg
        )));
    }

    let content = body["content"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|block| block["text"].as_str())
        .ok_or_else(|| {
            tracing::error!(body = %body, "Anthropic response missing content[0].text");
            AppError::InternalError(
                "Anthropic response did not contain expected text content".into(),
            )
        })?;

    Ok(content.to_string())
}

///
/// Handles:
/// - 30-second request timeout (AI responses can be slow — anything beyond is a hung request)
/// - Non-2xx HTTP status codes from Anthropic (rate limit, auth failure, overload)
/// - Anthropic API-level error objects embedded in the JSON body
/// - Missing or malformed `content[0].text` field
pub async fn call(
    api_key: &str,
    model: &str,
    prompt: &str,
    max_tokens: u32,
) -> Result<String, AppError> {
    let client = reqwest::ClientBuilder::new()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| AppError::InternalError(format!("HTTP client build failed: {}", e)))?;

    let http_response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "model": model,
            "max_tokens": max_tokens,
            "messages": [{"role": "user", "content": prompt}]
        }))
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                AppError::InternalError("AI request timed out after 30 seconds".into())
            } else {
                AppError::InternalError(format!("AI request failed: {}", e))
            }
        })?;

    let status = http_response.status();

    let body: serde_json::Value = http_response.json().await.map_err(|e| {
        AppError::InternalError(format!("AI response was not valid JSON: {}", e))
    })?;

    // Handle non-2xx responses — Anthropic includes structured error details in the body
    if !status.is_success() {
        let error_type = body["error"]["type"].as_str().unwrap_or("unknown_error");
        let error_msg = body["error"]["message"]
            .as_str()
            .unwrap_or("No error details provided");

        tracing::error!(
            status = status.as_u16(),
            error_type,
            error_msg,
            "Anthropic API returned non-2xx response"
        );

        return Err(match error_type {
            "authentication_error" => AppError::InternalError(
                "Anthropic authentication failed — check ANTHROPIC_API_KEY".into(),
            ),
            "rate_limit_error" => AppError::InternalError(
                "Anthropic rate limit reached — please try again shortly".into(),
            ),
            "overloaded_error" => AppError::InternalError(
                "Anthropic is temporarily overloaded — please try again".into(),
            ),
            _ => AppError::InternalError(format!(
                "Anthropic API error ({}): {}",
                error_type, error_msg
            )),
        });
    }

    // Guard against error objects embedded inside an otherwise 200 response
    if let Some(err) = body.get("error") {
        let error_type = err["type"].as_str().unwrap_or("unknown_error");
        let error_msg = err["message"].as_str().unwrap_or("No error details provided");
        tracing::error!(error_type, error_msg, "Anthropic returned error in 200 body");
        return Err(AppError::InternalError(format!(
            "Anthropic API error ({}): {}",
            error_type, error_msg
        )));
    }

    // Extract text content from the first content block
    let content = body["content"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|block| block["text"].as_str())
        .ok_or_else(|| {
            tracing::error!(body = %body, "Anthropic response missing content[0].text");
            AppError::InternalError(
                "Anthropic response did not contain expected text content".into(),
            )
        })?;

    Ok(content.to_string())
}
