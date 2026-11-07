//! Plugin management commands with premium validation - NEW ARCHITECTURE

use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::{command, State, WebviewWindow};
use uuid::Uuid;
use vortideck_core::features::jwt_features::{
    get_jwt_provider, get_user_plan, is_user_premium, update_jwt_token,
};
use vortideck_core::VortiDeckCore;

#[derive(Debug, Serialize, Deserialize)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PluginValidationResult {
    pub can_install: bool,
    pub current_count: u32,
    pub limit: Option<u32>,
    pub is_premium: bool,
    pub message: String,
}

const PLUGIN_FRAME_IDLE_TTL: Duration = Duration::from_secs(30 * 60);

#[derive(Debug, Clone)]
struct PluginFrameSession {
    package_id: String,
    owner_window_label: String,
    expires_at: SystemTime,
}

#[derive(Debug, Clone, Default)]
pub struct PluginFrameSessionRegistry {
    sessions: Arc<Mutex<HashMap<String, PluginFrameSession>>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IssuedPluginFrameSession {
    pub session_id: String,
    pub package_id: String,
    pub expires_at_ms: u64,
}

impl PluginFrameSessionRegistry {
    fn issue(
        &self,
        package_id: String,
        owner_window_label: String,
        now: SystemTime,
    ) -> Result<IssuedPluginFrameSession, String> {
        let session_id = Uuid::new_v4().to_string();
        let expires_at = now + PLUGIN_FRAME_IDLE_TTL;
        let expires_at_ms = expires_at
            .duration_since(UNIX_EPOCH)
            .map_err(|_| "System clock is before the Unix epoch".to_string())?
            .as_millis()
            .try_into()
            .map_err(|_| "Plugin frame session expiry is out of range".to_string())?;
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| "Plugin frame session registry is unavailable".to_string())?;
        sessions.retain(|_, session| session.expires_at > now);
        sessions.insert(
            session_id.clone(),
            PluginFrameSession {
                package_id: package_id.clone(),
                owner_window_label,
                expires_at,
            },
        );
        Ok(IssuedPluginFrameSession {
            session_id,
            package_id,
            expires_at_ms,
        })
    }

    fn resolve(
        &self,
        session_id: &str,
        owner_window_label: &str,
        now: SystemTime,
    ) -> Result<String, String> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| "Plugin frame session registry is unavailable".to_string())?;
        sessions.retain(|_, session| session.expires_at > now);
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| "Plugin frame session is missing, stale, or expired".to_string())?;
        if session.owner_window_label != owner_window_label {
            return Err("Plugin frame session belongs to another host window".to_string());
        }
        session.expires_at = now + PLUGIN_FRAME_IDLE_TTL;
        Ok(session.package_id.clone())
    }

    fn revoke(&self, session_id: &str, owner_window_label: &str) -> Result<bool, String> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| "Plugin frame session registry is unavailable".to_string())?;
        let belongs_to_owner = sessions
            .get(session_id)
            .map(|session| session.owner_window_label == owner_window_label)
            .unwrap_or(false);
        if !belongs_to_owner {
            return Ok(false);
        }
        Ok(sessions.remove(session_id).is_some())
    }

    pub fn revoke_package(&self, package_id: &str) -> Result<usize, String> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| "Plugin frame session registry is unavailable".to_string())?;
        let previous_len = sessions.len();
        sessions.retain(|_, session| session.package_id != package_id);
        Ok(previous_len - sessions.len())
    }
}

fn resolve_frame_package(
    registry: &PluginFrameSessionRegistry,
    session_id: &str,
    window: &WebviewWindow,
) -> Result<String, String> {
    registry.resolve(session_id, window.label(), SystemTime::now())
}

/// Check if user can install a new plugin (validates against premium limits) - NEW ARCHITECTURE
#[command]
pub async fn can_install_plugin(
    core: State<'_, VortiDeckCore>,
) -> Result<PluginValidationResult, String> {
    println!("🔍 Checking plugin installation permissions via NEW ARCHITECTURE...");

    let jwt_provider = get_jwt_provider();
    let is_premium = is_user_premium();
    let user_plan = get_user_plan();
    let user_limit = jwt_provider.get_plugin_limit();

    // Get REAL plugin count from NEW ARCHITECTURE
    let current_count = core.plugin_manager().get_plugin_count().await as u32;
    println!(
        "📊 Real plugin count from NEW vortideck-core: {}",
        current_count
    );

    let can_install = if user_limit == u32::MAX {
        true
    } else {
        current_count < user_limit
    };

    let message = if can_install {
        format!(
            "✅ Can install plugins. Current: {}, Limit: {} (via NEW ARCHITECTURE)",
            current_count,
            if user_limit == u32::MAX {
                "unlimited".to_string()
            } else {
                user_limit.to_string()
            }
        )
    } else {
        format!(
            "❌ Plugin limit reached. Current: {}, Limit: {}. Plan: {}. {} (via NEW ARCHITECTURE)",
            current_count,
            user_limit,
            user_plan,
            if is_premium {
                "Contact support to increase limit."
            } else {
                "Upgrade to Premium!"
            }
        )
    };

    println!("{}", message);

    Ok(PluginValidationResult {
        can_install,
        current_count,
        limit: if user_limit == u32::MAX {
            None
        } else {
            Some(user_limit)
        },
        is_premium,
        message,
    })
}

