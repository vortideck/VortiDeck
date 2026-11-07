use super::error::{StreamingError, StreamingResult};
use super::message_store::MessageStore;
use super::models::CloudEvent;
use super::session_manager::SessionManager;
use crate::events::{EventBus, EventPayload};
use crate::services::flow::event_router::DurableStreamEventIngressState;
use futures_util::{SinkExt, StreamExt};
use serde_json;
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Instant;
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

// Import action execution components
use super::command_policy::{
    check_command_permission, command_args, delivery_identity, substitute_action,
    validate_delivery_time,
};
use crate::services::execution::{
    ActionInvocationService, ButtonExecutionMode, ExecutionContext, RetryConfig, TriggerSource,
};

/// Callback type for event notifications
pub type EventCallback = Arc<dyn Fn(CloudEvent) + Send + Sync>;
pub type CleanupCallback = Arc<dyn Fn() + Send + Sync>;

/// Persistent WebSocket connection to backend streaming relay
///
/// This is a Tauri-independent WebSocket client that maintains a persistent
/// connection and handles reconnection logic. Events are delivered via callbacks.
pub struct StreamingWebSocket {
    message_store: Arc<MessageStore>,
    session_manager: Arc<SessionManager>,
    url: String,
    access_token: Arc<RwLock<Option<String>>>,
    /// Callback invoked for each CloudEvent received
    event_callback: Option<EventCallback>,
    /// Callback invoked when the task exits (for cleanup, e.g., resetting atomic flags)
    cleanup_callback: Option<CleanupCallback>,
    /// Channel to send commands to the WebSocket task
    command_tx: Option<mpsc::UnboundedSender<WebSocketCommand>>,
    /// Handle to the WebSocket background task
    task_handle: Option<tokio::task::JoinHandle<()>>,
    /// True only while the relay socket is actually open, not while reconnecting.
    connected: Arc<AtomicBool>,
    /// Canonical service for resolving and invoking saved actions.
    action_invocation_service: Option<Arc<ActionInvocationService>>,
    /// Cooldown tracking: trigger -> last execution time
    cooldowns: Arc<RwLock<HashMap<String, Instant>>>,
    /// Event bus for broadcasting events to the rest of the application
    event_bus: Option<Arc<EventBus>>,
    workflow_ingress: Option<DurableStreamEventIngressState>,
}

enum WebSocketCommand {
    Disconnect,
    UpdateToken(String),
    SendMessage(String), // Send raw JSON message to WebSocket
}

impl StreamingWebSocket {
    pub fn new(
        message_store: Arc<MessageStore>,
        session_manager: Arc<SessionManager>,
        url: String,
        access_token: Option<String>,
    ) -> Self {
        Self {
            message_store,
            session_manager,
            url,
            access_token: Arc::new(RwLock::new(access_token)),
            event_callback: None,
            cleanup_callback: None,
            command_tx: None,
            task_handle: None,
            connected: Arc::new(AtomicBool::new(false)),
            action_invocation_service: None,
            cooldowns: Arc::new(RwLock::new(HashMap::new())),
            event_bus: None,
            workflow_ingress: None,
        }
    }

    /// Set the canonical action invocation boundary.
    pub fn with_action_invocation_service(
        mut self,
        action_invocation_service: Arc<ActionInvocationService>,
    ) -> Self {
        self.action_invocation_service = Some(action_invocation_service);
        self
    }

    /// Set the event bus for broadcasting events
    pub fn with_event_bus(mut self, event_bus: Arc<EventBus>) -> Self {
        self.event_bus = Some(event_bus);
        self
    }

    pub fn with_workflow_ingress(mut self, ingress: DurableStreamEventIngressState) -> Self {
        self.workflow_ingress = Some(ingress);
        self
    }

    /// Set the callback for event notifications
    pub fn set_event_callback<F>(&mut self, callback: F)
    where
        F: Fn(CloudEvent) + Send + Sync + 'static,
    {
        self.event_callback = Some(Arc::new(callback));
    }

    /// Set a cleanup callback that's invoked when the WebSocket task exits
    pub fn set_cleanup_callback<F>(&mut self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.cleanup_callback = Some(Arc::new(callback));
    }

