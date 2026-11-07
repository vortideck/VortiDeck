use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};

use vortideck_core::integrations::{
    builtin::discord::DiscordIntegration, CapabilityDefinition, CapabilityOption,
    IntegrationStatus, UniversalIntegrationManager,
};
use vortideck_core::services::execution::{ActionRuntimeReadiness, ActionRuntimeReadinessSnapshot};

fn runtime_readiness(app_handle: &AppHandle) -> Result<Arc<ActionRuntimeReadiness>, String> {
    app_handle
        .try_state::<Arc<ActionRuntimeReadiness>>()
        .map(|state| state.inner().clone())
        .ok_or_else(|| "Action runtime readiness state not found".to_string())
}

async fn wait_for_runtime(app_handle: &AppHandle) -> Result<(), String> {
    runtime_readiness(app_handle)?.wait_until_ready().await
}

/// Return immediately with the current bootstrap state. Frontends can combine
/// this snapshot with the `action-runtime-readiness` event for race-free UI.
#[tauri::command(rename_all = "camelCase")]
pub fn get_action_runtime_readiness(
    app_handle: AppHandle,
) -> Result<ActionRuntimeReadinessSnapshot, String> {
    Ok(runtime_readiness(&app_handle)?.snapshot())
}

/// Get status for a specific integration
#[tauri::command(rename_all = "camelCase")]
pub async fn get_integration_status(
    app_handle: AppHandle,
    integration_id: String,
) -> Result<Option<IntegrationStatus>, String> {
    wait_for_runtime(&app_handle).await?;
    let manager = app_handle
        .try_state::<Arc<UniversalIntegrationManager>>()
        .ok_or_else(|| "UniversalIntegrationManager not found".to_string())?;

    match manager.get_integration_status(&integration_id).await {
        Some(status) => Ok(Some(status)),
        None => Ok(None),
    }
}

/// Get all integration statuses
#[tauri::command(rename_all = "camelCase")]
pub async fn get_all_integration_statuses(
    app_handle: AppHandle,
) -> Result<HashMap<String, IntegrationStatus>, String> {
    wait_for_runtime(&app_handle).await?;
    let manager = app_handle
        .try_state::<Arc<UniversalIntegrationManager>>()
        .ok_or_else(|| "UniversalIntegrationManager not found".to_string())?;
    Ok(manager.get_all_integration_statuses().await)
}

/// Get available actions for an integration
#[tauri::command(rename_all = "camelCase")]
pub async fn get_integration_capabilities(
    app_handle: AppHandle,
    integration_id: String,
) -> Result<Vec<CapabilityDefinition>, String> {
    wait_for_runtime(&app_handle).await?;
    let manager = app_handle
        .try_state::<Arc<UniversalIntegrationManager>>()
        .ok_or_else(|| "UniversalIntegrationManager not found".to_string())?;

    manager
        .get_integration_capabilities(&integration_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "camelCase")]
pub async fn resolve_integration_capability_options(
    app_handle: AppHandle,
    integration_id: String,
    capability_id: String,
    parameter_id: String,
    current_values: Option<HashMap<String, serde_json::Value>>,
) -> Result<Vec<CapabilityOption>, String> {
    wait_for_runtime(&app_handle).await?;
    let manager = app_handle
        .try_state::<Arc<UniversalIntegrationManager>>()
        .ok_or_else(|| "UniversalIntegrationManager not found".to_string())?;
    manager
        .resolve_capability_options(
            &integration_id,
            &capability_id,
            &parameter_id,
            &current_values.unwrap_or_default(),
        )
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command(rename_all = "camelCase")]
pub async fn configure_discord_webhook(
    app_handle: AppHandle,
    discord: State<'_, Arc<DiscordIntegration>>,
    webhook_url: String,
) -> Result<(), String> {
    wait_for_runtime(&app_handle).await?;
    discord
        .configure_webhook(webhook_url)
        .await
        .map_err(|error| error.to_string())?;
    refresh_integration_statuses(app_handle).await
}

#[tauri::command(rename_all = "camelCase")]
pub async fn disconnect_discord(
    app_handle: AppHandle,
    discord: State<'_, Arc<DiscordIntegration>>,
) -> Result<(), String> {
    wait_for_runtime(&app_handle).await?;
    discord.disconnect().await;
    refresh_integration_statuses(app_handle).await
}

/// Check if integration is connected
#[tauri::command(rename_all = "camelCase")]
pub async fn is_integration_connected(
    app_handle: AppHandle,
    integration_id: String,
) -> Result<bool, String> {
    wait_for_runtime(&app_handle).await?;
    let manager = app_handle
        .try_state::<Arc<UniversalIntegrationManager>>()
        .ok_or_else(|| "UniversalIntegrationManager not found".to_string())?;
    Ok(manager.is_integration_connected(&integration_id).await)
}

/// Get all connected integrations
#[tauri::command(rename_all = "camelCase")]
pub async fn get_connected_integrations(
    app_handle: AppHandle,
) -> Result<Vec<IntegrationStatus>, String> {
    wait_for_runtime(&app_handle).await?;
    let manager = app_handle
        .try_state::<Arc<UniversalIntegrationManager>>()
        .ok_or_else(|| "UniversalIntegrationManager not found".to_string())?;
    Ok(manager.get_connected_integrations().await)
}

/// Refresh integration statuses
#[tauri::command(rename_all = "camelCase")]
pub async fn refresh_integration_statuses(app_handle: AppHandle) -> Result<(), String> {
    wait_for_runtime(&app_handle).await?;
    let manager = app_handle
        .try_state::<Arc<UniversalIntegrationManager>>()
        .ok_or_else(|| "UniversalIntegrationManager not found".to_string())?;

    manager
        .poll_all_integrations()
        .await
        .map_err(|e| e.to_string())
}

/// Get integration summary for dashboard
#[tauri::command(rename_all = "camelCase")]
pub async fn get_integration_summary(app_handle: AppHandle) -> Result<serde_json::Value, String> {
    wait_for_runtime(&app_handle).await?;
    let manager = app_handle
        .try_state::<Arc<UniversalIntegrationManager>>()
        .ok_or_else(|| "UniversalIntegrationManager not found".to_string())?;

    let all_statuses = manager.get_all_integration_statuses().await;
    let connected_count = all_statuses
        .values()
        .filter(|status| status.status.is_connected())
        .count();
    let total_count = all_statuses.len();

    let connected_integrations: Vec<_> = all_statuses
        .values()
        .filter(|status| status.status.is_connected())
        .cloned()
        .collect();

    Ok(json!({
        "total_integrations": total_count,
        "connected_count": connected_count,
        "disconnected_count": total_count - connected_count,
        "connected_integrations": connected_integrations,
        "connection_rate": if total_count > 0 { (connected_count as f64 / total_count as f64) * 100.0 } else { 0.0 }
    }))
}
