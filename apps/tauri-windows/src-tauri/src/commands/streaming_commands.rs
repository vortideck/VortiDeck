use serde_json;
use std::collections::HashMap;
use std::sync::Arc;
/// Tauri command wrappers for the streaming service
///
/// These commands wrap the vortideck-core streaming functions with Tauri's command system.
use tauri::{AppHandle, Emitter, Manager, State};
use vortideck_core::services::execution::{
    ActionInvocationService, ButtonExecutionMode, ExecutionContext, InvocationResult, RetryConfig,
    TriggerSource, UniversalAction,
};
use vortideck_core::services::streaming::{
    models::CloudEvent, ChatMessage, PlatformStream, StreamSession, StreamingState,
    StreamingWebSocket,
};

async fn invoke_streaming_action(
    invocation_service: &ActionInvocationService,
    action_id: &str,
    parameters: HashMap<String, serde_json::Value>,
) -> Result<InvocationResult, String> {
    let context = ExecutionContext {
        trigger_source: TriggerSource::Manual {
            user_id: "streaming-ui".to_string(),
        },
        user_id: None,
        session_id: uuid::Uuid::new_v4().to_string(),
        variables: HashMap::new(),
        timeout_ms: Some(10_000),
        retry_config: RetryConfig {
            max_retries: 0,
            retry_delay_ms: 0,
            exponential_backoff: false,
        },
        execution_mode: ButtonExecutionMode::RunOnce,
        execution_config: None,
    };
    Ok(invocation_service
        .invoke_resolved_action(
            &UniversalAction::Integration {
                integration_id: "streaming".to_string(),
                action_id: action_id.to_string(),
                parameters,
            },
            &context,
        )
        .await)
}

async fn account_ids_for_platform(
    state: &StreamingState,
    platform: &str,
) -> Result<Vec<serde_json::Value>, String> {
    let sessions = state
        .session_manager
        .get_all_sessions()
        .await
        .map_err(|error| error.to_string())?;
    let mut account_ids = sessions
        .iter()
        .flat_map(|session| session.platforms.iter())
        .filter(|account| account.platform.to_string().eq_ignore_ascii_case(platform))
        .map(|account| account.account_id.clone())
        .collect::<Vec<_>>();
    account_ids.sort();
    account_ids.dedup();
    if account_ids.is_empty() {
        return Err(format!(
            "No configured streaming account exists for platform '{}'",
            platform
        ));
    }
    Ok(account_ids
        .into_iter()
        .map(serde_json::Value::String)
        .collect())
}

// ============================================================================
// WebSocket Commands
// ============================================================================