/// Get plugin usage statistics - NEW ARCHITECTURE  
#[command]
pub async fn get_plugin_usage_stats(
    core: State<'_, VortiDeckCore>,
) -> Result<serde_json::Value, String> {
    println!("📊 Getting plugin usage stats via NEW ARCHITECTURE...");

    let jwt_provider = get_jwt_provider();
    let user_limit = jwt_provider.get_plugin_limit();
    let current_count = core.plugin_manager().get_plugin_count().await as u32;

    println!(
        "📊 Plugin usage stats (NEW ARCHITECTURE) - Current: {}, Limit: {}",
        current_count,
        if user_limit == u32::MAX {
            "unlimited".to_string()
        } else {
            user_limit.to_string()
        }
    );

    Ok(serde_json::json!({
        "current_count": current_count,
        "limit": if user_limit == u32::MAX { serde_json::Value::Null } else { serde_json::Value::from(user_limit) },
        "is_premium": is_user_premium(),
        "plan": get_user_plan(),
        "unlimited": user_limit == u32::MAX,
        "architecture": "NEW_VORTIDECK_CORE"
    }))
}

/// Get detailed plugin information for debugging/display - NEW ARCHITECTURE
#[command]
pub async fn get_detailed_plugin_info(
    core: State<'_, VortiDeckCore>,
) -> Result<serde_json::Value, String> {
    println!("🔍 Getting detailed plugin information via NEW ARCHITECTURE...");

    let jwt_provider = get_jwt_provider();
    let user_limit = jwt_provider.get_plugin_limit();
    let current_count = core.plugin_manager().get_plugin_count().await as u32;

    // Get plugin bundles from NEW ARCHITECTURE (grouped for marketplace display)
    let plugin_bundles = core.plugin_manager().get_plugin_bundles().await;

    let web_extension_packages = core.plugin_manager().list_web_extension_package_ids().await;

    // Convert to JSON format for frontend
    let plugin_list: Vec<serde_json::Value> = plugin_bundles
        .into_iter()
        .map(|metadata| {
            let package_id = vortideck_core::plugins::bundle::canonical_package_id(&metadata.id);
            let web_extensions_enabled = web_extension_packages.contains(package_id);

            serde_json::json!({
                // `plugin.` is an internal runtime-registry namespace. Public
                // package surfaces use the stable manifest id so this record
                // merges with `list_packages` instead of appearing twice.
                "id": package_id,
                "name": metadata.name,
                "version": metadata.version,
                "description": metadata.description,
                "author": metadata.author,
                "icon_path": metadata.icon_path,
                "enabled": true,
                "web_extensions": web_extensions_enabled,
                "architecture": "NEW_VORTIDECK_CORE"
            })
        })
        .collect();

    let bundle_count = plugin_list.len() as u32;

    println!(
        "📦 Found {} plugin bundles in NEW ARCHITECTURE (marketplace view)",
        bundle_count
    );
    for plugin in &plugin_list {
        println!("  - {} ({})", plugin["name"], plugin["id"]);
    }

    Ok(serde_json::json!({
        "plugins": plugin_list,
        "count": bundle_count,
        "limit": if user_limit == u32::MAX { serde_json::Value::Null } else { serde_json::Value::from(user_limit) },
        "is_premium": is_user_premium(),
        "plan": get_user_plan(),
        "unlimited": user_limit == u32::MAX,
        "architecture": "NEW_VORTIDECK_CORE"
    }))
}

