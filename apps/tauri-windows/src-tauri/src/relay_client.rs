use crate::e2ee_handler;
use crate::relay_constants::{incoming, metadata, outgoing};
use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use serde_json::{json, Value};
use serde_json_canonicalizer::to_string as to_canonical_string;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
/// Backend Relay WebSocket Client
/// Connects desktop TO backend relay server to enable internet-based mobile connections
///
/// ✅ PLATFORM AGNOSTIC: Uses EventBus instead of Tauri-specific APIs
/// This allows relay_client to work in any Rust application (CLI, server, etc.)
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use uuid::Uuid;
use vortideck_core::core::StateManager;
use vortideck_core::db::Database;
use vortideck_core::events::types::RelayEventPayload;
use vortideck_core::events::{EventBus, EventPayload};
use vortideck_core::services::auth::oauth::OAuthService;
use vortideck_core::services::execution::ActionRuntimeReadiness;

/// Type alias for relay WebSocket writer
type RelayWriter =
    Arc<RwLock<SplitSink<WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>, Message>>>;

pub struct RelayClient {
    desktop_id: String,
    relay_base_url: String, // Base URL without token
    api_url: String,        // For token refresh
    access_token: Arc<RwLock<String>>,
    connected: Arc<RwLock<bool>>,
    /// Connected mobile client IDs (tracks who's connected via relay)
    mobile_clients: Arc<RwLock<HashSet<String>>>,
    /// WebSocket writer for sending messages to relay server
    writer: Arc<RwLock<Option<RelayWriter>>>,
    /// Last close code received (for token refresh logic)
    last_close_code: Arc<RwLock<Option<u16>>>,
    /// Handle to the retry loop task (for aborting on disconnect)
    retry_task_handle: Arc<RwLock<Option<JoinHandle<()>>>>,
}

impl RelayClient {
    /// Create a new RelayClient with validated desktop_id
    /// 🔐 SECURITY FIX: Validates desktop_id is a valid UUID format
    pub fn new(
        desktop_id: String,
        relay_base_url: String,
        api_url: String,
        access_token: String,
    ) -> Result<Self, String> {
        // Validate desktop_id is a valid UUID format
        Uuid::parse_str(&desktop_id)
            .map_err(|e| format!("Invalid desktop_id format (must be UUID): {}", e))?;

        // Validate relay_base_url is not empty
        if relay_base_url.trim().is_empty() {
            return Err("Relay base URL cannot be empty".to_string());
        }

        // Validate api_url is not empty
        if api_url.trim().is_empty() {
            return Err("API URL cannot be empty".to_string());
        }

        println!(
            "✅ RelayClient created with validated desktop_id: {}",
            desktop_id
        );

        Ok(Self {
            desktop_id,
            relay_base_url,
            api_url,
            access_token: Arc::new(RwLock::new(access_token)),
            connected: Arc::new(RwLock::new(false)),
            mobile_clients: Arc::new(RwLock::new(HashSet::new())),
            writer: Arc::new(RwLock::new(None)),
            last_close_code: Arc::new(RwLock::new(None)),
            retry_task_handle: Arc::new(RwLock::new(None)),
        })
    }

