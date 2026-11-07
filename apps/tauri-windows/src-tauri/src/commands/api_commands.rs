use serde_json::Value;
use std::collections::HashMap;
use vortideck_core::services::api;

/// Generic HTTP request
#[tauri::command]
pub async fn http_request(
    method: String,
    url: String,
    headers: Option<HashMap<String, String>>,
    body: Option<Value>,
    timeout_ms: Option<u64>,
) -> Result<Value, String> {
    use reqwest::Method;

    let method = match method.to_uppercase().as_str() {
        "GET" => Method::GET,
        "POST" => Method::POST,
        "PUT" => Method::PUT,
        "DELETE" => Method::DELETE,
        "PATCH" => Method::PATCH,
        _ => return Err("Invalid HTTP method".to_string()),
    };

    let mut builder = api::RequestBuilder::new(method, url);

    if let Some(headers) = headers {
        builder = builder.headers(headers);
    }

    if let Some(body) = body {
        builder = builder.json(body);
    }

    if let Some(timeout_ms) = timeout_ms {
        builder = builder.timeout_ms(timeout_ms);
    }

    let response = builder.send().await.map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "status": response.status,
        "headers": response.headers,
        "body": response.body,
        "elapsed_ms": response.elapsed_ms
    }))
}

/// Send webhook with retry
#[tauri::command]
pub async fn send_webhook(
    url: String,
    payload: Value,
    retry_count: Option<u32>,
) -> Result<String, String> {
    let retry_count = retry_count.unwrap_or(2);

    let response = api::send_webhook(&url, payload, retry_count)
        .await
        .map_err(|e| e.to_string())?;

    Ok(format!(
        "Webhook delivered in {}ms (status: {})",
        response.elapsed_ms, response.status
    ))
}

/// Send webhook event with standard format
#[tauri::command]
pub async fn send_webhook_event(
    url: String,
    event_type: String,
    data: Value,
) -> Result<String, String> {
    let response = api::send_event(&url, &event_type, data)
        .await
        .map_err(|e| e.to_string())?;

    Ok(format!(
        "Event '{}' sent in {}ms (status: {})",
        event_type, response.elapsed_ms, response.status
    ))
}
