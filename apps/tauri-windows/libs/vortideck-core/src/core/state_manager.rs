use crate::features::{
    FeatureRequirement, FeatureValidation, StateManagerAuthProvider, UniversalValidator,
};
use futures::future;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::net::IpAddr;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, RwLock,
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::{broadcast, oneshot, Mutex as TokioMutex, RwLock as AsyncRwLock};
use tokio_tungstenite::tungstenite::Message;

// === CLIENT FILTERING UTILITIES ===

/// Checks if a client should be excluded from peer listings based on device type and model
fn should_exclude_from_peer_list(device_type: &str, device_model: &str) -> bool {
    device_type == "host" || device_model == "obs_plugin" || device_model.contains("_plugin")
}

#[cfg(all(test, feature = "websocket-integration"))]
mod signed_broadcast_tests {
    use super::*;
    use base64::{engine::general_purpose::STANDARD as Base64, Engine as _};
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    #[tokio::test]
    async fn external_broadcasts_are_signed_and_unpaired_peers_receive_nothing() {
        let state = StateManager::new();
        let secret = [23u8; 32];
        let (paired_tx, mut paired_rx) = broadcast::channel(4);
        let (unpaired_tx, mut unpaired_rx) = broadcast::channel(4);
        state
            .add_peer(
                "mobile:paired".into(),
                "192.168.1.20".into(),
                paired_tx,
                "mobile".into(),
                "phone".into(),
            )
            .await;
        state
            .add_peer(
                "mobile:unpaired".into(),
                "192.168.1.21".into(),
                unpaired_tx,
                "mobile".into(),
                "phone".into(),
            )
            .await;
        state
            .store_shared_secret("mobile:paired".into(), secret)
            .await;

        state
            .broadcast_message(serde_json::json!({ "action": "deck_widget_state", "revision": 2 }))
            .await;

        let received = paired_rx.recv().await.expect("signed paired broadcast");
        let Message::Text(text) = received else {
            panic!("expected text broadcast")
        };
        let mut json: serde_json::Value = serde_json::from_str(&text).unwrap();
        let signature = json["signature"].as_str().unwrap().to_string();
        assert!(json["nonce"].as_str().is_some());
        json.as_object_mut().unwrap().remove("signature");
        let canonical = serde_json_canonicalizer::to_string(&json).unwrap();
        let decoded = Base64.decode(signature).unwrap();
        let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(&secret).unwrap();
        mac.update(canonical.as_bytes());
        assert!(mac.verify_slice(&decoded).is_ok());
        assert!(unpaired_rx.try_recv().is_err());
    }
}

// Performance monitoring structures
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub operation_count: u64,
    pub total_duration: Duration,
    pub average_duration: Duration,
    pub last_operation_time: Instant,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            operation_count: 0,
            total_duration: Duration::ZERO,
            average_duration: Duration::ZERO,
            last_operation_time: Instant::now(),
        }
    }
}

// Cache structure for frequently accessed data
#[derive(Debug, Clone)]
pub struct StateCache {
    pub peer_count: Option<(usize, Instant)>,
    pub server_status: Option<(bool, Instant)>,
    pub obs_status: Option<(bool, Instant)>,
    pub premium_status: Option<(bool, Instant)>,
    pub cache_ttl: Duration,
}

impl Default for StateCache {
    fn default() -> Self {
        Self {
            peer_count: None,
            server_status: None,
            obs_status: None,
            premium_status: None,
            cache_ttl: Duration::from_secs(30), // 30 second cache TTL
        }
    }
}

impl StateCache {
    fn is_expired(&self, cached_time: Instant) -> bool {
        cached_time.elapsed() > self.cache_ttl
    }

    fn get_peer_count(&self) -> Option<usize> {
        if let Some((count, time)) = self.peer_count {
            if !self.is_expired(time) {
                return Some(count);
            }
        }
        None
    }

    fn set_peer_count(&mut self, count: usize) {
        self.peer_count = Some((count, Instant::now()));
    }

    fn get_server_status(&self) -> Option<bool> {
        if let Some((status, time)) = self.server_status {
            if !self.is_expired(time) {
                return Some(status);
            }
        }
        None
    }

    fn set_server_status(&mut self, status: bool) {
        self.server_status = Some((status, Instant::now()));
    }
}

// Generic messaging types (no platform abstractions)
pub type MessageSender = broadcast::Sender<Message>;
pub type EventSender = broadcast::Sender<(String, serde_json::Value)>;

/// Execution settings for concurrent actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSettings {
    pub max_concurrent_actions: u32,
    pub max_action_timeout_ms: u64,
}

impl Default for ExecutionSettings {
    fn default() -> Self {
        Self {
            max_concurrent_actions: 10,
            max_action_timeout_ms: 300000, // 5 minutes
        }
    }
}

/// OBS Plugin Action definition (migrated from shared_state.rs)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObsPluginAction {
    #[serde(rename = "actionId")]
    pub action_id: String,
    pub name: String,
    pub parameters: Vec<ObsParameter>,
}

/// OBS Parameter definition (migrated from shared_state.rs)
#[derive(Debug, Clone, Serialize)]
pub struct ObsParameter {
    #[serde(rename = "assignmentMessage")]
    pub assignment_message: String,
    #[serde(default, rename = "errorMessage")]
    pub error_message: String,
    #[serde(default, rename = "listSelection")]
    pub list_selection: Vec<ObsListItem>,
    #[serde(rename = "parameterId")]
    pub parameter_id: String,
    #[serde(default = "default_obs_parameter_type", rename = "parameterType")]
    pub parameter_type: String,
    #[serde(default)]
    pub required: Option<bool>,
}

/// OBS companions released during the protocol transition can include both the
/// canonical SDK field and its legacy UI alias. Field-level serde aliases reject
/// that valid compatibility payload as a duplicate, so deserialize the two wire
/// generations separately and prefer the canonical value when both are present.
impl<'de> Deserialize<'de> for ObsParameter {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct WireParameter {
            #[serde(default, rename = "assignmentMessage")]
            assignment_message: Option<String>,
            #[serde(default)]
            description: Option<String>,
            #[serde(default, rename = "errorMessage")]
            error_message: String,
            #[serde(default, rename = "listSelection")]
            list_selection: Vec<ObsListItem>,
            #[serde(default, rename = "parameterId")]
            parameter_id: Option<String>,
            #[serde(default)]
            name: Option<String>,
            #[serde(default, rename = "parameterType")]
            parameter_type: Option<String>,
            #[serde(default)]
            required: Option<bool>,
        }

        let wire = WireParameter::deserialize(deserializer)?;
        let parameter_id = wire
            .parameter_id
            .or(wire.name)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| serde::de::Error::missing_field("parameterId"))?;

        Ok(Self {
            assignment_message: wire
                .assignment_message
                .or(wire.description)
                .unwrap_or_default(),
            error_message: wire.error_message,
            list_selection: wire.list_selection,
            parameter_id,
            parameter_type: wire
                .parameter_type
                .unwrap_or_else(default_obs_parameter_type),
            required: wire.required,
        })
    }
}

fn default_obs_parameter_type() -> String {
    "STRING".to_string()
}

#[cfg(test)]
mod obs_parameter_wire_tests {
    use super::ObsParameter;

    #[test]
    fn canonical_fields_win_when_companion_also_emits_legacy_aliases() {
        let parameter: ObsParameter = serde_json::from_value(serde_json::json!({
            "parameterId": "sceneName",
            "name": "legacySceneName",
            "assignmentMessage": "Select scene",
            "description": "Legacy scene description",
            "parameterType": "LIST",
            "listSelection": []
        }))
        .expect("compatibility payload should parse");

        assert_eq!(parameter.parameter_id, "sceneName");
        assert_eq!(parameter.assignment_message, "Select scene");
        assert_eq!(parameter.parameter_type, "LIST");
    }

    #[test]
    fn legacy_only_parameter_remains_supported() {
        let parameter: ObsParameter = serde_json::from_value(serde_json::json!({
            "name": "sourceName",
            "description": "Select source"
        }))
        .expect("legacy payload should parse");

        assert_eq!(parameter.parameter_id, "sourceName");
        assert_eq!(parameter.assignment_message, "Select source");
        assert_eq!(parameter.parameter_type, "STRING");
    }
}

/// OBS List Item definition (migrated from shared_state.rs)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObsListItem {
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Vec<ObsParameter>>,
}

/// OBS Integration State (migrated from ObsPluginState in shared_state.rs)
pub struct ObsIntegrationState {
    pub obs_plugin_sender: Arc<AsyncRwLock<Option<MessageSender>>>,
    pub instance_guid: Arc<RwLock<Option<String>>>,
    pub integration_guid: Arc<RwLock<Option<String>>>,
    pub actions: Arc<AsyncRwLock<Option<Vec<ObsPluginAction>>>>,
    pub status: Arc<AsyncRwLock<bool>>,
    pub last_status_time: Arc<AsyncRwLock<Option<u64>>>,
    pub action_receipts_supported: Arc<AtomicBool>,
    pub pending_action_receipts:
        Arc<TokioMutex<HashMap<String, oneshot::Sender<Result<(), String>>>>>,
}

impl std::fmt::Debug for ObsIntegrationState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ObsIntegrationState")
            .field(
                "obs_plugin_sender",
                &"Arc<AsyncRwLock<Option<MessageSender>>>",
            )
            .field("instance_guid", &self.instance_guid)
            .field("integration_guid", &self.integration_guid)
            .field("actions", &"Arc<AsyncRwLock<Option<Vec<ObsPluginAction>>>>")
            .field("status", &"Arc<AsyncRwLock<bool>>")
            .field("last_status_time", &"Arc<AsyncRwLock<Option<u64>>>")
            .field(
                "action_receipts_supported",
                &self.action_receipts_supported.load(Ordering::Relaxed),
            )
            .field("pending_action_receipts", &"Arc<TokioMutex<HashMap<...>>>")
            .finish()
    }
}

