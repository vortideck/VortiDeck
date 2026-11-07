#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(warnings)]

// Initialize rustls crypto provider for WebSocket plugin
use rustls::crypto::CryptoProvider;

mod create_websocket_mdns;
// mod mcp; // Moved to vortideck-core
mod command_handler;
mod power_events;
mod relay_connection;
mod streaming_connection;
mod tailscale;
// e2ee_handler and relay_client now in lib.rs
// Re-export modules from lib so commands module can use crate::*
use crate::utils::{build_main_window, check_premium_status, ensure_window, generate_token};
// Shared host modules are compiled by the library exactly once. Declaring the
// same files again with `mod` in this binary creates distinct Rust types, so a
// lifecycle service in `app_lib` cannot retrieve state managed by the binary.
use app_lib::crypto;
use app_lib::crypto::{decrypt_with_secret, encrypt_with_secret};
use app_lib::e2ee_handler;
use app_lib::relay_client;
pub use app_lib::{
    adapters, analytics, broadcast_helper, client_registry, commands, db, marketplace, overlay,
    package_lifecycle, utils, widget_document_cache,
};
// NEW ARCHITECTURE: Import flow components from VortiDeckCore services
use vortideck_core::services::flow::commands::{FlowExecutorState, FlowRegistryState};
use vortideck_core::services::flow::traits::FlowNodeProvider;
use vortideck_core::services::flow::HttpNodesProvider;
use vortideck_core::services::flow::StandardNodeProvider;
use vortideck_core::services::streaming::StreamingState;
use vortideck_core::services::system::flow_adapter::SystemActionProvider;
// ChatState import moved below after adapters module declaration

// mod create_websocket_dns;

// use create_websocket_dns::create_websocket_dns;
use create_websocket_mdns::create_websocket_mdns;
mod banners;
mod plugins;
mod webhook;
// 🎯 Import OBS Banner Commands
use commands::obs_banner_commands;
// 🎨 Import overlay system
use overlay::OverlayState;
// 🔌 Plugin content state for vd-plugin:// protocol
use tokio::sync::RwLock;
#[derive(Default)]
pub struct PluginContentState(pub Arc<RwLock<HashMap<String, String>>>);
// mod system_control; // Moved to vortideck-core
use futures_util::{SinkExt, StreamExt};
use local_ip_address::local_ip;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{Listener, Manager};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio::time::{sleep, timeout, Duration};
use tokio_tungstenite::{accept_async, tungstenite::Message};
// StateBridge removed - using direct StateManager
use vortideck_core::core::{ConnectionCandidate, StateManager};
// 🚀 UNIVERSAL ARCHITECTURE: Import platform abstractions and universal functions
use enigo::{Enigo, Settings};
use vortideck_core::core::{PlatformContext, SystemAction, SystemInfo};
use vortideck_core::services::system::{
    execute_system_action_universal, get_available_actions_universal, get_system_info_universal,
    set_volume_universal,
};
// ✅ NEW ARCHITECTURE: RecordingState is managed by VortiDeckCore, not as separate state
use platform_info::*;
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use std::net::IpAddr;
use std::net::SocketAddr;
use tauri::State;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use tauri::{AppHandle, Emitter};
use tauri_plugin_store::StoreExt;
use tokio::sync::{mpsc, Mutex as TokioMutex, Semaphore};
use tokio::task::JoinHandle;
use vortideck_core::db::db::*;
use vortideck_core::db::models::NewPeer;

use std::sync::OnceLock; // OnceLock is part of std now
static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();
const DEFAULT_UNIFIED_SERVER_PORT: u16 = 9001;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConnectionDiagnosticsSnapshot {
    server: Option<vortideck_core::core::SerializableServerInfo>,
    relay: serde_json::Value,
    mdns: create_websocket_mdns::MdnsStatus,
}

#[tauri::command]
async fn get_connection_diagnostics(
    state_manager: State<'_, Arc<StateManager>>,
    relay_client_state: State<'_, commands::relay_commands::RelayClientState>,
) -> Result<ConnectionDiagnosticsSnapshot, String> {
    let server = state_manager.get_server_info();
    let relay = commands::relay_commands::relay_status_snapshot(
        state_manager.inner(),
        relay_client_state.inner(),
    )
    .await;
    let mdns = create_websocket_mdns::mdns_status_snapshot()?;
    Ok(ConnectionDiagnosticsSnapshot {
        server,
        relay,
        mdns,
    })
}

// ✅ REMOVED: BUTTON_SEMAPHORE - concurrency now handled by UniversalExecutionEngine's internal semaphore

// E2EE Pending Pairings State
// Stores temporary E2EEKeys during pairing process (QR scan → handshake)
// Keys are removed after successful handshake and shared secret is stored
use app_lib::crypto::E2EEKeys;
use x25519_dalek::PublicKey; // For ECDH key exchange
type PendingPairingsMap = Arc<RwLock<HashMap<String, E2EEKeys>>>;

// Button repeat state management
#[derive(Debug)]
struct ButtonRepeatState {
    button_id: i32,
    repeat_handle: Option<JoinHandle<()>>,
    stop_sender: Option<broadcast::Sender<()>>,
}

type ButtonRepeatMap = Arc<RwLock<HashMap<i32, ButtonRepeatState>>>;

fn app_handle<'a>() -> &'a AppHandle {
    APP_HANDLE.get().unwrap()
}

struct SetupState {
    frontend_task: bool,
    backend_task: bool,
}

/// Execute a desktop widget interaction through the same persisted-binding
/// resolver used by direct and relay mobile clients. The renderer supplies an
/// intent only; action payloads and execution policy remain host-owned.
#[tauri::command]
async fn execute_deck_widget_intent(
    payload: Value,
    window: tauri::Window,
    app_handle: AppHandle,
    db_pool: State<'_, DbPool>,
    enigo: State<'_, Arc<TokioMutex<Enigo>>>,
    state_manager: State<'_, Arc<StateManager>>,
) -> Result<(), String> {
    if payload.get("action").and_then(Value::as_str) != Some("deck_widget_intent") {
        return Err("Invalid deck widget intent envelope".to_string());
    }

    let tx = state_manager
        .get_server_sender()
        .unwrap_or_else(|| broadcast::channel::<Message>(100).0);
    let obs_integration = Arc::new(
        vortideck_core::integrations::builtin::obs::ObsIntegration::new(
            state_manager.inner().clone(),
        ),
    );
    let ctx = command_handler::CommandExecutionContext {
        json: payload,
        obs_integration,
        db: Database::new(db_pool.inner()),
        tx,
        enigo: enigo.inner().clone(),
        app_handle,
    };
    static DESKTOP_WIDGET_SESSION: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    let source = command_handler::WidgetIntentSource {
        client_id: format!("desktop:{}", window.label()),
        connection_session_id: DESKTOP_WIDGET_SESSION
            .get_or_init(|| uuid::Uuid::new_v4().to_string())
            .clone(),
    };
    command_handler::handle_widget_intent_action(ctx, source).await;
    Ok(())
}

// ✅ NEW ARCHITECTURE: RecordingState managed internally by VortiDeckCore

// Temporary type stubs for legacy types during migration
#[derive(Debug, serde::Deserialize)]
struct SerializedMacroStep {
    action: ActionExecutable,
    delay: u64,
}

#[derive(Debug, serde::Deserialize)]
enum ActionExecutable {
    KeyDown(String),
    KeyUp(String),
    MouseMove { x: i32, y: i32 },
    MouseClick(String),
    ObsAction(String),
    SystemControl(String),
}

// A custom task for setting the state of a setup task (splashscreen)
#[tauri::command]
async fn set_complete(
    app: AppHandle,
    state: State<'_, tokio::sync::Mutex<SetupState>>,
    task: String,
) -> Result<(), ()> {
    // Lock the state without write access
    let mut state_lock = state.lock().await;
    match task.as_str() {
        "frontend" => state_lock.frontend_task = true,
        "backend" => state_lock.backend_task = true,
        _ => {
            println!("⚠️ Warning: Unknown task '{}' - ignoring", task);
            return Err(());
        }
    }
    // Check if both tasks are completed
    if state_lock.frontend_task {
        // Setup is complete, we can close the splashscreen
        // and unhide the main window!
        println!("set_complete frontend");

        // Align main window position to splashscreen center so it appears seamlessly in-place
        if let Some(splash_window) = app.get_webview_window("splashscreen") {
            if let Ok(splash_pos) = splash_window.outer_position() {
                if let Some(main_window) = app.get_webview_window("main") {
                    if let (Ok(splash_size), Ok(main_size)) =
                        (splash_window.inner_size(), main_window.inner_size())
                    {
                        let center_x = splash_pos.x + (splash_size.width as i32 / 2);
                        let center_y = splash_pos.y + (splash_size.height as i32 / 2);
                        let main_x = center_x - (main_size.width as i32 / 2);
                        let main_y = center_y - (main_size.height as i32 / 2);
                        let _ =
                            main_window.set_position(tauri::PhysicalPosition::new(main_x, main_y));
                    } else {
                        let _ = main_window.set_position(splash_pos);
                    }
                }
            }
            let _ = splash_window.close();
        }

        if let Some(main_window) = app.get_webview_window("main") {
            // On a cold Vite dependency prebundle WebView2 can give up its
            // first navigation while the splash page later becomes healthy.
            // Retry only failed development documents, using Tauri's configured
            // devUrl so this never creates another hard-coded port contract.
            #[cfg(dev)]
            if main_window
                .url()
                .is_ok_and(|url| matches!(url.scheme(), "about" | "chrome-error"))
                && let Some(dev_url) = app.config().build.dev_url.clone()
            {
                if let Err(error) = main_window.navigate(dev_url) {
                    log::warn!("Failed to retry main development navigation: {error}");
                }
            }
            let _ = main_window.show();
            let _ = main_window.set_focus();
        }
    }
    Ok(())
}

#[tauri::command]
async fn show_window_if_complete(
    app: AppHandle,
    window_label: &str,
    state: State<'_, tokio::sync::Mutex<SetupState>>,
) -> Result<(), ()> {
    let setup_state = state.lock().await;
    if setup_state.frontend_task {
        if let Some(window) = app.get_webview_window(window_label) {
            let _ = window.show();
        }
    }
    Ok(())
}

/// Trigger plugin loading with progress events for splashscreen
/// This is called manually from the splashscreen after event listeners are set up
#[tauri::command]
async fn trigger_plugin_loading(
    app: AppHandle,
    vortideck_core: State<'_, VortiDeckCore>,
    flow_registry: State<'_, FlowRegistryState>,
    db_pool: State<'_, DbPool>,
) -> Result<usize, String> {
    use std::sync::atomic::{AtomicBool, Ordering};
    static LOADING_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

    struct LoadingGuard<'a>(&'a AtomicBool);
    impl Drop for LoadingGuard<'_> {
        fn drop(&mut self) {
            self.0.store(false, Ordering::SeqCst);
        }
    }

    // Prevent duplicate concurrent calls (HMR re-mounts call this multiple times)
    if LOADING_IN_PROGRESS.swap(true, Ordering::SeqCst) {
        println!("🔌 [trigger_plugin_loading] Already in progress, skipping duplicate call");
        let _ = app.emit(
            "plugin-loading-progress",
            serde_json::json!({
                "status": "completed",
                "message": "Plugins already loaded",
            }),
        );
        return Ok(0);
    }
    let _loading_guard = LoadingGuard(&LOADING_IN_PROGRESS);

    println!("🔌 [trigger_plugin_loading] Manually triggered by splashscreen");

    package_lifecycle::recover_interrupted_operations(db_pool.inner()).await?;

    // One-time, prefix-compatible migration for legacy .vdext archives older releases left
    // beside (or inside) the plugins directory. Extraction is atomic and only
    // discovers the package; durable consent below still decides activation.
    let plugins_dir = VortiDeckCore::get_default_plugins_directory();
    let mut legacy_roots = vec![plugins_dir.clone()];
    if let Some(parent) = plugins_dir.parent() {
        legacy_roots.push(parent.to_path_buf());
    }
    for root in legacy_roots {
        let Ok(entries) = std::fs::read_dir(&root) else {
            continue;
        };
        for archive in entries.flatten().map(|entry| entry.path()).filter(|path| {
            path.extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("vdext"))
        }) {
            match vortideck_core
                .plugin_manager()
                .stage_plugin_bundle(&archive)
                .await
            {
                Ok(package_id) => log::info!(
                    "Migrated legacy package archive '{}' as '{}'",
                    archive.display(),
                    package_id
                ),
                Err(error) if error.contains("already exists") => {
                    log::info!("Legacy archive already migrated: {}", archive.display());
                }
                Err(error) => log::error!(
                    "Could not migrate legacy package archive '{}': {}",
                    archive.display(),
                    error
                ),
            }
        }
    }

    // Scan for canonical package folders.
    let (_, plugin_files) = vortideck_core.scan_plugins_directory(&plugins_dir)?;

    let mut package_folders = plugin_files;
    package_folders.sort();
    let total = package_folders.len();

    // Emit initial progress event (0/total)
    let _ = app.emit(
        "plugin-loading-progress",
        serde_json::json!({
            "status": "progress",
            "current": 0,
            "total": total,
        }),
    );

    // Startup, updates, auth changes, and manual retry all share one serialized
    // consent/quota/rollback coordinator. This prevents startup and watcher
    // races from spawning the same service runtime twice.
    let mut count = 0;
    for (index, plugin_path) in package_folders.into_iter().enumerate() {
        let event = vortideck_core::plugins::PluginChangeEvent {
            event_type: "modified".to_string(),
            plugin_path: plugin_path.to_string_lossy().to_string(),
            plugin_id: None,
        };
        if let Err(error) = package_lifecycle::reconcile_file_change(
            &app,
            vortideck_core.inner(),
            db_pool.inner(),
            &event,
        )
        .await
        {
            log::error!(
                "Startup package reconciliation failed for '{}': {}",
                plugin_path.display(),
                error
            );
        }
        if let Ok(manifest_json) = std::fs::read_to_string(plugin_path.join("manifest.json")) {
            if let Ok(manifest) =
                serde_json::from_str::<vortideck_core::plugins::BundleManifest>(&manifest_json)
            {
                if vortideck_core
                    .plugin_manager()
                    .is_package_runtime_loaded(&manifest.id)
                    .await
                {
                    count += 1;
                }
            }
        }
        let _ = app.emit(
            "plugin-loading-progress",
            serde_json::json!({
                "status": "progress",
                "current": index + 1,
                "total": total,
            }),
        );
    }

    // Register plugin flow nodes (WASM + Service)
    let wasm_provider = vortideck_core.get_wasm_flow_provider().await;
    let service_providers = vortideck_core
        .plugin_manager()
        .get_service_flow_providers()
        .await;

    let mut registry = flow_registry.registry.lock().unwrap();

    // Register WASM plugins
    let wasm_node_count = wasm_provider.get_flow_nodes().len();
    if wasm_node_count > 0 {
        registry.register_provider(Box::new(wasm_provider));
    }

    // Register service plugins
    let mut service_node_count = 0;
    for provider in service_providers {
        service_node_count += provider.nodes.len();
        registry.register_provider(Box::new(provider));
    }

    if wasm_node_count > 0 || service_node_count > 0 {
        println!(
            "🎯 Registered {} plugin flow nodes - total nodes: {} ({} WASM + {} Service)",
            wasm_node_count + service_node_count,
            registry.get_node_count(),
            wasm_node_count,
            service_node_count
        );
    }

    // Emit completion event
    let _ = app.emit("plugin-loading-progress", serde_json::json!({
        "status": "completed",
        "current": count,
        "total": total,
        "message": if count == 1 { "Loaded 1 plugin".to_string() } else { format!("Loaded {} plugins", count) },
    }));

    Ok(count)
}

#[derive(Deserialize, Serialize)]
struct CommandMessage {
    action: String,
    command: String,
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
use std::process::Command;
use tokio::sync::Mutex;

#[cfg(feature = "acceptance-test-hooks")]
fn write_packaged_acceptance_marker(payload: serde_json::Value) {
    let Some(path) = std::env::var_os("VORTIDECK_ACCEPTANCE_MARKER") else {
        log::error!("Packaged acceptance marker path is missing");
        return;
    };
    let path = std::path::PathBuf::from(path);
    if let Some(parent) = path.parent() {
        if let Err(error) = std::fs::create_dir_all(parent) {
            log::error!("Could not create packaged acceptance marker directory: {error}");
            return;
        }
    }
    if let Err(error) = std::fs::write(&path, payload.to_string()) {
        log::error!("Could not write packaged acceptance marker: {error}");
    }
}

fn packaged_acceptance_enabled() -> bool {
    #[cfg(feature = "acceptance-test-hooks")]
    {
        return std::env::var_os("VORTIDECK_ACCEPTANCE_SCENARIO").is_some();
    }

    #[cfg(not(feature = "acceptance-test-hooks"))]
    false
}

fn setup<'a>(app: &'a mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    APP_HANDLE.set(app.app_handle().to_owned()).unwrap();

    app.handle().listen("deck_widget_runtime_reset", |event| {
        let payload = serde_json::from_str::<Value>(event.payload()).unwrap_or_default();
        let deck_id = payload.get("deckId").and_then(Value::as_i64).unwrap_or(-1) as i32;
        let instance_ids: Vec<String> = payload
            .get("instanceIds")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect();
        tauri::async_runtime::spawn(async move {
            command_handler::cancel_removed_widget_repeats(deck_id, &instance_ids).await;
        });
    });

    app.handle().listen("widget_package_revoked", |_| {
        tauri::async_runtime::spawn(async {
            command_handler::cancel_all_widget_repeats().await;
        });
    });
    let integration_state_app = app.handle().clone();
    app.handle()
        .listen("integration_status_changed", move |event| {
            let app_handle = integration_state_app.clone();
            let values = serde_json::from_str::<Value>(event.payload()).unwrap_or_default();
            tauri::async_runtime::spawn(async move {
                let integration_id = values
                    .get("integration_id")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                let connected =
                    values.pointer("/status/status").and_then(Value::as_str) == Some("Connected");
                command_handler::publish_widget_topic_for_active_deck(
                    &app_handle,
                    "integration.status",
                    values.clone(),
                )
                .await;
                if let Some(integration_id) = integration_id {
                    command_handler::publish_widget_topic_availability_for_active_deck(
                        &app_handle,
                        &format!("integration.{}.status", integration_id),
                        values,
                        connected,
                    )
                    .await;
                }
            });
        });

    let telemetry_app = app.handle().clone();
    tauri::async_runtime::spawn(async move {
        use sysinfo::System;
        let mut system = System::new_all();
        loop {
            system.refresh_cpu();
            system.refresh_memory();
            let has_subscribers = command_handler::publish_widget_topic_for_active_deck(
                &telemetry_app,
                "telemetry",
                json!({
                    "cpu": system.global_cpu_info().cpu_usage(),
                    "usedMemory": system.used_memory(),
                    "totalMemory": system.total_memory(),
                    "memory": if system.total_memory() == 0 { 0.0 } else {
                        system.used_memory() as f64 * 100.0 / system.total_memory() as f64
                    },
                    "timestamp": chrono::Utc::now().timestamp_millis(),
                }),
            )
            .await;
            sleep(if has_subscribers {
                Duration::from_secs(2)
            } else {
                Duration::from_secs(10)
            })
            .await;
        }
    });

    println!("🚀 VortiDeck starting up...");
    log::info!("🚀 VortiDeck starting up...");

    #[cfg(any(windows, target_os = "linux"))]
    {
        use tauri_plugin_deep_link::DeepLinkExt;
        app.deep_link().register_all()?;
    }

    // Initialize context menu config and auto-register shell extension
    #[cfg(windows)]
    {
        if let Ok(exe_path) = std::env::current_exe() {
            commands::context_menu_commands::ensure_config(&exe_path.to_string_lossy());
        }
        commands::context_menu_commands::try_auto_register(app.handle());
    }

