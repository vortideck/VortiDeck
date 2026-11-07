//! WebSocket handlers exposed by the unified local HTTP server.
use super::*;

async fn queue_pending_overlay_commands(
    app: &AppHandle,
    tx: &tokio::sync::mpsc::UnboundedSender<tokio_tungstenite::tungstenite::Message>,
) {
    let Some(state) = app.try_state::<FlowExecutorState>() else { return; };
    let executor = state.executor.lock().await.clone();
    let Some(executor) = executor else { return; };
    match executor.pending_overlay_commands().await {
        Ok(commands) => for command in commands {
            let message = match crate::overlay::commands::delivery_message(app, &command, true).await {
                Ok(message) => message,
                Err(error) => { crate::overlay::commands::reject_delivery(app, &command, error).await; continue; },
            };
            if tx.send(tokio_tungstenite::tungstenite::Message::Text(message.to_string().into())).is_err() { break; }
        },
        Err(error) => log::error!("Failed to reconcile pending overlay commands: {error}"),
    }
}

pub(super) async fn handle_websocket_connection(
    websocket: warp::ws::WebSocket,
    peer_ip: String,
    app_handle: AppHandle,
    server_ip: std::net::IpAddr,
) {
    // Call the existing handle_connection_internal with pre-upgraded WebSocket
    handle_connection_internal(None, Some(websocket), app_handle, server_ip, Some(peer_ip)).await;
}