    /// Connect to backend relay server with automatic retry loop
    ///
    /// Spawns a background task that continuously maintains the connection
    pub async fn connect(
        &self,
        state_manager: Arc<StateManager>,
        event_bus: Arc<EventBus>,
        action_runtime_readiness: Arc<ActionRuntimeReadiness>,
        remote_command_database: Arc<Database>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("🔌 Starting relay client with internal retry loop");

        // Clone all the things we need for the retry loop
        let relay_base_url = self.relay_base_url.clone();
        let api_url = self.api_url.clone();
        let access_token = self.access_token.clone();
        let desktop_id = self.desktop_id.clone();
        let connected = self.connected.clone();
        let mobile_clients = self.mobile_clients.clone();
        let writer = self.writer.clone();
        let last_close_code = self.last_close_code.clone();
        let action_runtime_readiness = action_runtime_readiness.clone();
        let remote_command_database = remote_command_database.clone();
        let http_client = reqwest::Client::new();

        // Spawn internal retry loop
        let retry_handle = tokio::spawn(async move {
            let mut retry_delay_secs = 2;
            let max_delay_secs = 60;
            let mut retry_count = 0;
            let mut last_error_was_auth = false;

            loop {
                retry_count += 1;
                println!("🔄 Relay connection attempt #{}", retry_count);

                // Get current access token
                let current_token = access_token.read().await.clone();

                // If last disconnection was due to auth error, try to refresh token first
                if last_error_was_auth {
                    println!(
                        "🔐 Previous disconnection was auth error, attempting token refresh..."
                    );
                    match OAuthService::refresh_access_token_with_reqwest(&api_url).await {
                        Ok(new_token) => {
                            println!("✅ Access token refreshed successfully for relay");
                            *access_token.write().await = new_token;
                            retry_delay_secs = 2; // Reset backoff
                            retry_count = 0;
                            last_error_was_auth = false;
                            continue; // Retry immediately with new token
                        }
                        Err(refresh_err) => {
                            let error_str = refresh_err.to_string();
                            println!("❌ Failed to refresh token: {}", refresh_err);

                            // Check if refresh token is missing
                            if error_str.contains("Token not found")
                                || error_str.contains("refresh_token")
                            {
                                eprintln!("🔐 Refresh token missing - user session expired");
                                eprintln!("⏹️ Stopping relay reconnection loop");
                                return; // Exit task
                            }

                            last_error_was_auth = false;
                        }
                    }
                }

                // Exchange the account JWT over HTTPS for a short-lived, one-use
                // WebSocket ticket. Long-lived JWTs must not appear in URLs,
                // proxy access logs, browser history, or diagnostic reports.
                let relay_ticket =
                    match request_websocket_ticket(&http_client, &api_url, &current_token).await {
                        Ok(ticket) => ticket,
                        Err(error) => {
                            eprintln!("⚠️ Failed to obtain relay WebSocket ticket: {}", error);
                            last_error_was_auth = error.is_authentication_error();
                            *connected.write().await = false;
                            tokio::time::sleep(tokio::time::Duration::from_secs(retry_delay_secs))
                                .await;
                            retry_delay_secs = (retry_delay_secs * 2).min(max_delay_secs);
                            continue;
                        }
                    };
                let relay_url = format!("{}?ticket={}", relay_base_url, relay_ticket);

                // Attempt WebSocket connection
                match connect_async(&relay_url).await {
                    Ok((ws_stream, _)) => {
                        println!("✅ Relay WebSocket connected");
                        let connection_session_id = Uuid::new_v4().to_string();

                        let (write, mut read) = ws_stream.split();

                        // Store writer
                        let ws_writer = Arc::new(RwLock::new(write));
                        *writer.write().await = Some(ws_writer.clone());
                        *connected.write().await = true;

                        // Connection candidates are reachability hints. The
                        // paired E2EE handshake still authenticates the peer
                        // after a mobile chooses one.
                        let connection_candidates = state_manager
                            .get_server_info()
                            .map(|info| info.connection_candidates)
                            .unwrap_or_default();

                        // Send register_desktop message
                        let register_msg = json!({
                            "type": outgoing::REGISTER_DESKTOP,
                            metadata::DESKTOP_ID: desktop_id,
                            metadata::DESKTOP_NAME: get_computer_name(),
                            metadata::DESKTOP_METADATA: {
                                metadata::OS: std::env::consts::OS,
                                metadata::HOSTNAME: get_hostname(),
                                metadata::VERSION: env!("CARGO_PKG_VERSION"),
                                metadata::IP: get_local_ip(),
                                "connection_candidates": connection_candidates
                            }
                        });

                        if let Err(e) = ws_writer
                            .write()
                            .await
                            .send(Message::Text(register_msg.to_string().into()))
                            .await
                        {
                            eprintln!("❌ Failed to send register_desktop: {}", e);
                            *connected.write().await = false;
                            tokio::time::sleep(tokio::time::Duration::from_secs(retry_delay_secs))
                                .await;
                            retry_delay_secs = (retry_delay_secs * 2).min(max_delay_secs);
                            continue;
                        }

                        println!("📡 Sent register_desktop message");

                        // Emit connecting status
                        let _ =
                            event_bus.emit(EventPayload::Relay(RelayEventPayload::StatusChanged {
                                status: "connecting".to_string(),
                                desktop_id: Some(desktop_id.clone()),
                                relay_session_id: None,
                                error: None,
                            }));

                        // Reset retry counters on successful connection
                        retry_delay_secs = 2;
                        retry_count = 0;

                        // Message handling loop
                        while let Some(msg) = read.next().await {
                            match msg {
                                Ok(Message::Text(text)) => {
                                    println!("📥 Relay message received");
                                    if let Err(e) = handle_relay_message(
                                        &text,
                                        &state_manager,
                                        &event_bus,
                                        &ws_writer,
                                        &mobile_clients,
                                        &action_runtime_readiness,
                                        &remote_command_database,
                                        &connection_session_id,
                                    )
                                    .await
                                    {
                                        eprintln!("❌ Error handling relay message: {}", e);
                                    }
                                }
                                Ok(Message::Close(frame)) => {
                                    if let Some(close_frame) = frame {
                                        let code: u16 = close_frame.code.into();
                                        println!(
                                            "🔌 Relay closed: code={}, reason={}",
                                            code, close_frame.reason
                                        );

                                        // Store close code
                                        *last_close_code.write().await = Some(code);

                                        // Check for auth errors
                                        if code == 4001 || code == 4003 {
                                            println!("🔐 Auth error detected, will refresh token");
                                            last_error_was_auth = true;
                                            last_close_code.write().await.take();
                                        }

                                        let error_msg = if code == 4001 || code == 4003 {
                                            Some(format!("Authentication error (code {})", code))
                                        } else {
                                            Some(close_frame.reason.to_string())
                                        };

                                        *connected.write().await = false;

                                        let _ = event_bus.emit(EventPayload::Relay(
                                            RelayEventPayload::StatusChanged {
                                                status: "disconnected".to_string(),
                                                desktop_id: Some(desktop_id.clone()),
                                                relay_session_id: None,
                                                error: error_msg,
                                            },
                                        ));
                                    } else {
                                        println!("🔌 Relay connection closed");
                                        *connected.write().await = false;

                                        let _ = event_bus.emit(EventPayload::Relay(
                                            RelayEventPayload::StatusChanged {
                                                status: "disconnected".to_string(),
                                                desktop_id: Some(desktop_id.clone()),
                                                relay_session_id: None,
                                                error: None,
                                            },
                                        ));
                                    }
                                    break; // Exit message loop, will retry
                                }
                                Ok(Message::Ping(_)) => {
                                    // Auto-handled by tungstenite
                                }
                                Err(e) => {
                                    eprintln!("❌ Relay WebSocket error: {}", e);
                                    *connected.write().await = false;

                                    let _ = event_bus.emit(EventPayload::Relay(
                                        RelayEventPayload::StatusChanged {
                                            status: "error".to_string(),
                                            desktop_id: Some(desktop_id.clone()),
                                            relay_session_id: None,
                                            error: Some(e.to_string()),
                                        },
                                    ));

                                    break; // Exit message loop, will retry
                                }
                                _ => {}
                            }
                        }

                        println!("🔄 Relay connection ended, will reconnect...");
                    }
                    Err(e) => {
                        let error_str = e.to_string();
                        println!("⚠️ Failed to connect to relay: {}", e);

                        // Check if error suggests auth issue
                        last_error_was_auth = error_str.contains("Authentication error")
                            || error_str.contains("4001")
                            || error_str.contains("4003");

                        *connected.write().await = false;
                    }
                }

                // Exponential backoff before retry
                println!("   Retrying in {} seconds...", retry_delay_secs);
                tokio::time::sleep(tokio::time::Duration::from_secs(retry_delay_secs)).await;
                retry_delay_secs = (retry_delay_secs * 2).min(max_delay_secs);
            }
        });

        // Store the retry task handle
        *self.retry_task_handle.write().await = Some(retry_handle);

        println!("✅ Relay client retry loop started");
        Ok(())
    }