    #[cfg(desktop)]
    {
        app.handle().plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))?;

        // `tauri dev` executes the binary from Cargo's target directory. If a
        // developer enables autostart there, Windows persists that transient
        // debug path and launches a WebView that cannot reach Nuxt after the
        // next sign-in. Autostart is a packaged-application feature only.
        #[cfg(debug_assertions)]
        {
            use tauri_plugin_autostart::ManagerExt;
            if app.autolaunch().is_enabled()? {
                app.autolaunch().disable()?;
                log::warn!("Disabled stale autostart registration for debug VortiDeck");
            }
        }
    }

    // std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
    let app_handle = app.handle().clone();
    // let store = app.store("store.json")?;
    // store.set("some-key", json!({ "value": 5 }));

    // ✅ NEW ARCHITECTURE: RecordingState is managed internally by VortiDeckCore
    // 🔄 MIGRATION: OBS adapter removed - using StateManager directly
    let setup_state = Mutex::new(SetupState {
        frontend_task: false,
        backend_task: false,
    });

    // 🚀 UNIVERSAL ARCHITECTURE: Initialize platform context early (before node registration)
    let platform_context = Arc::new(create_tauri_platform_context(app_handle.clone()));
    println!("🚀 Platform context initialized");

    // 🌉 Start EventBus → Tauri IPC bridge (forward events to frontend)
    spawn_event_bridge(app_handle.clone(), platform_context.event_bus.clone());
    println!("🌉 Event bridge started - EventBus → Tauri IPC");

    // Initialize flow registry
    let flow_registry_state = FlowRegistryState::new();
    let flow_executor_state = FlowExecutorState::new();

    // Initialize MCP adapter chat state
    let chat_state = vortideck_core::services::mcp::commands::ChatState::new_with_flow_executor(
        Some(flow_executor_state.executor.clone()),
    );

    // 🔌 Initialize MCP servers globally at startup (once for all sessions)
    let chat_state_mcp_init = chat_state.clone();
    let app_handle_mcp_init = app_handle.clone();
    let flow_registry_state_mcp = flow_registry_state.clone();
    tauri::async_runtime::spawn(async move {
        println!("🔌 Initializing MCP servers globally at startup...");

        // Load MCP config
        let config_path = match utils::ensure_mcp_config_path(&app_handle_mcp_init) {
            Ok(path) => path,
            Err(e) => {
                println!("❌ Failed to resolve MCP config: {}", e);
                return;
            }
        };

        let config = match vortideck_core::services::mcp::config::Config::load(&config_path).await {
            Ok(cfg) => cfg,
            Err(e) => {
                println!(
                    "⚠️ No MCP config found (this is OK if not using MCP): {}",
                    e
                );
                return;
            }
        };

        // Connect to MCP servers if configured
        if let Some(mcp_config) = &config.mcp {
            use vortideck_core::services::mcp::handler::VortiDeckClientHandler;
            use vortideck_core::services::mcp::tool::get_mcp_tools;

            println!(
                "🔗 Connecting to {} MCP servers...",
                mcp_config.server.len()
            );

            let event_tx = adapters::mcp_commands::spawn_mcp_client_event_forwarder(
                app_handle_mcp_init.clone(),
                chat_state_mcp_init.clone(),
                flow_registry_state_mcp.clone(),
            );

            for (idx, server) in mcp_config.server.iter().enumerate() {
                println!(
                    "[{}/{}] Connecting to MCP server: {}",
                    idx + 1,
                    mcp_config.server.len(),
                    server.name
                );

                let handler =
                    VortiDeckClientHandler::for_server(server.name.clone(), event_tx.clone());

                match server.transport.start(Some(handler)).await {
                    Ok(client) => {
                        println!("✅ Connected to MCP server: {}", server.name);

                        // Load tools from this server
                        match get_mcp_tools(client.peer().clone()).await {
                            Ok(tools) => {
                                println!("✅ Loaded {} tools from {}", tools.len(), server.name);

                                if let Err(error) = chat_state_mcp_init
                                    .replace_server_tools(&server.name, &tools)
                                    .await
                                {
                                    log::error!(
                                        "Rejected tools from MCP server {}: {}",
                                        server.name,
                                        error
                                    );
                                    continue;
                                }
                                let toolset = chat_state_mcp_init.mcp_tools.lock().await.clone();
                                let mut registry = flow_registry_state_mcp.registry.lock().unwrap();
                                registry.clear_provider_nodes("McpToolProvider");
                                registry.register_provider(Box::new(
                                    vortideck_core::services::mcp::flow_adapter::McpToolProvider::new(
                                        toolset,
                                    ),
                                ));
                            }
                            Err(e) => {
                                println!("⚠️ Failed to load tools from {}: {}", server.name, e);
                            }
                        }

                        chat_state_mcp_init
                            .mcp_clients
                            .lock()
                            .await
                            .insert(server.name.clone(), client);
                    }
                    Err(e) => {
                        println!("❌ Failed to connect to MCP server {}: {}", server.name, e);
                    }
                }
            }

            let clients_state = chat_state_mcp_init.mcp_clients.lock().await;
            println!(
                "✅ Global MCP initialization complete: {} servers connected",
                clients_state.len()
            );
        } else {
            println!("ℹ️ No MCP servers configured in config");
        }

        // Store config in state after processing
        {
            let mut config_state = chat_state_mcp_init.config.lock().await;
            *config_state = Some(config);
        }
    });

    // 🚀 NEW ARCHITECTURE: Initialize VortiDeckCore and load plugins
    let vortideck_core = VortiDeckCore::new(platform_context.clone());
    println!("🚀 VortiDeckCore initialized");

    // Plugin loading moved to combined block below with flow provider registration

    // 🔥 START PLUGIN WATCHER FOR HOT RELOAD
    if !packaged_acceptance_enabled() {
        let app_handle_watcher = app_handle.clone();
        let vortideck_core_arc = Arc::new(vortideck_core.clone());
        let flow_registry_watcher = flow_registry_state.clone();
        let chat_state_watcher = chat_state.clone();
        tauri::async_runtime::spawn(async move {
            log::info!("🔥 Starting plugin watcher with hot reload...");

            let plugins_dir = VortiDeckCore::get_default_plugins_directory();

            // Create watcher with event emission
            match vortideck_core::plugins::PluginWatcher::with_events(plugins_dir) {
                Ok((watcher, mut event_receiver)) => {
                    // Start the file system monitoring
                    if let Err(e) = watcher.start_watching(vortideck_core_arc.clone()).await {
                        println!("❌ Failed to start plugin watcher: {}", e);
                        return;
                    }

                    println!("✅ Plugin watcher started - monitoring for changes");

                    // Bridge plugin events to frontend
                    while let Some(event) = event_receiver.recv().await {
                        println!(
                            "📡 Plugin event: {} - {}",
                            event.event_type, event.plugin_path
                        );

                        let Some(db_pool_state) = app_handle_watcher.try_state::<DbPool>() else {
                            log::warn!(
                                "Ignoring an early package filesystem event before the database is ready: {}",
                                event.plugin_path
                            );
                            continue;
                        };
                        let db_pool_watcher = db_pool_state.inner().clone();
                        drop(db_pool_state);
                        let should_refresh = match package_lifecycle::reconcile_file_change(
                            &app_handle_watcher,
                            &vortideck_core_arc,
                            &db_pool_watcher,
                            &event,
                        )
                        .await
                        {
                            Ok(refresh) => refresh,
                            Err(error) => {
                                log::error!(
                                    "Package lifecycle reconciliation failed for {}: {}",
                                    event.plugin_path,
                                    error
                                );
                                false
                            }
                        };

                        // Refresh providers only after the lifecycle coordinator
                        // has committed activation, quarantine, rollback, or removal.
                        if should_refresh {
                            let context = format!("plugin hot reload ({})", event.event_type);
                            refresh_flow_registry_with_plugins(
                                &vortideck_core_arc,
                                &flow_registry_watcher,
                                &chat_state_watcher,
                                &context,
                            )
                            .await;
                        }

                        // Emit Tauri event to frontend
                        if let Err(e) = app_handle_watcher.emit("plugin-changed", &event) {
                            println!("❌ Failed to emit plugin-changed event: {}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Failed to create plugin watcher: {}", e);
                }
            }
        });
    }

    // 🗑️ LEGACY SYSTEM REMOVED - No more old plugin registry!

    // 🔌 DON'T auto-load plugins on startup - let splashscreen trigger it when ready
    // This prevents race conditions where events are emitted before listeners are set up
    println!("🔌 Plugin loading will be triggered by splashscreen when ready");

    // Register built-in flow node providers (synchronously) - standard, HTTP, and system nodes
    {
        let standard_provider = StandardNodeProvider::new();
        let http_provider = HttpNodesProvider;
        let system_native_provider = SystemActionProvider::new(Some(platform_context.clone()));
        let system_provider = vortideck_core::services::flow::SystemProviderAdapter::new(Box::new(
            system_native_provider,
        ));

        let mut registry = flow_registry_state.registry.lock().unwrap();
        registry.register_provider(Box::new(standard_provider));
        registry.register_provider(Box::new(http_provider));
        registry.register_provider(Box::new(system_provider));

        let node_count = registry.get_node_count();
        println!(
            "🎯 Registered built-in flow node providers (standard + HTTP + system) with {} initial nodes",
            node_count
        );
    }

    let migration_outcome = initialize_database()
        .map_err(|error| format!("Failed to initialize VortiDeck database: {error}"))?;
    if !migration_outcome.applied_versions.is_empty() {
        log::info!(
            "Applied database migrations: {}",
            migration_outcome.applied_versions.join(", ")
        );
    }
    if let Some(backup_path) = migration_outcome.backup_path {
        log::info!(
            "Created pre-migration database backup at {}",
            backup_path.display()
        );
    }
    let db_pool = establish_pool();

    // 🔄 Register JWT change callback for dynamic plugin reloading (BEFORE managing the core)
    let vortideck_core_jwt = vortideck_core.clone();
    let flow_registry_jwt = flow_registry_state.clone();
    let chat_state_jwt = chat_state.clone();
    let app_handle_jwt = app_handle.clone();
    vortideck_core::features::jwt_features::register_jwt_change_callback(move |event| {
        println!("🎯 JWT Change Detected: {:?}", event.event_type);

        // Spawn async task to reload plugins AND refresh flow registry
        let core = vortideck_core_jwt.clone();
        let flow_registry = flow_registry_jwt.clone();
        let chat_state_task = chat_state_jwt.clone();
        let app_handle = app_handle_jwt.clone();
        tauri::async_runtime::spawn(async move {
            let Some(db_pool) = app_handle.try_state::<DbPool>() else {
                log::warn!("Skipping package reconciliation before the database is ready");
                return;
            };
            let db_pool = db_pool.inner().clone();

            // Rebuild the active set through the same durable-consent and quota
            // coordinator used by file updates. The previous core-only reload
            // path knew account limits but could not validate Diesel grants.
            for package in core.plugin_manager().list_discovered_packages().await {
                if package.has_runtime {
                    let _ = core
                        .plugin_manager()
                        .unload_package_runtime(&package.id)
                        .await;
                }
            }
            let plugins_dir = VortiDeckCore::get_default_plugins_directory();
            let mut folders = std::fs::read_dir(&plugins_dir)
                .ok()
                .into_iter()
                .flatten()
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.is_dir() && path.join("manifest.json").exists())
                .collect::<Vec<_>>();
            folders.sort();
            for folder in folders {
                let event = vortideck_core::plugins::PluginChangeEvent {
                    event_type: "modified".to_string(),
                    plugin_path: folder.to_string_lossy().to_string(),
                    plugin_id: None,
                };
                if let Err(error) =
                    package_lifecycle::reconcile_file_change(&app_handle, &core, &db_pool, &event)
                        .await
                {
                    log::error!("JWT package reconciliation failed: {}", error);
                }
            }
            refresh_flow_registry_with_plugins(
                &core,
                &flow_registry,
                &chat_state_task,
                "JWT authentication",
            )
            .await;
        });
    });

    let db_pool_for_engine = db_pool.clone();
    let db_pool_for_webhooks = db_pool.clone();
    let db_pool_for_e2ee = db_pool.clone(); // 🔐 PHASE 2: Clone for E2EE restore
    let db_pool_for_relay = db_pool.clone(); // For relay command handler
    let db_pool_for_streaming = db_pool.clone(); // 🎬 For streaming session persistence
    let db_pool_for_flows = db_pool.clone(); // 🔗 For flow executor (hybrid storage)
    app.manage(db_pool);

    // ── Timeout safety net (Layer 2) ─────────────────────────────────────
    // Polls every 15s: as soon as the main window is visible (splashscreen
    // called `show_window_if_complete`), the check cancels (no dialog). Only
    // if the window is STILL not visible after 180s → genuine failure → dialog.
    {
        let handle = app.handle().clone();
        tauri::async_runtime::spawn(async move {
            for _ in 0..12 {
                tokio::time::sleep(std::time::Duration::from_secs(15)).await;
                if let Some(window) = handle.get_webview_window("main") {
                    if window.is_visible().unwrap_or(false) {
                        return; // Window visible → app started → cancel.
                    }
                }
            }
            // 180s elapsed, window still not visible → genuine failure.
            log::warn!("⚠️ Main window not visible after 180s — page-load failure");
            if let Some(window) = handle.get_webview_window("main") {
                let _ = window.show();
            }
            use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};
            let log_dir = handle.path().app_log_dir().ok();
            handle
                .dialog()
                .message(
                    "VortiDeck failed to load. The main window couldn't start.\n\n\
                     This is usually a build or server issue.\n\n\
                     Click OK to open the logs folder, or Cancel to dismiss.",
                )
                .title("VortiDeck — Load Failure")
                .buttons(MessageDialogButtons::OkCancel)
                .show(move |ok| {
                    if ok {
                        let dir_clone = log_dir.clone();
                        tauri::async_runtime::spawn(async move {
                            // Report to backend (same as the crash dialog).
                            let timeout_error = serde_json::json!({
                                "message": "Main window failed to load (timeout)",
                                "source": "timeout-safety-net",
                                "stack": format!("App: {} | OS: {}", env!("CARGO_PKG_VERSION"), std::env::consts::OS),
                            });
                            match commands::error_commands::report_to_backend(&timeout_error).await {
                                Ok(()) => log::info!("✅ Load-timeout reported to developers"),
                                Err(e) => log::warn!("⚠️ Failed to report load-timeout: {}", e),
                            }
                            // Open logs folder.
                            if let Some(dir) = &dir_clone {
                                #[cfg(target_os = "windows")]
                                {
                                    let _ = std::process::Command::new("explorer").arg(dir).spawn();
                                }
                            }
                        });
                    }
                });
        });
    }

    // 🔗 Initialize WebhookServiceState (after DbPool is managed)
    let webhook_storage = Arc::new(vortideck_core::services::webhook::WebhookStorage::new(
        db_pool_for_webhooks,
    ));
    let webhook_service_for_commands =
        vortideck_core::services::webhook::WebhookService::new(webhook_storage);
    let webhook_service_state =
        vortideck_core::services::webhook::WebhookServiceState::new(webhook_service_for_commands);

    // 🚀 EAGER INITIALIZATION: Initialize flow executor at startup to prevent race conditions
    {
        let flow_registry_arc = Arc::new(flow_registry_state.clone());
        let chat_state_arc = Arc::new(chat_state.clone());
        let platform_context_arc = platform_context.clone();

        match vortideck_core::services::flow::executor::FlowExecutor::new_universal(
            flow_registry_arc,
            chat_state_arc,
            platform_context_arc,
            db_pool_for_flows,
        ) {
            Ok(executor) => {
                if let Ok(mut executor_guard) = flow_executor_state.executor.try_lock() {
                    *executor_guard = Some(executor);
                    println!("✅ Flow executor initialized at startup");
                    log::info!("✅ Flow executor initialized at startup");
                } else {
                    println!("⚠️ Could not acquire flow executor lock during startup");
                    log::warn!("⚠️ Could not acquire flow executor lock during startup");
                }
            }
            Err(e) => {
                println!("❌ Failed to initialize flow executor at startup: {:?}", e);
                log::error!("❌ Failed to initialize flow executor at startup: {:?}", e);
            }
        }
    }

    // ✅ NEW ARCHITECTURE: RecordingState managed internally by VortiDeckCore
    // obs_plugin_state removed - using StateManager directly
    app.manage(setup_state);
    let durable_stream_ingress_state =
        vortideck_core::services::flow::event_router::DurableStreamEventIngressState::default();
    let flow_runtime = vortideck_core::services::flow::FlowRuntime::from_initialized(
        platform_context.clone(),
        flow_registry_state.clone(),
        flow_executor_state.clone(),
        chat_state.clone(),
        durable_stream_ingress_state.clone(),
    );
    app.manage(flow_registry_state);
    app.manage(flow_executor_state);
    app.manage(durable_stream_ingress_state.clone());
    app.manage(flow_runtime.clone());
    // Reconcile incomplete persisted runs before accepting new source events.
    // The executor marks them RecoveryRequired; it never replays side effects
    // automatically after a restart.
    let flow_runtime_for_start = flow_runtime.clone();
    tauri::async_runtime::spawn(async move {
        #[cfg(feature = "acceptance-test-hooks")]
        if std::env::var("VORTIDECK_ACCEPTANCE_SCENARIO").as_deref() == Ok("seed-crash") {
            let executor = match flow_runtime_for_start.executor().await {
                Ok(executor) => executor,
                Err(error) => {
                    write_packaged_acceptance_marker(serde_json::json!({
                        "phase": "seeded",
                        "success": false,
                        "error": error,
                    }));
                    return;
                }
            };
            let result = executor.seed_packaged_recovery_acceptance_run().await;
            write_packaged_acceptance_marker(serde_json::json!({
                "phase": "seeded",
                "success": result.is_ok(),
                "error": result.err(),
            }));
            return;
        }

        let report = match flow_runtime_for_start.start().await {
            Ok(report) => report,
            Err(error) => {
                log::error!("Flow runtime failed to start: {}", error);
                #[cfg(feature = "acceptance-test-hooks")]
                write_packaged_acceptance_marker(serde_json::json!({
                    "phase": "runtime-start",
                    "success": false,
                    "error": error,
                }));
                return;
            }
        };
        if report.recovered_runs == 0 {
            log::info!("No interrupted workflow runs to recover");
        } else {
            log::warn!(
                "Marked {} interrupted workflow run(s) as recovery-required",
                report.recovered_runs
            );
        }
        log::info!(
            "Rebuilt workflow trigger registry with {} route(s)",
            report.active_trigger_routes
        );

        #[cfg(feature = "acceptance-test-hooks")]
        if std::env::var("VORTIDECK_ACCEPTANCE_SCENARIO").as_deref() == Ok("recover") {
            let executor = match flow_runtime_for_start.executor().await {
                Ok(executor) => executor,
                Err(error) => {
                    write_packaged_acceptance_marker(serde_json::json!({
                        "phase": "recovery",
                        "success": false,
                        "error": error,
                    }));
                    return;
                }
            };
            let recovery_runs = executor.list_recovery_required_runs().await;
            let wakes = executor
                .list_open_workflow_wakes()
                .await
                .unwrap_or_default();
            write_packaged_acceptance_marker(serde_json::json!({
                "phase": "recovery",
                "success": true,
                "recoveredThisStartup": report.recovered_runs,
                "recoveryRuns": recovery_runs.len(),
                "openApprovalWakes": wakes
                    .iter()
                    .filter(|wake| wake.kind == vortideck_core::services::flow::executor::WorkflowWakeKind::Approval)
                    .count(),
                "runIds": recovery_runs.iter().map(|run| run.run_id.clone()).collect::<Vec<_>>(),
                "wakeIds": wakes.iter().map(|wake| wake.id.clone()).collect::<Vec<_>>(),
            }));
        }
    });
    app.manage(chat_state);
    app.manage(webhook_service_state);
    app.manage(commands::plugin_commands::PluginFrameSessionRegistry::default());
    app.manage(vortideck_core);
    let platform_context_for_relay = platform_context.clone(); // For relay command handler
    app.manage(platform_context.clone());

    // 🔐 E2EE: Initialize pending pairings map for QR code pairing
    let pending_pairings: PendingPairingsMap = Arc::new(RwLock::new(HashMap::new()));
    app.manage(pending_pairings);

    // 📡 Backend Relay: Initialize relay client state (with internal retry loop)
    let relay_client_state: commands::relay_commands::RelayClientState =
        Arc::new(RwLock::new(None));
    app.manage(relay_client_state);

    // 🤖 Initialize AI Assistant context store
    commands::assistant_commands::init_assistant_store(&app_handle);

    // 🖼️ Initialize Icons Service State
    let icons_service_state = adapters::icons_commands::IconServiceState::new(&app_handle);
    app.manage(icons_service_state);

    // 🎥 Initialize OBS Service State
    // let obs_service_state = adapters::obs_commands::ObsServiceState::new();
    // app.manage(obs_service_state);

    // 📁 Initialize Workspace (Documents/VortiDeck/workspace/)
    let documents_dir = app_handle
        .path()
        .document_dir()
        .expect("Failed to get documents directory");
    let workspace_config = vortideck_core::services::WorkspaceConfig::new(documents_dir)
        .expect("Failed to initialize workspace");
    let workspace_state = commands::workspace_commands::WorkspaceState::new(workspace_config);
    app.manage(workspace_state);

    // 🚀 DIRECT ARCHITECTURE: Initialize state manager with simple event bridge
    let state_manager = adapters::state_manager::create_state_manager(app_handle.clone());

    // 🔐 PHASE 2: Restore E2EE shared secrets from database on startup
    {
        let db = Database::new(&db_pool_for_e2ee);
        let secrets = db.load_all_shared_secrets();
        let secret_count = secrets.len();

        if secret_count > 0 {
            let state_manager_clone = state_manager.clone();
            tauri::async_runtime::spawn(async move {
                for (client_id, secret) in secrets {
                    state_manager_clone
                        .restore_shared_secret_from_db(client_id, secret)
                        .await;
                }
                println!(
                    "🔄 Restored {} E2EE shared secrets from database",
                    secret_count
                );
                println!("✅ Paired devices will reconnect without re-pairing!");
            });
        }
    }

    // ⚙️ Load execution settings from database synchronously
    // This is loaded BEFORE ExecutionEngine initialization so it can use the limit
    let app_config_from_db = {
        use diesel::RunQueryDsl;
        use vortideck_core::db::schema::app_config;

        let mut conn = db_pool_for_e2ee
            .get()
            .expect("Failed to get DB connection for execution settings");
        app_config::table
            .first::<vortideck_core::db::models::AppConfig>(&mut conn)
            .expect("Failed to load app_config from database")
    };

    // Load execution settings into StateManager memory cache (async, don't block)
    {
        use vortideck_core::core::state_manager::ExecutionSettings;

        let state_manager_clone = state_manager.clone();
        let settings = ExecutionSettings {
            max_concurrent_actions: app_config_from_db.max_concurrent_actions as u32,
            max_action_timeout_ms: app_config_from_db.max_action_timeout_ms as u64,
        };

        tauri::async_runtime::spawn(async move {
            state_manager_clone
                .update_execution_settings(settings)
                .await;
        });
    }

    // 🎬 Initialize macro engine with Tauri focus integration
    let macro_engine =
        match adapters::macro_commands::init_macro_recording(&app_handle, state_manager.clone()) {
            Ok(engine) => engine,
            Err(e) => {
                log::error!("Failed to initialize macro recording: {}", e);
                return Err(format!("Failed to initialize macro recording: {}", e).into());
            }
        };

    // 🔊 Register volume change callback directly with Windows system
    tokio::spawn({
        let app_handle = app_handle.clone();
        let state_manager = state_manager.clone();
        async move {
            if let Err(e) =
                adapters::volume_callback::register_volume_callback(app_handle, state_manager).await
            {
                log::warn!("Failed to register volume callback: {}", e);
            }
        }
    });

    // Initialize Enigo for input simulation
    let enigo = Arc::new(tokio::sync::Mutex::new(
        Enigo::new(&Settings::default()).expect("Failed to initialize Enigo"),
    ));
    // 🔌 Initialize Universal Integration Manager
    use vortideck_core::integrations::{IntegrationStatusMonitor, UniversalIntegrationManager};
    let integration_event_emitter =
        adapters::tauri_event_emitter::create_tauri_event_emitter(app_handle.clone());
    let universal_integration_manager = Arc::new(
        UniversalIntegrationManager::new(state_manager.clone())
            .with_event_emitter(integration_event_emitter),
    );
    let action_runtime_readiness =
        Arc::new(vortideck_core::services::execution::ActionRuntimeReadiness::new());
    app.manage(action_runtime_readiness.clone());

    // 🎮 Register built-in integrations
    use vortideck_core::integrations::builtin::discord::DiscordIntegration;
    use vortideck_core::integrations::builtin::obs::ObsIntegration;
    use vortideck_core::integrations::builtin::overlay::OverlayIntegration;
    use vortideck_core::integrations::builtin::streaming::StreamingIntegration;

    // The relay state is created before provider registration so streaming
    // capabilities are available to discovery, authoring, and preflight from
    // the moment the universal manager becomes reachable.
    let streaming_state = StreamingState::with_db(Arc::new(db_pool_for_streaming.clone()))
        .with_event_bus(platform_context.event_bus.clone())
        .with_workflow_ingress(durable_stream_ingress_state.clone());
    let core_database = Arc::new(vortideck_core::db::Database::new(
        &app.state::<vortideck_core::db::db::DbPool>()
            .inner()
            .clone(),
    ));
    let obs_integration = Arc::new(ObsIntegration::new(state_manager.clone()));
    let discord_integration = Arc::new(DiscordIntegration::new(state_manager.clone()));
    let overlay_integration = Arc::new(
        OverlayIntegration::new(platform_context.event_bus.clone())
            .with_database(core_database.clone()),
    );
    let streaming_integration =
        Arc::new(StreamingIntegration::new(Arc::new(streaming_state.clone())));

    let enigo_for_relay = enigo.clone(); // For relay command handler
    let enigo_for_executor = enigo.clone(); // For universal execution engine
    app.manage(enigo);
    app.manage(state_manager.clone());
    app.manage(macro_engine);
    app.manage(discord_integration.clone());
    app.manage(overlay_integration.clone());
    app.manage(universal_integration_manager.clone());

    // 🚀 UNIVERSAL EXECUTION ENGINE: Initialize as singleton for all action execution
    let flow_executor_for_executor = app.state::<FlowExecutorState>().executor.clone();

    // Use max_concurrent_actions from app_config loaded above
    let max_concurrent = app_config_from_db.max_concurrent_actions as usize;
    println!(
        "✅ Using max_concurrent_actions from database: {}",
        max_concurrent
    );

    let universal_execution_engine = Arc::new(
        vortideck_core::services::execution::UniversalExecutionEngine::with_concurrency_limit(
            state_manager.clone(),
            max_concurrent,
        )
        .with_database(core_database.clone())
        .with_flow_executor(flow_executor_for_executor)
        .with_enigo(enigo_for_executor)
        .with_integration_manager(universal_integration_manager.clone())
        .with_permission_checker(
            app.state::<VortiDeckCore>()
                .plugin_manager()
                .get_permission_checker(),
        ),
    );
    app.manage(universal_execution_engine.clone());
    let action_invocation_service = Arc::new(
        vortideck_core::services::execution::ActionInvocationService::new(
            universal_execution_engine.clone(),
            core_database,
            action_runtime_readiness.clone(),
        ),
    );
    app.manage(action_invocation_service.clone());

    // External-effect MCP tools are installed only after their durable
    // invocation authority exists. This makes them available to assistant and
    // in-flow AI sessions without exposing raw FlowNode execution as a bypass.
    {
        let chat_state = app
            .state::<vortideck_core::services::mcp::commands::ChatState>()
            .inner()
            .clone();
        let invocation_service = action_invocation_service.clone();
        tauri::async_runtime::spawn(async move {
            let mut tools = chat_state.mcp_tools.lock().await;
            vortideck_core::services::mcp::tools::register_invocation_tools(
                &mut tools,
                invocation_service,
            );
            log::info!("Durable invocation-backed AI tools registered");
        });
    }

    // Wire the durable action boundary into flow execution.
    {
        let flow_executor_state = app.state::<FlowExecutorState>().executor.clone();
        let invocation_service_clone = action_invocation_service.clone();
        tauri::async_runtime::spawn(async move {
            for attempt in 1..=10 {
                let mut executor_guard = flow_executor_state.lock().await;
                if let Some(executor) = executor_guard.as_mut() {
                    executor.set_action_invocation_service(invocation_service_clone.clone());
                    println!("✅ Flow executor linked to ActionInvocationService");
                    log::info!("✅ Flow executor linked to ActionInvocationService");
                    return;
                }
                drop(executor_guard);
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                if attempt == 10 {
                    println!(
                        "⚠️ Flow executor not available when linking UniversalExecutionEngine"
                    );
                    log::warn!(
                        "⚠️ Flow executor not available when linking UniversalExecutionEngine"
                    );
                }
            }
        });
    }

    if max_concurrent > 0 {
        println!(
            "✅ Initialized Universal Execution Engine with concurrency limit: {}",
            max_concurrent
        );
        log::info!(
            "✅ Initialized Universal Execution Engine with concurrency limit: {}",
            max_concurrent
        );
    } else {
        println!("✅ Initialized Universal Execution Engine with UNLIMITED concurrency");
        log::info!("✅ Initialized Universal Execution Engine with UNLIMITED concurrency");
    }

    // 🔄 Listen for config updates and update execution engine dynamically
    let execution_engine_for_update = app
        .state::<Arc<vortideck_core::services::execution::UniversalExecutionEngine>>()
        .inner()
        .clone();
    let app_handle_for_listener = app_handle.clone();
    app_handle_for_listener.listen("app_config_updated", move |event| {
        let payload = event.payload();
        if let Ok(config_data) = serde_json::from_str::<serde_json::Value>(payload) {
            if let Some(data) = config_data.get("data") {
                if let Some(max_concurrent) =
                    data.get("max_concurrent_actions").and_then(|v| v.as_i64())
                {
                    let engine = execution_engine_for_update.clone();
                    tauri::async_runtime::spawn(async move {
                        engine
                            .update_concurrency_limit(max_concurrent as usize)
                            .await;
                    });
                }
            }
        }
    });
    println!("🔄 Execution engine listening for config updates (hot-reload enabled)");
    log::info!("🔄 Execution engine listening for config updates (hot-reload enabled)");

    // Restore the host-owned active overlay before OBS receives its capability.
    // The editor's localStorage is a project library, not runtime persistence:
    // OBS must render the last accepted design without requiring that route to
    // be opened after every desktop restart.
    let mut overlay_initial_state = OverlayState::new();
    match overlay_initial_state.load_persisted(&app_handle) {
        Ok(true) => log::info!(
            "Restored active overlay runtime with {} element(s)",
            overlay_initial_state.current_design.elements.len()
        ),
        Ok(false) => overlay_initial_state.load_legacy_canvas_size(&app_handle),
        Err(error) => {
            log::warn!("Unable to restore active overlay runtime: {error}");
            overlay_initial_state.load_legacy_canvas_size(&app_handle);
        }
    }
    let publication_receipt = overlay_initial_state.valid_publication_receipt();
    let overlay_state = Arc::new(tokio::sync::RwLock::new(overlay_initial_state));
    app.manage(overlay_state.clone());
    let composition_path = app_handle.path().app_data_dir()?.join("overlay_compositions.json");
    match vortideck_core::services::overlay_authoring::OverlayAuthoringService::open(composition_path) {
        Ok(service) => {
            if let Some(saved) = publication_receipt {
                // Setup runs inside the #[tokio::main] runtime, where starting another
                // runtime (tauri::async_runtime::block_on) panics.
                let reconciled = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(service.reconcile_publication(saved.receipt))
                });
                match reconciled {
                    Ok(true) => log::info!("Reconciled saved overlay publication receipt"),
                    Ok(false) => {},
                    Err(error) => log::error!("Saved overlay is intact; publication metadata recovery failed: {error}"),
                }
            }
            let service = Arc::new(service);
            let mut changes = service.subscribe_changes();
            let host = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    match changes.recv().await {
                        Ok(document) => { let _ = host.emit("vortideck:composition-changed", document); }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => { let _ = host.emit("vortideck:composition-reload-required", ()); }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    }
                }
            });
            let chat = app.state::<vortideck_core::services::mcp::commands::ChatState>().inner().clone();
            let authoring = service.clone();
            tauri::async_runtime::spawn(async move {
                let mut tools = chat.mcp_tools.lock().await;
                vortideck_core::services::mcp::overlay_authoring::register_overlay_authoring_tools(&mut tools, authoring);
            });
            app.manage(service);
        }
        Err(error) => log::error!("Composition authoring unavailable; original library retained and live output preserved: {error}"),
    }

    {
        let chat_state = app
            .state::<vortideck_core::services::mcp::commands::ChatState>()
            .inner()
            .clone();
        let api = Arc::new(overlay::settings::DesktopOverlayWidgetSettings(
            app_handle.clone(),
        ));
        tauri::async_runtime::spawn(async move {
            let mut tools = chat_state.mcp_tools.lock().await;
            vortideck_core::services::mcp::overlay_settings::register_overlay_settings_tools(
                &mut tools, api,
            );
        });
    }

    // 📡 Initialize client registry for type-specific WebSocket management
    let client_registry = Arc::new(client_registry::ClientRegistry::new());
    app.manage(client_registry.clone());

    // 🌉 Bridge EventBus events directly to overlay WebSocket clients
    let event_bus_for_overlay = platform_context.event_bus.clone();
    let client_registry_for_overlay = client_registry.clone();
    let app_handle_for_overlay = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        let mut rx = event_bus_for_overlay.subscribe();
        log::info!("📡 Spawning EventBus → Overlay WebSocket broadcaster task");
        loop {
            let event = match rx.recv().await {
                Ok(event) => event,
                Err(tokio::sync::broadcast::error::RecvError::Lagged(count)) => {
                    log::warn!(
                        "📡 Overlay EventBus bridge lagged by {} events; continuing",
                        count
                    );
                    match commands::overlay_authoring::observed_values_message(&app_handle_for_overlay).await {
                        Ok(snapshot) => { client_registry_for_overlay.send_to_overlays(&snapshot.to_string()).await; },
                        Err(error) => log::warn!("Could not reconcile observed stream values after lag: {error}"),
                    }
                    continue;
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            };
            use vortideck_core::events::{
                EventPayload, FlowEventPayload, SessionEventPayload, StreamEventPayload,
            };
            match event {
                EventPayload::Stream(payload) => {
                    if matches!(payload.event_type(), "follow" | "subscription" | "bits") {
                        match commands::overlay_authoring::observed_values_message(&app_handle_for_overlay).await {
                            Ok(snapshot) => { client_registry_for_overlay.send_to_overlays(&snapshot.to_string()).await; },
                            Err(error) => log::warn!("Could not deliver observed stream values: {error}"),
                        }
                    }
                    if let Ok(msg) = serde_json::to_string(&serde_json::json!({
                        "category": "stream",
                        "payload": payload
                    })) {
                        client_registry_for_overlay.send_to_overlays(&msg).await;
                    }
                }
                EventPayload::Session(session_payload) => {
                    match session_payload {
                        SessionEventPayload::SessionActivated {
                            session_id,
                            account_ids,
                        } => {
                            log::info!(
                                "🔄 [OVERLAY BROADCASTER] Session activated: {}. Clearing chat and sending history for accounts: {:?}",
                                session_id,
                                account_ids
                            );

                            // 1. Send clearChat event to all overlay clients
                            let clear_msg = serde_json::json!({
                                "category": "stream",
                                "payload": {
                                    "type": "clearChat"
                                }
                            })
                            .to_string();
                            client_registry_for_overlay
                                .send_to_overlays(&clear_msg)
                                .await;

                            // 2. Fetch recent 20 messages for these accounts from message store and send them
                            if let Some(streaming_state) =
                                app_handle_for_overlay.try_state::<StreamingState>()
                            {
                                let message_store = streaming_state.message_store.clone();
                                let client_registry_clone = client_registry_for_overlay.clone();
                                tauri::async_runtime::spawn(async move {
                                    if let Ok(messages) = message_store
                                        .get_messages_for_accounts(account_ids, Some(20))
                                        .await
                                    {
                                        for msg in messages {
                                            // Build the payload via the typed enum so fields — including
                                            // platformColor — can't be silently dropped, and the shape stays
                                            // identical to live events and the WS-connect replay path.
                                            let payload = EventPayload::Stream(StreamEventPayload::ChatMessage {
                                                metadata: vortideck_core::events::StreamEventMetadata {
                                                    event_id: Some(msg.id.clone()),
                                                    occurred_at: chrono::DateTime::parse_from_rfc3339(&msg.timestamp)
                                                        .ok()
                                                        .map(|value| value.with_timezone(&chrono::Utc)),
                                                    stream_id: None,
                                                    account_id: msg.account_id.unwrap_or(0),
                                                    platform: msg.platform.to_string(),
                                                },
                                                user_name: msg.user_name,
                                                user_display_name: msg.user_display_name,
                                                message: msg.message,
                                                is_moderator: msg.is_moderator,
                                                is_subscriber: msg.is_subscriber,
                                                platform_color: if msg.platform_color.is_empty() { None } else { Some(msg.platform_color.clone()) },
                                            });
                                            if let Ok(msg_str) = serde_json::to_string(&payload) {
                                                client_registry_clone
                                                    .send_to_overlays(&msg_str)
                                                    .await;
                                            }
                                        }
                                    }
                                });
                            }
                        }
                        SessionEventPayload::SessionDeactivated { session_id } => {
                            log::info!(
                                "🔄 [OVERLAY BROADCASTER] Session deactivated: {}. Clearing chat on overlays.",
                                session_id
                            );
                            let clear_msg = serde_json::json!({
                                "category": "stream",
                                "payload": {
                                    "type": "clearChat"
                                }
                            })
                            .to_string();
                            client_registry_for_overlay
                                .send_to_overlays(&clear_msg)
                                .await;
                        }
                        _ => {}
                    }
                }
                EventPayload::Generic {
                    source,
                    event_type,
                    payload,
                } => {
                    if let Ok(msg) = serde_json::to_string(&serde_json::json!({
                        "category": "generic",
                        "payload": {
                            "source": source,
                            "eventType": event_type,
                            "payload": payload
                        }
                    })) {
                        client_registry_for_overlay.send_to_overlays(&msg).await;
                    }
                }
                EventPayload::Plugin(_) => {
                    // Plugin events are persisted by EventRouter. They are not
                    // broadcast to every overlay without a scoped subscription.
                }
                EventPayload::Overlay(payload) => {
                    if payload
                        .expires_at
                        .is_some_and(|expires_at| expires_at <= chrono::Utc::now())
                    {
                        log::debug!("Skipping expired overlay command {}", payload.command_id);
                        continue;
                    }
                    match app_lib::overlay::commands::delivery_message(&app_handle_for_overlay, &payload, false).await {
                        Ok(message) => { client_registry_for_overlay.send_to_overlays(&message.to_string()).await; },
                        Err(error) => app_lib::overlay::commands::reject_delivery(&app_handle_for_overlay, &payload, error).await,
                    }
                }
                EventPayload::Flow(FlowEventPayload::FlowCompleted {
                    flow_id,
                    run_id,
                    result,
                }) => {
                    let msg = serde_json::json!({
                        "category": "flow",
                        "payload": {
                            "type": "flowCompleted",
                            "flowId": flow_id,
                            "runId": run_id,
                            "result": result
                        }
                    })
                    .to_string();
                    client_registry_for_overlay.send_to_overlays(&msg).await;
                }
                _ => {}
            }
        }
    });

    // 🎬 Initialize streaming service state (persistent WebSocket + message store + session manager with DB)
    app.manage(streaming_state.clone());
    println!(
        "✅ Initialized Streaming Service with database persistence (WebSocket, message store, session manager)"
    );
    log::info!(
        "✅ Initialized Streaming Service with database persistence (WebSocket, message store, session manager)"
    );

    // Provider initialization is asynchronous and independently scheduled. The
    // readiness barrier prevents all action/discovery surfaces from observing
    // the manager until the complete built-in catalog is published.
    {
        let app_handle = app_handle.clone();
        let readiness = action_runtime_readiness.clone();
        let manager = universal_integration_manager.clone();
        let streaming_state = streaming_state.clone();
        let discord = discord_integration.clone();
        let overlay = overlay_integration.clone();
        let obs = obs_integration.clone();
        let streaming = streaming_integration.clone();
        let state_manager = state_manager.clone();
        tauri::async_runtime::spawn(async move {
            let initializing = readiness.begin_phase("initializing_providers");
            let _ = app_handle.emit("action-runtime-readiness", &initializing);

            let (session_result, (), overlay_result) = tokio::join!(
                streaming_state.load_sessions(),
                discord.initialize(),
                overlay.start(),
            );
            let mut warnings = Vec::new();
            match session_result {
                Ok(count) => log::info!(
                    "Loaded {} streaming sessions before capability publication",
                    count
                ),
                Err(error) => {
                    let warning = format!("Streaming started without cached sessions: {}", error);
                    log::warn!("{}", warning);
                    warnings.push(warning);
                }
            }
            if let Err(error) = overlay_result {
                let warning = format!("Overlay media recovery is unavailable: {}", error);
                log::warn!("{}", warning);
                warnings.push(warning);
            }

            let publishing = readiness.begin_phase("publishing_catalog");
            let _ = app_handle.emit("action-runtime-readiness", &publishing);
            let providers: Vec<Arc<dyn vortideck_core::integrations::AppIntegration>> =
                vec![obs, discord, overlay, streaming];
            let snapshot = match manager.register_integrations(providers).await {
                Ok(()) => {
                    Arc::new(IntegrationStatusMonitor::new(
                        manager,
                        std::time::Duration::from_secs(2),
                    ))
                    .spawn_monitor();
                    readiness.mark_ready(warnings)
                }
                Err(error) => {
                    log::error!("Action runtime bootstrap failed: {}", error);
                    readiness.mark_failed(error.to_string())
                }
            };
            let _ = app_handle.emit("action-runtime-readiness", &snapshot);
            crate::broadcast_helper::broadcast_to_all_clients(
                &app_handle,
                &state_manager,
                json!({
                    "action": "action_runtime_readiness",
                    "protocolVersion": 1,
                    "readiness": snapshot,
                }),
            )
            .await;
        });
    }

    println!("✅ Using proper Tauri platform adapters with StateManager architecture");
    log::info!("✅ Using proper Tauri platform adapters with StateManager architecture");

    // 🔊 Volume change callback will be initialized after setup completes (in my_app function)

    // 🚀 Add small delay to ensure all state is properly managed before any async tasks start
    let app_handle_delay = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        my_app(app_handle_delay).await;
    });

    // 🎭 GUEST MODE: No automatic JWT loading - let user authenticate manually
    // 🧪 Development JWT tokens can be loaded via debug.vue for testing specific scenarios
    // This ensures we start in true guest mode with 0 plugin limit for proper testing
    println!(
        "🎭 Starting in Guest Mode - No JWT token loaded (use debug.vue to test different user types)"
    );
    log::info!(
        "🎭 Starting in Guest Mode - No JWT token loaded (use debug.vue to test different user types)"
    );

    // Register MCP chat commands - using new architecture
    // MCP commands are now registered via VortiDeckCore and available through the invoke_handler
    println!("✅ MCP commands registered via new architecture");
    log::info!("✅ MCP commands registered via new architecture");

    let quit_i = MenuItem::with_id(app, "quit", "Quit Server", true, None::<&str>)?;
    let open_i = MenuItem::with_id(app, "open", "Open", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open_i, &quit_i])?;

    let tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .icon_as_template(true)
        .menu(&menu)
        .menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => {
                let _ = ensure_window(&app, "main", "/".to_string());
            }
            "quit" => {
                let app = app.app_handle();
                app.exit(0)
            }
            _ => (),
        })
        .on_tray_icon_event(|tray, event| match event {
            TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } => {
                // in this example, let's show and focus the main window when the tray is clicked
                let app = tray.app_handle();
                println!("Left click pressed and released");
                let _ = ensure_window(&app, "main", "/".to_string());
            }
            // Match the right mouse button click
            TrayIconEvent::Click {
                button: MouseButton::Right,
                button_state: MouseButtonState::Up,
                ..
            } => {
                println!("Right click pressed and released");
                // Perform any actions related to the right click
                //TODO: Create custom menu for tray
            }
            _ => {}
        })
        .build(app)?;

    // // Get a value from the store.
    // let value = store.get("some-key").expect("Failed to get value from store");
    // println!("{}", value); // {"value":5}

    // 📡 Backend Relay: Auto-enable relay connection
    {
        let app_handle_relay = app_handle.clone();

        // 🔐 Load all shared secrets from database on startup
        {
            let db_pool = app_handle.state::<DbPool>().inner().clone();
            let state_manager = app_handle.state::<Arc<StateManager>>().inner().clone();

            tauri::async_runtime::spawn(async move {
                let db = Database::new(&db_pool);
                let secrets = db.load_all_shared_secrets();
                let count = secrets.len();

                for (client_id, secret) in secrets {
                    state_manager
                        .store_shared_secret(client_id.clone(), secret)
                        .await;
                    println!("🔐 Loaded shared secret from database for: {}", client_id);
                }

                if count > 0 {
                    println!("✅ Loaded {} shared secrets from database", count);
                } else {
                    println!("ℹ️ No saved shared secrets found in database");
                }
            });
        }

        // Check for existing token and emit auth_token_saved event if found
        // This uses the same code path as login to prevent duplicate spawns
        let app_handle_startup = app_handle.clone();
        tauri::async_runtime::spawn(async move {
            // Wait a bit for shared secrets and initialization to complete
            tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;

            // Check if user has a valid access token from previous session
            if let Ok(_token) =
                vortideck_core::services::auth::token_storage::TokenStorage::get_access_token()
            {
                println!("🔐 Found existing access token - triggering connection setup");
                // Emit the same event as login to reuse the connection logic
                if let Err(e) = app_handle_startup.emit("auth_token_saved", ()) {
                    eprintln!("❌ Failed to emit auth_token_saved event: {}", e);
                }
            } else {
                println!("ℹ️ No access token found - waiting for user login");
            }
        });

        // Listen for auth_token_saved event to start/restart connections
        // This is the SINGLE source for connection spawning (startup + login)
        // When websocket loops exit due to missing token, this event will restart them on login
        let app_handle_listener = app_handle.clone();
        let app_handle_streaming_listener = app_handle.clone();
        app_handle.listen("auth_token_saved", move |_event| {
            println!("🔐 auth_token_saved event - starting connection loops...");
            let app_handle_relay = app_handle_listener.clone();
            let app_handle_streaming = app_handle_streaming_listener.clone();
            tauri::async_runtime::spawn(async move {
                let _ = crate::relay_connection::attempt_relay_connection(app_handle_relay).await;
            });
            tauri::async_runtime::spawn(async move {
                let _ =
                    crate::streaming_connection::attempt_streaming_connection(app_handle_streaming)
                        .await;
            });
        });

        // EventBus disconnection listener removed - relay connection now has persistent loop
        // that handles all reconnections internally (prevents duplicate retry loops)

        // ✅ Subscribe to EventBus for relay commands (platform-agnostic architecture)
        {
            let app_handle_relay = app_handle.clone();
            let state_manager_relay = state_manager.clone();
            let db_pool_relay = db_pool_for_relay.clone();
            let obs_integration_relay = Arc::new(
                vortideck_core::integrations::builtin::obs::ObsIntegration::new(
                    state_manager.clone(),
                ),
            );
            let enigo_relay = enigo_for_relay.clone();
            let event_bus_relay = platform_context_for_relay.event_bus.clone();

            // Subscribe to EventBus
            let mut rx = event_bus_relay.subscribe();

            tauri::async_runtime::spawn(async move {
                println!("📡 EventBus relay command handler started");

                loop {
                    let event = match rx.recv().await {
                        Ok(event) => event,
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(count)) => {
                            log::warn!(
                                "📡 Relay EventBus handler lagged by {} events; continuing",
                                count
                            );
                            continue;
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    };
                    // Only handle Relay events
                    if let vortideck_core::events::EventPayload::Relay(relay_event) = event {
                        match relay_event {
                            vortideck_core::events::types::RelayEventPayload::CommandReceived {
                                client_id,
                                connection_session_id,
                                payload,
                            } => {
                                println!("📥 Relay command received from: {}", client_id);

                                // Handle relay actions from mobile
                                if let Some(action) = payload.get("action").and_then(|a| a.as_str())
                                {
                                    println!("🎯 Relay command action: {}", action);

                                    match action {
                                        "command" => {
                                            println!(
                                                "🔄 Processing relay command via shared handler"
                                            );

                                            // Create a dummy broadcast channel for relay commands
                                            // (relay responses go through broadcast_to_all_clients helper)
                                            let (tx, _rx) = tokio::sync::broadcast::channel::<
                                                tokio_tungstenite::tungstenite::Message,
                                            >(
                                                100
                                            );

                                            let db = Database::new(&db_pool_relay);

                                            // Use shared command handler
                                            let ctx = command_handler::CommandExecutionContext {
                                                json: payload.clone(),
                                                obs_integration: obs_integration_relay.clone(),
                                                db: db.clone(),
                                                tx: tx.clone(),
                                                enigo: enigo_relay.clone(),
                                                app_handle: app_handle_relay.clone(),
                                            };

                                            command_handler::handle_command_action(ctx).await;
                                        }
                                        "deck_widget_intent" => {
                                            let (tx, _rx) = tokio::sync::broadcast::channel::<
                                                tokio_tungstenite::tungstenite::Message,
                                            >(
                                                100
                                            );
                                            let ctx = command_handler::CommandExecutionContext {
                                                json: payload.clone(),
                                                obs_integration: obs_integration_relay.clone(),
                                                db: Database::new(&db_pool_relay),
                                                tx,
                                                enigo: enigo_relay.clone(),
                                                app_handle: app_handle_relay.clone(),
                                            };
                                            command_handler::handle_widget_intent_action(
                                                ctx,
                                                command_handler::WidgetIntentSource {
                                                    client_id: client_id.clone(),
                                                    connection_session_id: connection_session_id.clone(),
                                                },
                                            )
                                            .await;
                                        }
                                        "deck_widget_cache_inventory" => {
                                            let (tx, _rx) = tokio::sync::broadcast::channel::<
                                                tokio_tungstenite::tungstenite::Message,
                                            >(
                                                16
                                            );
                                            let ctx = command_handler::CommandExecutionContext {
                                                json: payload.clone(),
                                                obs_integration: obs_integration_relay.clone(),
                                                db: Database::new(&db_pool_relay),
                                                tx,
                                                enigo: enigo_relay.clone(),
                                                app_handle: app_handle_relay.clone(),
                                            };
                                            command_handler::handle_widget_cache_inventory(ctx)
                                                .await;
                                        }
                                        "workflow_approval_decision" => {
                                            let response = match command_handler::handle_workflow_approval_decision(
                                                &app_handle_relay,
                                                &payload,
                                                &client_id,
                                                "mobile_relay",
                                            )
                                            .await
                                            {
                                                Ok(outcome) => json!({
                                                    "action": "workflow_approval_decision_ack",
                                                    "wakeId": payload.get("wakeId"),
                                                    "outcome": outcome,
                                                }),
                                                Err(error) => json!({
                                                    "action": "workflow_approval_decision_ack",
                                                    "wakeId": payload.get("wakeId"),
                                                    "error": error,
                                                }),
                                            };
                                            if let Some(relay_state) = app_handle_relay
                                                .try_state::<commands::relay_commands::RelayClientState>()
                                            {
                                                if let Some(relay_client) = relay_state.read().await.as_ref() {
                                                    let _ = relay_client
                                                        .send_to_relay_mobile(
                                                            &client_id,
                                                            &response,
                                                            &state_manager_relay,
                                                        )
                                                        .await;
                                                }
                                            }
                                        }
                                        "list_workflow_approvals" => {
                                            if let Ok(snapshot) =
                                                command_handler::workflow_approvals_snapshot(
                                                    &app_handle_relay,
                                                    &client_id,
                                                    "mobile_relay",
                                                )
                                                .await
                                            {
                                                if let Some(relay_state) = app_handle_relay
                                                    .try_state::<commands::relay_commands::RelayClientState>()
                                                {
                                                    if let Some(relay_client) = relay_state.read().await.as_ref() {
                                                        let _ = relay_client
                                                            .send_to_relay_mobile(
                                                                &client_id,
                                                                &snapshot,
                                                                &state_manager_relay,
                                                            )
                                                            .await;
                                                    }
                                                }
                                            }
                                        }
                                        "get_current_deck" => {
                                            println!("🔄 Processing get_current_deck via relay");
                                            let db = Database::new(&db_pool_relay);
                                            // Get icons base path from IconServiceState (proper way)
                                            let icons_base_path = app_handle_relay.try_state::<adapters::icons_commands::IconServiceState>()
                                         .map(|icon_state| icon_state.service.get_icons_base_dir());

                                            // Fetch active deck with icons
                                            match db
                                                .get_active_deck_with_buttons(
                                                    true,
                                                    icons_base_path.as_deref(),
                                                )
                                                .await
                                            {
                                                Some(deck_with_buttons) => {
                                                    let deck_id = deck_with_buttons.id;
                                                    let response = json!({
                                                        "action": "active_deck",
                                                        "deck": deck_with_buttons
                                                    });
                                                    broadcast_helper::broadcast_to_all_clients(
                                                        &app_handle_relay,
                                                        &state_manager_relay,
                                                        response,
                                                    )
                                                    .await;
                                                    if let Some(snapshot) =
                                                        build_deck_widget_snapshot(&db, deck_id)
                                                            .await
                                                    {
                                                        broadcast_helper::broadcast_to_all_clients(
                                                            &app_handle_relay,
                                                            &state_manager_relay,
                                                            snapshot,
                                                        )
                                                        .await;
                                                    }
                                                    println!(
                                                        "✅ Sent active_deck response via relay"
                                                    );
                                                }
                                                None => {
                                                    eprintln!("❌ Failed to fetch active deck");
                                                }
                                            }
                                        }
                                        "get_all_decks" => {
                                            println!("🔄 Processing get_all_decks via relay");
                                            let db = Database::new(&db_pool_relay);
                                            // Get icons base path from IconServiceState (proper way)
                                            let icons_base_path = app_handle_relay.try_state::<adapters::icons_commands::IconServiceState>()
                                        .map(|icon_state| icon_state.service.get_icons_base_dir());
                                            // Fetch all decks with icons
                                            match db
                                                .get_all_decks_with_buttons(
                                                    true,
                                                    icons_base_path.as_deref(),
                                                )
                                                .await
                                            {
                                                Ok(decks_with_buttons) => {
                                                    let response = json!({
                                                        "action": "all_decks",
                                                        "decks": decks_with_buttons
                                                    });
                                                    broadcast_helper::broadcast_to_all_clients(
                                                        &app_handle_relay,
                                                        &state_manager_relay,
                                                        response,
                                                    )
                                                    .await;
                                                    println!(
                                                        "✅ Sent all_decks response via relay"
                                                    );
                                                }
                                                Err(e) => {
                                                    eprintln!(
                                                        "❌ Failed to fetch all decks: {}",
                                                        e
                                                    );
                                                }
                                            }
                                        }
                                        "update_active_deck" => {
                                            println!("🔄 Processing update_active_deck via relay");
                                            if let Some(deck_id) = payload
                                                .get("payload")
                                                .and_then(|p| p.get("deck_id"))
                                                .and_then(|id| id.as_u64())
                                            {
                                                let db = Database::new(&db_pool_relay);
                                                match db.update_active_deck(deck_id as u16).await {
                                                    Ok(_) => {
                                                        println!(
                                                            "✅ Updated active deck to ID {} via relay",
                                                            deck_id
                                                        );

                                                        // Broadcast updated deck to all clients
                                                        // Get icons base path from IconServiceState (proper way)
                                                        let icons_base_path = app_handle_relay.try_state::<adapters::icons_commands::IconServiceState>()
                                                    .map(|icon_state| icon_state.service.get_icons_base_dir());
                                                        if let Some(deck_with_buttons) = db
                                                            .get_active_deck_with_buttons(
                                                                true,
                                                                icons_base_path.as_deref(),
                                                            )
                                                            .await
                                                        {
                                                            let response = json!({
                                                                "action": "active_deck",
                                                                "deck": deck_with_buttons
                                                            });
                                                            broadcast_helper::broadcast_to_all_clients(&app_handle_relay, &state_manager_relay, response).await;
                                                        }
                                                    }
                                                    Err(e) => {
                                                        eprintln!(
                                                            "❌ Failed to update active deck: {}",
                                                            e
                                                        );
                                                    }
                                                }
                                            } else {
                                                eprintln!(
                                                    "❌ Invalid update_active_deck payload: missing deck_id"
                                                );
                                            }
                                        }
                                        _ => {
                                            // TODO: Handle other relay actions if needed
                                            println!("⚠️ Unhandled relay action: {}", action);
                                        }
                                    }
                                }
                            }
                            vortideck_core::events::types::RelayEventPayload::MobileDisconnected {
                                client_id,
                            } => {
                                command_handler::cancel_widget_repeats_for_client(&client_id).await;
                            }
                            _ => {
                                // Other relay events (StatusChanged, MobileConnected, etc.) are handled by platform_adapter
                            }
                        }
                    }
                }

                println!("⚠️ EventBus relay command handler stopped");
            });
        } // End scope for relay command handler

        // 🔐 PHASE 2: Listen for relay E2EE handshake completion to persist shared secret
        let app_handle_e2ee = app_handle.clone();
        app_handle.listen("e2ee_relay_handshake_complete", move |event| {
            println!("🔐 Received E2EE relay handshake completion event");
            let payload_str = event.payload();

            if let Ok(json) = serde_json::from_str::<serde_json::Value>(payload_str) {
                if let Some(client_id_val) = json.get("client_id") {
                    let client_id = client_id_val.as_str().unwrap_or("").to_string();

                    if !client_id.is_empty() {
                        let app_handle = app_handle_e2ee.clone();
                        tauri::async_runtime::spawn(async move {
                            // Get StateManager and retrieve the shared secret
                            if let Some(state_manager) = app_handle.try_state::<Arc<StateManager>>() {
                                if let Some(secret) = state_manager.get_shared_secret(&client_id).await {
                                    // Get database pool and persist the secret
                                    if let Some(db_pool) = app_handle.try_state::<DbPool>() {
                                        let db = Database::new(&db_pool);
                                        match db.save_shared_secret(&client_id, &secret) {
                                            Ok(_) => {
                                                println!("💾 Relay E2EE shared secret persisted to database for: {}", client_id);
                                            }
                                            Err(e) => {
                                                eprintln!("⚠️ Failed to persist relay shared secret to database: {}", e);
                                            }
                                        }
                                    } else {
                                        eprintln!("⚠️ Database pool not available for relay secret persistence");
                                    }
                                } else {
                                    eprintln!("⚠️ Shared secret not found in StateManager for: {}", client_id);
                                }
                            } else {
                                eprintln!("⚠️ StateManager not available for relay secret persistence");
                            }
                        });
                    } else {
                        eprintln!("⚠️ Empty client_id in E2EE relay handshake event");
                    }
                } else {
                    eprintln!("⚠️ Missing client_id in E2EE relay handshake event");
                }
            } else {
                eprintln!("❌ Failed to parse E2EE relay handshake event payload");
            }
        });
    }

    // 💤 Start Windows power event monitor for sleep/wake detection
    power_events::start_power_event_monitor(app_handle.clone());

    // 📊 PostHog analytics (logger is active here in setup)
    log::info!(
        "📊 PostHog: {}",
        if std::env::var("POSTHOG_API_KEY")
            .ok()
            .filter(|k| !k.is_empty())
            .is_some()
        {
            "active"
        } else {
            "inactive (no API key)"
        }
    );
    // Sync analytics_enabled flag from DB
    if let Some(pool) = app_handle.try_state::<crate::db::DbPool>() {
        if let Ok(conn) = &mut pool.get() {
            use crate::db::schema::app_config::dsl::*;
            use diesel::prelude::*;
            if let Ok(config) = app_config.first::<crate::db::models::AppConfig>(conn) {
                crate::analytics::ANALYTICS_ENABLED.store(
                    config.analytics_enabled,
                    std::sync::atomic::Ordering::Relaxed,
                );
            }
        }
    }
    crate::analytics::capture(
        &app_handle,
        crate::analytics::AnalyticsEvent::new("app_opened"),
    );

    Ok(())
}