/// Handle banner-specific WebSocket connections
pub(super) async fn handle_banner_websocket(websocket: warp::ws::WebSocket, app_handle: AppHandle) {
    use futures_util::{SinkExt, StreamExt};

    println!("📋 Banner client connected");

    let (mut ws_sender, mut ws_receiver) = websocket.split();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    // Get client registry and add this banner client - STORE THE CLIENT ID
    let client_id = if let Some(client_registry) =
        app_handle.try_state::<Arc<crate::client_registry::ClientRegistry>>()
    {
        Some(client_registry.add_banner_client(tx.clone()).await)
    } else {
        None
    };

    // Handle outgoing messages to this client
    let sender_task = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            if ws_sender
                .send(warp::ws::Message::text(message.to_string()))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    // Handle incoming messages from this client (currently just log them)
    let receiver_task = tokio::spawn(async move {
        while let Some(result) = ws_receiver.next().await {
            match result {
                Ok(msg) => {
                    if msg.is_text() {
                        println!("📋 Banner message: {}", msg.to_str().unwrap_or("invalid"));
                    }
                }
                Err(_) => break,
            }
        }
    });

    // Wait for either task to complete (client disconnect)
    tokio::select! {
        _ = sender_task => {},
        _ = receiver_task => {},
    }

    println!("📋 Banner client disconnected");

    // Remove the specific client by ID instead of just calling cleanup
    if let (Some(client_id), Some(client_registry)) = (
        client_id,
        app_handle.try_state::<Arc<crate::client_registry::ClientRegistry>>(),
    ) {
        client_registry.remove_banner_client(&client_id).await;
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OverlayWidgetActionRequest {
    request_id: String,
    package_id: String,
    element_instance_id: String,
    overlay_instance_id: String,
    action: String,
    #[serde(default)]
    payload: Value,
}

#[cfg(test)]
mod overlay_widget_action_contract_tests {
    use super::*;

    #[test]
    fn parses_correlated_widget_action_wire_contract() {
        let request: OverlayWidgetActionRequest = serde_json::from_value(json!({
            "type": "WIDGET_ACTION_REQUEST",
            "requestId": "request-1",
            "packageId": "alerts",
            "elementInstanceId": "element-1",
            "overlayInstanceId": "browser-1",
            "action": "triggerFlow:flow-1",
            "payload": { "viewer": "alice" }
        }))
        .expect("valid widget action request");
        assert_eq!(request.request_id, "request-1");
        assert_eq!(request.action, "triggerFlow:flow-1");
        assert_eq!(request.payload["viewer"], "alice");
    }
}

async fn dispatch_overlay_widget_action(
    app_handle: &AppHandle,
    request: &OverlayWidgetActionRequest,
) -> Result<Value, String> {
    if request.request_id.is_empty()
        || request.package_id.is_empty()
        || request.element_instance_id.is_empty()
        || request.overlay_instance_id.is_empty()
        || request.action.is_empty()
    {
        return Err("Malformed widget action identity".to_string());
    }

    // The authenticated browser source still cannot invent a package or action:
    // reconcile the request against the current typed overlay element.
    let overlay_state = app_handle
        .try_state::<Arc<RwLock<OverlayState>>>()
        .ok_or("Overlay state is unavailable")?;
    overlay_state.read().await.ensure_widget_action_authority(
        &request.element_instance_id,
        &request.package_id,
        &request.action,
    )?;

    // Stored DB consent is authoritative. The element's copied capability data
    // narrows a package grant but can never expand it.
    let pool = app_handle
        .try_state::<DbPool>()
        .ok_or("Database is unavailable")?;
    let database = vortideck_core::db::database::Database::new(pool.inner());
    let mut grant = database
        .get_package_permission_grant(&request.package_id)
        .await?;
    if grant.is_none() {
        let alias = if let Some(id) = request.package_id.strip_prefix("plugin.") {
            id.to_string()
        } else {
            format!("plugin.{}", request.package_id)
        };
        grant = database.get_package_permission_grant(&alias).await?;
    }
    let grant = grant.ok_or("Widget package is not granted")?;
    let permissions: Vec<vortideck_core::plugins::bundle::PluginPermission> =
        serde_json::from_str(&grant.permissions_json)
            .map_err(|error| format!("Stored widget grant is invalid: {}", error))?;
    let required =
        vortideck_core::plugins::bundle::PluginPermission::TriggerAction(request.action.clone());
    if !permissions.contains(&required) {
        return Err("Stored widget grant does not allow this action".to_string());
    }

    if let Some(flow_id) = request
        .action
        .strip_prefix("triggerFlow:")
        .filter(|id| !id.is_empty())
    {
        let executor_state = app_handle
            .try_state::<FlowExecutorState>()
            .ok_or("Flow executor state is unavailable")?;
        let executor = executor_state
            .executor
            .lock()
            .await
            .clone()
            .ok_or("Flow executor is not initialized")?;
        let flow = executor.load_flow(flow_id).await?;
        let node_id = if flow.start_node_id.is_empty() {
            flow.nodes
                .first()
                .map(|node| node.id.clone())
                .ok_or("Target flow has no nodes")?
        } else {
            flow.start_node_id.clone()
        };
        // DB and flow reads await external work. Recheck the live instance
        // immediately before admitting its invocation.
        overlay_state.read().await.ensure_widget_action_authority(
            &request.element_instance_id,
            &request.package_id,
            &request.action,
        )?;
        let run_id = executor
            .trigger_flow_from_event(vortideck_core::services::flow::executor::TriggerEvent {
                source: vortideck_core::services::flow::executor::TriggerSource::Manual,
                event_id: Some(format!(
                    "widget-action:{}:{}",
                    request.overlay_instance_id, request.request_id
                )),
                correlation_id: Some(request.request_id.clone()),
                flow_id: flow_id.to_string(),
                node_id,
                data: json!({
                    "widgetAction": {
                        "requestId": request.request_id,
                        "packageId": request.package_id,
                        "elementInstanceId": request.element_instance_id,
                        "action": request.action,
                        "payload": request.payload,
                    }
                }),
                timestamp: chrono::Utc::now(),
                dedup_key: Some(format!(
                    "widget-action:{}:{}",
                    request.overlay_instance_id, request.request_id
                )),
            })
            .await?;
        return Ok(json!({ "runId": run_id, "payloadAccepted": !request.payload.is_null() }));
    }

    Err(format!(
        "Widget action '{}' has no supported host dispatcher",
        request.action
    ))
}

/// Handle overlay-specific WebSocket connections
pub(super) async fn handle_overlay_websocket(
    websocket: warp::ws::WebSocket,
    app_handle: AppHandle,
    authorized: bool,
) {
    use futures_util::{SinkExt, StreamExt};

    if !authorized {
        log::warn!("Rejected unauthorised overlay WebSocket connection");
        return;
    }

    println!("🎨 Overlay client connected");
    if let Some(overlay) = app_handle
        .try_state::<Arc<vortideck_core::integrations::builtin::overlay::OverlayIntegration>>()
    {
        overlay.host_connected();
    }

    let (mut ws_sender, mut ws_receiver) = websocket.split();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    // Get client registry and add this overlay client - STORE THE CLIENT ID
    let client_id = if let Some(client_registry) =
        app_handle.try_state::<Arc<client_registry::ClientRegistry>>()
    {
        Some(client_registry.add_overlay_client(tx.clone()).await)
    } else {
        None
    };

    // Reconcile structural overlay state after every browser-source reconnect.
    // Workflow commands remain live-only until the durable command/ACK protocol
    // is introduced, but this ensures elements, layout, theme, and visibility
    // are immediately refreshed after a missed OVERLAY_UPDATED broadcast.
    if let Some(overlay_state) = app_handle.try_state::<Arc<RwLock<OverlayState>>>() {
        let last_updated = overlay_state.read().await.last_updated;
        let snapshot_message = serde_json::json!({
            "type": "OVERLAY_UPDATED",
            "timestamp": last_updated,
            "reason": "reconnect-reconciliation",
        });
        if let Err(error) = tx.send(tokio_tungstenite::tungstenite::Message::Text(
            snapshot_message.to_string().into(),
        )) {
            log::debug!(
                "Unable to queue overlay reconciliation for new client: {}",
                error
            );
        }
    }

    // Replay only persisted, still-valid ACK-required commands. Commands are
    // idempotent by command ID on the host side; this is reconciliation, not a
    // second workflow execution.
    queue_pending_overlay_commands(&app_handle, &tx).await;

    // Query and send existing chat history to the newly connected overlay client
    match commands::overlay_authoring::observed_values_message(&app_handle).await {
        Ok(snapshot) => { let _ = tx.send(tokio_tungstenite::tungstenite::Message::Text(snapshot.to_string().into())); },
        Err(error) => log::warn!("Observed stream values unavailable on overlay reconnect: {error}"),
    }

    // Query and send existing chat history to the newly connected overlay client
    if let Some(state_ref) = app_handle.try_state::<StreamingState>() {
        let state = state_ref.inner().clone();
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            if let Ok(active_sessions) = state.session_manager.get_active_sessions().await {
                let mut account_ids = Vec::new();
                for session in active_sessions {
                    for platform_stream in &session.platforms {
                        if let Ok(account_id) = platform_stream.account_id.parse::<u64>() {
                            account_ids.push(account_id);
                        }
                    }
                }

                if !account_ids.is_empty() {
                    if let Ok(messages) = state
                        .message_store
                        .get_messages_for_accounts(account_ids, Some(20))
                        .await
                    {
                        for msg in messages {
                            let payload = vortideck_core::events::EventPayload::Stream(
                                vortideck_core::events::StreamEventPayload::ChatMessage {
                                    metadata: vortideck_core::events::StreamEventMetadata {
                                        event_id: Some(msg.id.clone()),
                                        occurred_at: chrono::DateTime::parse_from_rfc3339(
                                            &msg.timestamp,
                                        )
                                        .ok()
                                        .map(|value| value.with_timezone(&chrono::Utc)),
                                        stream_id: None,
                                        account_id: msg.account_id.unwrap_or(0),
                                        platform: msg.platform.to_string(),
                                    },
                                    user_name: msg.user_name,
                                    user_display_name: msg.user_display_name,
                                    message: msg.message,
                                    is_moderator: msg.is_moderator,
                                    is_subscriber: msg.is_subscriber,
                                    platform_color: if msg.platform_color.is_empty() {
                                        None
                                    } else {
                                        Some(msg.platform_color.clone())
                                    },
                                },
                            );

                            if let Ok(msg_str) = serde_json::to_string(&payload) {
                                let _ = tx_clone.send(
                                    tokio_tungstenite::tungstenite::Message::Text(msg_str.into()),
                                );
                            }
                        }
                    }
                }
            }
        });
    }

    // Handle outgoing messages to this client
    let sender_task = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            if ws_sender
                .send(warp::ws::Message::text(message.to_string()))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    // Only an authenticated overlay source reaches this task. It may report a
    // receipt for a command that the host has forwarded to eligible widgets.
    let receiver_app_handle = app_handle.clone();
    let receiver_tx = tx.clone();
    let receiver_task = tokio::spawn(async move {
        let mut seen_widget_action_requests = std::collections::HashSet::new();
        let mut last_overlay_ready: Option<std::time::Instant> = None;
        let mut widget_action_rate: HashMap<
            String,
            std::collections::VecDeque<std::time::Instant>,
        > = HashMap::new();
        while let Some(result) = ws_receiver.next().await {
            match result {
                Ok(msg) => {
                    if msg.is_text() {
                        let raw = msg.to_str().unwrap_or("invalid");
                        if raw.len() > 65_536 {
                            log::warn!("Rejected oversized overlay client message");
                            continue;
                        }
                        let parsed = serde_json::from_str::<serde_json::Value>(raw).ok();
                        if parsed.as_ref().is_some_and(|value|
                            value.get("type").and_then(Value::as_str) == Some("OVERLAY_READY") && value.get("version").and_then(Value::as_u64) == Some(1)) {
                            let now = std::time::Instant::now();
                            if last_overlay_ready.is_none_or(|last| now.duration_since(last).as_millis() >= 250) {
                                last_overlay_ready = Some(now);
                                queue_pending_overlay_commands(&receiver_app_handle, &receiver_tx).await;
                            }
                            continue;
                        }
                        let acknowledgement = parsed
                            .clone()
                            .filter(|value| {
                                value.get("type").and_then(serde_json::Value::as_str)
                                    == Some("OVERLAY_COMMAND_ACK")
                            })
                            .and_then(|value| {
                                serde_json::from_value::<
                                    vortideck_core::events::OverlayCommandAcknowledgement,
                                >(value)
                                .ok()
                            });
                        if let Some(acknowledgement) = acknowledgement {
                            if let Some(overlay) = receiver_app_handle.try_state::<Arc<
                                vortideck_core::integrations::builtin::overlay::OverlayIntegration,
                            >>(
                            ) {
                                let _ = overlay.acknowledge(acknowledgement.clone()).await;
                            }
                            if let Some(executor_state) =
                                receiver_app_handle.try_state::<FlowExecutorState>()
                            {
                                if let Some(executor) = executor_state.executor.lock().await.clone()
                                {
                                    if !executor.acknowledge_overlay_command(acknowledgement).await
                                    {
                                        log::debug!(
                                            "Ignored stale or unexpected overlay command acknowledgement"
                                        );
                                    }
                                }
                            }
                        } else if let Some(request) = parsed
                            .filter(|value| {
                                value.get("type").and_then(Value::as_str)
                                    == Some("WIDGET_ACTION_REQUEST")
                            })
                            .and_then(|value| {
                                serde_json::from_value::<OverlayWidgetActionRequest>(value).ok()
                            })
                        {
                            let rate_key =
                                format!("{}:{}", request.package_id, request.element_instance_id);
                            let now = std::time::Instant::now();
                            let recent = widget_action_rate.entry(rate_key).or_default();
                            while recent.front().is_some_and(|timestamp| {
                                now.duration_since(*timestamp).as_secs() >= 10
                            }) {
                                recent.pop_front();
                            }
                            let outcome = if !seen_widget_action_requests
                                .insert(request.request_id.clone())
                            {
                                Err("Duplicate widget action request ID".to_string())
                            } else if recent.len() >= 10 {
                                Err("Widget action rate limit exceeded".to_string())
                            } else {
                                recent.push_back(now);
                                dispatch_overlay_widget_action(&receiver_app_handle, &request).await
                            };
                            if seen_widget_action_requests.len() > 1_000 {
                                seen_widget_action_requests.clear();
                            }
                            let (ok, result, error) = match outcome {
                                Ok(result) => (true, result, Value::Null),
                                Err(error) => (false, Value::Null, Value::String(error)),
                            };
                            let response = json!({
                                "type": "WIDGET_ACTION_RESULT",
                                "requestId": request.request_id,
                                "packageId": request.package_id,
                                "elementInstanceId": request.element_instance_id,
                                "overlayInstanceId": request.overlay_instance_id,
                                "ok": ok,
                                "result": result,
                                "error": error,
                            });
                            let _ =
                                receiver_tx.send(tokio_tungstenite::tungstenite::Message::Text(
                                    response.to_string().into(),
                                ));
                        } else {
                            log::debug!("Ignored unsupported overlay client message");
                        }
                    }
                }
                Err(_) => break,
            }
        }
    });

    // Wait for either task to complete (client disconnect)
    tokio::select! {
        _ = sender_task => {},
        _ = receiver_task => {},
    }

    println!("🎨 Overlay client disconnected");
    if let Some(overlay) = app_handle
        .try_state::<Arc<vortideck_core::integrations::builtin::overlay::OverlayIntegration>>()
    {
        overlay.host_disconnected();
    }

    // Remove the specific client by ID instead of just calling cleanup
    if let (Some(client_id), Some(client_registry)) = (
        client_id,
        app_handle.try_state::<Arc<crate::client_registry::ClientRegistry>>(),
    ) {
        client_registry.remove_overlay_client(&client_id).await;
    }
}
