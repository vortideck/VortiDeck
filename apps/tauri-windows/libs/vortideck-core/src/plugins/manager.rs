//! Plugin manager
//!
//! This module handles the management of Extism WebAssembly plugins.

use super::Plugin;
use crate::features::requirements::FeatureRequirement;
use crate::features::types::{PremiumFeature, ResetPeriod};
use crate::plugins::flow_integration::{WasmFlowNodeProvider, WasmFlowNodeWrapper};
use crate::plugins::interface::PluginError;
use crate::plugins::interface::{
    FlowNodeMetadata,
    PluginBundleInfo,
    PluginMetadata, // Alias for FlowNodeMetadata
};
use crate::services::flow::traits::{FlowNode, FlowNodeProvider, NodeOutput};
use async_trait::async_trait;
use base64::Engine;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::error::Error;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
#[cfg(feature = "extism")]
use tokio_util::sync::CancellationToken;

#[cfg(feature = "extism")]
const WASM_CALL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
#[cfg(feature = "extism")]
const WASM_LOCK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
#[cfg(feature = "extism")]
const WASM_MEMORY_PAGES: u32 = 1024; // 64 MiB
#[cfg(feature = "extism")]
const WASM_FUEL_LIMIT: u64 = 5_000_000_000;
#[cfg(feature = "extism")]
const WASM_MAX_INPUT_BYTES: usize = 1024 * 1024;
#[cfg(feature = "extism")]
const WASM_MAX_OUTPUT_BYTES: usize = 4 * 1024 * 1024;

// Add conditional compilation for extism
#[cfg(feature = "extism")]
use extism::*;

// Import our custom host function macro
#[cfg(feature = "extism")]
use vortideck_host_macro::vortideck_host;

// ===== THREAD-LOCAL PLUGIN CONTEXT =====
// Used to pass plugin identity to host functions for permission checking
use std::cell::RefCell;

thread_local! {
    /// Current plugin ID being executed (set before plugin.call(), cleared after)
    static CURRENT_PLUGIN_ID: RefCell<Option<String>> = RefCell::new(None);

    /// Permission checker reference (set once during PluginManager init)
    static PERMISSION_CHECKER: RefCell<Option<Arc<super::permissions::PermissionChecker>>> = RefCell::new(None);

    /// Platform context reference (set before plugin.call(), cleared after)
    static PLATFORM_CONTEXT: RefCell<Option<Arc<crate::core::PlatformContext>>> = RefCell::new(None);
}

/// Set the current plugin context before executing plugin code
fn set_plugin_context(
    plugin_id: &str,
    permission_checker: Arc<super::permissions::PermissionChecker>,
    platform_context: Arc<crate::core::PlatformContext>,
) {
    CURRENT_PLUGIN_ID.with(|id| {
        *id.borrow_mut() = Some(plugin_id.to_string());
    });
    PERMISSION_CHECKER.with(|checker| {
        *checker.borrow_mut() = Some(permission_checker);
    });
    PLATFORM_CONTEXT.with(|ctx| {
        *ctx.borrow_mut() = Some(platform_context);
    });
}

/// Clear the plugin context after execution
fn clear_plugin_context() {
    CURRENT_PLUGIN_ID.with(|id| {
        *id.borrow_mut() = None;
    });
    PLATFORM_CONTEXT.with(|ctx| {
        *ctx.borrow_mut() = None;
    });
    // Note: We keep PERMISSION_CHECKER set for reuse
}

#[cfg(feature = "extism")]
struct PluginContextGuard;

#[cfg(feature = "extism")]
impl Drop for PluginContextGuard {
    fn drop(&mut self) {
        clear_plugin_context();
    }
}

/// Cancels an already-started Extism call if its async owner is dropped (for
/// example by a workflow timeout or explicit run cancellation).
#[cfg(feature = "extism")]
struct ExtismCancellationGuard {
    handle: Option<extism::CancelHandle>,
}

#[cfg(feature = "extism")]
impl ExtismCancellationGuard {
    fn disarm(&mut self) {
        self.handle = None;
    }

    fn interrupt(&mut self) {
        if let Some(handle) = self.handle.take() {
            let _ = handle.cancel();
        }
    }
}

#[cfg(feature = "extism")]
impl Drop for ExtismCancellationGuard {
    fn drop(&mut self) {
        self.interrupt();
    }
}

#[cfg(feature = "extism")]
async fn call_extism_bounded(
    plugin: Arc<tokio::sync::Mutex<extism::Plugin>>,
    plugin_id: String,
    permission_checker: Arc<super::permissions::PermissionChecker>,
    platform_context: Arc<crate::core::PlatformContext>,
    function_name: String,
    input: String,
    external_cancellation: CancellationToken,
) -> Result<String, String> {
    if input.len() > WASM_MAX_INPUT_BYTES {
        return Err(format!(
            "WASM plugin input exceeds the {} byte limit",
            WASM_MAX_INPUT_BYTES
        ));
    }

    // Waiting is a distinct phase: dropping/timing out this future cannot later
    // acquire the mutex and start an effect.
    let plugin = tokio::time::timeout(WASM_LOCK_TIMEOUT, plugin.lock_owned())
        .await
        .map_err(|_| "WASM plugin is busy; no call was started".to_string())?;
    let mut cancellation = ExtismCancellationGuard {
        handle: Some(plugin.cancel_handle()),
    };

    let mut task = tokio::task::spawn_blocking(move || {
        let mut plugin = plugin;
        set_plugin_context(&plugin_id, permission_checker, platform_context);
        let _context = PluginContextGuard;
        plugin
            .call::<String, String>(&function_name, input)
            .map_err(|error| format!("WASM function call failed: {error}"))
    });

    enum CallCompletion<T> {
        Finished(T),
        TimedOut,
        Cancelled,
    }
    let completion = tokio::select! {
        joined = &mut task => CallCompletion::Finished(joined),
        _ = tokio::time::sleep(WASM_CALL_TIMEOUT) => CallCompletion::TimedOut,
        _ = external_cancellation.cancelled() => CallCompletion::Cancelled,
    };
    let result = match completion {
        CallCompletion::Finished(joined) => {
            joined.map_err(|error| format!("WASM worker failed: {error}"))?
        }
        reason @ (CallCompletion::TimedOut | CallCompletion::Cancelled) => {
            let externally_cancelled = matches!(reason, CallCompletion::Cancelled);
            cancellation.interrupt();
            return match tokio::time::timeout(std::time::Duration::from_secs(2), &mut task).await {
                Ok(_) if externally_cancelled => Err(
                    "WASM call was cancelled by plugin lifecycle shutdown and interrupted; host effects already started may be irreversible"
                        .to_string(),
                ),
                Ok(_) => Err(
                    "WASM call exceeded its execution budget and was interrupted; host effects already started may be irreversible"
                        .to_string(),
                ),
                Err(_) if externally_cancelled => Err(
                    "WASM lifecycle cancellation was requested but not confirmed; effect outcome is uncertain"
                        .to_string(),
                ),
                Err(_) => Err(
                    "WASM call interruption was requested but not confirmed; effect outcome is uncertain"
                        .to_string(),
                ),
            };
        }
    };
    cancellation.disarm();

    let output = result?;
    if output.len() > WASM_MAX_OUTPUT_BYTES {
        return Err(format!(
            "WASM plugin output exceeds the {} byte limit",
            WASM_MAX_OUTPUT_BYTES
        ));
    }
    Ok(output)
}

/// Get current plugin ID from thread-local storage
fn get_current_plugin_id() -> Result<String, String> {
    CURRENT_PLUGIN_ID.with(|id| {
        id.borrow()
            .clone()
            .ok_or_else(|| "No plugin context set".to_string())
    })
}

/// Get current PlatformContext from thread-local storage
fn get_platform_context() -> Result<Arc<crate::core::PlatformContext>, String> {
    PLATFORM_CONTEXT.with(|ctx| {
        ctx.borrow()
            .clone()
            .ok_or_else(|| "No platform context set".to_string())
    })
}

/// Check if current plugin has a specific permission
fn check_permission(permission: super::bundle::PluginPermission) -> Result<(), String> {
    let plugin_id = get_current_plugin_id()?;

    PERMISSION_CHECKER.with(|checker| {
        checker
            .borrow()
            .as_ref()
            .ok_or_else(|| "Permission checker not initialized".to_string())?
            .enforce_permission(&plugin_id, &permission)
    })
}

fn check_network_permission(url: &str) -> Result<(), String> {
    let plugin_id = get_current_plugin_id()?;
    PERMISSION_CHECKER.with(|checker| {
        checker
            .borrow()
            .as_ref()
            .ok_or_else(|| "Permission checker not initialized".to_string())?
            .enforce_network_url(&plugin_id, url)
    })
}

// ===== WASMTIME CACHE CONFIGURATION =====

/// Get or initialize the Wasmtime cache configuration path
/// Returns the path to wasmtime-config.toml for use with PluginBuilder
#[cfg(feature = "extism")]
fn get_cache_config_path() -> Option<std::path::PathBuf> {
    use std::sync::OnceLock;
    static CACHE_CONFIG_PATH: OnceLock<Option<std::path::PathBuf>> = OnceLock::new();

    CACHE_CONFIG_PATH
        .get_or_init(|| {
            // Use AppData\Local for cache (proper location for cache data)
            let appdata_local = dirs::data_local_dir()?;

            let vortideck_dir = appdata_local.join("VortiDeck");
            let cache_dir = vortideck_dir.join("cache");
            let cache_config_path = cache_dir.join("wasmtime-config.toml");

            // Create cache directory if it doesn't exist
            if !cache_dir.exists() {
                if let Err(e) = std::fs::create_dir_all(&cache_dir) {
                    println!("⚠️ Failed to create cache directory: {}, cache disabled", e);
                    return None;
                }
                println!("📁 Created Wasmtime cache directory: {:?}", cache_dir);
            }

            // Create the compiled cache directory structure
            let compiled_cache_dir = cache_dir.join("compiled");
            if !compiled_cache_dir.exists() {
                if let Err(e) = std::fs::create_dir_all(&compiled_cache_dir) {
                    println!(
                        "⚠️ Failed to create compiled cache directory: {}, cache disabled",
                        e
                    );
                    return None;
                }
                println!(
                    "📁 Created compiled cache directory: {:?}",
                    compiled_cache_dir
                );
            }

            // Create cache config file if it doesn't exist
            if !cache_config_path.exists() {
                // Format: Wasmtime 31.0.0+ removed the "enabled" key
                // Extism 1.13.0 uses Wasmtime 37.0.3, so we don't include "enabled = true"
                // See: https://docs.wasmtime.dev/cli-cache.html
                let config_content = format!(
                    "[cache]\ndirectory = \"{}\"\n",
                    compiled_cache_dir.to_str().unwrap().replace('\\', "/")
                );

                if let Err(e) = std::fs::write(&cache_config_path, config_content) {
                    println!("⚠️ Failed to write cache config: {}, cache disabled", e);
                    return None;
                }

                println!("📝 Created Wasmtime cache config: {:?}", cache_config_path);
                println!("   Cache directory: {:?}", compiled_cache_dir);
            }

            println!("💾 Using cache config: {:?}", cache_config_path);
            Some(cache_config_path)
        })
        .clone()
}

/// Create a plugin with consistent configuration (ensures same Engine hash for cache reuse)
/// Centralized function to guarantee both plugin loading paths use identical configuration
#[cfg(feature = "extism")]
fn create_plugin_with_cache(manifest: Manifest) -> Result<extism::Plugin, String> {
    create_plugin_with_limits(
        manifest,
        WASM_CALL_TIMEOUT,
        WASM_MEMORY_PAGES,
        WASM_FUEL_LIMIT,
    )
}

#[cfg(feature = "extism")]
fn create_plugin_with_limits(
    manifest: Manifest,
    timeout: std::time::Duration,
    memory_pages: u32,
    fuel_limit: u64,
) -> Result<extism::Plugin, String> {
    use extism::UserData;

    // Get all host functions (using macro-generated wrappers)
    let vortideck_log = vortideck_log_function();
    let vortideck_register_node = vortideck_register_node_function();
    let vortideck_get_context = vortideck_get_context_function();
    let vortideck_emit_progress = vortideck_emit_progress_function();
    let vortideck_emit_event = vortideck_emit_event_function();
    let vortideck_storage_get = vortideck_storage_get_function();
    let vortideck_storage_set = vortideck_storage_set_function();
    let vortideck_read_file = vortideck_read_file_function();
    let vortideck_write_file = vortideck_write_file_function();
    let vortideck_http_request = vortideck_http_request_function();

    // Build plugin with explicit cache configuration
    let manifest = manifest.with_timeout(timeout).with_memory_max(memory_pages);
    let mut builder = PluginBuilder::new(manifest)
        .with_wasi(true)
        .with_fuel_limit(fuel_limit);

    // // Add cache config if available (ensures consistent Engine configuration)
    if let Some(cache_config_path) = get_cache_config_path() {
        builder = builder.with_cache_config(cache_config_path);
    }

    // Build plugin with all host functions (order matters for consistency!)
    builder
        .with_functions([
            vortideck_log,
            vortideck_register_node,
            vortideck_get_context,
            vortideck_emit_progress,
            vortideck_emit_event,
            vortideck_storage_get,
            vortideck_storage_set,
            vortideck_read_file,
            vortideck_write_file,
            vortideck_http_request,
        ])
        .build()
        .map_err(|e| format!("Failed to create plugin: {}", e))
}

#[cfg(feature = "extism")]
fn compile_and_initialize_wasm(wasm: extism::Wasm) -> Result<(extism::Plugin, String), String> {
    let mut plugin = create_plugin_with_cache(extism::Manifest::new([wasm]))?;
    plugin
        .call::<_, ()>("register", "")
        .map_err(|error| format!("Failed to register plugin: {error}"))?;
    let metadata: String = plugin
        .call("get_metadata", "")
        .map_err(|error| format!("Failed to get plugin metadata: {error}"))?;
    if metadata.len() > WASM_MAX_OUTPUT_BYTES {
        return Err(format!(
            "Plugin metadata exceeds the {} byte limit",
            WASM_MAX_OUTPUT_BYTES
        ));
    }
    Ok((plugin, metadata))
}

// ===== HOST FUNCTIONS - Auto-generated TypeScript PDK =====
// These functions are callable from WASM plugins via Extism

/// Log a message from the plugin
#[cfg(feature = "extism")]
#[vortideck_host]
fn vortideck_log(message: String) -> Result<String, String> {
    println!("🔌 VortiDeck Plugin Log: {}", message);
    Ok("logged".to_string())
}

/// Register a flow node with VortiDeck system
#[cfg(feature = "extism")]
#[vortideck_host]
fn vortideck_register_node(metadata: String) -> Result<bool, String> {
    let _: serde_json::Value = serde_json::from_str(&metadata)
        .map_err(|error| format!("Invalid node metadata: {error}"))?;
    Err("Dynamic node registration is not supported; declare flow-node contributions in manifest.json".to_string())
}

/// Get the current VortiDeck execution context
#[cfg(feature = "extism")]
#[vortideck_host]
fn vortideck_get_context() -> Result<String, String> {
    Ok(serde_json::json!({
        "available": false,
        "flow_id": null,
        "node_id": null,
        "execution_id": null
    })
    .to_string())
}

/// Emit progress event during node execution
#[cfg(feature = "extism")]
#[vortideck_host]
fn vortideck_emit_progress(progress_json: String) -> Result<bool, String> {
    let payload: serde_json::Value = serde_json::from_str(&progress_json)
        .map_err(|error| format!("Invalid progress payload: {error}"))?;
    let package_id = super::bundle::canonical_package_id(&get_current_plugin_id()?).to_string();
    super::bundle::validate_package_id(&package_id)?;
    get_platform_context()?
        .event_bus
        .emit(crate::events::EventPayload::Plugin(
            crate::events::PluginEventPayload {
                package_id,
                event_type: "progress".to_string(),
                payload,
            },
        ))
        .map(|_| true)
        .map_err(|error| format!("Failed to emit plugin progress: {error}"))
}