// use self::db::schema::peers::dsl::*;
use crate::db::utils::*;
async fn start_services(app_handle: AppHandle, ip: IpAddr, icons_dir: Arc<std::path::PathBuf>) {
    // 🚀 NEW ARCHITECTURE: Use StateManager directly (with error handling for state access)
    let state_manager = match app_handle.try_state::<Arc<StateManager>>() {
        Some(state) => state,
        None => {
            println!("⚠️ StateManager not yet available, waiting for app initialization...");
            // Wait a bit for the app to fully initialize
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            match app_handle.try_state::<Arc<StateManager>>() {
                Some(state) => state,
                None => {
                    eprintln!(
                        "❌ StateManager still not available after wait - cannot start services"
                    );
                    return;
                }
            }
        }
    };

    // Check if services are already running
    if let Some(server_info) = state_manager.get_server_info() {
        println!(
            "Services already running on {}:{}, skipping restart",
            server_info.ip, server_info.port
        );
        log::info!(
            "Services already running on {}:{}, skipping restart",
            server_info.ip,
            server_info.port
        );
        return; // Exit early if services are already running
    }

    // 🧹 Clear any stale peer connections before starting
    state_manager.clear_stale_peers().await;
    println!("🧹 Cleared stale peer connections before starting services");

    // ============================================================================
    // WEBHOOK HANDLER - Initialize before Warp server
    // ============================================================================
    println!("🔗 Initializing Webhook handler...");
    log::info!("🔗 Initializing Webhook handler...");

    // Create webhook service for handler (separate instance from commands)
    let db_pool = app_handle.state::<DbPool>();
    let webhook_storage_handler = Arc::new(vortideck_core::services::webhook::WebhookStorage::new(
        db_pool.inner().clone(),
    ));
    let webhook_service_for_handler = Arc::new(
        vortideck_core::services::webhook::WebhookService::new(webhook_storage_handler),
    );

    // Create channel for webhook events to trigger flows
    let (webhook_tx, mut webhook_rx) = tokio::sync::mpsc::unbounded_channel::<
        vortideck_core::services::webhook::WebhookTriggerEvent,
    >();

    // Get EventBus from platform context
    let handler_event_bus = app_handle
        .state::<Arc<vortideck_core::core::PlatformContext>>()
        .event_bus
        .clone();

    // Create webhook handler (platform-agnostic)
    let webhook_test_sessions = app_handle
        .state::<vortideck_core::services::webhook::WebhookServiceState>()
        .test_sessions
        .clone();
    let webhook_handler = Arc::new(vortideck_core::services::webhook::WebhookHandler::new(
        webhook_service_for_handler.clone(),
        webhook_test_sessions,
        Some(webhook_tx),
        Some(handler_event_bus), // Pass EventBus for real-time updates
    ));

    println!("✅ Webhook handler initialized");
    log::info!("✅ Webhook handler initialized");

    // Keep one stable default for local integrations such as OBS. Discovery,
    // QR, Tailscale, and relay metadata consume the runtime value below.
    // Process-scoped capabilities rotate on restart. Overlay rendering and
    // operational metrics intentionally never share credentials.
    let access_tokens = overlay::ServerAccessTokens::generate();

    let tailscale_status = tailscale::detect().await;
    let tailscale_ips = tailscale_status
        .as_ref()
        .map(|status| status.ips.clone())
        .unwrap_or_default();
    let tailscale_dns_name = tailscale_status.and_then(|status| status.dns_name);
    let mut requested_listener_ips = vec![ip];
    requested_listener_ips.extend(
        tailscale_ips
            .iter()
            .copied()
            .filter(|candidate| *candidate != ip),
    );
    requested_listener_ips.sort();
    requested_listener_ips.dedup();

    println!(
        "🚀 Starting unified Warp server (WebSocket, Icons, Overlay, Webhooks); preferred endpoint {}:{}",
        ip, DEFAULT_UNIFIED_SERVER_PORT
    );

    // Start Warp server with all services unified
    let warp_listener_ips = requested_listener_ips;
    let warp_identity_ip = ip;
    let warp_icons_dir = icons_dir.clone();
    let warp_app_handle = app_handle.clone();
    let warp_webhook_handler = webhook_handler.clone();
    let warp_overlay_access_token = access_tokens.overlay.clone();
    let warp_metrics_access_token = access_tokens.metrics.clone();
    let (server_ready_tx, server_ready_rx) = tokio::sync::oneshot::channel();

    tauri::async_runtime::spawn(async move {
        start_warp_server(
            warp_listener_ips,
            warp_identity_ip,
            DEFAULT_UNIFIED_SERVER_PORT,
            warp_icons_dir.to_path_buf(),
            warp_app_handle,
            warp_webhook_handler,
            warp_overlay_access_token,
            warp_metrics_access_token,
            server_ready_tx,
        )
        .await;
    });

    // Wait for Warp to reserve its sockets. This removes the old check-then-bind
    // race and ensures every discovery surface receives the actual bound port.
    let (warp_port, bound_listener_ips) =
        match tokio::time::timeout(Duration::from_secs(5), server_ready_rx).await {
            Ok(Ok(Ok(binding))) => binding,
            Ok(Ok(Err(error))) => {
                log::error!("Could not start unified server: {}", error);
                return;
            }
            Ok(Err(_)) => {
                log::error!("Unified server stopped before reporting its binding");
                return;
            }
            Err(_) => {
                log::error!("Timed out while starting unified server");
                return;
            }
        };

    let mut connection_candidates = vec![ConnectionCandidate {
        transport: "lan".to_string(),
        host: ip.to_string(),
        port: warp_port,
        path: "/ws".to_string(),
        secure: false,
        priority: 10,
    }];
    let bound_tailscale_ips: Vec<IpAddr> = tailscale_ips
        .into_iter()
        .filter(|candidate| bound_listener_ips.contains(candidate))
        .collect();
    for tailscale_ip in &bound_tailscale_ips {
        connection_candidates.push(ConnectionCandidate {
            transport: "tailscale".to_string(),
            host: tailscale_ip.to_string(),
            port: warp_port,
            path: "/ws".to_string(),
            secure: false,
            priority: 20,
        });
    }
    if !bound_tailscale_ips.is_empty() {
        if let Some(dns_name) = tailscale_dns_name {
            connection_candidates.push(ConnectionCandidate {
                transport: "tailscale".to_string(),
                host: dns_name,
                port: warp_port,
                path: "/ws".to_string(),
                secure: false,
                priority: 30,
            });
        }
        log::info!("Tailscale direct-connect candidates are available");
    }

    // Spawn task to handle webhook trigger events
    let webhook_app_handle = app_handle.clone();
    let webhook_service_clone = webhook_service_for_handler.clone();

    tauri::async_runtime::spawn(async move {
        // Use the webhook handler module
        webhook::handle_webhook_events(webhook_app_handle, webhook_service_clone, webhook_rx).await;
    });

    // MCP Server Settings
    // let app_config = db::utils::get_app_config(app_handle.state::<DbPool>()).await;
    // println!("App Config: {:?}", app_config);
    // let mcp_enabled = match app_config {
    //     Ok(json) => json.get("data")
    //         .and_then(|data| data.get("mcp_enabled"))
    //         .and_then(|val| val.as_bool())
    //         .unwrap_or(false),
    //     Err(_) => false,
    // };

    // println!("MCP Enabled: {:?}", mcp_enabled);
    // log::info!("MCP Enabled: {:?}", mcp_enabled);

    // if enabled start mcp server

    // use crate::services::mcp::commands::InitRequest;
    // if mcp_enabled {
    //     println!("Starting MCP Server");
    //     log::info!("Starting MCP Server");
    //     let state = app_handle.state::<DbPool>();
    //     let request = InitRequest {
    //         config_path: None,
    //         openai_key: None,
    //         model_name: None,
    //     };
    //     use crate::services::mcp::commands::ChatState;
    //     mcp::commands::init_chat(app_handle.clone(), app_handle.state::<ChatState>(), app_handle.state::<FlowRegistryState>(), request).await;
    // }

    // Update app state with the unified server's runtime endpoint.
    // All services (WebSocket, Icons, Overlay) are on the same port now
    let info = PlatformInfo::new().expect("Unable to determine platform info");
    state_manager.set_server_info(ServerInfo {
        ip,
        port: warp_port,              // Unified Warp server port
        icons_server_port: warp_port, // Icons are served on same port via /icons/ route
        overlay_access_token: access_tokens.overlay,
        metrics_access_token: access_tokens.metrics,
        device_type: info.sysname().to_string_lossy().to_string(),
        server_name: info.nodename().to_string_lossy().to_string(),
        connection_candidates,
    });
    println!("🌐 Server info set in StateManager");

    // WebSocket is a route on the unified Warp listener bound above.
    println!("WebSocket route available at ws://{}:{}/ws", ip, warp_port);
    log::info!("WebSocket route available at ws://{}:{}/ws", ip, warp_port);

    // 🔥 NEW: Client access URLs through unified Warp server
    println!(
        "\n📡 Client Access URLs (unified Warp server on {}:{}):",
        ip, warp_port
    );
    println!("   WebSocket: ws://{}:{}/ws", ip, warp_port);
    println!("   Overlay: http://{}:{}/overlay.html", ip, warp_port);
    println!("   Icons: http://{}:{}/icons/", ip, warp_port);
    log::info!(
        "All services unified on Warp server at {}:{}",
        ip,
        warp_port
    );

    // mDNS advertises the unified Warp server with desktop_id
    let app_handle_mdns = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        // Get desktop_id from state manager
        let desktop_id =
            if let Some(state_manager) = app_handle_mdns.try_state::<Arc<StateManager>>() {
                state_manager.get_desktop_id().await
            } else {
                None
            };

        // Get hostname from system
        let hostname = hostname::get().ok().and_then(|h| h.into_string().ok());

        if let Err(err) =
            create_websocket_mdns(ip.to_string(), warp_port, desktop_id, hostname).await
        {
            eprintln!("Error in mDNS advertisement: {}", err);
            log::error!("Error in mDNS advertisement: {}", err);
        }
    });
}
async fn my_app(app_handle: AppHandle) {
    // Handle context menu deep link on first launch (app was not already running).
    // When already running, the single_instance callback handles it instead.
    let startup_args: Vec<String> = std::env::args().collect();
    log::debug!("[startup] my_app args: {:?}", startup_args);
    if let Some(url) = startup_args
        .iter()
        .find(|arg| arg.starts_with("vortideck://context-menu/"))
    {
        let url = url.clone();
        let app_handle_dl = app_handle.clone();
        println!("🔗 [startup] context-menu deep link found, scheduling handler");
        // Small delay so state (executor, DB pool) is fully managed before execution
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            handle_context_menu_url(&app_handle_dl, &url);
        });
    }

    // 🔊 Initialize volume change callback for mobile sync (after app setup is complete)
    let app_handle_volume = app_handle.clone();

    tokio::spawn(async move {
        // use adapters::volume_callback_adapter::register_tauri_volume_callback; // Removed - doesn't exist

        // if let Err(e) = register_tauri_volume_callback(app_handle_volume.clone()).await {
        //     println!("⚠️ Failed to register volume change callback: {}", e);
        //     log::warn!("⚠️ Failed to register volume change callback: {}", e);
        // } else {
        //     println!("🔊 Volume change callback registered for mobile sync");
        //     log::info!("🔊 Volume change callback registered for mobile sync");
        // }

        println!("🔊 Volume change callback registration skipped - function not implemented");

        // Listen for system volume change events and broadcast to mobile clients
        let app_handle_listener = app_handle_volume.clone();
        app_handle_volume.listen("system_volume_changed", move |event| {
            let app_handle = app_handle_listener.clone();

            tauri::async_runtime::spawn(async move {
                if let Ok(payload) = serde_json::from_str::<serde_json::Value>(event.payload()) {
                    let volume = payload["volume"].as_f64().unwrap_or(0.0);
                    let muted = payload["muted"].as_bool().unwrap_or(false);
                    let is_button_change = payload["is_button_change"].as_bool().unwrap_or(false);
                    let button_id = payload["button_id"].as_str().map(|s| s.to_string());

                    // 🔄 ANTI-RECURSION: Only broadcast to mobile if the change did NOT originate from a button/mobile
                    if !is_button_change {
                        // Get state manager from app handle within the async context (with error handling)
                        let state_manager = match app_handle.try_state::<Arc<StateManager>>() {
                            Some(state) => state,
                            None => {
                                println!("⚠️ StateManager not available for volume broadcast");
                                return;
                            }
                        };

                        // Broadcast to mobile clients via StateManager directly
                        state_manager.broadcast_volume_change_to_mobile(
                            volume as f32, // Keep as percentage (0-100 range) for mobile
                            muted,
                            is_button_change,
                            button_id
                        ).await;

                        println!("📱 Broadcasted volume change to mobile clients: {:.1}%, muted: {}", volume, muted);
                    } else {
                        println!("🔄 Skipped mobile broadcast - volume change originated from button/mobile (anti-recursion)");
                    }
                    command_handler::publish_widget_topic_for_active_deck(
                        &app_handle,
                        "system.volume",
                        json!({ "value": volume, "muted": muted }),
                    )
                    .await;
                }
            });
        });
    });

    // Initialize App Data Path and Icons Directory
    let app_data_dir = app_handle
        .path()
        .data_dir()
        .expect("Unable to find data dir");
    let icons_dir = Arc::new(app_data_dir.join("VortiDeck").join("icons"));

    // Check internet connection and start services
    let retry_delay = Duration::from_secs(5);
    let mut connected = false;

    while !connected {
        if check_internet_connection().await {
            // Internet is available, start services
            eprintln!("Internet connection available, starting services");
            connected = true;

            // Get the local IP only after confirming internet connection is available
            let ip = local_ip().unwrap_or_else(|_| "127.0.0.1".parse().unwrap());
            start_services(app_handle.clone(), ip, icons_dir.clone()).await;

            // Emit internet_status after services and server_info are fully initialized
            app_handle
                .emit_to("main", "internet_status", true)
                .unwrap_or_default();
        } else {
            // Internet is down, retry after delay
            eprintln!(
                "Internet connection unavailable, retrying in {} seconds",
                retry_delay.as_secs()
            );
            app_handle
                .emit_to("main", "internet_status", false)
                .unwrap_or_default();
            sleep(retry_delay).await;
        }
    }
}

