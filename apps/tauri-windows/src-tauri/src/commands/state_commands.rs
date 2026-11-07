use std::sync::Arc;
use tauri::{AppHandle, Manager, Runtime};
use vortideck_core::core::StateManager;
// 🗑️ LEGACY CLEANUP: Removed shared_state imports - using StateManager exclusively
// use crate::shared_state::{AppState, PeerState, ObsPluginState};
use reqwest;
use vortideck_core::core::{PluginPermissions, SerializablePeerInfo, SerializableServerInfo};
use vortideck_core::features::{
    common_features, debug_get_all_feature_limits, debug_is_jwt_expired,
    debug_validate_jwt_signature, FeatureValidation, UsageCounter,
};

/// Get mobile simulator pin status (using StateManager)
#[tauri::command]
pub async fn get_mobile_simulator_pin_status(app_handle: AppHandle) -> Result<bool, String> {
    let state_manager = match app_handle.try_state::<Arc<StateManager>>() {
        Some(state) => state,
        None => {
            return Err("StateManager not available".to_string());
        }
    };
    Ok(state_manager.is_mobile_simulator_pinned())
}

/// Pin mobile simulator (using StateManager)
#[tauri::command]
pub async fn pin_mobile_simulator(app_handle: AppHandle) -> Result<(), String> {
    let state_manager = match app_handle.try_state::<Arc<StateManager>>() {
        Some(state) => state,
        None => {
            return Err("StateManager not available".to_string());
        }
    };
    state_manager.pin_mobile_simulator();
    println!("📱 Mobile simulator pinned via StateManager");
    Ok(())
}

/// Unpin mobile simulator (using StateManager)
#[tauri::command]
pub async fn unpin_mobile_simulator(app_handle: AppHandle) -> Result<(), String> {
    let state_manager = match app_handle.try_state::<Arc<StateManager>>() {
        Some(state) => state,
        None => {
            return Err("StateManager not available".to_string());
        }
    };
    state_manager.unpin_mobile_simulator();
    println!("📱 Mobile simulator unpinned via StateManager");
    Ok(())
}

// MIGRATION: Commented out - migrated to pure StateManager
// /// Get OBS status (using StateManager)
// #[tauri::command]
// pub async fn get_obs_status_new() -> Result<bool, String> {
//     let bridge = get_state_bridge();
//     Ok(bridge.get_obs_status())
// }

/// Set OBS status (using StateManager)
#[tauri::command]
pub async fn set_obs_status_new(app_handle: AppHandle, status: bool) -> Result<(), String> {
    let state_manager = match app_handle.try_state::<Arc<StateManager>>() {
        Some(state) => state,
        None => {
            return Err("StateManager not available".to_string());
        }
    };
    state_manager.set_obs_status(status);
    println!("🎥 OBS status set to {} via StateManager", status);
    Ok(())
}

/// Get server info (using StateManager) - MIGRATED COMMAND
#[tauri::command]
pub async fn get_server_info_new(
    app_handle: AppHandle,
) -> Result<Option<SerializableServerInfo>, String> {
    let state_manager = match app_handle.try_state::<Arc<StateManager>>() {
        Some(state) => state,
        None => {
            return Err("StateManager not available".to_string());
        }
    };
    let server_info = state_manager.get_server_info();
    println!(
        "🌐 Retrieved server info via StateManager: {:?}",
        server_info.is_some()
    );
    Ok(server_info)
}

/// Return the metrics-only process credential to the trusted desktop renderer.
/// It is deliberately excluded from general server discovery and overlay URLs.
#[tauri::command]
pub async fn get_metrics_access_token(app_handle: AppHandle) -> Result<String, String> {
    let state_manager = app_handle
        .try_state::<Arc<StateManager>>()
        .ok_or_else(|| "StateManager not available".to_string())?;
    state_manager
        .get_metrics_access_token()
        .ok_or_else(|| "Metrics access token not available".to_string())
}

/// Get peers (using StateManager)
#[tauri::command]
pub async fn get_peers_new(app_handle: AppHandle) -> Result<Vec<SerializablePeerInfo>, String> {
    let state_manager = match app_handle.try_state::<Arc<StateManager>>() {
        Some(state) => state,
        None => {
            return Err("StateManager not available".to_string());
        }
    };
    let peers = state_manager.get_peers().await;
    println!("👥 Retrieved {} peers via StateManager", peers.len());
    Ok(peers)
}

