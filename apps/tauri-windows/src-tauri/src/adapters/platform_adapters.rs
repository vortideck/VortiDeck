// 🚀 TAURI PLATFORM ADAPTERS: Real implementations for Tauri platform
// This file provides concrete implementations of the platform abstractions for Tauri

use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_store::StoreExt;
use tokio::sync::RwLock;
use vortideck_core::core::abstractions::*;

// ============================================================================
// REMOVED: TauriEventEmitter
// Events are now handled by the EventBus (platform-agnostic).
// Tauri-specific IPC forwarding is done in main.rs via spawn_event_bridge().
// ============================================================================

/// Tauri implementation of UniversalStorageProvider using Tauri's plugin-store
pub struct TauriStorageProvider {
    app_handle: AppHandle,
}

impl TauriStorageProvider {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }
}

#[async_trait]
impl UniversalStorageProvider for TauriStorageProvider {
    async fn set(&self, key: &str, value: &Value) -> Result<(), PlatformError> {
        let app_data_dir = self
            .app_handle
            .path()
            .app_data_dir()
            .map_err(|e| PlatformError::StorageError(e.to_string()))?;

        // Create directories if they don't exist
        if !app_data_dir.exists() {
            std::fs::create_dir_all(&app_data_dir)
                .map_err(|e| PlatformError::StorageError(e.to_string()))?;
        }

        // Use Tauri's store plugin (simplified - store methods don't return Results)
        let store = self
            .app_handle
            .store("vortideck-store.json")
            .map_err(|e| PlatformError::StorageError(e.to_string()))?;

        store.set(key, value.clone());

        // Save is usually automatic or may not return Result
        if let Err(e) = store.save() {
            return Err(PlatformError::StorageError(e.to_string()));
        }

        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Option<Value>, PlatformError> {
        let store = self
            .app_handle
            .store("vortideck-store.json")
            .map_err(|e| PlatformError::StorageError(e.to_string()))?;

        let value = store.get(key);

        Ok(value)
    }

    async fn delete(&self, key: &str) -> Result<(), PlatformError> {
        let store = self
            .app_handle
            .store("vortideck-store.json")
            .map_err(|e| PlatformError::StorageError(e.to_string()))?;

        store.delete(key);

        if let Err(e) = store.save() {
            return Err(PlatformError::StorageError(e.to_string()));
        }

        Ok(())
    }

    async fn list_keys(&self, prefix: Option<&str>) -> Result<Vec<String>, PlatformError> {
        let store = self
            .app_handle
            .store("vortideck-store.json")
            .map_err(|e| PlatformError::StorageError(e.to_string()))?;

        let keys = store.keys();

        let filtered_keys: Vec<String> = if let Some(prefix) = prefix {
            keys.into_iter()
                .filter(|key| key.starts_with(prefix))
                .collect()
        } else {
            keys
        };

        Ok(filtered_keys)
    }

    async fn set_binary(&self, key: &str, data: &[u8]) -> Result<(), PlatformError> {
        let encoded = base64::encode(data);
        let value = Value::String(encoded);
        self.set(key, &value).await
    }

    async fn get_binary(&self, key: &str) -> Result<Option<Vec<u8>>, PlatformError> {
        if let Some(Value::String(encoded)) = self.get(key).await? {
            let decoded =
                base64::decode(&encoded).map_err(|e| PlatformError::StorageError(e.to_string()))?;
            Ok(Some(decoded))
        } else {
            Ok(None)
        }
    }

    fn storage_type(&self) -> &'static str {
        "TauriStorageProvider"
    }

    fn get_app_data_path(&self, subpath: &str) -> Result<String, PlatformError> {
        let app_data_dir = self
            .app_handle
            .path()
            .app_data_dir()
            .map_err(|e| PlatformError::StorageError(e.to_string()))?;

        let full_path = app_data_dir.join(subpath);

        // Create directories if they don't exist
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| PlatformError::StorageError(e.to_string()))?;
        }

        Ok(full_path.to_string_lossy().to_string())
    }
}

/// Tauri implementation of UniversalHttpClient using Tauri's HTTP plugin
pub struct TauriHttpClient {
    app_handle: AppHandle,
}