// Helper function to check internet connection
async fn check_internet_connection() -> bool {
    // Try to connect to a reliable external service
    match reqwest::get("https://www.google.com").await {
        Ok(response) => response.status().is_success(),
        Err(_) => false,
    }
}

use crate::commands::auth_commands::exchange_auth_code;
use dotenvy::dotenv;

use vortideck_core::{
    core::ServerInfo, // Add ServerInfo import
    features::FeatureRegistry,
    integrations::framework::IntegrationRegistry,
    VortiDeckCore,
};

// 🚀 DIRECT ARCHITECTURE: Import Tauri adapters (no factory pattern)
use adapters::{
    commands::register_commands, create_tauri_platform_context, spawn_event_bridge,
    IntegrationEventEmitter,
};
// Use the adapter ChatState for full MCP functionality
// use vortideck_core::services::mcp::commands::ChatState;

// Add this helper function near the top of main.rs, after the imports
/// 🔄 REUSABLE FLOW REGISTRY REFRESH
/// Centralizes the logic for refreshing flow registry when plugins change
pub async fn refresh_flow_registry_with_plugins(
    vortideck_core: &VortiDeckCore,
    flow_registry_state: &FlowRegistryState,
    chat_state: &vortideck_core::services::mcp::commands::ChatState,
    context: &str,
) {
    println!("🔄 Refreshing flow registry after {} ...", context);

    // Re-register ALL plugins (WASM + Service) via the new architecture
    {
        use crate::adapters::flow_registry_adapter::TauriFlowRegistryAdapter;
        use std::sync::Arc;
        let adapter = Arc::new(TauriFlowRegistryAdapter::new(Arc::new(
            flow_registry_state.clone(),
        )));
        vortideck_core
            .plugin_manager()
            .set_flow_registry(adapter)
            .await;
    }

    // Handle MCP tools from global tool storage
    let mut mcp_node_count = 0;
    let mut mcp_provider_opt = None;

    {
        let tool_set_guard = chat_state.mcp_tools.lock().await;
        let tools = tool_set_guard.tools();
        println!(
            "🔍 Found {} MCP tools to convert to flow nodes",
            tools.len()
        );

        if !tools.is_empty() {
            // Create MCP flow provider from the global ToolSet
            use vortideck_core::services::mcp::flow_adapter::McpToolProvider;
            let mcp_provider = McpToolProvider::new(tool_set_guard.clone());
            let mcp_nodes = mcp_provider.get_flow_nodes();
            mcp_node_count = mcp_nodes.len();

            for node in &mcp_nodes {
                println!("  🔧 MCP flow node: {} ({})", node.name(), node.id());
            }

            mcp_provider_opt = Some(Box::new(mcp_provider));
        }
    }

    {
        let mut registry = flow_registry_state.registry.lock().unwrap();

        // Clear ALL plugin nodes before re-registering (prevents stale nodes from deleted plugins)
        registry.clear_provider_nodes("WasmPluginProvider");
        registry.clear_provider_nodes("ServiceFlowNodeProvider");
        println!("🧹 Cleared old plugin nodes from flow registry");

        // Register MCP tools if available
        if let Some(mcp_provider) = mcp_provider_opt {
            registry.clear_provider_nodes("McpToolProvider");
            registry.register_provider(mcp_provider);
        }

        let node_count = registry.get_node_count();
        println!(
            "🔌 Flow registry refreshed after {}: {} total nodes ({} MCP)",
            context, node_count, mcp_node_count
        );
    }
}