fn plugin_event_from_untrusted_json(
    plugin_id: &str,
    event_json: &str,
) -> Result<crate::events::EventPayload, String> {
    if event_json.len() > 256 * 1024 {
        return Err("Plugin event payload exceeds 256 KiB".to_string());
    }
    let package_id = super::bundle::canonical_package_id(plugin_id).to_string();
    super::bundle::validate_package_id(&package_id)?;
    let document: Value = serde_json::from_str(event_json)
        .map_err(|error| format!("Invalid plugin event JSON: {error}"))?;
    let object = document
        .as_object()
        .ok_or_else(|| "Plugin event must be a JSON object".to_string())?;
    let event_type = object
        .get("eventType")
        .or_else(|| object.get("type"))
        .and_then(Value::as_str)
        .ok_or_else(|| "Plugin event requires an eventType string".to_string())?;
    if event_type.is_empty()
        || event_type.len() > 128
        || !event_type.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_')
        })
    {
        return Err("Plugin eventType contains unsupported characters or length".to_string());
    }
    let payload = object
        .get("payload")
        .or_else(|| object.get("data"))
        .cloned()
        .unwrap_or(Value::Null);
    Ok(crate::events::EventPayload::Plugin(
        crate::events::PluginEventPayload {
            package_id,
            event_type: event_type.to_string(),
            payload,
        },
    ))
}

/// Emit custom event from plugin
#[cfg(feature = "extism")]
#[vortideck_host]
fn vortideck_emit_event(event_json: String) -> Result<bool, String> {
    println!("📢 VortiDeck: Custom event: {}", event_json);

    let event = plugin_event_from_untrusted_json(&get_current_plugin_id()?, &event_json)?;

    let platform_context = get_platform_context()?;
    match platform_context.event_bus.emit(event) {
        Ok(_) => Ok(true),
        Err(e) => Err(format!("Failed to emit event: {}", e)),
    }
}

/// Get plugin-scoped storage value
#[cfg(feature = "extism")]
#[vortideck_host]
fn vortideck_storage_get(key: String) -> Result<String, String> {
    check_permission(super::bundle::PluginPermission::Storage)?;
    match std::fs::read_to_string(plugin_storage_path(&key)?) {
        Ok(value) => Ok(value),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(format!("Failed to read plugin storage: {error}")),
    }
}

/// Set plugin-scoped storage value
#[cfg(feature = "extism")]
#[vortideck_host]
fn vortideck_storage_set(key: String, value: String) -> Result<bool, String> {
    check_permission(super::bundle::PluginPermission::Storage)?;
    if value.len() > 1024 * 1024 {
        return Err("Plugin storage values are limited to 1 MiB".to_string());
    }
    let path = plugin_storage_path(&key)?;
    let parent = path
        .parent()
        .ok_or_else(|| "Plugin storage path has no parent".to_string())?;
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("Failed to create plugin storage: {error}"))?;
    let temporary = parent.join(format!(".{}.tmp", uuid::Uuid::new_v4()));
    std::fs::write(&temporary, value)
        .map_err(|error| format!("Failed to stage plugin storage: {error}"))?;
    std::fs::rename(&temporary, &path)
        .map_err(|error| format!("Failed to commit plugin storage: {error}"))?;
    Ok(true)
}

#[cfg(feature = "extism")]
fn plugin_storage_path(key: &str) -> Result<PathBuf, String> {
    use sha2::{Digest, Sha256};

    if key.is_empty() || key.len() > 512 {
        return Err("Plugin storage keys must contain 1-512 bytes".to_string());
    }
    let plugin_id = get_current_plugin_id()?;
    let package_id = super::bundle::canonical_package_id(&plugin_id);
    super::bundle::validate_package_id(package_id)?;
    let root = dirs::data_local_dir()
        .ok_or_else(|| "Failed to resolve local application data".to_string())?
        .join("VortiDeck")
        .join("plugin-storage")
        .join(package_id);
    Ok(root.join(format!(
        "{}.value",
        hex::encode(Sha256::digest(key.as_bytes()))
    )))
}

/// Read file from workspace (requires FileSystem permission)
#[cfg(feature = "extism")]
#[vortideck_host]
fn vortideck_read_file(path: String) -> Result<String, String> {
    println!("📖 VortiDeck: Read file: {}", path);

    // Check FileSystem permission
    check_permission(super::bundle::PluginPermission::FileSystem)?;

    // Use block_in_place to avoid blocking Tokio worker threads
    tokio::task::block_in_place(|| {
        let resolved = super::permissions::resolve_workspace_read_path(&path)?;
        std::fs::read_to_string(resolved).map_err(|e| format!("Failed to read file: {}", e))
    })
}

/// Write file to workspace (requires FileSystem permission)
#[cfg(feature = "extism")]
#[vortideck_host]
fn vortideck_write_file(path: String, content: String) -> Result<bool, String> {
    println!(
        "📝 VortiDeck: Write file: {} ({} bytes)",
        path,
        content.len()
    );

    // Check FileSystem permission
    check_permission(super::bundle::PluginPermission::FileSystem)?;

    // Use block_in_place to avoid blocking Tokio worker threads
    tokio::task::block_in_place(|| {
        let resolved = super::permissions::resolve_workspace_write_path(&path)?;
        std::fs::write(resolved, content).map_err(|e| format!("Failed to write file: {}", e))?;

        Ok(true)
    })
}

/// Make HTTP request (requires Network permission)
#[cfg(feature = "extism")]
#[vortideck_host]
fn vortideck_http_request(options_json: String) -> Result<String, String> {
    // Parse options
    let options: serde_json::Value =
        serde_json::from_str(&options_json).map_err(|e| format!("Invalid JSON: {}", e))?;

    let url = options
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'url' field")?;
    check_network_permission(url)?;

    let method = options
        .get("method")
        .and_then(|v| v.as_str())
        .unwrap_or("GET");

    let method = reqwest::Method::from_bytes(method.as_bytes())
        .map_err(|error| format!("Invalid HTTP method: {error}"))?;
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| format!("Failed to create plugin HTTP client: {error}"))?;
    let mut request = client.request(method, url);
    if let Some(headers) = options
        .get("headers")
        .and_then(serde_json::Value::as_object)
    {
        for (name, value) in headers {
            let value = value
                .as_str()
                .ok_or_else(|| format!("HTTP header '{name}' must be a string"))?;
            request = request.header(name, value);
        }
    }
    if let Some(body) = options.get("body") {
        request = if let Some(text) = body.as_str() {
            request.body(text.to_owned())
        } else {
            request.json(body)
        };
    }
    let response = request
        .send()
        .map_err(|error| format!("Plugin HTTP request failed: {error}"))?;
    if response
        .content_length()
        .is_some_and(|length| length > 4 * 1024 * 1024)
    {
        return Err("Plugin HTTP responses are limited to 4 MiB".to_string());
    }
    let status = response.status().as_u16();
    let headers = response
        .headers()
        .iter()
        .map(|(name, value)| {
            (
                name.as_str().to_string(),
                serde_json::Value::String(value.to_str().unwrap_or_default().to_string()),
            )
        })
        .collect::<serde_json::Map<String, serde_json::Value>>();
    let bytes = response
        .bytes()
        .map_err(|error| format!("Failed to read plugin HTTP response: {error}"))?;
    if bytes.len() > 4 * 1024 * 1024 {
        return Err("Plugin HTTP responses are limited to 4 MiB".to_string());
    }
    Ok(serde_json::json!({
        "status": status,
        "headers": headers,
        "body": String::from_utf8_lossy(&bytes),
    })
    .to_string())
}

/// Trait for flow registry integration to enable headless architecture
pub trait FlowRegistryProvider: Send + Sync {
    /// Replace the complete plugin-owned catalog atomically. Runtime package
    /// lifecycle is the authority; keeping incremental registry ownership here
    /// previously left stale nodes after unload and reconfiguration.
    fn replace_plugin_providers(&self, providers: Vec<Box<dyn FlowNodeProvider>>);
    fn get_node_count(&self) -> usize;
}

/// Manages plugins with full WASM execution and flow integration
pub struct PluginManager {
    /// Registered plugins
    plugins: Arc<RwLock<HashMap<String, Arc<dyn Plugin>>>>,
    /// WASM plugins with flow node capability
    wasm_plugins: Arc<RwLock<Vec<Arc<WasmPluginWrapper>>>>,
    /// Service plugins with flow node capability
    service_plugins: Arc<RwLock<Vec<Arc<super::service_plugin::ServicePlugin>>>>,
    /// Flow registry for automatic plugin-to-flow integration (headless architecture)
    flow_registry: Arc<Mutex<Option<Arc<dyn FlowRegistryProvider>>>>,
    /// Web extensions storage (plugin_id -> WebResources)
    web_extensions: Arc<RwLock<HashMap<String, super::bundle::WebResources>>>,
    /// Plugin bundle paths (plugin_id -> bundle_path) for hot reload
    plugin_bundles: Arc<RwLock<HashMap<String, PathBuf>>>,
    /// Folder-loaded plugin manifests keyed by normalized plugin id — the source
    /// for legacy `permissions` + typed `contributions` in the unified package
    /// list. Populated in `load_plugin_from_folder` (common entry for wasm/service).
    package_manifests: Arc<RwLock<HashMap<String, super::bundle::PluginManifest>>>,
    /// Available widget packages (scanned from folders; widget contributions),
    /// stored as `PackageInfo` (the single package type). Listed via
    /// `list_available_widgets` and merged into `list_packages`.
    available_widgets: Arc<RwLock<Vec<super::contributions::PackageInfo>>>,
    /// Fail-closed discovery errors, surfaced to the package UI instead of
    /// silently hiding malformed folders.
    package_discovery_errors: Arc<RwLock<Vec<super::contributions::PackageDiscoveryError>>>,
    /// Permission checker for sandboxed web extensions
    permission_checker: Arc<super::permissions::PermissionChecker>,
    /// Platform context for EventBus access in WasmPluginWrapper
    platform_context: Arc<crate::core::PlatformContext>,
}

impl PluginManager {
    /// Construct an isolated manager only for unit tests. Production owns one
    /// context-backed manager through `VortiDeckCore`.
    #[cfg(test)]
    pub fn new() -> Self {
        let platform_context = Arc::new(crate::core::PlatformContext::new(
            Arc::new(crate::core::NoopStorageProvider),
            Arc::new(crate::core::NoopHttpClient),
            Arc::new(crate::core::NoopSystemInterface),
            None,
        ));
        Self::new_with_context(platform_context)
    }