    pub async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }

    /// Disconnect and abort the internal retry loop
    pub async fn disconnect(&self) {
        println!("🔌 RelayClient disconnecting...");

        // Abort the retry task if it's running
        if let Some(handle) = self.retry_task_handle.write().await.take() {
            if !handle.is_finished() {
                println!("⚠️ Aborting relay retry loop task");
                handle.abort();
            }
        }

        // Mark as disconnected and clean up
        *self.connected.write().await = false;
        *self.writer.write().await = None;
        println!("✅ RelayClient disconnected");
    }

    pub fn get_desktop_id(&self) -> &str {
        &self.desktop_id
    }

    /// Send message to ALL connected relay mobile clients
    /// Messages are E2EE signed with HMAC (signature + nonce)
    pub async fn broadcast_to_relay_mobiles(
        &self,
        message: &Value,
        state_manager: &Arc<StateManager>,
    ) -> Result<usize, String> {
        if !self.is_connected().await {
            return Err("Relay not connected".to_string());
        }

        let writer_guard = self.writer.read().await;
        let writer = match writer_guard.as_ref() {
            Some(w) => w,
            None => return Err("Relay writer not available".to_string()),
        };

        let mobile_clients = self.mobile_clients.read().await;
        let client_count = mobile_clients.len();

        if client_count == 0 {
            return Ok(0); // No clients connected
        }

        println!("📡 Broadcasting to {} relay mobile client(s)", client_count);

        // Send to each mobile client with E2EE signing
        let mut sent_count = 0;
        for client_id in mobile_clients.iter() {
            match send_to_relay_mobile(writer, client_id, message, state_manager).await {
                Ok(_) => {
                    sent_count += 1;
                }
                Err(e) => {
                    eprintln!("❌ Failed to send to relay mobile {}: {}", client_id, e);
                }
            }
        }

        println!(
            "✅ Sent to {}/{} relay mobile clients",
            sent_count, client_count
        );
        Ok(sent_count)
    }

    /// Send a private response to one authenticated relay mobile. Command
    /// acknowledgements and snapshots must not be broadcast to unrelated
    /// paired devices.
    pub async fn send_to_relay_mobile(
        &self,
        client_id: &str,
        message: &Value,
        state_manager: &Arc<StateManager>,
    ) -> Result<bool, String> {
        if !self.is_connected().await {
            return Err("Relay not connected".to_string());
        }
        if !self.mobile_clients.read().await.contains(client_id) {
            return Ok(false);
        }
        let writer_guard = self.writer.read().await;
        let writer = writer_guard.as_ref().ok_or("Relay writer not available")?;
        send_to_relay_mobile(writer, client_id, message, state_manager).await?;
        Ok(true)
    }

    /// Get count of connected relay mobile clients
    pub async fn get_relay_mobile_count(&self) -> usize {
        self.mobile_clients.read().await.len()
    }

    /// Check if any relay mobile clients are connected
    pub async fn has_relay_mobiles(&self) -> bool {
        !self.mobile_clients.read().await.is_empty()
    }
}

