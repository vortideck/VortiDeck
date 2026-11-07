use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use vortideck_core::core::StateManager;
use vortideck_core::db::models::{NewWebhook, UpdateWebhook, Webhook};
use vortideck_core::services::webhook::{WebhookService, WebhookServiceState};

/// Webhook response with additional computed fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookResponse {
    #[serde(flatten)]
    pub webhook: Webhook,
    pub trigger_count: i64, // Today's trigger count
}

/// Never expose the provider signing secret through ordinary read commands.
/// It is write-only from the editor's perspective and is consumed only by the
/// ingress signature verifier.
fn redact_inbound_secret(mut webhook: Webhook) -> Webhook {
    webhook.inbound_secret = None;
    webhook
}

/// Get all webhooks with today's request counts
#[tauri::command]
pub async fn get_webhooks(
    webhook_state: State<'_, WebhookServiceState>,
) -> Result<Vec<WebhookResponse>, String> {
    let service = webhook_state.service.lock().await;
    let webhooks = service.list_webhooks().map_err(|e| e.to_string())?;

    // Get today's count for each webhook
    let mut webhook_responses = Vec::new();
    for webhook in webhooks {
        let today_count = service.get_today_events_count(&webhook.id).unwrap_or(0); // Default to 0 if query fails

        webhook_responses.push(WebhookResponse {
            webhook: redact_inbound_secret(webhook),
            trigger_count: today_count,
        });
    }

    Ok(webhook_responses)
}

/// Get a single webhook by ID
#[tauri::command]
pub async fn get_webhook(
    webhook_state: State<'_, WebhookServiceState>,
    webhook_id: String,
) -> Result<Webhook, String> {
    let service = webhook_state.service.lock().await;
    service
        .get_webhook(&webhook_id)
        .map(redact_inbound_secret)
        .map_err(|e| e.to_string())
}

/// Create a new webhook
#[tauri::command]
pub async fn create_webhook(
    webhook_state: State<'_, WebhookServiceState>,
    name: String,
    description: Option<String>,
    service: String,
    action: Option<String>,
    response_mode: Option<String>,
    allowed_methods: Option<String>,
) -> Result<Webhook, String> {
    let webhook_service = webhook_state.service.lock().await;
    webhook_service
        .create_webhook(
            &name,
            description.as_deref(),
            &service,
            action.as_deref(),
            response_mode.as_deref(),
            allowed_methods.as_deref(),
        )
        .map_err(|e| e.to_string())
}

/// Update an existing webhook
#[tauri::command]
pub async fn update_webhook(
    webhook_state: State<'_, WebhookServiceState>,
    webhook_id: String,
    name: Option<String>,
    description: Option<String>,
    service: Option<String>,
    inbound_secret: Option<String>,
    action: Option<String>,
    enabled: Option<bool>,
    subscription_id: Option<String>,
) -> Result<Webhook, String> {
    if enabled == Some(false) {
        webhook_state.test_sessions.revoke(&webhook_id);
    }
    let webhook_service = webhook_state.service.lock().await;
    webhook_service
        .update_webhook(
            &webhook_id,
            name.as_deref(),
            description.as_deref(),
            service.as_deref(),
            inbound_secret.as_deref(),
            action.as_deref(),
            enabled,
            subscription_id.as_deref(),
        )
        .map_err(|e| e.to_string())
}

/// Delete a webhook
#[tauri::command]
pub async fn delete_webhook(
    webhook_state: State<'_, WebhookServiceState>,
    webhook_id: String,
) -> Result<(), String> {
    webhook_state.test_sessions.revoke(&webhook_id);
    let service = webhook_state.service.lock().await;
    service
        .delete_webhook(&webhook_id)
        .map_err(|e| e.to_string())
}