    /// Create a new plugin manager with a specific PlatformContext
    pub fn new_with_context(platform_context: Arc<crate::core::PlatformContext>) -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            wasm_plugins: Arc::new(RwLock::new(Vec::new())),
            service_plugins: Arc::new(RwLock::new(Vec::new())),
            flow_registry: Arc::new(Mutex::new(None)),
            web_extensions: Arc::new(RwLock::new(HashMap::new())),
            plugin_bundles: Arc::new(RwLock::new(HashMap::new())),
            package_manifests: Arc::new(RwLock::new(HashMap::new())),
            available_widgets: Arc::new(RwLock::new(vec![
                super::contributions::builtin_overlay_package(),
            ])),
            package_discovery_errors: Arc::new(RwLock::new(Vec::new())),
            permission_checker: Arc::new(super::permissions::PermissionChecker::new()),
            platform_context,
        }
    }

    /// Register a widget package as available (scanned from a folder). Dedups by
    /// id — replaces on re-scan / hot-reload, so no duplication.
    pub async fn register_available_widget(&self, widget: super::contributions::PackageInfo) {
        let mut widgets = self.available_widgets.write().await;
        if let Some(existing) = widgets.iter_mut().find(|w| w.id == widget.id) {
            if existing.trust == "builtin" && widget.trust != "builtin" {
                log::warn!(
                    "Ignoring package '{}' because its id is reserved by an embedded builtin",
                    widget.id
                );
                return;
            }
            *existing = widget;
        } else {
            widgets.push(widget);
        }
    }

    /// List all available widget packages (for the editor's widget library).
    pub async fn list_available_widgets(&self) -> Vec<super::contributions::PackageInfo> {
        self.available_widgets
            .read()
            .await
            .iter()
            .filter(|package| {
                package.contributions.iter().any(|contribution| {
                    matches!(
                        contribution,
                        super::contributions::Contribution::Widget { .. }
                    )
                })
            })
            .cloned()
            .collect()
    }

    pub async fn list_discovered_packages(&self) -> Vec<super::contributions::PackageInfo> {
        self.available_widgets.read().await.clone()
    }

    pub async fn list_package_discovery_errors(
        &self,
    ) -> Vec<super::contributions::PackageDiscoveryError> {
        self.package_discovery_errors.read().await.clone()
    }

    /// Remove the non-executable discovery projection for a package whose
    /// files have been transactionally removed. Runtime unloading is separate
    /// because awaiting-consent packages must remain discoverable.
    pub async fn remove_discovered_package(&self, package_id: &str) {
        let canonical = Self::canonical_package_id(package_id).to_string();
        self.package_manifests
            .write()
            .await
            .retain(|id, _| Self::canonical_package_id(id) != canonical);
        self.available_widgets.write().await.retain(|package| {
            package.trust == "builtin" || Self::canonical_package_id(&package.id) != canonical
        });
    }

    pub async fn package_manifest(
        &self,
        package_id: &str,
    ) -> Option<super::bundle::PluginManifest> {
        let normalized = Self::normalize_plugin_id(package_id);
        let manifests = self.package_manifests.read().await;
        manifests
            .get(&normalized)
            .or_else(|| manifests.get(package_id))
            .cloned()
    }

    pub async fn is_package_runtime_loaded(&self, package_id: &str) -> bool {
        let canonical = Self::canonical_package_id(package_id);
        if self.wasm_plugins.read().await.iter().any(|plugin| {
            plugin
                .plugin_bundle
                .as_ref()
                .map(|bundle| Self::canonical_package_id(&bundle.id) == canonical)
                .unwrap_or_else(|| Self::canonical_package_id(&plugin.id) == canonical)
        }) {
            return true;
        }
        if self
            .service_plugins
            .read()
            .await
            .iter()
            .any(|plugin| Self::canonical_package_id(plugin.id()) == canonical)
        {
            return true;
        }
        self.plugins
            .read()
            .await
            .keys()
            .any(|loaded_id| Self::canonical_package_id(loaded_id) == canonical)
    }

    /// Unified package list for the "My Plugins" surface: runtime plugins
    /// (`self.plugins`, via `get_plugin_bundles`) merged with available widget
    /// packages (`available_widgets`). `granted_widget_ids` is the set of widget
    /// ids with a stored durable package grant and drives
    /// `is_granted` for widget packages; runtime plugins are loaded == granted.
    ///
    /// Runtime and widget scans are merged by canonical package ID, so a bundle
    /// appears once even when the runtime registry uses the `plugin.` prefix.
    pub async fn list_packages(
        &self,
        granted_widget_ids: &std::collections::HashSet<String>,
    ) -> Vec<super::contributions::PackageInfo> {
        use super::contributions::PackageInfo;

        let mut out: Vec<PackageInfo> = Vec::new();

        // Runtime plugins (loaded WASM/service bundles) — host-trusted, granted.
        // Enriched with legacy permissions + typed contributions from the cached
        // manifest (package_manifests), keyed by the bundle id get_plugin_bundles
        // emits (normalize_plugin_id). icon from the bundle metadata.
        let manifests = self.package_manifests.read().await;
        for m in self.get_plugin_bundles().await {
            let (permissions, contributions) = manifests
                .get(&m.id)
                .map(|mf| (mf.permissions.clone(), mf.contributions.clone()))
                .unwrap_or_default();
            out.push(PackageInfo {
                id: m.id.clone(),
                name: m.name.clone(),
                version: m.version.clone(),
                author: Some(m.author.clone()),
                description: Some(m.description.clone()),
                icon_path: m
                    .icon_path
                    .as_ref()
                    .map(|p| p.to_string_lossy().to_string()),
                // Trust by verification: a runtime plugin is "builtin" only if it's
                // a known first-party package. Folder-loaded (user-installed) runtime
                // plugins are community — ed25519 signature verification (Phase 2) is
                // the robust path; this id-check covers the first-party manifest.
                trust: if super::integrity::is_known_builtin(&m.id) {
                    "builtin".to_string()
                } else {
                    "community".to_string()
                },
                contributions,
                permissions,
                has_runtime: true,
                runtime_active: true,
                is_granted: true,
                needs_reconsent: false,
            });
        }

        // Every discovered manifest is visible even if its runtime is not active.
        for package in self.list_discovered_packages().await {
            Self::merge_widget_package_view(&mut out, package, granted_widget_ids);
        }

        out
    }

    /// Normalize plugin ID (ensure "plugin." prefix)
    /// This is a single source of truth for plugin ID normalization
    fn normalize_plugin_id(id: &str) -> String {
        if id.starts_with("plugin.") {
            id.to_string()
        } else {
            format!("plugin.{}", id)
        }
    }

    fn canonical_package_id(id: &str) -> &str {
        super::bundle::canonical_package_id(id)
    }

    fn merge_widget_package_view(
        packages: &mut Vec<super::contributions::PackageInfo>,
        mut widget: super::contributions::PackageInfo,
        granted_widget_ids: &std::collections::HashSet<String>,
    ) {
        let canonical_widget_id = Self::canonical_package_id(&widget.id);
        widget.is_granted = granted_widget_ids
            .iter()
            .any(|granted| Self::canonical_package_id(granted) == canonical_widget_id);
        if let Some(runtime) = packages
            .iter_mut()
            .find(|candidate| Self::canonical_package_id(&candidate.id) == canonical_widget_id)
        {
            // The manifest/package ID is the stable public identity used by
            // existing widget grants and placed overlay elements. Runtime
            // execution normalizes this ID at its own boundary.
            runtime.id = widget.id;
            runtime.name = widget.name;
            runtime.version = widget.version;
            runtime.author = widget.author.or(runtime.author.take());
            runtime.description = widget.description.or(runtime.description.take());
            runtime.icon_path = widget.icon_path.or(runtime.icon_path.take());
            runtime.trust = widget.trust;
            runtime.permissions = widget.permissions;
            runtime.contributions = widget.contributions;
            runtime.has_runtime |= widget.has_runtime;
            runtime.is_granted |= widget.is_granted;
            runtime.needs_reconsent |= widget.needs_reconsent;
        } else {
            packages.push(widget);
        }
    }

    /// Get permission checker for API bridge
    pub fn get_permission_checker(&self) -> Arc<super::permissions::PermissionChecker> {
        self.permission_checker.clone()
    }

    /// Register a plugin
    pub async fn register_plugin(
        &self,
        plugin_id: String,
        plugin: Box<dyn Plugin>,
    ) -> Result<(), PluginError> {
        if self.plugins.read().await.contains_key(&plugin_id) {
            return Err(PluginError::Error(format!(
                "Plugin already registered: {}",
                plugin_id
            )));
        }

        let init_start = std::time::Instant::now();
        plugin
            .initialize()
            .await
            .map_err(|e| PluginError::Error(format!("Initialization failed: {}", e)))?;
        println!("    ⏱️ Plugin initialize(): {:.2?}", init_start.elapsed());

        let plugin: Arc<dyn Plugin> = Arc::from(plugin);
        let lock_start = std::time::Instant::now();
        let mut plugins = self.plugins.write().await;
        println!(
            "    ⏱️ Acquired plugins write lock: {:.2?}",
            lock_start.elapsed()
        );
        if plugins.contains_key(&plugin_id) {
            drop(plugins);
            let _ = plugin.shutdown().await;
            return Err(PluginError::Error(format!(
                "Plugin already registered: {}",
                plugin_id
            )));
        }
        plugins.insert(plugin_id, plugin);
        Ok(())
    }

    /// Shut down one runtime registry entry. Package callers must use
    /// `unload_package` so every catalog and grant is pruned together.
    async fn unregister_runtime_plugin(&self, plugin_id: &str) -> Result<(), PluginError> {
        let plugin = self
            .plugins
            .read()
            .await
            .get(plugin_id)
            .cloned()
            .ok_or_else(|| PluginError::NotFound(plugin_id.to_string()))?;

        plugin
            .shutdown()
            .await
            .map_err(|e| PluginError::Error(format!("Shutdown failed: {}", e)))?;
        self.plugins.write().await.remove(plugin_id);
        Ok(())
    }

    /// Get a plugin by ID - returns reference for read-only access
    pub async fn get_plugin_metadata(&self, plugin_id: &str) -> Option<PluginMetadata> {
        let plugins = self.plugins.read().await;
        plugins.get(plugin_id).map(|plugin| plugin.metadata())
    }

    /// Execute a plugin action safely
    pub async fn execute_plugin(
        &self,
        plugin_id: &str,
        action: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        // WASM nodes are also published in the generic registry for flow
        // discovery, so executable runtimes must be resolved first.
        let wasm_plugin = {
            let wasm_plugins = self.wasm_plugins.read().await;
            wasm_plugins
                .iter()
                .find(|wasm_plugin| wasm_plugin.id == plugin_id)
                .cloned()
        };
        if let Some(wasm_plugin) = wasm_plugin {
            return wasm_plugin.execute(params).await;
        }

        // Generic/service contributions can be discovered but do not expose
        // the WASM action entrypoint.
        {
            let plugins = self.plugins.read().await;
            if plugins.contains_key(plugin_id) {
                return Err(format!(
                    "Plugin '{}' does not expose executable actions through this runtime (requested '{}')",
                    plugin_id, action
                ));
            }
        }

        Err(format!("Plugin '{}' not found", plugin_id))
    }

    /// Check if a plugin exists
    pub async fn has_plugin(&self, plugin_id: &str) -> bool {
        let plugins = self.plugins.read().await;
        plugins.contains_key(plugin_id)
    }

    /// List all registered plugins
    pub async fn list_plugins(&self) -> Vec<String> {
        let plugins = self.plugins.read().await;
        plugins.keys().cloned().collect()
    }

    /// Get plugin count
    pub async fn get_plugin_count(&self) -> usize {
        let plugins = self.plugins.read().await;
        plugins.len()
    }

    /// Count executable package runtimes, not individual contributed nodes.
    /// A seven-node WASM bundle consumes one package slot.
    pub async fn get_loaded_package_count(&self) -> usize {
        let mut package_ids = std::collections::BTreeSet::new();
        for plugin in self.wasm_plugins.read().await.iter() {
            let id = plugin
                .plugin_bundle
                .as_ref()
                .map(|bundle| bundle.id.as_str())
                .unwrap_or(&plugin.id);
            package_ids.insert(Self::canonical_package_id(id).to_string());
        }
        for plugin in self.service_plugins.read().await.iter() {
            package_ids.insert(Self::canonical_package_id(plugin.id()).to_string());
        }
        package_ids.len()
    }

    /// Set flow registry for automatic plugin-to-flow integration (headless architecture)
    pub async fn set_flow_registry(&self, registry: Arc<dyn FlowRegistryProvider>) {
        {
            let mut flow_registry = self.flow_registry.lock().await;
            *flow_registry = Some(registry);
        }

        // Auto-register all existing plugins with the flow system
        self.refresh_flow_registration().await;
        println!("🔌 Flow registry connected to PluginManager - plugins will auto-register");
    }

    /// Refresh flow registration for all plugins (internal method)
    async fn refresh_flow_registration(&self) {
        let wasm_plugins = self.wasm_plugins.read().await.clone();
        let service_plugins = self.service_plugins.read().await.clone();
        let mut providers: Vec<Box<dyn FlowNodeProvider>> = Vec::new();
        if !wasm_plugins.is_empty() {
            providers.push(Box::new(WasmFlowNodeProvider::new(wasm_plugins.clone())));
        }
        for plugin in &service_plugins {
            providers.push(Box::new(ServiceFlowNodeProvider {
                plugin: plugin.clone(),
                nodes: plugin.get_nodes().await,
            }));
        }

        let flow_registry = self.flow_registry.lock().await;
        if let Some(registry) = flow_registry.as_ref() {
            registry.replace_plugin_providers(providers);
            println!(
                "🔌 Replaced flow catalog from {} WASM and {} service package runtime(s)",
                wasm_plugins.len(),
                service_plugins.len()
            );
            println!(
                "🎯 Total flow nodes registered: {}",
                registry.get_node_count()
            );
        }
    }

    /// Get plugin metadata for all plugins (includes all individual nodes for flow system)
    pub async fn get_all_plugin_metadata(&self) -> Vec<PluginMetadata> {
        let plugins = self.plugins.read().await;
        let mut metadata_list = Vec::new();

        for plugin in plugins.values() {
            metadata_list.push(plugin.metadata());
        }

        metadata_list
    }

    /// Get plugin bundles for marketplace display (groups multi-node plugins into single entries)
    pub async fn get_plugin_bundles(&self) -> Vec<PluginMetadata> {
        use std::collections::HashMap;

        let all_metadata = self.get_all_plugin_metadata().await;
        let mut seen_bundles: HashMap<String, PluginMetadata> = HashMap::new();

        for metadata in all_metadata {
            if let Some(ref bundle) = metadata.plugin_bundle {
                // This is a node in a multi-node plugin
                // Only keep the first node we encounter for this bundle
                if !seen_bundles.contains_key(&bundle.id) {
                    // Create a bundle-level metadata entry
                    let bundle_metadata = PluginMetadata {
                        id: bundle.id.clone(),
                        name: bundle.name.clone(),
                        description: format!("{} (multi-node plugin)", metadata.description),
                        version: bundle.version.clone(),
                        author: bundle.author.clone(),
                        icon_path: metadata.icon_path.clone(),
                        plugin_bundle: None, // Bundle itself has no parent bundle
                        category: metadata.category.clone(),
                        parameters_schema: metadata.parameters_schema.clone(),
                        output_schema: metadata.output_schema.clone(),
                        type_color: metadata.type_color.clone(),
                    };
                    seen_bundles.insert(bundle.id.clone(), bundle_metadata);
                }
                // Skip other nodes from this bundle
            } else {
                // This is a standalone single-node plugin
                seen_bundles.insert(metadata.id.clone(), metadata);
            }
        }

        seen_bundles.into_values().collect()
    }

    /// Load a plugin from WASM bytes with full functionality
    #[cfg(feature = "extism")]
    pub async fn load_plugin_from_bytes(
        &self,
        wasm_bytes: &[u8],
        file_path: Option<PathBuf>,
        file_size: Option<u64>,
        permissions: Option<&Vec<super::bundle::PluginPermission>>,
    ) -> Result<String, String> {
        let total_start = std::time::Instant::now();

        println!("  🔨 Starting bounded Extism compilation and initialization...");
        let wasm = if let Some(ref path) = file_path {
            println!("  📁 Loading from file (caching enabled): {:?}", path);
            Wasm::file(path)
        } else {
            println!("  💾 Loading from memory (no caching)");
            Wasm::data(wasm_bytes.to_vec())
        };
        let plugin_start = std::time::Instant::now();
        let (plugin, metadata_json) =
            tokio::task::spawn_blocking(move || compile_and_initialize_wasm(wasm))
                .await
                .map_err(|error| format!("WASM initialization worker failed: {error}"))??;
        println!(
            "  ⏱️ Extism compilation and initialization: {:.2?}",
            plugin_start.elapsed()
        );

        let metadata: serde_json::Value = serde_json::from_str(&metadata_json)
            .map_err(|e| format!("Failed to parse plugin metadata: {}", e))?;

        // Check if this is new multi-node format or old single-node format
        let (
            plugin_id,
            plugin_name,
            description,
            version,
            author,
            category,
            category_display_name,
            subcategory,
            type_color,
            output_schema,
            parameters_schema,
        ) = if let Some(plugin_info) = metadata.get("plugin") {
            // New multi-node format: { plugin: {...}, nodes: [...] }
            let nodes = metadata.get("nodes").and_then(|v| v.as_array());

            // Use plugin-level info and first node's metadata
            let first_node = nodes.and_then(|arr| arr.get(0));

            let plugin_id = plugin_info
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let plugin_name = plugin_info
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown Plugin")
                .to_string();
            let version = plugin_info
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("1.0.0")
                .to_string();
            let author = plugin_info
                .get("author")
                .and_then(|v| v.as_str())
                .unwrap_or("Plugin Author")
                .to_string();

            // Get node-specific fields from first node (for backwards compatibility)
            let description = first_node
                .and_then(|n| n.get("description"))
                .and_then(|v| v.as_str())
                .unwrap_or("Multi-node plugin")
                .to_string();
            let category = first_node
                .and_then(|n| n.get("category"))
                .and_then(|v| v.as_str())
                .unwrap_or("PLUGINS")
                .to_string();
            let category_display_name = first_node
                .and_then(|n| n.get("category_display_name"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let subcategory = first_node
                .and_then(|n| n.get("subcategory"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let type_color = first_node
                .and_then(|n| n.get("type_color"))
                .and_then(|v| v.as_str())
                .unwrap_or("#666666")
                .to_string();
            let output_schema = first_node.and_then(|n| n.get("output_schema")).cloned(); // Optional output schema
            let parameters_schema = first_node
                .and_then(|n| n.get("parameters_schema"))
                .cloned()
                .unwrap_or(serde_json::json!({}));

            (
                plugin_id,
                plugin_name,
                description,
                version,
                author,
                category,
                category_display_name,
                subcategory,
                type_color,
                output_schema,
                parameters_schema,
            )
        } else {
            // Old single-node format (backwards compatibility)
            let plugin_id = metadata
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let plugin_name = metadata
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown Plugin")
                .to_string();
            let description = metadata
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let version = metadata
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("1.0.0")
                .to_string();
            let author = metadata
                .get("author")
                .and_then(|v| v.as_str())
                .unwrap_or("Plugin Author")
                .to_string();
            let category = metadata
                .get("category")
                .and_then(|v| v.as_str())
                .unwrap_or("PLUGINS")
                .to_string();
            let category_display_name = metadata
                .get("category_display_name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let subcategory = metadata
                .get("subcategory")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let type_color = metadata
                .get("type_color")
                .and_then(|v| v.as_str())
                .unwrap_or("#666666")
                .to_string();
            let output_schema = metadata.get("output_schema").cloned(); // Optional output schema
            let parameters_schema = metadata
                .get("parameters_schema")
                .cloned()
                .unwrap_or(serde_json::json!({}));

            (
                plugin_id,
                plugin_name,
                description,
                version,
                author,
                category,
                category_display_name,
                subcategory,
                type_color,
                output_schema,
                parameters_schema,
            )
        };

        // Check if this is multi-node format
        let nodes_array = metadata.get("nodes").and_then(|v| v.as_array());

        let loaded_id = if let Some(nodes) = nodes_array {
            // Multi-node plugin - create separate flow node for each
            println!(
                "🔌 Multi-node plugin detected: {} with {} nodes",
                plugin_name,
                nodes.len()
            );

            let plugin_arc = Arc::new(tokio::sync::Mutex::new(plugin));

            for node in nodes {
                let node_id = node
                    .get("_node_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let node_metadata_id = node.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
                let node_name = node
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown Node")
                    .to_string();
                let node_description = node
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let node_category = node
                    .get("category")
                    .and_then(|v| v.as_str())
                    .unwrap_or("PLUGINS")
                    .to_string();
                let node_category_display_name = node
                    .get("category_display_name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let node_subcategory = node
                    .get("subcategory")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let node_type_color = node
                    .get("type_color")
                    .and_then(|v| v.as_str())
                    .unwrap_or("#666666")
                    .to_string();
                let node_output_schema = node.get("output_schema").cloned(); // Optional output schema
                let node_parameters_schema = node
                    .get("parameters_schema")
                    .cloned()
                    .unwrap_or(serde_json::json!({}));

                let node_prefixed_id = Self::normalize_plugin_id(node_metadata_id);

                println!(
                    "  📦 Creating flow node: {} ({})",
                    node_name, node_prefixed_id
                );

                let wasm_wrapper = Arc::new(WasmPluginWrapper {
                    id: node_prefixed_id.clone(),
                    name: node_name.clone(),
                    description: node_description,
                    version: version.clone(),
                    author: author.clone(),
                    category: node_category.clone(),
                    category_display_name: node_category_display_name,
                    subcategory: node_subcategory,
                    type_color: node_type_color,
                    output_schema: node_output_schema,
                    parameters_schema: node_parameters_schema,
                    enabled: Arc::new(tokio::sync::RwLock::new(true)),
                    #[cfg(feature = "extism")]
                    execution_cancellation: Arc::new(tokio::sync::RwLock::new(
                        CancellationToken::new(),
                    )),
                    plugin: plugin_arc.clone(),
                    file_path: file_path.clone(),
                    file_size,
                    icon_path: None, // Standalone WASM files don't have web/assets folder structure
                    node_id: Some(node_id.to_string()),
                    plugin_bundle: Some(PluginBundleInfo {
                        id: Self::normalize_plugin_id(plugin_id),
                        name: plugin_name.clone(),
                        version: version.clone(),
                        author: author.clone(),
                    }),
                    permission_checker: self.permission_checker.clone(),
                    platform_context: self.platform_context.clone(),
                });

                // Store in WASM plugins collection
                {
                    let mut wasm_plugins = self.wasm_plugins.write().await;
                    wasm_plugins.push(wasm_wrapper.clone());
                }

                // Register as separate flow node
                let generic_plugin = Box::new(ExtismPluginAdapter {
                    inner: wasm_wrapper,
                });

                self.register_plugin(node_prefixed_id.clone(), generic_plugin)
                    .await
                    .map_err(|e| format!("Failed to register node {}: {}", node_name, e))?;
            }

            println!(
                "✅ Multi-node plugin registered: {} ({} nodes)",
                plugin_name,
                nodes.len()
            );

            // Return the first node's ID for compatibility
            let first_node_id = nodes
                .get(0)
                .and_then(|n| n.get("id"))
                .and_then(|v| v.as_str())
                .unwrap_or(plugin_id);
            Self::normalize_plugin_id(first_node_id)
        } else {
            // Single-node plugin (backwards compatibility)
            let prefixed_id = Self::normalize_plugin_id(plugin_id);

            println!(
                "🔌 Successfully loaded WASM plugin: {} ({})",
                plugin_name, prefixed_id
            );
            if let Some(display_name) = &category_display_name {
                println!("  📂 Category: {} ({})", category, display_name);
            } else {
                println!("  📂 Category: {}", category);
            }

            // Create WASM plugin wrapper with real execution capability
            let wasm_wrapper = Arc::new(WasmPluginWrapper {
                id: prefixed_id.clone(),
                name: plugin_name.clone(),
                description,
                version: version.clone(),
                author: author.clone(),
                category: category.clone(),
                category_display_name,
                subcategory,
                type_color,
                output_schema,
                parameters_schema,
                enabled: Arc::new(tokio::sync::RwLock::new(true)),
                #[cfg(feature = "extism")]
                execution_cancellation: Arc::new(
                    tokio::sync::RwLock::new(CancellationToken::new()),
                ),
                plugin: Arc::new(tokio::sync::Mutex::new(plugin)),
                file_path: file_path.clone(),
                file_size,
                icon_path: None, // Standalone WASM files don't have web/assets folder structure
                node_id: None,   // Single-node plugins don't need node_id
                plugin_bundle: None, // Single-node plugins are standalone
                permission_checker: self.permission_checker.clone(),
                platform_context: self.platform_context.clone(),
            });

            // Store in WASM plugins collection for flow integration
            {
                let lock_start = std::time::Instant::now();
                let mut wasm_plugins = self.wasm_plugins.write().await;
                println!(
                    "  ⏱️ Acquired wasm_plugins write lock: {:.2?}",
                    lock_start.elapsed()
                );
                wasm_plugins.push(wasm_wrapper.clone());
            }

            // Create the generic plugin interface
            let generic_plugin = Box::new(ExtismPluginAdapter {
                inner: wasm_wrapper,
            });

            // Register the plugin
            let register_start = std::time::Instant::now();
            self.register_plugin(prefixed_id.clone(), generic_plugin)
                .await
                .map_err(|e| format!("Failed to register plugin: {}", e))?;
            println!(
                "  ⏱️ Plugin registration (total): {:.2?}",
                register_start.elapsed()
            );

            println!(
                "  ⏱️ TOTAL load_plugin_from_bytes: {:.2?}",
                total_start.elapsed()
            );

            prefixed_id
        };
        self.refresh_flow_registration().await;
        Ok(loaded_id)
    }

    /// Discover and validate a package without executing its runtime or granting
    /// permissions. Discovery is unconditional; consent and subscription limits
    /// are activation concerns.
    pub async fn discover_plugin_from_folder<P: AsRef<Path>>(
        &self,
        plugin_folder: P,
    ) -> Result<super::bundle::PluginManifest, String> {
        let plugin_folder = plugin_folder.as_ref().to_path_buf();
        let result = self.discover_plugin_from_folder_inner(&plugin_folder).await;
        let folder = plugin_folder.to_string_lossy().to_string();
        let mut errors = self.package_discovery_errors.write().await;
        errors.retain(|entry| entry.folder != folder);
        if let Err(error) = &result {
            errors.push(super::contributions::PackageDiscoveryError {
                folder,
                manifest_path: plugin_folder
                    .join("manifest.json")
                    .to_string_lossy()
                    .to_string(),
                error: error.clone(),
            });
        }
        result
    }

    async fn discover_plugin_from_folder_inner(
        &self,
        plugin_folder: &Path,
    ) -> Result<super::bundle::PluginManifest, String> {
        let manifest = self.validate_plugin_folder(plugin_folder).await?;

        let normalized_id = Self::normalize_plugin_id(&manifest.id);
        self.package_manifests
            .write()
            .await
            .insert(normalized_id, manifest.clone());

        let has_runtime = plugin_folder.join(manifest.get_wasm_path()).exists()
            || manifest.get_service_config().is_some();
        let widget_content = manifest
            .contributions
            .iter()
            .filter_map(|contribution| match contribution {
                super::contributions::Contribution::Widget { content, .. } => {
                    Some(content.as_str())
                }
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n<!-- vortideck-widget-boundary -->\n");
        let trust = if !widget_content.is_empty()
            && super::integrity::verify_builtin_hash(&manifest.id, &widget_content)
        {
            "builtin"
        } else {
            "community"
        };
        self.register_available_widget(super::contributions::PackageInfo {
            id: manifest.id.clone(),
            name: manifest.name.clone(),
            version: manifest.version.clone(),
            author: (!manifest.author.is_empty()).then(|| manifest.author.clone()),
            description: (!manifest.description.is_empty()).then(|| manifest.description.clone()),
            icon_path: None,
            trust: trust.to_string(),
            contributions: manifest.contributions.clone(),
            permissions: manifest.permissions.clone(),
            has_runtime,
            runtime_active: false,
            is_granted: false,
            needs_reconsent: false,
        })
        .await;

        Ok(manifest)
    }

    /// Parse and validate a package folder without changing any discovery,
    /// widget, permission, or runtime registry. Update candidates use this
    /// before their atomic filesystem commit.
    pub async fn validate_plugin_folder<P: AsRef<Path>>(
        &self,
        plugin_folder: P,
    ) -> Result<super::bundle::PluginManifest, String> {
        let plugin_folder = plugin_folder.as_ref();
        let manifest_path = plugin_folder.join("manifest.json");
        let manifest_json = tokio::fs::read_to_string(&manifest_path)
            .await
            .map_err(|e| format!("Failed to read manifest: {}", e))?;
        let manifest: super::bundle::PluginManifest = serde_json::from_str(&manifest_json)
            .map_err(|e| format!("Failed to parse manifest: {}", e))?;
        super::bundle::validate_manifest_identity_and_host(&manifest)?;

        use super::bundle::{
            check_schema_compatibility, SchemaCompatibility, CURRENT_SCHEMA_VERSION,
        };
        match check_schema_compatibility(&manifest.schema_version) {
            SchemaCompatibility::Compatible => {}
            SchemaCompatibility::NewerMinor => log::warn!(
                "Package '{}' schema {} is newer than host {}",
                manifest.id,
                manifest.schema_version,
                CURRENT_SCHEMA_VERSION
            ),
            SchemaCompatibility::Incompatible {
                plugin_version,
                host_version,
            } => {
                return Err(format!(
                    "Incompatible plugin schema version: plugin requires {} but host supports {}",
                    plugin_version, host_version
                ));
            }
        }

        Ok(manifest)
    }

    /// Activate a previously discovered package runtime. The caller must restore
    /// a matching durable grant into PermissionChecker before calling this.
    #[cfg(feature = "extism")]
    pub async fn load_plugin_from_folder<P: AsRef<Path>>(
        &self,
        plugin_folder: P,
    ) -> Result<String, String> {
        use extism::{Function, Manifest, Plugin, UserData, Val, ValType, Wasm};
        use serde_json::Value;
        let plugin_folder = plugin_folder.as_ref();
        let total_start = std::time::Instant::now();

        println!("📂 Loading plugin from folder: {}", plugin_folder.display());
        let manifest = self.discover_plugin_from_folder(plugin_folder).await?;

        // 1.1. Check if plugin is already loaded (skip expensive WASM compilation)
        let normalized_id = Self::normalize_plugin_id(&manifest.id);
        {
            let plugins = self.plugins.read().await;
            if plugins.keys().any(|k| k.starts_with(&normalized_id)) {
                println!("  ⏩ Plugin {} already loaded, skipping", normalized_id);
                return Ok(normalized_id);
            }
        }

        // Load the runtime plugin ONLY if its artifact exists (bundle or
        // plugin-only). Widget-only packages (no plugin.wasm / service config)
        // skip this — their widget contribution was registered above.
        let wasm_path = plugin_folder.join("plugin.wasm");
        if wasm_path.exists() {
            println!("  🔷 Runtime: WASM (in-process)");
            return self
                .load_wasm_plugin_from_folder(plugin_folder, manifest)
                .await;
        }
        if manifest.get_service_config().is_some() {
            println!("  🔶 Runtime: Service (external process)");
            return self
                .load_service_plugin_from_folder(plugin_folder, manifest)
                .await;
        }
        // No runnable artifact — widget/contributions-only package.
        Ok(normalized_id)
    }

    /// Load a WASM plugin from a folder
    async fn load_wasm_plugin_from_folder(
        &self,
        plugin_folder: &Path,
        manifest: super::bundle::PluginManifest,
    ) -> Result<String, String> {
        use extism::{Function, Manifest, Plugin, UserData, Val, ValType, Wasm};
        use serde_json::Value;
        use tokio::fs;

        // 2. Load plugin.wasm using Wasm::file() for better caching
        let wasm_path = plugin_folder.join("plugin.wasm");
        if !wasm_path.exists() {
            return Err("plugin.wasm not found in folder".to_string());
        }

        // Create plugin from FILE using centralized function
        let plugin_start = std::time::Instant::now();
        println!("  🔨 Starting Extism WASM→Native compilation (from file)...");
        let wasm = Wasm::file(&wasm_path);
        let (plugin, metadata_json) =
            tokio::task::spawn_blocking(move || compile_and_initialize_wasm(wasm))
                .await
                .map_err(|error| format!("WASM initialization worker failed: {error}"))??;
        println!(
            "  ⏱️ Extism compilation and initialization: {:.2?}",
            plugin_start.elapsed()
        );

        let metadata: Value = serde_json::from_str(&metadata_json)
            .map_err(|e| format!("Failed to parse plugin metadata: {}", e))?;

        // Check if this is new multi-node format or old single-node format
        let (
            plugin_id,
            plugin_name,
            description,
            version,
            author,
            category,
            category_display_name,
            subcategory,
            type_color,
            output_schema,
            parameters_schema,
        ) = if let Some(plugin_info) = metadata.get("plugin") {
            // New multi-node format: { plugin: {...}, nodes: [...] }
            let nodes = metadata.get("nodes").and_then(|v| v.as_array());

            // Use plugin-level info and first node's metadata
            let first_node = nodes.and_then(|arr| arr.get(0));

            // IMPORTANT: Use manifest as primary source (user-defined values)
            // WASM metadata contains placeholders that will be overridden
            let plugin_id = manifest.id.as_str();
            let plugin_name = manifest.name.clone();
            let version = manifest.version.clone();
            let author = manifest.author.clone();

            // Get node-specific fields from first node (for backwards compatibility)
            let description = first_node
                .and_then(|n| n.get("description"))
                .and_then(|v| v.as_str())
                .unwrap_or(&manifest.description)
                .to_string();
            let category = first_node
                .and_then(|n| n.get("category"))
                .and_then(|v| v.as_str())
                .unwrap_or("PLUGINS")
                .to_string();
            let category_display_name = first_node
                .and_then(|n| n.get("category_display_name"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let subcategory = first_node
                .and_then(|n| n.get("subcategory"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let type_color = first_node
                .and_then(|n| n.get("type_color"))
                .and_then(|v| v.as_str())
                .unwrap_or("#666666")
                .to_string();
            let output_schema = first_node.and_then(|n| n.get("output_schema")).cloned(); // Optional output schema
            let parameters_schema = first_node
                .and_then(|n| n.get("parameters_schema"))
                .cloned()
                .unwrap_or(serde_json::json!({}));

            (
                plugin_id,
                plugin_name,
                description,
                version,
                author,
                category,
                category_display_name,
                subcategory,
                type_color,
                output_schema,
                parameters_schema,
            )
        } else {
            // Old single-node format (backwards compatibility)
            let plugin_id = metadata
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or(manifest.id.as_str());
            let plugin_name = metadata
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(&manifest.name)
                .to_string();
            let description = metadata
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or(&manifest.description)
                .to_string();
            let version = metadata
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or(&manifest.version)
                .to_string();
            let author = metadata
                .get("author")
                .and_then(|v| v.as_str())
                .unwrap_or(&manifest.author)
                .to_string();
            let category = metadata
                .get("category")
                .and_then(|v| v.as_str())
                .unwrap_or("PLUGINS")
                .to_string();
            let category_display_name = metadata
                .get("category_display_name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let subcategory = metadata
                .get("subcategory")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let type_color = metadata
                .get("type_color")
                .and_then(|v| v.as_str())
                .unwrap_or("#666666")
                .to_string();
            let output_schema = metadata.get("output_schema").cloned(); // Optional output schema
            let parameters_schema = metadata
                .get("parameters_schema")
                .cloned()
                .unwrap_or(serde_json::json!({}));

            (
                plugin_id,
                plugin_name,
                description,
                version,
                author,
                category,
                category_display_name,
                subcategory,
                type_color,
                output_schema,
                parameters_schema,
            )
        };

        // Check if this is multi-node format
        let nodes_array = metadata.get("nodes").and_then(|v| v.as_array());
        let file_size = Some(fs::metadata(&wasm_path).await.map(|m| m.len()).unwrap_or(0));

        // DEBUG: Print metadata structure
        println!(
            "  🐛 DEBUG: metadata has 'nodes'? {}",
            metadata.get("nodes").is_some()
        );
        if let Some(nodes_val) = metadata.get("nodes") {
            println!("  🐛 DEBUG: 'nodes' is array? {}", nodes_val.is_array());
            if let Some(arr) = nodes_val.as_array() {
                println!("  🐛 DEBUG: 'nodes' array length: {}", arr.len());
            }
        }

        // Web extensions are package contributions, not runtime nodes. Keep the
        // public manifest identity here; `plugin.` is an internal runtime key.
        let web_extension_package_id = Self::canonical_package_id(&manifest.id).to_string();

        let prefixed_id = if let Some(nodes) = nodes_array {
            // Multi-node plugin - create separate flow node for each
            println!(
                "🔌 Multi-node plugin detected: {} with {} nodes",
                plugin_name,
                nodes.len()
            );

            let plugin_arc = Arc::new(tokio::sync::Mutex::new(plugin));

            for node in nodes {
                let node_id = node
                    .get("_node_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let node_metadata_id = node.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
                let node_name = node
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown Node")
                    .to_string();
                let node_description = node
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let node_category = node
                    .get("category")
                    .and_then(|v| v.as_str())
                    .unwrap_or("PLUGINS")
                    .to_string();
                let node_category_display_name = node
                    .get("category_display_name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let node_subcategory = node
                    .get("subcategory")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let node_type_color = node
                    .get("type_color")
                    .and_then(|v| v.as_str())
                    .unwrap_or("#666666")
                    .to_string();
                let node_output_schema = node.get("output_schema").cloned(); // Optional output schema
                let node_parameters_schema = node
                    .get("parameters_schema")
                    .cloned()
                    .unwrap_or(serde_json::json!({}));

                let node_prefixed_id = Self::normalize_plugin_id(node_metadata_id);

                println!(
                    "  📦 Creating flow node: {} ({})",
                    node_name, node_prefixed_id
                );

                let wasm_wrapper = Arc::new(WasmPluginWrapper {
                    id: node_prefixed_id.clone(),
                    name: node_name.clone(),
                    description: node_description,
                    version: version.clone(),
                    author: author.clone(),
                    category: node_category.clone(),
                    category_display_name: node_category_display_name,
                    subcategory: node_subcategory,
                    type_color: node_type_color,
                    output_schema: node_output_schema,
                    parameters_schema: node_parameters_schema,
                    enabled: Arc::new(tokio::sync::RwLock::new(true)),
                    #[cfg(feature = "extism")]
                    execution_cancellation: Arc::new(tokio::sync::RwLock::new(
                        CancellationToken::new(),
                    )),
                    plugin: plugin_arc.clone(),
                    file_path: Some(wasm_path.clone()),
                    file_size,
                    icon_path: Some(plugin_folder.join("icon.png")),
                    node_id: Some(node_id.to_string()),
                    plugin_bundle: Some(PluginBundleInfo {
                        id: Self::normalize_plugin_id(plugin_id),
                        name: plugin_name.clone(),
                        version: version.clone(),
                        author: author.clone(),
                    }),
                    permission_checker: self.permission_checker.clone(),
                    platform_context: self.platform_context.clone(),
                });

                // Store in WASM plugins collection
                {
                    let mut wasm_plugins = self.wasm_plugins.write().await;
                    wasm_plugins.push(wasm_wrapper.clone());
                }

                // Register as separate flow node
                let generic_plugin = Box::new(ExtismPluginAdapter {
                    inner: wasm_wrapper,
                });

                self.register_plugin(node_prefixed_id.clone(), generic_plugin)
                    .await
                    .map_err(|e| format!("Failed to register node {}: {}", node_name, e))?;
            }

            println!(
                "✅ Multi-node plugin registered: {} ({} nodes)",
                plugin_name,
                nodes.len()
            );

            nodes
                .get(0)
                .and_then(|n| n.get("id"))
                .and_then(|v| v.as_str())
                .map(|id| Self::normalize_plugin_id(id))
                .unwrap_or_else(|| Self::normalize_plugin_id(plugin_id))
        } else {
            // Single-node plugin (backwards compatibility)
            let prefixed_id = Self::normalize_plugin_id(plugin_id);

            println!(
                "🔌 Successfully loaded WASM plugin: {} ({})",
                plugin_name, prefixed_id
            );
            if let Some(display_name) = &category_display_name {
                println!("  📂 Category: {} ({})", category, display_name);
            } else {
                println!("  📂 Category: {}", category);
            }

            // Create WASM plugin wrapper
            let wasm_wrapper = Arc::new(WasmPluginWrapper {
                id: prefixed_id.clone(),
                name: plugin_name.clone(),
                description,
                version,
                author,
                category,
                category_display_name,
                subcategory,
                type_color,
                output_schema,
                parameters_schema,
                enabled: Arc::new(tokio::sync::RwLock::new(true)),
                #[cfg(feature = "extism")]
                execution_cancellation: Arc::new(
                    tokio::sync::RwLock::new(CancellationToken::new()),
                ),
                plugin: Arc::new(tokio::sync::Mutex::new(plugin)),
                file_path: Some(wasm_path.clone()),
                file_size,
                icon_path: Some(plugin_folder.join("icon.png")),
                node_id: None,
                plugin_bundle: None, // Single-node plugin has no bundle
                permission_checker: self.permission_checker.clone(),
                platform_context: self.platform_context.clone(),
            });

            // Store in WASM plugins collection for flow integration
            {
                let lock_start = std::time::Instant::now();
                let mut wasm_plugins = self.wasm_plugins.write().await;
                println!(
                    "  ⏱️ Acquired wasm_plugins write lock: {:.2?}",
                    lock_start.elapsed()
                );
                wasm_plugins.push(wasm_wrapper.clone());
            }

            // Create the generic plugin interface
            let generic_plugin = Box::new(ExtismPluginAdapter {
                inner: wasm_wrapper,
            });

            // Register the plugin
            let register_start = std::time::Instant::now();
            self.register_plugin(prefixed_id.clone(), generic_plugin)
                .await
                .map_err(|e| format!("Failed to register plugin: {}", e))?;
            println!(
                "  ⏱️ Plugin registration (total): {:.2?}",
                register_start.elapsed()
            );

            prefixed_id
        };

        // 3. Load web resources (if present)
        let web_dir = plugin_folder.join("web");
        if web_dir.exists() {
            let mut web_resources = super::bundle::WebResources {
                extension_js: None,
                styles_css: None,
                assets: HashMap::new(),
            };

            // Load extension.js
            let js_path = web_dir.join("extension.js");
            if js_path.exists() {
                web_resources.extension_js = Some(
                    fs::read_to_string(&js_path)
                        .await
                        .map_err(|e| format!("Failed to read extension.js: {}", e))?,
                );
            }

            // Load styles.css
            let css_path = web_dir.join("styles.css");
            if css_path.exists() {
                web_resources.styles_css = Some(
                    fs::read_to_string(&css_path)
                        .await
                        .map_err(|e| format!("Failed to read styles.css: {}", e))?,
                );
            }

            // Load assets
            let assets_dir = web_dir.join("assets");
            if assets_dir.exists() {
                let mut entries = fs::read_dir(&assets_dir)
                    .await
                    .map_err(|e| format!("Failed to read assets directory: {}", e))?;

                while let Some(entry) = entries
                    .next_entry()
                    .await
                    .map_err(|e| format!("Failed to read asset entry: {}", e))?
                {
                    if entry.path().is_file() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        let data = fs::read(entry.path())
                            .await
                            .map_err(|e| format!("Failed to read asset {}: {}", name, e))?;
                        web_resources.assets.insert(name, data);
                    }
                }
            }

            // Store web extensions using the canonical package ID (not a node
            // ID or the internal `plugin.` runtime namespace).
            let mut extensions = self.web_extensions.write().await;
            extensions.insert(web_extension_package_id.clone(), web_resources);
            println!(
                "✅ Loaded web extensions for plugin: {}",
                web_extension_package_id
            );
        }

        self.refresh_flow_registration().await;
        Ok(prefixed_id)
    }

    /// Load a Service plugin from a folder
    async fn load_service_plugin_from_folder(
        &self,
        plugin_folder: &Path,
        manifest: super::bundle::PluginManifest,
    ) -> Result<String, String> {
        use super::service_plugin::ServicePlugin;

        // Spawn the service plugin
        let service_plugin = ServicePlugin::spawn(
            manifest.clone(),
            plugin_folder.to_path_buf(),
            self.permission_checker.clone(),
        )
        .await?;
        let service_plugin_arc = Arc::new(service_plugin);

        // Normalize plugin ID
        let normalized_id = Self::normalize_plugin_id(&manifest.id);

        // Get nodes from the plugin (they're already cached in ServicePlugin)
        let nodes = service_plugin_arc.get_nodes().await;

        // Every owned service process participates in package lifecycle, even
        // when it contributes no flow nodes. Otherwise unload cannot find and
        // terminate a node-less service.
        self.service_plugins
            .write()
            .await
            .push(service_plugin_arc.clone());

        if !nodes.is_empty() {
            println!("📝 Service plugin registered {} nodes", nodes.len());
        }

        // Register the plugin in the main plugin registry
        let generic_plugin: Box<dyn super::interface::Plugin> = Box::new(ServicePluginAdapter {
            inner: service_plugin_arc,
        });
        self.register_plugin(normalized_id.clone(), generic_plugin)
            .await
            .map_err(|e| format!("Failed to register service plugin: {:?}", e))?;

        self.refresh_flow_registration().await;

        println!("✅ Service plugin loaded: {}", normalized_id);

        Ok(normalized_id)
    }

    /// Load plugin from bytes when extism is not available
    #[cfg(not(feature = "extism"))]
    pub async fn load_plugin_from_bytes(
        &self,
        _wasm_bytes: &[u8],
        file_path: Option<PathBuf>,
        file_size: Option<u64>,
        _permissions: Option<&Vec<super::bundle::PluginPermission>>,
    ) -> Result<String, String> {
        // Create a basic plugin without WASM execution
        let plugin_id = file_path
            .as_ref()
            .and_then(|p| p.file_stem())
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let prefixed_id = Self::normalize_plugin_id(&plugin_id);

        let wasm_wrapper = Arc::new(WasmPluginWrapper {
            id: prefixed_id.clone(),
            name: format!("Plugin {}", plugin_id),
            description: "WASM plugin (extism feature not enabled)".to_string(),
            version: "1.0.0".to_string(),
            author: "Plugin Author".to_string(),
            category: "PLUGINS".to_string(),
            category_display_name: None,
            subcategory: None,
            type_color: "#666666".to_string(),
            output_schema: None,
            parameters_schema: json!({}),
            enabled: Arc::new(tokio::sync::RwLock::new(true)),
            #[cfg(feature = "extism")]
            execution_cancellation: Arc::new(tokio::sync::RwLock::new(CancellationToken::new())),
            plugin: Arc::new(tokio::sync::Mutex::new(())), // Dummy plugin
            file_path: file_path.clone(),
            file_size,
            icon_path: None, // Standalone WASM files don't have web/assets folder structure
            node_id: None,
            plugin_bundle: None, // No-extism fallback has no bundle
            permission_checker: self.permission_checker.clone(),
            platform_context: self.platform_context.clone(),
        });

        // Store in WASM plugins collection
        {
            let mut wasm_plugins = self.wasm_plugins.write().await;
            wasm_plugins.push(wasm_wrapper.clone());
        }

        // Create the generic plugin interface
        let generic_plugin = Box::new(ExtismPluginAdapter {
            inner: wasm_wrapper,
        });

        // Register the plugin
        self.register_plugin(prefixed_id.clone(), generic_plugin)
            .await
            .map_err(|e| format!("Failed to register plugin: {}", e))?;

        println!(
            "🔌 VortiDeckCore: Successfully loaded plugin: {} (extism disabled)",
            prefixed_id
        );

        Ok(prefixed_id)
    }

    /// Get WASM plugins for flow integration
    pub async fn get_wasm_plugins(&self) -> Vec<Arc<WasmPluginWrapper>> {
        let wasm_plugins = self.wasm_plugins.read().await;
        wasm_plugins.clone()
    }

    /// Get service plugin flow providers
    pub async fn get_service_flow_providers(&self) -> Vec<ServiceFlowNodeProvider> {
        let service_plugins = self.service_plugins.read().await.clone();
        let mut providers = Vec::new();

        for plugin in &service_plugins {
            let nodes = plugin.get_nodes().await;
            if !nodes.is_empty() {
                providers.push(ServiceFlowNodeProvider {
                    plugin: plugin.clone(),
                    nodes,
                });
            }
        }

        providers
    }

    /// Enable or disable a plugin
    pub async fn set_plugin_enabled(&self, plugin_id: &str, enabled: bool) -> Result<(), String> {
        let plugin = {
            let wasm_plugins = self.wasm_plugins.read().await;
            wasm_plugins
                .iter()
                .find(|plugin| plugin.id == plugin_id)
                .cloned()
        };
        if let Some(plugin) = plugin {
            plugin.set_enabled(enabled).await;
            println!(
                "🔌 Plugin {} {}",
                plugin_id,
                if enabled { "enabled" } else { "disabled" }
            );
            return Ok(());
        }

        Err(format!("Plugin with ID '{}' not found", plugin_id))
    }

    /// Check if a plugin is enabled
    pub async fn is_plugin_enabled(&self, plugin_id: &str) -> Result<bool, String> {
        let plugin = {
            let wasm_plugins = self.wasm_plugins.read().await;
            wasm_plugins
                .iter()
                .find(|plugin| plugin.id == plugin_id)
                .cloned()
        };
        if let Some(plugin) = plugin {
            return Ok(*plugin.enabled.read().await);
        }

        Err(format!("Plugin with ID '{}' not found", plugin_id))
    }

    /// Stop every executable contribution belonging to one manifest package.
    /// Runtime registries may contain several WASM node ids plus a service
    /// process for a single package, so lifecycle operations must never target
    /// just the marketplace/card id.
    pub async fn unload_package_runtime(&self, package_id: &str) -> Result<bool, String> {
        let canonical = Self::canonical_package_id(package_id).to_string();
        let mut runtime_ids = std::collections::BTreeSet::new();
        let mut wasm_runtimes = Vec::new();

        {
            let wasm_plugins = self.wasm_plugins.read().await;
            for plugin in wasm_plugins.iter() {
                let belongs_to_package = plugin
                    .plugin_bundle
                    .as_ref()
                    .map(|bundle| Self::canonical_package_id(&bundle.id) == canonical)
                    .unwrap_or_else(|| Self::canonical_package_id(&plugin.id) == canonical);
                if belongs_to_package {
                    runtime_ids.insert(plugin.id.clone());
                    wasm_runtimes.push(plugin.clone());
                }
            }
        }
        {
            let service_plugins = self.service_plugins.read().await;
            for plugin in service_plugins.iter() {
                if Self::canonical_package_id(plugin.id()) == canonical {
                    runtime_ids.insert(Self::normalize_plugin_id(plugin.id()));
                }
            }
        }

        // Revoke authority and cancel executable calls before awaiting any
        // package shutdown. A host function reached during cancellation must
        // revalidate against the revoked package, never a stale grant.
        let checker = self.get_permission_checker();
        checker.revoke_all_permissions(&canonical);
        checker.revoke_all_permissions(&Self::normalize_plugin_id(&canonical));
        for plugin in &wasm_runtimes {
            plugin.set_enabled(false).await;
        }

        // One package-level deadline prevents packages with several runtime
        // contributions from multiplying the shutdown timeout per node.
        let shutdown = async {
            let mut errors = Vec::new();
            for runtime_id in &runtime_ids {
                if let Err(error) = self.unregister_runtime_plugin(runtime_id).await {
                    errors.push(format!("{runtime_id}: {error}"));
                }
            }
            errors
        };
        let errors = tokio::time::timeout(std::time::Duration::from_secs(15), shutdown)
            .await
            .map_err(|_| {
                format!(
                    "Timed out after 15s while stopping package '{}' runtimes",
                    package_id
                )
            })?;
        if !errors.is_empty() {
            return Err(format!(
                "Failed to stop package '{}': {}",
                package_id,
                errors.join(", ")
            ));
        }

        self.wasm_plugins.write().await.retain(|plugin| {
            plugin
                .plugin_bundle
                .as_ref()
                .map(|bundle| Self::canonical_package_id(&bundle.id) != canonical)
                .unwrap_or_else(|| Self::canonical_package_id(&plugin.id) != canonical)
        });
        self.service_plugins
            .write()
            .await
            .retain(|plugin| Self::canonical_package_id(plugin.id()) != canonical);
        self.web_extensions
            .write()
            .await
            .retain(|id, _| Self::canonical_package_id(id) != canonical);
        self.plugin_bundles
            .write()
            .await
            .retain(|id, _| Self::canonical_package_id(id) != canonical);

        self.refresh_flow_registration().await;

        Ok(!runtime_ids.is_empty())
    }

    /// Get the allowed plugin count for the current user/authentication status
    pub async fn get_allowed_plugin_count(&self) -> u32 {
        use crate::features::jwt_features::{get_jwt_provider, get_user_plan, is_user_premium};

        // Get user info from JWT (same as validate_plugin_installation)
        let is_premium = is_user_premium();

        // Get user's plugin limit from JWT token (single source of truth)
        let jwt_provider = get_jwt_provider();
        let user_limit = jwt_provider.get_plugin_limit();

        // ✅ FIX: Allow basic plugin functionality in guest mode for testing/demo
        if user_limit == 0 {
            // Guest mode detected - allow 1 plugin for basic functionality
            println!("🎭 Guest mode detected - allowing 1 plugin for demo/testing purposes");
            1
        } else {
            // Return the JWT-provided limit for authenticated users
            user_limit
        }
    }

    /// Atomically stage a plugin bundle for discovery. Consent-aware runtime
    /// activation belongs to the desktop package lifecycle coordinator.
    pub async fn stage_plugin_bundle<P: AsRef<Path>>(
        &self,
        bundle_path: P,
    ) -> Result<String, String> {
        use super::bundle::PluginBundle;
        use tokio::fs;

        let bundle_path = bundle_path.as_ref();

        println!("📦 Staging plugin bundle: {:?}", bundle_path);

        // 1. Extract bundle
        let bundle = PluginBundle::from_file(bundle_path)
            .map_err(|e| format!("Failed to extract bundle: {}", e))?;

        // 2. Validate manifest
        bundle
            .validate_manifest()
            .map_err(|e| format!("Invalid manifest: {}", e))?;

        // 3. Report declared permissions. Authority is granted only by the
        // desktop consent coordinator after extraction and discovery.
        if !bundle.manifest.permissions.is_empty() {
            println!(
                "🔐 Plugin requests permissions: {:?}",
                bundle.manifest.permissions
            );
        }

        // 4. Verify a legacy embedded creator signature when present. This is
        //    only proof of continuity with the self-declared creator key; it is
        //    never marketplace trust. Remote packages require a separately
        //    pinned marketplace attestation before reaching this mechanism.
        if let Some(signature) = &bundle.signature {
            let actual_hash = bundle.content_hash()?;
            if signature.package_hash.to_ascii_lowercase() != actual_hash {
                return Err(format!(
                    "Plugin signature hash does not match package content for '{}'",
                    signature.author_id
                ));
            }
            match base64::engine::general_purpose::STANDARD
                .decode(signature.author_pubkey.as_bytes())
            {
                Ok(pk) => {
                    if super::signing::verify_plugin_signature(signature, &pk) {
                        println!(
                            "🔏 Package content is self-signed by creator '{}' (untrusted until attested)",
                            signature.author_id
                        );
                    } else {
                        return Err(format!(
                            "Plugin signature is invalid for creator '{}'",
                            signature.author_id
                        ));
                    }
                }
                Err(_) => {
                    return Err(format!(
                        "Plugin signature public key is malformed for creator '{}'",
                        signature.author_id
                    ));
                }
            }
        }

        // 5. Extract outside the watched directory and atomically publish the
        // complete folder. Legacy code extracted beside the archive, which made
        // packages disappear after restart when the archive lived in the app
        // data root rather than app-data/plugins.
        if bundle.manifest.id.is_empty()
            || !bundle.manifest.id.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
            })
        {
            return Err(format!("Unsafe package id '{}'", bundle.manifest.id));
        }
        let plugins_dir = crate::VortiDeckCore::get_default_plugins_directory();
        fs::create_dir_all(&plugins_dir)
            .await
            .map_err(|e| format!("Failed to create plugins directory: {}", e))?;
        let plugin_folder = plugins_dir.join(&bundle.manifest.id);
        if plugin_folder.exists() {
            return Err(format!(
                "Package '{}' already exists; use the update lifecycle instead of overwriting it",
                bundle.manifest.id
            ));
        }
        let staging_root = plugins_dir
            .parent()
            .ok_or_else(|| "Plugins directory has no parent".to_string())?
            .join("plugin-install-staging");
        fs::create_dir_all(&staging_root)
            .await
            .map_err(|e| format!("Failed to create package staging directory: {}", e))?;
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let staged_folder = staging_root.join(format!("{}-{}", bundle.manifest.id, nonce));

        fs::create_dir_all(&staged_folder)
            .await
            .map_err(|e| format!("Failed to create plugin folder: {}", e))?;

        // Write manifest.json
        let manifest_json = serde_json::to_string_pretty(&bundle.manifest)
            .map_err(|e| format!("Failed to serialize manifest: {}", e))?;
        fs::write(staged_folder.join("manifest.json"), manifest_json)
            .await
            .map_err(|e| format!("Failed to write manifest: {}", e))?;

        // Write plugin.wasm
        fs::write(staged_folder.join("plugin.wasm"), &bundle.wasm_bytes)
            .await
            .map_err(|e| format!("Failed to write WASM: {}", e))?;

        // Write web resources (if present)
        if let Some(web_resources) = bundle.web_resources {
            let web_dir = staged_folder.join("web");
            fs::create_dir_all(&web_dir)
                .await
                .map_err(|e| format!("Failed to create web directory: {}", e))?;

            if let Some(js) = &web_resources.extension_js {
                fs::write(web_dir.join("extension.js"), js)
                    .await
                    .map_err(|e| format!("Failed to write extension.js: {}", e))?;
            }

            if let Some(css) = &web_resources.styles_css {
                fs::write(web_dir.join("styles.css"), css)
                    .await
                    .map_err(|e| format!("Failed to write styles.css: {}", e))?;
            }

            // Write assets to their manifest-declared canonical relative paths.
            if !web_resources.assets.is_empty() {
                for (name, data) in &web_resources.assets {
                    let relative = super::bundle::validate_relative_artifact_path(name)?;
                    let destination = staged_folder.join(relative);
                    if let Some(parent) = destination.parent() {
                        fs::create_dir_all(parent)
                            .await
                            .map_err(|e| format!("Failed to create asset directory: {}", e))?;
                    }
                    fs::write(&destination, data)
                        .await
                        .map_err(|e| format!("Failed to write asset {}: {}", name, e))?;
                }
            }
        }

        fs::rename(&staged_folder, &plugin_folder)
            .await
            .map_err(|e| format!("Failed to publish staged package: {}", e))?;
        println!("📂 Extracted plugin to: {}", plugin_folder.display());

        // 7. Delete .vdext file after successful extraction
        if let Err(e) = fs::remove_file(bundle_path).await {
            println!("⚠️ Warning: Failed to delete .vdext file: {}", e);
            // Don't fail installation if we can't delete the file
        } else {
            println!("🗑️ Deleted .vdext file after extraction");
        }

        println!(
            "📦 Plugin bundle staged successfully; consent is still required: {}",
            bundle.manifest.id
        );

        Ok(bundle.manifest.id)
    }

    /// Safely extract a verified marketplace archive into the watched package
    /// directory. Trust verification happens before this call; this method
    /// independently enforces archive/path/resource limits and atomic publish.
    pub async fn stage_verified_package_archive<P: AsRef<Path>>(
        &self,
        archive_path: P,
        expected_package_id: &str,
        expected_version: &str,
    ) -> Result<PathBuf, String> {
        const MAX_FILES: usize = 2048;
        const MAX_EXTRACTED_BYTES: u64 = 200 * 1024 * 1024;
        const MAX_SINGLE_FILE_BYTES: u64 = 100 * 1024 * 1024;

        let archive_path = archive_path.as_ref().to_path_buf();
        let package_id = expected_package_id.to_string();
        let version = expected_version.to_string();
        tokio::task::spawn_blocking(move || {
            let file = std::fs::File::open(&archive_path)
                .map_err(|error| format!("Failed to open marketplace artifact: {error}"))?;
            let mut archive = zip::ZipArchive::new(file)
                .map_err(|error| format!("Marketplace artifact is not a valid ZIP: {error}"))?;
            if archive.len() > MAX_FILES {
                return Err(format!("Marketplace artifact exceeds {MAX_FILES} files"));
            }
            let mut manifest_json = String::new();
            archive
                .by_name("manifest.json")
                .map_err(|_| "Marketplace artifact requires root manifest.json".to_string())?
                .read_to_string(&mut manifest_json)
                .map_err(|error| format!("Failed to read marketplace manifest: {error}"))?;
            let manifest: super::bundle::PluginManifest = serde_json::from_str(&manifest_json)
                .map_err(|error| format!("Invalid marketplace manifest: {error}"))?;
            if manifest.id != package_id || manifest.version != version {
                return Err("Marketplace artifact identity does not match the release".to_string());
            }
            super::bundle::validate_package_id(&manifest.id)?;

            let plugins_dir = crate::VortiDeckCore::get_default_plugins_directory();
            std::fs::create_dir_all(&plugins_dir)
                .map_err(|error| format!("Failed to create plugins directory: {error}"))?;
            let staging_root = plugins_dir
                .parent()
                .ok_or_else(|| "Plugins directory has no parent".to_string())?
                .join("plugin-install-staging");
            std::fs::create_dir_all(&staging_root)
                .map_err(|error| format!("Failed to create install staging: {error}"))?;
            let staged = staging_root.join(format!("{}-{}", manifest.id, uuid::Uuid::new_v4()));
            std::fs::create_dir(&staged)
                .map_err(|error| format!("Failed to create staged package: {error}"))?;

            let extraction = (|| {
                let mut seen = std::collections::BTreeSet::new();
                let mut total = 0_u64;
                let mut written = 0_u64;
                for index in 0..archive.len() {
                    let mut entry = archive
                        .by_index(index)
                        .map_err(|error| format!("Failed to inspect archive entry: {error}"))?;
                    let raw_name = entry.name().to_string();
                    if raw_name.contains('\\') {
                        return Err(format!("Archive path uses backslashes: {raw_name}"));
                    }
                    let relative = super::bundle::validate_relative_artifact_path(
                        raw_name.trim_end_matches('/'),
                    )?;
                    if !seen.insert(relative.clone()) {
                        return Err(format!("Duplicate archive path: {}", relative.display()));
                    }
                    if entry
                        .unix_mode()
                        .is_some_and(|mode| mode & 0o170000 == 0o120000)
                    {
                        return Err(format!("Symbolic links are forbidden: {raw_name}"));
                    }
                    if entry.is_dir() {
                        std::fs::create_dir_all(staged.join(&relative)).map_err(|error| {
                            format!("Failed to create package directory: {error}")
                        })?;
                        continue;
                    }
                    if entry.size() > MAX_SINGLE_FILE_BYTES {
                        return Err(format!("Package file is too large: {raw_name}"));
                    }
                    total = total.saturating_add(entry.size());
                    if total > MAX_EXTRACTED_BYTES {
                        return Err("Marketplace artifact expands beyond 200 MiB".to_string());
                    }
                    let output = staged.join(&relative);
                    if let Some(parent) = output.parent() {
                        std::fs::create_dir_all(parent).map_err(|error| {
                            format!("Failed to create package directory: {error}")
                        })?;
                    }
                    let mut writer = std::fs::File::create(&output)
                        .map_err(|error| format!("Failed to create package file: {error}"))?;
                    // Declared sizes are archive-controlled; bound what is
                    // actually decompressed per file and across the package.
                    let limit =
                        MAX_SINGLE_FILE_BYTES.min(MAX_EXTRACTED_BYTES.saturating_sub(written));
                    let copied = Self::copy_archive_entry_bounded(&mut entry, &mut writer, limit)
                        .map_err(|error| format!("Failed to extract package file: {error}"))?
                        .ok_or_else(|| {
                            if limit < MAX_SINGLE_FILE_BYTES {
                                "Marketplace artifact expands beyond 200 MiB".to_string()
                            } else {
                                format!("Package file is too large: {raw_name}")
                            }
                        })?;
                    written = written.saturating_add(copied);
                }
                // Verified delivery never publishes directly into the watched
                // directory. The desktop lifecycle authority owns both first
                // installation and update so receipt persistence, filesystem
                // publication and watcher reconciliation share one lock.
                Ok(staged.clone())
            })();
            if extraction.is_err() {
                let _ = std::fs::remove_dir_all(&staged);
            }
            extraction
        })
        .await
        .map_err(|error| format!("Marketplace extraction task failed: {error}"))?
    }

    /// Copy at most `limit` decompressed bytes. Returns `None` when the entry
    /// holds more than `limit` bytes, regardless of its declared size.
    fn copy_archive_entry_bounded<R: Read, W: std::io::Write>(
        reader: R,
        writer: &mut W,
        limit: u64,
    ) -> std::io::Result<Option<u64>> {
        let copied = std::io::copy(&mut reader.take(limit.saturating_add(1)), writer)?;
        Ok((copied <= limit).then_some(copied))
    }

    fn web_extension_files(
        package_id: &str,
        resources: &super::bundle::WebResources,
    ) -> Vec<WebExtensionFile> {
        let mut files = Vec::new();
        if let Some(js_content) = &resources.extension_js {
            files.push(WebExtensionFile {
                plugin_id: package_id.to_string(),
                file_type: "js".to_string(),
                content: js_content.clone(),
            });
        }
        if let Some(css_content) = &resources.styles_css {
            files.push(WebExtensionFile {
                plugin_id: package_id.to_string(),
                file_type: "css".to_string(),
                content: css_content.clone(),
            });
        }
        files
    }

    /// Return the extension payload for one manifest package. Public callers
    /// may provide either a canonical package ID or an internal runtime ID,
    /// but the returned identity is always canonical.
    pub async fn get_web_extensions_for_package(&self, package_id: &str) -> Vec<WebExtensionFile> {
        let extensions = self.web_extensions.read().await;
        let canonical = Self::canonical_package_id(package_id);
        extensions
            .iter()
            .find(|(id, _)| Self::canonical_package_id(id) == canonical)
            .map(|(_, resources)| Self::web_extension_files(canonical, resources))
            .unwrap_or_default()
    }

    /// List package identities with a usable web extension without cloning the
    /// extension source into package-list responses.
    pub async fn list_web_extension_package_ids(&self) -> std::collections::HashSet<String> {
        self.web_extensions
            .read()
            .await
            .iter()
            .filter(|(_, resources)| {
                resources.extension_js.is_some() || resources.styles_css.is_some()
            })
            .map(|(id, _)| Self::canonical_package_id(id).to_string())
            .collect()
    }

    /// Get a specific plugin asset (for asset URLs)
    pub async fn get_plugin_asset(&self, plugin_id: &str, asset_path: &str) -> Option<Vec<u8>> {
        // The id is joined to the plugins directory below; reject traversal.
        super::bundle::validate_package_id(Self::canonical_package_id(plugin_id)).ok()?;
        let extensions = self.web_extensions.read().await;
        let normalized = Self::normalize_plugin_id(plugin_id);
        let canonical = Self::canonical_package_id(plugin_id);
        let basename = std::path::Path::new(asset_path)
            .file_name()
            .and_then(|value| value.to_str());
        for key in [plugin_id, canonical, normalized.as_str()] {
            if let Some(resources) = extensions.get(key) {
                if let Some(bytes) = resources.assets.get(asset_path) {
                    return Some(bytes.clone());
                }
                if let Some(bytes) = basename.and_then(|name| resources.assets.get(name)) {
                    return Some(bytes.clone());
                }
                let canonical = format!("web/assets/{asset_path}");
                if let Some(bytes) = resources.assets.get(&canonical) {
                    return Some(bytes.clone());
                }
            }
        }
        drop(extensions);

        let relative = super::bundle::validate_relative_artifact_path(asset_path).ok()?;
        let plugins_dir = crate::VortiDeckCore::get_default_plugins_directory();
        let package_id = Self::canonical_package_id(plugin_id);
        let package_root = tokio::fs::canonicalize(plugins_dir.join(package_id))
            .await
            .ok()?;
        let candidate = if relative.starts_with("web/assets") {
            package_root.join(relative)
        } else {
            package_root.join("web").join("assets").join(relative)
        };
        let resolved = tokio::fs::canonicalize(candidate).await.ok()?;
        if !resolved.starts_with(&package_root) {
            return None;
        }
        let bytes = tokio::fs::read(resolved).await.ok()?;
        (bytes.len() <= 2 * 1024 * 1024).then_some(bytes)
    }

    /// Call a specific function on a WASM plugin (used by sandboxed webview bridge)
    #[cfg(feature = "extism")]
    pub async fn call_plugin_function(
        &self,
        plugin_id: &str,
        function_name: &str,
        data: &[u8],
    ) -> Result<Vec<u8>, String> {
        // Normalize plugin ID (handle both with and without "plugin." prefix)
        let normalized_id = Self::normalize_plugin_id(plugin_id);

        // Find the WASM plugin
        let plugin_wrapper = self
            .wasm_plugins
            .read()
            .await
            .iter()
            .find(|p| {
                // Direct ID match (for single-node plugins or specific node calls)
                p.id == normalized_id || p.id == plugin_id ||
                // Bundle ID match (for multi-node plugins calling from web extension)
                p.plugin_bundle.as_ref().map(|b| b.id == plugin_id || format!("plugin.{}", b.id) == normalized_id).unwrap_or(false)
            })
            .cloned()
            .ok_or_else(|| format!("Plugin not found: {}", plugin_id))?;

        // Check if plugin is enabled
        let enabled = *plugin_wrapper.enabled.read().await;
        if !enabled {
            return Err(format!("Plugin {} is disabled", plugin_id));
        }

        println!(
            "🔌 [call_plugin_function] Calling {}::{}",
            plugin_id, function_name
        );

        // Convert data to string (WASM plugins expect string input)
        let data_str = String::from_utf8_lossy(data).into_owned();
        let execution_cancellation = plugin_wrapper.execution_cancellation.read().await.clone();
        let result = call_extism_bounded(
            plugin_wrapper.plugin.clone(),
            plugin_wrapper.package_principal(),
            plugin_wrapper.permission_checker.clone(),
            plugin_wrapper.platform_context.clone(),
            function_name.to_string(),
            data_str,
            execution_cancellation,
        )
        .await?;

        println!(
            "🔌 [call_plugin_function] Returned {} byte(s)",
            result.len()
        );

        Ok(result.into_bytes())
    }

    /// Call a specific function on a WASM plugin (stub when extism disabled)
    #[cfg(not(feature = "extism"))]
    pub async fn call_plugin_function(
        &self,
        plugin_id: &str,
        function_name: &str,
        _data: &[u8],
    ) -> Result<Vec<u8>, String> {
        Err(format!(
            "Cannot call {}::{} - extism feature not enabled",
            plugin_id, function_name
        ))
    }
}

/// Web extension file for frontend
#[derive(Debug, Clone, serde::Serialize)]
pub struct WebExtensionFile {
    pub plugin_id: String,
    pub file_type: String, // "js" or "css"
    pub content: String,
}

/// ✅ JWT-POWERED UNIVERSAL FEATURE INTEGRATION
impl FeatureRequirement for PluginManager {
    fn get_requirements(&self) -> Vec<PremiumFeature> {
        vec![PremiumFeature::UsageLimit {
            feature_id: "plugin_count".to_string(),
            free_limit: 0,       // ✅ Ignored - JWT provides real limits
            premium_limit: None, // ✅ Ignored - JWT provides real limits
            reset_period: ResetPeriod::Never,
        }]
    }

    fn feature_context(&self) -> String {
        "Plugin System".to_string()
    }
}

/// WASM Plugin Wrapper with flow node capability
pub struct WasmPluginWrapper {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub category: String,
    pub category_display_name: Option<String>,
    pub subcategory: Option<String>,
    pub type_color: String,
    pub output_schema: Option<Value>, // JSON Schema for outputs (like MCP tools)
    pub parameters_schema: Value,
    pub enabled: Arc<tokio::sync::RwLock<bool>>,
    #[cfg(feature = "extism")]
    execution_cancellation: Arc<tokio::sync::RwLock<CancellationToken>>,
    #[cfg(feature = "extism")]
    pub plugin: Arc<tokio::sync::Mutex<extism::Plugin>>,
    #[cfg(not(feature = "extism"))]
    pub plugin: Arc<tokio::sync::Mutex<()>>,
    pub file_path: Option<PathBuf>,
    pub file_size: Option<u64>,
    pub icon_path: Option<PathBuf>, // Path to plugin icon for UI display
    pub node_id: Option<String>,    // For multi-node plugins - which node to execute
    /// Plugin bundle info (for multi-node plugins)
    pub plugin_bundle: Option<PluginBundleInfo>,
    /// Permission checker for enforcing plugin permissions
    pub permission_checker: Arc<super::permissions::PermissionChecker>,
    /// Platform context reference for EventBus access
    pub platform_context: Arc<crate::core::PlatformContext>,
}

impl WasmPluginWrapper {
    fn package_principal(&self) -> String {
        let id = self
            .plugin_bundle
            .as_ref()
            .map(|bundle| bundle.id.as_str())
            .unwrap_or(&self.id);
        super::bundle::canonical_package_id(id).to_string()
    }

    async fn set_enabled(&self, enabled: bool) {
        let mut current = self.enabled.write().await;
        if enabled {
            #[cfg(feature = "extism")]
            {
                let mut cancellation = self.execution_cancellation.write().await;
                if cancellation.is_cancelled() {
                    *cancellation = CancellationToken::new();
                }
            }
            *current = true;
        } else {
            *current = false;
            #[cfg(feature = "extism")]
            self.execution_cancellation.read().await.cancel();
        }
    }

    /// Execute the plugin with flow parameters
    #[cfg(feature = "extism")]
    pub async fn execute(&self, mut params: Value) -> Result<Value, String> {
        // Check if plugin is enabled
        let enabled = *self.enabled.read().await;
        if !enabled {
            return Err(format!("Plugin {} is disabled", self.name));
        }

        // Inject node_id for multi-node plugins
        if let Some(node_id) = &self.node_id {
            if let Some(params_obj) = params.as_object_mut() {
                params_obj.insert("__node_id".to_string(), Value::String(node_id.clone()));
                println!("🔌 VortiDeckCore: Injected __node_id: {}", node_id);
            }
        }

        println!(
            "🔌 VortiDeckCore: Executing plugin {} with params: {}",
            self.name, params
        );

        let input = serde_json::to_string(&params)
            .map_err(|error| format!("Failed to serialize plugin input: {error}"))?;
        let execution_cancellation = self.execution_cancellation.read().await.clone();
        let result = call_extism_bounded(
            self.plugin.clone(),
            self.package_principal(),
            self.permission_checker.clone(),
            self.platform_context.clone(),
            "execute".to_string(),
            input,
            execution_cancellation,
        )
        .await
        .map_err(|error| format!("Plugin execution error: {error}"))?;

        println!("🔌 Plugin returned {} byte(s)", result.len());

        // Parse the result
        let parsed_result: Value = serde_json::from_str(&result)
            .map_err(|e| format!("Failed to parse plugin result: {}", e))?;

        Ok(parsed_result)
    }

    #[cfg(not(feature = "extism"))]
    pub async fn execute(&self, _params: Value) -> Result<Value, String> {
        // Check if plugin is enabled
        let enabled = *self.enabled.read().await;
        if !enabled {
            return Err(format!("Plugin {} is disabled", self.name));
        }

        Err("Extism feature not enabled".to_string())
    }
}

/// Adapter to make WasmPluginWrapper work with the generic Plugin trait
struct ExtismPluginAdapter {
    inner: Arc<WasmPluginWrapper>,
}

#[async_trait]
impl Plugin for ExtismPluginAdapter {
    fn id(&self) -> &str {
        &self.inner.id
    }

    fn name(&self) -> &str {
        &self.inner.name
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn metadata(&self) -> FlowNodeMetadata {
        FlowNodeMetadata {
            id: self.inner.id.clone(),
            name: self.inner.name.clone(),
            version: self.inner.version.clone(),
            description: self.inner.description.clone(),
            author: self.inner.author.clone(),
            icon_path: self.inner.icon_path.clone(),
            plugin_bundle: self.inner.plugin_bundle.clone(),
            category: Some(self.inner.category.clone()),
            parameters_schema: Some(self.inner.parameters_schema.clone()),
            output_schema: self.inner.output_schema.clone(),
            type_color: Some(self.inner.type_color.clone()),
        }
    }

    async fn initialize(&self) -> Result<(), PluginError> {
        println!(
            "🚀 VortiDeckCore: Initializing WASM plugin: {}",
            self.inner.name
        );
        Ok(())
    }

    async fn shutdown(&self) -> Result<(), PluginError> {
        println!(
            "⏹️ VortiDeckCore: Shutting down WASM plugin: {}",
            self.inner.name
        );
        self.inner.set_enabled(false).await;
        Ok(())
    }
}

/// Wrapper for service plugin flow nodes
pub struct ServiceFlowNodeWrapper {
    pub plugin: Arc<super::service_plugin::ServicePlugin>,
    pub metadata: FlowNodeMetadata,
}

#[async_trait]
impl FlowNode for ServiceFlowNodeWrapper {
    fn id(&self) -> String {
        self.metadata.id.clone()
    }

    fn name(&self) -> String {
        self.metadata.name.clone()
    }

    fn description(&self) -> String {
        self.metadata.description.clone()
    }

    fn category(&self) -> String {
        self.metadata
            .category
            .clone()
            .unwrap_or_else(|| "SERVICES".to_string())
    }

    fn category_display_name(&self) -> Option<String> {
        None
    }

    fn parameters_schema(&self) -> Value {
        self.metadata
            .parameters_schema
            .clone()
            .unwrap_or_else(|| serde_json::json!({}))
    }

    fn type_color(&self) -> String {
        self.metadata
            .type_color
            .clone()
            .unwrap_or_else(|| "#FF6B35".to_string())
    }

    fn outputs_schema(&self) -> Vec<NodeOutput> {
        use crate::services::flow::traits::parse_output_schema_to_outputs;

        if let Some(ref schema) = self.metadata.output_schema {
            parse_output_schema_to_outputs(schema)
        } else {
            // No schema - return default "result" output
            vec![crate::services::flow::traits::NodeOutput {
                id: "result".to_string(),
                name: "Result".to_string(),
                output_type: "any".to_string(),
                description: Some("Node execution result".to_string()),
                is_dynamic_type: None,
            }]
        }
    }

    async fn execute(&self, params: Value) -> Result<Value, String> {
        // Call the service plugin via JSON-RPC
        let node_id = self.metadata.id.clone();

        // Prepare execution request
        let execution_params = serde_json::json!({
            "nodeId": node_id,
            "inputs": params
        });

        // Call the plugin's execute method via JSON-RPC
        self.plugin.call_method("execute", execution_params).await
    }
}

/// Adapter to make ServicePlugin work with the generic Plugin trait
struct ServicePluginAdapter {
    inner: Arc<super::service_plugin::ServicePlugin>,
}

#[async_trait]
impl Plugin for ServicePluginAdapter {
    fn id(&self) -> &str {
        self.inner.id()
    }

    fn name(&self) -> &str {
        self.inner.name()
    }

    fn version(&self) -> &str {
        self.inner.version()
    }

    fn metadata(&self) -> FlowNodeMetadata {
        self.inner.metadata()
    }

    async fn initialize(&self) -> Result<(), PluginError> {
        self.inner.initialize().await
    }

    async fn shutdown(&self) -> Result<(), PluginError> {
        self.inner
            .shutdown()
            .await
            .map_err(|e| PluginError::Error(e))
    }
}

/// Provider for service plugin flow nodes
pub struct ServiceFlowNodeProvider {
    pub plugin: Arc<super::service_plugin::ServicePlugin>,
    pub nodes: Vec<FlowNodeMetadata>,
}

impl FlowNodeProvider for ServiceFlowNodeProvider {
    fn get_flow_nodes(&self) -> Vec<Box<dyn FlowNode>> {
        self.nodes
            .iter()
            .map(|metadata| {
                Box::new(ServiceFlowNodeWrapper {
                    plugin: self.plugin.clone(),
                    metadata: metadata.clone(),
                }) as Box<dyn FlowNode>
            })
            .collect()
    }

    fn provider_name(&self) -> &'static str {
        "ServicePlugin"
    }
}

#[cfg(test)]
mod flow_registry_lifecycle_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct RecordingRegistry(AtomicUsize);

    impl FlowRegistryProvider for RecordingRegistry {
        fn replace_plugin_providers(&self, providers: Vec<Box<dyn FlowNodeProvider>>) {
            self.0.store(providers.len(), Ordering::SeqCst);
        }

        fn get_node_count(&self) -> usize {
            self.0.load(Ordering::SeqCst)
        }
    }

    #[tokio::test]
    async fn attaching_flow_registry_publishes_without_relocking_itself() {
        let manager = PluginManager::new();
        let registry = Arc::new(RecordingRegistry(AtomicUsize::new(usize::MAX)));

        tokio::time::timeout(
            std::time::Duration::from_millis(250),
            manager.set_flow_registry(registry.clone()),
        )
        .await
        .expect("flow registry attachment must not self-deadlock");
        assert_eq!(registry.get_node_count(), 0);
    }
}

#[cfg(all(test, feature = "extism"))]
mod execution_budget_tests {
    use super::*;

    const LOOP_WAT: &[u8] = br#"
        (module
            (func (export "loop_forever") (result i32)
                (loop $again (br $again))
                (i32.const 0)))
    "#;
    const NOOP_WAT: &[u8] = br#"
        (module (func (export "noop") (result i32) (i32.const 0)))
    "#;
    const LARGE_OUTPUT_WAT: &[u8] = br#"
        (module
            (import "extism:host/env" "alloc" (func $alloc (param i64) (result i64)))
            (import "extism:host/env" "output_set" (func $output_set (param i64 i64)))
            (func (export "large_output") (result i32)
                (local $output i64)
                (local.set $output (call $alloc (i64.const 4194305)))
                (call $output_set (local.get $output) (i64.const 4194305))
                (i32.const 0)))
    "#;

    fn test_context() -> (
        Arc<super::super::permissions::PermissionChecker>,
        Arc<crate::core::PlatformContext>,
    ) {
        let manager = PluginManager::new();
        (manager.permission_checker, manager.platform_context)
    }

    fn test_plugin(wasm: &[u8]) -> extism::Plugin {
        extism::PluginBuilder::new(extism::Manifest::new([extism::Wasm::data(wasm)]))
            .with_wasi(true)
            .build()
            .expect("test plugin should compile")
    }

    fn managed_test_wrapper(
        manager: &PluginManager,
        id: &str,
        package_id: &str,
        wasm: &[u8],
    ) -> Arc<WasmPluginWrapper> {
        Arc::new(WasmPluginWrapper {
            id: id.to_string(),
            name: id.to_string(),
            description: "F05 lifecycle fixture".to_string(),
            version: "1.0.0".to_string(),
            author: "VortiDeck tests".to_string(),
            category: "TEST".to_string(),
            category_display_name: None,
            subcategory: None,
            type_color: "#000000".to_string(),
            output_schema: None,
            parameters_schema: json!({}),
            enabled: Arc::new(tokio::sync::RwLock::new(true)),
            execution_cancellation: Arc::new(tokio::sync::RwLock::new(CancellationToken::new())),
            plugin: Arc::new(tokio::sync::Mutex::new(test_plugin(wasm))),
            file_path: None,
            file_size: None,
            icon_path: None,
            node_id: None,
            plugin_bundle: Some(PluginBundleInfo {
                id: package_id.to_string(),
                name: package_id.to_string(),
                version: "1.0.0".to_string(),
                author: "VortiDeck tests".to_string(),
            }),
            permission_checker: manager.permission_checker.clone(),
            platform_context: manager.platform_context.clone(),
        })
    }

    #[test]
    fn runtime_timeout_and_memory_limit_stop_hostile_wasm() {
        let started = std::time::Instant::now();
        let mut looping = create_plugin_with_limits(
            extism::Manifest::new([extism::Wasm::data(LOOP_WAT)]),
            std::time::Duration::from_millis(40),
            16,
            10_000_000_000,
        )
        .expect("loop plugin should compile");
        let error = looping
            .call::<(), ()>("loop_forever", ())
            .expect_err("loop must be interrupted")
            .to_string();
        assert!(started.elapsed() < std::time::Duration::from_secs(2));
        assert!(
            error.contains("timeout") || error.contains("fuel"),
            "{error}"
        );

        let oversized_memory = br#"
            (module (memory 17) (func (export "noop") (result i32) (i32.const 0)))
        "#;
        let mut oversized_memory = create_plugin_with_limits(
            extism::Manifest::new([extism::Wasm::data(oversized_memory)]),
            std::time::Duration::from_secs(1),
            16,
            1_000_000,
        )
        .expect("module compilation may defer memory instantiation");
        let error = oversized_memory
            .call::<(), ()>("noop", ())
            .expect_err("memory over the host limit must fail when instantiated")
            .to_string();
        let normalized = error.to_lowercase();
        assert!(
            normalized.contains("memory") || normalized.contains("oom"),
            "{error}"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn dropped_call_interrupts_wasm_and_does_not_block_unrelated_plugin() {
        let looping = Arc::new(tokio::sync::Mutex::new(test_plugin(LOOP_WAT)));
        let unrelated = Arc::new(tokio::sync::Mutex::new(test_plugin(NOOP_WAT)));
        let (checker, context) = test_context();

        let loop_call = call_extism_bounded(
            looping.clone(),
            "loop-test".to_string(),
            checker.clone(),
            context.clone(),
            "loop_forever".to_string(),
            String::new(),
            CancellationToken::new(),
        );
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(50), loop_call)
                .await
                .is_err(),
            "dropping the caller should happen before the normal budget"
        );

        let unrelated_output = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            call_extism_bounded(
                unrelated,
                "unrelated-test".to_string(),
                checker,
                context,
                "noop".to_string(),
                String::new(),
                CancellationToken::new(),
            ),
        )
        .await
        .expect("unrelated plugin remained responsive")
        .expect("unrelated plugin call succeeded");
        assert!(unrelated_output.is_empty());
        let _released = tokio::time::timeout(std::time::Duration::from_secs(1), looping.lock())
            .await
            .expect("cancelled plugin released its execution lock");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn input_output_and_waiting_call_are_bounded() {
        let plugin = Arc::new(tokio::sync::Mutex::new(test_plugin(NOOP_WAT)));
        let held = plugin.clone().lock_owned().await;
        let (checker, context) = test_context();
        let waiting = call_extism_bounded(
            plugin.clone(),
            "wait-test".to_string(),
            checker.clone(),
            context.clone(),
            "noop".to_string(),
            String::new(),
            CancellationToken::new(),
        );
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(30), waiting)
                .await
                .is_err()
        );
        drop(held);
        tokio::task::yield_now().await;
        assert!(
            plugin.try_lock().is_ok(),
            "dropped waiter never started later"
        );

        let error = call_extism_bounded(
            plugin,
            "input-test".to_string(),
            checker.clone(),
            context.clone(),
            "noop".to_string(),
            "x".repeat(WASM_MAX_INPUT_BYTES + 1),
            CancellationToken::new(),
        )
        .await
        .expect_err("oversized input must fail before execution");
        assert!(error.contains("input exceeds"));

        let large_output = Arc::new(tokio::sync::Mutex::new(test_plugin(LARGE_OUTPUT_WAT)));
        let error = call_extism_bounded(
            large_output,
            "output-test".to_string(),
            checker,
            context,
            "large_output".to_string(),
            String::new(),
            CancellationToken::new(),
        )
        .await
        .expect_err("oversized output must be rejected");
        assert!(error.contains("output exceeds"), "{error}");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn blocked_plugin_execution_does_not_hold_the_wasm_registry() {
        let manager = Arc::new(PluginManager::new());
        let wrapper =
            managed_test_wrapper(&manager, "plugin.blocked-node", "blocked-package", NOOP_WAT);
        let held_execution = wrapper.plugin.clone().lock_owned().await;
        manager.wasm_plugins.write().await.push(wrapper);

        let executing_manager = manager.clone();
        let execution = tokio::spawn(async move {
            executing_manager
                .execute_plugin("plugin.blocked-node", "execute", json!({}))
                .await
        });
        tokio::task::yield_now().await;

        let _registry = tokio::time::timeout(
            std::time::Duration::from_millis(250),
            manager.wasm_plugins.write(),
        )
        .await
        .expect("a per-plugin wait must not retain the global WASM registry lock");

        execution.abort();
        let _ = execution.await;
        drop(held_execution);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn unloading_a_package_cancels_its_active_wasm_call() {
        let manager = Arc::new(PluginManager::new());
        let wrapper =
            managed_test_wrapper(&manager, "plugin.loop-package", "loop-package", LOOP_WAT);
        manager.wasm_plugins.write().await.push(wrapper.clone());
        manager.plugins.write().await.insert(
            wrapper.id.clone(),
            Arc::new(ExtismPluginAdapter {
                inner: wrapper.clone(),
            }),
        );

        let executing_manager = manager.clone();
        let execution = tokio::spawn(async move {
            executing_manager
                .execute_plugin("plugin.loop-package", "execute", json!({}))
                .await
        });
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                if wrapper.plugin.try_lock().is_err() {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("the fixture call should enter the WASM runtime");

        assert!(manager
            .unload_package_runtime("loop-package")
            .await
            .unwrap());
        let execution_error = tokio::time::timeout(std::time::Duration::from_secs(3), execution)
            .await
            .expect("unload should bound cancellation")
            .unwrap()
            .expect_err("the executing call must not survive package unload");
        assert!(
            execution_error.contains("lifecycle shutdown"),
            "{execution_error}"
        );
        assert!(manager.wasm_plugins.read().await.is_empty());
        assert!(!manager
            .plugins
            .read()
            .await
            .contains_key("plugin.loop-package"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "requires `pnpm --filter example-plugin-with-sdk build` first"]
    async fn built_sdk_example_emits_progress_and_custom_events_with_or_without_subscribers() {
        use crate::events::EventPayload;

        let manager = PluginManager::new();
        manager.permission_checker.grant_permissions(
            "example.text-processor",
            vec![crate::plugins::bundle::PluginPermission::Storage],
        );
        let example = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../../examples/plugin-with-sdk/dist/example-plugin-with-sdk-plugin");
        assert!(
            example.join("plugin.wasm").is_file(),
            "build the SDK example before running this acceptance test"
        );

        let mut events = manager.platform_context.event_bus.subscribe();
        manager
            .load_plugin_from_folder(&example)
            .await
            .expect("load the current built SDK example");
        let result = manager
            .execute_plugin(
                "plugin.example:text-uppercase",
                "execute",
                json!({ "input": "VortiDeck" }),
            )
            .await
            .expect("invoke the uppercase node with an EventBus subscriber");
        assert_eq!(
            result.get("output").and_then(Value::as_str),
            Some("VORTIDECK")
        );

        let mut event_types = Vec::new();
        for _ in 0..4 {
            let event = tokio::time::timeout(std::time::Duration::from_secs(1), events.recv())
                .await
                .expect("plugin event deadline")
                .expect("plugin event channel");
            let EventPayload::Plugin(event) = event else {
                panic!("the isolated example should emit only plugin events");
            };
            assert_eq!(event.package_id, "example.text-processor");
            event_types.push(event.event_type);
        }
        assert_eq!(
            event_types,
            vec!["progress", "progress", "progress", "text_processed"]
        );

        drop(events);
        let result_without_subscriber = manager
            .execute_plugin(
                "plugin.example:text-uppercase",
                "execute",
                json!({ "input": "No listener" }),
            )
            .await
            .expect("EventBus without listeners is a successful no-op");
        assert_eq!(
            result_without_subscriber
                .get("output")
                .and_then(Value::as_str),
            Some("NO LISTENER")
        );
    }
}

#[cfg(test)]
mod package_view_tests {
    use super::*;
    use crate::plugins::{Contribution, PackageInfo, WidgetCapabilities};

    fn package(id: &str, has_runtime: bool, contributions: Vec<Contribution>) -> PackageInfo {
        PackageInfo {
            id: id.to_string(),
            name: "Bundle".to_string(),
            version: "1.0.0".to_string(),
            author: None,
            description: None,
            icon_path: None,
            trust: "community".to_string(),
            contributions,
            permissions: Vec::new(),
            has_runtime,
            runtime_active: has_runtime,
            is_granted: has_runtime,
            needs_reconsent: false,
        }
    }

    #[test]
    fn runtime_and_widget_views_merge_under_manifest_identity() {
        let mut packages = vec![package("plugin.alerts", true, Vec::new())];
        packages[0].name = "Individual runtime node".to_string();
        packages[0].description = Some("Node-level description".to_string());
        let mut widget = package(
            "alerts",
            true,
            vec![Contribution::Widget {
                id: None,
                name: None,
                description: None,
                icon: None,
                schema_id: None,
                width: None,
                height: None,
                config_schema: None,
                commands: None,
                deck_surface: None,
                capabilities: WidgetCapabilities::default(),
                content: "<p>alert</p>".to_string(),
            }],
        );
        widget.name = "Alerts package".to_string();
        widget.description = Some("Canonical manifest description".to_string());
        widget.permissions = vec![crate::plugins::PluginPermission::Storage];
        let grants = std::collections::HashSet::from(["alerts".to_string()]);

        PluginManager::merge_widget_package_view(&mut packages, widget, &grants);

        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].id, "alerts");
        assert!(packages[0].has_runtime);
        assert!(packages[0].is_granted);
        assert_eq!(packages[0].contributions.len(), 1);
        assert_eq!(packages[0].name, "Alerts package");
        assert_eq!(
            packages[0].description.as_deref(),
            Some("Canonical manifest description")
        );
        assert_eq!(packages[0].permissions.len(), 1);
    }

    #[tokio::test]
    async fn discovered_package_cannot_shadow_embedded_builtin_id() {
        let manager = PluginManager::new();
        manager
            .register_available_widget(package("builtin.vortideck-widgets", false, Vec::new()))
            .await;

        let packages = manager.list_available_widgets().await;
        let builtin = packages
            .iter()
            .find(|package| package.id == "builtin.vortideck-widgets")
            .expect("embedded builtin package");
        assert_eq!(builtin.trust, "builtin");
        assert_eq!(builtin.contributions.len(), 8);
    }

    #[tokio::test]
    async fn web_extension_lookup_uses_canonical_package_identity() {
        let manager = PluginManager::new();
        manager.web_extensions.write().await.insert(
            "plugin.text-processor-plugin".to_string(),
            crate::plugins::bundle::WebResources {
                extension_js: Some("window.extensionLoaded = true;".to_string()),
                styles_css: Some("body { color: blue; }".to_string()),
                assets: std::collections::HashMap::new(),
            },
        );

        for requested_id in ["text-processor-plugin", "plugin.text-processor-plugin"] {
            let files = manager.get_web_extensions_for_package(requested_id).await;
            assert_eq!(files.len(), 2);
            assert!(files
                .iter()
                .all(|file| file.plugin_id == "text-processor-plugin"));
        }

        assert_eq!(
            manager.list_web_extension_package_ids().await,
            std::collections::HashSet::from(["text-processor-plugin".to_string()])
        );
    }

    #[test]
    fn plugin_event_principal_and_category_are_host_owned() {
        let malicious = serde_json::json!({
            "category": "stream",
            "payload": { "platform": "twitch" },
            "eventType": "raid"
        });
        let event = plugin_event_from_untrusted_json("plugin.example", &malicious.to_string())
            .expect("host-stamped plugin event");
        match event {
            crate::events::EventPayload::Plugin(event) => {
                assert_eq!(event.package_id, "example");
                assert_eq!(event.event_type, "raid");
                assert_eq!(event.payload["platform"], "twitch");
            }
            other => panic!("unexpected event category: {other:?}"),
        }
    }

    #[test]
    fn plugin_event_rejects_invalid_names_and_oversized_payloads() {
        assert!(plugin_event_from_untrusted_json(
            "plugin.example",
            r#"{"eventType":"stream:raid","data":{}}"#,
        )
        .is_err());
        let oversized = format!(
            r#"{{"eventType":"status","data":"{}"}}"#,
            "x".repeat(256 * 1024)
        );
        assert!(plugin_event_from_untrusted_json("plugin.example", &oversized).is_err());
    }
}

#[cfg(test)]
mod archive_limit_tests {
    use super::*;

    #[test]
    fn bounded_entry_copy_counts_actual_bytes_not_declared_size() {
        let mut out = Vec::new();
        assert_eq!(
            PluginManager::copy_archive_entry_bounded(&[7_u8; 16][..], &mut out, 16).unwrap(),
            Some(16)
        );
        let mut out = Vec::new();
        assert_eq!(
            PluginManager::copy_archive_entry_bounded(&[7_u8; 17][..], &mut out, 16).unwrap(),
            None
        );
        assert!(out.len() <= 17, "copy must stop just past the limit");
    }

    #[tokio::test]
    async fn plugin_asset_rejects_traversing_package_id() {
        let manager = PluginManager::new();
        assert!(manager
            .get_plugin_asset("../outside", "web/assets/icon.svg")
            .await
            .is_none());
        assert!(manager
            .get_plugin_asset("plugin...", "icon.svg")
            .await
            .is_none());
    }
}