impl Clone for ObsIntegrationState {
    fn clone(&self) -> Self {
        Self {
            obs_plugin_sender: self.obs_plugin_sender.clone(),
            instance_guid: self.instance_guid.clone(),
            integration_guid: self.integration_guid.clone(),
            actions: self.actions.clone(),
            status: self.status.clone(),
            last_status_time: self.last_status_time.clone(),
            action_receipts_supported: self.action_receipts_supported.clone(),
            pending_action_receipts: self.pending_action_receipts.clone(),
        }
    }
}

impl Default for ObsIntegrationState {
    fn default() -> Self {
        Self {
            obs_plugin_sender: Arc::new(AsyncRwLock::new(None)),
            instance_guid: Arc::new(RwLock::new(None)),
            integration_guid: Arc::new(RwLock::new(None)),
            actions: Arc::new(AsyncRwLock::new(None)),
            status: Arc::new(AsyncRwLock::new(false)),
            last_status_time: Arc::new(AsyncRwLock::new(None)),
            action_receipts_supported: Arc::new(AtomicBool::new(false)),
            pending_action_receipts: Arc::new(TokioMutex::new(HashMap::new())),
        }
    }
}

/// Public state that plugins can safely access
#[derive(Debug, Clone)]
pub struct PublicState {
    pub ui_state: Arc<RwLock<UIState>>,
    pub module_state: Arc<RwLock<ModuleState>>,
    pub peer_state: Arc<AsyncRwLock<PeerStateData>>,
}

/// Private state that only core systems can access
#[derive(Debug)]
pub struct PrivateState {
    pub auth_state: Arc<AsyncRwLock<AuthState>>,
    pub server_state: Arc<RwLock<ServerState>>,
    pub security_state: Arc<RwLock<SecurityState>>,
    pub relay_state: Arc<AsyncRwLock<RelayState>>,
    #[cfg(feature = "obs-integration")]
    pub obs_state: Arc<ObsIntegrationState>,
}

/// UI-related state
#[derive(Debug, Clone)]
pub struct UIState {
    pub is_mobile_simulator_pinned: bool,
    pub window_positions: HashMap<String, WindowInfo>,
    pub theme_settings: HashMap<String, serde_json::Value>,
}

/// Module-specific state
#[derive(Debug, Clone)]
pub struct ModuleState {
    pub obs_status: bool,
    pub macro_recording: bool,
    pub flow_execution_status: HashMap<String, bool>,
    pub integration_status: HashMap<String, bool>,
}

/// Metadata for shared secret lifecycle management
#[derive(Debug, Clone)]
pub struct SharedSecretMetadata {
    pub secret: [u8; 32],
    pub created_at: SystemTime,
    pub last_used: SystemTime,
}

/// Authentication state (private)
#[derive(Debug, Clone)]
pub struct AuthState {
    pub secret_key: Option<String>,
    pub access_tokens: HashMap<String, String>,
    pub refresh_tokens: HashMap<String, String>,
    pub user_permissions: HashMap<String, Vec<String>>,
    pub server_client_id: Option<String>, // Migrated from AppState
    /// E2EE pairing keys - Maps pairing_token to mobile public key (base64)
    /// After handshake, the shared_secret is stored in e2ee_shared_secrets
    pub e2ee_pairing_keys: HashMap<String, String>,
    /// E2EE shared secrets - Maps client_id to SharedSecretMetadata
    /// Used for HMAC signing of regular commands
    /// 🔐 SECURITY FIX: Now includes metadata for lifecycle management
    pub e2ee_shared_secrets: HashMap<String, SharedSecretMetadata>,
}

/// Server state (private)
pub struct ServerState {
    pub server_info: Option<ServerInfo>,
    pub server_sender: Option<MessageSender>,
    pub is_running: bool,
    pub startup_time: Option<u64>,
}

impl std::fmt::Debug for ServerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServerState")
            .field("server_info", &self.server_info)
            .field("server_sender", &self.server_sender.is_some())
            .field("is_running", &self.is_running)
            .field("startup_time", &self.startup_time)
            .finish()
    }
}

/// Relay state for backend relay connection (private)
#[derive(Debug, Clone)]
pub struct RelayState {
    pub enabled: bool,
    pub relay_url: Option<String>,
    pub desktop_id: Option<String>,
}

impl Default for RelayState {
    fn default() -> Self {
        Self {
            enabled: false,
            relay_url: None,
            desktop_id: None,
        }
    }
}

/// Security state (private) with bounded collections for performance
#[derive(Debug, Clone)]
pub struct SecurityState {
    pub plugin_permissions: HashMap<String, PluginPermissions>,
    pub rate_limits: HashMap<String, u64>,
    /// Bounded security events (latest 1000 events to prevent memory growth)
    pub security_events: std::collections::VecDeque<SecurityEvent>,
    /// Nonce tracking for replay attack prevention
    /// Maps nonce -> (timestamp, client_id) for expiration cleanup
    /// Bounded to 10000 nonces (auto-cleanup old ones)
    pub used_nonces: HashMap<String, (u64, String)>,
}

/// Peer management state
#[derive(Debug, Clone)]
pub struct PeerStateData {
    pub active_peers: HashMap<String, PeerInfo>,
    pub peer_history: Vec<PeerInfo>,
    pub connection_stats: HashMap<String, ConnectionStats>,
}

/// Peer information
#[derive(Clone)]
pub struct PeerInfo {
    pub client_id: String,
    pub ip: IpAddr,
    pub device_type: String,
    pub device: String,
    pub connected_at: u64,
    pub last_seen: u64,
    pub sender: Option<MessageSender>,
}

impl std::fmt::Debug for PeerInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PeerInfo")
            .field("client_id", &self.client_id)
            .field("ip", &self.ip)
            .field("device_type", &self.device_type)
            .field("device", &self.device)
            .field("connected_at", &self.connected_at)
            .field("last_seen", &self.last_seen)
            .field("sender", &self.sender.is_some())
            .finish()
    }
}

/// Serializable peer info for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializablePeerInfo {
    pub client_id: String,
    pub device_type: String,
    pub device: String,
}

/// Server information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionCandidate {
    /// Network on which this endpoint is expected to be reachable.
    pub transport: String,
    /// IP address or DNS name. Credentials never belong in this field.
    pub host: String,
    pub port: u16,
    /// Absolute WebSocket path, normally `/ws`.
    pub path: String,
    /// Whether the endpoint uses `wss` instead of `ws`.
    pub secure: bool,
    /// Lower values are attempted first by clients.
    pub priority: u16,
}

impl ConnectionCandidate {
    pub fn websocket_url(&self) -> String {
        let scheme = if self.secure { "wss" } else { "ws" };
        let host = if self.host.contains(':') && !self.host.starts_with('[') {
            format!("[{}]", self.host)
        } else {
            self.host.clone()
        };
        format!("{}://{}:{}{}", scheme, host, self.port, self.path)
    }
}

/// Server information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub ip: IpAddr,
    pub port: u16,
    pub icons_server_port: u16,
    /// Opaque bearer token required by browser-source overlay endpoints for the
    /// lifetime of this server process. It is intentionally not persisted.
    pub overlay_access_token: String,
    /// Separate process-scoped credential for operational metrics. It is never
    /// included in SerializableServerInfo or delivered to overlay clients.
    pub metrics_access_token: String,
    pub device_type: String,
    pub server_name: String,
    /// Reachability hints only. The normal VortiDeck pairing/authentication
    /// handshake still proves the desktop identity on every transport.
    pub connection_candidates: Vec<ConnectionCandidate>,
}

/// Serializable server info for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializableServerInfo {
    pub ip: IpAddr,
    pub port: u16,
    pub icons_server_port: u16,
    pub overlay_access_token: String,
    pub device_type: String,
    pub server_name: String,
    pub connection_candidates: Vec<ConnectionCandidate>,
}

/// Window information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Plugin security level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginSecurityLevel {
    /// Full access (native plugins, high trust)
    Trusted,
    /// Limited access (verified plugins)
    Verified,
    /// Restricted access (community plugins)
    Community,
    /// Maximum restrictions (untested plugins)
    Sandbox,
}

/// Plugin sandbox mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginSandboxMode {
    /// No sandboxing (trusted plugins only)
    None,
    /// Basic isolation (limited system access)
    Basic,
    /// Strong isolation (WebAssembly sandbox)
    Strict,
    /// Maximum isolation (separate process)
    Isolated,
}

/// Result of plugin permission check
#[derive(Debug, Clone)]
pub enum PluginPermissionResult {
    /// Permission granted
    Allowed,
    /// Permission denied with reason
    Denied(String),
}

/// Enhanced plugin permissions with detailed access control
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginPermissions {
    // === Basic Access Permissions ===
    pub can_access_ui_state: bool,
    pub can_access_module_state: bool,
    pub can_execute_system_commands: bool,

    // === Integration Permissions ===
    pub allowed_integrations: Vec<String>,
    pub can_provide_integrations: bool,
    pub can_register_integrations: bool,

    // === Network & Security Permissions ===
    pub can_make_network_requests: bool,
    pub allowed_domains: Vec<String>,
    pub can_access_files: bool,
    pub allowed_file_paths: Vec<String>,

    // === Event & Communication Permissions ===
    pub can_emit_events: bool,
    pub can_listen_to_events: bool,
    pub allowed_event_types: Vec<String>,

    // === Resource Limits ===
    pub max_memory_mb: Option<u64>,
    pub max_cpu_time_ms: Option<u64>,
    pub max_network_requests_per_minute: Option<u32>,

    // === Feature & Capability Requirements ===
    pub feature_requirements: Vec<String>,
    pub required_capabilities: Vec<String>,

    // === Security Level ===
    pub security_level: PluginSecurityLevel,
    pub sandbox_mode: PluginSandboxMode,
}

