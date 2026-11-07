use crate::relay_client::RelayClient; // Works in both lib and bin
use serde_json::json;
use std::sync::Arc;
/// Tauri commands for managing backend relay connection
use tauri::{AppHandle, Manager, State};
use tokio::sync::RwLock;
use uuid::Uuid;
use vortideck_core::core::StateManager;

// Global relay client state
pub type RelayClientState = Arc<RwLock<Option<Arc<RelayClient>>>>;

#[tauri::command]
pub async fn enable_relay(
    app_handle: AppHandle,
    state_manager: State<'_, Arc<StateManager>>,
    relay_client_state: State<'_, RelayClientState>,
    db_pool: State<'_, vortideck_core::db::db::DbPool>,
) -> Result<String, String> {
    println!("📡 Enabling relay mode");

    // Check if relay is already enabled
    if let Some(existing_client) = relay_client_state.read().await.as_ref() {
        if existing_client.is_connected().await {
            return Ok(existing_client.get_desktop_id().to_string());
        }
    }

    // Generate or retrieve persistent desktop ID
    let desktop_id = match state_manager.get_desktop_id().await {
        Some(id) => {
            println!("📱 Using existing desktop_id: {}", id);
            id
        }
        None => {
            let new_id = Uuid::new_v4().to_string();
            println!("📱 Generated new desktop_id: {}", new_id);
            state_manager.set_desktop_id(new_id.clone()).await;
            // TODO: Persist to database for permanent storage
            new_id
        }
    };

    let ws_url = std::env::var("WS_URL").unwrap_or_else(|_| "wss://ws.vortideck.com".to_string());
    // Get API URL and construct relay base URL
    let api_url =
        std::env::var("API_URL").unwrap_or_else(|_| "https://backend.vortideck.com".to_string());
    // Use ws_url as relay_base_url (append relay path if not included)
    let relay_base_url = ws_url.trim_end_matches('/').to_string() + "/ws/relay/desktop/";

    // Get access token
    use vortideck_core::services::auth::token_storage::TokenStorage;
    let access_token = match TokenStorage::get_access_token() {
        Ok(token) => token,
        Err(e) => {
            return Err(format!("No access token available: {}", e));
        }
    };

    // Create relay client with internal retry loop
    let relay_client =
        match RelayClient::new(desktop_id.clone(), relay_base_url, api_url, access_token) {
            Ok(client) => Arc::new(client),
            Err(e) => {
                return Err(format!("Failed to create relay client: {}", e));
            }
        };

    // Get PlatformContext to access EventBus
    let platform_context =
        match app_handle.try_state::<Arc<vortideck_core::core::PlatformContext>>() {
            Some(ctx) => ctx.inner().clone(),
            None => {
                return Err("PlatformContext not available".to_string());
            }
        };

    // Connect to backend using EventBus (starts internal retry loop)
    let action_runtime_readiness = app_handle
        .try_state::<Arc<vortideck_core::services::execution::ActionRuntimeReadiness>>()
        .map(|state| state.inner().clone())
        .ok_or_else(|| "Action runtime readiness state not available".to_string())?;
    relay_client
        .connect(
            state_manager.inner().clone(),
            platform_context.event_bus.clone(),
            action_runtime_readiness,
            Arc::new(vortideck_core::db::Database::new(db_pool.inner())),
        )
        .await
        .map_err(|e| format!("Failed to connect to relay: {}", e))?;

    // Store relay client in state
    *relay_client_state.write().await = Some(relay_client);
    state_manager.set_relay_enabled(true).await;

    println!(
        "✅ Relay enabled successfully with desktop_id: {}",
        desktop_id
    );

    Ok(desktop_id)
}

#[tauri::command]
pub async fn disable_relay(
    state_manager: State<'_, Arc<StateManager>>,
    relay_client_state: State<'_, RelayClientState>,
) -> Result<(), String> {
    println!("📡 Disabling relay mode");

    // Disconnect relay client
    *relay_client_state.write().await = None;
    state_manager.set_relay_enabled(false).await;
    state_manager.set_relay_url(String::new()).await;

    println!("✅ Relay disabled successfully");

    Ok(())
}

#[tauri::command]
pub async fn get_relay_status(
    state_manager: State<'_, Arc<StateManager>>,
    relay_client_state: State<'_, RelayClientState>,
) -> Result<serde_json::Value, String> {
    Ok(relay_status_snapshot(state_manager.inner(), relay_client_state.inner()).await)
}

pub async fn relay_status_snapshot(
    state_manager: &Arc<StateManager>,
    relay_client_state: &RelayClientState,
) -> serde_json::Value {
    let enabled = state_manager.is_relay_enabled().await;
    let url = state_manager.get_relay_url().await;
    let desktop_id = state_manager.get_desktop_id().await;

    let connected = if let Some(client) = relay_client_state.read().await.as_ref() {
        client.is_connected().await
    } else {
        false
    };

    json!({
        "enabled": enabled,
        "connected": connected,
        "relay_url": url,
        "desktop_id": desktop_id,
    })
}

#[tauri::command]
pub async fn get_or_create_desktop_id(
    state_manager: State<'_, Arc<StateManager>>,
) -> Result<String, String> {
    match state_manager.get_desktop_id().await {
        Some(id) => Ok(id),
        None => {
            let new_id = Uuid::new_v4().to_string();
            state_manager.set_desktop_id(new_id.clone()).await;
            // TODO: Persist to database
            Ok(new_id)
        }
    }
}
