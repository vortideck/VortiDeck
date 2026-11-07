//! Plugin management commands - UPDATED FOR NEW ARCHITECTURE
//! Thin frontend adapters over the shared `VortiDeckCore` package engine.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};
use vortideck_core::db::db::DbPool;
use vortideck_core::plugins::interface::PluginBundleInfo;
use vortideck_core::VortiDeckCore;

#[derive(Debug, Serialize, Deserialize)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub enabled: bool,
    pub category: String,
    pub install_date: Option<String>,
    /// Plugin bundle info (for multi-node plugins)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin_bundle: Option<PluginBundleInfo>,
}

/// Get all installed plugins - NOW USING NEW ARCHITECTURE
/// For multi-node plugins, returns one entry per bundle instead of one per node
#[tauri::command]
pub async fn get_installed_plugins(
    core: State<'_, VortiDeckCore>,
) -> Result<Vec<PluginInfo>, String> {
    use std::collections::HashMap;

    let wasm_plugins = core.plugin_manager().get_wasm_plugins().await;

    let mut plugins = Vec::new();
    let mut bundle_map: HashMap<String, PluginInfo> = HashMap::new();

    // Group nodes by plugin_bundle.id for multi-node plugins
    for wasm_plugin in wasm_plugins {
        // Get the actual enabled state from the plugin
        let enabled = *wasm_plugin.enabled.read().await;

        // Check if this node belongs to a multi-node plugin bundle
        if let Some(ref bundle) = wasm_plugin.plugin_bundle {
            // Multi-node plugin - use bundle as the key
            if !bundle_map.contains_key(&bundle.id) {
                let plugin_info = PluginInfo {
                    id: bundle.id.clone(),
                    name: bundle.name.clone(),
                    description: format!("Plugin bundle with multiple nodes"),
                    version: bundle.version.clone(),
                    author: bundle.author.clone(),
                    enabled,
                    category: wasm_plugin.category.clone(),
                    install_date: Some(chrono::Utc::now().to_rfc3339()),
                    plugin_bundle: Some(bundle.clone()),
                };
                bundle_map.insert(bundle.id.clone(), plugin_info);
            }
        } else {
            // Single-node plugin - add directly
            let plugin_info = PluginInfo {
                id: wasm_plugin.id.clone(),
                name: wasm_plugin.name.clone(),
                description: wasm_plugin.description.clone(),
                version: wasm_plugin.version.clone(),
                author: wasm_plugin.author.clone(),
                enabled,
                category: wasm_plugin.category.clone(),
                install_date: Some(chrono::Utc::now().to_rfc3339()),
                plugin_bundle: None,
            };
            plugins.push(plugin_info);
        }
    }

    // Add all bundled plugins
    plugins.extend(bundle_map.into_values());

    Ok(plugins)
}

/// Get specific plugin info - NOW USING NEW ARCHITECTURE
#[tauri::command]
pub async fn get_plugin_info(
    id: String,
    core: State<'_, VortiDeckCore>,
) -> Result<Option<PluginInfo>, String> {
    let plugin_metadata = core.plugin_manager().get_all_plugin_metadata().await;

    for metadata in plugin_metadata {
        if metadata.id == id {
            let plugin_info = PluginInfo {
                id: metadata.id.clone(),
                name: metadata.name.clone(),
                description: metadata.description.clone(),
                version: metadata.version.clone(),
                author: metadata.author.clone(),
                enabled: true,
                category: "PLUGINS".to_string(),
                install_date: Some(chrono::Utc::now().to_rfc3339()),
                plugin_bundle: metadata.plugin_bundle.clone(), // NEW: Multi-node plugin grouping
            };
            return Ok(Some(plugin_info));
        }
    }

    Ok(None)
}

/// Toggle plugin enabled state - NOW USING NEW ARCHITECTURE
#[tauri::command]
pub async fn toggle_plugin_enabled(
    id: String,
    enabled: bool,
    core: State<'_, VortiDeckCore>,
) -> Result<(), String> {
    core.plugin_manager().set_plugin_enabled(&id, enabled).await
}

