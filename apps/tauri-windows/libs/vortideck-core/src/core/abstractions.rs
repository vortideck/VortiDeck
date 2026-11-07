use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// Core error types for platform abstractions
#[derive(Debug, Clone)]
pub enum PlatformError {
    EventEmissionFailed(String),
    StorageError(String),
    NetworkError(String),
    SystemError(String),
    ValidationError(String),
    UnsupportedPlatform(String),
}

impl std::fmt::Display for PlatformError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlatformError::EventEmissionFailed(msg) => write!(f, "Event emission failed: {}", msg),
            PlatformError::StorageError(msg) => write!(f, "Storage error: {}", msg),
            PlatformError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            PlatformError::SystemError(msg) => write!(f, "System error: {}", msg),
            PlatformError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            PlatformError::UnsupportedPlatform(msg) => write!(f, "Unsupported platform: {}", msg),
        }
    }
}

impl std::error::Error for PlatformError {}

// ============================================================================
// REMOVED: UniversalEventEmitter and EventSubscriber traits
// These have been replaced by the EventBus system (events/bus.rs)
// which provides platform-agnostic event broadcasting using tokio channels.
//
// Migration: Use platform_context.event_bus instead of event_emitter
// ============================================================================

/// Universal storage provider for cross-platform data persistence
#[async_trait]
pub trait UniversalStorageProvider: Send + Sync {
    /// Store a value
    async fn set(&self, key: &str, value: &Value) -> Result<(), PlatformError>;

    /// Retrieve a value
    async fn get(&self, key: &str) -> Result<Option<Value>, PlatformError>;

    /// Delete a value
    async fn delete(&self, key: &str) -> Result<(), PlatformError>;

    /// List all keys with optional prefix
    async fn list_keys(&self, prefix: Option<&str>) -> Result<Vec<String>, PlatformError>;

    /// Store binary data
    async fn set_binary(&self, key: &str, data: &[u8]) -> Result<(), PlatformError>;

    /// Retrieve binary data
    async fn get_binary(&self, key: &str) -> Result<Option<Vec<u8>>, PlatformError>;

    /// Get storage type for debugging
    fn storage_type(&self) -> &'static str;

    /// Get application data directory path
    fn get_app_data_path(&self, subpath: &str) -> Result<String, PlatformError>;
}

/// Universal HTTP client for cross-platform network requests
#[async_trait]
pub trait UniversalHttpClient: Send + Sync {
    /// Make a GET request
    async fn get(
        &self,
        url: &str,
        headers: Option<HashMap<String, String>>,
    ) -> Result<Value, PlatformError>;

    /// Make a POST request
    async fn post(
        &self,
        url: &str,
        body: &Value,
        headers: Option<HashMap<String, String>>,
    ) -> Result<Value, PlatformError>;

    /// Make a PUT request
    async fn put(
        &self,
        url: &str,
        body: &Value,
        headers: Option<HashMap<String, String>>,
    ) -> Result<Value, PlatformError>;

    /// Make a DELETE request
    async fn delete(
        &self,
        url: &str,
        headers: Option<HashMap<String, String>>,
    ) -> Result<Value, PlatformError>;

    /// Get client type for debugging
    fn client_type(&self) -> &'static str;
}

/// Universal system interface for cross-platform system operations
#[async_trait]
pub trait UniversalSystemInterface: Send + Sync {
    /// Get system information
    async fn get_system_info(&self) -> Result<SystemInfo, PlatformError>;

    /// Execute a system command (with permission checks)
    async fn execute_command(
        &self,
        command: &str,
        args: &[&str],
    ) -> Result<CommandResult, PlatformError>;

    /// Get available system actions
    async fn get_available_actions(&self) -> Result<Vec<SystemAction>, PlatformError>;

    /// Get interface type for debugging
    fn interface_type(&self) -> &'static str;
}

/// Universal window manager for cross-platform window operations
#[async_trait]
pub trait UniversalWindowManager: Send + Sync {
    /// Create a new window
    async fn create_window(&self, config: WindowConfig) -> Result<String, PlatformError>;

