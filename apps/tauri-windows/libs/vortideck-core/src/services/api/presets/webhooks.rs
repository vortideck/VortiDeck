use serde_json::{json, Value};
use std::time::Duration;

use crate::services::api::error::ApiResult;
use crate::services::api::request_builder::post;
use crate::services::api::response::ApiResponse;

/// Send webhook with automatic retry and exponential backoff
pub async fn send_webhook(url: &str, payload: Value, retry_count: u32) -> ApiResult<ApiResponse> {
    let mut last_error = None;

    for retry in 0..=retry_count {
        match post(url)
            .json(payload.clone())
            .timeout(Duration::from_secs(5))
            .send()
            .await
        {
            Ok(response) => return Ok(response),
            Err(e) => {
                last_error = Some(e);
                if retry < retry_count {
                    // Exponential backoff: 100ms, 200ms, 400ms, etc.
                    let delay = Duration::from_millis(100 * (2_u64.pow(retry)));
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }

    Err(last_error.unwrap())
}

/// Send webhook with event format (event type + timestamp + data)
pub async fn send_event(url: &str, event_type: &str, data: Value) -> ApiResult<ApiResponse> {
    let payload = json!({
        "event": event_type,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "data": data
    });

    send_webhook(url, payload, 2).await
}

/// Fire-and-forget webhook (doesn't wait for response)
pub fn send_webhook_async(url: String, payload: Value) {
    tokio::spawn(async move {
        let _ = post(&url)
            .json(payload)
            .timeout(Duration::from_secs(5))
            .send()
            .await;
    });
}