#[tauri::command]
pub async fn streaming_connect(
    url: String,
    access_token: Option<String>,
    app_handle: AppHandle,
    state: State<'_, StreamingState>,
) -> Result<(), String> {
    log::info!("🎬 [STREAMING] streaming_connect called with URL: {}", url);
    log::info!(
        "🔑 [STREAMING] Has access token: {}",
        access_token.is_some()
    );

    // Get the canonical invocation service from app state.
    use std::sync::Arc;

    let invocation_service = match app_handle
        .try_state::<Arc<vortideck_core::services::execution::ActionInvocationService>>()
    {
        Some(service) => service.inner().clone(),
        None => {
            log::error!("❌ ActionInvocationService not available");
            return Err("ActionInvocationService not available".to_string());
        }
    };

    let platform_context =
        match app_handle.try_state::<Arc<vortideck_core::core::PlatformContext>>() {
            Some(ctx) => ctx.inner().clone(),
            None => {
                log::error!("❌ PlatformContext not available");
                return Err("PlatformContext not available".to_string());
            }
        };

    // Create WebSocket with the canonical invocation service and event bus.
    let mut ws = StreamingWebSocket::new(
        state.message_store.clone(),
        state.session_manager.clone(),
        url.clone(),
        access_token.clone(),
    )
    .with_action_invocation_service(invocation_service)
    .with_event_bus(platform_context.event_bus.clone());
    if let Some(ingress) = &state.workflow_ingress {
        ws = ws.with_workflow_ingress(ingress.clone());
    }
    log::info!("✅ [STREAMING] StreamingWebSocket created with ActionInvocationService");

    // Set up event callback to emit events to frontend
    let app_handle_clone = app_handle.clone();
    ws.set_event_callback(move |event| {
        log::info!("🎯 [EMIT] Emitting event to frontend: {:?}", event);

        // Broadcast session state change to frontend if stream status changes in background
        if matches!(
            event,
            CloudEvent::StreamOnline { .. } | CloudEvent::StreamOffline { .. }
        ) {
            let _ = app_handle_clone.emit("session-state-changed", ());
        }

        // Emit the event to all listening frontend windows
        if let Err(e) = app_handle_clone.emit("streaming:event", &event) {
            log::error!("❌ [EMIT] Failed to emit streaming event: {}", e);
        } else {
            log::info!("✅ [EMIT] Event emitted successfully");
        }
    });
    log::info!("✅ [STREAMING] Event callback configured");

    // Connect the WebSocket
    log::info!("🔌 [STREAMING] Calling ws.connect()...");
    match ws.connect().await {
        Ok(_) => log::info!("✅ [STREAMING] WebSocket connect() succeeded"),
        Err(e) => {
            log::error!("❌ [STREAMING] WebSocket connect() failed: {}", e);
            return Err(e.to_string());
        }
    }

    // Store the WebSocket instance
    log::info!("💾 [STREAMING] Storing WebSocket in state");
    *state.websocket.write().await = Some(ws);
    log::info!("✅ [STREAMING] streaming_connect completed successfully");

    // 📊 Track streaming connection
    crate::analytics::capture(
        &app_handle,
        crate::analytics::AnalyticsEvent::new("streaming_connected").prop("url", url),
    );

    Ok(())
}

#[tauri::command]
pub async fn streaming_disconnect(state: State<'_, StreamingState>) -> Result<(), String> {
    vortideck_core::services::streaming::commands::streaming_disconnect(&state).await
}

#[tauri::command]
pub async fn streaming_update_token(
    token: String,
    state: State<'_, StreamingState>,
) -> Result<(), String> {
    vortideck_core::services::streaming::commands::streaming_update_token(token, &state).await
}

#[tauri::command]
pub async fn streaming_send_chat_message(
    account_ids: Vec<u64>,
    message: String,
    reply_to: Option<String>,
    invocation_service: State<'_, Arc<ActionInvocationService>>,
) -> Result<InvocationResult, String> {
    invoke_streaming_action(
        &invocation_service,
        "send_chat",
        HashMap::from([
            ("accountIds".to_string(), serde_json::json!(account_ids)),
            ("message".to_string(), serde_json::json!(message)),
            ("replyTo".to_string(), serde_json::json!(reply_to)),
        ]),
    )
    .await
}

#[tauri::command]
pub async fn streaming_ban_user(
    platform: String,
    user_id: String,
    reason: Option<String>,
    state: State<'_, StreamingState>,
    invocation_service: State<'_, Arc<ActionInvocationService>>,
) -> Result<InvocationResult, String> {
    let account_ids = account_ids_for_platform(&state, &platform).await?;
    invoke_streaming_action(
        &invocation_service,
        "ban_user",
        HashMap::from([
            (
                "accountIds".to_string(),
                serde_json::Value::Array(account_ids),
            ),
            ("userId".to_string(), serde_json::json!(user_id)),
            ("reason".to_string(), serde_json::json!(reason)),
        ]),
    )
    .await
}

#[tauri::command]
pub async fn streaming_timeout_user(
    platform: String,
    user_id: String,
    duration_seconds: u32,
    reason: Option<String>,
    state: State<'_, StreamingState>,
    invocation_service: State<'_, Arc<ActionInvocationService>>,
) -> Result<InvocationResult, String> {
    let account_ids = account_ids_for_platform(&state, &platform).await?;
    invoke_streaming_action(
        &invocation_service,
        "timeout_user",
        HashMap::from([
            (
                "accountIds".to_string(),
                serde_json::Value::Array(account_ids),
            ),
            ("userId".to_string(), serde_json::json!(user_id)),
            (
                "durationSeconds".to_string(),
                serde_json::json!(duration_seconds),
            ),
            ("reason".to_string(), serde_json::json!(reason)),
        ]),
    )
    .await
}