/// Quick test command to verify plugin count fix - NEW ARCHITECTURE
#[command]
pub async fn test_plugin_count_fix(core: State<'_, VortiDeckCore>) -> Result<String, String> {
    println!("🧪 TESTING: Plugin count fix verification via NEW ARCHITECTURE");

    let current_count = core.plugin_manager().get_plugin_count().await;
    let jwt_provider = get_jwt_provider();
    let user_limit = jwt_provider.get_plugin_limit();

    let message = format!(
        "🔧 PLUGIN COUNT FIX TEST (NEW ARCHITECTURE):\n\
        📊 Real plugin count: {}\n\
        🎯 User limit: {}\n\
        💎 Is premium: {}\n\
        📋 Plan: {}\n\
        🏗️ Architecture: NEW VortiDeckCore\n\
        ✅ This should NOT show 0 anymore! (Unless you have 0 plugins)",
        current_count,
        if user_limit == u32::MAX {
            "unlimited".to_string()
        } else {
            user_limit.to_string()
        },
        is_user_premium(),
        get_user_plan()
    );

    println!("{}", message);
    Ok(message)
}

// ============================================================================
// PLUGIN SANDBOXING API BRIDGE COMMANDS
// ============================================================================

/// Issue a bounded principal for one sandboxed plugin frame. The package ID is
/// accepted only at this trusted host setup boundary; subsequent plugin calls
/// resolve it from the registry and cannot replace it in message payloads.
#[command]
pub async fn register_plugin_frame_session(
    package_id: String,
    window: WebviewWindow,
    registry: State<'_, PluginFrameSessionRegistry>,
    vortideck_core: State<'_, VortiDeckCore>,
) -> Result<IssuedPluginFrameSession, String> {
    let manifest = vortideck_core
        .plugin_manager()
        .package_manifest(&package_id)
        .await
        .ok_or_else(|| format!("Plugin package is not installed: {package_id}"))?;
    let canonical_package_id =
        vortideck_core::plugins::bundle::canonical_package_id(&manifest.id).to_string();
    if !vortideck_core
        .plugin_manager()
        .is_package_runtime_loaded(&canonical_package_id)
        .await
    {
        return Err(format!(
            "Plugin package is not active: {canonical_package_id}"
        ));
    }
    registry.issue(
        canonical_package_id,
        window.label().to_string(),
        SystemTime::now(),
    )
}

#[command]
pub fn revoke_plugin_frame_session(
    session_id: String,
    window: WebviewWindow,
    registry: State<'_, PluginFrameSessionRegistry>,
) -> Result<bool, String> {
    registry.revoke(&session_id, window.label())
}

#[command]
pub fn get_plugin_frame_scope(
    session_id: String,
    window: WebviewWindow,
    registry: State<'_, PluginFrameSessionRegistry>,
    vortideck_core: State<'_, VortiDeckCore>,
) -> Result<vortideck_core::plugins::EffectivePluginScope, String> {
    let package_id = resolve_frame_package(&registry, &session_id, &window)?;
    Ok(vortideck_core
        .plugin_manager()
        .get_permission_checker()
        .effective_scope(&package_id))
}

/// Call a WASM plugin function from a webview (with permission enforcement)
#[command]
pub async fn plugin_call_wasm(
    session_id: String,
    function: String,
    data: String,
    window: WebviewWindow,
    registry: State<'_, PluginFrameSessionRegistry>,
    vortideck_core: State<'_, VortiDeckCore>,
) -> Result<String, String> {
    let plugin_id = resolve_frame_package(&registry, &session_id, &window)?;
    println!(
        "🔌 [plugin_call_wasm] Plugin '{}' calling function '{}'",
        plugin_id, function
    );

    // Call WASM plugin (no permission needed - plugin calling its own code)
    let result = vortideck_core
        .plugin_manager()
        .call_plugin_function(&plugin_id, &function, data.as_bytes())
        .await?;

    // Convert bytes to string
    let result_str = String::from_utf8(result)
        .map_err(|e| format!("Failed to convert result to string: {}", e))?;

    Ok(result_str)
}

/// Run a flow from a plugin webview (requires "flows" permission)
#[command]
pub async fn plugin_run_flow(
    session_id: String,
    flow_name: String,
    _input: String,
    window: WebviewWindow,
    registry: State<'_, PluginFrameSessionRegistry>,
    vortideck_core: State<'_, VortiDeckCore>,
) -> Result<String, String> {
    let plugin_id = resolve_frame_package(&registry, &session_id, &window)?;
    println!(
        "🔌 [plugin_run_flow] Plugin '{}' running flow '{}'",
        plugin_id, flow_name
    );

    // Check "flows" permission
    use vortideck_core::plugins::bundle::PluginPermission;
    vortideck_core
        .plugin_manager()
        .get_permission_checker()
        .enforce_permission(&plugin_id, &PluginPermission::Flows)?;

    // TODO: Implement flow execution
    // For now, return a placeholder
    Err("Flow execution not yet implemented".to_string())
}