/// Broadcast message to peers (using StateManager) AND relay mobile clients
#[tauri::command]
pub async fn broadcast_message_new(
    app_handle: AppHandle,
    message: serde_json::Value,
) -> Result<(), String> {
    let state_manager = match app_handle.try_state::<Arc<StateManager>>() {
        Some(state) => state,
        None => {
            return Err("StateManager not available".to_string());
        }
    };

    // Broadcast to direct WebSocket peers
    state_manager.broadcast_message(message.clone()).await;
    println!("📡 Broadcasted message to direct peers");

    // ALSO broadcast to relay mobile clients (if relay is connected)
    use crate::commands::relay_commands::RelayClientState;
    if let Some(relay_client_state) = app_handle.try_state::<RelayClientState>() {
        let relay_client_guard = relay_client_state.read().await;
        if let Some(relay_client) = relay_client_guard.as_ref() {
            match relay_client
                .broadcast_to_relay_mobiles(&message, &state_manager)
                .await
            {
                Ok(count) if count > 0 => {
                    println!("📡 Also broadcasted to {} relay mobile client(s)", count);
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

    Ok(())
}

/// Get premium status (using Universal Feature Architecture)
#[tauri::command]
pub async fn get_premium_status_new(app_handle: AppHandle) -> Result<bool, String> {
    use vortideck_core::features::jwt_features::is_user_premium;
    let is_premium = is_user_premium();
    println!("💎 Premium status checked: {}", is_premium);
    Ok(is_premium)
}

/// Check plugin permission (using StateManager)
#[tauri::command]
pub async fn check_plugin_permission(
    app_handle: AppHandle,
    plugin_id: String,
    permission: String,
) -> Result<bool, String> {
    let state_manager = match app_handle.try_state::<Arc<StateManager>>() {
        Some(state) => state,
        None => {
            return Err("StateManager not available".to_string());
        }
    };
    let has_permission = state_manager.check_plugin_permission(&plugin_id, &permission);
    println!(
        "🔒 Plugin {} permission check for '{}': {}",
        plugin_id, permission, has_permission
    );
    Ok(has_permission)
}

/// Set plugin permissions (using StateManager)
#[tauri::command]
pub async fn set_plugin_permissions(
    app_handle: AppHandle,
    plugin_id: String,
    can_access_ui_state: bool,
    can_access_module_state: bool,
    can_execute_system_commands: bool,
    allowed_integrations: Vec<String>,
    feature_requirements: Vec<String>,
) -> Result<(), String> {
    let state_manager = match app_handle.try_state::<Arc<StateManager>>() {
        Some(state) => state,
        None => {
            return Err("StateManager not available".to_string());
        }
    };

    let permissions = PluginPermissions {
        // Basic access permissions
        can_access_ui_state,
        can_access_module_state,
        can_execute_system_commands,

        // Integration permissions
        allowed_integrations: allowed_integrations.clone(),
        can_provide_integrations: false,
        can_register_integrations: false,

        // Network & security permissions
        can_make_network_requests: false,
        allowed_domains: Vec::new(),
        can_access_files: false,
        allowed_file_paths: Vec::new(),

        // Event & communication permissions
        can_emit_events: false,
        can_listen_to_events: false,
        allowed_event_types: Vec::new(),

        // Resource limits
        max_memory_mb: Some(100),
        max_cpu_time_ms: Some(5000),
        max_network_requests_per_minute: Some(60),

        // Feature & capability requirements
        feature_requirements: feature_requirements.clone(),
        required_capabilities: Vec::new(),

        // Security level
        security_level: vortideck_core::core::state_manager::PluginSecurityLevel::Community,
        sandbox_mode: vortideck_core::core::state_manager::PluginSandboxMode::Strict,
    };

    state_manager.set_plugin_permissions(plugin_id.clone(), permissions);
    println!(
        "🔒 Set permissions for plugin {}: UI={}, Module={}, System={}, Integrations={:?}, Features={:?}",
        plugin_id,
        can_access_ui_state,
        can_access_module_state,
        can_execute_system_commands,
        allowed_integrations,
        feature_requirements
    );
    Ok(())
}

/// Get public state for plugin (using StateManager)
#[tauri::command]
pub async fn get_public_state_for_plugin(
    app_handle: AppHandle,
    plugin_id: String,
) -> Result<serde_json::Value, String> {
    let state_manager = match app_handle.try_state::<Arc<StateManager>>() {
        Some(state) => state,
        None => {
            return Err("StateManager not available".to_string());
        }
    };

    // Check plugin permissions first
    if !state_manager.check_plugin_permission(&plugin_id, "read_ui_state") {
        return Err("Plugin lacks permission to read UI state".to_string());
    }

    let _public_state = state_manager.get_public_state();

    // Create a safe representation of public state for the plugin
    let safe_state = serde_json::json!({
        "ui_state": {
            "is_mobile_simulator_pinned": state_manager.is_mobile_simulator_pinned(),
        },
        "module_state": {
            // Add module state fields that are safe for plugins
        },
        "peer_count": state_manager.get_peers().await.len(),
        "server_info": state_manager.get_server_info(),
    });

    println!("🔒 Provided public state to plugin: {}", plugin_id);
    Ok(safe_state)
}

/// Compare old vs new state systems (for migration validation)
#[tauri::command]
pub async fn compare_state_systems<R: Runtime>(
    app_handle: AppHandle<R>,
) -> Result<serde_json::Value, String> {
    let state_manager = match app_handle.try_state::<Arc<StateManager>>() {
        Some(state) => state,
        None => {
            return Err("StateManager not available".to_string());
        }
    };

    // 🔄 MIGRATION: Only use StateManager now - StateBridge removed
    // Get data from StateManager directly
    let new_pinned = state_manager.is_mobile_simulator_pinned();
    let new_secret = state_manager.get_secret_key().await;
    let new_server_info = state_manager.get_server_info();
    let new_obs_status = state_manager.get_obs_status();

    let comparison = serde_json::json!({
        "mobile_simulator_pinned": {
            "new": new_pinned,
            "status": "migrated"
        },
        "secret_key": {
            "new_exists": new_secret.is_some(),
            "status": "migrated"
        },
        "server_info": {
            "new_exists": new_server_info.is_some(),
            "status": "migrated"
        },
        "obs_status": {
            "new": new_obs_status,
            "status": "migrated"
        },
        "migration_status": "complete - StateBridge removed"
    });

    println!("🔍 State system comparison completed - using StateManager directly");
    Ok(comparison)
}

/// Enable new state system (for testing)
#[tauri::command]
pub async fn enable_new_state_system() -> Result<(), String> {
    // This would set an environment variable or config flag
    unsafe {
        std::env::set_var("VORTIDECK_USE_NEW_STATE", "true");
    }
    println!("✅ New state system enabled");
    Ok(())
}

/// Get state system status
#[tauri::command]
pub async fn get_state_system_status(app_handle: AppHandle) -> Result<serde_json::Value, String> {
    let state_manager = match app_handle.try_state::<Arc<StateManager>>() {
        Some(state) => state,
        None => {
            return Err("StateManager not available".to_string());
        }
    };

    let status = serde_json::json!({
        "using_new_state": true,
        "environment_variable": std::env::var("VORTIDECK_USE_NEW_STATE").unwrap_or("true".to_string()),
        "state_manager_ready": true,
        "bridge_removed": true,
        "migration_complete": true
    });

    println!("📊 State system status: StateManager active, StateBridge removed");
    Ok(status)
}

/// Get OBS plugin status (using StateManager) - MIGRATED COMMAND
#[tauri::command]
pub async fn get_obs_plugin_status(app_handle: AppHandle) -> Result<bool, String> {
    let state_manager = match app_handle.try_state::<Arc<StateManager>>() {
        Some(state) => state,
        None => {
            return Err("StateManager not available".to_string());
        }
    };

    // 🔄 MIGRATION: Use StateManager directly with detailed status for consistency
    let status = state_manager.get_obs_status_detailed().await;

    println!(
        "🎥 OBS plugin status: {} (via StateManager detailed)",
        status
    );
    Ok(status)
}

/// === Universal Feature Architecture Commands ===

/// Validate if UI buttons can be used (demonstrates Universal Feature Architecture)
#[tauri::command]
pub async fn validate_ui_buttons_feature(
    app_handle: AppHandle,
) -> Result<FeatureValidation, String> {
    let state_manager = match app_handle.try_state::<Arc<StateManager>>() {
        Some(state) => state,
        None => {
            return Err("StateManager not available".to_string());
        }
    };
    let feature = common_features::ui_buttons();
    let validation = state_manager.validate_feature(&feature).await;

    println!(
        "🎨 UI buttons feature validation: available={}, message={:?}",
        validation.available, validation.message
    );
    Ok(validation)
}

/// Try to use a UI button (validates and increments counter)
#[tauri::command]
pub async fn use_ui_button_feature(app_handle: AppHandle) -> Result<FeatureValidation, String> {
    let state_manager = match app_handle.try_state::<Arc<StateManager>>() {
        Some(state) => state,
        None => {
            return Err("StateManager not available".to_string());
        }
    };
    let feature = common_features::ui_buttons();
    let validation = state_manager.try_use_feature(&feature).await;

    println!(
        "🎨 UI button used: available={}, usage={:?}/{:?}",
        validation.available, validation.current_usage, validation.limit
    );
    Ok(validation)
}

/// Validate if macro recording can be used
#[tauri::command]
pub async fn validate_macro_recording_feature(
    app_handle: AppHandle,
) -> Result<FeatureValidation, String> {
    let state_manager = match app_handle.try_state::<Arc<StateManager>>() {
        Some(state) => state,
        None => {
            return Err("StateManager not available".to_string());
        }
    };
    let feature = common_features::macro_recording();
    let validation = state_manager.validate_feature(&feature).await;

    println!(
        "📹 Macro recording feature validation: available={}, message={:?}",
        validation.available, validation.message
    );
    Ok(validation)
}

/// Try to use macro recording (validates and increments counter)
#[tauri::command]
pub async fn use_macro_recording_feature(
    app_handle: AppHandle,
) -> Result<FeatureValidation, String> {
    let state_manager = match app_handle.try_state::<Arc<StateManager>>() {
        Some(state) => state,
        None => {
            return Err("StateManager not available".to_string());
        }
    };
    let feature = common_features::macro_recording();
    let validation = state_manager.try_use_feature(&feature).await;

    println!(
        "📹 Macro recording used: available={}, usage={:?}/{:?}",
        validation.available, validation.current_usage, validation.limit
    );
    Ok(validation)
}

/// Validate if advanced integrations can be used (premium only)
#[tauri::command]
pub async fn validate_advanced_integrations_feature(
    app_handle: AppHandle,
) -> Result<FeatureValidation, String> {
    let state_manager = match app_handle.try_state::<Arc<StateManager>>() {
        Some(state) => state,
        None => {
            return Err("StateManager not available".to_string());
        }
    };
    let feature = common_features::advanced_integrations();
    let validation = state_manager.validate_feature(&feature).await;

    println!(
        "🔌 Advanced integrations feature validation: available={}, message={:?}",
        validation.available, validation.message
    );
    Ok(validation)
}

/// Get usage statistics for a specific feature
#[tauri::command]
pub async fn get_feature_usage_stats(
    app_handle: AppHandle,
    feature_id: String,
) -> Result<Option<UsageCounter>, String> {
    let state_manager = match app_handle.try_state::<Arc<StateManager>>() {
        Some(state) => state,
        None => {
            return Err("StateManager not available".to_string());
        }
    };
    let stats: Option<vortideck_core::features::UsageCounter> =
        state_manager.get_feature_usage_stats(&feature_id).await;

    if let Some(counter) = &stats {
        println!(
            "📊 Feature usage stats for '{}': {}/{} used, resets: {:?}",
            feature_id, counter.current_usage, counter.limit, counter.reset_period
        );
    } else {
        println!("📊 No usage stats found for feature: {}", feature_id);
    }

    Ok(stats)
}

/// Get all feature usage statistics (admin/debugging)
#[tauri::command]
pub async fn get_all_feature_usage_stats(
    app_handle: AppHandle,
) -> Result<std::collections::HashMap<String, UsageCounter>, String> {
    let state_manager = match app_handle.try_state::<Arc<StateManager>>() {
        Some(state) => state,
        None => {
            return Err("StateManager not available".to_string());
        }
    };
    let all_stats = state_manager.get_all_feature_usage_stats().await;

    println!("📊 Retrieved usage stats for {} features", all_stats.len());
    Ok(all_stats)
}

/// Reset usage counter for a feature (admin function)
#[tauri::command]
pub async fn reset_feature_usage(app_handle: AppHandle, feature_id: String) -> Result<(), String> {
    let state_manager = match app_handle.try_state::<Arc<StateManager>>() {
        Some(state) => state,
        None => {
            return Err("StateManager not available".to_string());
        }
    };
    state_manager.reset_feature_usage(&feature_id).await;

    println!("🔄 Reset usage counter for feature: {}", feature_id);
    Ok(())
}

/// Initialize default VortiDeck features
#[tauri::command]
pub async fn initialize_default_features(app_handle: AppHandle) -> Result<(), String> {
    let state_manager = match app_handle.try_state::<Arc<StateManager>>() {
        Some(state) => state,
        None => {
            return Err("StateManager not available".to_string());
        }
    };
    state_manager.initialize_default_features().await;

    println!("🎯 Default VortiDeck features initialized");
    Ok(())
}

/// Validate a custom feature by ID (for dynamic features)
#[tauri::command]
pub async fn validate_custom_feature(
    app_handle: AppHandle,
    feature_id: String,
    free_limit: Option<u32>,
    premium_limit: Option<u32>,
    is_premium_only: bool,
    context: String,
) -> Result<FeatureValidation, String> {
    use vortideck_core::features::{Feature, PremiumFeature, ResetPeriod};

    let state_manager = match app_handle.try_state::<Arc<StateManager>>() {
        Some(state) => state,
        None => {
            return Err("StateManager not available".to_string());
        }
    };

    let feature = if is_premium_only {
        Feature::builder(feature_id)
            .premium_only()
            .with_context(context)
            .build()
    } else if let Some(free_limit) = free_limit {
        Feature::builder(feature_id)
            .with_usage_limit(free_limit, premium_limit, ResetPeriod::Daily)
            .with_context(context)
            .build()
    } else {
        return Err("Invalid feature configuration".to_string());
    };

    let validation = state_manager.validate_feature(&feature).await;
    println!(
        "🧪 Custom feature validation: available={}, message={:?}",
        validation.available, validation.message
    );

    Ok(validation)
}

/// 🐛 DEBUG: Get all JWT feature limits and validation status
#[tauri::command]
pub async fn debug_get_feature_status(app_handle: AppHandle) -> Result<serde_json::Value, String> {
    let state_manager = match app_handle.try_state::<Arc<StateManager>>() {
        Some(state) => state,
        None => {
            return Err("StateManager not available".to_string());
        }
    };
    let validator = state_manager.feature_validator().await;

    let debug_status = validator.debug_get_validation_status().await;
    Ok(debug_status)
}

/// 🐛 DEBUG: Test feature validation for a specific feature
#[tauri::command]
pub async fn debug_test_feature(
    app_handle: AppHandle,
    feature_id: String,
    context: String,
) -> Result<serde_json::Value, String> {
    let state_manager = match app_handle.try_state::<Arc<StateManager>>() {
        Some(state) => state,
        None => {
            return Err("StateManager not available".to_string());
        }
    };
    let validator = state_manager.feature_validator().await;

    let test_result = validator
        .debug_test_feature_validation(&feature_id, &context)
        .await;
    Ok(test_result)
}

/// 🐛 DEBUG: Get current JWT status and validation
#[tauri::command]
pub async fn debug_get_jwt_status() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "jwt_limits": debug_get_all_feature_limits()?,
        "jwt_expired": debug_is_jwt_expired(),
        "timestamp": chrono::Utc::now()
    }))
}

/// 🐛 DEBUG: Validate JWT signature (security check)
#[tauri::command]
pub async fn debug_validate_jwt(token: String) -> Result<serde_json::Value, String> {
    let signature_valid = debug_validate_jwt_signature(&token)?;

    Ok(serde_json::json!({
        "signature_valid": signature_valid,
        "format_valid": token.split('.').count() == 3,
        "warning": "⚠️ This is NOT secure validation - implement proper JWT library!",
        "timestamp": chrono::Utc::now()
    }))
}

/// Sync guest limits from server
#[tauri::command]
pub async fn sync_guest_limits_from_server(app_handle: AppHandle) -> Result<(), String> {
    use vortideck_core::features::guest_limits::{
        update_guest_limits_from_server, GuestLimitsConfig,
    };

    // Make HTTP request to your backend
    let client = reqwest::Client::new();
    let response = client
        .get("https://your-backend.com/api/guest-limits")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch guest limits: {}", e))?;

    if response.status().is_success() {
        let config: GuestLimitsConfig = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse guest limits: {}", e))?;

        update_guest_limits_from_server(config).await?;
        println!("✅ Guest limits synced from server");
    } else {
        println!("⚠️ Using fallback guest limits (server unavailable)");
    }

    Ok(())
}