/// Deactivate test mode for a webhook
#[tauri::command]
pub async fn deactivate_test_mode(
    webhook_state: State<'_, WebhookServiceState>,
    webhook_id: String,
) -> Result<Webhook, String> {
    webhook_state.test_sessions.revoke(&webhook_id);
    let service = webhook_state.service.lock().await;
    service
        .deactivate_test_mode(&webhook_id)
        .map_err(|e| e.to_string())
}

/// Set response mode for a webhook
#[tauri::command]
pub async fn set_webhook_response_mode(
    webhook_state: State<'_, WebhookServiceState>,
    webhook_id: String,
    response_mode: String,
) -> Result<Webhook, String> {
    let service = webhook_state.service.lock().await;
    service
        .set_response_mode(&webhook_id, &response_mode)
        .map_err(|e| e.to_string())
}

/// Set allowed HTTP methods for a webhook
#[tauri::command]
pub async fn set_webhook_allowed_methods(
    webhook_state: State<'_, WebhookServiceState>,
    webhook_id: String,
    methods_json: String,
) -> Result<Webhook, String> {
    let service = webhook_state.service.lock().await;
    service
        .set_allowed_methods(&webhook_id, &methods_json)
        .map_err(|e| e.to_string())
}

/// Register webhook with external service (Twitch, GitHub, etc.)
#[tauri::command]
pub async fn register_webhook_with_service(
    webhook_state: State<'_, WebhookServiceState>,
    name: String,
    service: String,
    event_type: String,
    config: Value,
) -> Result<Webhook, String> {
    let webhook_service = webhook_state.service.lock().await;
    webhook_service
        .register_with_external_service(&name, &service, &event_type, &config)
        .await
        .map_err(|e| e.to_string())
}

/// Unregister webhook from external service
#[tauri::command]
pub async fn unregister_webhook_from_service(
    webhook_state: State<'_, WebhookServiceState>,
    webhook_id: String,
) -> Result<(), String> {
    let webhook_service = webhook_state.service.lock().await;
    webhook_service
        .unregister_from_external_service(&webhook_id)
        .await
        .map_err(|e| e.to_string())
}

/// Check if external webhook still exists
#[tauri::command]
pub async fn check_external_webhook(
    webhook_state: State<'_, WebhookServiceState>,
    webhook_id: String,
) -> Result<bool, String> {
    let webhook_service = webhook_state.service.lock().await;
    webhook_service
        .check_external_webhook(&webhook_id)
        .await
        .map_err(|e| e.to_string())
}

/// Get webhook statistics
#[tauri::command]
pub async fn get_webhook_statistics(
    webhook_state: State<'_, WebhookServiceState>,
) -> Result<HashMap<String, Value>, String> {
    let service = webhook_state.service.lock().await;
    let webhooks = service.list_webhooks().map_err(|e| e.to_string())?;

    let total = webhooks.len();
    let enabled = webhooks.iter().filter(|w| w.enabled).count();
    let disabled = total - enabled;
    let with_external = webhooks
        .iter()
        .filter(|w| w.subscription_id.is_some())
        .count();

    let mut stats = HashMap::new();
    stats.insert("total".to_string(), Value::from(total));
    stats.insert("enabled".to_string(), Value::from(enabled));
    stats.insert("disabled".to_string(), Value::from(disabled));
    stats.insert(
        "with_external_registration".to_string(),
        Value::from(with_external),
    );

    Ok(stats)
}