/// Get value from plugin-scoped storage (requires "storage" permission)
#[command]
pub async fn plugin_storage_get(
    session_id: String,
    key: String,
    window: WebviewWindow,
    registry: State<'_, PluginFrameSessionRegistry>,
    vortideck_core: State<'_, VortiDeckCore>,
) -> Result<Option<String>, String> {
    let plugin_id = resolve_frame_package(&registry, &session_id, &window)?;
    println!(
        "🔌 [plugin_storage_get] Plugin '{}' getting key '{}'",
        plugin_id, key
    );

    // Check "storage" permission
    use vortideck_core::plugins::bundle::PluginPermission;
    vortideck_core
        .plugin_manager()
        .get_permission_checker()
        .enforce_permission(&plugin_id, &PluginPermission::Storage)?;

    Err("Plugin-scoped storage is not available in this release".to_string())
}

/// Set value in plugin-scoped storage (requires "storage" permission)
#[command]
pub async fn plugin_storage_set(
    session_id: String,
    key: String,
    _value: String,
    window: WebviewWindow,
    registry: State<'_, PluginFrameSessionRegistry>,
    vortideck_core: State<'_, VortiDeckCore>,
) -> Result<(), String> {
    let plugin_id = resolve_frame_package(&registry, &session_id, &window)?;
    println!(
        "🔌 [plugin_storage_set] Plugin '{}' setting key '{}'",
        plugin_id, key
    );

    // Check "storage" permission
    use vortideck_core::plugins::bundle::PluginPermission;
    vortideck_core
        .plugin_manager()
        .get_permission_checker()
        .enforce_permission(&plugin_id, &PluginPermission::Storage)?;

    Err("Plugin-scoped storage is not available in this release".to_string())
}

/// Open a plugin in a separate native window
#[command]
pub async fn open_plugin_window(app: tauri::AppHandle, plugin_id: String) -> Result<(), String> {
    use tauri::Manager;

    let plugin_id = vortideck_core::plugins::bundle::canonical_package_id(&plugin_id).to_string();

    println!(
        "🪟 [open_plugin_window] Opening window for plugin '{}'",
        plugin_id
    );

    // Generate unique window label (sanitize: replace dots with dashes)
    let sanitized_id = plugin_id.replace('.', "-");
    let window_label = format!("plugin-{}", sanitized_id);

    // Check if window already exists
    if let Some(existing_window) = app.get_webview_window(&window_label) {
        // If window exists, focus it instead of creating a new one
        if let Err(e) = existing_window.set_focus() {
            eprintln!("Error focusing existing plugin window: {}", e);
        }
        println!("🪟 Plugin window already exists, focusing it");
        return Ok(());
    }

    // Create new plugin window
    let window = tauri::WebviewWindowBuilder::new(
        &app,
        &window_label,
        tauri::WebviewUrl::App(
            format!("plugin-window?id={}", urlencoding::encode(&plugin_id)).into(),
        ),
    )
    .title(format!("Plugin: {}", plugin_id))
    .inner_size(800.0, 600.0)
    .resizable(true)
    .build()
    .map_err(|e| format!("Failed to create plugin window: {}", e))?;

    println!("✅ Plugin window created successfully: {}", window_label);
    Ok(())
}

#[cfg(test)]
mod frame_session_tests {
    use super::*;

    #[test]
    fn frame_session_binds_package_and_owner_until_revoked() {
        let registry = PluginFrameSessionRegistry::default();
        let now = UNIX_EPOCH + Duration::from_secs(10_000);
        let issued = registry
            .issue("example.plugin".into(), "main".into(), now)
            .expect("issue session");

        assert_eq!(
            registry
                .resolve(&issued.session_id, "main", now + Duration::from_secs(1))
                .expect("resolve owner"),
            "example.plugin"
        );
        assert!(registry
            .resolve(&issued.session_id, "other", now + Duration::from_secs(2))
            .unwrap_err()
            .contains("another host window"));
        assert!(registry
            .revoke(&issued.session_id, "other")
            .is_ok_and(|removed| !removed));
        assert!(registry
            .revoke(&issued.session_id, "main")
            .is_ok_and(|removed| removed));
        assert!(registry
            .resolve(&issued.session_id, "main", now + Duration::from_secs(3))
            .unwrap_err()
            .contains("missing, stale, or expired"));
    }

    #[test]
    fn inactive_frame_session_expires() {
        let registry = PluginFrameSessionRegistry::default();
        let now = UNIX_EPOCH + Duration::from_secs(20_000);
        let issued = registry
            .issue("example.plugin".into(), "main".into(), now)
            .expect("issue session");

        assert!(registry
            .resolve(
                &issued.session_id,
                "main",
                now + PLUGIN_FRAME_IDLE_TTL + Duration::from_secs(1),
            )
            .unwrap_err()
            .contains("missing, stale, or expired"));
    }
}