async fn accept_relay_mobile_command(
    message: &Value,
    state_manager: &Arc<StateManager>,
    event_bus: &Arc<EventBus>,
    writer: &RelayWriter,
    mobile_clients: &Arc<RwLock<HashSet<String>>>,
    remote_command_database: &Database,
    connection_session_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match e2ee_handler::verify_relay_message(message, state_manager, remote_command_database).await
    {
        Ok(verified) => {
            let client_id = verified.client_id;
            println!("✅ Relay message signature verified for: {}", client_id);

            if let Some(receipt) = verified.disposition.receipt() {
                let mut receipt = receipt.clone();
                receipt["duplicate"] = json!(!verified.disposition.should_dispatch());
                send_to_relay_mobile(writer, &client_id, &receipt, state_manager)
                    .await
                    .map_err(|error| format!("Unable to return remote command receipt: {error}"))?;
            }
            if !verified.disposition.should_dispatch() {
                println!("↩️ Returned prior relay command receipt without redispatch");
                return Ok(());
            }

            let mut clients = mobile_clients.write().await;
            if clients.insert(client_id.clone()) {
                println!("📊 Added mobile client to tracking: {}", client_id);
                println!("📊 Total relay mobile clients: {}", clients.len());
            }
            drop(clients);

            event_bus.emit(EventPayload::Relay(RelayEventPayload::CommandReceived {
                client_id,
                connection_session_id: connection_session_id.to_string(),
                payload: verified.payload,
            }))?;
        }
        Err(error) => {
            eprintln!("❌ Failed to verify relay message: {error}");
        }
    }
    Ok(())
}

