use crate::commands::relay_commands::RelayClientState;
use crate::db::Database;
use crate::relay_client::RelayClient;
use std::sync::Arc;
/// Relay Connection Module
///
/// Handles establishing and maintaining connection to backend relay server.
/// This module is called:
/// 1. On app startup (if user already has tokens)
/// 2. After successful login (via auth_token_saved event)
use tauri::{AppHandle, Manager};
use uuid::Uuid;
use vortideck_core::core::StateManager;
use vortideck_core::db::db::DbPool;
use vortideck_core::services::auth::token_storage::TokenStorage;

/// Attempt to establish relay connection with backend
/// Returns Ok(desktop_id) if connected, Err if failed (but doesn't crash)
pub async fn attempt_relay_connection(app_handle: AppHandle) -> Result<String, String> {
    println!("🔄 Attempting relay connection...");

    let state_manager = match app_handle.try_state::<Arc<StateManager>>() {
        Some(state) => state.inner().clone(),
        None => {
            println!("❌ StateManager not available");
            return Err("StateManager not available".to_string());
        }
    };

    // 🔒 CHECK: Relay must be enabled in settings
    let relay_enabled = {
        use diesel::prelude::*;
        use vortideck_core::db::schema::app_config;

        match app_handle.try_state::<DbPool>() {
            Some(db_pool) => match db_pool.get() {
                Ok(mut conn) => {
                    match app_config::table
                        .first::<vortideck_core::db::models::AppConfig>(&mut conn)
                    {
                        Ok(config) => config.relay_enabled,
                        Err(_) => {
                            println!(
                                "⚠️ Could not load relay_enabled from database, defaulting to enabled"
                            );
                            true
                        }
                    }
                }
                Err(_) => {
                    println!("⚠️ Could not get database connection, defaulting to enabled");
                    true
                }
            },
            None => {
                println!("⚠️ Database pool not available, defaulting to enabled");
                true
            }
        }
    };

    if !relay_enabled {
        println!("🚫 Relay connection disabled in settings");
        return Err("Relay disabled in settings".to_string());
    }

    let relay_client_state = match app_handle.try_state::<RelayClientState>() {
        Some(state) => state,
        None => {
            println!("❌ RelayClientState not available");
            return Err("RelayClientState not available".to_string());
        }
    };

    // 🔒 CRITICAL: Hold lock for entire operation to prevent race conditions
    // This ensures only ONE reconnection can happen at a time
    let mut client_lock = relay_client_state.write().await;

    // Disconnect existing relay client if present (handles stale/broken connections after sleep)
    if let Some(existing_client) = client_lock.take() {
        println!("🔌 Disconnecting existing relay client before reconnect");
        existing_client.disconnect().await; // Aborts internal retry loop
    }

    let ws_url = std::env::var("WS_URL").unwrap_or_else(|_| "wss://ws.vortideck.com".to_string());
    // Get API URL
    let api_url =
        std::env::var("API_URL").unwrap_or_else(|_| "https://backend.vortideck.com".to_string());
    // Use ws_url as relay_base_url (append relay path if not included)
    let relay_base_url = ws_url.trim_end_matches('/').to_string() + "/ws/relay/desktop/";

    // Get JWT access token
    let access_token = match TokenStorage::get_access_token() {
        Ok(token) => token,
        Err(e) => {
            println!("⚠️ No JWT token available for relay: {}", e);
            println!("   Relay will connect automatically after login");
            return Err(format!("No token: {}", e));
        }
    };

    // 🔐 Load desktop_id from database (generated on first app launch in db.rs)
    let desktop_id = match state_manager.get_desktop_id().await {
        Some(id) => {
            println!("📱 Using desktop_id from memory: {}", id);
            id
        }
        None => {
            // Load from database (should always exist after initial setup)
            let db_pool = app_handle.state::<DbPool>();
            let db = Database::new(&db_pool);

            match db.load_desktop_id() {
                Some(id) => {
                    println!("📱 Loaded desktop_id from database: {}", id);
                    // Cache in memory for this session
                    state_manager.set_desktop_id(id.clone()).await;
                    id
                }
                None => {
                    // This should never happen (db.rs creates it on first launch)
                    // But handle gracefully just in case
                    let new_id = Uuid::new_v4().to_string();
                    eprintln!(
                        "⚠️ desktop_id missing from database, generating new one: {}",
                        new_id
                    );

                    state_manager.set_desktop_id(new_id.clone()).await;
                    if let Err(e) = db.save_desktop_id(&new_id) {
                        eprintln!("⚠️ Failed to persist desktop_id to database: {}", e);
                    }

                    new_id
                }
            }
        }
    };

    println!("📡 Initializing relay connection...");

    // Create RelayClient with internal retry loop
    let relay_client = match RelayClient::new(
        desktop_id.clone(),
        relay_base_url.clone(),
        api_url,
        access_token,
    ) {
        Ok(client) => Arc::new(client),
        Err(e) => {
            println!("❌ Failed to create relay client: {}", e);
            return Err(format!("Failed to create relay client: {}", e));
        }
    };

    // Get PlatformContext for EventBus
    let platform_context =
        match app_handle.try_state::<Arc<vortideck_core::core::PlatformContext>>() {
            Some(ctx) => ctx.inner().clone(),
            None => {
                println!("❌ PlatformContext not available");
                return Err("PlatformContext not available".to_string());
            }
        };

    // Connect (starts internal retry loop)
    let action_runtime_readiness = app_handle
        .try_state::<Arc<vortideck_core::services::execution::ActionRuntimeReadiness>>()
        .map(|state| state.inner().clone())
        .ok_or_else(|| "Action runtime readiness state not available".to_string())?;
    let remote_command_database = app_handle
        .try_state::<DbPool>()
        .map(|pool| Arc::new(vortideck_core::db::Database::new(pool.inner())))
        .ok_or_else(|| "Database pool not available for relay command acceptance".to_string())?;
    match relay_client
        .connect(
            state_manager.clone(),
            platform_context.event_bus.clone(),
            action_runtime_readiness,
            remote_command_database,
        )
        .await
    {
        Ok(_) => {
            println!("✅ Relay client started successfully");
        }
        Err(e) => {
            println!("❌ Relay connection failed: {}", e);
            return Err(e.to_string());
        }
    }

    // Store the RelayClient instance (lock still held - prevents race conditions)
    *client_lock = Some(relay_client);
    state_manager.set_relay_enabled(true).await;
    state_manager.set_relay_url(relay_base_url).await;

    println!("✅ Relay connection established with auto-retry");
    println!("📱 Desktop ID: {}", desktop_id);

    Ok(desktop_id)
}
