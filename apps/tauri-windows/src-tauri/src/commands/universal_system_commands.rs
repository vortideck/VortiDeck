use std::sync::Arc;
use tauri::{AppHandle, Manager};
use vortideck_core::core::{PlatformContext, SystemAction, SystemInfo};
use vortideck_core::services::system::{
    execute_system_action_universal, get_available_actions_universal, get_system_info_universal,
    set_volume_universal,
};

/// Tauri command to set system volume using universal abstractions
#[tauri::command]
pub async fn set_volume(
    app_handle: AppHandle,
    volume: f32,
    button_id: Option<String>,
) -> Result<String, String> {
    // Get platform context from app state
    let platform_context = app_handle.state::<Arc<PlatformContext>>();

    set_volume_universal(&platform_context, volume, button_id)
        .await
        .map_err(|e| e.to_string())
}

/// Tauri command to get system information using universal abstractions
#[tauri::command]
pub async fn get_system_info_universal_cmd(app_handle: AppHandle) -> Result<SystemInfo, String> {
    // Get platform context from app state
    let platform_context = app_handle.state::<Arc<PlatformContext>>();

    get_system_info_universal(&platform_context)
        .await
        .map_err(|e| e.to_string())
}

/// Tauri command to get available system actions using universal abstractions
#[tauri::command]
pub async fn get_available_actions(app_handle: AppHandle) -> Result<Vec<SystemAction>, String> {
    // Get platform context from app state
    let platform_context = app_handle.state::<Arc<PlatformContext>>();

    get_available_actions_universal(&platform_context)
        .await
        .map_err(|e| e.to_string())
}

/// Tauri command to execute system action using universal abstractions
#[tauri::command]
pub async fn execute_system_action(
    app_handle: AppHandle,
    action_id: String,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    // Get platform context from app state
    let platform_context = app_handle.state::<Arc<PlatformContext>>();

    execute_system_action_universal(&platform_context, &action_id, params)
        .await
        .map_err(|e| e.to_string())
}

/// Register all universal system commands with Tauri
pub fn register_universal_system_commands() -> tauri::plugin::Builder<tauri::Wry> {
    tauri::plugin::Builder::new("universal-system").invoke_handler(tauri::generate_handler![
        set_volume,
        get_system_info_universal_cmd,
        get_available_actions,
        execute_system_action
    ])
}