impl TauriHttpClient {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }
}

#[async_trait]
impl UniversalHttpClient for TauriHttpClient {
    async fn get(
        &self,
        url: &str,
        headers: Option<HashMap<String, String>>,
    ) -> Result<Value, PlatformError> {
        let client = reqwest::Client::new();
        let mut request = client.get(url);

        if let Some(headers) = headers {
            for (key, value) in headers {
                request = request.header(&key, &value);
            }
        }

        let response = request
            .send()
            .await
            .map_err(|e| PlatformError::NetworkError(e.to_string()))?;

        let text = response
            .text()
            .await
            .map_err(|e| PlatformError::NetworkError(e.to_string()))?;

        // Try to parse as JSON, fallback to string
        let result: Result<Value, serde_json::Error> = serde_json::from_str(&text);
        match result {
            Ok(json_value) => Ok(json_value),
            Err(_) => Ok(Value::String(text)),
        }
    }

    async fn post(
        &self,
        url: &str,
        body: &Value,
        headers: Option<HashMap<String, String>>,
    ) -> Result<Value, PlatformError> {
        let client = reqwest::Client::new();
        let mut request = client.post(url).json(body);

        if let Some(headers) = headers {
            for (key, value) in headers {
                request = request.header(&key, &value);
            }
        }

        let response = request
            .send()
            .await
            .map_err(|e| PlatformError::NetworkError(e.to_string()))?;

        let text = response
            .text()
            .await
            .map_err(|e| PlatformError::NetworkError(e.to_string()))?;

        let result: Result<Value, serde_json::Error> = serde_json::from_str(&text);
        match result {
            Ok(json_value) => Ok(json_value),
            Err(_) => Ok(Value::String(text)),
        }
    }

    async fn put(
        &self,
        url: &str,
        body: &Value,
        headers: Option<HashMap<String, String>>,
    ) -> Result<Value, PlatformError> {
        let client = reqwest::Client::new();
        let mut request = client.put(url).json(body);

        if let Some(headers) = headers {
            for (key, value) in headers {
                request = request.header(&key, &value);
            }
        }

        let response = request
            .send()
            .await
            .map_err(|e| PlatformError::NetworkError(e.to_string()))?;

        let text = response
            .text()
            .await
            .map_err(|e| PlatformError::NetworkError(e.to_string()))?;

        let result: Result<Value, serde_json::Error> = serde_json::from_str(&text);
        match result {
            Ok(json_value) => Ok(json_value),
            Err(_) => Ok(Value::String(text)),
        }
    }

    async fn delete(
        &self,
        url: &str,
        headers: Option<HashMap<String, String>>,
    ) -> Result<Value, PlatformError> {
        let client = reqwest::Client::new();
        let mut request = client.delete(url);

        if let Some(headers) = headers {
            for (key, value) in headers {
                request = request.header(&key, &value);
            }
        }

        let response = request
            .send()
            .await
            .map_err(|e| PlatformError::NetworkError(e.to_string()))?;

        let text = response
            .text()
            .await
            .map_err(|e| PlatformError::NetworkError(e.to_string()))?;

        let result: Result<Value, serde_json::Error> = serde_json::from_str(&text);
        match result {
            Ok(json_value) => Ok(json_value),
            Err(_) => Ok(Value::String(text)),
        }
    }

    fn client_type(&self) -> &'static str {
        "TauriHttpClient"
    }
}

/// Tauri implementation of UniversalSystemInterface
pub struct TauriSystemInterface {
    app_handle: AppHandle,
}

impl TauriSystemInterface {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }
}

#[async_trait]
impl UniversalSystemInterface for TauriSystemInterface {
    async fn get_system_info(&self) -> Result<SystemInfo, PlatformError> {
        let platform = std::env::consts::OS.to_string();
        let arch = std::env::consts::ARCH.to_string();
        let version = std::env::var("OS_VERSION").unwrap_or_else(|_| "Unknown".to_string());
        let hostname = hostname::get()
            .map_err(|e| PlatformError::SystemError(e.to_string()))?
            .to_string_lossy()
            .to_string();

        let total_memory = 8 * 1024 * 1024 * 1024; // Default 8GB, would use system crate for real implementation
        let available_memory = 4 * 1024 * 1024 * 1024; // Default 4GB available
        let cpu_count = num_cpus::get() as u32;

        Ok(SystemInfo {
            platform,
            arch,
            version,
            hostname,
            total_memory,
            available_memory,
            cpu_count,
        })
    }