/// Handle incoming message from backend relay.
///
/// Platform-agnostic: uses EventBus instead of AppHandle.
async fn handle_relay_message(
    text: &str,
    state_manager: &Arc<StateManager>,
    event_bus: &Arc<EventBus>,
    writer: &RelayWriter,
    mobile_clients: &Arc<RwLock<HashSet<String>>>,
    action_runtime_readiness: &Arc<ActionRuntimeReadiness>,
    remote_command_database: &Arc<Database>,
    connection_session_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let json: Value = serde_json::from_str(text)?;

    // Check both "type" (backend format) and "action" (direct mobile format)
    let message_type = json["type"]
        .as_str()
        .or_else(|| json["action"].as_str())
        .unwrap_or("");

    println!("📥 Relay message type: {}", message_type);

    match message_type {
        // Handle direct relay_message format from mobile (new E2EE format)
        "relay_message" => {
            println!("📱 Received relay_message from mobile");

            // Mobile sends: { action: "relay_message", payload: {...}, signature: "...", nonce: "..." }
            // Extract the payload
            if let Some(payload) = json.get("payload") {
                // Check if signature and nonce are provided (E2EE active)
                let has_signature = json
                    .get("signature")
                    .and_then(|s| s.as_str())
                    .map_or(false, |s| !s.is_empty());
                println!("🔐 Has signature: {}", has_signature);

                if has_signature {
                    accept_relay_mobile_command(
                        &json,
                        state_manager,
                        event_bus,
                        writer,
                        mobile_clients,
                        remote_command_database,
                        connection_session_id,
                    )
                    .await?;
                } else {
                    // No signature - E2EE not active
                    // 🔒 SECURITY: E2EE handshakes are ONLY allowed via direct connection (QR code pairing)
                    // Relay messages MUST be signed - if no signature, reject the message
                    if let Some(action) = payload.get("action").and_then(|a| a.as_str()) {
                        println!("⚠️ Unsigned relay_message rejected - action: {}", action);
                        println!(
                            "🔒 E2EE handshakes must be done via direct connection (QR code pairing)"
                        );
                        println!("🔒 Relay requires prior pairing with saved shared secret");

                        if action == "e2ee_handshake" {
                            println!("❌ E2EE handshake rejected: Not allowed via relay");
                            println!(
                                "🔒 E2EE keys must be exchanged via QR code pairing (direct connection)"
                            );
                            println!(
                                "📱 Mobile must scan QR code while on same network as desktop"
                            );
                        } else {
                            println!(
                                "🚫 Unsigned relay command rejected without execution: {}",
                                action
                            );
                        }
                    } else {
                        println!("❌ relay_message missing action in payload");
                    }
                }
            } else {
                println!("❌ relay_message missing payload");
            }
        }

        "relay_registered" => {
            let relay_session_id = json["relay_session_id"].as_str().unwrap_or("");
            println!("✅ Desktop registered for relay");
            println!("📡 Relay Session ID: {}", relay_session_id);

            // Emit status event via EventBus
            event_bus.emit(EventPayload::Relay(RelayEventPayload::StatusChanged {
                status: "registered".to_string(),
                desktop_id: None,
                relay_session_id: Some(relay_session_id.to_string()),
                error: None,
            }))?;
        }

        "relay_from_mobile" => {
            // Mobile sent a command through relay
            println!("📱 Received message from mobile via relay");
            accept_relay_mobile_command(
                &json,
                state_manager,
                event_bus,
                writer,
                mobile_clients,
                remote_command_database,
                connection_session_id,
            )
            .await?;
        }

        "mobile_connected" => {
            // Extract mobile client_id from the message
            let client_id = json["client_id"].as_str().unwrap_or("unknown").to_string();

            // 🛡️ DEFENSIVE: Don't track "unknown" clients (backend should provide client_id)
            if client_id == "unknown" {
                println!(
                    "⚠️ Mobile connected with unknown client_id - backend should provide client_id"
                );
                println!(
                    "📊 Total relay mobile clients: {} (not tracking unknown)",
                    mobile_clients.read().await.len()
                );
                return Ok(());
            }

            println!("📱 Mobile app connected to relay: {}", client_id);

            // 🛡️ DEFENSIVE: Clean up any stale "unknown" entries when real client connects
            {
                let mut clients = mobile_clients.write().await;
                clients.remove("unknown");

                // Track this mobile client (HashSet prevents duplicates automatically)
                let was_new = clients.insert(client_id.clone());
                if was_new {
                    println!("✅ Added new relay mobile client: {}", client_id);
                } else {
                    println!(
                        "🔄 Mobile client reconnected (already tracked): {}",
                        client_id
                    );
                }
                println!("📊 Total relay mobile clients: {}", clients.len());
            }

            // Emit mobile connection event via EventBus
            event_bus.emit(EventPayload::Relay(RelayEventPayload::MobileConnected {
                client_id: client_id.clone(),
            }))?;
            send_to_relay_mobile(
                writer,
                &client_id,
                &json!({
                    "action": "action_runtime_readiness",
                    "protocolVersion": 1,
                    "readiness": action_runtime_readiness.snapshot(),
                }),
                state_manager,
            )
            .await
            .map_err(|error| {
                format!(
                    "Unable to send runtime readiness to relay mobile {}: {}",
                    client_id, error
                )
            })?;
        }

        "mobile_disconnected" => {
            // Extract mobile client_id from the message
            let client_id = json["client_id"].as_str().unwrap_or("unknown").to_string();
            println!("📱 Mobile app disconnected from relay: {}", client_id);

            // Remove this mobile client from tracking
            mobile_clients.write().await.remove(&client_id);
            println!(
                "📊 Total relay mobile clients: {}",
                mobile_clients.read().await.len()
            );

            // Emit mobile disconnection event via EventBus
            event_bus.emit(EventPayload::Relay(RelayEventPayload::MobileDisconnected {
                client_id: client_id,
            }))?;
        }

        "connection_established" => {
            // Relay server acknowledges connection
            println!("🔗 Connection established with relay server");
            // This is just an acknowledgment, relay_registered will follow with session details
        }

        "error" => {
            let error_msg = json["message"].as_str().unwrap_or("Unknown error");
            eprintln!("❌ Relay error from backend: {}", error_msg);

            // Emit error event via EventBus
            event_bus.emit(EventPayload::Relay(RelayEventPayload::StatusChanged {
                status: "error".to_string(),
                desktop_id: None,
                relay_session_id: None,
                error: Some(error_msg.to_string()),
            }))?;
        }

        "ping" => {
            // Backend keepalive ping - no action needed
            println!("🏓 Ping from relay server");
        }

        _ => {
            println!("⚠️ Unknown relay message type: {}", message_type);
        }
    }

    Ok(())
}