/// Handle a context menu deep link URL (shared by first-launch and single-instance paths).
/// Format: vortideck://context-menu/item-{id}?paths=url-encoded|pipe-separated
fn handle_context_menu_url(app_handle: &AppHandle, url: &str) {
    log::info!("[context-menu] URL received: {}", url);
    let remainder = url.strip_prefix("vortideck://context-menu/").unwrap_or("");
    let (action_id, query_string) = if let Some(q) = remainder.find('?') {
        (&remainder[..q], &remainder[q + 1..])
    } else {
        (remainder, "")
    };

    let paths: Vec<String> = query_string
        .split('&')
        .find(|p| p.starts_with("paths="))
        .map(|p| &p[6..])
        .map(|encoded| url_decode(encoded))
        .unwrap_or_default()
        .split('|')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();

    log::debug!(
        "[context-menu] action_id={:?}, paths={:?}",
        action_id,
        paths
    );

    // 📊 Track context menu usage
    crate::analytics::capture(
        &app_handle,
        crate::analytics::AnalyticsEvent::new("context_menu_triggered")
            .prop("action_id", action_id)
            .prop("file_count", paths.len() as u64),
    );

    if let Some(item_id) = action_id.strip_prefix("item-") {
        execute_context_menu_item(app_handle, item_id, &paths);
    } else if let Some(page) = action_id.strip_prefix("open-page-") {
        if let Some(window) = app_handle.get_webview_window("main") {
            let _ = window.emit("navigate-to-page", page);
        }
    }
}

/// Simple URL decoding for context menu file paths.
fn url_decode(s: &str) -> String {
    let mut decoded = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            // Slice bytes, not the str: `&s[i + 1..i + 3]` panics when a
            // multibyte character follows the '%' in an external deep link.
            if let Some(byte) = std::str::from_utf8(&bytes[i + 1..i + 3])
                .ok()
                .and_then(|hex| u8::from_str_radix(hex, 16).ok())
            {
                decoded.push(byte);
                i += 3;
                continue;
            }
        }
        decoded.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&decoded).to_string()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Install default crypto provider for rustls (required for WebSocket plugin)
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    if let Some(exit_code) = commands::obs_companion_commands::run_elevated_helper_from_args() {
        std::process::exit(exit_code);
    }

    dotenv().ok();

    #[cfg(debug_assertions)]
    {
        if std::env::var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS").is_err() {
            unsafe {
                std::env::set_var(
                    "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
                    "--remote-debugging-port=9222",
                );
            }
        }
    }

    // Initialize PostHog via spawn_blocking — the crate creates a
    // reqwest::blocking::Client (with its own tokio runtime) internally,
    // which cannot be constructed inside an active async runtime.
    // Guard is leaked so its Drop doesn't block-flush inside async context.
    let posthog_key = std::env::var("POSTHOG_API_KEY")
        .ok()
        .filter(|k| !k.is_empty());
    tokio::task::spawn_blocking(move || {
        let guard = better_posthog::init(better_posthog::ClientOptions {
            api_key: posthog_key.map(Into::into),
            host: better_posthog::Host::EU,
            ..Default::default()
        });
        std::mem::forget(guard);
    })
    .await
    .expect("PostHog init failed");

    let mut builder = tauri::Builder::default();
    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(
            |app_handle, argv, _cwd| {
                log::debug!("[single-instance] callback fired! argv={:?}", argv);
                let app_handle = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    // Check if any arguments match our deep link scheme

                    if let Some(url) = argv
                        .iter()
                        .find(|arg| arg.starts_with("vortideck://oauth/callback"))
                    {
                        // println!("Deep link URL received: {}", url);
                        let encrypted_code = url
                            .split("code=")
                            .nth(1)
                            .unwrap_or_default()
                            .split("&")
                            .next()
                            .unwrap_or_default();
                        let login_id = url
                            .split("login_id=")
                            .nth(1)
                            .unwrap_or_default()
                            .split("&")
                            .next()
                            .unwrap_or_default();
                        // println!("encrypted code: {}", encrypted_code);
                        if let Err(e) = exchange_auth_code(
                            encrypted_code.to_string(),
                            login_id.to_string(),
                            app_handle.clone(),
                        )
                        .await
                        {
                            eprintln!("Failed to exchange auth code: {}", e);
                            log::error!("Failed to exchange auth code: {}", e);
                        }
                    } else if let Some(url) = argv
                        .iter()
                        .find(|arg| arg.starts_with("vortideck://context-menu/"))
                    {
                        log::info!("[single-instance] context-menu deep link: {}", url);
                        handle_context_menu_url(&app_handle, url);
                    } else if let Some(url) = argv
                        .iter()
                        .find(|arg| arg.starts_with("vortideck://open_page/"))
                    {
                        // Extract the page path from the deeplink
                        let page_path = url
                            .strip_prefix("vortideck://open_page/")
                            .unwrap_or("")
                            .to_string();

                        // Focus the main window first
                        if let Some(window) = app_handle.get_webview_window("main") {
                            let _ = window.set_focus();

                            // Emit navigation event to frontend
                            let _ = window.emit("navigate-to-page", &page_path);
                        }
                    }

                    if let Some(window) = app_handle.get_webview_window("main") {
                        let _ = window.set_focus();
                    }
                });
            },
        ));
    }

    let runtime_shutdown_started = Arc::new(std::sync::atomic::AtomicBool::new(false));
    builder
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(
                    tauri_plugin_window_state::StateFlags::all()
                        - tauri_plugin_window_state::StateFlags::VISIBLE,
                )
                .with_denylist(&["splashscreen", "mobile_simulator"])
                .build(),
        )
        .setup(setup)
        // .manage(state)
        // .manage(listener.clone())
        .invoke_handler(tauri::generate_handler![
            set_complete,
            trigger_plugin_loading,
            // API Commands (HTTP Client)
            commands::api_commands::http_request,
            commands::api_commands::send_webhook,
            commands::api_commands::send_webhook_event,
            // HTTP Request Template Commands
            commands::http_request_commands::save_http_request,
            commands::http_request_commands::list_http_requests,
            commands::http_request_commands::get_http_request_by_id,
            commands::http_request_commands::update_http_request,
            commands::http_request_commands::delete_http_request,
            commands::http_request_commands::execute_http_request_by_id,
            commands::http_request_commands::test_http_request,
            // Focus Window Commands
            commands::focus_window_commands::get_visible_windows,
            commands::focus_window_commands::get_focus_windows,
            commands::focus_window_commands::get_focus_window_by_id,
            commands::focus_window_commands::create_focus_window,
            commands::focus_window_commands::update_focus_window,
            commands::focus_window_commands::delete_focus_window,
            // Webhook Management Commands
            commands::webhook_commands::get_webhooks,
            commands::webhook_commands::get_webhook,
            commands::webhook_commands::create_webhook,
            commands::webhook_commands::update_webhook,
            commands::webhook_commands::delete_webhook,
            commands::webhook_commands::deactivate_test_mode,
            commands::webhook_commands::set_webhook_response_mode,
            commands::webhook_commands::set_webhook_allowed_methods,
            commands::webhook_commands::register_webhook_with_service,
            commands::webhook_commands::unregister_webhook_from_service,
            commands::webhook_commands::check_external_webhook,
            commands::webhook_commands::get_webhook_statistics,
            commands::webhook_commands::send_test_webhook,
            commands::generate_qr_code::generate_qr_code,
            commands::discover_websocket::discover_websocket,
            commands::retrieve_peers::retrieve_peers,
            commands::retrieve_peers::retrieve_active_peers,
            commands::open_mobile_simulator::open_mobile_simulator,
            commands::open_mobile_simulator::toggle_simulator_pin,
            commands::open_mobile_simulator::toggle_simulator_always_on_top,
            commands::open_mobile_simulator::get_simulator_always_on_top,
            commands::open_mobile_simulator::get_mobile_simulator_pinned,
            commands::open_logs::open_logs,
            commands::open_in_new_window::open_in_new_window,
            commands::open_overlay_window::open_overlay_window,
            // 🚀 UNIVERSAL ARCHITECTURE: Universal system commands (original names, universal implementation)
            commands::universal_system_commands::set_volume,
            commands::universal_system_commands::get_system_info_universal_cmd,
            commands::universal_system_commands::get_available_actions,
            commands::universal_system_commands::execute_system_action,
            // system_control::commands::set_brightness,
            // ✅ NEW ARCHITECTURE: Complete macro system with new function names
            adapters::macro_commands::start_real_time_macro_recording,
            adapters::macro_commands::stop_real_time_macro_recording,
            adapters::macro_commands::reset_macro_recording,
            adapters::macro_commands::get_macro_recording_state,
            adapters::macro_commands::execute_macro,
            adapters::macro_commands::get_user_macros,
            adapters::macro_commands::delete_macro,
            adapters::macro_commands::save_macro_to_db,
            // TODO: Add window focus commands when Tauri API is clarified
            // Legacy icons commands replaced with new adapter architecture
            adapters::icons_commands::save_icon,
            adapters::icons_commands::remove_icon,
            adapters::icons_commands::get_icon_path,
            adapters::icons_commands::get_icon_storage_stats,
            adapters::icons_commands::read_file_as_base64,
            #[cfg(feature = "icons-server")]
            adapters::icons_commands::start_icons_server,
            #[cfg(feature = "icons-server")]
            adapters::icons_commands::get_icons_server_info,
            commands::obs_companion_commands::get_obs_companion_overview,
            commands::obs_companion_commands::inspect_obs_companion_target,
            commands::obs_companion_commands::install_obs_companion,
            commands::obs_companion_commands::repair_obs_companion,
            commands::obs_companion_commands::remove_obs_companion,
            commands::obs_companion_commands::forget_obs_companion_target,
            commands::overlay_commands::send_overlay_message,
            db::get_all_decks,
            db::remove_deck,
            db::get_active_deck_id,
            db::utils::get_feature_limit,
            db::utils::change_button_position,
            db::utils::set_button_action,
            db::utils::set_button_mode,
            db::utils::set_button_icon_fit,
            db::utils::set_button_execution_mode,
            db::utils::get_active_deck,
            db::utils::create_new_deck,
            db::utils::update_active_deck,
            // db::utils::save_macro_to_db, // ✅ MIGRATED: Using new architecture bridge command
            db::utils::update_macro,
            db::utils::get_all_macros,
            db::utils::get_macro_by_id,
            db::utils::remove_macro,
            // Program commands
            db::utils::save_program_to_db,
            db::utils::get_saved_programs,
            db::utils::get_programs_with_meta,
            db::utils::get_program_by_id,
            db::utils::update_program_in_db,
            db::utils::remove_program_from_db,
            db::utils::reset_button,
            db::utils::update_deck,
            db::utils::update_deck_grid,
            db::utils::update_deck_settings,
            db::utils::download_theme,
            db::utils::uninstall_theme,
            db::utils::get_themes,
            db::utils::get_app_config,
            db::utils::update_app_config,
            commands::state_commands::get_server_info_new,
            commands::state_commands::get_metrics_access_token,
            commands::state_commands::get_obs_plugin_status,
            // Legacy OBS commands - temporarily commented out
            // obs::invoke_obs_action,
            // obs::get_obs_actions,
            // obs::is_obs_installed,
            commands::get_installed_apps::get_installed_apps,
            commands::auth_commands::get_refresh_token,
            commands::auth_commands::get_access_token,
            commands::auth_commands::set_refresh_token,
            commands::auth_commands::refresh_access_token,
            commands::auth_commands::logout_user,
            utils::check_premium_status,
            show_window_if_complete,
            // New State Management Commands (for testing and migration)
            commands::state_commands::get_mobile_simulator_pin_status,
            commands::state_commands::pin_mobile_simulator,
            commands::state_commands::unpin_mobile_simulator,
            // commands::state_commands::get_obs_status_new, // Function removed - using get_obs_plugin_status
            commands::state_commands::set_obs_status_new,
            commands::state_commands::get_peers_new,
            commands::state_commands::broadcast_message_new,
            commands::state_commands::get_premium_status_new,
            commands::state_commands::check_plugin_permission,
            commands::state_commands::set_plugin_permissions,
            commands::state_commands::get_public_state_for_plugin,
            commands::state_commands::compare_state_systems,
            commands::state_commands::enable_new_state_system,
            commands::state_commands::get_state_system_status,
            // MCP Chat Commands - PROPER ADAPTER ARCHITECTURE ✅
            adapters::mcp_commands::init_chat,
            adapters::mcp_commands::connect_mcp_server,
            adapters::mcp_commands::disconnect_mcp_server,
            // Flow Control Commands - PROPER ADAPTER ARCHITECTURE
            adapters::flow_commands::retrieve_system_tools,
            adapters::flow_commands::retrieve_all_flow_nodes,
            adapters::flow_commands::execute_flow_node,
            adapters::flow_commands::retrieve_all_tools,
            adapters::flow_commands::save_flow,
            adapters::flow_commands::preflight_flow_actions,
            adapters::flow_commands::load_flow,
            adapters::flow_commands::list_flows,
            adapters::flow_commands::delete_flow,
            adapters::flow_commands::open_flow_editor,
            adapters::flow_commands::open_workflow_approval,
            // Flow Execution Commands (✅ All Used by Frontend)
            adapters::flow_commands::init_flow_executor,
            adapters::flow_commands::start_flow_execution,
            adapters::flow_commands::get_flow_execution_status,
            adapters::flow_commands::cancel_flow_execution,
            adapters::flow_commands::list_flow_recovery_runs,
            adapters::flow_commands::list_flow_execution_history,
            adapters::flow_commands::list_workflow_wakes,
            adapters::flow_commands::resolve_workflow_approval,
            adapters::flow_commands::fail_flow_recovery_run,
            adapters::flow_commands::resume_flow_recovery_run,
            adapters::flow_commands::inspect_workflow_runtime,
            adapters::flow_commands::export_workflow_metrics,
            adapters::flow_commands::trace_workflow_event,
            adapters::flow_commands::replay_workflow_dead_letter,
            // Flow Button Assignment Commands
            adapters::flow_commands::assign_flow_to_button,
            // Plugin Management Commands
            plugins::commands::get_installed_plugins,
            plugins::commands::get_plugin_info,
            plugins::commands::toggle_plugin_enabled,
            plugins::commands::uninstall_plugin,
            plugins::commands::refresh_plugins,
            plugins::commands::reload_plugins_with_auth_check,
            plugins::commands::get_plugins_directory,
            plugins::commands::debug_plugin_system,
            plugins::commands::get_plugin_web_extensions,
            plugins::commands::get_plugin_asset,
            // Plugin Premium Validation Commands (for testing/debugging)
            commands::plugin_commands::can_install_plugin,
            commands::plugin_commands::get_plugin_usage_stats,
            commands::plugin_commands::get_detailed_plugin_info,
            commands::plugin_commands::test_plugin_count_fix,
            // Plugin Sandboxing API Bridge Commands
            commands::plugin_commands::register_plugin_frame_session,
            commands::plugin_commands::revoke_plugin_frame_session,
            commands::plugin_commands::get_plugin_frame_scope,
            commands::plugin_commands::plugin_call_wasm,
            commands::plugin_commands::plugin_run_flow,
            commands::plugin_commands::plugin_storage_get,
            commands::plugin_commands::plugin_storage_set,
            commands::plugin_commands::open_plugin_window,
            // Universal Integration Commands
            commands::universal_integration_commands::get_action_runtime_readiness,
            commands::universal_integration_commands::get_integration_status,
            commands::universal_integration_commands::get_all_integration_statuses,
            commands::universal_integration_commands::get_integration_capabilities,
            commands::universal_integration_commands::resolve_integration_capability_options,
            commands::universal_integration_commands::configure_discord_webhook,
            commands::universal_integration_commands::disconnect_discord,
            commands::universal_integration_commands::is_integration_connected,
            commands::universal_integration_commands::get_connected_integrations,
            commands::universal_integration_commands::refresh_integration_statuses,
            commands::universal_integration_commands::get_integration_summary,
            // Universal Action Commands (for saved actions in DB)
            commands::universal_action_commands::save_integration_action_to_db,
            commands::universal_action_commands::update_integration_action_in_db,
            commands::universal_action_commands::get_integration_action_by_id,
            commands::universal_action_commands::get_saved_integration_action_by_stable_id,
            commands::universal_action_commands::get_saved_integration_actions,
            commands::universal_action_commands::get_all_saved_integration_actions,
            commands::universal_action_commands::preflight_saved_integration_actions,
            commands::universal_action_commands::preflight_saved_integration_action,
            commands::universal_action_commands::invoke_action_reference,
            commands::universal_action_commands::get_action_input_contract,
            commands::universal_action_commands::save_action_input_contract,
            commands::universal_action_commands::get_action_invocation,
            commands::universal_action_commands::list_action_invocations,
            commands::universal_action_commands::get_saved_action_dependencies,
            commands::universal_action_commands::rebind_saved_integration_action,
            commands::universal_action_commands::replace_saved_action_references,
            commands::universal_action_commands::disable_flows_using_saved_action,
            commands::universal_action_commands::remove_integration_action_from_db,
            // 🎯 OBS Banner Commands
            commands::obs_banner_commands::obs_banner_set_data,
            commands::obs_banner_commands::obs_banner_set_queue,
            commands::obs_banner_commands::obs_banner_add_to_queue,
            commands::obs_banner_commands::obs_banner_hide,
            commands::obs_banner_commands::obs_banner_show,
            commands::obs_banner_commands::obs_analytics_enable,
            commands::obs_banner_commands::obs_analytics_get_report,
            commands::obs_banner_commands::obs_banner_get_queue_status,
            commands::obs_banner_commands::obs_banner_test,
            // 🔄 Context-Aware Ad Refresh Commands
            commands::obs_banner_commands::smart_fetch_ads_with_context,
            commands::obs_banner_commands::update_stream_context,
            commands::obs_banner_commands::get_refresh_recommendations,
            commands::obs_banner_commands::clear_context_cache,
            // Simple Ad Service Commands
            // 🎨 Overlay System Commands (Clean Architecture)
            commands::overlay_commands::generate_overlay_html_command,
            commands::overlay_commands::batch_sync_overlay,
            commands::overlay_authoring::list_overlay_compositions,
            commands::overlay_authoring::read_overlay_composition,
            commands::overlay_authoring::create_overlay_composition,
            commands::overlay_authoring::edit_overlay_composition,
            commands::overlay_authoring::delete_overlay_composition,
            commands::overlay_authoring::validate_overlay_composition,
            commands::overlay_authoring::validate_draft_widget_settings,
            commands::overlay_authoring::export_overlay_setup,
            commands::overlay_authoring::import_overlay_setup,
            commands::overlay_authoring::pick_overlay_image,
            commands::overlay_authoring::approve_imported_overlay_content,
            overlay::settings::get_overlay_widget_settings,
            overlay::settings::configure_overlay_widget,
            // add_overlay_element removed - V3 uses streaming elements system
            commands::overlay_commands::add_overlay_element_v3, // V3 scalable with HTML
            commands::overlay_commands::remove_overlay_element,
            commands::overlay_commands::update_overlay_element,
            commands::overlay_commands::clear_overlay_elements,
            commands::overlay_commands::set_overlay_design,
            commands::overlay_commands::get_overlay_status,
            commands::overlay_commands::get_saved_canvas_size,
            commands::overlay_commands::start_overlay_server_command,
            commands::overlay_commands::stop_overlay_server_command,
            commands::overlay_commands::send_overlay_to_obs,
            commands::overlay_commands::apply_obs_canvas_size,
            overlay::companion_auth::obs_companion_pairing_key,
            // add_test_overlay_element removed - V3 uses streaming elements
            // commands::overlay_commands::trigger_test_overlay_ad,
            commands::overlay_commands::broadcast_element_patch,
            commands::overlay_commands::broadcast_element_update,
            commands::overlay_commands::broadcast_overlay_vars,
            commands::overlay_commands::set_overlay_theme_vars,
            commands::widget_commands::revoke_widget,
            commands::widget_commands::get_widget_permissions,
            commands::widget_commands::list_available_widgets,
            commands::widget_commands::list_package_discovery_errors,
            commands::widget_commands::list_packages,
            commands::widget_commands::grant_package_permissions,
            marketplace::list_plugin_library,
            marketplace::install_marketplace_release,
            commands::widget_commands::get_deck_widget_document,
            commands::widget_commands::save_deck_widget_document,
            commands::widget_commands::revert_deck_widget_document,
            execute_deck_widget_intent,
            commands::error_commands::report_frontend_crash,
            commands::overlay_commands::request_obs_canvas_size,
            commands::overlay_commands::send_banner_message,
            // 🚀 PROCESS MANAGEMENT: Process tracking and control commands
            commands::process_commands::get_all_processes,
            commands::process_commands::get_running_processes,
            commands::process_commands::get_process,
            commands::process_commands::kill_process,
            commands::process_commands::kill_all_processes,
            commands::process_commands::get_process_count,
            commands::process_commands::get_processes_by_button,
            commands::process_commands::get_processes_by_deck,
            commands::process_commands::kill_processes_by_button,
            commands::process_commands::kill_processes_by_deck,
            commands::process_commands::execute_program,
            // 🤖 AI Assistant commands
            commands::assistant_commands::open_assistant_window,
            commands::assistant_commands::send_chat_message,
            commands::assistant_commands::assistant_fill_value,
            commands::assistant_commands::get_assistant_suggestions,
            commands::assistant_commands::get_assistant_context,
            commands::assistant_commands::get_available_mcp_tools,
            commands::assistant_commands::update_chat_settings,
            commands::assistant_commands::save_default_chat_settings,
            commands::assistant_commands::get_default_chat_settings,
            commands::assistant_commands::read_mcp_config,
            commands::assistant_commands::read_model_cache,
            commands::assistant_commands::write_model_cache,
            commands::assistant_commands::abort_generation,
            // 💬 Conversation Management commands
            commands::conversation_commands::create_conversation,
            commands::conversation_commands::list_conversations,
            commands::conversation_commands::load_conversation_messages,
            commands::conversation_commands::delete_conversation,
            commands::conversation_commands::attach_conversation_to_session,
            commands::conversation_commands::create_and_attach_conversation,
            commands::conversation_commands::update_conversation_title,
            // Window Management commands
            commands::window_commands::open_streaming_window,
            commands::window_commands::close_streaming_window,
            commands::window_commands::toggle_streaming_window_always_on_top,
            commands::window_commands::get_streaming_window_always_on_top,
            commands::window_commands::move_streaming_window_to_monitor,
            commands::window_commands::get_monitors_info,
            commands::window_commands::get_global_modifiers,
            // Backend Relay commands
            commands::relay_commands::enable_relay,
            commands::relay_commands::disable_relay,
            commands::relay_commands::get_relay_status,
            commands::relay_commands::get_or_create_desktop_id,
            get_connection_diagnostics,
            // Streaming commands - WebSocket
            commands::streaming_commands::streaming_connect,
            commands::streaming_commands::streaming_disconnect,
            commands::streaming_commands::streaming_update_token,
            commands::streaming_commands::update_data_subscriptions,
            // Streaming commands - WebSocket Send Operations
            commands::streaming_commands::streaming_send_chat_message,
            commands::streaming_commands::streaming_ban_user,
            commands::streaming_commands::streaming_timeout_user,
            commands::streaming_commands::streaming_delete_message,
            // Streaming commands - Message Store
            commands::streaming_commands::streaming_get_messages,
            commands::streaming_commands::streaming_get_messages_for_accounts,
            // Streaming commands - Session Manager (auto-persists to DB)
            commands::streaming_commands::streaming_get_all_sessions,
            commands::streaming_commands::streaming_get_session,
            commands::streaming_commands::streaming_create_session,
            commands::streaming_commands::streaming_update_session,
            commands::streaming_commands::streaming_delete_session,
            commands::streaming_commands::streaming_activate_session,
            commands::streaming_commands::streaming_start_session,
            commands::streaming_commands::streaming_pause_session,
            commands::streaming_commands::streaming_resume_session,
            commands::streaming_commands::streaming_end_session,
            commands::streaming_commands::streaming_check_conflicts,
            commands::streaming_commands::streaming_check_auto_resume,
            commands::streaming_commands::streaming_find_session_by_account,
            commands::streaming_commands::streaming_is_session_live,
            commands::streaming_commands::streaming_get_session_account_ids,
            commands::streaming_commands::streaming_get_active_sessions,
            commands::streaming_commands::streaming_get_draft_sessions,
            commands::streaming_commands::streaming_get_recent_sessions,
            // Workspace commands
            commands::workspace_commands::get_workspace_root,
            commands::workspace_commands::get_workspace_output_dir,
            commands::workspace_commands::get_workspace_input_dir,
            commands::workspace_commands::get_workspace_temp_dir,
            commands::workspace_commands::open_workspace_directory,
            // Context Menu commands (Windows 11 modern context menu management)
            commands::context_menu_commands::get_context_menu_items,
            commands::context_menu_commands::get_context_menu_config,
            commands::context_menu_commands::add_context_menu_item,
            commands::context_menu_commands::update_context_menu_item,
            commands::context_menu_commands::remove_context_menu_item,
            commands::context_menu_commands::toggle_context_menu_item,
            commands::context_menu_commands::reorder_context_menu_items,
            commands::context_menu_commands::get_context_menu_status,
            commands::context_menu_commands::register_context_menu_extension,
            commands::context_menu_commands::unregister_context_menu_extension,
        ])
        .plugin(tauri_plugin_better_posthog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_websocket::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_pinia::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(
            tauri_plugin_log::Builder::new()
                .clear_targets()
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::LogDir {
                        file_name: Some("logs".to_string()),
                    },
                ))
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::Webview,
                ))
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::Stdout,
                ))
                .filter(|metadata| {
                    const NOISY_DEPENDENCY_TARGETS: [&str; 7] = [
                        "hyper",
                        "hyper_util",
                        "h2",
                        "reqwest",
                        "mdns_sd",
                        "rmcp",
                        "tracing::span",
                    ];
                    !NOISY_DEPENDENCY_TARGETS
                        .iter()
                        .any(|target| metadata.target().starts_with(target))
                        || metadata.level() <= log::Level::Info
                })
                .level(log::LevelFilter::Debug)
                .build(),
        )
        .build(tauri::generate_context!())
        .expect("Error while running Tauri application")
        .run(move |app_handle, event| match event {
            tauri::RunEvent::ExitRequested { code, api, .. } => match code {
                Some(c) => {
                    println!("Exit Request with code: {}", c);
                    log::info!("Exit Request with code: {}", c);
                    if !runtime_shutdown_started.swap(true, std::sync::atomic::Ordering::AcqRel) {
                        api.prevent_exit();
                        let runtime = app_handle
                            .state::<vortideck_core::services::flow::FlowRuntime>()
                            .inner()
                            .clone();
                        let handle = app_handle.clone();
                        let invocations = app_handle
                            .state::<Arc<vortideck_core::services::execution::ActionInvocationService>>()
                            .inner().clone();
                        let shutdown_started = runtime_shutdown_started.clone();
                        tauri::async_runtime::spawn(async move {
                            invocations.close_admission().await;
                            match runtime.stop().await {
                                Ok(()) => {
                                    invocations.wait_for_idle().await;
                                    handle.exit(c);
                                }
                                Err(error) => {
                                    log::error!("Flow runtime shutdown failed: {}", error);
                                    // Keep the process alive and allow a later explicit exit
                                    // request to retry checkpointing/joining.
                                    shutdown_started
                                        .store(false, std::sync::atomic::Ordering::Release);
                                }
                            }
                        });
                    }
                }
                _ => {
                    println!("Exit Request with code: {:?} - prevented exit", code);
                    log::info!("Exit Request with code: {:?} - prevented exit", code);
                    api.prevent_exit();
                }
            },
            _ => (),
        });

    Ok(())
}