/// Security event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    pub timestamp: u64,
    pub event_type: String,
    pub plugin_id: Option<String>,
    pub description: String,
}

/// Connection statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStats {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_transferred: u64,
    pub connection_duration: u64,
}

/// Main StateManager with feature flag support and performance optimizations
pub struct StateManager {
    pub public_state: PublicState,
    private_state: PrivateState,
    event_sender: Option<EventSender>,
    /// Performance optimization layer
    performance_layer: Arc<crate::core::performance::OptimizedStateAccessor>,
    /// Universal feature validator for premium features and usage tracking
    feature_validator: Arc<tokio::sync::RwLock<Arc<UniversalValidator>>>,
    /// 🚀 PERFORMANCE: Add caching and monitoring
    cache: Arc<TokioMutex<StateCache>>,
    performance_metrics: Arc<TokioMutex<HashMap<String, PerformanceMetrics>>>,
    mobile_volume_listeners: Arc<TokioMutex<HashMap<String, MessageSender>>>,
    /// 🆕 Execution settings cache (loaded from DB once, updated in memory)
    execution_settings: Arc<TokioMutex<ExecutionSettings>>,
}

impl Default for UIState {
    fn default() -> Self {
        Self {
            is_mobile_simulator_pinned: true,
            window_positions: HashMap::new(),
            theme_settings: HashMap::new(),
        }
    }
}

impl Default for ModuleState {
    fn default() -> Self {
        Self {
            obs_status: false,
            macro_recording: false,
            flow_execution_status: HashMap::new(),
            integration_status: HashMap::new(),
        }
    }
}

impl Default for AuthState {
    fn default() -> Self {
        Self {
            secret_key: None,
            access_tokens: HashMap::new(),
            refresh_tokens: HashMap::new(),
            user_permissions: HashMap::new(),
            server_client_id: None,
            e2ee_pairing_keys: HashMap::new(),
            e2ee_shared_secrets: HashMap::new(),
        }
    }
}

impl Default for ServerState {
    fn default() -> Self {
        Self {
            server_info: None,
            server_sender: None,
            is_running: false,
            startup_time: None,
        }
    }
}

impl Default for SecurityState {
    fn default() -> Self {
        Self {
            plugin_permissions: HashMap::new(),
            rate_limits: HashMap::new(),
            security_events: VecDeque::new(),
            used_nonces: HashMap::new(),
        }
    }
}

impl Default for PluginSecurityLevel {
    fn default() -> Self {
        Self::Community // Default to community level (restricted)
    }
}

impl Default for PluginSandboxMode {
    fn default() -> Self {
        Self::Strict // Default to strict sandboxing
    }
}

impl Default for PluginPermissions {
    fn default() -> Self {
        Self {
            // Basic access - all disabled by default
            can_access_ui_state: false,
            can_access_module_state: false,
            can_execute_system_commands: false,

            // Integration permissions - restricted by default
            allowed_integrations: Vec::new(),
            can_provide_integrations: false,
            can_register_integrations: false,

            // Network & security - restricted by default
            can_make_network_requests: false,
            allowed_domains: Vec::new(),
            can_access_files: false,
            allowed_file_paths: Vec::new(),

            // Events - limited by default
            can_emit_events: false,
            can_listen_to_events: false,
            allowed_event_types: Vec::new(),

            // Resource limits - conservative defaults
            max_memory_mb: Some(100),                  // 100MB default limit
            max_cpu_time_ms: Some(5000),               // 5 second default limit
            max_network_requests_per_minute: Some(60), // 1 per second default

            // Requirements
            feature_requirements: Vec::new(),
            required_capabilities: Vec::new(),

            // Security - safe defaults
            security_level: PluginSecurityLevel::default(),
            sandbox_mode: PluginSandboxMode::default(),
        }
    }
}

impl Default for PeerStateData {
    fn default() -> Self {
        Self {
            active_peers: HashMap::new(),
            peer_history: Vec::new(),
            connection_stats: HashMap::new(),
        }
    }
}

impl StateManager {
    /// Create a new StateManager instance with performance optimizations
    pub fn new() -> Self {
        let performance_layer = Arc::new(crate::core::performance::OptimizedStateAccessor::new());
        performance_layer.initialize();

        Self {
            public_state: PublicState {
                ui_state: Arc::new(RwLock::new(UIState::default())),
                module_state: Arc::new(RwLock::new(ModuleState::default())),
                peer_state: Arc::new(AsyncRwLock::new(PeerStateData::default())),
            },
            private_state: PrivateState {
                auth_state: Arc::new(AsyncRwLock::new(AuthState::default())),
                server_state: Arc::new(RwLock::new(ServerState::default())),
                security_state: Arc::new(RwLock::new(SecurityState::default())),
                relay_state: Arc::new(AsyncRwLock::new(RelayState::default())),
                #[cfg(feature = "obs-integration")]
                obs_state: Arc::new(ObsIntegrationState::default()),
            },
            event_sender: None,
            performance_layer,
            feature_validator: Arc::new(tokio::sync::RwLock::new(Arc::new(
                UniversalValidator::new(
                    Arc::new(crate::features::MockAuthProvider::new(false)), // Temporary mock, will be updated
                ),
            ))),
            cache: Arc::new(TokioMutex::new(StateCache::default())),
            performance_metrics: Arc::new(TokioMutex::new(HashMap::new())),
            mobile_volume_listeners: Arc::new(TokioMutex::new(HashMap::new())),
            execution_settings: Arc::new(TokioMutex::new(ExecutionSettings::default())),
        }
    }

    /// Set event sender for headless operation
    pub fn with_event_sender(mut self, sender: EventSender) -> Self {
        self.event_sender = Some(sender);
        self
    }

    /// Initialize feature validator with StateManager auth integration
    pub async fn initialize_feature_validator(&self) {
        // Create StateManager-integrated auth provider
        let auth_provider = Arc::new(crate::features::StateManagerAuthProvider::new(
            self.private_state.auth_state.clone(),
        ));

        // Replace the temporary mock with real validator
        let validator = Arc::new(crate::features::UniversalValidator::with_cache_duration(
            auth_provider,
            60,
        ));

        // Register default VortiDeck features
        validator
            .register_feature(crate::features::PremiumFeature::UsageLimit {
                feature_id: "ui_buttons".to_string(),
                free_limit: 6,
                premium_limit: None, // Unlimited for premium
                reset_period: crate::features::ResetPeriod::Never,
            })
            .await;

        validator
            .register_feature(crate::features::PremiumFeature::StorageLimit {
                feature_id: "macro_recording".to_string(),
                free_limit: 10,
                premium_limit: None, // Unlimited for premium
            })
            .await;

        validator
            .register_feature(crate::features::PremiumFeature::StorageLimit {
                feature_id: "saved_decks".to_string(),
                free_limit: 3,
                premium_limit: None, // Unlimited for premium
            })
            .await;

        validator
            .register_feature(crate::features::PremiumFeature::FeatureFlag {
                feature_id: "advanced_integrations".to_string(),
                enabled: true, // Requires premium
            })
            .await;

        validator
            .register_feature(crate::features::PremiumFeature::UsageLimit {
                feature_id: "plugin_installations".to_string(),
                free_limit: 3,
                premium_limit: None, // Unlimited for premium
                reset_period: crate::features::ResetPeriod::Never,
            })
            .await;

        validator
            .register_feature(crate::features::PremiumFeature::FeatureFlag {
                feature_id: "cloud_sync".to_string(),
                enabled: true, // Requires premium
            })
            .await;

        validator
            .register_feature(crate::features::PremiumFeature::UsageLimit {
                feature_id: "custom_themes".to_string(),
                free_limit: 1,
                premium_limit: None, // Unlimited for premium
                reset_period: crate::features::ResetPeriod::Never,
            })
            .await;

        validator
            .register_feature(crate::features::PremiumFeature::TimeLimit {
                feature_id: "session_recording".to_string(),
                free_limit_seconds: 300,     // 5 minutes for free
                premium_limit_seconds: None, // Unlimited for premium
            })
            .await;

        validator
            .register_feature(crate::features::PremiumFeature::UsageLimit {
                feature_id: "ai_suggestions".to_string(),
                free_limit: 20,
                premium_limit: None, // Unlimited for premium
                reset_period: crate::features::ResetPeriod::Daily,
            })
            .await;

        validator
            .register_feature(crate::features::PremiumFeature::FeatureFlag {
                feature_id: "priority_support".to_string(),
                enabled: true, // Requires premium
            })
            .await;

        validator
            .register_feature(crate::features::PremiumFeature::UsageLimit {
                feature_id: "export_formats".to_string(),
                free_limit: 2,       // Basic formats only
                premium_limit: None, // All formats for premium
                reset_period: crate::features::ResetPeriod::Never,
            })
            .await;

        // Replace the feature validator safely
        let mut feature_validator_guard = self.feature_validator.write().await;
        *feature_validator_guard = validator;

        log::info!("✅ Feature validator initialized with backend integration and 60s cache");
    }

    /// Get performance metrics from the optimization layer
    pub async fn get_performance_metrics(&self) -> crate::core::performance::PerformanceMetrics {
        self.performance_layer.get_metrics().await
    }

