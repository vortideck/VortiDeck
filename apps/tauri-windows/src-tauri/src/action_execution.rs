//! Host-side action execution and deck protocol handlers.
use super::*;
use std::collections::HashSet;

pub(super) struct WidgetExecutionContext {
    pub instance_id: String,
    pub deck_id: String,
    pub mode: String,
    pub config: Option<serde_json::Value>,
    pub requested_values: serde_json::Value,
    pub commit_requested_on_success: bool,
}

async fn publish_widget_dispatch_failure(
    app_handle: &AppHandle,
    tx: &broadcast::Sender<Message>,
    is_widget_execution: bool,
    deck_id: &str,
    instance_id: &str,
    code: &str,
    message: &str,
) {
    if !is_widget_execution {
        return;
    }
    let Ok(deck_id) = deck_id.parse::<i32>() else {
        return;
    };
    crate::command_handler::publish_widget_runtime_state(
        app_handle,
        tx,
        deck_id,
        instance_id,
        "error",
        json!({ "lastSuccess": false }),
        Some(json!({ "code": code, "message": message })),
    )
    .await;
}

pub(super) async fn execute_action_universal(
    action: Action,
    value: Option<f64>,
    button_id: Option<String>,
    db: Arc<Database>,
    tx: broadcast::Sender<Message>,
    execution_override: Option<WidgetExecutionContext>,
) {
    // Guard: Enforce per-deck button usage limits based on position for free/guest
    if execution_override.is_none() {
        if let Some(button_id_str) = &button_id {
            if let Ok(button_id_int) = button_id_str.parse::<i32>() {
                use vortideck_core::features::jwt_features::get_jwt_provider;

                if let Ok(Some(button_record)) = db.get_button_by_id(button_id_int).await {
                    let provider = get_jwt_provider();
                    let plan = provider
                        .get_user_plan()
                        .unwrap_or_else(|| "guest".to_string());
                    let effective_limit =
                        provider.get_effective_feature_limit("ui_buttons") as usize;

                    // If a widget document exists for this deck, resolve position through widget items
                    let resolved_index = match db
                        .to_core_database()
                        .get_deck_widget_document(button_record.deck_id)
                        .await
                    {
                        Ok(document) => {
                            let mut items: Vec<_> =
                                document.items.iter().filter(|it| it.item.enabled).collect();
                            items.sort_by_key(|doc| {
                                let placement = doc
                                    .layouts
                                    .iter()
                                    .find(|l| l.layout_key == "landscape")
                                    .or_else(|| {
                                        doc.layouts.iter().find(|l| l.layout_key == "portrait")
                                    })
                                    .or_else(|| {
                                        doc.layouts.iter().find(|l| l.layout_key == "default")
                                    })
                                    .or_else(|| doc.layouts.first());
                                (
                                    placement.map(|l| l.y).unwrap_or(i32::MAX),
                                    placement.map(|l| l.x).unwrap_or(i32::MAX),
                                    doc.item.id.clone(),
                                )
                            });
                            items
                                .iter()
                                .position(|it| it.item.legacy_button_id == Some(button_id_int))
                        }
                        _ => None,
                    };

                    let index = if let Some(idx) = resolved_index {
                        Some(idx)
                    } else if let Some(mut deck_with_buttons) = db
                        .get_deck_with_buttons(button_record.deck_id, false, None)
                        .await
                    {
                        deck_with_buttons.buttons.sort_by_key(|b| b.position);
                        deck_with_buttons
                            .buttons
                            .iter()
                            .position(|b| b.id == button_id_int)
                    } else {
                        None
                    };

                    if let Some(index) = index {
                        println!(
                            "\u{1F4CA} Button exec guard: plan={}, ui_buttons_limit={}, index={}, button_id={}",
                            plan, effective_limit, index, button_id_int
                        );
                        if index >= effective_limit {
                            let msg = format!(
                                "Button in slot {} is blocked by {} limit (maximum {} active buttons).",
                                index + 1, plan, effective_limit
                            );
                            println!(
                                "\u{1F512} Blocking execution of button {} (index {}) due to plan limit (allowed: {})",
                                button_id_int, index, effective_limit
                            );
                            if let Some(app_handle) = APP_HANDLE.get() {
                                use tauri::Emitter;
                                let _ = app_handle.emit(
                                    "action_execution_failed",
                                    serde_json::json!({
                                        "button_id": button_id_int,
                                        "error": msg,
                                        "is_plan_limit": true
                                    }),
                                );
                            }
                            return;
                        } else {
                            println!(
                                "\u{2705} Allowed execution for button {} (index {}) within limit {}",
                                button_id_int, index, effective_limit
                            );
                        }
                    }
                }
            }
        }
    }

    // Get button execution context if button_id is provided
    let is_widget_execution = execution_override.is_some();
    let (
        execution_mode,
        execution_config,
        trigger_id,
        trigger_deck_id,
        requested_values,
        commit_requested_on_success,
    ) = if let Some(widget) = execution_override {
        (
            widget.mode,
            widget.config,
            widget.instance_id,
            widget.deck_id,
            widget.requested_values,
            widget.commit_requested_on_success,
        )
    } else if let Some(id) = &button_id {
        if let Ok(button_id_int) = id.parse::<i32>() {
            match db.get_button_by_id(button_id_int).await {
                Ok(Some(button)) => {
                    let mode = button.get_execution_mode();
                    let config = button.get_execution_config().unwrap_or(None);
                    (
                        mode,
                        config,
                        id.clone(),
                        button.deck_id.to_string(),
                        json!({}),
                        false,
                    )
                }
                Ok(None) => {
                    println!("⚠️  Button not found with ID: {}", button_id_int);
                    (
                        "run_once".to_string(),
                        None,
                        id.clone(),
                        "unknown".to_string(),
                        json!({}),
                        false,
                    )
                }
                Err(e) => {
                    println!("❌ Error fetching button: {}", e);
                    (
                        "run_once".to_string(),
                        None,
                        id.clone(),
                        "unknown".to_string(),
                        json!({}),
                        false,
                    )
                }
            }
        } else {
            println!("⚠️  Invalid button ID format: {}", id);
            (
                "run_once".to_string(),
                None,
                id.clone(),
                "unknown".to_string(),
                json!({}),
                false,
            )
        }
    } else {
        (
            "run_once".to_string(),
            None,
            "unknown".to_string(),
            "unknown".to_string(),
            json!({}),
            false,
        )
    };

    println!(
        "🎯 Executing action with execution mode: {} for button: {:?}",
        execution_mode, button_id
    );

    // 🚀 NEW ARCHITECTURE: Spawn async execution (concurrency handled by UniversalExecutionEngine)
    tokio::spawn(async move {
        // 🚀 UNIFIED ARCHITECTURE: All actions go through UniversalExecutionEngine
        // Convert execution_mode string to enum
        use vortideck_core::services::execution::ButtonExecutionMode;
        let execution_mode_enum = execution_mode
            .parse::<ButtonExecutionMode>()
            .unwrap_or(ButtonExecutionMode::RunOnce);

        // All actions (including Macro, Flow, App) now go through UniversalExecutionEngine
        if let Some(app_handle) = APP_HANDLE.get() {
            use tauri::Manager;

            if let Some(invocation_service) = app_handle
                .try_state::<Arc<vortideck_core::services::execution::ActionInvocationService>>()
            {
                let universal_action = match invocation_service
                    .prepare_database_action(&action, value, button_id.clone(), &execution_config.as_ref()
                        .and_then(|config| config.get("inputs")).cloned().unwrap_or_else(|| json!({})))
                    .await
                {
                    Ok(ua) => ua,
                    Err(e) => {
                        println!("❌ Failed to convert action: {}", e);
                        use tauri::Emitter;
                        let _ = app_handle.emit(
                            "action_execution_failed",
                            serde_json::json!({
                                "button_id": button_id,
                                "error": format!("Failed to resolve action: {}", e),
                                "is_plan_limit": false
                            }),
                        );
                        publish_widget_dispatch_failure(
                            app_handle,
                            &tx,
                            is_widget_execution,
                            &trigger_deck_id,
                            &trigger_id,
                            "action_resolution_failed",
                            "The saved action could not be resolved.",
                        )
                        .await;
                        return;
                    }
                };

                let persist_requested_state = execution_config
                    .as_ref()
                    .and_then(|config| config.get("persistState"))
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);

                // Create execution context
                let context = vortideck_core::services::execution::ExecutionContext {
                    trigger_source: vortideck_core::services::execution::TriggerSource::Button {
                        button_id: trigger_id.clone(),
                        deck_id: trigger_deck_id.clone(),
                    },
                    user_id: None,
                    session_id: "system".to_string(),
                    variables: HashMap::new(),
                    timeout_ms: Some(30000),
                    retry_config: vortideck_core::services::execution::RetryConfig {
                        max_retries: 3,
                        retry_delay_ms: 1000,
                        exponential_backoff: true,
                    },
                    execution_mode: execution_mode_enum,
                    execution_config: execution_config,
                };

                let result = invocation_service
                    .invoke_resolved_action(&universal_action, &context)
                    .await;

                if is_widget_execution {
                    if let Ok(deck_id) = trigger_deck_id.parse::<i32>() {
                        if result.succeeded()
                            && commit_requested_on_success
                            && persist_requested_state
                        {
                            if let Err(error) = db
                                .to_core_database()
                                .save_deck_item_persistent_state(&trigger_id, &requested_values)
                                .await
                            {
                                log::warn!(
                                    "Failed to persist applied widget state for '{}': {}",
                                    trigger_id,
                                    error
                                );
                            }
                        }
                        let error = result.error.as_ref().map(|error| {
                            json!({
                                "code": format!("{:?}", error.error_type),
                                "message": error.message,
                            })
                        });
                        let mut state_values = json!({
                            "lastSuccess": result.succeeded(),
                            "lastExecutionMs": result.execution_time_ms,
                        });
                        if result.succeeded() && commit_requested_on_success {
                            if let (Some(target), Some(requested)) =
                                (state_values.as_object_mut(), requested_values.as_object())
                            {
                                target.extend(requested.clone());
                            }
                        }
                        crate::command_handler::publish_widget_runtime_state(
                            app_handle,
                            &tx,
                            deck_id,
                            &trigger_id,
                            if result.succeeded() { "ready" } else { "error" },
                            state_values,
                            error,
                        )
                        .await;
                    }
                }

                // Handle WebSocket notifications for flow execution
                if let vortideck_core::services::execution::UniversalAction::Flow {
                    flow_id, ..
                } = &universal_action
                {
                    if result.succeeded() {
                        // Extract execution_id from outputs
                        if let Some(execution_id) = result.outputs.get("execution_id") {
                            println!(
                                "✅ Flow started successfully with execution ID: {}",
                                execution_id
                            );
                            log::info!(
                                "✅ Flow started successfully with execution ID: {}",
                                execution_id
                            );

                            // Send success message to frontend
                            let message = serde_json::json!({
                                "action": "flow_execution_started",
                                "flow_id": flow_id,
                                "execution_id": execution_id
                            });

                            if let Err(e) = tx.send(Message::text(message.to_string())) {
                                println!("❌ Failed to send flow execution message: {}", e);
                            }
                        }
                    } else if let Some(error) = result.error {
                        println!("❌ Flow execution failed: {}", error.message);
                        log::error!("❌ Flow execution failed: {}", error.message);

                        // Send error message to frontend
                        let message = serde_json::json!({
                            "action": "flow_execution_failed",
                            "flow_id": flow_id,
                            "error": error.message
                        });

                        if let Err(e) = tx.send(Message::text(message.to_string())) {
                            println!("❌ Failed to send flow error message: {}", e);
                        }
                    }
                } else {
                    // For non-flow actions, just log
                    if result.succeeded() {
                        println!("✅ Action executed successfully via UniversalExecutionEngine");
                    } else {
                        let error_msg = result
                            .error
                            .map(|e| e.message)
                            .unwrap_or_else(|| "Unknown error".to_string());
                        println!(
                            "❌ Action execution failed via UniversalExecutionEngine: {}",
                            error_msg
                        );
                        use tauri::Emitter;
                        let _ = app_handle.emit(
                            "action_execution_failed",
                            serde_json::json!({
                                "button_id": button_id,
                                "error": error_msg,
                                "is_plan_limit": false
                            }),
                        );
                    }
                }
            } else {
                println!("❌ ActionInvocationService not available");
                publish_widget_dispatch_failure(
                    app_handle,
                    &tx,
                    is_widget_execution,
                    &trigger_deck_id,
                    &trigger_id,
                    "runtime_unavailable",
                    "The action runtime is unavailable.",
                )
                .await;
            }
        } else {
            println!("❌ AppHandle not available");
        }

        println!("✅ Button execution completed");
    });
}