#[tauri::command]
pub async fn streaming_delete_message(
    platform: String,
    message_id: String,
    state: State<'_, StreamingState>,
    invocation_service: State<'_, Arc<ActionInvocationService>>,
) -> Result<InvocationResult, String> {
    let account_ids = account_ids_for_platform(&state, &platform).await?;
    invoke_streaming_action(
        &invocation_service,
        "delete_message",
        HashMap::from([
            (
                "accountIds".to_string(),
                serde_json::Value::Array(account_ids),
            ),
            ("messageId".to_string(), serde_json::json!(message_id)),
        ]),
    )
    .await
}

// ============================================================================
// Message Store Commands
// ============================================================================

#[tauri::command]
pub async fn streaming_get_messages(
    account_id: u64,
    limit: Option<usize>,
    state: State<'_, StreamingState>,
) -> Result<Vec<ChatMessage>, String> {
    vortideck_core::services::streaming::commands::streaming_get_messages(account_id, limit, &state)
        .await
}

#[tauri::command]
pub async fn streaming_get_messages_for_accounts(
    account_ids: Vec<u64>,
    limit: Option<usize>,
    state: State<'_, StreamingState>,
) -> Result<Vec<ChatMessage>, String> {
    vortideck_core::services::streaming::commands::streaming_get_messages_for_accounts(
        account_ids,
        limit,
        &state,
    )
    .await
}

// ============================================================================
// Session Manager Commands
// ============================================================================

// Removed: streaming_load_sessions and streaming_save_sessions
// Sessions are now persisted to database automatically. No manual save/load needed.

#[tauri::command]
pub async fn streaming_get_all_sessions(
    state: State<'_, StreamingState>,
) -> Result<Vec<StreamSession>, String> {
    let sessions =
        vortideck_core::services::streaming::commands::streaming_get_all_sessions(&state).await?;
    // Dereference Arc for serialization
    Ok(sessions.iter().map(|s| (**s).clone()).collect())
}

#[tauri::command]
pub async fn streaming_get_session(
    session_id: String,
    state: State<'_, StreamingState>,
) -> Result<StreamSession, String> {
    let session =
        vortideck_core::services::streaming::commands::streaming_get_session(session_id, &state)
            .await?;
    // Dereference Arc for serialization
    Ok((*session).clone())
}

#[tauri::command]
pub async fn streaming_create_session(
    app: AppHandle,
    session: StreamSession,
    state: State<'_, StreamingState>,
) -> Result<StreamSession, String> {
    let session =
        vortideck_core::services::streaming::commands::streaming_create_session(session, &state)
            .await?;

    // Emit to EventBus
    if let Some(event_bus) = &state.event_bus {
        let _ = event_bus.emit(vortideck_core::events::EventPayload::Session(
            vortideck_core::events::SessionEventPayload::SessionCreated {
                session_id: session.id.clone(),
            },
        ));
    }

    let _ = app.emit("session-state-changed", ());
    // Dereference Arc for serialization
    Ok((*session).clone())
}

#[tauri::command]
pub async fn streaming_update_session(
    app: AppHandle,
    session_id: String,
    updates: HashMap<String, serde_json::Value>,
    state: State<'_, StreamingState>,
) -> Result<StreamSession, String> {
    let session = vortideck_core::services::streaming::commands::streaming_update_session(
        session_id, updates, &state,
    )
    .await?;

    // Emit to EventBus
    if let Some(event_bus) = &state.event_bus {
        let _ = event_bus.emit(vortideck_core::events::EventPayload::Session(
            vortideck_core::events::SessionEventPayload::SessionUpdated {
                session_id: session.id.clone(),
            },
        ));
    }

    let _ = app.emit("session-state-changed", ());
    // Dereference Arc for serialization
    Ok((*session).clone())
}

