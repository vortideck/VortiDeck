use chrono::Utc;
use serde_json::Value;
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use vortideck_core::events::EventEnvelope;
use vortideck_core::services::flow::commands::FlowExecutorState;
use vortideck_core::services::flow::executor::FlowRunResponseSignal;
use vortideck_core::services::webhook::{
    WebhookIngressAcceptance, WebhookService, WebhookTriggerEvent,
};

/// Handle incoming webhook trigger events from the webhook server
///
/// This handler:
/// 1. Looks up webhook configuration (flow_id, response_mode)
/// 2. Triggers flow execution
/// 3. Confirms durable canonical-envelope acceptance
/// 4. Uses the run-scoped response channel for synchronous response modes
pub async fn handle_webhook_events(
    app_handle: AppHandle,
    webhook_service: Arc<WebhookService>,
    mut webhook_rx: tokio::sync::mpsc::UnboundedReceiver<WebhookTriggerEvent>,
) {
    while let Some(event) = webhook_rx.recv().await {
        log::info!("📨 Webhook event received: {}", event.webhook_id);

        // Get the response channel (for LastNode/ResponseNode modes)
        let response_channel = event.response_channel;
        let mut acceptance_channel = Some(event.acceptance_channel);

        // Look up webhook to get flow_id and response_mode
        let webhook_config_result = webhook_service
            .get_webhook(&event.webhook_id)
            .map_err(|e| format!("Failed to get webhook: {}", e))
            .and_then(|webhook| {
                // Parse action field to get FlowId
                let flow_id = if let Some(action_str) = &webhook.action {
                    let action: Result<vortideck_core::db::models::Action, _> =
                        serde_json::from_str(action_str);
                    match action {
                        Ok(vortideck_core::db::models::Action::FlowId(flow_id)) => Ok(flow_id),
                        Ok(other_action) => {
                            Err(format!("Webhook action is not FlowId: {:?}", other_action))
                        }
                        Err(e) => Err(format!("Failed to parse webhook action: {}", e)),
                    }
                } else {
                    Err("Webhook has no action field".to_string())
                }?;

                // Return both flow_id and response_mode
                Ok((flow_id, webhook.response_mode.clone()))
            });

        let (flow_id, response_mode) = match webhook_config_result {
            Ok((id, mode)) => (id, mode),
            Err(e) => {
                log::error!("❌ Failed to get webhook config: {}", e);
                if let Some(tx) = acceptance_channel.take() {
                    let _ = tx.send(WebhookIngressAcceptance::Rejected {
                        message: "Invalid webhook configuration".to_string(),
                    });
                }
                if let Some(tx) = response_channel {
                    let _ = tx.send(serde_json::json!({
                        "status": "error",
                        "error": format!("Invalid webhook configuration: {}", e)
                    }));
                }
                continue;
            }
        };

        log::info!(
            "   Webhook {} triggers flow {} (mode: {})",
            event.webhook_id,
            flow_id,
            response_mode
        );

        // Get flow executor
        if let Some(flow_executor_state) = app_handle.try_state::<FlowExecutorState>() {
            let executor_guard = flow_executor_state.executor.lock().await;

            // Check if executor is initialized
            let flow_executor = match executor_guard.as_ref() {
                Some(exec) => exec.clone(),
                None => {
                    log::warn!("⚠️ Flow executor not initialized");
                    if let Some(tx) = acceptance_channel.take() {
                        let _ = tx.send(WebhookIngressAcceptance::Rejected {
                            message: "Workflow runtime is not initialized".to_string(),
                        });
                    }
                    if let Some(tx) = response_channel {
                        let _ = tx.send(serde_json::json!({
                            "status": "error",
                            "error": "Flow executor not initialized"
                        }));
                    }
                    continue;
                }
            };

            drop(executor_guard); // Release lock before async operations

            // Persist the canonical source event before workflow side effects.
            // Provider delivery IDs are namespaced by webhook; without one we
            // still retain an auditable event but intentionally do not invent a
            // payload-hash deduplication key (identical webhook payloads can be
            // legitimate independent deliveries).
            let received_at = Utc::now();
            let provider_event_id = event.event_id.clone();
            let canonical_event_id = provider_event_id
                .as_ref()
                .map(|delivery_id| format!("webhook:{}:{}", event.webhook_id, delivery_id))
                .unwrap_or_else(|| {
                    format!("webhook:{}:{}", event.webhook_id, uuid::Uuid::new_v4())
                });
            let idempotency_key = provider_event_id
                .as_ref()
                .map(|delivery_id| format!("webhook:{}:{}", event.webhook_id, delivery_id));
            let envelope = EventEnvelope {
                event_id: canonical_event_id.clone(),
                event_type: "webhook.received".to_string(),
                source: format!("webhook:{}", event.webhook_id),
                schema_version: 1,
                occurred_at: received_at,
                received_at,
                account_id: None,
                session_id: None,
                stream_id: None,
                correlation_id: Some(flow_id.clone()),
                causation_id: None,
                idempotency_key,
                payload: event.payload.clone(),
            };
            match flow_executor.record_event_envelope(&envelope).await {
                Ok(true) => {}
                Ok(false) => {
                    log::info!(
                        "⏭️ Duplicate webhook delivery ignored: {}",
                        envelope.event_id
                    );
                    if let Some(tx) = acceptance_channel.take() {
                        let _ = tx.send(WebhookIngressAcceptance::Duplicate {
                            event_id: envelope.event_id.clone(),
                        });
                    }
                    if let Some(tx) = response_channel {
                        let _ = tx.send(serde_json::json!({
                            "status": "duplicate",
                            "eventId": envelope.event_id,
                        }));
                    }
                    continue;
                }
                Err(error) => {
                    log::error!("❌ Failed to persist webhook envelope: {}", error);
                    if let Some(tx) = acceptance_channel.take() {
                        let _ = tx.send(WebhookIngressAcceptance::Rejected {
                            message: "Webhook event could not be persisted".to_string(),
                        });
                    }
                    if let Some(tx) = response_channel {
                        let _ = tx.send(serde_json::json!({
                            "status": "error",
                            "error": "Webhook event could not be persisted",
                        }));
                    }
                    continue;
                }
            }

            if let Some(tx) = acceptance_channel.take() {
                let _ = tx.send(WebhookIngressAcceptance::Accepted {
                    event_id: envelope.event_id.clone(),
                });
            }

            // Trigger the flow
            let capture_run_response = response_channel.is_some();
            match flow_executor
                .trigger_flow_from_webhook(
                    &flow_id,
                    event.payload.clone(),
                    Some(canonical_event_id),
                    capture_run_response,
                )
                .await
            {
                Ok(execution_id) => {
                    log::info!(
                        "✅ Flow {} triggered successfully: {}",
                        flow_id,
                        execution_id
                    );

                    // If response channel present, wait for flow completion
                    if let Some(tx) = response_channel {
                        match flow_executor
                            .take_run_response_receiver(&execution_id)
                            .await
                        {
                            Some(receiver) => {
                                // A response-mode webhook must not prevent the
                                // ingress loop from accepting subsequent work.
                                tokio::spawn(async move {
                                    handle_flow_response(
                                        receiver,
                                        flow_id,
                                        execution_id,
                                        response_mode,
                                        tx,
                                    )
                                    .await;
                                });
                            }
                            None => {
                                let _ = tx.send(serde_json::json!({
                                    "status": "error",
                                    "error": "Workflow response channel unavailable",
                                }));
                            }
                        }
                    }
                }
                Err(e) => {
                    log::error!("❌ Failed to trigger flow: {}", e);

                    // If response channel present, send error
                    if let Some(tx) = response_channel {
                        let _ = tx.send(serde_json::json!({
                            "status": "error",
                            "error": e
                        }));
                    }
                }
            }
        } else {
            log::warn!("⚠️ Flow executor not available");

            if let Some(tx) = acceptance_channel.take() {
                let _ = tx.send(WebhookIngressAcceptance::Rejected {
                    message: "Workflow runtime is not initialized".to_string(),
                });
            }

            // Send error through response channel if present
            if let Some(tx) = response_channel {
                let _ = tx.send(serde_json::json!({
                    "status": "error",
                    "error": "Flow executor not initialized"
                }));
            }
        }
    }
}