/// Send test webhook request to the test URL
#[tauri::command]
pub async fn send_test_webhook(
    app_handle: AppHandle,
    webhook_state: State<'_, WebhookServiceState>,
    webhook_id: String,
    payload: String,
    method: Option<String>,
    content_type: Option<String>,
    headers: Option<HashMap<String, String>>,
) -> Result<Value, String> {
    // The Tauri command is the authenticated editor boundary. It creates a
    // short-lived scoped capability and presents it to the HTTP adapter; the
    // durable test flag alone never authorizes ingress.
    let capability = {
        let service = webhook_state.service.lock().await;
        service
            .activate_test_mode(&webhook_id)
            .map_err(|e| e.to_string())?;
        log::info!("🧪 Auto-activated test mode for webhook {}", webhook_id);
        webhook_state
            .test_sessions
            .issue(&webhook_id, std::time::Duration::from_secs(120))
    };

    // Step 2: Get webhook and server info
    let service = webhook_state.service.lock().await;
    let webhook = service
        .get_webhook(&webhook_id)
        .map_err(|e| e.to_string())?;

    let state_manager = app_handle.state::<Arc<StateManager>>();
    let server_info = state_manager.get_server_info();

    drop(service); // Release the lock before making HTTP request

    let (ip, port) = match server_info {
        Some(info) => (info.ip, info.port),
        None => return Err("Server info not available".to_string()),
    };

    // Build the full webhook URL using test_url
    let default_test_path = format!("/webhook-test/{}", webhook_id);
    let test_path = webhook.test_url.as_ref().unwrap_or(&default_test_path);
    let webhook_url = format!("http://{}:{}{}", ip, port, test_path);

    log::info!("🧪 Sending test webhook to: {}", webhook_url);

    // Send HTTP POST request
    let client = reqwest::Client::new();
    let start_time = std::time::Instant::now();

    let method = reqwest::Method::from_bytes(method.as_deref().unwrap_or("POST").as_bytes())
        .map_err(|_| "Invalid HTTP test method".to_string())?;
    let mut request = client
        .request(method, &webhook_url)
        .bearer_auth(&capability.token)
        .header(
            "Content-Type",
            content_type.as_deref().unwrap_or("application/json"),
        );
    for (name, value) in headers.unwrap_or_default() {
        if name.eq_ignore_ascii_case("authorization")
            || name.eq_ignore_ascii_case("host")
            || name.eq_ignore_ascii_case("content-length")
        {
            continue;
        }
        let name = reqwest::header::HeaderName::from_bytes(name.as_bytes())
            .map_err(|_| format!("Invalid test header name: {name}"))?;
        let value = reqwest::header::HeaderValue::from_str(&value)
            .map_err(|_| format!("Invalid value for test header: {name}"))?;
        request = request.header(name, value);
    }
    let result = request.body(payload.clone()).send().await;

    let duration_ms = start_time.elapsed().as_millis() as u64;

    match result {
        Ok(response) => {
            let status = response.status().as_u16();
            let response_text = response.text().await.unwrap_or_default();

            let response_json: Value = serde_json::from_str(&response_text)
                .unwrap_or_else(|_| serde_json::json!({"raw": response_text}));

            Ok(serde_json::json!({
                "success": status >= 200 && status < 300,
                "status": status,
                "response": response_json,
                "responseTime": duration_ms,
                "testModeActive": true,
                "testModeExpiresIn": capability.expires_in_seconds,
                "realEffects": true
            }))
        }
        Err(e) => Ok(serde_json::json!({
            "success": false,
            "error": e.to_string(),
            "responseTime": duration_ms
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::redact_inbound_secret;
    use vortideck_core::db::models::Webhook;

    #[test]
    fn ordinary_webhook_reads_never_return_the_signing_secret() {
        let webhook = Webhook {
            id: "webhook-1".to_string(),
            name: "GitHub".to_string(),
            description: None,
            service: "github".to_string(),
            endpoint_url: "/webhook/webhook-1".to_string(),
            inbound_secret: Some("must-not-leave-the-host".to_string()),
            action: None,
            enabled: true,
            created_at: "2026-09-05T00:00:00Z".to_string(),
            updated_at: "2026-09-05T00:00:00Z".to_string(),
            subscription_id: None,
            test_url: Some("/webhook-test/webhook-1".to_string()),
            production_url: Some("/webhook/webhook-1".to_string()),
            is_test_active: false,
            test_expires_at: None,
            response_mode: "onReceived".to_string(),
            allowed_methods: "[\"POST\"]".to_string(),
        };

        let redacted = redact_inbound_secret(webhook);

        assert!(redacted.inbound_secret.is_none());
    }
}