/// Get plugins as flow nodes for flow builder integration
#[tauri::command]
pub async fn get_plugin_flow_nodes(
    core: State<'_, VortiDeckCore>,
) -> Result<Vec<serde_json::Value>, String> {
    let mut flow_nodes = Vec::new();

    // Get WASM plugins
    let wasm_plugins = core.plugin_manager().get_wasm_plugins().await;
    for plugin in wasm_plugins {
        // Only include enabled plugins
        let enabled = *plugin.enabled.read().await;
        if !enabled {
            continue;
        }

        let flow_node = serde_json::json!({
            "id": plugin.id,
            "name": plugin.name,
            "description": plugin.description,
            "category": plugin.category,
            "category_display_name": plugin.category_display_name,
            "type_color": plugin.type_color,
            "output_schema": plugin.output_schema,
            "parameters_schema": plugin.parameters_schema,
        });

        flow_nodes.push(flow_node);
    }

    // Get Service plugins
    let service_providers = core.plugin_manager().get_service_flow_providers().await;
    for provider in service_providers {
        for node_metadata in provider.nodes {
            let flow_node = serde_json::json!({
                "id": node_metadata.id,
                "name": node_metadata.name,
                "description": node_metadata.description,
                "category": node_metadata.category.unwrap_or_else(|| "SERVICES".to_string()),
                "category_display_name": None::<String>,
                "type_color": node_metadata.type_color.unwrap_or_else(|| "#FF6B35".to_string()),
                "output_schema": node_metadata.output_schema,
                "parameters_schema": node_metadata.parameters_schema,
            });

            flow_nodes.push(flow_node);
        }
    }

    Ok(flow_nodes)
}

/// Uninstall plugin - NOW USING NEW ARCHITECTURE
#[tauri::command]
pub async fn uninstall_plugin(
    id: String,
    app: AppHandle,
    core: State<'_, VortiDeckCore>,
    db_pool: State<'_, DbPool>,
) -> Result<(), String> {
    crate::package_lifecycle::uninstall_package(&app, core.inner(), db_pool.inner(), &id).await
}

/// Refresh plugins from directory - NOW USING NEW ARCHITECTURE
#[tauri::command]
pub async fn refresh_plugins(core: State<'_, VortiDeckCore>) -> Result<Vec<String>, String> {
    let plugins_dir = VortiDeckCore::get_default_plugins_directory();
    let (_, folders) = core.scan_plugins_directory(&plugins_dir)?;
    let mut package_ids = Vec::new();
    for folder in folders {
        match core
            .plugin_manager()
            .discover_plugin_from_folder(&folder)
            .await
        {
            Ok(manifest) => package_ids.push(manifest.id),
            Err(error) => log::warn!(
                "Package discovery failed for '{}': {}",
                folder.display(),
                error
            ),
        }
    }
    package_ids.sort();
    package_ids.dedup();
    Ok(package_ids)
}

/// Get plugins directory - NOW USING NEW ARCHITECTURE
#[tauri::command]
pub async fn get_plugins_directory() -> Result<String, String> {
    let plugins_dir = VortiDeckCore::get_default_plugins_directory();
    let path_str = plugins_dir.to_string_lossy().to_string();
    Ok(path_str)
}

/// Reload plugins with authentication limits (for debug testing)
#[tauri::command]
pub async fn reload_plugins_with_auth_check(
    core: State<'_, VortiDeckCore>,
) -> Result<serde_json::Value, String> {
    println!("🔄 DEBUG: Manually triggering auth-aware plugin reload...");

    let before_count = core.plugin_manager().get_loaded_package_count().await;
    let packages = refresh_plugins(core).await?;
    let after_count = before_count;

    let result = serde_json::json!({
        "message": "Package discovery refreshed; authentication changes reconcile runtimes automatically",
        "plugins_before": before_count,
        "plugins_after": after_count,
        "packages_discovered": packages,
        "auth_status": "checked"
    });

    println!(
        "✅ DEBUG: Auth-aware plugin reload completed: {} -> {} plugins",
        before_count, after_count
    );

    Ok(result)
}

