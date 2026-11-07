use std::sync::Arc;
/// Streaming Connection Module
///
/// Handles establishing and maintaining connection to backend streaming relay server.
/// This module is called:
/// 1. On app startup (if user already has tokens)
/// 2. After successful login (via auth_token_saved event)
use tauri::{AppHandle, Emitter, Manager};
use vortideck_core::db::database::DbPool;
use vortideck_core::services::auth::token_storage::TokenStorage;
use vortideck_core::services::streaming::{models::CloudEvent, StreamingState, StreamingWebSocket};

/// Attempt to establish streaming connection with backend
/// Returns Ok(()) if connected, Err if failed (but doesn't crash)
pub async fn attempt_streaming_connection(app_handle: AppHandle) -> Result<(), String> {
    log::info!("🔄 Attempting streaming relay connection...");

    // 🔒 CHECK: Streaming must be enabled in settings
    let streaming_enabled = {
        use diesel::prelude::*;
        use vortideck_core::db::schema::app_config;

        match app_handle.try_state::<DbPool>() {
            Some(db_pool) => match db_pool.get() {
                Ok(mut conn) => {
                    match app_config::table
                        .first::<vortideck_core::db::models::AppConfig>(&mut conn)
                    {
                        Ok(config) => config.streaming_enabled,
                        Err(_) => {
                            log::warn!(
                                "⚠️ Could not load streaming_enabled from database, defaulting to enabled"
                            );
                            true
                        }
                    }
                }
                Err(_) => {
                    log::warn!("⚠️ Could not get database connection, defaulting to enabled");
                    true
                }
            },
            None => {
                log::warn!("⚠️ Database pool not available, defaulting to enabled");
                true
            }
        }
    };

    if !streaming_enabled {
        log::info!("🚫 Streaming connection disabled in settings");
        return Err("Streaming disabled in settings".to_string());
    }

    // Get streaming state
    let streaming_state = match app_handle.try_state::<StreamingState>() {
        Some(state) => state,
        None => {
            log::error!("❌ StreamingState not available");
            return Err("StreamingState not available".to_string());
        }
    };

    // 🔒 CRITICAL: Hold lock for entire operation to prevent race conditions
    // This ensures only ONE reconnection can happen at a time
    let mut ws_lock = streaming_state.websocket.write().await;

    // Disconnect existing websocket if present (handles stale/broken connections after sleep)
    if let Some(mut existing_ws) = ws_lock.take() {
        log::info!("🔌 Disconnecting existing streaming websocket before reconnect");
        let _ = existing_ws.disconnect(); // Ignore errors, we're reconnecting anyway
    }

    // Get API URL
    let ws_url = std::env::var("WS_URL").unwrap_or_else(|_| "wss://ws.vortideck.com".to_string());
    let api_url =
        std::env::var("API_URL").unwrap_or_else(|_| "https://backend.vortideck.com".to_string());
    // Use ws_url as streaming_ws_url (append streaming path if not included)
    let streaming_ws_url = ws_url.trim_end_matches('/').to_string() + "/ws/streaming/";

    // Get JWT access token
    let access_token = match TokenStorage::get_access_token() {
        Ok(token) => Some(token),
        Err(e) => {
            log::warn!("⚠️ No JWT token available for streaming: {}", e);
            log::info!("   Streaming will connect automatically after login");
            return Err(format!("No token: {}", e));
        }
    };

    log::info!("📡 Initializing streaming relay connection...");

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

    let workflow_ingress = streaming_state
        .workflow_ingress
        .clone()
        .ok_or("Durable workflow stream ingress state is unavailable")?;

    // Create WebSocket with the canonical invocation service and event bus.
    let mut ws = StreamingWebSocket::new(
        streaming_state.message_store.clone(),
        streaming_state.session_manager.clone(),
        streaming_ws_url.clone(),
        access_token.clone(),
    )
    .with_action_invocation_service(invocation_service)
    .with_event_bus(platform_context.event_bus.clone())
    .with_workflow_ingress(workflow_ingress);

    log::info!("✅ StreamingWebSocket created with ActionInvocationService");

    // Set up event callback to emit events to frontend
    let app_handle_clone = app_handle.clone();
    ws.set_event_callback(move |event| {
        log::debug!("🎯 [EMIT] Emitting streaming event to frontend");

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
        }
    });

    // Connect the WebSocket
    match ws.connect().await {
        Ok(_) => {
            log::info!("✅ Streaming relay connected successfully");
        }
        Err(e) => {
            log::error!("❌ Streaming relay connection failed: {}", e);
            return Err(e.to_string());
        }
    }

    // Store the WebSocket instance (lock still held - prevents race conditions)
    *ws_lock = Some(ws);
    log::info!("✅ Streaming relay connection established");

    Ok(())
}