async fn start_websocket(listener: Arc<TcpListener>, app_handle: AppHandle, ip: std::net::IpAddr) {
    loop {
        // println!("test {}", app_state.get_secret());
        match listener.accept().await {
            Ok((stream, _)) => {
                tokio::spawn(handle_connection(stream, app_handle.clone(), ip));
            }
            Err(e) => {
                eprintln!("Error accepting connection: {}", e);
                log::error!("Error accepting connection: {}", e);
                break;
            }
        }
    }
}

//? TODO: 1. Reuse Peers from db dont create new every new connection
//? TODO: 2. Find way to clear DB from PEERS which are not existing anymore, or smth.
//? TODO: 3. When RE-Paired change existing peer token
// Handle an incoming WebSocket connection
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;

/// Truncate text to at most `max_chars` characters for logging. Byte slicing
/// (`&s[..n]`) panics when `n` falls inside a multibyte character, and the
/// logged text is often client-controlled.
fn truncate_for_log(text: &str, max_chars: usize) -> &str {
    text.char_indices()
        .nth(max_chars)
        .map(|(index, _)| &text[..index])
        .unwrap_or(text)
}

async fn handle_connection(stream: TcpStream, app_handle: AppHandle, server_ip: IpAddr) {
    handle_connection_internal(Some(stream), None, app_handle, server_ip, None).await;
}