async fn send_to_relay_mobile(
    writer: &RelayWriter,
    client_id: &str,
    message: &Value,
    state_manager: &Arc<StateManager>,
) -> Result<(), String> {
    let shared_secret = state_manager
        .get_shared_secret(client_id)
        .await
        .ok_or_else(|| format!("No shared secret for relay mobile {}", client_id))?;
    let nonce = Uuid::new_v4().to_string();
    let message_id = Uuid::new_v4().to_string();
    let sent_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let expires_at = sent_at + 30_000;
    let unsigned_envelope = json!({
        "protocol_version": 2,
        "message_id": message_id,
        "sent_at": sent_at,
        "expires_at": expires_at,
        "payload": message,
        "nonce": nonce,
    });
    let message_str = to_canonical_string(&unsigned_envelope)
        .map_err(|error| format!("Unable to canonicalize relay payload: {}", error))?;
    let signature = crate::crypto::create_hmac_signature(&shared_secret, &message_str);
    let relay_message = json!({
        "type": outgoing::RELAY_TO_MOBILE,
        "action": "relay_message",
        "target_client_id": client_id,
        "protocol_version": 2,
        "message_id": message_id,
        "sent_at": sent_at,
        "expires_at": expires_at,
        "payload": message,
        "signature": signature,
        "nonce": nonce,
    });
    writer
        .write()
        .await
        .send(Message::Text(relay_message.to_string().into()))
        .await
        .map_err(|error| error.to_string())
}