#[tauri::command]
pub async fn streaming_delete_session(
    app: AppHandle,
    session_id: String,
    state: State<'_, StreamingState>,
) -> Result<(), String> {
    vortideck_core::services::streaming::commands::streaming_delete_session(
        session_id.clone(),
        &state,
    )
    .await?;

    // Emit to EventBus
    if let Some(event_bus) = &state.event_bus {
        let _ = event_bus.emit(vortideck_core::events::EventPayload::Session(
            vortideck_core::events::SessionEventPayload::SessionDeleted { session_id },
        ));
    }

    let _ = app.emit("session-state-changed", ());
    Ok(())
}

#[tauri::command]
pub async fn streaming_activate_session(
    app: AppHandle,
    session_id: String,
    state: State<'_, StreamingState>,
) -> Result<StreamSession, String> {
    let session = vortideck_core::services::streaming::commands::streaming_activate_session(
        session_id, &state,
    )
    .await?;

    // Emit to EventBus
    if let Some(event_bus) = &state.event_bus {
        let mut account_ids = Vec::new();
        for platform_stream in &session.platforms {
            if let Ok(account_id) = platform_stream.account_id.parse::<u64>() {
                account_ids.push(account_id);
            }
        }
        let _ = event_bus.emit(vortideck_core::events::EventPayload::Session(
            vortideck_core::events::SessionEventPayload::SessionActivated {
                session_id: session.id.clone(),
                account_ids,
            },
        ));
    }

    let _ = app.emit("session-state-changed", ());
    // Dereference Arc for serialization
    Ok((*session).clone())
}

#[tauri::command]
pub async fn streaming_start_session(
    app: AppHandle,
    session_id: String,
    state: State<'_, StreamingState>,
) -> Result<StreamSession, String> {
    let session =
        vortideck_core::services::streaming::commands::streaming_start_session(session_id, &state)
            .await?;

    // Emit to EventBus
    if let Some(event_bus) = &state.event_bus {
        let mut account_ids = Vec::new();
        for platform_stream in &session.platforms {
            if let Ok(account_id) = platform_stream.account_id.parse::<u64>() {
                account_ids.push(account_id);
            }
        }
        let _ = event_bus.emit(vortideck_core::events::EventPayload::Session(
            vortideck_core::events::SessionEventPayload::SessionActivated {
                session_id: session.id.clone(),
                account_ids,
            },
        ));
    }

    let _ = app.emit("session-state-changed", ());
    // Dereference Arc for serialization
    Ok((*session).clone())
}

#[tauri::command]
pub async fn streaming_pause_session(
    app: AppHandle,
    session_id: String,
    state: State<'_, StreamingState>,
) -> Result<StreamSession, String> {
    let session =
        vortideck_core::services::streaming::commands::streaming_pause_session(session_id, &state)
            .await?;

    // Emit to EventBus
    if let Some(event_bus) = &state.event_bus {
        let _ = event_bus.emit(vortideck_core::events::EventPayload::Session(
            vortideck_core::events::SessionEventPayload::SessionStateChanged,
        ));
    }

    let _ = app.emit("session-state-changed", ());
    // Dereference Arc for serialization
    Ok((*session).clone())
}

#[tauri::command]
pub async fn streaming_resume_session(
    app: AppHandle,
    session_id: String,
    state: State<'_, StreamingState>,
) -> Result<StreamSession, String> {
    let session =
        vortideck_core::services::streaming::commands::streaming_resume_session(session_id, &state)
            .await?;

    // Emit to EventBus
    if let Some(event_bus) = &state.event_bus {
        let mut account_ids = Vec::new();
        for platform_stream in &session.platforms {
            if let Ok(account_id) = platform_stream.account_id.parse::<u64>() {
                account_ids.push(account_id);
            }
        }
        let _ = event_bus.emit(vortideck_core::events::EventPayload::Session(
            vortideck_core::events::SessionEventPayload::SessionActivated {
                session_id: session.id.clone(),
                account_ids,
            },
        ));
    }

    let _ = app.emit("session-state-changed", ());
    // Dereference Arc for serialization
    Ok((*session).clone())
}