use futures_util::stream::SplitSink;
use tokio_tungstenite::WebSocketStream;
pub(super) async fn handle_get_current_deck(
    write: Arc<
        RwLock<
            Box<
                dyn futures_util::Sink<Message, Error = tokio_tungstenite::tungstenite::Error>
                    + Send
                    + Sync
                    + Unpin,
            >,
        >,
    >,
    db: Arc<Database>,
    icons_base_path: Option<std::path::PathBuf>,
    shared_secret: Option<[u8; 32]>,
) {
    let Some(shared_secret) = shared_secret else {
        eprintln!("🚫 Refusing to send deck snapshot to an unpaired direct client");
        return;
    };
    // Fetch decks from the database
    // Always include icons as base64 to support both direct and relay connections
    let include_icons = true;
    match db
        .get_active_deck_with_buttons(include_icons, icons_base_path.as_deref())
        .await
    {
        Some(decks_with_buttons) => {
            // Serialize the result to JSON
            match serde_json::to_string(&decks_with_buttons) {
                Ok(json) => {
                    // Wrap the response in a structured message without extra serialization
                    let client_message = json!({
                        "action": "active_deck",
                        "deck": serde_json::from_str::<serde_json::Value>(&json).unwrap()
                    });
                    let client_message =
                        match crate::crypto::sign_json_message(&shared_secret, client_message) {
                            Ok(message) => message,
                            Err(error) => {
                                eprintln!("Failed to sign active deck: {}", error);
                                return;
                            }
                        };

                    // Lock the mutex only during the send operation
                    let mut write_guard = write.write().await;
                    if let Err(err) = write_guard
                        .send(Message::Text(client_message.to_string().into()))
                        .await
                    {
                        eprintln!("Failed to send decks to client: {}", err);
                    }
                    drop(write_guard);

                    if let Some(snapshot) =
                        build_deck_widget_snapshot(&db, decks_with_buttons.id).await
                    {
                        let snapshot =
                            match crate::crypto::sign_json_message(&shared_secret, snapshot) {
                                Ok(message) => message,
                                Err(error) => {
                                    eprintln!("Failed to sign deck widget snapshot: {}", error);
                                    return;
                                }
                            };
                        let mut write_guard = write.write().await;
                        if let Err(err) = write_guard
                            .send(Message::Text(snapshot.to_string().into()))
                            .await
                        {
                            eprintln!("Failed to send deck widget snapshot: {}", err);
                        }
                    }
                }
                Err(err) => {
                    eprintln!("Failed to serialize decks: {}", err);
                }
            }
        }
        None => {
            // Log the case when no active deck is found
            eprintln!("No active deck found");
        }
    }
}

