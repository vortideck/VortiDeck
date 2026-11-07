use crate::commands::relay_commands::RelayClientState;
use std::sync::Arc;
/// Broadcast Helper
/// Centralized function to broadcast messages to BOTH direct WebSocket peers AND relay mobile clients
use tauri::{AppHandle, Manager};
use vortideck_core::core::StateManager;

/// Broadcast message to ALL connected clients (direct + relay)
/// This is the unified broadcast function that should be used instead of calling
/// `state_manager.broadcast_message()` directly
pub async fn broadcast_to_all_clients(
    app_handle: &AppHandle,
    state_manager: &Arc<StateManager>,
    message: serde_json::Value,
) {
    // 1. Broadcast to direct WebSocket peers
    state_manager.broadcast_message(message.clone()).await;

    // 2. ALSO broadcast to relay mobile clients (if relay is connected)
    if let Some(relay_client_state) = app_handle.try_state::<RelayClientState>() {
        let relay_client_guard = relay_client_state.read().await;
        if let Some(relay_client) = relay_client_guard.as_ref() {
            match relay_client
                .broadcast_to_relay_mobiles(&message, state_manager)
                .await
            {
                Ok(count) if count > 0 => {
                    println!("📡 Broadcasted to {} relay mobile client(s)", count);
                }
                Ok(_) => {
                    // No relay clients connected - silent
                }
                Err(e) => {
                    eprintln!("⚠️ Failed to broadcast to relay clients: {}", e);
                }
            }
        }
    }
}