#[tauri::command]
pub async fn streaming_end_session(
    app: AppHandle,
    session_id: String,
    auto: bool,
    state: State<'_, StreamingState>,
) -> Result<StreamSession, String> {
    let session = vortideck_core::services::streaming::commands::streaming_end_session(
        session_id, auto, &state,
    )
    .await?;

    // Emit to EventBus
    if let Some(event_bus) = &state.event_bus {
        let _ = event_bus.emit(vortideck_core::events::EventPayload::Session(
            vortideck_core::events::SessionEventPayload::SessionDeactivated {
                session_id: session.id.clone(),
            },
        ));
    }

    let _ = app.emit("session-state-changed", ());
    // Dereference Arc for serialization
    Ok((*session).clone())
}

#[tauri::command]
pub async fn streaming_check_conflicts(
    platforms: Vec<PlatformStream>,
    state: State<'_, StreamingState>,
) -> Result<Vec<StreamSession>, String> {
    let conflicts =
        vortideck_core::services::streaming::commands::streaming_check_conflicts(platforms, &state)
            .await?;
    // Dereference Arc for serialization
    Ok(conflicts.iter().map(|s| (**s).clone()).collect())
}

#[tauri::command]
pub async fn streaming_check_auto_resume(
    platform: String,
    account_id: String,
    state: State<'_, StreamingState>,
) -> Result<Option<StreamSession>, String> {
    let session_opt = vortideck_core::services::streaming::commands::streaming_check_auto_resume(
        platform, account_id, &state,
    )
    .await?;
    // Dereference Arc for serialization
    Ok(session_opt.map(|s| (*s).clone()))
}

#[tauri::command]
pub async fn streaming_find_session_by_account(
    platform: String,
    account_id: String,
    state: State<'_, StreamingState>,
) -> Result<Option<StreamSession>, String> {
    let session_opt =
        vortideck_core::services::streaming::commands::streaming_find_session_by_account(
            platform, account_id, &state,
        )
        .await?;
    // Dereference Arc for serialization
    Ok(session_opt.map(|s| (*s).clone()))
}

#[tauri::command]
pub async fn streaming_is_session_live(
    session_id: String,
    state: State<'_, StreamingState>,
) -> Result<bool, String> {
    vortideck_core::services::streaming::commands::streaming_is_session_live(session_id, &state)
        .await
}

#[tauri::command]
pub async fn streaming_get_session_account_ids(
    session_id: String,
    state: State<'_, StreamingState>,
) -> Result<Vec<String>, String> {
    vortideck_core::services::streaming::commands::streaming_get_session_account_ids(
        session_id, &state,
    )
    .await
}

#[tauri::command]
pub async fn streaming_get_active_sessions(
    state: State<'_, StreamingState>,
) -> Result<Vec<StreamSession>, String> {
    let sessions =
        vortideck_core::services::streaming::commands::streaming_get_active_sessions(&state)
            .await?;
    // Dereference Arc for serialization
    Ok(sessions.iter().map(|s| (**s).clone()).collect())
}

#[tauri::command]
pub async fn streaming_get_draft_sessions(
    state: State<'_, StreamingState>,
) -> Result<Vec<StreamSession>, String> {
    let sessions =
        vortideck_core::services::streaming::commands::streaming_get_draft_sessions(&state).await?;
    // Dereference Arc for serialization
    Ok(sessions.iter().map(|s| (**s).clone()).collect())
}

#[tauri::command]
pub async fn streaming_get_recent_sessions(
    limit: usize,
    state: State<'_, StreamingState>,
) -> Result<Vec<StreamSession>, String> {
    let sessions =
        vortideck_core::services::streaming::commands::streaming_get_recent_sessions(limit, &state)
            .await?;
    // Dereference Arc for serialization
    Ok(sessions.iter().map(|s| (**s).clone()).collect())
}

#[tauri::command]
pub async fn update_data_subscriptions(
    sources: Vec<String>,
    app_handle: AppHandle,
) -> Result<(), String> {
    log::info!(
        "🎬 [STREAMING] update_data_subscriptions called with sources: {:?}",
        sources
    );
    // Non-op for now to satisfy IPC layer; returns success to clear console errors.
    Ok(())
}