    async fn execute_command(
        &self,
        command: &str,
        args: &[&str],
    ) -> Result<CommandResult, PlatformError> {
        let start_time = std::time::Instant::now();

        let output = tokio::process::Command::new(command)
            .args(args)
            .output()
            .await
            .map_err(|e| PlatformError::SystemError(e.to_string()))?;

        let duration_ms = start_time.elapsed().as_millis() as u64;

        Ok(CommandResult {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            duration_ms,
        })
    }

    async fn get_available_actions(&self) -> Result<Vec<SystemAction>, PlatformError> {
        // This would be loaded from the system service in a real implementation
        Ok(vec![
            SystemAction {
                id: "system.volume.set".to_string(),
                name: "Set Volume".to_string(),
                description: "Set system volume level".to_string(),
                category: "SYSTEM".to_string(),
                requires_elevation: false,
                parameters: vec![ActionParameter {
                    name: "volume".to_string(),
                    parameter_type: "number".to_string(),
                    required: true,
                    default_value: Some(Value::Number(serde_json::Number::from(50))),
                    description: "Volume level (0-100)".to_string(),
                    min_value: Some(0.0),
                    max_value: Some(100.0),
                }],
                is_system: true,
                action_type: "volume".to_string(),
            },
            // SystemAction {
            //     id: "system.app.launch".to_string(),
            //     name: "Launch Application".to_string(),
            //     description: "Launch an application".to_string(),
            //     category: "SYSTEM".to_string(),
            //     requires_elevation: false,
            //     parameters: vec![
            //         ActionParameter {
            //             name: "app_path".to_string(),
            //             parameter_type: "string".to_string(),
            //             required: true,
            //             default_value: None,
            //             description: "Path to the application to launch".to_string(),
            //             min_value: None,
            //             max_value: None,
            //         }
            //     ],
            //     is_system: true,
            //     action_type: "launch".to_string(),
            // },
            // SystemAction {
            //     id: "system.info.get".to_string(),
            //     name: "Get System Info".to_string(),
            //     description: "Get system information".to_string(),
            //     category: "SYSTEM".to_string(),
            //     requires_elevation: false,
            //     parameters: vec![],
            //     is_system: true,
            //     action_type: "info".to_string(),
            // },
        ])
    }

    fn interface_type(&self) -> &'static str {
        "TauriSystemInterface"
    }
}

/// Tauri implementation of UniversalWindowManager
pub struct TauriWindowManager {
    app_handle: AppHandle,
}

impl TauriWindowManager {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }
}

#[async_trait]
impl UniversalWindowManager for TauriWindowManager {
    async fn create_window(&self, config: WindowConfig) -> Result<String, PlatformError> {
        let window_id = format!("window_{}", uuid::Uuid::new_v4());

        let window_builder = tauri::WebviewWindowBuilder::new(
            &self.app_handle,
            &window_id,
            tauri::WebviewUrl::App(config.url.into()),
        )
        .title(&config.title)
        .inner_size(config.width as f64, config.height as f64)
        .resizable(config.resizable)
        .always_on_top(config.always_on_top);

        let _window = window_builder
            .build()
            .map_err(|e| PlatformError::SystemError(e.to_string()))?;

        Ok(window_id)
    }

    async fn close_window(&self, window_id: &str) -> Result<(), PlatformError> {
        if let Some(window) = self.app_handle.get_webview_window(window_id) {
            window
                .close()
                .map_err(|e| PlatformError::SystemError(e.to_string()))?;
        }
        Ok(())
    }

    async fn set_window_visibility(
        &self,
        window_id: &str,
        visible: bool,
    ) -> Result<(), PlatformError> {
        if let Some(window) = self.app_handle.get_webview_window(window_id) {
            if visible {
                window
                    .show()
                    .map_err(|e| PlatformError::SystemError(e.to_string()))?;
            } else {
                window
                    .hide()
                    .map_err(|e| PlatformError::SystemError(e.to_string()))?;
            }
        }
        Ok(())
    }