    /// Get public state (safe for plugins)
    pub fn get_public_state(&self) -> &PublicState {
        &self.public_state
    }

    // === UI State Management ===

    pub fn is_mobile_simulator_pinned(&self) -> bool {
        self.public_state
            .ui_state
            .read()
            .unwrap()
            .is_mobile_simulator_pinned
    }

    pub fn pin_mobile_simulator(&self) {
        if let Ok(mut ui_state) = self.public_state.ui_state.write() {
            ui_state.is_mobile_simulator_pinned = true;
        }
    }

    pub fn unpin_mobile_simulator(&self) {
        if let Ok(mut ui_state) = self.public_state.ui_state.write() {
            ui_state.is_mobile_simulator_pinned = false;
        }
    }

    // === Authentication Management (Private) ===

    pub async fn set_secret_key(&self, secret: String) {
        let mut auth_state = self.private_state.auth_state.write().await;
        auth_state.secret_key = Some(secret);
    }

    pub async fn get_secret_key(&self) -> Option<String> {
        let auth_state = self.private_state.auth_state.read().await;
        auth_state.secret_key.clone()
    }

    pub async fn clear_secret_key(&self) {
        let mut auth_state = self.private_state.auth_state.write().await;
        auth_state.secret_key = None;
    }

    // === Server Client ID Management (migrated from AppState) ===

    pub async fn set_server_client_id(&self, client_id: String) {
        let mut auth_state = self.private_state.auth_state.write().await;
        auth_state.server_client_id = Some(client_id);
    }

    pub async fn get_server_client_id(&self) -> Option<String> {
        let auth_state = self.private_state.auth_state.read().await;
        auth_state.server_client_id.clone()
    }

    pub async fn clear_server_client_id(&self) {
        let mut auth_state = self.private_state.auth_state.write().await;
        auth_state.server_client_id = None;
    }

    // === OBS Integration Management (migrated from ObsPluginState) ===

    #[cfg(feature = "obs-integration")]
    pub async fn set_obs_plugin_sender(&self, sender: MessageSender) {
        let mut obs_sender = self.private_state.obs_state.obs_plugin_sender.write().await;
        *obs_sender = Some(sender);
    }

    #[cfg(feature = "obs-integration")]
    pub async fn get_obs_plugin_sender(&self) -> Option<MessageSender> {
        let obs_sender = self.private_state.obs_state.obs_plugin_sender.read().await;
        obs_sender.clone()
    }

    #[cfg(feature = "obs-integration")]
    pub async fn set_obs_actions(&self, actions: Vec<ObsPluginAction>) {
        let mut obs_actions = self.private_state.obs_state.actions.write().await;
        *obs_actions = Some(actions);
    }

    #[cfg(feature = "obs-integration")]
    pub async fn get_obs_actions(&self) -> Option<Vec<ObsPluginAction>> {
        let obs_actions = self.private_state.obs_state.actions.read().await;
        obs_actions.clone()
    }

    #[cfg(feature = "obs-integration")]
    pub async fn set_obs_status_detailed(&self, status: bool) {
        // Update both the detailed OBS state and the public module state
        let mut obs_status = self.private_state.obs_state.status.write().await;
        *obs_status = status;

        // Also update the public module state for consistency
        let mut module_state = self.public_state.module_state.write().unwrap();
        module_state.obs_status = status;
    }

    #[cfg(feature = "obs-integration")]
    pub async fn get_obs_status_detailed(&self) -> bool {
        let obs_status = self.private_state.obs_state.status.read().await;
        *obs_status
    }

    #[cfg(feature = "obs-integration")]
    pub async fn set_obs_last_status_time(&self, timestamp: u64) {
        let mut last_status_time = self.private_state.obs_state.last_status_time.write().await;
        *last_status_time = Some(timestamp);
    }

    #[cfg(feature = "obs-integration")]
    pub async fn get_obs_last_status_time(&self) -> Option<u64> {
        let last_status_time = self.private_state.obs_state.last_status_time.read().await;
        *last_status_time
    }

    #[cfg(feature = "obs-integration")]
    pub fn set_obs_instance_guid(&self, guid: String) {
        if let Ok(mut instance_guid) = self.private_state.obs_state.instance_guid.write() {
            *instance_guid = Some(guid);
        }
    }

    #[cfg(feature = "obs-integration")]
    pub fn get_obs_instance_guid(&self) -> Option<String> {
        match self.private_state.obs_state.instance_guid.read() {
            Ok(instance_guid) => instance_guid.clone(),
            _ => None,
        }
    }

    #[cfg(feature = "obs-integration")]
    pub fn set_obs_integration_guid(&self, guid: String) {
        if let Ok(mut integration_guid) = self.private_state.obs_state.integration_guid.write() {
            *integration_guid = Some(guid);
        }
    }

    #[cfg(feature = "obs-integration")]
    pub fn get_obs_integration_guid(&self) -> Option<String> {
        match self.private_state.obs_state.integration_guid.read() {
            Ok(integration_guid) => integration_guid.clone(),
            _ => None,
        }
    }

    #[cfg(feature = "obs-integration")]
    pub fn set_obs_action_receipts_supported(&self, supported: bool) {
        self.private_state
            .obs_state
            .action_receipts_supported
            .store(supported, Ordering::Release);
    }

    #[cfg(feature = "obs-integration")]
    pub fn obs_action_receipts_supported(&self) -> bool {
        self.private_state
            .obs_state
            .action_receipts_supported
            .load(Ordering::Acquire)
    }

    #[cfg(feature = "obs-integration")]
    pub async fn register_obs_action_receipt(
        &self,
        request_id: String,
    ) -> oneshot::Receiver<Result<(), String>> {
        let (sender, receiver) = oneshot::channel();
        self.private_state
            .obs_state
            .pending_action_receipts
            .lock()
            .await
            .insert(request_id, sender);
        receiver
    }

    #[cfg(feature = "obs-integration")]
    pub async fn resolve_obs_action_receipt(
        &self,
        request_id: &str,
        outcome: Result<(), String>,
    ) -> bool {
        self.private_state
            .obs_state
            .pending_action_receipts
            .lock()
            .await
            .remove(request_id)
            .is_some_and(|sender| sender.send(outcome).is_ok())
    }

    #[cfg(feature = "obs-integration")]
    pub async fn cancel_obs_action_receipt(&self, request_id: &str) {
        self.private_state
            .obs_state
            .pending_action_receipts
            .lock()
            .await
            .remove(request_id);
    }

    /// Reset per-connection OBS protocol capabilities and fail every live
    /// receipt waiter immediately. A newly connected companion must negotiate
    /// its capabilities again through action registration.
    #[cfg(feature = "obs-integration")]
    pub async fn disconnect_obs_companion(&self, reason: &str) {
        self.set_obs_status(false);
        self.set_obs_action_receipts_supported(false);
        // Action/scene/source definitions are connection-scoped. Keeping the
        // previous companion's catalog makes a later session appear populated
        // with targets that may no longer exist.
        self.set_obs_actions(Vec::new()).await;
        *self.private_state.obs_state.obs_plugin_sender.write().await = None;
        let pending = std::mem::take(
            &mut *self
                .private_state
                .obs_state
                .pending_action_receipts
                .lock()
                .await,
        );
        for (_, sender) in pending {
            let _ = sender.send(Err(reason.to_string()));
        }
    }

    #[cfg(feature = "obs-integration")]
    fn parse_obs_actions(actions: &[serde_json::Value]) -> Result<Vec<ObsPluginAction>, String> {
        actions
            .iter()
            .map(|action_value| {
                serde_json::from_value::<ObsPluginAction>(action_value.clone())
                    .map_err(|e| format!("Failed to parse OBS action: {}", e))
            })
            .collect()
    }

    #[cfg(feature = "obs-integration")]
    pub async fn replace_obs_action_catalog(
        &self,
        actions: &[serde_json::Value],
    ) -> Result<(), String> {
        let mut registered = Self::parse_obs_actions(actions)?;
        registered.sort_by(|left, right| left.action_id.cmp(&right.action_id));
        self.set_obs_actions(registered).await;
        Ok(())
    }

    #[cfg(feature = "obs-integration")]
    pub async fn handle_obs_action_register(
        &self,
        actions: &[serde_json::Value],
    ) -> Result<(), String> {
        // Incremental publishers can register action groups during one
        // connection. Upsert refreshed definitions by stable action ID; the
        // complete catalog is cleared when that connection is torn down.
        let registered = Self::parse_obs_actions(actions)?;
        let refreshed_ids = registered
            .iter()
            .map(|action| action.action_id.clone())
            .collect::<std::collections::HashSet<_>>();
        let mut obs_actions = self.private_state.obs_state.actions.write().await;
        let catalog = obs_actions.get_or_insert_with(Vec::new);
        catalog.retain(|action| !refreshed_ids.contains(action.action_id.as_str()));
        catalog.extend(registered);
        catalog.sort_by(|left, right| left.action_id.cmp(&right.action_id));
        Ok(())
    }

    #[cfg(feature = "obs-integration")]
    pub async fn check_obs_status(&self) {
        use std::time::{SystemTime, UNIX_EPOCH};

        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Check if we have a recent status update (within last 30 seconds)
        if let Some(last_time) = self.get_obs_last_status_time().await {
            if current_time - last_time < 30 {
                // Status is recent, consider OBS connected
                self.set_obs_status_detailed(true).await;

                // Emit event if we have an event sender
                if let Some(sender) = &self.event_sender {
                    let _ = sender.send(("obs_plugin_status".to_string(), serde_json::json!(true)));
                }
                return;
            }
        }

        // No recent status, consider OBS disconnected
        self.set_obs_status_detailed(false).await;

        // Emit event if we have an event sender
        if let Some(sender) = &self.event_sender {
            let _ = sender.send(("obs_plugin_status".to_string(), serde_json::json!(false)));
        }
    }