    /// Check if WebSocket is connected
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Acquire)
    }

    /// Connect to the WebSocket and start listening for messages
    pub async fn connect(&mut self) -> StreamingResult<()> {
        // If already connected, abort the old task first
        if let Some(handle) = self.task_handle.take() {
            if !handle.is_finished() {
                handle.abort();
                log::info!("🔌 Aborted previous WebSocket task");
            }
        }
        self.connected.store(false, Ordering::Release);

        let url = self.url.clone();
        log::info!("🔌 Connecting to streaming WebSocket: {}", url);

        // Create command channel
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        self.command_tx = Some(command_tx);

        let message_store = self.message_store.clone();
        let session_manager = self.session_manager.clone();
        let access_token = self.access_token.clone();
        let event_callback = self.event_callback.clone();
        let cleanup_callback = self.cleanup_callback.clone();
        let action_invocation_service = self.action_invocation_service.clone();
        let cooldowns = self.cooldowns.clone();
        let event_bus = self.event_bus.clone();
        let workflow_ingress = self.workflow_ingress.clone();
        let connected = self.connected.clone();

        // Spawn WebSocket task and store the handle
        let task_handle = tokio::spawn(async move {
            Self::websocket_task(
                url,
                access_token,
                message_store,
                session_manager,
                event_callback,
                command_rx,
                action_invocation_service,
                cooldowns,
                event_bus,
                workflow_ingress,
                connected.clone(),
            )
            .await;

            connected.store(false, Ordering::Release);

            // Call cleanup callback when task exits
            if let Some(cleanup) = cleanup_callback {
                cleanup();
            }
        });

        self.task_handle = Some(task_handle);
        Ok(())
    }

    /// WebSocket background task
    async fn websocket_task(
        url: String,
        access_token: Arc<RwLock<Option<String>>>,
        message_store: Arc<MessageStore>,
        session_manager: Arc<SessionManager>,
        event_callback: Option<EventCallback>,
        mut command_rx: mpsc::UnboundedReceiver<WebSocketCommand>,
        action_invocation_service: Option<Arc<ActionInvocationService>>,
        cooldowns: Arc<RwLock<HashMap<String, Instant>>>,
        event_bus: Option<Arc<EventBus>>,
        workflow_ingress: Option<DurableStreamEventIngressState>,
        connected: Arc<AtomicBool>,
    ) {
        let mut reconnect_attempts = 0;
        let mut backoff_seconds = 1;
        let mut last_close_code: Option<u16> = None;

        loop {
            // Build URL with current token
            let current_token = access_token.read().await.clone();
            let ws_url = if let Some(token) = current_token {
                format!("{}?access_token={}", url, token)
            } else {
                url.clone()
            };

            match connect_async(&ws_url).await {
                Ok((ws_stream, _)) => {
                    log::info!("✅ Connected to streaming WebSocket");
                    connected.store(true, Ordering::Release);
                    reconnect_attempts = 0;
                    backoff_seconds = 1;

                    let (mut write, mut read) = ws_stream.split();

                    // Message handling loop
                    loop {
                        tokio::select! {
                            // Handle incoming WebSocket messages
                            Some(msg) = read.next() => {
                                match msg {
                                    Ok(Message::Text(text)) => {
                                        log::info!("📥 [WS] Received WebSocket message: {}", log_preview(&text, 200));
                                        if let Err(e) = Self::handle_message(
                                            &text,
                                            &message_store,
                                            &session_manager,
                                            &event_callback,
                                            &action_invocation_service,
                                            &cooldowns,
                                            &event_bus,
                                            &workflow_ingress,
                                        ).await {
                                            log::error!("❌ Error handling message: {}", e);
                                        }
                                    }
                                    Ok(Message::Close(frame)) => {
                                        if let Some(close_frame) = frame {
                                            let code = close_frame.code.into();
                                            log::warn!("⚠️ WebSocket closed by server: code={}, reason={}",
                                                code, close_frame.reason);

                                            // Store close code for token refresh check
                                            last_close_code = Some(code);
                                        } else {
                                            log::warn!("⚠️ WebSocket closed by server (no close frame)");
                                        }
                                        break;
                                    }
                                    Ok(Message::Ping(data)) => {
                                        if let Err(e) = write.send(Message::Pong(data)).await {
                                            log::error!("❌ Error sending pong: {}", e);
                                            break;
                                        }
                                    }
                                    Err(e) => {
                                        log::error!("❌ WebSocket error: {}", e);
                                        break;
                                    }
                                    _ => {}
                                }
                            }

                            // Handle commands from the main thread
                            Some(cmd) = command_rx.recv() => {
                                match cmd {
                                    WebSocketCommand::Disconnect => {
                                        log::info!("🔌 Disconnecting WebSocket...");
                                        let _ = write.close().await;
                                        return; // Exit task completely
                                    }
                                    WebSocketCommand::UpdateToken(new_token) => {
                                        log::info!("🔑 Token updated, reconnecting...");
                                        *access_token.write().await = Some(new_token);
                                        let _ = write.close().await;
                                        break; // Break to reconnect with new token
                                    }
                                    WebSocketCommand::SendMessage(msg) => {
                                        log::debug!("📤 Sending message: {}", msg);
                                        if let Err(e) = write.send(Message::Text(msg.into())).await {
                                            log::error!("❌ Error sending message: {}", e);
                                        }
                                    }
                                }
                            }
                        }
                    }

                    connected.store(false, Ordering::Release);
                    log::info!("🔌 WebSocket connection closed");

                    // Check if connection was closed due to auth error (codes 4001 or 4003)
                    if let Some(code) = last_close_code {
                        if code == 4001 || code == 4003 {
                            log::warn!("🔐 Connection closed with auth error code {}, attempting token refresh...", code);

                            // Try to refresh the token
                            use crate::services::auth::oauth::OAuthService;
                            let api_url = url
                                .replace("wss://", "https://")
                                .replace("ws://", "http://")
                                .split("/ws/")
                                .next()
                                .unwrap_or("")
                                .to_string();

                            match OAuthService::refresh_access_token_with_reqwest(&api_url).await {
                                Ok(new_token) => {
                                    log::info!(
                                        "✅ Access token refreshed successfully for streaming"
                                    );
                                    *access_token.write().await = Some(new_token);
                                    backoff_seconds = 1; // Reset backoff after successful refresh
                                    reconnect_attempts = 0;
                                    last_close_code = None;
                                    continue; // Retry immediately with new token
                                }
                                Err(refresh_err) => {
                                    let error_str = refresh_err.to_string();
                                    log::error!("❌ Failed to refresh token: {}", refresh_err);

                                    // Check if refresh token is missing (user needs to log in again)
                                    if error_str.contains("Token not found")
                                        || error_str.contains("refresh_token")
                                    {
                                        log::error!("🔐 Refresh token missing - user session expired completely");
                                        log::error!("   User MUST log in again to restore streaming connection");
                                        log::error!("⏹️ Stopping reconnection loop - auth_token_saved event will restart");

                                        // Emit error event to notify frontend
                                        if let Some(callback) = &event_callback {
                                            callback(CloudEvent::Error {
                                                message: "Session expired. Please log in again to enable streaming.".to_string(),
                                                status: Some("auth_required".to_string()),
                                            });
                                        }

                                        // Stop loop - auth_token_saved listener will spawn new loop on login
                                        return;
                                    } else {
                                        log::warn!(
                                            "   Please log in again to enable streaming connection"
                                        );
                                    }
                                }
                            }
                        }
                    }
                    last_close_code = None; // Reset for next iteration
                }
                Err(e) => {
                    connected.store(false, Ordering::Release);
                    log::error!("❌ Failed to connect to WebSocket: {}", e);
                    reconnect_attempts += 1;
                }
            }

            // Exponential backoff (will retry forever until explicitly disconnected)
            log::info!(
                "🔄 Reconnecting in {} seconds... (attempt #{})",
                backoff_seconds,
                reconnect_attempts + 1
            );
            tokio::time::sleep(tokio::time::Duration::from_secs(backoff_seconds)).await;
            backoff_seconds = (backoff_seconds * 2).min(30); // Cap at 30 seconds
        }
    }

    /// Disconnect from the WebSocket
    pub fn disconnect(&mut self) -> StreamingResult<()> {
        self.connected.store(false, Ordering::Release);
        if let Some(tx) = &self.command_tx {
            tx.send(WebSocketCommand::Disconnect)
                .map_err(|e| StreamingError::CommandSendFailed(e.to_string()))?;
        }

        // Abort the task handle
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
        }

        Ok(())
    }

    /// Update the access token and reconnect
    pub fn update_token(&self, token: String) -> StreamingResult<()> {
        let tx = self
            .command_tx
            .as_ref()
            .ok_or(StreamingError::NotConnected)?;

        tx.send(WebSocketCommand::UpdateToken(token))
            .map_err(|e| StreamingError::CommandSendFailed(e.to_string()))?;

        Ok(())
    }

    /// Send a message through the WebSocket
    pub fn send(&self, message: String) -> StreamingResult<()> {
        let tx = self
            .command_tx
            .as_ref()
            .ok_or(StreamingError::NotConnected)?;

        tx.send(WebSocketCommand::SendMessage(message))
            .map_err(|e| StreamingError::CommandSendFailed(e.to_string()))?;

        Ok(())
    }

    /// Handle incoming WebSocket message
    async fn handle_message(
        text: &str,
        message_store: &MessageStore,
        session_manager: &SessionManager,
        event_callback: &Option<EventCallback>,
        action_invocation_service: &Option<Arc<ActionInvocationService>>,
        cooldowns: &Arc<RwLock<HashMap<String, Instant>>>,
        event_bus: &Option<Arc<EventBus>>,
        workflow_ingress: &Option<DurableStreamEventIngressState>,
    ) -> StreamingResult<()> {
        log::info!("📦 [HANDLE] Parsing CloudEvent...");
        // Parse the CloudEvent
        let event: CloudEvent = serde_json::from_str(text)
            .map_err(|e| StreamingError::MessageParseFailed(e.to_string()))?;

        // Log event type
        match &event {
            CloudEvent::ConnectionEstablished { .. } => {
                log::info!("🔗 [EVENT] Connection established")
            }
            CloudEvent::ChatMessage {
                account_id,
                platform,
                ..
            } => {
                log::info!(
                    "💬 [EVENT] Chat message from {} account {}",
                    platform,
                    account_id
                );
            }
            CloudEvent::StreamOnline {
                account_id,
                platform,
                ..
            } => {
                log::info!("🟢 Stream online: {} account {}", platform, account_id);
            }
            CloudEvent::StreamOffline {
                account_id,
                platform,
            } => {
                log::info!("🔴 Stream offline: {} account {}", platform, account_id);
            }
            CloudEvent::ChannelUpdate {
                account_id,
                platform,
                ..
            } => {
                log::info!("📝 Channel update: {} account {}", platform, account_id);
            }
            CloudEvent::Subscription {
                account_id,
                platform,
                ..
            } => {
                log::info!("⭐ Subscription: {} account {}", platform, account_id);
            }
            CloudEvent::Raid {
                account_id,
                platform,
                ..
            } => {
                log::info!("🎉 Raid: {} account {}", platform, account_id);
            }
            CloudEvent::Follow {
                account_id,
                platform,
                ..
            } => {
                log::info!("❤️ Follow: {} account {}", platform, account_id);
            }
            CloudEvent::Bits {
                account_id,
                platform,
                ..
            } => {
                log::info!("💎 Bits: {} account {}", platform, account_id);
            }
            CloudEvent::PlatformStatus { platform, .. } => {
                log::info!("📊 Platform status: {}", platform);
            }
            CloudEvent::SendChatResult { success, .. } => {
                log::debug!("💬 Send chat result: {}", success);
            }
            CloudEvent::Error { message, .. } => {
                log::error!("❌ Server error: {}", message);
            }
            CloudEvent::Ping { .. } => {
                log::debug!("🏓 Ping from server");
            }
        }

        // Store chat messages in MessageStore and process commands
        if let CloudEvent::ChatMessage {
            account_id,
            platform,
            data,
        } = &event
        {
            let mut msg = data.clone();
            // Set account_id from CloudEvent if not already present
            msg.account_id = Some(*account_id);
            msg.platform = platform.clone();

            // Check for commands in the message
            let platform_str = platform.to_string();
            let account_id_str = account_id.to_string();

            // Try to find an active session for this platform/account
            if let Ok(Some(session)) = session_manager
                .find_session_by_account_id(&platform_str, &account_id_str)
                .await
            {
                // Store message with session context (session_id + run_index)
                let session_id = session.id.clone();
                let run_index = session.total_runs.saturating_sub(1).max(0) as i32; // Current run index
                log::info!(
                    "💾 [STORE] Storing message for session {} (run {})",
                    session_id,
                    run_index
                );
                message_store
                    .store_message_with_session(msg.clone(), Some(session_id), Some(run_index))
                    .await?;
                // Only process commands if session is active
                if session.status == super::models::SessionStatus::Active {
                    log::debug!(
                        "🎮 [COMMAND] Checking message for commands in active session: {}",
                        session.name
                    );

                    for cmd in &session.commands {
                        if !cmd.enabled {
                            continue;
                        }
                        let Some(args) = command_args(&cmd.trigger, &msg.message) else {
                            continue;
                        };
                        if !check_command_permission(cmd, &msg, &platform_str) {
                            continue;
                        }
                        if msg.user_id.trim().is_empty() {
                            log::warn!(
                                "Command '{}' requires a stable provider user ID",
                                cmd.trigger
                            );
                            continue;
                        }
                        if let Err(error) =
                            validate_delivery_time(&msg.timestamp, chrono::Utc::now())
                        {
                            log::warn!("Command '{}' rejected: {}", cmd.trigger, error);
                            continue;
                        }
                        let Some(invocation_service) = action_invocation_service else {
                            log::warn!("Chat action runtime is unavailable");
                            continue;
                        };
                        let delivery_id = match delivery_identity(
                            &platform_str,
                            *account_id,
                            &session.id,
                            &cmd.id,
                            &msg.id,
                        ) {
                            Ok(identity) => identity,
                            Err(error) => {
                                log::warn!("Command '{}' rejected: {}", cmd.trigger, error);
                                continue;
                            }
                        };
                        let var_map = HashMap::from([
                            ("username".to_string(), msg.user_name.clone()),
                            (
                                "user_display_name".to_string(),
                                msg.user_display_name.clone(),
                            ),
                            ("platform".to_string(), platform_str.clone()),
                            ("message".to_string(), msg.message.clone()),
                            ("args".to_string(), args.to_string()),
                        ]);
                        let action = match substitute_action(&cmd.action, &var_map) {
                            Ok(action) => action,
                            Err(error) => {
                                log::warn!("Command '{}' rejected: {}", cmd.trigger, error);
                                continue;
                            }
                        };
                        let inputs = match super::command_policy::command_inputs(
                            cmd,
                            &msg,
                            args,
                            &platform_str,
                        ) {
                            Ok(inputs) => inputs,
                            Err(error) => {
                                log::warn!("Command '{}' rejected: {}", cmd.trigger, error);
                                continue;
                            }
                        };
                        let context = ExecutionContext {
                            trigger_source: TriggerSource::WebSocket {
                                client_id: format!(
                                    "stream:{}:{}:{}",
                                    platform_str, account_id, msg.user_id
                                ),
                            },
                            user_id: Some(format!(
                                "{}:{}:{}",
                                platform_str, account_id, msg.user_id
                            )),
                            session_id: session.id.clone(),
                            variables: var_map
                                .into_iter()
                                .map(|(key, value)| (key, serde_json::Value::String(value)))
                                .collect(),
                            timeout_ms: Some(30_000),
                            retry_config: RetryConfig {
                                max_retries: 0,
                                retry_delay_ms: 1_000,
                                exponential_backoff: false,
                            },
                            execution_mode: ButtonExecutionMode::RunOnce,
                            execution_config: Some(
                                serde_json::json!({ "invocationId": delivery_id, "idempotencyKey": delivery_id, "inputs": inputs }),
                            ),
                        };
                        let cooldown_key = format!("{}/{}", session.id, cmd.id);
                        let mut cooldown_guard = cooldowns.write().await;
                        if cmd.cooldown.is_some_and(|seconds| {
                            cooldown_guard
                                .get(&cooldown_key)
                                .is_some_and(|last| last.elapsed().as_secs() < u64::from(seconds))
                        }) {
                            continue;
                        }
                        match invocation_service
                            .submit_database_action(&action, &context)
                            .await
                        {
                            Ok(submission) => {
                                if submission.duplicate {
                                    continue;
                                }
                                if cmd.cooldown.is_some() {
                                    cooldown_guard.insert(cooldown_key, Instant::now());
                                }
                                log::info!(
                                    "Chat command '{}' durably admitted as {}",
                                    cmd.trigger,
                                    submission.invocation_id
                                );
                                let manager = session_manager.clone();
                                let command_session = session.id.clone();
                                let command_run = session.current_run_started_at.clone();
                                let trigger = cmd.trigger.clone();
                                tokio::spawn(async move {
                                    let result = submission.wait().await;
                                    if result.succeeded() {
                                        if let Err(error) = manager
                                            .increment_command_executions_for_run(
                                                &command_session,
                                                command_run.as_deref(),
                                            )
                                            .await
                                        {
                                            log::error!(
                                                "Failed to update command statistics: {}",
                                                error
                                            );
                                        }
                                    } else {
                                        log::warn!(
                                            "Chat command '{}' ended as {:?}",
                                            trigger,
                                            result.status
                                        );
                                    }
                                });
                            }
                            Err(error) => {
                                log::warn!("Command '{}' was not admitted: {}", cmd.trigger, error)
                            }
                        }
                    }
                }
            } else {
                // No active session found for this account - store message without session context
                log::debug!(
                    "💾 [STORE] No session found for account {}, storing without session context",
                    account_id
                );
                message_store.store_message(msg.clone()).await?;
            }
        }

        // Auto-activate sessions when stream goes online
        if let CloudEvent::StreamOnline {
            account_id,
            platform,
            ..
        } = &event
        {
            log::info!("🎬 [SESSION] Stream went live, checking for sessions to activate...");
            let platform_str = platform.to_string();
            let account_id_str = account_id.to_string();

            // Try to find a session by account_id
            match session_manager
                .find_session_by_account_id(&platform_str, &account_id_str)
                .await
            {
                Ok(Some(session)) => {
                    log::info!(
                        "📋 [SESSION] Found session: {} (status: {:?})",
                        session.name,
                        session.status
                    );

                    // Auto-activate if waiting_for_live
                    if session.status == super::models::SessionStatus::WaitingForLive {
                        log::info!(
                            "🚀 [SESSION] Auto-activating session {} -> active",
                            session.id
                        );
                        match session_manager.start_session(&session.id).await {
                            Ok(updated) => {
                                log::info!("✅ [SESSION] Session {} is now active!", updated.name);

                                // Ingest into event bus
                                if let Some(bus) = &event_bus {
                                    let mut account_ids = Vec::new();
                                    for platform_stream in &updated.platforms {
                                        if let Ok(account_id) =
                                            platform_stream.account_id.parse::<u64>()
                                        {
                                            account_ids.push(account_id);
                                        }
                                    }
                                    let _ = bus.emit(crate::events::EventPayload::Session(
                                        crate::events::SessionEventPayload::SessionActivated {
                                            session_id: updated.id.clone(),
                                            account_ids,
                                        },
                                    ));
                                }

                                // Cleanup old session runs (keep only last 2)
                                let run_index = updated.total_runs.saturating_sub(1).max(0) as i32;
                                if let Err(e) =
                                    message_store.cleanup_old_runs(&updated.id, run_index).await
                                {
                                    log::error!(
                                        "❌ [CLEANUP] Failed to cleanup old messages: {}",
                                        e
                                    );
                                } else {
                                    log::info!(
                                        "🧹 [CLEANUP] Cleaned up old message runs for session {}",
                                        updated.id
                                    );
                                }
                            }
                            Err(e) => {
                                log::error!("❌ [SESSION] Failed to activate session: {}", e);
                            }
                        }
                    } else if session.status == super::models::SessionStatus::Inactive {
                        // Auto-resume inactive sessions if enabled
                        if session.settings.auto_resume {
                            log::info!("🔄 [SESSION] Auto-resuming inactive session {} -> waiting_for_live -> active", session.id);
                            match session_manager.activate_session(&session.id).await {
                                Ok(_) => {
                                    // Now start it since stream is already live
                                    match session_manager.start_session(&session.id).await {
                                        Ok(updated) => {
                                            log::info!(
                                                "✅ [SESSION] Session {} resumed and activated!",
                                                updated.name
                                            );

                                            // Ingest into event bus
                                            if let Some(bus) = &event_bus {
                                                let mut account_ids = Vec::new();
                                                for platform_stream in &updated.platforms {
                                                    if let Ok(account_id) =
                                                        platform_stream.account_id.parse::<u64>()
                                                    {
                                                        account_ids.push(account_id);
                                                    }
                                                }
                                                let _ = bus.emit(crate::events::EventPayload::Session(
                                                    crate::events::SessionEventPayload::SessionActivated {
                                                        session_id: updated.id.clone(),
                                                        account_ids,
                                                    }
                                                ));
                                            }

                                            // Cleanup old session runs (keep only last 2)
                                            let run_index =
                                                updated.total_runs.saturating_sub(1).max(0) as i32;
                                            if let Err(e) = message_store
                                                .cleanup_old_runs(&updated.id, run_index)
                                                .await
                                            {
                                                log::error!("❌ [CLEANUP] Failed to cleanup old messages: {}", e);
                                            } else {
                                                log::info!("🧹 [CLEANUP] Cleaned up old message runs for session {}", updated.id);
                                            }
                                        }
                                        Err(e) => {
                                            log::error!(
                                                "❌ [SESSION] Failed to start resumed session: {}",
                                                e
                                            );
                                        }
                                    }
                                }
                                Err(e) => {
                                    log::error!("❌ [SESSION] Failed to resume session: {}", e);
                                }
                            }
                        } else {
                            log::info!(
                                "ℹ️ [SESSION] Session {} is inactive but auto_resume is disabled",
                                session.name
                            );
                        }
                    } else {
                        log::info!(
                            "ℹ️ [SESSION] Session {} is already {:?}, no action needed",
                            session.name,
                            session.status
                        );
                    }
                }
                Ok(None) => {
                    log::info!(
                        "ℹ️ [SESSION] No session found for {} account {}",
                        platform,
                        account_id
                    );
                }
                Err(e) => {
                    log::error!("❌ [SESSION] Error finding session: {}", e);
                }
            }
        }

        // Auto-end or pause sessions when stream goes offline
        if let CloudEvent::StreamOffline {
            account_id,
            platform,
        } = &event
        {
            log::info!("⏹️ [SESSION] Stream went offline, checking for sessions to end/pause...");
            let platform_str = platform.to_string();
            let account_id_str = account_id.to_string();

            match session_manager
                .find_session_by_account_id(&platform_str, &account_id_str)
                .await
            {
                Ok(Some(session)) => {
                    if session.status == super::models::SessionStatus::Active {
                        log::info!(
                            "🛑 [SESSION] Auto-ending session {} due to stream offline",
                            session.name
                        );
                        match session_manager.end_session(&session.id, true).await {
                            Ok(updated) => {
                                log::info!(
                                    "✅ [SESSION] Session {} ended (now {:?})",
                                    updated.name,
                                    updated.status
                                );

                                // Ingest into event bus
                                if let Some(bus) = &event_bus {
                                    let _ = bus.emit(crate::events::EventPayload::Session(
                                        crate::events::SessionEventPayload::SessionDeactivated {
                                            session_id: updated.id.clone(),
                                        },
                                    ));
                                }
                            }
                            Err(e) => {
                                log::error!("❌ [SESSION] Failed to end session: {}", e);
                            }
                        }
                    }
                }
                Ok(None) => {
                    log::info!("ℹ️ [SESSION] No session found for offline stream");
                }
                Err(e) => {
                    log::error!("❌ [SESSION] Error finding session: {}", e);
                }
            }
        }

        if let Some(mut stream_payload) = Self::normalize_to_stream_payload(&event) {
            // Direct ingress and the EventBus's EventRouter both accept this
            // payload; a receipt time lets id-less events derive one durable id.
            stamp_receipt_time(&mut stream_payload);
            // If this event can start a workflow, persistence and routing must
            // finish before lossy live fan-out. A failed durable ingress is
            // reported, but live observers still receive the event.
            if let Some(ingress) = workflow_ingress {
                match ingress.accept(stream_payload.clone()).await {
                    Ok(receipt) => {
                        if matches!(receipt.disposition,
                            crate::services::flow::event_router::StreamIngressDisposition::Duplicate
                            | crate::services::flow::event_router::StreamIngressDisposition::AcceptedExpired) {
                            return Ok(());
                        }
                    }
                    Err(error) => log::error!(
                        "❌ [WORKFLOW] Stream event not accepted by workflow ingress: {}",
                        error
                    ),
                }
            }
            // The EventBus remains the low-latency observation channel for UI,
            // overlays, and non-consequential consumers.
            if let Some(bus) = event_bus {
                log::info!("📢 [EVENTBUS] Emitting stream event to EventBus");
                let _ = bus.emit(EventPayload::Stream(stream_payload));
            }
        }

        // Invoke callback with the event
        if let Some(callback) = event_callback {
            log::info!("📢 [CALLBACK] Invoking event callback");
            callback(event);
            log::info!("✅ [CALLBACK] Event callback invoked successfully");
        } else {
            log::warn!("⚠️ [CALLBACK] No event callback registered!");
        }

        Ok(())
    }

    /// Normalize a CloudEvent into a StreamEventPayload for the EventBus
    fn normalize_to_stream_payload(
        event: &CloudEvent,
    ) -> Option<crate::events::StreamEventPayload> {
        match event {
            CloudEvent::ChatMessage {
                account_id,
                platform,
                data,
            } => Some(crate::events::StreamEventPayload::ChatMessage {
                metadata: crate::events::StreamEventMetadata {
                    event_id: Some(data.id.clone()),
                    occurred_at: chrono::DateTime::parse_from_rfc3339(&data.timestamp)
                        .ok()
                        .map(|value| value.with_timezone(&chrono::Utc)),
                    stream_id: None,
                    account_id: *account_id,
                    platform: platform.to_string(),
                },
                user_name: data.user_name.clone(),
                user_display_name: data.user_display_name.clone(),
                message: data.message.clone(),
                is_moderator: data.is_moderator,
                is_subscriber: data.is_subscriber,
                platform_color: if data.platform_color.is_empty() {
                    None
                } else {
                    Some(data.platform_color.clone())
                },
            }),
            CloudEvent::StreamOnline {
                account_id,
                platform,
                data,
            } => Some(crate::events::StreamEventPayload::StreamOnline {
                metadata: crate::events::StreamEventMetadata {
                    event_id: None,
                    occurred_at: chrono::DateTime::parse_from_rfc3339(&data.started_at)
                        .ok()
                        .map(|value| value.with_timezone(&chrono::Utc)),
                    stream_id: Some(data.stream_id.clone()),
                    account_id: *account_id,
                    platform: platform.to_string(),
                },
                title: Some(data.title.clone()),
            }),
            CloudEvent::StreamOffline {
                account_id,
                platform,
            } => Some(crate::events::StreamEventPayload::StreamOffline {
                metadata: crate::events::StreamEventMetadata::new(
                    *account_id,
                    platform.to_string(),
                ),
            }),
            CloudEvent::ChannelUpdate {
                account_id,
                platform,
                data,
            } => {
                if let Some(viewer_count) = data.viewer_count {
                    Some(crate::events::StreamEventPayload::ViewerCountUpdated {
                        metadata: crate::events::StreamEventMetadata::new(
                            *account_id,
                            platform.to_string(),
                        ),
                        count: viewer_count,
                    })
                } else {
                    None
                }
            }
            CloudEvent::Follow {
                account_id,
                platform,
                data,
            } => {
                let user_name = data
                    .get("userName")
                    .or_else(|| data.get("user_name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let user_display_name = data
                    .get("userDisplayName")
                    .or_else(|| data.get("user_display_name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or(&user_name)
                    .to_string();
                Some(crate::events::StreamEventPayload::Follow {
                    metadata: crate::events::StreamEventMetadata {
                        event_id: Self::json_metadata_string(data, &["eventId", "event_id", "id"]),
                        occurred_at: Self::json_metadata_time(data),
                        stream_id: Self::json_metadata_string(data, &["streamId", "stream_id"]),
                        account_id: *account_id,
                        platform: platform.to_string(),
                    },
                    user_name,
                    user_display_name,
                })
            }
            CloudEvent::Subscription {
                account_id,
                platform,
                data,
            } => {
                let user_name = data
                    .get("userName")
                    .or_else(|| data.get("user_name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let tier = data
                    .get("tier")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let message = data
                    .get("message")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                Some(crate::events::StreamEventPayload::Subscription {
                    metadata: crate::events::StreamEventMetadata {
                        event_id: Self::json_metadata_string(data, &["eventId", "event_id", "id"]),
                        occurred_at: Self::json_metadata_time(data),
                        stream_id: Self::json_metadata_string(data, &["streamId", "stream_id"]),
                        account_id: *account_id,
                        platform: platform.to_string(),
                    },
                    user_name,
                    tier,
                    message,
                })
            }
            CloudEvent::Raid {
                account_id,
                platform,
                data,
            } => {
                let raider = data
                    .get("raider")
                    .or_else(|| data.get("userName"))
                    .or_else(|| data.get("user_name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let viewer_count = data
                    .get("viewerCount")
                    .or_else(|| data.get("viewer_count"))
                    .and_then(|v| v.as_u64())
                    .map(|n| n as u32);
                Some(crate::events::StreamEventPayload::Raid {
                    metadata: crate::events::StreamEventMetadata {
                        event_id: Self::json_metadata_string(data, &["eventId", "event_id", "id"]),
                        occurred_at: Self::json_metadata_time(data),
                        stream_id: Self::json_metadata_string(data, &["streamId", "stream_id"]),
                        account_id: *account_id,
                        platform: platform.to_string(),
                    },
                    raider,
                    viewer_count,
                })
            }
            CloudEvent::Bits {
                account_id,
                platform,
                data,
            } => {
                let user_name = data
                    .get("userName")
                    .or_else(|| data.get("user_name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let amount = data.get("amount").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                let message = data
                    .get("message")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                Some(crate::events::StreamEventPayload::Bits {
                    metadata: crate::events::StreamEventMetadata {
                        event_id: Self::json_metadata_string(data, &["eventId", "event_id", "id"]),
                        occurred_at: Self::json_metadata_time(data),
                        stream_id: Self::json_metadata_string(data, &["streamId", "stream_id"]),
                        account_id: *account_id,
                        platform: platform.to_string(),
                    },
                    user_name,
                    amount,
                    message,
                })
            }
            _ => None,
        }
    }

    fn json_metadata_string(data: &serde_json::Value, keys: &[&str]) -> Option<String> {
        keys.iter()
            .find_map(|key| data.get(*key).and_then(serde_json::Value::as_str))
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    }

    fn json_metadata_time(data: &serde_json::Value) -> Option<chrono::DateTime<chrono::Utc>> {
        let value = Self::json_metadata_string(
            data,
            &[
                "occurredAt",
                "occurred_at",
                "timestamp",
                "createdAt",
                "created_at",
            ],
        )?;
        chrono::DateTime::parse_from_rfc3339(&value)
            .ok()
            .map(|timestamp| timestamp.with_timezone(&chrono::Utc))
    }
}

impl Drop for StreamingWebSocket {
    fn drop(&mut self) {
        log::info!("🔌 StreamingWebSocket dropped, aborting task...");
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
        }
    }
}

/// At most `max_bytes` of `text`, cut back to a char boundary so relay text
/// with multibyte characters can be logged without panicking.
fn log_preview(text: &str, max_bytes: usize) -> &str {
    if text.len() <= max_bytes {
        return text;
    }
    let mut end = max_bytes;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

/// Give an id-less, untimed relay event its receipt time before it is handed to
/// both ingress paths, so they derive the same event id. Events carrying a
/// provider id keep their metadata untouched (observed counters require a
/// provider time, and id-less events never count).
fn stamp_receipt_time(payload: &mut crate::events::StreamEventPayload) {
    use crate::events::StreamEventPayload as P;
    let metadata = match payload {
        P::Follow { metadata, .. }
        | P::Subscription { metadata, .. }
        | P::Raid { metadata, .. }
        | P::Bits { metadata, .. }
        | P::ChatMessage { metadata, .. }
        | P::StreamOnline { metadata, .. }
        | P::StreamOffline { metadata }
        | P::ViewerCountUpdated { metadata, .. } => metadata,
    };
    if metadata.event_id.is_none() && metadata.occurred_at.is_none() {
        metadata.occurred_at = Some(chrono::Utc::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::r2d2::{ConnectionManager, Pool};
    use diesel::SqliteConnection;

    fn test_message_store() -> Arc<MessageStore> {
        let manager = ConnectionManager::<SqliteConnection>::new(":memory:");
        let pool = Pool::builder()
            .max_size(1)
            .build(manager)
            .expect("in-memory message database");
        Arc::new(MessageStore::new(Arc::new(crate::db::Database::new(&pool))))
    }

    #[test]
    fn test_websocket_creation() {
        let message_store = test_message_store();
        let session_manager = Arc::new(SessionManager::new());
        let ws = StreamingWebSocket::new(
            message_store,
            session_manager,
            "ws://localhost:8000/ws/streaming/".to_string(),
            Some("test_token".to_string()),
        );

        assert!(ws.event_callback.is_none());
        assert!(ws.command_tx.is_none());
        assert!(ws.task_handle.is_none());
    }

    #[test]
    fn test_set_event_callback() {
        let message_store = test_message_store();
        let session_manager = Arc::new(SessionManager::new());
        let mut ws = StreamingWebSocket::new(
            message_store,
            session_manager,
            "ws://localhost:8000/ws/streaming/".to_string(),
            None,
        );

        ws.set_event_callback(|event| {
            println!("Event received: {:?}", event);
        });

        assert!(ws.event_callback.is_some());
    }

    #[test]
    fn test_not_connected_initially() {
        let message_store = test_message_store();
        let session_manager = Arc::new(SessionManager::new());
        let ws = StreamingWebSocket::new(
            message_store,
            session_manager,
            "ws://localhost:8000/ws/streaming/".to_string(),
            None,
        );

        assert!(!ws.is_connected());
    }

    #[test]
    fn log_preview_never_splits_a_multibyte_character() {
        // 199 ASCII bytes, then a 4-byte emoji straddling byte 200.
        let text = format!("{}😀tail", "a".repeat(199));
        assert!(!text.is_char_boundary(200));
        assert_eq!(log_preview(&text, 200), "a".repeat(199));
        let polish = "żółć".repeat(100);
        assert!(log_preview(&polish, 200).len() <= 200);
        assert_eq!(log_preview("short", 200), "short");
    }

    #[test]
    fn receipt_time_is_stamped_only_on_id_less_untimed_events() {
        use crate::events::{StreamEventMetadata, StreamEventPayload};
        let mut offline = StreamEventPayload::StreamOffline {
            metadata: StreamEventMetadata::new(7, "twitch"),
        };
        stamp_receipt_time(&mut offline);
        assert!(offline.metadata().occurred_at.is_some());

        let mut identified = StreamEventMetadata::new(7, "twitch");
        identified.event_id = Some("provider-1".into());
        let mut follow = StreamEventPayload::Follow {
            metadata: identified,
            user_name: "viewer".into(),
            user_display_name: "Viewer".into(),
        };
        stamp_receipt_time(&mut follow);
        assert!(follow.metadata().occurred_at.is_none());
    }
}

#[cfg(test)]
mod command_admission_tests {
    use super::*;
    use crate::db::{models::NewHttpRequest, schema::http_requests, Database};
    use crate::services::execution::{ActionRuntimeReadiness, UniversalExecutionEngine};
    use diesel::{
        connection::SimpleConnection,
        prelude::*,
        r2d2::{ConnectionManager, Pool},
    };
    use diesel_migrations::MigrationHarness;
    use std::sync::atomic::AtomicUsize;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    struct Fixture {
        _directory: tempfile::TempDir,
        pool: Arc<Pool<ConnectionManager<SqliteConnection>>>,
        database: Arc<Database>,
        messages: MessageStore,
        sessions: SessionManager,
        service: Arc<ActionInvocationService>,
        requests: Arc<AtomicUsize>,
        bodies: Arc<tokio::sync::Mutex<Vec<serde_json::Value>>>,
        server: tokio::task::JoinHandle<()>,
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            self.server.abort();
        }
    }
    async fn fixture() -> Fixture {
        let directory = tempfile::tempdir().unwrap();
        let manager = ConnectionManager::<SqliteConnection>::new(
            directory.path().join("chat.db").to_string_lossy(),
        );
        let pool = Arc::new(Pool::builder().max_size(2).build(manager).unwrap());
        {
            let mut first = pool.get().unwrap();
            first
                .batch_execute("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;")
                .unwrap();
            first
                .run_pending_migrations(crate::db::db::MIGRATIONS)
                .unwrap();
            let mut second = pool.get().unwrap();
            second.batch_execute("PRAGMA busy_timeout=5000;").unwrap();
        }
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/command", listener.local_addr().unwrap());
        let requests = Arc::new(AtomicUsize::new(0));
        let observed = requests.clone();
        let bodies = Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let captured = bodies.clone();
        let server = tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                let mut buffer = [0; 4096];
                let mut request = Vec::new();
                loop {
                    let count = socket.read(&mut buffer).await.unwrap_or(0);
                    if count == 0 {
                        break;
                    }
                    request.extend_from_slice(&buffer[..count]);
                    assert!(request.len() <= 131_072);
                    if let Some(start) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
                        let headers = String::from_utf8_lossy(&request[..start]);
                        let length = headers
                            .lines()
                            .filter_map(|line| line.split_once(':'))
                            .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                            .map(|(_, length)| length.trim().parse::<usize>().unwrap())
                            .unwrap_or(0);
                        if request.len() >= start + 4 + length {
                            if length > 0 {
                                captured.lock().await.push(
                                    serde_json::from_slice(&request[start + 4..start + 4 + length])
                                        .unwrap(),
                                );
                            }
                            observed.fetch_add(1, Ordering::SeqCst);
                            let _ = socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}").await;
                            break;
                        }
                    }
                }
            }
        });
        diesel::insert_into(http_requests::table)
            .values(NewHttpRequest {
                name: "Fixture",
                method: "POST",
                url: &url,
                headers: None,
                body: Some(r#"{"text":"literal","duration":0}"#),
                accepted_status_codes: None,
                timeout_ms: Some(2000),
                created_at: &chrono::Utc::now().to_rfc3339(),
            })
            .execute(&mut pool.get().unwrap())
            .unwrap();
        let database = Arc::new(Database::new(&pool));
        let sessions = SessionManager::with_db(pool.clone());
        let session = serde_json::from_value(serde_json::json!({
            "id":"session-1", "name":"Fixture", "platforms":[{"platform":"twitch","accountId":"12345"}],
            "commands":[{"id":"command-1","permissionVersion":1,"trigger":"!clip","action":"{\"HttpRequestId\":1}",
                "enabled":true,"allowedRoles":{"twitch":["moderator"]}}],
            "settings":{"autoStart":false,"notifications":false,"autoResume":false},
            "status":"draft","createdAt":"", "updatedAt":"", "runs":[], "totalRuns":0
        })).unwrap();
        sessions.create_session(session).await.unwrap();
        sessions.start_session("session-1").await.unwrap();
        let engine = Arc::new(
            UniversalExecutionEngine::new(
                Arc::new(crate::core::state_manager::StateManager::new()),
            )
            .with_database(database.clone()),
        );
        let readiness = Arc::new(ActionRuntimeReadiness::new());
        readiness.mark_ready(vec![]);
        let service = Arc::new(ActionInvocationService::new(
            engine,
            database.clone(),
            readiness,
        ));
        Fixture {
            _directory: directory,
            pool,
            database: database.clone(),
            messages: MessageStore::new(database),
            sessions,
            service,
            requests,
            bodies,
            server,
        }
    }
    fn message(id: &str, text: &str, moderator: bool) -> String {
        serde_json::json!({"type":"chat_message","platform":"twitch","account_id":12345,
            "data":{"id":id,"platform":"twitch","user_id":"viewer-1","user_name":"Alice","user_display_name":"Alice",
                "message":text,"timestamp":chrono::Utc::now().to_rfc3339(),"badges":[],"is_moderator":moderator,
                "is_subscriber":false,"is_vip":false,"platform_color":""}}).to_string()
    }
    async fn handle(fixture: &Fixture, sessions: &SessionManager, message: &str) {
        StreamingWebSocket::handle_message(
            message,
            &fixture.messages,
            sessions,
            &None,
            &Some(fixture.service.clone()),
            &Arc::new(RwLock::new(HashMap::new())),
            &None,
            &None,
        )
        .await
        .unwrap();
    }
    async fn wait_terminal(fixture: &Fixture) {
        tokio::time::timeout(std::time::Duration::from_secs(3), async {
            loop {
                if fixture
                    .database
                    .list_action_invocations(10, None, None, None, None)
                    .await
                    .unwrap()
                    .iter()
                    .any(|record| record.completed_at.is_some())
                {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
    }
    #[tokio::test]
    async fn chat_handler_denies_viewers_and_prefix_matches_before_effects() {
        let fixture = fixture().await;
        handle(
            &fixture,
            &fixture.sessions,
            &message("denied", "!clip hello", false),
        )
        .await;
        handle(
            &fixture,
            &fixture.sessions,
            &message("prefix", "!clipjunk", true),
        )
        .await;
        assert!(fixture
            .database
            .list_action_invocations(10, None, None, None, None)
            .await
            .unwrap()
            .is_empty());
        assert_eq!(fixture.requests.load(Ordering::SeqCst), 0);
    }
    #[tokio::test]
    async fn chat_handler_accepts_one_effect_for_concurrent_and_restart_redelivery() {
        let fixture = fixture().await;
        let event = message("delivery-1", "!clip hello", true);
        tokio::join!(
            handle(&fixture, &fixture.sessions, &event),
            handle(&fixture, &fixture.sessions, &event)
        );
        wait_terminal(&fixture).await;
        let reopened_pool = Arc::new(
            Pool::builder()
                .max_size(2)
                .build(ConnectionManager::<SqliteConnection>::new(
                    fixture._directory.path().join("chat.db").to_string_lossy(),
                ))
                .unwrap(),
        );
        reopened_pool
            .get()
            .unwrap()
            .batch_execute("PRAGMA busy_timeout=5000;")
            .unwrap();
        let reopened_database = Arc::new(Database::new(&reopened_pool));
        let reconstructed = SessionManager::with_db(reopened_pool);
        reconstructed.load_from_db().await.unwrap();
        let readiness = Arc::new(ActionRuntimeReadiness::new());
        readiness.mark_ready(vec![]);
        let engine = Arc::new(
            UniversalExecutionEngine::new(
                Arc::new(crate::core::state_manager::StateManager::new()),
            )
            .with_database(reopened_database.clone()),
        );
        let reopened_service = Arc::new(ActionInvocationService::new(
            engine,
            reopened_database.clone(),
            readiness,
        ));
        StreamingWebSocket::handle_message(
            &event,
            &MessageStore::new(reopened_database),
            &reconstructed,
            &None,
            &Some(reopened_service),
            &Arc::new(RwLock::new(HashMap::new())),
            &None,
            &None,
        )
        .await
        .unwrap();
        assert_eq!(
            fixture
                .database
                .list_action_invocations(10, None, None, None, None)
                .await
                .unwrap()
                .len(),
            1
        );
        assert_eq!(fixture.requests.load(Ordering::SeqCst), 1);
    }
    #[tokio::test]
    async fn chat_handler_cannot_dispatch_when_admission_persistence_fails() {
        let fixture = fixture().await;
        fixture
            .pool
            .get()
            .unwrap()
            .batch_execute("DROP TABLE action_invocations")
            .unwrap();
        handle(
            &fixture,
            &fixture.sessions,
            &message("delivery-1", "!clip hello", true),
        )
        .await;
        assert_eq!(fixture.requests.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn chat_and_native_invocations_prepare_the_same_saved_http_action() {
        use crate::services::execution::{ActionReference, ActionReferenceKind};
        let fixture = fixture().await;
        let reference = ActionReference {
            kind: ActionReferenceKind::HttpRequest,
            id: "1".into(),
        };
        let contract = serde_json::from_value(serde_json::json!({"version":1,"inputSchema":{
            "type":"object","properties":{"text":{"type":"string","maxLength":100},"duration":{"type":"integer","minimum":1,"maximum":30,"default":10}},
            "required":["text"]},"bindings":[{"input":"text","target":{"type":"httpBody","pointer":"/text"}},
                {"input":"duration","target":{"type":"httpBody","pointer":"/duration"}}]})).unwrap();
        let saved = fixture
            .service
            .save_input_contract(&reference, 0, Some(contract))
            .await
            .unwrap();
        assert_eq!(saved.revision, 1);
        assert!(fixture
            .service
            .save_input_contract(&reference, 0, saved.contract.clone())
            .await
            .unwrap_err()
            .contains("Stale"));
        let mut commands = fixture
            .sessions
            .get_session("session-1")
            .await
            .unwrap()
            .commands
            .clone();
        commands[0].input_bindings.insert(
            "text".into(),
            super::super::models::CommandInputSource::Args,
        );
        fixture
            .sessions
            .update_session(
                "session-1",
                HashMap::from([("commands".into(), serde_json::to_value(commands).unwrap())]),
            )
            .await
            .unwrap();
        let text = "quotes \" and 雪\nnext";
        handle(
            &fixture,
            &fixture.sessions,
            &message("typed-chat", &format!("!clip {text}"), true),
        )
        .await;
        wait_terminal(&fixture).await;
        let context = ExecutionContext {
            trigger_source: TriggerSource::Manual {
                user_id: "desktop".into(),
            },
            user_id: None,
            session_id: "fixture".into(),
            variables: HashMap::new(),
            timeout_ms: Some(2000),
            retry_config: RetryConfig {
                max_retries: 0,
                retry_delay_ms: 0,
                exponential_backoff: false,
            },
            execution_mode: ButtonExecutionMode::RunOnce,
            execution_config: Some(serde_json::json!({"inputs":{"text":text,"duration":20}})),
        };
        let result = fixture
            .service
            .invoke_reference(&reference, None, None, &context)
            .await
            .unwrap();
        assert!(result.succeeded());
        let bodies = fixture.bodies.lock().await;
        assert_eq!(bodies[0], serde_json::json!({"text":text,"duration":10}));
        assert_eq!(bodies[1], serde_json::json!({"text":text,"duration":20}));
        drop(bodies);
        assert_eq!(
            fixture
                .service
                .get_input_contract(&reference)
                .await
                .unwrap()
                .revision,
            1
        );
        let mut invalid = context.clone();
        invalid.execution_config =
            Some(serde_json::json!({"inputs":{"text":"hello","duration":"20"}}));
        assert!(fixture
            .service
            .invoke_reference(&reference, None, None, &invalid)
            .await
            .is_err());
        invalid.execution_config =
            Some(serde_json::json!({"inputs":{"text":"hello","host":"http://other"}}));
        assert!(fixture
            .service
            .invoke_reference(&reference, None, None, &invalid)
            .await
            .is_err());
        assert_eq!(fixture.requests.load(Ordering::SeqCst), 2);
        assert_eq!(
            fixture
                .database
                .get_http_request_by_id(1)
                .await
                .unwrap()
                .body
                .as_deref(),
            Some(r#"{"text":"literal","duration":0}"#)
        );
    }

    #[tokio::test]
    async fn invalid_template_edit_rolls_back_and_concurrent_contract_updates_use_one_revision() {
        use crate::db::schema::http_requests;
        use crate::services::execution::{ActionReference, ActionReferenceKind};
        use diesel::prelude::*;
        let fixture = fixture().await;
        let reference = ActionReference {
            kind: ActionReferenceKind::HttpRequest,
            id: "1".into(),
        };
        let contract = serde_json::from_value(serde_json::json!({"version":1,"inputSchema":{"type":"object","properties":{"text":{"type":"string"}}},
            "bindings":[{"input":"text","target":{"type":"httpBody","pointer":"/text"}}]})).unwrap();
        let declaration = fixture
            .service
            .save_input_contract(&reference, 0, Some(contract))
            .await
            .unwrap();
        let original = fixture.database.get_http_request_by_id(1).await.unwrap();
        let original_body = original.body.clone();
        let mut invalid = original;
        invalid.body = Some("{}".into());
        let template =
            crate::services::execution::action_converter::saved_http_request_template(&invalid)
                .unwrap();
        let mut connection = fixture.pool.get().unwrap();
        let changed = connection.immediate_transaction::<(), anyhow::Error, _>(|connection| {
            diesel::update(http_requests::table.find(1))
                .set(http_requests::body.eq(Some("{}")))
                .execute(connection)?;
            crate::db::database::validate_saved_input_contract(connection, &reference, &template)
                .map_err(anyhow::Error::msg)?;
            Ok(())
        });
        assert!(changed.is_err());
        drop(connection);
        assert_eq!(
            fixture
                .database
                .get_http_request_by_id(1)
                .await
                .unwrap()
                .body,
            original_body
        );
        let (first, second) = tokio::join!(
            fixture
                .service
                .save_input_contract(&reference, 1, declaration.contract.clone()),
            fixture
                .service
                .save_input_contract(&reference, 1, declaration.contract.clone()),
        );
        assert_ne!(first.is_ok(), second.is_ok());
        assert_eq!(
            fixture
                .service
                .get_input_contract(&reference)
                .await
                .unwrap()
                .revision,
            2
        );
    }
}