    fn manager_type(&self) -> &'static str {
        "TauriWindowManager"
    }
}

/// Factory function to create a PlatformContext with Tauri implementations
///
/// Note: EventBus is created automatically in PlatformContext::new()
/// To bridge events to the Tauri frontend, use spawn_event_bridge() in main.rs
pub fn create_tauri_platform_context(app_handle: AppHandle) -> PlatformContext {
    let event_bus_capacity = std::env::var("VORTIDECK_EVENT_BUS_CAPACITY")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(10_000);
    log::info!("Creating EventBus with capacity {}", event_bus_capacity);

    PlatformContext::new_with_event_bus_size(
        Arc::new(TauriStorageProvider::new(app_handle.clone())),
        Arc::new(TauriHttpClient::new(app_handle.clone())),
        Arc::new(TauriSystemInterface::new(app_handle.clone())),
        Some(Arc::new(TauriWindowManager::new(app_handle))),
        event_bus_capacity,
    )
}

/// Spawn a background task to bridge EventBus events to Tauri IPC (frontend)
///
/// This forwards platform-agnostic EventBus events to Tauri's IPC system
/// so the frontend can receive them. This is the ONLY place where events
/// are converted to Tauri-specific format.
pub fn spawn_event_bridge(app_handle: AppHandle, event_bus: Arc<vortideck_core::events::EventBus>) {
    use vortideck_core::events::{EventPayload, FlowEventPayload, SystemEventPayload};

    tauri::async_runtime::spawn(async move {
        let mut rx = event_bus.subscribe();

        log::info!("🌉 Event bridge started - forwarding EventBus → Tauri IPC");

        loop {
            let event = match rx.recv().await {
                Ok(event) => event,
                Err(tokio::sync::broadcast::error::RecvError::Lagged(count)) => {
                    log::warn!("🌉 Event bridge lagged by {} events; continuing", count);
                    continue;
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            };
            match event {
                // Flow events → "flow_event" IPC
                EventPayload::Flow(flow_event) => {
                    if let Ok(json_data) = serde_json::to_value(&flow_event) {
                        if let Err(e) = app_handle.emit("flow_event", json_data) {
                            log::warn!("Failed to forward flow event to frontend: {}", e);
                        }
                    }
                    let mobile_message = match &flow_event {
                        FlowEventPayload::ApprovalRequested { request, .. }
                            if request
                                .payload
                                .get("requestedChannels")
                                .and_then(serde_json::Value::as_array)
                                .is_some_and(|channels| {
                                    channels.iter().any(|channel| channel == "mobile")
                                }) =>
                        {
                            Some(serde_json::json!({
                                "action": "workflow_approval_requested",
                                "protocolVersion": 1,
                                "approval": request,
                            }))
                        }
                        FlowEventPayload::ApprovalResolved {
                            wake_id,
                            resolution,
                            ..
                        } => Some(serde_json::json!({
                            "action": "workflow_approval_resolved",
                            "protocolVersion": 1,
                            "wakeId": wake_id,
                            "decision": resolution.decision,
                            "decidedAt": resolution.decided_at,
                        })),
                        _ => None,
                    };
                    if let Some(message) = mobile_message {
                        if let Some(state_manager) =
                            app_handle.try_state::<Arc<vortideck_core::core::StateManager>>()
                        {
                            crate::broadcast_helper::broadcast_to_all_clients(
                                &app_handle,
                                state_manager.inner(),
                                message,
                            )
                            .await;
                        }
                    }
                }

                // System events → specific IPC events
                EventPayload::System(sys_event) => match sys_event {
                    SystemEventPayload::VolumeChanged {
                        volume,
                        button_id,
                        timestamp,
                    } => {
                        let data = serde_json::json!({
                            "volume": volume,
                            "button_id": button_id,
                            "timestamp": timestamp
                        });
                        if let Err(e) = app_handle.emit("volume_changed", data) {
                            log::warn!("Failed to forward volume_changed to frontend: {}", e);
                        }
                    }

                    SystemEventPayload::SystemInfoRetrieved { system_info } => {
                        if let Ok(data) = serde_json::to_value(&system_info) {
                            if let Err(e) = app_handle.emit("system_info_retrieved", data) {
                                log::warn!(
                                    "Failed to forward system_info_retrieved to frontend: {}",
                                    e
                                );
                            }
                        }
                    }
                },

                // Webhook events → "webhook_event" IPC
                EventPayload::Webhook(webhook_event) => {
                    if let Ok(json_data) = serde_json::to_value(&webhook_event) {
                        if let Err(e) = app_handle.emit("webhook_event", json_data) {
                            log::warn!("Failed to forward webhook event to frontend: {}", e);
                        }
                    }
                }

                // Relay events → specific IPC events (relay_status, relay_command, etc.)
                EventPayload::Relay(relay_event) => {
                    if let Ok(json_data) = serde_json::to_value(&relay_event) {
                        if let Err(e) = app_handle.emit("relay_event", json_data) {
                            log::warn!("Failed to forward relay event to frontend: {}", e);
                        }
                    }
                }

                // Stream events → "stream_event" IPC
                EventPayload::Stream(stream_event) => {
                    if let Ok(json_data) = serde_json::to_value(&stream_event) {
                        if let Err(e) = app_handle.emit("stream_event", json_data) {
                            log::warn!("Failed to forward stream event to frontend: {}", e);
                        }
                    }
                }

                // Session events → "session_event" IPC
                EventPayload::Session(session_event) => {
                    if let Ok(json_data) = serde_json::to_value(&session_event) {
                        if let Err(e) = app_handle.emit("session_event", json_data) {
                            log::warn!("Failed to forward session event to frontend: {}", e);
                        }
                    }
                }

                // Auth events → "auth_event" IPC
                EventPayload::Auth(auth_event) => {
                    if let Ok(json_data) = serde_json::to_value(&auth_event) {
                        if let Err(e) = app_handle.emit("auth_event", json_data) {
                            log::warn!("Failed to forward auth event to frontend: {}", e);
                        }
                    }
                }

                // Overlay commands → "overlay_event" IPC for diagnostics and
                // desktop consumers. The WebSocket overlay bridge handles the
                // actual widget delivery.
                EventPayload::Overlay(overlay_event) => {
                    if let Ok(json_data) = serde_json::to_value(&overlay_event) {
                        if let Err(e) = app_handle.emit("overlay_event", json_data) {
                            log::warn!("Failed to forward overlay event to frontend: {}", e);
                        }
                    }
                }

                // Runtime infrastructure events (trigger changes, etc.)
                EventPayload::Runtime(runtime_event) => {
                    if let Ok(json_data) = serde_json::to_value(&runtime_event) {
                        if let Err(e) = app_handle.emit("runtime_event", json_data) {
                            log::warn!("Failed to forward runtime event to frontend: {}", e);
                        }
                    }
                }

                // Generic/Schemaless events → "generic_event" IPC
                EventPayload::Generic {
                    source,
                    event_type,
                    payload,
                } => {
                    let data = serde_json::json!({
                        "source": source,
                        "eventType": event_type,
                        "payload": payload
                    });
                    if let Err(e) = app_handle.emit("generic_event", data) {
                        log::warn!("Failed to forward generic event to frontend: {}", e);
                    }
                }
                EventPayload::Plugin(plugin_event) => {
                    // The Tauri document is trusted, but untrusted plugin
                    // frames receive this only through the source-bound,
                    // permission-filtered adapter in plugin-window.vue.
                    let event_type = format!(
                        "plugin.{}.{}",
                        plugin_event.package_id, plugin_event.event_type
                    );
                    let data = serde_json::json!({
                        "sourcePackageId": plugin_event.package_id,
                        "eventType": event_type,
                        "declaredEventType": plugin_event.event_type,
                        "payload": plugin_event.payload,
                    });
                    if let Err(e) = app_handle.emit("plugin_event", data) {
                        log::warn!("Failed to forward scoped plugin event to frontend: {}", e);
                    }
                }
            }
        }

        log::warn!("🌉 Event bridge stopped - EventBus channel closed");
    });
}