    // === Server Management (Private) ===

    pub fn set_server_info(&self, server_info: ServerInfo) {
        if let Ok(mut server_state) = self.private_state.server_state.write() {
            server_state.server_info = Some(server_info);
            server_state.is_running = true;
            server_state.startup_time = Some(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            );
        }
    }

    pub fn get_server_info(&self) -> Option<SerializableServerInfo> {
        let server_state = self.private_state.server_state.read().ok()?;
        let server_info = server_state.server_info.as_ref()?;
        Some(SerializableServerInfo {
            ip: server_info.ip,
            port: server_info.port,
            icons_server_port: server_info.icons_server_port,
            overlay_access_token: server_info.overlay_access_token.clone(),
            device_type: server_info.device_type.clone(),
            server_name: server_info.server_name.clone(),
            connection_candidates: server_info.connection_candidates.clone(),
        })
    }

    pub fn get_metrics_access_token(&self) -> Option<String> {
        let server_state = self.private_state.server_state.read().ok()?;
        Some(
            server_state
                .server_info
                .as_ref()?
                .metrics_access_token
                .clone(),
        )
    }

    pub fn is_server_running(&self) -> bool {
        self.private_state
            .server_state
            .read()
            .map(|state| state.is_running)
            .unwrap_or(false)
    }

    #[cfg(feature = "websocket-integration")]
    pub fn set_server_sender(&self, sender: MessageSender) {
        if let Ok(mut server_state) = self.private_state.server_state.write() {
            server_state.server_sender = Some(sender);
        }
    }

    #[cfg(feature = "websocket-integration")]
    pub fn get_server_sender(&self) -> Option<MessageSender> {
        self.private_state
            .server_state
            .read()
            .ok()?
            .server_sender
            .clone()
    }

    // === Peer Management ===

    #[cfg(feature = "websocket-integration")]
    pub async fn add_peer(
        &self,
        client_id: String,
        ip: String,
        sender: MessageSender,
        device_type: String,
        device: String,
    ) {
        let peer_info = PeerInfo {
            client_id: client_id.clone(),
            ip: ip.parse().unwrap_or("127.0.0.1".parse().unwrap()),
            device_type: device_type.clone(),
            device: device.clone(),
            connected_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            last_seen: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            sender: Some(sender),
        };

        let mut peer_state = self.public_state.peer_state.write().await;

        // Check if peer already exists
        let is_existing_peer = peer_state.active_peers.contains_key(&client_id);

        // Insert or update the peer
        peer_state
            .active_peers
            .insert(client_id.clone(), peer_info.clone());

        // Only add to peer history if it's a new peer (to avoid duplicates in history)
        if !is_existing_peer {
            peer_state.peer_history.push(peer_info);
        }

        // Emit event if sender is available and not an internal client
        if let Some(sender) = &self.event_sender {
            if device_type != "host" && device != "obs_plugin" {
                let event_type = if is_existing_peer {
                    "active_peer_updated"
                } else {
                    "new_active_peer_added"
                };

                let event_data = serde_json::json!({
                    "client_id": client_id,
                    "ip": ip,
                    "device_type": device_type,
                    "device": device,
                    "deviceType": device_type,  // Add compatibility field for frontend
                    "deviceName": device,       // Add device name for frontend
                    "is_reconnection": is_existing_peer
                });

                let _ = sender.send((event_type.to_string(), event_data));
            }
        }
    }

    pub async fn remove_peer(&self, client_id: &str) {
        let mut peer_state = self.public_state.peer_state.write().await;

        // Get peer info before removing for event emission
        let removed_peer = peer_state.active_peers.get(client_id).cloned();
        peer_state.active_peers.remove(client_id);

        // Emit event if sender is available
        if let Some(sender) = &self.event_sender {
            let event_data = if let Some(peer) = removed_peer {
                // Include peer info in removal event
                serde_json::json!({
                    "client_id": client_id,
                    "device_type": peer.device_type,
                    "device": peer.device,
                    "deviceType": peer.device_type  // Add compatibility field
                })
            } else {
                // Fallback if peer info not available
                serde_json::json!(client_id)
            };

            let _ = sender.send(("active_peer_removed".to_string(), event_data));
        }
    }

    pub async fn get_peers(&self) -> Vec<SerializablePeerInfo> {
        let peer_state = self.public_state.peer_state.read().await;
        peer_state
            .active_peers
            .values()
            .filter(|peer| !should_exclude_from_peer_list(&peer.device_type, &peer.device)) // Use utility function
            .map(|peer| SerializablePeerInfo {
                client_id: peer.client_id.clone(),
                device_type: peer.device_type.clone(),
                device: peer.device.clone(),
            })
            .collect()
    }

    pub async fn broadcast_message(&self, message: serde_json::Value) {
        let recipients: Vec<_> = {
            let peer_state = self.public_state.peer_state.read().await;
            peer_state
                .active_peers
                .values()
                .filter(|peer| !should_exclude_from_peer_list(&peer.device_type, &peer.device))
                .filter_map(|peer| {
                    peer.sender
                        .clone()
                        .map(|sender| (peer.client_id.clone(), sender))
                })
                .collect()
        };

        for (client_id, sender) in recipients {
            let Some(shared_secret) = self.get_shared_secret(&client_id).await else {
                eprintln!(
                    "🚫 Skipping unsigned broadcast to unpaired external client: {}",
                    client_id
                );
                continue;
            };

            use base64::{engine::general_purpose::STANDARD as Base64, Engine as _};
            use hmac::{Hmac, Mac};
            use sha2::Sha256;

            let mut signed = message.clone();
            signed["nonce"] = serde_json::json!(uuid::Uuid::new_v4().to_string());
            let canonical = match serde_json_canonicalizer::to_string(&signed) {
                Ok(value) => value,
                Err(error) => {
                    eprintln!("🚫 Failed to canonicalize direct broadcast: {}", error);
                    continue;
                }
            };
            let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(&shared_secret)
                .expect("HMAC accepts a 32-byte shared secret");
            mac.update(canonical.as_bytes());
            signed["signature"] = serde_json::json!(Base64.encode(mac.finalize().into_bytes()));
            let _ = sender.send(Message::Text(signed.to_string().into()));
        }
    }

    #[cfg(feature = "websocket-integration")]
    pub async fn clear_stale_peers(&self) {
        let mut peer_state = self.public_state.peer_state.write().await;
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Remove peers that haven't been seen in more than 30 seconds
        let stale_threshold = 30;
        let stale_peers: Vec<String> = peer_state
            .active_peers
            .iter()
            .filter(|(_, peer)| current_time - peer.last_seen > stale_threshold)
            .map(|(client_id, _)| client_id.clone())
            .collect();

        let peer_count = stale_peers.len(); // Store count before moving

        for client_id in stale_peers {
            peer_state.active_peers.remove(&client_id);
            println!("🧹 Removed stale peer: {}", client_id);

            // Also remove from mobile volume listeners if it exists
            self.mobile_volume_listeners.lock().await.remove(&client_id);
        }

        println!("🧹 Cleared {} stale peer connections", peer_count);
    }

    // === Plugin Security Management ===