    /// Close a window
    async fn close_window(&self, window_id: &str) -> Result<(), PlatformError>;

    /// Show/hide a window
    async fn set_window_visibility(
        &self,
        window_id: &str,
        visible: bool,
    ) -> Result<(), PlatformError>;

    /// Get window manager type
    fn manager_type(&self) -> &'static str;
}

/// System information structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SystemInfo {
    pub platform: String,
    pub arch: String,
    pub version: String,
    pub hostname: String,
    pub total_memory: u64,
    pub available_memory: u64,
    pub cpu_count: u32,
}

/// Command execution result
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CommandResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
}

/// System action definition
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SystemAction {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub requires_elevation: bool,
    pub parameters: Vec<ActionParameter>,
    pub is_system: bool,
    pub action_type: String,
}

/// Action parameter definition
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ActionParameter {
    pub name: String,
    pub parameter_type: String,
    pub required: bool,
    pub default_value: Option<Value>,
    pub description: String,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
}

/// Volume change event data
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VolumeChangeEvent {
    pub volume: f32,
    pub muted: bool,
    pub is_button_change: bool,
    pub button_id: Option<String>,
    pub timestamp: i64,
}

/// Pure volume change callback trait - no platform dependencies
#[async_trait]
pub trait VolumeChangeCallback: Send + Sync {
    /// Called when system volume changes
    async fn on_volume_change(&self, event: VolumeChangeEvent) -> Result<(), PlatformError>;

    /// Get callback type for debugging
    fn callback_type(&self) -> &'static str;
}

/// Window configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WindowConfig {
    pub title: String,
    pub url: String,
    pub width: u32,
    pub height: u32,
    pub resizable: bool,
    pub always_on_top: bool,
}

/// Platform abstraction bundle - contains all platform-specific implementations
///
/// This struct provides platform-agnostic interfaces for all core functionality.
/// The EventBus handles all event emission - no platform-specific event emitter needed!
#[derive(Clone)]
pub struct PlatformContext {
    pub storage_provider: Arc<dyn UniversalStorageProvider>,
    pub http_client: Arc<dyn UniversalHttpClient>,
    pub system_interface: Arc<dyn UniversalSystemInterface>,
    pub window_manager: Option<Arc<dyn UniversalWindowManager>>,
    /// Platform-agnostic event bus (replaces UniversalEventEmitter)
    pub event_bus: Arc<crate::events::EventBus>,
}

impl PlatformContext {
    /// Create a new PlatformContext with the default EventBus capacity.
    pub fn new(
        storage_provider: Arc<dyn UniversalStorageProvider>,
        http_client: Arc<dyn UniversalHttpClient>,
        system_interface: Arc<dyn UniversalSystemInterface>,
        window_manager: Option<Arc<dyn UniversalWindowManager>>,
    ) -> Self {
        // EventBus owns the documented default capacity.
        let event_bus = Arc::new(crate::events::EventBus::new_default());

        Self {
            storage_provider,
            http_client,
            system_interface,
            window_manager,
            event_bus,
        }
    }

    /// Create PlatformContext with custom event bus buffer size
    pub fn new_with_event_bus_size(
        storage_provider: Arc<dyn UniversalStorageProvider>,
        http_client: Arc<dyn UniversalHttpClient>,
        system_interface: Arc<dyn UniversalSystemInterface>,
        window_manager: Option<Arc<dyn UniversalWindowManager>>,
        event_bus_buffer_size: usize,
    ) -> Self {
        let event_bus = Arc::new(crate::events::EventBus::new(event_bus_buffer_size));

        Self {
            storage_provider,
            http_client,
            system_interface,
            window_manager,
            event_bus,
        }
    }