// Modified version that can handle both TcpStream and pre-upgraded Warp WebSocket
async fn handle_connection_internal(
    stream: Option<TcpStream>,
    warp_websocket: Option<warp::ws::WebSocket>,
    app_handle: AppHandle,
    server_ip: IpAddr,
    peer_ip_override: Option<String>,
) {
    let peer_ip = if let Some(override_ip) = peer_ip_override {
        log::info!("🔍 Using Warp peer IP: {}", override_ip);
        override_ip
    } else if let Some(ref stream) = stream {
        let ip = stream.peer_addr().unwrap().ip().to_string();
        log::info!("🔍 Using TCP peer IP: {}", ip);
        ip
    } else {
        log::info!("🔍 Using default localhost IP");
        "127.0.0.1".to_string() // Default to localhost for Warp connections
    };

    log::info!("🔍 Server IP: {} | Peer IP: {}", server_ip, peer_ip);
    let mut client_id = String::new();
    let mut device_type = String::new();
    let mut device_model = String::new();
    let mut provided_secret: Option<String> = None;
    let mut device_token: Option<String> = None;
    let mut pairing_token: Option<String> = None; // 🔐 E2EE: For retrieving E2EEKeys from PendingPairingsMap
    let mut mobile_public_key_b64: Option<String> = None; // 🔐 E2EE: Mobile's public key from authenticate message
    let mut is_obs_plugin = false;
    let mut obs_client_nonce = None;
    let mut obs_session = None;
    let mut is_plugin = false; // General plugin detection
    let mut integration_guid = String::new();

    // 🚀 NEW ARCHITECTURE: Use StateManager directly (with error handling)
    let state_manager = match app_handle.try_state::<Arc<StateManager>>() {
        Some(state) => state,
        None => {
            eprintln!("❌ StateManager not available for WebSocket connection - rejecting");
            return;
        }
    };

    let pool = match app_handle.try_state::<DbPool>() {
        Some(pool) => pool,
        None => {
            eprintln!("❌ DbPool not available for WebSocket connection - rejecting");
            return;
        }
    };
    let db = Arc::new(Database::new(&pool)); // Wrap in Arc for sharing

    // Get icons base path for loading icons as base64
    let icons_base_path = app_handle
        .try_state::<adapters::icons_commands::IconServiceState>()
        .map(|icon_state| icon_state.service.get_icons_base_dir());

    // Initialize button repeat map for this connection
    let button_repeat_map: ButtonRepeatMap = Arc::new(RwLock::new(HashMap::new()));

    let count = db.get_peer_count();
    println!("Total number of peers: {}", count);

    // Handle WebSocket upgrade or use pre-upgraded Warp WebSocket
    let (write, mut read): (
        Box<
            dyn futures_util::Sink<Message, Error = tokio_tungstenite::tungstenite::Error>
                + Send
                + Sync
                + Unpin,
        >,
        Box<
            dyn futures_util::Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>>
                + Send
                + Sync
                + Unpin,
        >,
    ) = if let Some(warp_ws) = warp_websocket {
        // Use pre-upgraded Warp WebSocket
        let (warp_write, warp_read) = warp_ws.split();

        // Create adapters to convert between Warp and tokio-tungstenite message types
        use futures_util::{SinkExt, StreamExt};

        let write_adapter = WarpWriteAdapter::new(warp_write);
        let read_adapter = WarpReadAdapter::new(warp_read);

        log::info!("Using pre-upgraded Warp WebSocket");
        (Box::new(write_adapter), Box::new(read_adapter))
    } else if let Some(stream) = stream {
        // Traditional TCP stream - do WebSocket upgrade
        let ws_stream = match accept_async(stream).await {
            Ok(ws) => ws,
            Err(e) => {
                eprintln!("WebSocket handshake failed: {}", e);
                log::error!("WebSocket handshake failed: {}", e);
                return;
            }
        };
        let (write, read) = ws_stream.split();
        (Box::new(write), Box::new(read))
    } else {
        eprintln!("❌ No stream or WebSocket provided");
        return;
    };
    let write = Arc::new(RwLock::new(write));

    let (tx, mut rx) = broadcast::channel::<Message>(2000); // Increased buffer size

    // Check if this is the first connection and set the server sender
    // ✅ NEW ARCHITECTURE: Using StateManager for server sender management
    if state_manager.get_server_sender().is_none() {
        state_manager.set_server_sender(tx.clone());
        println!("🔄 Set server sender via StateManager");
    }

    // Parse initial client message
    if let Some(Ok(msg)) = read.next().await {
        if let Ok(text) = msg.to_text() {
            if let Ok(json) = serde_json::from_str::<Value>(text) {
                log::debug!("Received client initialization envelope");

                client_id = json["clientId"].as_str().unwrap_or("unknown").to_string();

                // 🔐 Check if payload is encrypted
                let decrypted_json: Value = if let Some(encrypted_payload) =
                    json["encrypted_payload"].as_str()
                {
                    println!("🔐 Encrypted authenticate detected - decrypting payload...");

                    use base64::engine::general_purpose::STANDARD as Base64;
                    use base64::Engine;

                    // Get mobile public key if provided (first-time pairing)
                    mobile_public_key_b64 =
                        json["mobile_public_key"].as_str().map(|s| s.to_string());

                    // Check for pairing_token (indicates NEW pairing, should derive fresh secret)
                    let is_new_pairing =
                        json["pairing_token"].as_str().is_some() && mobile_public_key_b64.is_some();

                    let mut shared_secret_opt: Option<[u8; 32]> = None;

                    // NEW PAIRING: Derive fresh shared secret (ignore old secrets)
                    if is_new_pairing {
                        if let Some(ref mobile_pub_b64) = mobile_public_key_b64 {
                            // Mobile sent public key + pairing_token - this is NEW pairing
                            if let Some(p_token) = json["pairing_token"].as_str() {
                                println!(
                                    "🔐 NEW PAIRING detected - deriving fresh shared secret (ignoring old secrets)"
                                );

                                if let Some(pending_pairings) =
                                    app_handle.try_state::<PendingPairingsMap>()
                                {
                                    let mut pairings_map = pending_pairings.write().await;
                                    if let Some(e2ee_keys) = pairings_map.get_mut(p_token) {
                                        match Base64.decode(mobile_pub_b64) {
                                            Ok(mobile_pub_bytes)
                                                if mobile_pub_bytes.len() == 32 =>
                                            {
                                                use x25519_dalek::PublicKey;
                                                let mut key_bytes = [0u8; 32];
                                                key_bytes.copy_from_slice(&mobile_pub_bytes);
                                                let mobile_public_key = PublicKey::from(key_bytes);

                                                match e2ee_keys
                                                    .derive_shared_secret(&mobile_public_key)
                                                {
                                                    Ok(_) => {
                                                        if let Some(secret) =
                                                            e2ee_keys.shared_secret
                                                        {
                                                            println!(
                                                                "✅ Derived NEW shared secret for first-time pairing"
                                                            );
                                                            shared_secret_opt = Some(secret);

                                                            // ✅ SAVE to database immediately (replace old secret if exists)
                                                            db.save_shared_secret(
                                                                &client_id, &secret,
                                                            );
                                                            println!(
                                                                "💾 Saved NEW shared secret to database"
                                                            );

                                                            // ✅ STORE in memory (replace old secret if exists)
                                                            state_manager
                                                                .store_shared_secret(
                                                                    client_id.clone(),
                                                                    secret,
                                                                )
                                                                .await;
                                                            println!(
                                                                "💾 Stored NEW shared secret in memory"
                                                            );

                                                            // 🎉 Emit pairing success event to frontend
                                                            let _ = app_handle.emit(
                                                                "pairing-completed",
                                                                json!({
                                                                    "client_id": client_id,
                                                                    "device_type": device_type,
                                                                }),
                                                            );
                                                            println!(
                                                                "📡 Emitted pairing-completed event to frontend"
                                                            );

                                                            // 📊 Track device pairing
                                                            crate::analytics::capture(&app_handle,
                                                                crate::analytics::AnalyticsEvent::new("device_paired")
                                                                    .prop("device_type", device_type.clone())
                                                            );

                                                            // 🗑️ Remove pairing token from map to invalidate the QR code
                                                            pairings_map.remove(p_token);
                                                            println!(
                                                                "🗑️ Invalidated pairing token"
                                                            );
                                                        }
                                                    }
                                                    Err(e) => {
                                                        println!(
                                                            "❌ Failed to derive shared secret: {}",
                                                            e
                                                        );
                                                    }
                                                }
                                            }
                                            _ => {
                                                println!("❌ Invalid mobile public key format");
                                            }
                                        }
                                    } else {
                                        println!("❌ Pairing token not found in pending pairings");
                                    }
                                }
                            }
                        }
                    } else {
                        // RECONNECTION: Use existing shared secret
                        println!("🔐 RECONNECTION - loading existing shared secret");

                        // Try to get existing shared secret from memory first
                        shared_secret_opt = state_manager.get_shared_secret(&client_id).await;

                        if shared_secret_opt.is_some() {
                            println!("🔐 Found shared secret in MEMORY for client: {}", client_id);
                        }

                        // If not in memory, try loading from database
                        if shared_secret_opt.is_none() {
                            if let Some(secret_array) = db.load_shared_secret(&client_id) {
                                shared_secret_opt = Some(secret_array);

                                // Store in memory for future use
                                state_manager
                                    .store_shared_secret(client_id.clone(), secret_array)
                                    .await;
                                println!(
                                    "🔐 Loaded shared secret from DATABASE for client: {}",
                                    client_id
                                );
                            } else {
                                println!(
                                    "⚠️ No shared secret found in database for client: {}",
                                    client_id
                                );
                            }
                        }
                    }

                    // Decrypt payload if we have shared secret
                    match shared_secret_opt {
                        Some(secret) => match Base64.decode(encrypted_payload) {
                            Ok(ciphertext) => match decrypt_with_secret(&secret, &ciphertext) {
                                Ok(decrypted_bytes) => {
                                    match serde_json::from_slice::<Value>(&decrypted_bytes) {
                                        Ok(decrypted) => {
                                            println!(
                                                "✅ Successfully decrypted authenticate payload"
                                            );
                                            decrypted
                                        }
                                        Err(e) => {
                                            eprintln!("❌ Failed to parse decrypted JSON: {}", e);
                                            let _ = tx.send(Message::Close(None));
                                            return;
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("❌ Failed to decrypt payload: {}", e);
                                    let _ = tx.send(Message::Close(None));
                                    return;
                                }
                            },
                            Err(e) => {
                                eprintln!("❌ Failed to decode encrypted payload: {}", e);
                                let _ = tx.send(Message::Close(None));
                                return;
                            }
                        },
                        None => {
                            eprintln!("❌ No shared secret available for decryption");
                            let _ = tx.send(Message::Close(None));
                            return;
                        }
                    }
                } else {
                    // No encryption - use original JSON
                    json.clone()
                };

                // Extract credentials from decrypted payload (or original if not encrypted)
                device_type = decrypted_json["device"]["type"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string();

                device_model = decrypted_json["device"]["model"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string();
                provided_secret = decrypted_json["secret"].as_str().map(|s| s.to_string());
                device_token = decrypted_json["token"].as_str().map(|s| s.to_string());
                pairing_token = decrypted_json["pairing_token"]
                    .as_str()
                    .map(|s| s.to_string()); // 🔐 E2EE

                // mobile_public_key should be in plaintext part (already extracted above if encrypted)
                if mobile_public_key_b64.is_none() {
                    mobile_public_key_b64 =
                        json["mobile_public_key"].as_str().map(|s| s.to_string());
                    // 🔐 E2EE
                }

                if let Some(payload) = json["payload"].as_object() {
                    // Check if it's ANY plugin by looking for integrationIdentifier
                    if let Some(integration_identifier) = payload.get("integrationIdentifier") {
                        is_plugin = true; // Mark as any plugin

                        if integration_identifier == "vortideck_obs_plugin" {
                            is_obs_plugin = true;
                            obs_client_nonce = json["obs_client_nonce"].as_str().map(str::to_owned);
                            // Recognition is not authorization. OBS activation and
                            // capability delivery happen only after authentication.
                            device_model = "obs_plugin".to_string();
                        } else {
                            // Handle other plugins generically
                            println!("🔌 Detected plugin: {}", integration_identifier);
                            device_model = format!("{}_plugin", integration_identifier);
                        }
                    }
                }

                // Additional plugin detection methods
                if !is_plugin {
                    // Check device model for plugin indicators
                    if device_model.contains("plugin") || device_model.contains("integration") {
                        is_plugin = true;
                        println!("🔌 Detected plugin via device model: {}", device_model);
                    }

                    // Check device type for plugin indicators
                    if device_type.contains("plugin") || device_type.contains("integration") {
                        is_plugin = true;
                        println!("🔌 Detected plugin via device type: {}", device_type);
                    }
                }

                // 🚀 NEW ARCHITECTURE: Alternative OBS plugin detection
                // If we haven't detected it as OBS yet, check for OBS-specific patterns
                if !is_obs_plugin && is_plugin {
                    // Check if this might be an OBS plugin by other indicators
                    if device_model.contains("obs")
                        || device_type.contains("obs")
                        || client_id.contains("obs")
                        || device_model.contains("plugin")
                    {
                        println!("🔍 Potential OBS plugin detected via alternative indicators");
                        // We'll mark it as a potential OBS plugin and check for OBS-specific messages later
                    }
                }
            }
        }
    }

    // Authenticate client
    // Trust direct local transport; do not expose this endpoint through an unauthenticated proxy.
    // Desktop webviews use this loopback/same-process channel for simulator
    // events. It is the one intentionally unsigned client path; mobile peers
    // still require their per-device E2EE shared secret.
    let is_local_transport = peer_ip == server_ip.to_string() || peer_ip.parse::<std::net::IpAddr>().is_ok_and(|ip| ip.is_loopback());
    if is_obs_plugin && (!is_local_transport || obs_client_nonce.is_some()) {
        let Some(nonce) = obs_client_nonce.filter(|n| uuid::Uuid::parse_str(n).is_ok()) else { return; };
        let Ok(key) = overlay::companion_auth::credential(&app_handle) else { return; };
        let session = overlay::companion_auth::Session::new(key, format!("{nonce}:{}", uuid::Uuid::new_v4()));
        let challenge = json!({"type":"OBS_AUTH_CHALLENGE","session":session.id,"proof":session.proof("host")});
        if write.write().await.send(Message::Text(challenge.to_string().into())).await.is_err() { return; }
        let Ok(Some(Ok(reply))) = timeout(Duration::from_secs(10),read.next()).await else { return; };
        let Ok(reply) = serde_json::from_str::<Value>(reply.to_text().unwrap_or("")) else { return; };
        if reply["type"] != "OBS_AUTH_RESPONSE" || !session.verify_proof(reply["proof"].as_str().unwrap_or("")) { return; }
        obs_session = Some(session);
    }
    let is_trusted_local_client = obs_session.is_some() || (is_local_transport && (device_type == "host" || is_obs_plugin));
    log::info!(
        "🔒 Authentication check: peer_ip='{}' vs server_ip='{}' vs localhost='127.0.0.1'",
        peer_ip,
        server_ip
    );
    if !is_trusted_local_client {
        log::info!("🔐 Client is not a trusted host integration - requiring authentication");
        if let Some(ref token) = device_token {
            // Check if the provided device token exists in the database
            match db.get_peer_by_token(token) {
                // A device token is only valid for the client it was issued to.
                Some(existing_peer) if existing_peer.client_id != client_id => {
                    eprintln!("Device token does not belong to client: {}", client_id);
                    log::warn!("Rejected device token presented by a different client id");
                    let _ = tx.send(Message::Close(None));
                    return;
                }
                Some(existing_peer) => {
                    eprintln!("Using existing authenticated device token");
                    println!("Using existing device");
                    // Update last-seen timestamp for existing peer
                    // TODO: Implement last-seen timestamp update in database
                    println!("Existing peer reconnected: {}", token);
                }
                None => {
                    eprintln!("Invalid token for client: {}", client_id);
                    let _ = tx.send(Message::Close(None));
                    return;
                }
            }
        } else if let Some(secret) = provided_secret {
            // 🚀 NEW ARCHITECTURE: Use StateManager for authentication
            match state_manager.get_secret_key().await {
                Some(app_secret)
                    if commands::generate_qr_code::pairing_secret_matches(&secret, &app_secret) =>
                {
                    // The QR pairing secret is single-use: invalidate it before
                    // issuing the device token so it cannot pair a second device.
                    state_manager.clear_secret_key().await;
                    // Generate new token regardless of whether peer exists
                    let new_token = generate_token();
                    log::info!("Client authenticated with pairing secret via StateManager");

                    // 🔐 E2EE: Derive shared secret from mobile_public_key if provided
                    let (mobile_public_key_str, shared_secret_str): (
                        Option<String>,
                        Option<String>,
                    ) = if let Some(ref mobile_pub_b64) = mobile_public_key_b64 {
                        eprintln!("🔐 Mobile public key provided - deriving E2EE shared secret...");

                        // Get or generate desktop E2EE keys from pairing_token
                        if let Some(ref p_token) = pairing_token {
                            if let Some(pending_pairings) =
                                app_handle.try_state::<PendingPairingsMap>()
                            {
                                // Get write lock to modify E2EE keys
                                let mut pairings_map = pending_pairings.write().await;
                                if let Some(e2ee_keys) = pairings_map.get_mut(p_token) {
                                    use base64::engine::general_purpose::STANDARD as Base64;
                                    use base64::Engine;

                                    // Decode mobile public key
                                    match Base64.decode(mobile_pub_b64) {
                                        Ok(mobile_pub_bytes) if mobile_pub_bytes.len() == 32 => {
                                            let mut key_bytes = [0u8; 32];
                                            key_bytes.copy_from_slice(&mobile_pub_bytes);
                                            let mobile_public_key = PublicKey::from(key_bytes);

                                            // 🔐 Check if desktop private key has been consumed (used in prior pairing)
                                            // If so, regenerate keys for fresh re-pairing
                                            if e2ee_keys.desktop_private.is_none()
                                                && e2ee_keys.shared_secret.is_some()
                                            {
                                                eprintln!(
                                                    "🔄 Desktop private key already used - regenerating E2EE keys for re-pairing"
                                                );
                                                *e2ee_keys = E2EEKeys::generate();
                                            }

                                            // ALWAYS derive new shared secret from mobile's public key
                                            // This ensures desktop and mobile have matching secrets, even during re-pairing
                                            match e2ee_keys.derive_shared_secret(&mobile_public_key)
                                            {
                                                Ok(_) => {
                                                    if let Some(shared_secret) =
                                                        e2ee_keys.shared_secret
                                                    {
                                                        let shared_secret_b64 =
                                                            Base64.encode(shared_secret);
                                                        eprintln!(
                                                            "✅ E2EE shared secret derived successfully"
                                                        );

                                                        // 🎉 Emit pairing success event to frontend
                                                        let _ = app_handle.emit(
                                                            "pairing-completed",
                                                            json!({
                                                                "client_id": client_id,
                                                                "device_type": device_type,
                                                            }),
                                                        );
                                                        eprintln!(
                                                            "📡 Emitted pairing-completed event to frontend"
                                                        );

                                                        // 📊 Track device pairing
                                                        crate::analytics::capture(
                                                            &app_handle,
                                                            crate::analytics::AnalyticsEvent::new(
                                                                "device_paired",
                                                            )
                                                            .prop(
                                                                "device_type",
                                                                device_type.clone(),
                                                            ),
                                                        );

                                                        // 🗑️ Remove pairing token from map to invalidate the QR code
                                                        pairings_map.remove(p_token);
                                                        eprintln!("🗑️ Invalidated pairing token");

                                                        (
                                                            Some(mobile_pub_b64.clone()),
                                                            Some(shared_secret_b64),
                                                        )
                                                    } else {
                                                        eprintln!(
                                                            "⚠️ Shared secret derivation succeeded but secret is None"
                                                        );
                                                        (None, None)
                                                    }
                                                }
                                                Err(e) => {
                                                    eprintln!(
                                                        "❌ Failed to derive shared secret: {}",
                                                        e
                                                    );
                                                    (None, None)
                                                }
                                            }
                                        }
                                        Ok(_) => {
                                            eprintln!("❌ Invalid mobile public key length");
                                            (None, None)
                                        }
                                        Err(e) => {
                                            eprintln!(
                                                "❌ Failed to decode mobile public key: {}",
                                                e
                                            );
                                            (None, None)
                                        }
                                    }
                                } else {
                                    eprintln!(
                                        "⚠️ No E2EE keys found for pairing_token: {}",
                                        p_token
                                    );
                                    (None, None)
                                }
                            } else {
                                eprintln!("⚠️ PendingPairingsMap not available");
                                (None, None)
                            }
                        } else {
                            eprintln!(
                                "⚠️ Mobile public key provided but no pairing_token in message"
                            );
                            (None, None)
                        }
                    } else {
                        (None, None)
                    };

                    if db.peer_exists(&client_id) {
                        eprintln!("Updating existing peer: {}", client_id);
                        // Update existing peer with new token
                        db.update_peer_token(&client_id, &new_token);

                        // 🔐 Update E2EE keys if provided
                        if mobile_public_key_str.is_some() || shared_secret_str.is_some() {
                            if let Err(e) = db.update_peer_e2ee_keys(
                                &client_id,
                                mobile_public_key_str.as_deref(),
                                shared_secret_str.as_deref(),
                            ) {
                                eprintln!("❌ Failed to update E2EE keys: {}", e);
                            }
                        }
                    } else {
                        // Add new peer
                        let new_peer = NewPeer {
                            client_id: &client_id,
                            ip: &peer_ip,
                            device_type: &device_type,
                            device: &device_model,
                            device_token: Some(&new_token),
                            mobile_public_key: mobile_public_key_str.as_deref(),
                            shared_secret: shared_secret_str.as_deref(),
                        };

                        if let Err(e) = db.add_peer(new_peer.clone()) {
                            eprintln!("Error adding peer to database: {}", e);
                            log::error!("Error adding peer to database: {}", e);
                        } else {
                            eprintln!("New device added to the database");
                            log::info!("New device added to the database");
                            app_handle.emit_to("main", "new_peer_added", new_peer);
                        }
                    }

                    // 🔐 CRITICAL: Store shared secret in memory for relay mode
                    // After saving to database, MUST also store in StateManager's memory cache
                    // Otherwise relay message verification will fail with "No shared secret for client"
                    if let Some(ref secret_b64) = shared_secret_str {
                        use base64::engine::general_purpose::STANDARD as Base64;
                        use base64::Engine;

                        match Base64.decode(secret_b64) {
                            Ok(secret_bytes) if secret_bytes.len() == 32 => {
                                let mut secret_array = [0u8; 32];
                                secret_array.copy_from_slice(&secret_bytes);
                                state_manager
                                    .store_shared_secret(client_id.clone(), secret_array)
                                    .await;
                                eprintln!(
                                    "🔐 Stored shared secret in memory for client: {}",
                                    client_id
                                );
                            }
                            Ok(_) => {
                                eprintln!("❌ Invalid shared secret length (expected 32 bytes)");
                            }
                            Err(e) => {
                                eprintln!("❌ Failed to decode shared secret: {}", e);
                            }
                        }
                    }

                    // Always send the new token back to the client
                    // 🔐 E2EE: Check if shared secret exists - if yes, send encrypted response
                    use base64::engine::general_purpose::STANDARD as Base64;
                    use base64::Engine;

                    if let Some(ref secret_b64) = shared_secret_str {
                        // Decode shared secret for encryption
                        if let Ok(secret_bytes) = Base64.decode(secret_b64) {
                            if secret_bytes.len() == 32 {
                                let mut secret_array = [0u8; 32];
                                secret_array.copy_from_slice(&secret_bytes);

                                // Build encrypted payload
                                let mut token_payload = json!({
                                    "token": new_token
                                });

                                // Add desktop_public_key if pairing
                                if let Some(ref p_token) = pairing_token {
                                    if let Some(pending_pairings) =
                                        app_handle.try_state::<PendingPairingsMap>()
                                    {
                                        if let Some(e2ee_keys) =
                                            pending_pairings.read().await.get(p_token)
                                        {
                                            let desktop_public_key =
                                                Base64.encode(e2ee_keys.desktop_public.as_bytes());
                                            token_payload["desktop_public_key"] =
                                                json!(desktop_public_key);
                                            println!(
                                                "🔐 Including desktop_public_key in encrypted token response"
                                            );
                                        }
                                    }
                                }

                                // Encrypt the payload
                                match encrypt_with_secret(
                                    &secret_array,
                                    token_payload.to_string().as_bytes(),
                                ) {
                                    Ok(encrypted_bytes) => {
                                        let response = json!({
                                            "action": "new_token",
                                            "encrypted_payload": Base64.encode(&encrypted_bytes)
                                        });

                                        if tx
                                            .send(Message::Text(response.to_string().into()))
                                            .is_err()
                                        {
                                            eprintln!("Failed to send encrypted new_token");
                                        } else {
                                            println!("✅ Sent encrypted new_token response");
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("❌ Failed to encrypt new_token: {}", e);
                                    }
                                }
                            }
                        }
                    } else {
                        // No shared secret - send plaintext (legacy/unencrypted mode)
                        let mut token_message = json!({
                            "action": "new_token",
                            "token": new_token,
                        });

                        if let Some(ref p_token) = pairing_token {
                            // Get pending pairings map and retrieve desktop public key
                            if let Some(pending_pairings) =
                                app_handle.try_state::<PendingPairingsMap>()
                            {
                                if let Some(e2ee_keys) = pending_pairings.read().await.get(p_token)
                                {
                                    let desktop_public_key =
                                        Base64.encode(e2ee_keys.desktop_public.as_bytes());
                                    token_message["desktop_public_key"] = json!(desktop_public_key);
                                    println!(
                                        "🔐 Including desktop_public_key in token response for E2EE pairing"
                                    );
                                }
                            }
                        }

                        if tx
                            .send(Message::Text(token_message.to_string().into()))
                            .is_err()
                        {
                            eprintln!("Failed to send token to client");
                            log::error!("Failed to send token to client")
                        }
                    }
                }
                _ => {
                    eprintln!(
                        "Authentication failed for client via StateManager: {}",
                        client_id
                    );
                    log::error!(
                        "Authentication failed for client via StateManager: {}",
                        client_id
                    );

                    // 📢 Emit event to frontend for notification
                    let _ = app_handle.emit_to("main", "connection_error", serde_json::json!({
                        "type": "authentication_failed",
                        "client_id": client_id,
                        "message": "Mobile device authentication failed. They may be using an old QR code.",
                        "device_type": device_type
                    }));

                    let _ = tx.send(Message::Close(None));
                    return;
                }
            }
        } else {
            eprintln!("No token or secret provided for client: {}", client_id);
            log::warn!("No token or secret provided for client: {}", client_id);

            let _ = tx.send(Message::Close(None));
            return;
        }
    } else {
        log::info!(
            "✅ Trusted local or authenticated OBS connection: peer_ip='{}' matches server_ip='{}' or localhost",
            peer_ip,
            server_ip
        );
    }

    // OBS identity was parsed from the registration envelope above, but no OBS
    // state or capability is established until the connection crosses the
    // authentication boundary. The activation response is queued before the
    // overlay update on the same broadcast sender, preserving protocol order.
    if is_obs_plugin {
        let (updated_integration_guid, updated_client_id) =
            activate_obs_plugin(tx.clone(), state_manager.inner().clone()).await;
        integration_guid = updated_integration_guid;
        client_id = updated_client_id;
        device_model = "obs_plugin".to_string();
        state_manager.set_obs_integration_guid(integration_guid.clone());
        state_manager.set_obs_status(true);

        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        state_manager.set_obs_last_status_time(current_time).await;
        let _ = app_handle.emit("obs_plugin_status", true);
        command_handler::publish_widget_topic_availability_for_active_deck(
            &app_handle,
            "integration.obs.status",
            json!({ "connected": true }),
            true,
        )
        .await;

        let overlay_url = overlay::get_server_url(&state_manager);
        if let Err(error) = commands::overlay_commands::send_overlay_capability_to_obs(
            &tx,
            &integration_guid,
            &overlay_url,
        ) {
            log::error!("Unable to deliver refreshed overlay capability to OBS: {error}");
        } else {
            log::info!("Delivered process-scoped overlay capability after OBS activation");
        }
    }

    // Add peer to state system
    // ✅ NEW ARCHITECTURE: Using StateManager for peer management
    let peer_sender = tx.clone();
    state_manager
        .add_peer(
            client_id.clone(),
            peer_ip.clone(),
            peer_sender,
            device_type.clone(),
            device_model.clone(),
        )
        .await;
    println!("👥 Added peer {} via StateManager", client_id);

    // 🆕 AUTO-SUBSCRIBE: Automatically register mobile clients for volume updates using utility function
    if crate::utils::should_register_for_volume_updates(is_plugin, &peer_ip, &server_ip) {
        let mobile_sender = tx.clone();
        state_manager
            .register_mobile_volume_listener(client_id.clone(), mobile_sender)
            .await;

        // Send current volume state immediately
        #[cfg(target_os = "windows")]
        let current_volume = vortideck_core::SystemControl::get_volume().ok();
        #[cfg(not(target_os = "windows"))]
        let current_volume: Option<f32> = None;

        if let Some(current_volume) = current_volume {
            let volume_message = serde_json::json!({
                "action": "volume_changed",
                "volume": current_volume * 100.0,
                "muted": current_volume < 0.001,
                "payload": {
                    "is_button_change": false,
                    "button_id": null
                }
            });

            let Some(shared_secret) = state_manager.get_shared_secret(&client_id).await else {
                eprintln!(
                    "🚫 Refusing unsigned initial volume delivery to {}",
                    client_id
                );
                return;
            };
            let volume_message = match crypto::sign_json_message(&shared_secret, volume_message) {
                Ok(message) => message,
                Err(error) => {
                    eprintln!("Failed to sign initial volume message: {}", error);
                    return;
                }
            };

            if tx
                .send(Message::Text(volume_message.to_string().into()))
                .is_err()
            {
                eprintln!("Failed to send initial volume to mobile: {}", client_id);
                log::error!("Failed to send initial volume to mobile: {}", client_id);
            } else {
                println!(
                    "📱 Auto-subscribed {} to volume updates and sent current volume",
                    client_id
                );
            }
        }
    } else {
        let reason = crate::utils::get_client_exclusion_reason(is_plugin, &peer_ip, &server_ip);
        println!(
            "🏠 Skipping volume listener registration for {}: {}",
            client_id, reason
        );
    }

    // Send server info to the client
    // ✅ NEW ARCHITECTURE: Using StateManager for server info
    let client_message = if let Some(server_info) = state_manager.get_server_info() {
        println!("🌐 Retrieved server info via StateManager");

        // Get all feature limits for mobile
        let feature_limits = crate::db::utils::get_all_feature_limits();
        println!(
            "📊 Including feature limits in server_info: {:?}",
            feature_limits
        );

        json!({
            "action": "server_info",
            "server": {
                "ip": server_info.ip,
                "icons_server_port": server_info.icons_server_port,
                "port": server_info.port,
                "device_type": server_info.device_type,
                "server_name": server_info.server_name,
            },
            "feature_limits": feature_limits
        })
    } else {
        println!("⚠️ No server info available in StateManager");
        json!({
            "action": "server_info",
            "error": "Server info not available"
        })
    };

    // Debug: Check OBS status
    let current_obs_status = state_manager.get_obs_status();
    // println!("🔧 DEBUG: Current OBS status when sending server info: {}", current_obs_status);

    // Only the trusted local host webview and the local/challenge-authenticated
    // OBS companion may use the unsigned channel. A remote peer that merely
    // claims to be a plugin (client-supplied device type/model or
    // integrationIdentifier) must still hold a per-device E2EE shared secret;
    // otherwise the connection ends here, before the message loop.
    let client_message = if is_trusted_local_client {
        client_message
    } else {
        let Some(shared_secret) = state_manager.get_shared_secret(&client_id).await else {
            eprintln!("🚫 Refusing unsigned server info delivery to {}", client_id);
            return;
        };
        match crypto::sign_json_message(&shared_secret, client_message) {
            Ok(message) => message,
            Err(error) => {
                eprintln!("Failed to sign server info message: {}", error);
                return;
            }
        }
    };

    if tx
        .send(Message::Text(client_message.to_string().into()))
        .is_err()
    {
        eprintln!("Failed to send server info to client");
        log::warn!("Failed to send server info to client");
    }

    // Mobile may connect after the one-time bootstrap event, so every newly
    // authenticated client also receives the current correlated snapshot.
    if !is_trusted_local_client {
        if let Some(readiness) = app_handle
            .try_state::<Arc<vortideck_core::services::execution::ActionRuntimeReadiness>>()
        {
            let readiness_message = json!({
                "action": "action_runtime_readiness",
                "protocolVersion": 1,
                "readiness": readiness.snapshot(),
            });
            let Some(shared_secret) = state_manager.get_shared_secret(&client_id).await else {
                eprintln!(
                    "🚫 Refusing unsigned runtime readiness delivery to {}",
                    client_id
                );
                return;
            };
            match crypto::sign_json_message(&shared_secret, readiness_message) {
                Ok(message) => {
                    if tx.send(Message::Text(message.to_string().into())).is_err() {
                        log::warn!("Failed to send runtime readiness to {}", client_id);
                    }
                }
                Err(error) => {
                    log::warn!(
                        "Failed to sign runtime readiness for {}: {}",
                        client_id,
                        error
                    );
                }
            }
        }
    }

    let client_id_clone = client_id.clone();
    let widget_connection_session_id = uuid::Uuid::new_v4().to_string();

    let peer_ip_for_cleanup = peer_ip.clone();
    let client_id_clone = client_id.clone();
    let device_model_for_cleanup = device_model.clone(); // Move this outside async block
    let is_plugin_for_cleanup = is_plugin;
    let peer_ip_for_cleanup = peer_ip.clone();

    let peer_map = {};
    // Spawn a task for incoming messages
    tokio::spawn({
        let write = write.clone(); // Clone for use in the async block
        let db = db.clone();
        let app_handle = app_handle.clone(); // Clone the Arc to move into the task

        // ✅ NEW ARCHITECTURE: Using StateManager for Enigo access (with error handling)
        let enigo = match app_handle.try_state::<Arc<tokio::sync::Mutex<Enigo>>>() {
            Some(enigo) => enigo.inner().clone(),
            None => {
                eprintln!("❌ Enigo not available for WebSocket task - cannot handle messages");
                return;
            }
        };
        let enigo_arc = enigo;
        let state_manager = match app_handle.try_state::<Arc<StateManager>>() {
            Some(state) => state.inner().clone(),
            None => {
                eprintln!(
                    "❌ StateManager not available for WebSocket task - cannot handle messages"
                );
                return;
            }
        };

        // Clone values for cleanup
        let is_plugin_clone = is_plugin;
        let peer_ip_clone = peer_ip.clone();
        let server_ip_string = server_ip.to_string();
        let device_model_clone = device_model.clone();
        let is_obs_plugin_clone = is_obs_plugin;
        let device_model_for_cleanup = device_model.clone();
        let peer_ip_for_cleanup = peer_ip.clone();

        // 🚀 NEW ARCHITECTURE: Create OBS integration handler (outside async block for scope)
        let obs_integration =
            vortideck_core::integrations::builtin::obs::ObsIntegration::new(state_manager.clone());
        let obs_integration_for_cleanup = obs_integration.clone();

        // 🔐 E2EE: Helper function to sign and send messages
        let send_signed_message = {
            let tx = tx.clone();
            let state_manager = state_manager.clone();
            let client_id = client_id.clone();

            move |mut message: serde_json::Value| -> Result<(), String> {
                let tx = tx.clone();
                let state_manager = state_manager.clone();
                let client_id = client_id.clone();

                tauri::async_runtime::spawn(async move {
                    // Check if this client has a shared secret for HMAC signing
                    if let Some(shared_secret) = state_manager.get_shared_secret(&client_id).await {
                        use crypto::create_hmac_signature;
                        use uuid::Uuid;

                        // Generate nonce for replay attack prevention
                        let nonce = Uuid::new_v4().to_string();

                        // Add nonce to message
                        message["nonce"] = json!(nonce);

                        // ⭐ Use RFC 8785 canonical JSON for deterministic serialization
                        let message_str = serde_json_canonicalizer::to_string(&message)
                            .unwrap_or_else(|e| {
                                eprintln!(
                                    "❌ Canonical JSON error: {}, falling back to regular JSON",
                                    e
                                );
                                message.to_string()
                            });

                        println!(
                            "🔍 Desktop signing (canonical, direct mode): {}",
                            truncate_for_log(&message_str, 200)
                        );

                        // Create HMAC signature
                        let signature = create_hmac_signature(&shared_secret, &message_str);

                        // Add signature to message
                        message["signature"] = json!(signature);

                        println!("🔐 Signed message for client {}", client_id);
                    }
                    // Non-plugin external clients cannot reach this loop without
                    // a shared secret. Unsigned delivery remains only for the
                    // separately authenticated plugin protocol.

                    // Send message
                    if let Err(e) = tx.send(Message::Text(message.to_string().into())) {
                        eprintln!("Failed to send message: {}", e);
                    }
                });

                Ok(())
            }
        };

        let mut incoming_obs_session = obs_session.clone();
        async move {
            while let Some(Ok(msg)) = read.next().await {
                if let Ok(text) = msg.to_text() {
                    // Add debug logging for OBS plugin detection
                    // if text.contains("obs") || text.contains("OBS") || text.contains("action") {
                    //     println!("🔍 DEBUG: Received potential OBS message: {}", text);
                    // }

                    if let Ok(mut json) = serde_json::from_str::<Value>(text) {
                        if let Some(session) = incoming_obs_session.as_mut() {
                            if overlay::companion_auth::credential(&app_handle).ok().as_ref() != Some(&session.key) { let _ = tx.send(Message::Close(None)); break; }
                            let Ok(payload) = session.unwrap(&json) else { let _ = tx.send(Message::Close(None)); break; };
                            json = payload;
                        }
                        // Paired direct peers use the same versioned command identity as
                        // relay peers. Signature verification happens before one atomic
                        // durable claim; a duplicate receives the prior unresolved receipt
                        // and never reaches an effect handler again.
                        if state_manager
                            .get_shared_secret(&client_id_clone)
                            .await
                            .is_some()
                        {
                            let core_database = db.to_core_database();
                            match e2ee_handler::verify_e2ee_message(
                                json,
                                &client_id_clone,
                                &state_manager,
                                &core_database,
                            )
                            .await
                            {
                                Ok(verified) => {
                                    if let Some(receipt) = verified.disposition.receipt() {
                                        let mut receipt = receipt.clone();
                                        receipt["duplicate"] =
                                            json!(!verified.disposition.should_dispatch());
                                        let _ = send_signed_message(receipt);
                                    }
                                    if !verified.disposition.should_dispatch() {
                                        log::info!(
                                            "Returned prior direct command receipt without redispatch for {}",
                                            client_id_clone
                                        );
                                        continue;
                                    }
                                    json = verified.payload;
                                }
                                Err(error) => {
                                    log::warn!(
                                        "Rejected authenticated message from {}: {}",
                                        client_id_clone,
                                        error
                                    );
                                    let _ = app_handle.emit_to(
                                        "main",
                                        "connection_error",
                                        serde_json::json!({
                                            "type": "signature_verification_failed",
                                            "client_id": client_id_clone,
                                            "message": "The signed message was invalid, expired, already used, or could not be accepted durably."
                                        }),
                                    );
                                    continue;
                                }
                            }
                        }
                        // Non-plugin external clients without a shared secret are
                        // rejected before this loop. Plugins use their separate
                        // authenticated protocol and are not mobile E2EE peers.

                        // 🚀 NEW ARCHITECTURE: Use centralized OBS integration for all OBS-related messages
                        // Note: OBS sender should already be set up when the OBS plugin initially connected.
                        // We don't set it here to avoid overriding the actual OBS plugin sender with other clients.

                        // Create platform-specific event emitter function
                        let emit_event = |window: &str,
                                          event: &str,
                                          payload: serde_json::Value|
                         -> Result<
                            (),
                            Box<dyn std::error::Error + Send + Sync>,
                        > {
                            app_handle.emit_to(window, event, payload).map_err(|e| {
                                Box::new(e) as Box<dyn std::error::Error + Send + Sync>
                            })
                        };

                        // 🎯 NEW: Handle banner-specific messages first
                        if is_obs_plugin && obs_integration
                            .handle_banner_message(&json, &tx, &emit_event)
                            .await
                        {
                            // Banner message was handled by the integration
                            continue;
                        }

                        // Handle general OBS messages
                        if is_obs_plugin && obs_integration
                            .handle_obs_message(&json, &tx, &emit_event)
                            .await
                        {
                            // OBS message was handled by the integration
                            continue;
                        }

                        // 📏 Handle OBS canvas size messages for overlay system
                        if let Some(message_type) = json.get("type").and_then(|t| t.as_str()) {
                            match message_type {
                                "OBS_CANVAS_SIZE_UPDATE" => {
                                    if !is_obs_plugin {
                                        log::warn!(
                                            "Rejected OBS canvas update from non-OBS client {}",
                                            client_id_clone
                                        );
                                        continue;
                                    }
                                    println!("📏 Received OBS canvas size update from plugin");

                                    if let Err(e) = crate::commands::overlay_commands::handle_obs_canvas_size_update(
                                        &json,
                                        &app_handle
                                    ).await {
                                        println!("❌ Failed to handle canvas size update: {}", e);
                                    }
                                    continue;
                                }
                                "OBS_CANVAS_APPLY_RESULT" => {
                                    if !is_obs_plugin {
                                        log::warn!(
                                            "Rejected OBS canvas Apply result from non-OBS client {}",
                                            client_id_clone
                                        );
                                        continue;
                                    }
                                    if let Err(error) =
                                        app_handle.emit("obs-canvas-apply-result", json.clone())
                                    {
                                        log::warn!(
                                            "Unable to forward OBS canvas Apply result: {error}"
                                        );
                                    }
                                    continue;
                                }
                                "GET_OBS_CANVAS_SIZE" => {
                                    println!("🔄 Received canvas size request from VortiDeck");
                                    // This is a request from VortiDeck to OBS plugin, forward it
                                    // (This should be handled by the OBS plugin, not here)
                                    continue;
                                }
                                _ => {
                                    // Not an overlay-related message, continue with other handlers
                                }
                            }
                        }

                        // Handle other plugin messages
                        if is_plugin {
                            // Handle other plugin messages
                            // ... existing plugin handling code ...
                        } else {
                            // Handle non-plugin messages (Deck app, etc.)
                            // println!("Received from {}: {}", client_id_clone, text);
                            // Auth and pairing messages may carry credentials. Log only
                            // connection metadata, never the raw protocol envelope.
                            log::debug!("Received authenticated message from {}", client_id_clone);

                            // Handle logic specific to Deck app here
                            // eprintln!("Handling message from Deck app...");

                            if let Some(action_str) = json["action"].as_str() {
                                match action_str {
                                    // 🔐 E2EE Handshake - Mobile sends its public key to complete pairing
                                    "e2ee_handshake" => {
                                        println!("🔐 Received E2EE handshake from mobile");

                                        // Extract mobile's public key and pairing token from handshake
                                        let mobile_public_key_b64 =
                                            json["mobile_public_key"].as_str();
                                        let pairing_token_from_msg = json["pairing_token"].as_str();

                                        if let (Some(mobile_pub_b64), Some(token)) =
                                            (mobile_public_key_b64, pairing_token_from_msg)
                                        {
                                            use base64::engine::general_purpose::STANDARD as Base64;
                                            use base64::Engine;
                                            use x25519_dalek::PublicKey;

                                            // Decode mobile's public key
                                            match Base64.decode(mobile_pub_b64) {
                                                Ok(mobile_pub_bytes)
                                                    if mobile_pub_bytes.len() == 32 =>
                                                {
                                                    // Get pending pairings map
                                                    if let Some(pending_pairings) =
                                                        app_handle.try_state::<PendingPairingsMap>()
                                                    {
                                                        // Remove the E2EEKeys (consumes the private key)
                                                        if let Some(mut e2ee_keys) =
                                                            pending_pairings
                                                                .write()
                                                                .await
                                                                .remove(token)
                                                        {
                                                            // Convert bytes to PublicKey
                                                            let mut key_bytes = [0u8; 32];
                                                            key_bytes
                                                                .copy_from_slice(&mobile_pub_bytes);
                                                            let mobile_public_key =
                                                                PublicKey::from(key_bytes);

                                                            // Derive shared secret
                                                            match e2ee_keys.derive_shared_secret(
                                                                &mobile_public_key,
                                                            ) {
                                                                Ok(_) => {
                                                                    let shared_secret = e2ee_keys
                                                                        .shared_secret
                                                                        .unwrap();

                                                                    // Store shared secret in StateManager (memory)
                                                                    state_manager
                                                                        .store_shared_secret(
                                                                            client_id_clone.clone(),
                                                                            shared_secret,
                                                                        )
                                                                        .await;

                                                                    // 🔐 PHASE 2: Persist shared secret to database
                                                                    if let Err(e) = db
                                                                        .save_shared_secret(
                                                                            &client_id_clone,
                                                                            &shared_secret,
                                                                        )
                                                                    {
                                                                        eprintln!(
                                                                            "⚠️ Failed to save shared secret to database: {}",
                                                                            e
                                                                        );
                                                                    } else {
                                                                        println!(
                                                                            "💾 Shared secret persisted to database"
                                                                        );
                                                                    }

                                                                    println!(
                                                                        "✅ E2EE handshake complete for client: {}",
                                                                        client_id_clone
                                                                    );

                                                                    // Send handshake confirmation
                                                                    let confirmation = serde_json::json!({
                                                                        "action": "e2ee_handshake_complete",
                                                                        "success": true
                                                                    });

                                                                    if let Err(e) =
                                                                        tx.send(Message::Text(
                                                                            confirmation
                                                                                .to_string()
                                                                                .into(),
                                                                        ))
                                                                    {
                                                                        eprintln!(
                                                                            "Failed to send handshake confirmation: {}",
                                                                            e
                                                                        );
                                                                    }
                                                                }
                                                                Err(e) => {
                                                                    eprintln!(
                                                                        "❌ Failed to derive shared secret: {}",
                                                                        e
                                                                    );

                                                                    let error_response = serde_json::json!({
                                                                        "action": "e2ee_handshake_complete",
                                                                        "success": false,
                                                                        "error": "Failed to derive shared secret"
                                                                    });

                                                                    let _ = tx.send(Message::Text(
                                                                        error_response
                                                                            .to_string()
                                                                            .into(),
                                                                    ));
                                                                }
                                                            }
                                                        } else {
                                                            eprintln!(
                                                                "❌ No pending pairing found for token: {}",
                                                                token
                                                            );

                                                            let error_response = serde_json::json!({
                                                                "action": "e2ee_handshake_complete",
                                                                "success": false,
                                                                "error": "Invalid or expired pairing token"
                                                            });

                                                            let _ = tx.send(Message::Text(
                                                                error_response.to_string().into(),
                                                            ));
                                                        }
                                                    } else {
                                                        eprintln!(
                                                            "❌ PendingPairingsMap not available"
                                                        );
                                                    }
                                                }
                                                Ok(_) => {
                                                    eprintln!(
                                                        "❌ Invalid mobile public key length"
                                                    );
                                                }
                                                Err(e) => {
                                                    eprintln!(
                                                        "❌ Failed to decode mobile public key: {}",
                                                        e
                                                    );
                                                }
                                            }
                                        } else {
                                            eprintln!(
                                                "❌ Missing mobile_public_key or pairing_token in handshake"
                                            );
                                        }
                                    }
                                    "ping" => {
                                        println!("Received ping, sending pong");
                                        log::info!("Received ping, sending pong");

                                        // Create a pong response
                                        let pong_response = serde_json::json!({
                                            "action": "pong",
                                        });

                                        // Send the pong response back to the client
                                        if let Err(e) =
                                            tx.send(Message::Text(pong_response.to_string().into()))
                                        {
                                            eprintln!("Failed to send pong response: {}", e);
                                            log::error!("Failed to send pong response: {}", e);
                                        }
                                    }
                                    "subscribe_volume_updates" => {
                                        println!(
                                            "Mobile client requested volume updates (already auto-subscribed)"
                                        );
                                        log::info!(
                                            "Mobile client requested volume updates (already auto-subscribed)"
                                        );

                                        // 🔄 REMOVED DUPLICATE: Client is already auto-registered on connection
                                        // Just send current volume state as confirmation
                                        #[cfg(target_os = "windows")]
                                        let current_volume_result =
                                            vortideck_core::SystemControl::get_volume();
                                        #[cfg(not(target_os = "windows"))]
                                        let current_volume_result: Result<f32, _> = Err("Not supported on this platform");

                                        if let Ok(current_volume) = current_volume_result {
                                            let volume_message = serde_json::json!({
                                                "action": "volume_changed",
                                                "volume": current_volume * 100.0,
                                                "muted": current_volume < 0.001,
                                                "payload": {
                                                    "is_button_change": false,
                                                    "button_id": null
                                                }
                                            });

                                            if let Err(e) = tx.send(Message::Text(
                                                volume_message.to_string().into(),
                                            )) {
                                                eprintln!(
                                                    "Failed to send current volume to mobile: {}",
                                                    e
                                                );
                                                log::error!(
                                                    "Failed to send current volume to mobile: {}",
                                                    e
                                                );
                                            }
                                        }
                                    }
                                    "test_obs" => {
                                        use vortideck_core::db::models::SavedActionBinding as ObsActionModel;
                                        println!("Testing Obs");
                                        log::info!("Testing Obs");

                                        let empty_map: HashMap<String, String> = HashMap::new();
                                        let json_value = serde_json::to_value(empty_map).unwrap(); // Convert HashMap to JsonValue

                                        // ✅ NEW ARCHITECTURE: Using StateManager for OBS plugin
                                        if let Some(obs_plugin_tx) =
                                            state_manager.get_obs_plugin_sender().await
                                        {
                                            let obs_action = ObsActionModel {
                                                id: 1,
                                                stable_id: "builtin:test-obs-toggle-desktop-mute"
                                                    .to_string(),
                                                integration_id: "obs".to_string(),
                                                name: "Toggle Desktop Mute".to_string(),
                                                action: "DesktopMuteToggle".to_string(),
                                                params: "{}".to_string(),
                                                capability_version: 1,
                                                connection_selector: None,
                                                presentation: "{}".to_string(),
                                            };

                                            // Convert Arc<dyn MessageSender> to broadcast::Sender<Message>
                                            // For now, we'll skip this call since the types don't match
                                            // This will be properly implemented when we update execute_obs_plugin_action
                                            println!("OBS plugin action would be executed here");
                                            log::info!("OBS plugin action would be executed here");
                                        } else {
                                            println!("No OBS plugin sender available");
                                            log::warn!("No OBS plugin sender available");
                                        }
                                    }
                                    "debug_set_obs_true" => {
                                        println!("🔧 DEBUG: Manually setting OBS status to true");
                                        state_manager.set_obs_status(true);
                                        app_handle.emit("obs_plugin_status", true);

                                        // Set the last status time to prevent check_obs_status from resetting it
                                        use std::time::{SystemTime, UNIX_EPOCH};
                                        let current_time = SystemTime::now()
                                            .duration_since(UNIX_EPOCH)
                                            .unwrap()
                                            .as_secs();
                                        state_manager.set_obs_last_status_time(current_time).await;

                                        println!("✅ DEBUG: OBS status set to true");
                                    }
                                    "command" => {
                                        // 🔄 REFACTORED: Use shared command handler (works for both direct and relay connections)
                                        let ctx = command_handler::CommandExecutionContext {
                                            json: json.clone(),
                                            obs_integration: Arc::new(obs_integration.clone()),
                                            db: (*db).clone(), // Dereference Arc<Database> to get Database
                                            tx: tx.clone(),
                                            enigo: enigo_arc.clone(),
                                            app_handle: app_handle.clone(),
                                        };
                                        command_handler::handle_command_action(ctx).await;
                                    }
                                    "deck_widget_intent" => {
                                        let ctx = command_handler::CommandExecutionContext {
                                            json: json.clone(),
                                            obs_integration: Arc::new(obs_integration.clone()),
                                            db: (*db).clone(),
                                            tx: tx.clone(),
                                            enigo: enigo_arc.clone(),
                                            app_handle: app_handle.clone(),
                                        };
                                        command_handler::handle_widget_intent_action(
                                            ctx,
                                            command_handler::WidgetIntentSource {
                                                client_id: client_id_clone.clone(),
                                                connection_session_id: widget_connection_session_id
                                                    .clone(),
                                            },
                                        )
                                        .await;
                                    }
                                    "deck_widget_cache_inventory" => {
                                        let ctx = command_handler::CommandExecutionContext {
                                            json: json.clone(),
                                            obs_integration: Arc::new(obs_integration.clone()),
                                            db: (*db).clone(),
                                            tx: tx.clone(),
                                            enigo: enigo_arc.clone(),
                                            app_handle: app_handle.clone(),
                                        };
                                        command_handler::handle_widget_cache_inventory(ctx).await;
                                    }
                                    "workflow_approval_decision" => {
                                        let response = match command_handler::handle_workflow_approval_decision(
                                            &app_handle,
                                            &json,
                                            &client_id_clone,
                                            "mobile_direct",
                                        )
                                        .await
                                        {
                                            Ok(outcome) => json!({
                                                "action": "workflow_approval_decision_ack",
                                                "wakeId": json.get("wakeId"),
                                                "outcome": outcome,
                                            }),
                                            Err(error) => json!({
                                                "action": "workflow_approval_decision_ack",
                                                "wakeId": json.get("wakeId"),
                                                "error": error,
                                            }),
                                        };
                                        let _ = send_signed_message(response);
                                    }
                                    "list_workflow_approvals" => {
                                        if let Ok(snapshot) =
                                            command_handler::workflow_approvals_snapshot(
                                                &app_handle,
                                                &client_id_clone,
                                                "mobile_direct",
                                            )
                                            .await
                                        {
                                            let _ = send_signed_message(snapshot);
                                        }
                                    }
                                    "start_button_repeat" => {
                                        if let Some(button_id) =
                                            json.get("buttonId").and_then(|v| v.as_i64())
                                        {
                                            let button_id = button_id as i32;
                                            println!(
                                                "🔄 Starting repeat for button: {}",
                                                button_id
                                            );

                                            // Clone necessary variables for the async task
                                            let db_clone = db.clone();
                                            let tx_clone = tx.clone();
                                            let button_repeat_map_clone = button_repeat_map.clone();
                                            let app_handle_clone = app_handle.clone();

                                            tokio::spawn(async move {
                                                start_button_repeat(
                                                    button_id,
                                                    db_clone,
                                                    tx_clone,
                                                    button_repeat_map_clone,
                                                    app_handle_clone,
                                                )
                                                .await;
                                            });
                                        }
                                    }
                                    "stop_button_repeat" => {
                                        if let Some(button_id) =
                                            json.get("buttonId").and_then(|v| v.as_i64())
                                        {
                                            let button_id = button_id as i32;
                                            println!(
                                                "⏹️ Stopping repeat for button: {}",
                                                button_id
                                            );

                                            let button_repeat_map_clone = button_repeat_map.clone();
                                            tokio::spawn(async move {
                                                stop_button_repeat(
                                                    button_id,
                                                    button_repeat_map_clone,
                                                )
                                                .await;
                                            });
                                        }
                                    }
                                    "get_current_deck" => {
                                        println!("Fetching current deck...");
                                        log::info!("Fetching current deck");
                                        handle_get_current_deck(
                                            write.clone(),
                                            db.clone(),
                                            icons_base_path.clone(),
                                            state_manager.get_shared_secret(&client_id_clone).await,
                                        )
                                        .await;
                                    }
                                    "get_all_decks" => {
                                        log::info!("Fetching all decks");
                                        handle_get_all_decks(
                                            write.clone(),
                                            db.clone(),
                                            icons_base_path.clone(),
                                            state_manager.get_shared_secret(&client_id_clone).await,
                                        )
                                        .await;
                                    }
                                    // ❌ REMOVED: Individual feature limit fetching - limits now sent in server_info
                                    // "get_feature_limit" is deprecated, mobile gets all limits on connection
                                    "update_active_deck" => {
                                        if let Some(deck_id) = json["payload"]
                                            .get("deck_id")
                                            .and_then(|id| id.as_u64())
                                        {
                                            let pool = match app_handle.try_state::<DbPool>() {
                                                Some(pool) => pool,
                                                None => {
                                                    println!(
                                                        "❌ DbPool not available for deck update"
                                                    );
                                                    continue;
                                                }
                                            };
                                            handle_update_active_deck(
                                                pool.clone(),
                                                serde_json::json!({"deck_id": deck_id}),
                                            )
                                            .await;
                                        } else {
                                            log::error!(
                                                "Invalid payload: missing or incorrect deck_id"
                                            );
                                        }
                                    }
                                    _ => {
                                        println!("Unknown action: {}", action_str);
                                        log::error!("Unknown action: {}", action_str);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            let _ = tx.send(Message::Close(None));
            // Client transport closed; release the outgoing loop as well.
            println!("Client disconnected unexpectedly: {}", client_id_clone);
            log::info!("Client disconnected unexpectedly: {}", client_id_clone);

            // ✅ NEW ARCHITECTURE: Using StateManager for peer removal
            state_manager.remove_peer(&client_id_clone).await;
            command_handler::cancel_widget_repeats_for_source(
                &command_handler::WidgetIntentSource {
                    client_id: client_id_clone.clone(),
                    connection_session_id: widget_connection_session_id.clone(),
                },
            )
            .await;

            // 🚀 NEW ARCHITECTURE: Use centralized OBS integration for disconnection
            if device_model_for_cleanup == "obs_plugin" {
                let emit_event =
                    |window: &str,
                     event: &str,
                     payload: serde_json::Value|
                     -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                        app_handle
                            .emit_to(window, event, payload)
                            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
                    };
                obs_integration_for_cleanup
                    .handle_disconnection(&emit_event)
                    .await;
            }

            // 🧹 CLEANUP: Remove from mobile volume listeners when client disconnects using utility function
            if let Ok(server_ip_addr) = server_ip_string.parse::<std::net::IpAddr>() {
                if crate::utils::should_register_for_volume_updates(
                    is_plugin_clone,
                    &peer_ip_clone,
                    &server_ip_addr,
                ) {
                    state_manager
                        .remove_mobile_volume_listener(&client_id_clone)
                        .await;
                    println!("📱 Removed mobile volume listener: {}", client_id_clone);
                }
            }
        }
    });

    // Handle outgoing messages
    while let Ok(mut msg) = rx.recv().await {
        if let Some(session) = obs_session.as_mut() {
            if overlay::companion_auth::credential(&app_handle).ok().as_ref() != Some(&session.key) { break; }
            if let Message::Text(body) = &msg { msg = Message::Text(session.wrap(body).into()); }
        }
        let mut write_guard = write.write().await; // Lock the mutex
        let closing = matches!(msg, Message::Close(_));
        if write_guard.send(msg).await.is_err() || closing {
            break;
        }
    }

    println!("Client disconnected: {}", client_id);
    log::info!("Client disconnected: {}", client_id);
    command_handler::cancel_all_widget_repeats().await;

    // 🚀 NEW ARCHITECTURE: Check if disconnected client was OBS plugin and set status to false
    if device_model_for_cleanup == "obs_plugin" {
        println!("[OBS] OBS plugin disconnected, setting status to false");
        state_manager
            .disconnect_obs_companion("OBS companion WebSocket disconnected")
            .await;
        app_handle.emit("obs_plugin_status", false);
        command_handler::publish_widget_topic_availability_for_active_deck(
            &app_handle,
            "integration.obs.status",
            json!({ "connected": false }),
            false,
        )
        .await;
    }

    // 🧹 CLEANUP: Remove from mobile volume listeners when client disconnects normally using utility function
    if crate::utils::should_register_for_volume_updates(
        is_plugin_for_cleanup,
        &peer_ip_for_cleanup,
        &server_ip,
    ) {
        state_manager.remove_peer(&client_id).await;
        state_manager
            .remove_mobile_volume_listener(&client_id)
            .await;
        println!("📱 Cleaned up mobile volume listener: {}", client_id);
    } else {
        // Still remove from peer list regardless
        state_manager.remove_peer(&client_id).await;
    }
}

// Context menu execution — looks up item from config and executes via UniversalExecutionEngine
// Follows the same pattern as execute_button_action: parse Action → convert → execute
fn execute_context_menu_item(app_handle: &AppHandle, item_id: &str, paths: &[String]) {
    use tauri::Manager;
    log::info!(
        "[context-menu] execute: item_id={}, paths={:?}",
        item_id,
        paths
    );

    let config = match crate::commands::context_menu_commands::read_config() {
        Ok(c) => c,
        Err(e) => {
            log::error!("[context-menu] Failed to read config: {}", e);
            return;
        }
    };
    log::debug!("[context-menu] Config loaded, {} items", config.items.len());

    let item = match config.items.iter().find(|i| i.id == item_id && i.enabled) {
        Some(i) => i,
        None => {
            log::warn!("[context-menu] Item not found or disabled: {}", item_id);
            return;
        }
    };

    let action_str = item.action.clone();
    let item_title = item.title.clone();
    let item_id = item_id.to_string();
    let paths = paths.to_vec();

    // Focus main window
    if let Some(window) = app_handle.get_webview_window("main") {
        let _ = window.set_focus();
    }

    // Page navigation — emit to frontend
    if action_str.starts_with("open-page-") {
        let page = action_str.strip_prefix("open-page-").unwrap_or("");
        if let Some(window) = app_handle.get_webview_window("main") {
            let _ = window.emit("navigate-to-page", page);
        }
        return;
    }

    log::debug!(
        "[context-menu] item found: title={}, action={}",
        item_title,
        truncate_for_log(&action_str, 120)
    );

    // Parse as Action enum JSON and execute through the canonical invocation boundary.
    let action = match serde_json::from_str::<vortideck_core::db::models::Action>(&action_str) {
        Ok(a) => a,
        Err(e) => {
            log::error!(
                "[context-menu] Failed to parse action JSON: {} | json: {}",
                e,
                truncate_for_log(&action_str, 200)
            );
            return;
        }
    };

    let invocation_service = match app_handle
        .try_state::<std::sync::Arc<vortideck_core::services::execution::ActionInvocationService>>()
    {
        Some(service) => service.inner().clone(),
        None => {
            log::error!("[context-menu] ActionInvocationService state not found");
            return;
        }
    };

    let app_handle = app_handle.clone();
    log::info!("[context-menu] spawning executor for item={}", item_id);
    tauri::async_runtime::spawn(async move {
        let universal_action = match invocation_service
            .resolve_database_action(&action, None, None)
            .await
        {
            Ok(ua) => ua,
            Err(e) => {
                log::error!(
                    "[context-menu] convert_action failed for {}: {}",
                    item_id,
                    e
                );
                return;
            }
        };
        log::debug!("[context-menu] action converted, executing...");

        // Auto-append file path(s) to Program actions from context menu.
        // User just writes "uv run script.py" and we add the right-clicked
        // file path automatically. If the command already uses {context_menu_*}
        // placeholders, skip auto-append — the user controls placement explicitly.
        let universal_action = match universal_action {
            vortideck_core::services::execution::UniversalAction::Program {
                executable,
                args,
                working_dir,
                env_vars,
                timeout_ms,
                program_type,
                show_terminal,
            } if !paths.is_empty() => {
                let has_placeholder = executable.contains("{context_menu_")
                    || args.iter().any(|a| a.contains("{context_menu_"));
                let new_args = if has_placeholder {
                    log::debug!("[context-menu] command uses placeholders, skipping auto-append");
                    args
                } else {
                    let mut a = args;
                    for p in &paths {
                        // Push raw path — quoting is handled by the execution engine
                        a.push(p.clone());
                    }
                    log::debug!(
                        "[context-menu] auto-appended {} path(s) to args",
                        paths.len()
                    );
                    a
                };
                vortideck_core::services::execution::UniversalAction::Program {
                    executable,
                    args: new_args,
                    working_dir,
                    env_vars,
                    timeout_ms,
                    program_type,
                    show_terminal,
                }
            }
            other => other,
        };

        // Inject file paths into execution context variables (still available for advanced use)
        let mut variables = std::collections::HashMap::new();
        if !paths.is_empty() {
            let first = &paths[0];
            let path_obj = std::path::Path::new(first.as_str());

            variables.insert(
                "context_menu_path".to_string(),
                serde_json::Value::String(first.clone()),
            );
            variables.insert(
                "context_menu_paths".to_string(),
                serde_json::Value::String(paths.join("|")),
            );
            variables.insert(
                "context_menu_file_count".to_string(),
                serde_json::Value::Number(serde_json::Number::from(paths.len())),
            );

            // Derived path components for use in program args and working_dir
            let name = path_obj
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            let name_no_ext = path_obj
                .file_stem()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            let parent = path_obj
                .parent()
                .and_then(|p| p.to_str())
                .unwrap_or("")
                .to_string();
            let extension = path_obj
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| format!(".{}", e))
                .unwrap_or_default();

            variables.insert(
                "context_menu_name".to_string(),
                serde_json::Value::String(name),
            );
            variables.insert(
                "context_menu_name_no_ext".to_string(),
                serde_json::Value::String(name_no_ext),
            );
            variables.insert(
                "context_menu_parent".to_string(),
                serde_json::Value::String(parent),
            );
            variables.insert(
                "context_menu_extension".to_string(),
                serde_json::Value::String(extension),
            );

            // Also provide as JSON array for flows
            if let Ok(json_paths) = serde_json::to_value(&paths) {
                variables.insert("context_menu_paths_json".to_string(), json_paths);
            }
        }

        let context = vortideck_core::services::execution::ExecutionContext {
            trigger_source: vortideck_core::services::execution::TriggerSource::ContextMenu {
                item_id: item_id.clone(),
            },
            user_id: None,
            session_id: "context-menu".to_string(),
            variables,
            timeout_ms: Some(30000),
            retry_config: vortideck_core::services::execution::RetryConfig {
                max_retries: 0,
                retry_delay_ms: 0,
                exponential_backoff: false,
            },
            execution_mode: vortideck_core::services::execution::ButtonExecutionMode::RunOnce,
            execution_config: None,
        };

        let result = invocation_service
            .invoke_resolved_action(&universal_action, &context)
            .await;

        let (success, notification_message) = if result.succeeded() {
            log::info!("Context menu action executed for item {}", item_id);
            let msg = result
                .outputs
                .get("stdout")
                .and_then(|v| v.as_str())
                .map(|s| s.trim())
                .filter(|s| !s.is_empty() && !s.contains("not captured"))
                .map(|s| s.to_string())
                .or_else(|| {
                    result
                        .outputs
                        .get("message")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                })
                .unwrap_or_else(|| "Completed successfully".to_string());
            (true, msg)
        } else {
            let err = result
                .error
                .as_ref()
                .map(|e| e.message.clone())
                .unwrap_or_else(|| "Unknown error".to_string());
            log::error!("Context menu action failed for item {}: {}", item_id, err);
            (false, err)
        };

        if let Some(window) = app_handle.get_webview_window("main") {
            let _ = window.emit(
                "context-menu-result",
                serde_json::json!({
                    "success": success,
                    "title": item_title,
                    "message": notification_message,
                }),
            );
        }
    });
}

// Function to execute a command
use vortideck_core::db::Action;
// Legacy import commented out during migration
// use crate::macros::SerializedMacroStep;

use device_query::{DeviceQuery, DeviceState, Keycode};
use enigo::{
    Button, Coordinate, Direction,
    Direction::{Click, Press, Release},
    Key, Keyboard, Mouse,
};

// 🚀 UNIVERSAL ARCHITECTURE: New universal execution function
mod action_execution;
use action_execution::{
    build_deck_widget_snapshot, execute_action_universal, handle_get_all_decks,
    handle_get_current_deck, handle_update_active_deck,
};
mod device_execution;
use device_execution::{activate_obs_plugin, start_button_repeat, stop_button_repeat};
mod local_server;
use local_server::start_warp_server;
mod local_websockets;
use local_websockets::{
    handle_banner_websocket, handle_overlay_websocket, handle_websocket_connection,
};
mod warp_adapters;
use warp_adapters::{WarpReadAdapter, WarpWriteAdapter};