    /// Set plugin permissions with cache invalidation
    pub fn set_plugin_permissions(&self, plugin_id: String, permissions: PluginPermissions) {
        if let Ok(mut security_state) = self.private_state.security_state.write() {
            security_state
                .plugin_permissions
                .insert(plugin_id.clone(), permissions);
        }

        // Invalidate cache for this plugin
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            let _ = handle.block_on(self.performance_layer.invalidate_plugin(&plugin_id));
        }
    }

    /// Optimized permission checking with caching
    pub fn check_plugin_permission(&self, plugin_id: &str, permission: &str) -> bool {
        // Use async runtime to check cache (this is a design trade-off for performance)
        let rt_handle = tokio::runtime::Handle::try_current().ok();

        if let Some(handle) = &rt_handle {
            // We're in an async context, use optimized cached path
            if let Some(cached_result) = handle.block_on(
                self.performance_layer
                    .check_permission(plugin_id, permission),
            ) {
                return cached_result; // Cache hit!
            }
        }

        // Cache miss or not in async context - compute permission
        let result = self.compute_plugin_permission(plugin_id, permission);

        // Store in cache if in async context
        if let Some(handle) = &rt_handle {
            let _ = handle.block_on(
                self.performance_layer
                    .store_permission(plugin_id, permission, result),
            );
        }

        result
    }

    /// Compute plugin permission (internal, uncached)
    fn compute_plugin_permission(&self, plugin_id: &str, permission: &str) -> bool {
        match self.private_state.security_state.read() {
            Ok(security_state) => {
                if let Some(permissions) = security_state.plugin_permissions.get(plugin_id) {
                    match permission {
                        // Basic access permissions
                        "read_ui_state" => permissions.can_access_ui_state,
                        "read_module_state" => permissions.can_access_module_state,
                        "execute_system_commands" => permissions.can_execute_system_commands,

                        // Integration permissions
                        "provide_integrations" => permissions.can_provide_integrations,
                        "register_integrations" => permissions.can_register_integrations,

                        // Network permissions
                        "make_network_requests" => permissions.can_make_network_requests,
                        "access_files" => permissions.can_access_files,

                        // Event permissions
                        "emit_events" => permissions.can_emit_events,
                        "listen_to_events" => permissions.can_listen_to_events,

                        _ => false,
                    }
                } else {
                    false // No permissions set = no access
                }
            }
            _ => false,
        }
    }

    /// Enhanced permission checking with context
    pub fn check_plugin_permission_detailed(
        &self,
        plugin_id: &str,
        permission: &str,
        context: Option<&str>,
    ) -> PluginPermissionResult {
        match self.private_state.security_state.read() {
            Ok(security_state) => {
                if let Some(permissions) = security_state.plugin_permissions.get(plugin_id) {
                    match permission {
                        "access_integration" => {
                            if let Some(integration_id) = context {
                                if permissions
                                    .allowed_integrations
                                    .contains(&integration_id.to_string())
                                {
                                    PluginPermissionResult::Allowed
                                } else {
                                    PluginPermissionResult::Denied(
                                        "Integration not in allowed list".to_string(),
                                    )
                                }
                            } else {
                                PluginPermissionResult::Denied(
                                    "No integration specified".to_string(),
                                )
                            }
                        }
                        "access_domain" => {
                            if let Some(domain) = context {
                                if permissions.can_make_network_requests
                                    && (permissions.allowed_domains.is_empty()
                                        || permissions
                                            .allowed_domains
                                            .contains(&domain.to_string()))
                                {
                                    PluginPermissionResult::Allowed
                                } else {
                                    PluginPermissionResult::Denied("Domain not allowed".to_string())
                                }
                            } else {
                                PluginPermissionResult::Denied("No domain specified".to_string())
                            }
                        }
                        "access_file_path" => {
                            if let Some(path) = context {
                                if permissions.can_access_files
                                    && permissions
                                        .allowed_file_paths
                                        .iter()
                                        .any(|allowed| path.starts_with(allowed))
                                {
                                    PluginPermissionResult::Allowed
                                } else {
                                    PluginPermissionResult::Denied(
                                        "File path not allowed".to_string(),
                                    )
                                }
                            } else {
                                PluginPermissionResult::Denied("No file path specified".to_string())
                            }
                        }
                        "emit_event_type" => {
                            if let Some(event_type) = context {
                                if permissions.can_emit_events
                                    && (permissions.allowed_event_types.is_empty()
                                        || permissions
                                            .allowed_event_types
                                            .contains(&event_type.to_string()))
                                {
                                    PluginPermissionResult::Allowed
                                } else {
                                    PluginPermissionResult::Denied(
                                        "Event type not allowed".to_string(),
                                    )
                                }
                            } else {
                                PluginPermissionResult::Denied(
                                    "No event type specified".to_string(),
                                )
                            }
                        }
                        _ => {
                            if self.check_plugin_permission(plugin_id, permission) {
                                PluginPermissionResult::Allowed
                            } else {
                                PluginPermissionResult::Denied("Permission denied".to_string())
                            }
                        }
                    }
                } else {
                    PluginPermissionResult::Denied(
                        "No permissions configured for plugin".to_string(),
                    )
                }
            }
            _ => PluginPermissionResult::Denied("Security state unavailable".to_string()),
        }
    }

    // === Module State Management ===

    pub fn set_obs_status(&self, status: bool) {
        if let Ok(mut module_state) = self.public_state.module_state.write() {
            module_state.obs_status = status;
        }
    }

    pub fn get_obs_status(&self) -> bool {
        self.public_state
            .module_state
            .read()
            .map(|state| state.obs_status)
            .unwrap_or(false)
    }

    pub fn set_macro_recording(&self, recording: bool) {
        if let Ok(mut module_state) = self.public_state.module_state.write() {
            module_state.macro_recording = recording;
        }
    }

    pub fn is_macro_recording(&self) -> bool {
        self.public_state
            .module_state
            .read()
            .map(|state| state.macro_recording)
            .unwrap_or(false)
    }

    // === Security Event Logging ===

    /// Log security event with bounded collection (maintains max 1000 events)
    pub fn log_security_event(
        &self,
        event_type: String,
        plugin_id: Option<String>,
        description: String,
    ) {
        if let Ok(mut security_state) = self.private_state.security_state.write() {
            let event = SecurityEvent {
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                event_type,
                plugin_id,
                description,
            };

            // Maintain bounded collection to prevent memory growth
            if security_state.security_events.len() >= 1000 {
                security_state.security_events.pop_front(); // Remove oldest event
            }
            security_state.security_events.push_back(event);
        }
    }

    // === Feature Flag Support ===

    /// Check if feature is enabled
    pub fn is_feature_enabled(&self, feature: &str) -> bool {
        match feature {
            "new-state-system" => cfg!(feature = "new-state-system"),
            "websocket-integration" => cfg!(feature = "websocket-integration"),
            "obs-integration" => cfg!(feature = "obs-integration"),
            "plugin-system" => cfg!(feature = "plugin-system"),
            _ => false,
        }
    }

    // === Universal Feature Architecture ===

    /// Get access to the universal feature validator
    pub async fn feature_validator(&self) -> Arc<UniversalValidator> {
        self.feature_validator.read().await.clone()
    }

    // Legacy premium methods removed - use feature_validator().is_premium() directly

    /// Validate if a feature can be used
    pub async fn validate_feature<T: FeatureRequirement>(&self, feature: &T) -> FeatureValidation {
        let validator = self.feature_validator.read().await;
        validator.validate_feature(feature).await
    }

    /// Try to use a feature (validates and increments usage if successful)
    pub async fn try_use_feature<T: FeatureRequirement>(&self, feature: &T) -> FeatureValidation {
        let validator = self.feature_validator.read().await;
        let validation = validator.try_use_feature(feature).await;

        // Log feature usage attempt
        let feature_requirements = feature.get_requirements();
        if let Some(first_req) = feature_requirements.first() {
            let feature_id = match first_req {
                crate::features::PremiumFeature::UsageLimit { feature_id, .. } => feature_id,
                crate::features::PremiumFeature::FeatureFlag { feature_id, .. } => feature_id,
                crate::features::PremiumFeature::TimeLimit { feature_id, .. } => feature_id,
                crate::features::PremiumFeature::StorageLimit { feature_id, .. } => feature_id,
            };

            if validation.available {
                self.log_security_event(
                    "feature_used".to_string(),
                    None,
                    format!(
                        "Feature '{}' used successfully in {}",
                        feature_id,
                        feature.feature_context()
                    ),
                );
            } else {
                self.log_security_event(
                    "feature_blocked".to_string(),
                    None,
                    format!(
                        "Feature '{}' blocked in {}: {}",
                        feature_id,
                        feature.feature_context(),
                        validation.message.as_deref().unwrap_or("Unknown reason")
                    ),
                );
            }
        }

        validation
    }

    /// Validate storage-based feature with actual storage count (for save operations)
    pub async fn validate_storage_feature(
        &self,
        feature_id: &str,
        current_count: u32,
        context: &str,
    ) -> FeatureValidation {
        let validator = self.feature_validator.read().await;
        validator
            .validate_storage_feature(feature_id, current_count, context)
            .await
    }

    /// Register a feature configuration with the validator
    pub async fn register_feature(&self, feature: crate::features::PremiumFeature) {
        let validator = self.feature_validator.read().await;
        validator.register_feature(feature).await;
    }

    /// Get usage statistics for a specific feature
    pub async fn get_feature_usage_stats(
        &self,
        feature_id: &str,
    ) -> Option<crate::features::UsageCounter> {
        let validator = self.feature_validator.read().await;
        validator.get_usage_stats(feature_id).await
    }

    /// Get all feature usage statistics (admin/debugging)
    pub async fn get_all_feature_usage_stats(
        &self,
    ) -> std::collections::HashMap<String, crate::features::UsageCounter> {
        let validator = self.feature_validator.read().await;
        validator.get_all_usage_stats().await
    }

    /// Reset usage counter for a feature (admin function)
    pub async fn reset_feature_usage(&self, feature_id: &str) {
        let validator = self.feature_validator.read().await;
        validator.reset_usage_counter(feature_id).await;

        self.log_security_event(
            "feature_usage_reset".to_string(),
            None,
            format!("Usage counter reset for feature: {}", feature_id),
        );
    }

    /// Initialize common VortiDeck features
    pub async fn initialize_default_features(&self) {
        use crate::features::{common_features, PremiumFeature, ResetPeriod};

        // Register all common features
        let features = vec![
            // UI customization features
            PremiumFeature::UsageLimit {
                feature_id: "ui_buttons".to_string(),
                free_limit: 6,
                premium_limit: None,
                reset_period: ResetPeriod::Never,
            },
            // Macro system features
            PremiumFeature::StorageLimit {
                feature_id: "macro_recording".to_string(),
                free_limit: 10,
                premium_limit: None,
            },
            // Advanced integrations (premium only)
            PremiumFeature::FeatureFlag {
                feature_id: "advanced_integrations".to_string(),
                enabled: true,
            },
            // Plugin uploads
            PremiumFeature::UsageLimit {
                feature_id: "plugin_uploads".to_string(),
                free_limit: 5,
                premium_limit: Some(50),
                reset_period: ResetPeriod::Monthly,
            },
            // Cloud sync (premium only)
            PremiumFeature::FeatureFlag {
                feature_id: "cloud_sync".to_string(),
                enabled: true,
            },
            // Screen recording time limits
            PremiumFeature::TimeLimit {
                feature_id: "screen_recording".to_string(),
                free_limit_seconds: 30 * 60, // 30 minutes
                premium_limit_seconds: None, // Unlimited
            },
        ];

        for feature in features {
            self.register_feature(feature).await;
        }

        self.log_security_event(
            "features_initialized".to_string(),
            None,
            "Default VortiDeck features initialized".to_string(),
        );
    }

    // === Performance Monitoring and Optimization ===

    /// Track performance of a state operation
    async fn track_performance<T, F, Fut>(&self, operation_name: &str, operation: F) -> T
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        let start = Instant::now();
        let result = operation().await;
        let duration = start.elapsed();

        let mut metrics = self.performance_metrics.lock().await;
        let entry = metrics
            .entry(operation_name.to_string())
            .or_insert_with(PerformanceMetrics::default);

        entry.operation_count += 1;
        entry.total_duration += duration;
        entry.average_duration = entry.total_duration / entry.operation_count as u32;
        entry.last_operation_time = start;

        // Log slow operations (> 100ms)
        if duration > Duration::from_millis(100) {
            println!(
                "⚠️ Slow state operation '{}' took {:?}",
                operation_name, duration
            );
        }

        result
    }

    /// Get performance metrics for monitoring
    pub async fn get_state_performance_metrics(&self) -> HashMap<String, PerformanceMetrics> {
        self.performance_metrics.lock().await.clone()
    }

    /// Clear performance metrics
    pub async fn clear_performance_metrics(&self) {
        self.performance_metrics.lock().await.clear();
    }

    /// Get cached peer count with performance optimization
    pub async fn get_peer_count_cached(&self) -> usize {
        // Check cache first
        {
            let cache = self.cache.lock().await;
            if let Some(count) = cache.get_peer_count() {
                return count;
            }
        }

        // Cache miss - compute and cache
        self.track_performance("get_peer_count", || async {
            let peers = self.public_state.peer_state.read().await;
            let count = peers.active_peers.len();

            // Update cache
            {
                let mut cache = self.cache.lock().await;
                cache.set_peer_count(count);
            }

            count
        })
        .await
    }

    /// Batch operation for adding multiple peers efficiently
    pub async fn add_peers_batch(
        &self,
        peers: Vec<(String, String, MessageSender, String, String)>,
    ) {
        self.track_performance("add_peers_batch", || async {
            let mut peer_map = self.public_state.peer_state.write().await;
            let current_time = SystemTime::now();

            for (client_id, ip, sender, device_type, device) in peers {
                let peer_info = PeerInfo {
                    client_id: client_id.clone(),
                    ip: ip.parse().unwrap_or("127.0.0.1".parse().unwrap()),
                    sender: Some(sender),
                    device_type,
                    device,
                    connected_at: current_time.duration_since(UNIX_EPOCH).unwrap().as_secs(),
                    last_seen: current_time.duration_since(UNIX_EPOCH).unwrap().as_secs(),
                };

                peer_map.active_peers.insert(client_id, peer_info);
            }

            // Invalidate cache
            {
                let mut cache = self.cache.lock().await;
                cache.peer_count = None;
            }
        })
        .await;
    }

    /// Optimized server status check with caching
    pub async fn is_server_running_cached(&self) -> bool {
        // Check cache first
        {
            let cache = self.cache.lock().await;
            if let Some(status) = cache.get_server_status() {
                return status;
            }
        }

        // Cache miss - compute and cache
        self.track_performance("is_server_running", || async {
            let server_state = self.private_state.server_state.read().unwrap();
            let status = server_state.is_running;

            // Update cache
            {
                let mut cache = self.cache.lock().await;
                cache.set_server_status(status);
            }

            status
        })
        .await
    }

    /// Register a mobile client for volume updates
    pub async fn register_mobile_volume_listener(&self, client_id: String, sender: MessageSender) {
        let mut listeners = self.mobile_volume_listeners.lock().await;
        listeners.insert(client_id.clone(), sender);
        println!("📱 Registered mobile volume listener: {}", client_id);
    }

    /// Remove a mobile client from volume updates
    pub async fn remove_mobile_volume_listener(&self, client_id: &str) {
        let mut listeners = self.mobile_volume_listeners.lock().await;
        if listeners.remove(client_id).is_some() {
            println!("📱 Removed mobile volume listener: {}", client_id);
        }
    }

    /// Broadcast volume change to all registered mobile clients with performance optimization
    pub async fn broadcast_volume_change_to_mobile(
        &self,
        volume: f32,
        muted: bool,
        is_button_change: bool,
        button_id: Option<String>,
    ) {
        let listeners = self.mobile_volume_listeners.lock().await;

        if listeners.is_empty() {
            return; // Early return if no listeners
        }

        // 🚀 PERFORMANCE: Pre-serialize message once instead of per-client
        let volume_message = serde_json::json!({
            "action": "volume_changed",
            "volume": volume,
            "muted": muted,
            "payload": {
                "is_button_change": is_button_change,
                "button_id": button_id
            }
        });

        let message_str = volume_message.to_string();

        // 🚀 PERFORMANCE: Send to all clients concurrently using futures
        let send_futures: Vec<_> = listeners
            .iter()
            .map(|(client_id, sender)| {
                let message = message_str.clone();
                let client = client_id.clone();
                async move {
                    match sender.send(Message::Text(message.into())) {
                        Err(e) => {
                            println!(
                                "⚠️ Failed to send volume update to mobile client {}: {}",
                                client, e
                            );
                            Some(client) // Return client_id for cleanup
                        }
                        _ => None,
                    }
                }
            })
            .collect();

        // Execute all sends concurrently
        let results = futures::future::join_all(send_futures).await;

        // Collect failed clients for potential cleanup (optional)
        let failed_clients: Vec<_> = results.into_iter().filter_map(|r| r).collect();
        if !failed_clients.is_empty() {
            println!(
                "⚠️ {} mobile clients failed to receive volume update",
                failed_clients.len()
            );
        }

        println!(
            "📱 Broadcasted volume change to {} mobile clients: {:.1}%",
            listeners.len(),
            volume
        );
    }

    // === UNIVERSAL INTEGRATION STATUS METHODS ===

    /// Set integration status for any integration (universal method)
    pub async fn set_integration_status(
        &self,
        integration_id: String,
        status: &crate::integrations::ConnectionStatus,
    ) {
        let mut module_state = self.public_state.module_state.write().unwrap();
        module_state
            .integration_status
            .insert(integration_id.clone(), status.is_connected());

        // Also handle OBS specifically for backward compatibility
        if integration_id == "obs" {
            module_state.obs_status = status.is_connected();
        }

        println!(
            "🔄 Set integration status: {} -> {}",
            integration_id,
            status.is_connected()
        );
    }

    /// Get integration status for any integration (universal method)
    pub fn get_integration_status(&self, integration_id: &str) -> bool {
        let module_state = self.public_state.module_state.read().unwrap();

        // Handle OBS backward compatibility
        if integration_id == "obs" {
            return module_state.obs_status;
        }

        module_state
            .integration_status
            .get(integration_id)
            .copied()
            .unwrap_or(false)
    }

    /// Get all integration statuses
    pub fn get_all_integration_statuses(&self) -> HashMap<String, bool> {
        let module_state = self.public_state.module_state.read().unwrap();
        let mut all_statuses = module_state.integration_status.clone();

        // Add OBS for backward compatibility
        all_statuses.insert("obs".to_string(), module_state.obs_status);

        all_statuses
    }

    /// Remove integration status when integration is unregistered
    pub fn remove_integration_status(&self, integration_id: &str) {
        let mut module_state = self.public_state.module_state.write().unwrap();
        module_state.integration_status.remove(integration_id);

        println!("🗑️ Removed integration status: {}", integration_id);
    }

    /// Get connected integration count
    pub fn get_connected_integration_count(&self) -> usize {
        let module_state = self.public_state.module_state.read().unwrap();
        let mut count = module_state
            .integration_status
            .values()
            .filter(|&&status| status)
            .count();

        // Add OBS for backward compatibility
        if module_state.obs_status {
            count += 1;
        }

        count
    }

    /// Check if any integrations are connected
    pub fn has_connected_integrations(&self) -> bool {
        self.get_connected_integration_count() > 0
    }

    // 🎯 NEW: Banner Analytics Methods
    pub async fn add_analytics_data(
        &self,
        analytics_data: serde_json::Map<String, serde_json::Value>,
    ) {
        // Store analytics data - could be extended to use proper storage
        println!("📊 Storing analytics data: {:?}", analytics_data);
        // TODO: Implement proper analytics storage (database, etc.)
    }

    pub async fn add_batch_analytics_data(
        &self,
        batch_data: serde_json::Map<String, serde_json::Value>,
    ) {
        // Store batch analytics data - could be extended to use proper storage
        println!("📊 Storing batch analytics data: {:?}", batch_data);
        // TODO: Implement proper batch analytics storage
    }

    // 🎯 NEW: Banner Queue State Methods
    pub async fn set_banner_queue_state(&self, queue_state: serde_json::Value) {
        // Store current banner queue state
        println!("🎯 Setting banner queue state: {:?}", queue_state);
        // TODO: Implement proper banner queue state storage
    }

    pub async fn get_banner_queue_state(&self) -> Option<serde_json::Value> {
        // Retrieve current banner queue state
        println!("🎯 Getting banner queue state");
        // TODO: Implement proper banner queue state retrieval
        None
    }

    // ============================================================================
    // E2EE (End-to-End Encryption) Methods
    // ============================================================================

    /// Store pairing token and desktop public key for E2EE handshake
    pub async fn store_pairing_token(&self, pairing_token: String, desktop_public_key: String) {
        let mut auth = self.private_state.auth_state.write().await;
        auth.e2ee_pairing_keys
            .insert(pairing_token, desktop_public_key);
        println!("🔐 Stored pairing token for E2EE handshake");
    }

    /// Get desktop public key for pairing token
    pub async fn get_pairing_public_key(&self, pairing_token: &str) -> Option<String> {
        let auth = self.private_state.auth_state.read().await;
        auth.e2ee_pairing_keys.get(pairing_token).cloned()
    }

    /// Remove pairing token after successful handshake
    pub async fn remove_pairing_token(&self, pairing_token: &str) {
        let mut auth = self.private_state.auth_state.write().await;
        auth.e2ee_pairing_keys.remove(pairing_token);
        println!("🔐 Removed pairing token after handshake");
    }

    /// Store shared secret for a client after E2EE handshake
    /// 🔐 SECURITY FIX: Now includes metadata for lifecycle management
    /// Secrets persist until explicitly unpaired (no automatic expiration)
    ///
    /// Note: This only stores in memory. To persist across restarts, call
    /// `db.save_shared_secret()` from the Tauri layer after this method.
    pub async fn store_shared_secret(&self, client_id: String, shared_secret: [u8; 32]) {
        let mut auth = self.private_state.auth_state.write().await;
        let now = SystemTime::now();

        let metadata = SharedSecretMetadata {
            secret: shared_secret,
            created_at: now,
            last_used: now,
        };

        auth.e2ee_shared_secrets.insert(client_id.clone(), metadata);
        println!(
            "🔐 Stored shared secret in memory for client: {}",
            client_id
        );
        println!("💡 Remember to call db.save_shared_secret() to persist across restarts");
    }

    /// Get shared secret for HMAC signing
    /// 🔐 SECURITY FIX: Updates last_used timestamp for lifecycle tracking
    pub async fn get_shared_secret(&self, client_id: &str) -> Option<[u8; 32]> {
        // First, get the secret with read lock
        let secret = {
            let auth = self.private_state.auth_state.read().await;
            auth.e2ee_shared_secrets.get(client_id).map(|m| m.secret)
        };

        // If secret exists, update last_used timestamp with write lock
        if secret.is_some() {
            let mut auth = self.private_state.auth_state.write().await;
            if let Some(metadata) = auth.e2ee_shared_secrets.get_mut(client_id) {
                metadata.last_used = SystemTime::now();
            }
        }

        secret
    }

    /// Remove shared secret when client disconnects or unpairs
    /// 🔐 SECURITY FIX: Properly cleans up metadata and securely zeros memory
    pub async fn remove_shared_secret(&self, client_id: &str) {
        let mut auth = self.private_state.auth_state.write().await;

        if let Some(mut metadata) = auth.e2ee_shared_secrets.remove(client_id) {
            // Securely zero out the secret in memory before dropping
            metadata.secret.fill(0);
            println!(
                "🔐 Removed and zeroed shared secret for client: {}",
                client_id
            );

            // TODO: Remove from database as well
            // self.db.delete_shared_secret(client_id).await;
        } else {
            println!(
                "⚠️ Attempted to remove non-existent shared secret for client: {}",
                client_id
            );
        }
    }

    /// Check if a shared secret should be rotated (e.g., after 30 days)
    /// 🔐 SECURITY: Optional rotation policy for enhanced security
    /// Currently not enforced - secrets persist until unpaired
    pub async fn should_rotate_secret(&self, client_id: &str) -> bool {
        let auth = self.private_state.auth_state.read().await;

        if let Some(metadata) = auth.e2ee_shared_secrets.get(client_id) {
            let age = SystemTime::now()
                .duration_since(metadata.created_at)
                .unwrap_or(Duration::ZERO);

            // Recommend rotation after 30 days (optional policy)
            return age > Duration::from_secs(30 * 24 * 60 * 60);
        }

        false
    }

    /// Get shared secret metadata for debugging/monitoring
    pub async fn get_shared_secret_info(
        &self,
        client_id: &str,
    ) -> Option<(SystemTime, SystemTime)> {
        let auth = self.private_state.auth_state.read().await;
        auth.e2ee_shared_secrets
            .get(client_id)
            .map(|m| (m.created_at, m.last_used))
    }

    /// Restore shared secrets from database into memory (call on startup)
    /// 🔐 PHASE 2: Shared secrets now survive desktop restarts
    ///
    /// This should be called from the Tauri layer after loading secrets from DB:
    /// ```rust,ignore
    /// let secrets = db.load_all_shared_secrets();
    /// for (client_id, secret) in secrets {
    ///     state_manager.restore_shared_secret_from_db(client_id, secret).await;
    /// }
    /// ```
    pub async fn restore_shared_secret_from_db(&self, client_id: String, shared_secret: [u8; 32]) {
        let mut auth = self.private_state.auth_state.write().await;
        let now = SystemTime::now();

        let metadata = SharedSecretMetadata {
            secret: shared_secret,
            created_at: now, // We don't have original created_at from DB, use now
            last_used: now,
        };

        auth.e2ee_shared_secrets.insert(client_id.clone(), metadata);
        println!(
            "🔄 Restored shared secret from database for client: {}",
            client_id
        );
    }

    // ============================================================================
    // Nonce Management for Replay Attack Prevention
    // ============================================================================

    /// Atomically claim a short-lived nonce for a non-consequential message.
    /// Consequential remote commands use the durable database claim instead.
    /// Returns `true` only for the first claimant.
    pub async fn claim_transient_nonce(&self, nonce: String, client_id: String) -> bool {
        let mut security = self.private_state.security_state.write().unwrap();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if security.used_nonces.contains_key(&nonce) {
            return false;
        }

        security
            .used_nonces
            .insert(nonce.clone(), (timestamp, client_id.clone()));
        println!(
            "🔐 Marked nonce as used: {} for client: {}",
            &nonce[..8.min(nonce.len())],
            client_id
        );

        // 🔐 SECURITY FIX: Automatic cleanup on every insert to prevent memory leaks
        // Cleanup old nonces if we have more than 10000 OR periodically
        if security.used_nonces.len() > 10000 {
            let cutoff_time = timestamp - 300; // Remove nonces older than 5 minutes
            let before = security.used_nonces.len();
            security.used_nonces.retain(|_, (ts, _)| *ts > cutoff_time);
            let after = security.used_nonces.len();
            println!(
                "🧹 Auto-cleanup: Removed {} expired nonces (5+ min old), {} remaining",
                before - after,
                after
            );
        }
        true
    }

    /// Cleanup expired nonces (older than 5 minutes)
    /// 🔐 SECURITY: This should be called periodically (e.g., every minute) to prevent memory leaks
    /// Nonces are one-time-use tokens that expire after 5 minutes
    pub async fn cleanup_expired_nonces(&self) {
        let mut security = self.private_state.security_state.write().unwrap();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let cutoff_time = timestamp - 300; // 5 minutes (300 seconds)

        let before = security.used_nonces.len();
        security.used_nonces.retain(|_, (ts, _)| *ts > cutoff_time);
        let after = security.used_nonces.len();

        if before != after {
            println!(
                "🧹 Periodic cleanup: Removed {} expired nonces (5+ min old), {} remaining",
                before - after,
                after
            );
        } else if before > 0 {
            // Log even if nothing removed, to show cleanup is running
            println!(
                "🧹 Periodic cleanup: All {} nonces are still valid (< 5 min old)",
                before
            );
        }
    }

    // ============================================================================
    // Backend Relay State Management
    // ============================================================================

    /// Enable relay mode
    pub async fn set_relay_enabled(&self, enabled: bool) {
        let mut relay = self.private_state.relay_state.write().await;
        relay.enabled = enabled;
        println!("📡 Relay enabled: {}", enabled);
    }

    /// Check if relay is enabled
    pub async fn is_relay_enabled(&self) -> bool {
        let relay = self.private_state.relay_state.read().await;
        relay.enabled
    }

    /// Set relay URL
    pub async fn set_relay_url(&self, url: String) {
        let mut relay = self.private_state.relay_state.write().await;
        relay.relay_url = Some(url.clone());
        println!("📡 Relay URL set: {}", url);
    }

    /// Get relay URL
    pub async fn get_relay_url(&self) -> Option<String> {
        let relay = self.private_state.relay_state.read().await;
        relay.relay_url.clone()
    }

    /// Set desktop ID (persistent identifier for relay mode)
    pub async fn set_desktop_id(&self, id: String) {
        let mut relay = self.private_state.relay_state.write().await;
        relay.desktop_id = Some(id.clone());
        println!("📡 Desktop ID set: {}", id);
    }

    /// Get desktop ID
    pub async fn get_desktop_id(&self) -> Option<String> {
        let relay = self.private_state.relay_state.read().await;
        relay.desktop_id.clone()
    }

    // ============================================================================
    // EXECUTION SETTINGS MANAGEMENT
    // ============================================================================

    /// Load execution settings from database into memory cache
    pub async fn load_execution_settings(&self, config: &crate::db::models::AppConfig) {
        let settings = ExecutionSettings {
            max_concurrent_actions: config.max_concurrent_actions as u32,
            max_action_timeout_ms: config.max_action_timeout_ms as u64,
        };

        *self.execution_settings.lock().await = settings.clone();
        println!("✅ Loaded execution settings into memory: {:?}", settings);
    }

    /// Get current execution settings (fast memory read - no database access)
    pub async fn get_execution_settings(&self) -> ExecutionSettings {
        self.execution_settings.lock().await.clone()
    }

    /// Update execution settings in both memory cache and prepare for database update
    /// Returns the settings for the caller to persist to database
    pub async fn update_execution_settings(
        &self,
        settings: ExecutionSettings,
    ) -> ExecutionSettings {
        // Update memory cache immediately
        *self.execution_settings.lock().await = settings.clone();
        println!("✅ Updated execution settings in memory: {:?}", settings);

        // Return settings for caller to persist to DB
        settings
    }
}