pub(super) async fn build_deck_widget_snapshot(
    db: &Database,
    deck_id: i32,
) -> Option<serde_json::Value> {
    let document = db.get_deck_widget_document(deck_id).await.ok()?;
    if document.revision == 0 && document.items.is_empty() {
        return None;
    }

    let mut definitions = Vec::new();
    let mut definition_ids = HashSet::new();
    let mut public_config_paths = HashMap::<String, Vec<String>>::new();
    let mut public_state_paths = HashMap::<String, Vec<String>>::new();
    for item in &document.items {
        if !definition_ids.insert(item.item.definition_id.clone()) {
            continue;
        }
        if let Some(definition) = builtin_widget_definition(&item.item.definition_id) {
            definitions.push(with_widget_content_hash(definition));
            continue;
        }
        let Some(package_id) = item.item.package_id.as_deref() else {
            continue;
        };
        let Some(app_handle) = APP_HANDLE.get() else {
            continue;
        };
        let Some(core) = app_handle.try_state::<vortideck_core::VortiDeckCore>() else {
            continue;
        };
        let Some(manifest) = core.plugin_manager().package_manifest(package_id).await else {
            continue;
        };
        for contribution in &manifest.contributions {
            if let vortideck_core::plugins::Contribution::Widget {
                id,
                name,
                deck_surface: Some(surface),
                ..
            } = contribution
            {
                if surface.definition_id != item.item.definition_id {
                    continue;
                }
                let definition = json!({
                    "id": surface.definition_id,
                    "name": name.as_deref().or(id.as_deref()).unwrap_or("Plugin widget"),
                    "category": "plugins",
                    "rendererId": surface.renderer_id,
                    "interactionType": surface.interaction_type,
                    "defaultSpan": surface.default_span,
                    "minSpan": surface.min_span,
                    "maxSpan": surface.max_span,
                    "configSchema": surface.config_schema,
                    "stateSchema": surface.state_schema,
                    "inputs": surface.inputs,
                    "stateSubscriptions": surface.state_subscriptions,
                    "mobileSupport": if surface.portable_view.is_some() { "host_schema" } else { surface.mobile_support.as_str() },
                    "portableView": surface.portable_view.clone(),
                });
                public_config_paths.insert(
                    surface.definition_id.clone(),
                    surface.public_config_paths.clone(),
                );
                public_state_paths.insert(
                    surface.definition_id.clone(),
                    surface.public_state_paths.clone(),
                );
                definitions.push(with_widget_content_hash(definition));
                break;
            }
        }
    }

    #[cfg(target_os = "windows")]
    let system_volume = vortideck_core::SystemControl::get_volume()
        .ok()
        .map(|volume| volume * 100.0);
    #[cfg(not(target_os = "windows"))]
    let system_volume: Option<f32> = None;
    let system_volume_observed_at = chrono::Utc::now().timestamp_millis().max(0) as u64;
    let mut state: Vec<_> = system_volume
        .map(|volume| {
            document
                .items
                .iter()
                .filter(|item| {
                    matches!(
                        item.item.definition_id.as_str(),
                        "vortideck.rotary" | "vortideck.fader"
                    )
                })
                .map(|item| {
                    json!({
                        "protocolVersion": 1,
                        "instanceId": item.item.id,
                        "revision": 1,
                        "status": "ready",
                        "values": {
                            "value": volume,
                            "muted": volume < 0.1,
                        },
                        "providerStates": {
                            "system.volume": {
                                "revision": 1,
                                "availability": "available",
                                "observedAt": system_volume_observed_at,
                                "values": {
                                    "value": volume,
                                    "muted": volume < 0.1,
                                },
                            },
                        },
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let instance_ids: Vec<_> = document
        .items
        .iter()
        .map(|item| item.item.id.clone())
        .collect();
    for item in &document.items {
        let persists_toggle = item.bindings.iter().any(|binding| {
            serde_json::from_str::<Value>(&binding.policy_json)
                .ok()
                .is_some_and(|policy| {
                    policy.get("type").and_then(Value::as_str) == Some("toggle")
                        && policy.get("persistState").and_then(Value::as_bool) == Some(true)
                })
        });
        if !persists_toggle {
            continue;
        }
        if let Ok(Some(persisted)) = db
            .to_core_database()
            .get_deck_item_persistent_state(&item.item.id)
            .await
        {
            state.push(json!({
                "protocolVersion": 1,
                "instanceId": item.item.id,
                "revision": 1,
                "status": "ready",
                "values": persisted,
            }));
        }
    }
    state.extend(crate::command_handler::widget_runtime_snapshots(deck_id, &instance_ids).await);
    // Initial provider samples, persisted local state and live runtime records
    // may all describe one instance. Send exactly one snapshot per instance;
    // later runtime metadata wins while earlier observed values fill only gaps.
    let mut state_by_instance = HashMap::<String, Value>::new();
    for mut snapshot in state {
        let Some(instance_id) = snapshot
            .get("instanceId")
            .and_then(Value::as_str)
            .map(str::to_string)
        else {
            continue;
        };
        if let Some(previous) = state_by_instance.remove(&instance_id) {
            let mut values = previous
                .get("values")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            if let Some(current) = snapshot.get("values").and_then(Value::as_object) {
                values.extend(current.clone());
            }
            snapshot["values"] = Value::Object(values);
            let mut providers = previous
                .get("providerStates")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            if let Some(current) = snapshot.get("providerStates").and_then(Value::as_object) {
                providers.extend(current.clone());
            }
            snapshot["providerStates"] = Value::Object(providers);
        }
        state_by_instance.insert(instance_id, snapshot);
    }
    let mut state = state_by_instance.into_values().collect::<Vec<_>>();
    state.sort_by(|left, right| {
        left.get("instanceId")
            .and_then(Value::as_str)
            .cmp(&right.get("instanceId").and_then(Value::as_str))
    });
    let definition_by_instance = document
        .items
        .iter()
        .map(|item| (item.item.id.as_str(), item.item.definition_id.as_str()))
        .collect::<HashMap<_, _>>();
    for snapshot in &mut state {
        let Some(instance_id) = snapshot.get("instanceId").and_then(Value::as_str) else {
            continue;
        };
        let Some(definition_id) = definition_by_instance.get(instance_id) else {
            continue;
        };
        if definition_id.starts_with("vortideck.") {
            continue;
        }
        let paths = public_state_paths
            .get(*definition_id)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let projected = project_public_json(snapshot.get("values").unwrap_or(&Value::Null), paths);
        snapshot["values"] = projected;
        if let Some(providers) = snapshot
            .get_mut("providerStates")
            .and_then(Value::as_object_mut)
        {
            for provider in providers.values_mut() {
                provider["values"] =
                    project_public_json(provider.get("values").unwrap_or(&Value::Null), paths);
            }
        }
        if let Some(request) = snapshot.get_mut("request") {
            request["values"] =
                project_public_json(request.get("values").unwrap_or(&Value::Null), paths);
        }
    }

    let instances: Vec<_> = document
        .items
        .into_iter()
        .map(|document| {
            let layouts = document
                .layouts
                .into_iter()
                .map(|layout| {
                    (
                        layout.layout_key,
                        json!({
                            "x": layout.x,
                            "y": layout.y,
                            "cols": layout.col_span,
                            "rows": layout.row_span,
                        }),
                    )
                })
                .collect::<serde_json::Map<String, serde_json::Value>>();
            let available_ports = document
                .bindings
                .iter()
                .map(|binding| binding.port_id.clone())
                .collect::<HashSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            let parsed_config = serde_json::from_str::<serde_json::Value>(&document.item.config_json)
                .unwrap_or(json!({}));
            let config = if document.item.definition_id.starts_with("vortideck.") {
                parsed_config
            } else {
                let paths = public_config_paths
                    .get(&document.item.definition_id)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                project_public_json(&parsed_config, paths)
            };
            json!({
                "id": document.item.id,
                "definitionId": document.item.definition_id,
                "packageId": document.item.package_id,
                "contributionId": document.item.contribution_id,
                "configVersion": document.item.config_version,
                "config": config,
                "appearance": serde_json::from_str::<serde_json::Value>(&document.item.appearance_json).unwrap_or(json!({})),
                "layouts": layouts,
                "availablePorts": available_ports,
                "legacyButtonId": document.item.legacy_button_id,
                "enabled": document.item.enabled,
            })
        })
        .collect();

    Some(json!({
        "action": "deck_widget_snapshot",
        "protocolVersion": 1,
        "deckId": deck_id,
        "revision": document.revision,
        "layoutKey": "portrait",
        "definitions": definitions,
        "instances": instances,
        "state": state,
    }))
}

fn portable_asset_matches(reference: &Value, bytes: &[u8]) -> bool {
    use sha2::{Digest, Sha256};
    let Some(media_type) = reference.get("mediaType").and_then(Value::as_str) else {
        return false;
    };
    let Some(expected_length) = reference.get("byteLength").and_then(Value::as_u64) else {
        return false;
    };
    let Some(expected_hash) = reference.get("sha256").and_then(Value::as_str) else {
        return false;
    };
    let valid_magic = match media_type {
        "image/png" => bytes.starts_with(&[137, 80, 78, 71, 13, 10, 26, 10]),
        "image/jpeg" => bytes.starts_with(&[0xff, 0xd8]),
        "image/webp" => bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP".as_slice()),
        _ => false,
    };
    valid_magic
        && bytes.len() as u64 == expected_length
        && format!("{:x}", Sha256::digest(bytes)) == expected_hash
}

pub(super) async fn build_deck_widget_manifest(db: &Database, deck_id: i32) -> Option<Value> {
    let snapshot = build_deck_widget_snapshot(db, deck_id).await?;
    let definitions = snapshot
        .get("definitions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let definition_refs = definitions
        .iter()
        .filter_map(|definition| {
            Some(json!({
                "definitionId": definition.get("id")?.as_str()?,
                "contentHash": definition.get("contentHash")?.as_str()?,
            }))
        })
        .collect::<Vec<_>>();
    let mut asset_hashes = HashSet::new();
    let assets = definitions
        .iter()
        .flat_map(|definition| {
            definition
                .pointer("/portableView/assets")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
        })
        .filter(|asset| {
            asset
                .get("sha256")
                .and_then(Value::as_str)
                .is_some_and(|hash| asset_hashes.insert(hash.to_string()))
        })
        .cloned()
        .collect::<Vec<_>>();
    Some(json!({
        "action": "deck_widget_manifest",
        "protocolVersion": 2,
        "schemaVersions": [1],
        "deckId": deck_id,
        "deckRevision": snapshot.get("revision").and_then(Value::as_i64).unwrap_or(0),
        "definitions": definition_refs,
        "assets": assets,
    }))
}

pub(super) async fn build_deck_widget_objects(
    db: &Database,
    deck_id: i32,
    cached_definition_hashes: &HashSet<String>,
    cached_asset_hashes: &HashSet<String>,
) -> Option<Value> {
    use base64::{engine::general_purpose::STANDARD, Engine as _};

    let snapshot = build_deck_widget_snapshot(db, deck_id).await?;
    let definitions = snapshot
        .get("definitions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let missing_definitions = definitions
        .iter()
        .filter(|definition| {
            definition
                .get("contentHash")
                .and_then(Value::as_str)
                .is_some_and(|hash| !cached_definition_hashes.contains(hash))
        })
        .cloned()
        .collect::<Vec<_>>();

    let document = db.get_deck_widget_document(deck_id).await.ok()?;
    let package_by_definition = document
        .items
        .iter()
        .filter_map(|item| {
            item.item
                .package_id
                .as_ref()
                .map(|package_id| (item.item.definition_id.clone(), package_id.clone()))
        })
        .collect::<HashMap<_, _>>();
    let app_handle = APP_HANDLE.get()?;
    let core = app_handle.try_state::<vortideck_core::VortiDeckCore>()?;
    let mut emitted = HashSet::new();
    let mut assets = Vec::new();
    for definition in &definitions {
        let Some(definition_id) = definition.get("id").and_then(Value::as_str) else {
            continue;
        };
        let Some(package_id) = package_by_definition.get(definition_id) else {
            continue;
        };
        let Some(references) = definition
            .pointer("/portableView/assets")
            .and_then(Value::as_array)
        else {
            continue;
        };
        for reference in references {
            let Some(hash) = reference.get("sha256").and_then(Value::as_str) else {
                continue;
            };
            if cached_asset_hashes.contains(hash) || !emitted.insert(hash.to_string()) {
                continue;
            }
            let Some(asset_id) = reference.get("id").and_then(Value::as_str) else {
                continue;
            };
            let Some(bytes) = core
                .plugin_manager()
                .get_plugin_asset(package_id, asset_id)
                .await
            else {
                continue;
            };
            if !portable_asset_matches(reference, &bytes) {
                log::warn!(
                    "Portable asset '{}' for '{}' failed manifest integrity validation",
                    asset_id,
                    definition_id
                );
                continue;
            }
            assets.push(json!({
                "asset": reference,
                "data": STANDARD.encode(bytes),
            }));
        }
    }

    Some(json!({
        "action": "deck_widget_objects",
        "protocolVersion": 2,
        "deckId": deck_id,
        "definitions": missing_definitions,
        "assets": assets,
    }))
}

#[cfg(test)]
mod portable_asset_tests {
    use super::*;

    #[test]
    fn portable_assets_require_hash_length_media_and_magic_to_match() {
        let bytes = [137, 80, 78, 71, 13, 10, 26, 10];
        let reference = json!({
            "sha256": "4c4b6a3be1314ab86138bef4314dde022e600960d8689a2c8f8631802d20dab6",
            "mediaType": "image/png",
            "byteLength": 8,
        });
        assert!(portable_asset_matches(&reference, &bytes));
        assert!(!portable_asset_matches(
            &json!({ "sha256": reference["sha256"], "mediaType": "image/jpeg", "byteLength": 8 }),
            &bytes,
        ));
        assert!(!portable_asset_matches(&reference, b"tampered"));
    }
}

fn with_widget_content_hash(mut definition: Value) -> Value {
    use sha2::{Digest, Sha256};
    let canonical =
        serde_json_canonicalizer::to_string(&definition).unwrap_or_else(|_| definition.to_string());
    let hash = format!("{:x}", Sha256::digest(canonical.as_bytes()));
    if let Some(object) = definition.as_object_mut() {
        object.insert("contentHash".into(), Value::String(hash));
    }
    definition
}

fn project_public_json(source: &Value, paths: &[String]) -> Value {
    let mut target = json!({});
    for path in paths {
        let segments = path.split('.').collect::<Vec<_>>();
        let mut current = source;
        let mut found = true;
        for segment in &segments {
            let Some(next) = current.get(*segment) else {
                found = false;
                break;
            };
            current = next;
        }
        if !found {
            continue;
        }
        let mut cursor = &mut target;
        for (index, segment) in segments.iter().enumerate() {
            if index + 1 == segments.len() {
                cursor[*segment] = current.clone();
            } else {
                if !cursor.get(*segment).is_some_and(Value::is_object) {
                    cursor[*segment] = json!({});
                }
                cursor = &mut cursor[*segment];
            }
        }
    }
    target
}

fn builtin_widget_definition(definition_id: &str) -> Option<serde_json::Value> {
    let definition = match definition_id {
        "vortideck.action-button" => json!({
            "id": definition_id,
            "name": "Action Button",
            "category": "actions",
            "rendererId": "legacy-action-button",
            "interactionType": "atomic_button",
            "defaultSpan": { "cols": 1, "rows": 1 },
            "minSpan": { "cols": 1, "rows": 1 },
            "maxSpan": { "cols": 4, "rows": 4 },
            "configSchema": { "type": "object" },
            "inputs": [{
                "id": "primary", "kind": "trigger",
                "supportedPolicies": ["run_once", "repeat_while_pressed", "toggle", "queue"]
            }],
            "mobileSupport": "host_schema",
        }),
        "vortideck.rotary" => json!({
            "id": definition_id,
            "name": "Rotary Control",
            "category": "controls",
            "rendererId": "legacy-rotary",
            "interactionType": "continuous_value",
            "defaultSpan": { "cols": 1, "rows": 1 },
            "minSpan": { "cols": 1, "rows": 1 },
            "maxSpan": { "cols": 4, "rows": 4 },
            "configSchema": { "type": "object" },
            "stateSchema": { "type": "object" },
            "inputs": [{
                "id": "value", "kind": "value", "valueType": "number",
                "min": 0, "max": 100, "supportedDelivery": ["live", "commit", "live_and_commit"]
            }],
            "stateSubscriptions": ["system.volume"],
            "mobileSupport": "host_schema",
        }),
        "vortideck.fader" => json!({
            "id": definition_id, "name": "Fader", "category": "controls",
            "rendererId": "builtin-fader", "interactionType": "continuous_value",
            "defaultSpan": { "cols": 1, "rows": 2 }, "minSpan": { "cols": 1, "rows": 1 },
            "maxSpan": { "cols": 2, "rows": 4 },
            "configSchema": { "type": "object", "properties": {
                "label": { "type": "string" }, "min": { "type": "number" }, "max": { "type": "number" }
            }},
            "inputs": [{ "id": "value", "kind": "value", "valueType": "number", "min": 0, "max": 100,
                "supportedDelivery": ["live", "commit", "live_and_commit"] }],
            "stateSubscriptions": ["system.volume"], "mobileSupport": "host_schema",
        }),
        "vortideck.toggle" => json!({
            "id": definition_id, "name": "Toggle", "category": "controls",
            "rendererId": "builtin-toggle", "interactionType": "atomic_button",
            "defaultSpan": { "cols": 1, "rows": 1 }, "minSpan": { "cols": 1, "rows": 1 },
            "maxSpan": { "cols": 2, "rows": 2 },
            "configSchema": { "type": "object", "properties": { "label": { "type": "string" } } },
            "inputs": [{ "id": "toggle", "kind": "trigger", "supportedPolicies": ["toggle", "run_once"] }],
            "mobileSupport": "host_schema",
        }),
        "vortideck.media-controls" => json!({
            "id": definition_id, "name": "Media Controls", "category": "media",
            "rendererId": "builtin-media-controls", "interactionType": "compound_zones",
            "defaultSpan": { "cols": 3, "rows": 1 }, "minSpan": { "cols": 2, "rows": 1 },
            "maxSpan": { "cols": 4, "rows": 2 }, "configSchema": { "type": "object" },
            "inputs": [
                { "id": "previous", "kind": "trigger", "supportedPolicies": ["run_once", "queue"] },
                { "id": "playPause", "kind": "trigger", "supportedPolicies": ["run_once", "queue"] },
                { "id": "next", "kind": "trigger", "supportedPolicies": ["run_once", "queue"] }
            ],
            "stateSubscriptions": ["media.playback"], "mobileSupport": "host_schema",
        }),
        "vortideck.telemetry" => json!({
            "id": definition_id, "name": "Telemetry", "category": "status",
            "rendererId": "builtin-telemetry", "interactionType": "telemetry_display",
            "defaultSpan": { "cols": 2, "rows": 1 }, "minSpan": { "cols": 1, "rows": 1 },
            "maxSpan": { "cols": 4, "rows": 3 },
            "configSchema": { "type": "object", "properties": {
                "label": { "type": "string" }, "valueKey": { "type": "string" }, "unit": { "type": "string" }
            }},
            "inputs": [], "stateSubscriptions": ["telemetry"], "mobileSupport": "host_schema",
        }),
        _ => return None,
    };
    Some(definition)
}

pub(super) async fn handle_get_all_decks(
    write: Arc<
        RwLock<
            Box<
                dyn futures_util::Sink<Message, Error = tokio_tungstenite::tungstenite::Error>
                    + Send
                    + Sync
                    + Unpin,
            >,
        >,
    >,
    db: Arc<Database>,
    icons_base_path: Option<std::path::PathBuf>,
    shared_secret: Option<[u8; 32]>,
) {
    let Some(shared_secret) = shared_secret else {
        eprintln!("🚫 Refusing to send deck list to an unpaired direct client");
        return;
    };
    // Fetch decks from the database
    // Always include icons as base64 to support both direct and relay connections
    let include_icons = true;
    match db
        .get_all_decks_with_buttons(include_icons, icons_base_path.as_deref())
        .await
    {
        Ok(decks_with_buttons) => {
            // Serialize the result to JSON
            if let Ok(json) = serde_json::to_string(&decks_with_buttons) {
                // Wrap the response in a structured message
                let client_message = json!({
                    "action": "all_decks",
                    "decks": serde_json::from_str::<serde_json::Value>(&json).unwrap()
                });
                let client_message =
                    match crate::crypto::sign_json_message(&shared_secret, client_message) {
                        Ok(message) => message,
                        Err(error) => {
                            eprintln!("Failed to sign deck list: {}", error);
                            return;
                        }
                    };

                // Lock the mutex only during the send operation
                let mut write_guard = write.write().await;
                if let Err(err) = write_guard
                    .send(Message::Text(client_message.to_string().into()))
                    .await
                {
                    eprintln!("Failed to send decks to client: {}", err);
                }
            } else {
                eprintln!("Failed to serialize decks");
            }
        }
        Err(err) => {
            eprintln!("Error fetching decks: {}", err);
        }
    }
}

// ❌ REMOVED: handle_get_feature_limit function
// Feature limits are now sent in bulk via server_info message on connection
// This eliminates the need for individual WebSocket requests per feature

pub(super) async fn handle_update_active_deck(
    pool: tauri::State<'_, DbPool>,
    payload: serde_json::Value,
) -> Result<String, String> {
    if let Some(deck_id) = payload["deck_id"].as_u64() {
        let db = Database::new(&pool);
        log::info!("Updating active deck to: {}", deck_id);

        db.update_active_deck(deck_id as u16)
            .await
            .map(|_| {
                let success_message =
                    format!("Successfully updated the active deck to ID {}", deck_id);
                log::info!("{}", success_message);
                success_message
            })
            .map_err(|err| {
                let error_message = format!("Failed to update active deck: {}", err);
                log::error!("{}", error_message);
                error_message
            })
    } else {
        let error_message = "Invalid payload: missing deck_id".to_string();
        log::error!("{}", error_message);
        Err(error_message)
    }
}

// // Helper function to verify token
//     // Logic to verify if the token exists and is valid
//     app_state.verify_device_token(token).await
// }

// // Helper function to store the device token
//     // Logic to store the token in a persistent database
//     app_state.store_device_token(client_id, token).await
// }