    /// Get platform information for debugging
    pub fn get_platform_info(&self) -> HashMap<String, String> {
        let mut info = HashMap::new();
        info.insert(
            "event_system".to_string(),
            "EventBus (tokio broadcast)".to_string(),
        );
        info.insert(
            "event_subscribers".to_string(),
            self.event_bus.subscriber_count().to_string(),
        );
        info.insert(
            "storage_provider".to_string(),
            self.storage_provider.storage_type().to_string(),
        );
        info.insert(
            "http_client".to_string(),
            self.http_client.client_type().to_string(),
        );
        info.insert(
            "system_interface".to_string(),
            self.system_interface.interface_type().to_string(),
        );
        if let Some(wm) = &self.window_manager {
            info.insert("window_manager".to_string(), wm.manager_type().to_string());
        }
        info
    }
}

// ============================================================================
// REMOVED: NoopEventEmitter
// No longer needed - EventBus is used directly for all event operations
// ============================================================================

pub struct NoopStorageProvider;

#[async_trait]
impl UniversalStorageProvider for NoopStorageProvider {
    async fn set(&self, _key: &str, _value: &Value) -> Result<(), PlatformError> {
        Ok(())
    }

    async fn get(&self, _key: &str) -> Result<Option<Value>, PlatformError> {
        Ok(None)
    }

    async fn delete(&self, _key: &str) -> Result<(), PlatformError> {
        Ok(())
    }

    async fn list_keys(&self, _prefix: Option<&str>) -> Result<Vec<String>, PlatformError> {
        Ok(vec![])
    }

    async fn set_binary(&self, _key: &str, _data: &[u8]) -> Result<(), PlatformError> {
        Ok(())
    }

    async fn get_binary(&self, _key: &str) -> Result<Option<Vec<u8>>, PlatformError> {
        Ok(None)
    }

    fn storage_type(&self) -> &'static str {
        "NoopStorageProvider"
    }

    fn get_app_data_path(&self, subpath: &str) -> Result<String, PlatformError> {
        // Simple implementation for testing - would be platform-specific in real implementation
        Ok(format!("./app_data/{}", subpath))
    }
}

pub struct NoopHttpClient;

#[async_trait]
impl UniversalHttpClient for NoopHttpClient {
    async fn get(
        &self,
        _url: &str,
        _headers: Option<HashMap<String, String>>,
    ) -> Result<Value, PlatformError> {
        Ok(Value::Null)
    }

    async fn post(
        &self,
        _url: &str,
        _body: &Value,
        _headers: Option<HashMap<String, String>>,
    ) -> Result<Value, PlatformError> {
        Ok(Value::Null)
    }

    async fn put(
        &self,
        _url: &str,
        _body: &Value,
        _headers: Option<HashMap<String, String>>,
    ) -> Result<Value, PlatformError> {
        Ok(Value::Null)
    }

    async fn delete(
        &self,
        _url: &str,
        _headers: Option<HashMap<String, String>>,
    ) -> Result<Value, PlatformError> {
        Ok(Value::Null)
    }

    fn client_type(&self) -> &'static str {
        "NoopHttpClient"
    }
}

pub struct NoopSystemInterface;

#[async_trait]
impl UniversalSystemInterface for NoopSystemInterface {
    async fn get_system_info(&self) -> Result<SystemInfo, PlatformError> {
        Ok(SystemInfo {
            platform: "unknown".to_string(),
            arch: "unknown".to_string(),
            version: "unknown".to_string(),
            hostname: "unknown".to_string(),
            total_memory: 0,
            available_memory: 0,
            cpu_count: 1,
        })
    }

    async fn execute_command(
        &self,
        _command: &str,
        _args: &[&str],
    ) -> Result<CommandResult, PlatformError> {
        Ok(CommandResult {
            exit_code: 0,
            stdout: "".to_string(),
            stderr: "".to_string(),
            duration_ms: 0,
        })
    }

    async fn get_available_actions(&self) -> Result<Vec<SystemAction>, PlatformError> {
        Ok(vec![SystemAction {
            id: "test_action".to_string(),
            name: "Test Action".to_string(),
            description: "A test action for development".to_string(),
            category: "SYSTEM".to_string(),
            requires_elevation: false,
            parameters: vec![],
            is_system: true,
            action_type: "test".to_string(),
        }])
    }

    fn interface_type(&self) -> &'static str {
        "NoopSystemInterface"
    }
}