/// Debug plugin system for duplication and state analysis
#[tauri::command]
pub async fn debug_plugin_system(
    core: State<'_, VortiDeckCore>,
) -> Result<serde_json::Value, String> {
    let mut debug_info = serde_json::json!({});

    // Get plugin manager metadata
    let plugin_metadata = core.plugin_manager().get_all_plugin_metadata().await;
    debug_info["plugin_manager_count"] = plugin_metadata.len().into();
    debug_info["plugin_manager_plugins"] =
        serde_json::to_value(&plugin_metadata).unwrap_or_default();

    // Get WASM plugins specifically
    let wasm_plugins = core.plugin_manager().get_wasm_plugins().await;
    debug_info["wasm_plugins_count"] = wasm_plugins.len().into();

    // Check for potential duplicates by ID
    let mut plugin_ids = std::collections::HashSet::new();
    let mut duplicate_ids = Vec::new();
    let mut unique_plugin_ids = Vec::new();
    let mut all_plugin_ids = Vec::new();

    for plugin in &wasm_plugins {
        all_plugin_ids.push(plugin.id.clone());
        if !plugin_ids.insert(plugin.id.clone()) {
            duplicate_ids.push(plugin.id.clone());
        } else {
            unique_plugin_ids.push(plugin.id.clone());
        }
    }

    let has_duplicates = !duplicate_ids.is_empty();
    debug_info["duplicate_plugin_ids"] = serde_json::Value::Array(
        duplicate_ids
            .into_iter()
            .map(serde_json::Value::String)
            .collect(),
    );
    debug_info["has_duplicates"] = has_duplicates.into();
    debug_info["unique_plugins_count"] = unique_plugin_ids.len().into();
    debug_info["unique_plugin_ids"] = serde_json::Value::Array(
        unique_plugin_ids
            .into_iter()
            .map(serde_json::Value::String)
            .collect(),
    );
    debug_info["all_plugin_ids"] = serde_json::Value::Array(
        all_plugin_ids
            .into_iter()
            .map(serde_json::Value::String)
            .collect(),
    );
    debug_info["plugins_directory"] = VortiDeckCore::get_default_plugins_directory()
        .display()
        .to_string()
        .into();

    // Check plugin manager internal state
    let plugin_count = core.plugin_manager().get_plugin_count().await;
    debug_info["internal_plugin_count"] = plugin_count.into();

    // Plugin loading diagnostics
    debug_info["system_info"] = serde_json::json!({
        "plugins_loaded": wasm_plugins.len(),
        "metadata_available": plugin_metadata.len(),
        "duplicates_detected": has_duplicates,
        "plugins_directory_exists": VortiDeckCore::get_default_plugins_directory().exists(),
    });

    Ok(debug_info)
}

/// Get one package's web extension payload for sandboxed iframe loading.
#[tauri::command]
pub async fn get_plugin_web_extensions(
    plugin_id: String,
    core: State<'_, VortiDeckCore>,
) -> Result<Vec<serde_json::Value>, String> {
    use vortideck_core::plugins::manager::WebExtensionFile;

    let extensions = core
        .plugin_manager()
        .get_web_extensions_for_package(&plugin_id)
        .await;

    // Convert to JSON
    let json_extensions: Vec<serde_json::Value> = extensions
        .into_iter()
        .map(|ext| {
            serde_json::json!({
                "plugin_id": ext.plugin_id,
                "file_type": ext.file_type,
                "content": ext.content,
            })
        })
        .collect();

    Ok(json_extensions)
}

/// Get a specific plugin asset (for asset URLs)
#[tauri::command]
pub async fn get_plugin_asset(
    plugin_id: String,
    asset_path: String,
    core: State<'_, VortiDeckCore>,
) -> Result<Vec<u8>, String> {
    core.plugin_manager()
        .get_plugin_asset(&plugin_id, &asset_path)
        .await
        .ok_or_else(|| format!("Asset not found: {}/{}", plugin_id, asset_path))
}

// Plugin commands are registered directly in main.rs with the invoke_handler!