/// Handle flow response based on webhook response_mode
///
/// Consumes the run-scoped response stream created before execution starts.
async fn handle_flow_response(
    rx: tokio::sync::mpsc::UnboundedReceiver<FlowRunResponseSignal>,
    flow_id: String,
    execution_id: String,
    response_mode: String,
    tx: tokio::sync::oneshot::Sender<Value>,
) {
    handle_flow_response_with_timeout(
        rx,
        flow_id,
        execution_id,
        response_mode,
        tx,
        std::time::Duration::from_secs(30),
    )
    .await;
}

async fn handle_flow_response_with_timeout(
    mut rx: tokio::sync::mpsc::UnboundedReceiver<FlowRunResponseSignal>,
    flow_id: String,
    execution_id: String,
    response_mode: String,
    tx: tokio::sync::oneshot::Sender<Value>,
    response_timeout: std::time::Duration,
) {
    log::info!(
        "📡 Waiting for flow {} run {} response (mode: {})",
        flow_id,
        execution_id,
        response_mode
    );

    // Wait for flow completion (with timeout)
    match tokio::time::timeout(response_timeout, async {
        loop {
            let Some(signal) = rx.recv().await else {
                return Err("Flow execution interrupted");
            };
            match signal {
                FlowRunResponseSignal::ResponseNode(response_data)
                    if response_mode == "responseNode" =>
                {
                    log::info!(
                        "RespondToWebhookNode executed in flow {} run {}",
                        flow_id,
                        execution_id
                    );
                    return Ok(response_data);
                }
                FlowRunResponseSignal::Completed(result) if response_mode == "lastNode" => {
                    return Ok(result);
                }
                FlowRunResponseSignal::Completed(_) if response_mode == "responseNode" => {
                    return Err("Flow completed without a Respond to Webhook node");
                }
                FlowRunResponseSignal::Failed(error) => {
                    return Ok(serde_json::json!({
                        "status": "error",
                        "error": error
                    }));
                }
                _ => {}
            }
        }
    })
    .await
    {
        Ok(Ok(result)) => {
            log::info!("📤 Sending flow result through response channel");
            if let Err(_) = tx.send(result) {
                log::warn!("⚠️ Failed to send webhook response (client disconnected?)");
            }
        }
        Ok(Err(e)) => {
            log::error!("❌ Flow response unavailable: {}", e);
            let _ = tx.send(serde_json::json!({
                "status": "error",
                "error": e
            }));
        }
        Err(_) => {
            // Timeout - different messages based on mode
            let error_message = if response_mode == "responseNode" {
                "No Respond to Webhook node executed within 30s timeout"
            } else {
                "Flow execution timeout (30s)"
            };

            log::error!("❌ {}", error_message);

            let _ = tx.send(serde_json::json!({
                "status": "error",
                "error": error_message
            }));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{handle_flow_response, handle_flow_response_with_timeout};
    use serde_json::json;
    use vortideck_core::services::flow::executor::FlowRunResponseSignal;

    #[tokio::test]
    async fn run_scoped_queue_preserves_fast_response_node_signal() {
        let (signal_tx, signal_rx) = tokio::sync::mpsc::unbounded_channel();
        signal_tx
            .send(FlowRunResponseSignal::ResponseNode(json!({"answer": 42})))
            .unwrap();
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();

        handle_flow_response(
            signal_rx,
            "flow-1".to_string(),
            "run-1".to_string(),
            "responseNode".to_string(),
            response_tx,
        )
        .await;

        assert_eq!(response_rx.await.unwrap(), json!({"answer": 42}));
    }

    #[tokio::test]
    async fn response_node_mode_reports_terminal_run_without_response_node() {
        let (signal_tx, signal_rx) = tokio::sync::mpsc::unbounded_channel();
        signal_tx
            .send(FlowRunResponseSignal::Completed(json!({"ignored": true})))
            .unwrap();
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();

        handle_flow_response(
            signal_rx,
            "flow-1".to_string(),
            "run-1".to_string(),
            "responseNode".to_string(),
            response_tx,
        )
        .await;

        let response = response_rx.await.unwrap();
        assert_eq!(
            response["error"],
            "Flow completed without a Respond to Webhook node"
        );
    }

    #[tokio::test]
    async fn last_node_mode_returns_terminal_result() {
        let (signal_tx, signal_rx) = tokio::sync::mpsc::unbounded_channel();
        signal_tx
            .send(FlowRunResponseSignal::Completed(json!({"result": "ok"})))
            .unwrap();
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();

        handle_flow_response(
            signal_rx,
            "flow-1".to_string(),
            "run-1".to_string(),
            "lastNode".to_string(),
            response_tx,
        )
        .await;

        assert_eq!(response_rx.await.unwrap(), json!({"result": "ok"}));
    }

    #[tokio::test]
    async fn synchronous_response_wait_is_bounded() {
        let (_signal_tx, signal_rx) = tokio::sync::mpsc::unbounded_channel();
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();

        handle_flow_response_with_timeout(
            signal_rx,
            "flow-1".to_string(),
            "run-1".to_string(),
            "responseNode".to_string(),
            response_tx,
            std::time::Duration::from_millis(1),
        )
        .await;

        assert_eq!(
            response_rx.await.unwrap()["error"],
            "No Respond to Webhook node executed within 30s timeout"
        );
    }
}