fn get_computer_name() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "VortiDeck Desktop".to_string())
}

fn get_hostname() -> String {
    hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "unknown".to_string())
}

fn get_local_ip() -> String {
    use local_ip_address::local_ip;
    local_ip()
        .unwrap_or_else(|_| "127.0.0.1".parse().unwrap())
        .to_string()
}

#[derive(Debug)]
struct WebSocketTicketError {
    status: Option<reqwest::StatusCode>,
    message: String,
}

impl WebSocketTicketError {
    fn is_authentication_error(&self) -> bool {
        matches!(
            self.status,
            Some(reqwest::StatusCode::UNAUTHORIZED | reqwest::StatusCode::FORBIDDEN)
        )
    }
}

impl std::fmt::Display for WebSocketTicketError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

async fn request_websocket_ticket(
    client: &reqwest::Client,
    api_url: &str,
    access_token: &str,
) -> Result<String, WebSocketTicketError> {
    let endpoint = format!("{}/api/v1/auth/ws-ticket", api_url.trim_end_matches('/'));
    let response = client
        .post(endpoint)
        .header("Authorization", format!("Token {}", access_token))
        .header("X-Client-Type", "native")
        .json(&json!({ "audience": "relay" }))
        .send()
        .await
        .map_err(|error| WebSocketTicketError {
            status: None,
            message: error.to_string(),
        })?;
    let status = response.status();
    if !status.is_success() {
        return Err(WebSocketTicketError {
            status: Some(status),
            message: format!("ticket endpoint returned HTTP {}", status),
        });
    }

    let body: Value = response
        .json()
        .await
        .map_err(|error| WebSocketTicketError {
            status: Some(status),
            message: format!("invalid ticket response: {}", error),
        })?;
    body.get("ticket")
        .and_then(Value::as_str)
        .filter(|ticket| !ticket.is_empty() && ticket.len() <= 256)
        .map(str::to_owned)
        .ok_or_else(|| WebSocketTicketError {
            status: Some(status),
            message: "ticket response did not contain a valid ticket".to_string(),
        })
}
