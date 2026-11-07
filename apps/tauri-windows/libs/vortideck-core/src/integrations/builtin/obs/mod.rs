use crate::core::state_manager::{ObsParameter, ObsPluginAction, StateManager};
use crate::integrations::{
    contracts::{
        ActionContract, ActionReceiptSemantics, ActionRecoverySemantics, CapabilityDefinition,
        CapabilityKind, CapabilityOption, CapabilityParameter, CapabilityPresentation,
        CapabilityWidget, ContractValueType, DynamicOptionSource,
    },
    interface::{
        IntegrationAction, IntegrationActionResult, IntegrationActionStatus, IntegrationId,
        IntegrationMetadata, IntegrationRequirements, IntegrationResult,
    },
    traits::{AppIntegration, EventCallback, IntegrationError},
};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

// Add banner-specific structures
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BannerContent {
    pub id: String,
    pub content_data: String,
    pub content_type: String, // "image", "video", "color", "html"
    pub duration_seconds: u32,
    pub campaign_id: Option<String>,
    pub priority: Option<String>,
    pub css: Option<String>,
    pub css_locked: Option<bool>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BannerQueue {
    pub banners: Vec<BannerContent>,
    pub premium_status: String,
    pub auto_rotation: bool,
    pub batch_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AnalyticsData {
    pub ad_id: String,
    pub campaign_id: Option<String>,
    pub display_count: u32,
    pub display_duration_ms: u64,
    pub planned_duration_ms: u64,
    pub completion_percentage: f64,
    pub viewer_count: u32,
    pub premium_status: String,
    pub revenue_share: f64,
    pub timestamp: String,
}

#[derive(Clone)]
pub struct ObsIntegration {
    id: IntegrationId,
    name: String,
    description: String,
    state_manager: Arc<StateManager>,
}

impl ObsIntegration {
    pub fn new(state_manager: Arc<StateManager>) -> Self {
        Self {
            id: IntegrationId::new("obs"),
            name: "OBS Studio".to_string(),
            description: "Integration with OBS Studio for stream control".to_string(),
            state_manager,
        }
    }

    /// Operational semantics used by both dispatch and workflow recovery. OBS
    /// owns this policy; the generic flow executor must not classify connector
    /// actions by name itself.
    pub fn operational_contract(
        action_id: &str,
        parameters: &HashMap<String, Value>,
        receipt: ActionReceiptSemantics,
    ) -> ActionContract {
        let action = action_id.to_ascii_lowercase();
        let non_convergent = [
            "toggle",
            "next",
            "previous",
            "increment",
            "decrement",
            "add_to_queue",
        ]
        .iter()
        .any(|fragment| action.contains(fragment));
        let convergent = !non_convergent
            && (action.starts_with("set_")
                || action.starts_with("obs_set_")
                || action.contains("show")
                || action.contains("hide")
                || action.contains("mute")
                || action.starts_with("start_")
                || action.starts_with("stop_")
                || action.starts_with("obs_stream_")
                || action.starts_with("obs_recording_")
                || action == "obs_scene_activate"
                || action == "obs_source_activate");

        let value = |names: &[&str]| {
            names.iter().find_map(|name| {
                parameters
                    .get(*name)
                    .and_then(Value::as_str)
                    .filter(|value| !value.is_empty())
            })
        };
        let resource_key = if action.contains("record") {
            "obs:recording".to_string()
        } else if action.contains("stream") {
            "obs:streaming".to_string()
        } else if action.contains("set_scene")
            || action.contains("program_scene")
            || action == "obs_scene_activate"
        {
            "obs:program-scene".to_string()
        } else if let Some(source) = value(&[
            "sourceName",
            "source_name",
            "source",
            "inputName",
            "input_name",
            "APPLET_OBS_SOURCE_SELECTION",
        ]) {
            let scene = value(&[
                "sceneName",
                "scene_name",
                "scene",
                "APPLET_OBS_SCENE_SELECTION",
            ])
            .unwrap_or("*");
            format!("obs:scene:{}:source:{}", scene, source)
        } else if action.contains("banner") || action.contains("overlay") {
            let instance = value(&["id", "bannerId", "overlayId"]).unwrap_or("*");
            format!("obs:visual:{}", instance)
        } else {
            format!("obs:action:{}", action_id)
        };

        ActionContract {
            integration_id: "obs".into(),
            action_id: action_id.into(),
            version: 1,
            required_permission: crate::plugins::bundle::PluginPermission::TriggerAction(format!(
                "integration:obs:{}",
                action_id
            )),
            timeout_ms: 15_000,
            recovery: if convergent {
                ActionRecoverySemantics::Reconcile
            } else {
                ActionRecoverySemantics::FailClosed
            },
            resource_key: Some(resource_key),
            receipt,
        }
    }

    /// Resolve the live OBS capability. The companion catalog supplies the UI
    /// schema while VortiDeck supplies operational reliability semantics.
    pub async fn resolve_capability_definition(
        &self,
        action_id: &str,
        parameters: &HashMap<String, Value>,
    ) -> IntegrationResult<CapabilityDefinition> {
        let registered = self
            .state_manager
            .get_obs_actions()
            .await
            .unwrap_or_default();
        let contract = Self::operational_contract(
            action_id,
            parameters,
            if self.state_manager.obs_action_receipts_supported() {
                ActionReceiptSemantics::Applied { timeout_ms: 15_000 }
            } else {
                ActionReceiptSemantics::TransportAccepted
            },
        );

        let capability = if let Some(action) = registered
            .iter()
            .find(|action| action.action_id == action_id)
        {
            self.capability_from_obs_action(action, contract)
        } else if let Some(capability) = self
            .fallback_capabilities()
            .into_iter()
            .find(|capability| capability.capability_id == action_id)
        {
            CapabilityDefinition {
                contract,
                ..capability
            }
        } else {
            return Err(IntegrationError::NotFound(format!(
                "OBS action '{}' is not registered by the connected companion",
                action_id
            )));
        };

        capability
            .validate_parameters(parameters)
            .map_err(IntegrationError::ValidationError)?;
        Ok(capability)
    }

    fn contract_value_type(parameter_type: &str) -> ContractValueType {
        match parameter_type.to_ascii_lowercase().as_str() {
            "boolean" | "bool" | "checkbox" => ContractValueType::Boolean,
            "number" | "float" | "double" | "slider" => ContractValueType::Number,
            "integer" | "int" => ContractValueType::Integer,
            "object" => ContractValueType::Object,
            "array" => ContractValueType::Array,
            // The OBS companion uses LIST for a single selected string; the
            // allowed choices are carried separately in listSelection.
            "string" | "text" | "select" | "selection" | "list" => ContractValueType::String,
            _ => ContractValueType::Any,
        }
    }

    /// 🚀 NEW ARCHITECTURE: Centralized OBS plugin detection and setup
    /// This replaces all the scattered OBS detection logic in main.rs
    pub async fn handle_obs_message<M: Clone + Send + Sync>(
        &self,
        message: &Value,
        tx: &broadcast::Sender<M>,
        emit_event: &(dyn Fn(
            &str,
            &str,
            serde_json::Value,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
              + Send
              + Sync),
    ) -> bool {
        // Check for OBS-specific message patterns
        if let Some(path) = message["path"].as_str() {
            match path {
                "/api/v1/integration/sdk/action/result"
                | "/api/v1/integration/sdk/action/receipt" => {
                    let payload = message.get("payload").unwrap_or(message);
                    let request_id = payload
                        .get("requestId")
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    if request_id.is_empty() {
                        log::warn!("Ignoring OBS action receipt without requestId");
                    } else {
                        let succeeded = payload
                            .get("success")
                            .and_then(Value::as_bool)
                            .unwrap_or(false);
                        let outcome = if succeeded {
                            Ok(())
                        } else {
                            Err(payload
                                .get("error")
                                .and_then(Value::as_str)
                                .unwrap_or("OBS action failed without an error message")
                                .to_string())
                        };
                        if !self
                            .state_manager
                            .resolve_obs_action_receipt(request_id, outcome)
                            .await
                        {
                            log::warn!(
                                "Ignoring unknown or late OBS action receipt {}",
                                request_id
                            );
                        }
                    }
                    return true;
                }
                "/api/v1/integration/obs/status" => {
                    // println!("🔍 OBS plugin status message received - setting status to true");
                    // Status messages should also indicate OBS is connected, especially after app restart
                    self.setup_obs_integration(tx, emit_event, "status message")
                        .await;
                    return true;
                }
                "/api/v1/actions/register" => {
                    // println!("🔍 OBS plugin action registration received - setting status to true");
                    self.setup_obs_integration(tx, emit_event, "action registration")
                        .await;

                    // Handle action registration
                    if let Some(payload) = message["payload"].as_object() {
                        self.state_manager.set_obs_action_receipts_supported(
                            payload
                                .get("supportsActionReceipts")
                                .and_then(Value::as_bool)
                                .unwrap_or(false),
                        );
                        if let Some(actions) = payload["actions"].as_array() {
                            let registration =
                                if payload.get("registrationMode").and_then(Value::as_str)
                                    == Some("merge")
                                {
                                    self.state_manager.handle_obs_action_register(actions).await
                                } else {
                                    self.state_manager.replace_obs_action_catalog(actions).await
                                };
                            if let Err(e) = registration {
                                println!("Failed to handle OBS actions: {}", e);
                            }
                            let obs_actions = self.state_manager.get_obs_actions().await;
                            let _ = emit_event(
                                "main",
                                "obs_register_action",
                                serde_json::json!(obs_actions),
                            );
                        }
                    }
                    return true;
                }
                "/api/v1/integration/obs/analytics/ad-display" => {
                    // Handle OBS analytics ad display event
                    self.setup_obs_integration(tx, emit_event, "analytics ad-display")
                        .await;

                    // Read and print the analytics message data nicely, recursively, without hardcoding
                    fn print_json_value(key: &str, value: &serde_json::Value, indent: usize) {
                        let indent_str = "  ".repeat(indent);
                        match value {
                            serde_json::Value::Object(map) => {
                                if !key.is_empty() {
                                    println!("{}{}:", indent_str, key);
                                }
                                for (k, v) in map {
                                    print_json_value(k, v, indent + 1);
                                }
                            }
                            serde_json::Value::Array(arr) => {
                                println!("{}{}: [", indent_str, key);
                                for v in arr {
                                    print_json_value("", v, indent + 1);
                                }
                                println!("{}]", indent_str);
                            }
                            _ => {
                                if !key.is_empty() {
                                    println!("{}{}: {}", indent_str, key, value);
                                } else {
                                    println!("{}{}", indent_str, value);
                                }
                            }
                        }
                    }

                    if let Some(payload) = message.get("payload") {
                        println!("--- OBS Analytics Ad Display Event ---");
                        print_json_value("", payload, 0);
                        println!("--------------------------------------");
                    } else {
                        println!("OBS Analytics Ad Display Event: No payload found.");
                    }

                    return true;
                }
                _ => {}
            }
        }

        false
    }

    /// 🎯 NEW: Handle banner-specific OBS messages
    pub async fn handle_banner_message<M: Clone + Send + Sync>(
        &self,
        message: &Value,
        tx: &broadcast::Sender<M>,
        emit_event: &(dyn Fn(
            &str,
            &str,
            serde_json::Value,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
              + Send
              + Sync),
    ) -> bool {
        // Check for banner-specific message patterns
        if let Some(path) = message["path"].as_str() {
            match path {
                "/api/v1/integration/obs/banner/set-data" => {
                    self.handle_banner_set_data(message, tx, emit_event).await;
                    return true;
                }
                "/api/v1/integration/obs/banner/set-queue" => {
                    self.handle_banner_set_queue(message, tx, emit_event).await;
                    return true;
                }
                "/api/v1/integration/obs/banner/add-to-queue" => {
                    self.handle_banner_add_to_queue(message, tx, emit_event)
                        .await;
                    return true;
                }
                "/api/v1/integration/obs/banner/hide" => {
                    self.handle_banner_hide(message, tx, emit_event).await;
                    return true;
                }
                "/api/v1/integration/obs/banner/show" => {
                    self.handle_banner_show(message, tx, emit_event).await;
                    return true;
                }
                "/api/v1/integration/obs/analytics/ad-display" => {
                    self.handle_analytics_report(message, tx, emit_event).await;
                    return true;
                }
                "/api/v1/integration/obs/analytics/batch-report" => {
                    self.handle_batch_analytics_report(message, tx, emit_event)
                        .await;
                    return true;
                }
                _ => {}
            }
        }

        // Check for banner action IDs
        if let Some(action_id) = message["actionId"].as_str() {
            match action_id {
                "obs_banner_set_data" => {
                    self.handle_banner_action_set_data(message, tx, emit_event)
                        .await;
                    return true;
                }
                "obs_banner_set_queue" => {
                    self.handle_banner_action_set_queue(message, tx, emit_event)
                        .await;
                    return true;
                }
                "obs_banner_add_to_queue" => {
                    self.handle_banner_action_add_to_queue(message, tx, emit_event)
                        .await;
                    return true;
                }
                "obs_banner_hide" => {
                    self.handle_banner_action_hide(message, tx, emit_event)
                        .await;
                    return true;
                }
                "obs_banner_show" => {
                    self.handle_banner_action_show(message, tx, emit_event)
                        .await;
                    return true;
                }
                "obs_analytics_enable" => {
                    self.handle_analytics_enable(message, tx, emit_event).await;
                    return true;
                }
                "obs_analytics_get_report" => {
                    self.handle_analytics_get_report(message, tx, emit_event)
                        .await;
                    return true;
                }
                _ => {}
            }
        }

        false
    }

    /// 🚀 NEW ARCHITECTURE: Single source of truth for OBS integration setup
    async fn setup_obs_integration<M: Clone + Send + Sync>(
        &self,
        tx: &broadcast::Sender<M>,
        emit_event: &(dyn Fn(
            &str,
            &str,
            serde_json::Value,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
              + Send
              + Sync),
        detection_method: &str,
    ) {
        // Check if this is a new OBS connection (no active sender means new connection)
        let is_obs_connected = self.state_manager.get_obs_plugin_sender().await.is_some();

        if !self.state_manager.get_obs_status() {
            // println!("🔍 OBS plugin detected via {} - setting up OBS integration", detection_method);

            // Set OBS status to true
            self.state_manager.set_obs_status(true);
            let _ = emit_event("main", "obs_plugin_status", serde_json::json!(true));
            // println!("🔧 DEBUG: Set OBS status to true during {}", detection_method);

            // Set the last status time to prevent check_obs_status from resetting it
            use std::time::{SystemTime, UNIX_EPOCH};
            let current_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            self.state_manager
                .set_obs_last_status_time(current_time)
                .await;

            // 🎯 Start automatic banner rotation when OBS connects
            if is_obs_connected {
                println!("🎯 OBS connected - starting banner system");
                // Note: Banner system will be started from the Tauri layer
                // using the banners::auto_rotation module
            }

            // println!("✅ OBS plugin integration activated via {}", detection_method);
        } else {
            // OBS plugin is already detected, just update status
            self.state_manager.set_obs_status(true);
            let _ = emit_event("main", "obs_plugin_status", serde_json::json!(true));
            // println!("🔧 DEBUG: Updated OBS status to true via {}", detection_method);

            // Update the last status time to keep it fresh
            use std::time::{SystemTime, UNIX_EPOCH};
            let current_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            self.state_manager
                .set_obs_last_status_time(current_time)
                .await;
        }
    }

    /// Handle OBS plugin disconnection
    pub async fn handle_disconnection(
        &self,
        emit_event: &(dyn Fn(
            &str,
            &str,
            serde_json::Value,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
              + Send
              + Sync),
    ) {
        // println!("[OBS] OBS plugin disconnected, setting status to false");
        self.state_manager
            .disconnect_obs_companion("OBS companion disconnected")
            .await;
        let _ = emit_event("main", "obs_plugin_status", serde_json::json!(false));
    }

    /// Handle initial OBS plugin connection
    pub async fn handle_initial_connection<M: Clone + Send + Sync>(
        &self,
        tx: &broadcast::Sender<M>,
        emit_event: &(dyn Fn(
            &str,
            &str,
            serde_json::Value,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
              + Send
              + Sync),
    ) -> (String, String) {
        // OBS sender setup will be handled by the platform-specific code
        // This is just for activation and GUID generation

        // Activate the OBS plugin
        let (integration_guid, instance_guid) = self.activate().await.unwrap_or_else(|_| {
            // Fallback to generated GUIDs if activation fails
            let integration_guid = format!("obs_plugin_{}", uuid::Uuid::new_v4());
            let instance_guid = format!("obs_plugin_{}", uuid::Uuid::new_v4());
            (integration_guid, instance_guid)
        });

        // Set OBS status to true when plugin connects
        self.state_manager.set_obs_status(true);
        let _ = emit_event("main", "obs_plugin_status", serde_json::json!(true));
        // println!("🔧 DEBUG: Set OBS status to true during initial connection");

        // Store GUIDs in StateManager
        self.state_manager
            .set_obs_integration_guid(integration_guid.clone());
        self.state_manager
            .set_obs_instance_guid(instance_guid.clone());

        (integration_guid, instance_guid)
    }

    pub async fn activate(&self) -> Result<(String, String), IntegrationError> {
        // Generate GUIDs
        let integration_guid = format!("obs_plugin_{}", uuid::Uuid::new_v4());
        let instance_guid = format!("obs_plugin_{}", uuid::Uuid::new_v4());
        // Store in StateManager
        self.state_manager
            .set_obs_integration_guid(integration_guid.clone());
        self.state_manager
            .set_obs_instance_guid(instance_guid.clone());
        // Build activation message
        let activate_message = json!({
            "path": "/api/v1/integration/activate",
            "verb": "SET",
            "payload": {
                "integrationGuid": integration_guid,
                "instanceGuid": instance_guid
            }
        });
        let message_str = activate_message.to_string();
        println!("[OBS Integration] Activating OBS plugin: {}", message_str);
        // Send activation message
        match self.state_manager.get_obs_plugin_sender().await {
            Some(sender) => match sender.send(Message::Text(message_str.into())) {
                Err(e) => {
                    println!("[OBS Integration] Failed to activate OBS plugin: {}", e);
                    return Err(IntegrationError::ExecutionError(e.to_string()));
                }
                _ => {
                    println!("[OBS Integration] OBS plugin activated successfully");
                    self.state_manager.set_obs_status(true);
                    return Ok((integration_guid, instance_guid));
                }
            },
            _ => {
                println!("[OBS Integration] No OBS plugin sender available for activation");
                return Err(IntegrationError::NotConnected(
                    "No OBS plugin sender".to_string(),
                ));
            }
        }
    }

    // 🎯 Banner Action Handlers
    async fn handle_banner_action_set_data<M: Clone + Send + Sync>(
        &self,
        message: &Value,
        tx: &broadcast::Sender<M>,
        emit_event: &(dyn Fn(
            &str,
            &str,
            serde_json::Value,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
              + Send
              + Sync),
    ) {
        if let Some(obs_plugin_tx) = self.state_manager.get_obs_plugin_sender().await {
            let response = json!({
                "path": "/api/v1/integration/obs/banner/set-data",
                "verb": "SET",
                "payload": {
                    "content_data": message["content_data"].as_str().unwrap_or(""),
                    "content_type": message["content_type"].as_str().unwrap_or("image"),
                    "duration_seconds": message["duration_seconds"].as_u64().unwrap_or(30),
                    "premium_status": message["premium_status"].as_str().unwrap_or("false"),
                    "interrupt_current": message["interrupt_current"].as_bool().unwrap_or(false),
                    "css": message["css"].as_str(),
                    "css_locked": message["css_locked"].as_bool().unwrap_or(false)
                }
            });

            match obs_plugin_tx.send(tokio_tungstenite::tungstenite::Message::Text(
                response.to_string().into(),
            )) {
                Err(e) => {
                    println!("❌ Failed to send banner set data to OBS plugin: {}", e);
                }
                _ => {
                    println!("✅ Sent banner set data to OBS plugin");
                    let _ = emit_event("main", "obs_banner_data_set", json!({"success": true}));
                }
            }
        }
    }

    async fn handle_banner_action_set_queue<M: Clone + Send + Sync>(
        &self,
        message: &Value,
        tx: &broadcast::Sender<M>,
        emit_event: &(dyn Fn(
            &str,
            &str,
            serde_json::Value,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
              + Send
              + Sync),
    ) {
        if let Some(obs_plugin_tx) = self.state_manager.get_obs_plugin_sender().await {
            let response = json!({
                "path": "/api/v1/integration/obs/banner/set-queue",
                "verb": "SET",
                "payload": {
                    "banners": message["banners"].clone(),
                    "premium_status": message["premium_status"].as_str().unwrap_or("false"),
                    "auto_rotation": message["auto_rotation"].as_bool().unwrap_or(true),
                    "batch_id": message["batch_id"].as_str().unwrap_or(&Uuid::new_v4().to_string())
                }
            });

            match obs_plugin_tx.send(tokio_tungstenite::tungstenite::Message::Text(
                response.to_string().into(),
            )) {
                Err(e) => {
                    println!("❌ Failed to send banner queue to OBS plugin: {}", e);
                }
                _ => {
                    println!("✅ Sent banner queue to OBS plugin");
                    let _ = emit_event("main", "obs_banner_queue_set", json!({"success": true}));
                }
            }
        }
    }

    async fn handle_banner_action_add_to_queue<M: Clone + Send + Sync>(
        &self,
        message: &Value,
        tx: &broadcast::Sender<M>,
        emit_event: &(dyn Fn(
            &str,
            &str,
            serde_json::Value,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
              + Send
              + Sync),
    ) {
        if let Some(obs_plugin_tx) = self.state_manager.get_obs_plugin_sender().await {
            let response = json!({
                "path": "/api/v1/integration/obs/banner/add-to-queue",
                "verb": "SET",
                "payload": {
                    "banners": message["banners"].clone(),
                    "premium_status": message["premium_status"].as_str().unwrap_or("false"),
                    "append_to_existing": message["append_to_existing"].as_bool().unwrap_or(true)
                }
            });

            match obs_plugin_tx.send(tokio_tungstenite::tungstenite::Message::Text(
                response.to_string().into(),
            )) {
                Err(e) => {
                    println!("❌ Failed to add banners to OBS plugin queue: {}", e);
                }
                _ => {
                    println!("✅ Added banners to OBS plugin queue");
                    let _ =
                        emit_event("main", "obs_banner_queue_updated", json!({"success": true}));
                }
            }
        }
    }

    async fn handle_banner_action_hide<M: Clone + Send + Sync>(
        &self,
        message: &Value,
        tx: &broadcast::Sender<M>,
        emit_event: &(dyn Fn(
            &str,
            &str,
            serde_json::Value,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
              + Send
              + Sync),
    ) {
        if let Some(obs_plugin_tx) = self.state_manager.get_obs_plugin_sender().await {
            let response = json!({
                "path": "/api/v1/integration/obs/banner/hide",
                "verb": "SET",
                "payload": {
                    "banner_id": message["banner_id"].as_str(),
                    "fade_out": message["fade_out"].as_bool().unwrap_or(true),
                    "fade_duration_ms": message["fade_duration_ms"].as_u64().unwrap_or(500)
                }
            });

            match obs_plugin_tx.send(tokio_tungstenite::tungstenite::Message::Text(
                response.to_string().into(),
            )) {
                Err(e) => {
                    println!("❌ Failed to hide banner in OBS plugin: {}", e);
                }
                _ => {
                    println!("✅ Sent banner hide command to OBS plugin");
                    let _ = emit_event("main", "obs_banner_hidden", json!({"success": true}));
                }
            }
        }
    }

    async fn handle_banner_action_show<M: Clone + Send + Sync>(
        &self,
        message: &Value,
        tx: &broadcast::Sender<M>,
        emit_event: &(dyn Fn(
            &str,
            &str,
            serde_json::Value,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
              + Send
              + Sync),
    ) {
        if let Some(obs_plugin_tx) = self.state_manager.get_obs_plugin_sender().await {
            let response = json!({
                "path": "/api/v1/integration/obs/banner/show",
                "verb": "SET",
                "payload": {
                    "banner_id": message["banner_id"].as_str(),
                    "fade_in": message["fade_in"].as_bool().unwrap_or(true),
                    "fade_duration_ms": message["fade_duration_ms"].as_u64().unwrap_or(500)
                }
            });

            match obs_plugin_tx.send(tokio_tungstenite::tungstenite::Message::Text(
                response.to_string().into(),
            )) {
                Err(e) => {
                    println!("❌ Failed to show banner in OBS plugin: {}", e);
                }
                _ => {
                    println!("✅ Sent banner show command to OBS plugin");
                    let _ = emit_event("main", "obs_banner_shown", json!({"success": true}));
                }
            }
        }
    }

    // 📊 Analytics Handlers
    async fn handle_analytics_enable<M: Clone + Send + Sync>(
        &self,
        message: &Value,
        tx: &broadcast::Sender<M>,
        emit_event: &(dyn Fn(
            &str,
            &str,
            serde_json::Value,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
              + Send
              + Sync),
    ) {
        if let Some(obs_plugin_tx) = self.state_manager.get_obs_plugin_sender().await {
            let response = json!({
                "path": "/api/v1/integration/obs/analytics/enable",
                "verb": "SET",
                "payload": {
                    "tracking_level": message["parameters"]["tracking_level"].as_str().unwrap_or("detailed"),
                    "report_interval_seconds": message["parameters"]["report_interval_seconds"].as_u64().unwrap_or(60),
                    "include_viewer_metrics": message["parameters"]["include_viewer_metrics"].as_bool().unwrap_or(true),
                    "include_revenue_attribution": message["parameters"]["include_revenue_attribution"].as_bool().unwrap_or(true)
                }
            });

            match obs_plugin_tx.send(tokio_tungstenite::tungstenite::Message::Text(
                response.to_string().into(),
            )) {
                Err(e) => {
                    println!("❌ Failed to enable analytics in OBS plugin: {}", e);
                }
                _ => {
                    println!("✅ Enabled analytics in OBS plugin");
                    let _ = emit_event("main", "obs_analytics_enabled", json!({"success": true}));
                }
            }
        }
    }

    async fn handle_analytics_get_report<M: Clone + Send + Sync>(
        &self,
        message: &Value,
        tx: &broadcast::Sender<M>,
        emit_event: &(dyn Fn(
            &str,
            &str,
            serde_json::Value,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
              + Send
              + Sync),
    ) {
        if let Some(obs_plugin_tx) = self.state_manager.get_obs_plugin_sender().await {
            let response = json!({
                "path": "/api/v1/integration/obs/analytics/get-report",
                "verb": "GET",
                "payload": {
                    "report_type": message["parameters"]["report_type"].as_str().unwrap_or("campaign_performance"),
                    "time_range": message["parameters"]["time_range"].as_str().unwrap_or("last_24_hours"),
                    "include_revenue": message["parameters"]["include_revenue"].as_bool().unwrap_or(true)
                }
            });

            match obs_plugin_tx.send(tokio_tungstenite::tungstenite::Message::Text(
                response.to_string().into(),
            )) {
                Err(e) => {
                    println!(
                        "❌ Failed to request analytics report from OBS plugin: {}",
                        e
                    );
                }
                _ => {
                    println!("✅ Requested analytics report from OBS plugin");
                    let _ = emit_event(
                        "main",
                        "obs_analytics_report_requested",
                        json!({"success": true}),
                    );
                }
            }
        }
    }

    // 📊 Analytics Response Handlers
    async fn handle_analytics_report<M: Clone + Send + Sync>(
        &self,
        message: &Value,
        tx: &broadcast::Sender<M>,
        emit_event: &(dyn Fn(
            &str,
            &str,
            serde_json::Value,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
              + Send
              + Sync),
    ) {
        if let Some(payload) = message["payload"].as_object() {
            // Store analytics data in StateManager
            self.state_manager.add_analytics_data(payload.clone()).await;

            // Emit to frontend
            let _ = emit_event("main", "obs_analytics_received", json!(payload));

            println!(
                "📊 Received analytics report: ad_id={}, revenue={}",
                payload
                    .get("ad_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown"),
                payload
                    .get("estimated_revenue")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0)
            );
        }
    }

    async fn handle_batch_analytics_report<M: Clone + Send + Sync>(
        &self,
        message: &Value,
        tx: &broadcast::Sender<M>,
        emit_event: &(dyn Fn(
            &str,
            &str,
            serde_json::Value,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
              + Send
              + Sync),
    ) {
        if let Some(payload) = message["payload"].as_object() {
            // Store batch analytics data in StateManager
            self.state_manager
                .add_batch_analytics_data(payload.clone())
                .await;

            // Emit to frontend
            let _ = emit_event("main", "obs_batch_analytics_received", json!(payload));

            println!(
                "📊 Received batch analytics report: total_ads={}, revenue={}",
                payload
                    .get("total_ads_displayed")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0),
                payload
                    .get("revenue_generated")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0)
            );
        }
    }

    // 🎯 PATH-based handlers (for direct plugin messages)
    async fn handle_banner_set_data<M: Clone + Send + Sync>(
        &self,
        message: &Value,
        tx: &broadcast::Sender<M>,
        emit_event: &(dyn Fn(
            &str,
            &str,
            serde_json::Value,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
              + Send
              + Sync),
    ) {
        // Handle direct plugin messages for banner data
        if let Some(payload) = message["payload"].as_object() {
            let _ = emit_event("main", "obs_banner_data_received", json!(payload));
            println!("🎯 Received banner data from OBS plugin");
        }
    }

    async fn handle_banner_set_queue<M: Clone + Send + Sync>(
        &self,
        message: &Value,
        tx: &broadcast::Sender<M>,
        emit_event: &(dyn Fn(
            &str,
            &str,
            serde_json::Value,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
              + Send
              + Sync),
    ) {
        // Handle direct plugin messages for banner queue
        if let Some(payload) = message["payload"].as_object() {
            let _ = emit_event("main", "obs_banner_queue_received", json!(payload));
            println!("🎯 Received banner queue from OBS plugin");
        }
    }

    async fn handle_banner_add_to_queue<M: Clone + Send + Sync>(
        &self,
        message: &Value,
        tx: &broadcast::Sender<M>,
        emit_event: &(dyn Fn(
            &str,
            &str,
            serde_json::Value,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
              + Send
              + Sync),
    ) {
        // Handle direct plugin messages for adding to banner queue
        if let Some(payload) = message["payload"].as_object() {
            let _ = emit_event("main", "obs_banner_queue_updated_received", json!(payload));
            println!("🎯 Received banner queue update from OBS plugin");
        }
    }

    async fn handle_banner_hide<M: Clone + Send + Sync>(
        &self,
        message: &Value,
        tx: &broadcast::Sender<M>,
        emit_event: &(dyn Fn(
            &str,
            &str,
            serde_json::Value,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
              + Send
              + Sync),
    ) {
        // Handle direct plugin messages for banner hide
        if let Some(payload) = message["payload"].as_object() {
            let _ = emit_event("main", "obs_banner_hidden_received", json!(payload));
            println!("🎯 Received banner hide confirmation from OBS plugin");
        }
    }

    async fn handle_banner_show<M: Clone + Send + Sync>(
        &self,
        message: &Value,
        tx: &broadcast::Sender<M>,
        emit_event: &(dyn Fn(
            &str,
            &str,
            serde_json::Value,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
              + Send
              + Sync),
    ) {
        // Handle direct plugin messages for banner show
        if let Some(payload) = message["payload"].as_object() {
            let _ = emit_event("main", "obs_banner_shown_received", json!(payload));
            println!("🎯 Received banner show confirmation from OBS plugin");
        }
    }
}

#[cfg(test)]
mod catalog_tests {
    use super::*;

    #[tokio::test]
    async fn disconnected_fallback_uses_canonical_saved_parameter_ids() {
        let integration = ObsIntegration::new(Arc::new(StateManager::new()));
        let parameters =
            HashMap::from([("scene_name".to_string(), Value::String("Scene".to_string()))]);

        let capability = integration
            .resolve_capability_definition("obs_scene_activate", &parameters)
            .await
            .expect("a saved scene action should remain structurally valid while disconnected");

        assert_eq!(capability.inputs[0].id, "scene_name");
        assert!(capability.inputs[0].required);
    }

    #[tokio::test]
    async fn registration_replaces_stale_catalog_and_preserves_nested_choices() {
        let state_manager = Arc::new(StateManager::new());
        let integration = ObsIntegration::new(state_manager.clone());
        let first_snapshot = vec![json!({
            "actionId": "obs_scene_activate",
            "name": "Activate scene",
            "parameters": [{
                "assignmentMessage": "Select Scene",
                "errorMessage": "Scene is required",
                "parameterId": "scene_name",
                "parameterType": "LIST",
                "required": true,
                "listSelection": [{
                    "displayName": "Main",
                    "value": "Main",
                    "parameters": [{
                        "assignmentMessage": "Select Source",
                        "errorMessage": "Source is required",
                        "parameterId": "source_name",
                        "parameterType": "LIST",
                        "required": true,
                        "listSelection": [{ "displayName": "Camera", "value": "Camera" }]
                    }]
                }]
            }]
        })];

        state_manager
            .replace_obs_action_catalog(&first_snapshot)
            .await
            .expect("first OBS catalog should register");

        let capabilities = integration.get_capabilities().await;
        let scene = &capabilities[0].inputs[0].options;
        let nested = &scene[0].parameters;
        assert_eq!(nested[0].id, "source_name");
        assert_eq!(nested[0].options[0].value, "Camera");

        state_manager
            .replace_obs_action_catalog(&[json!({
                "actionId": "obs_stream_start",
                "name": "Start stream",
                "parameters": []
            })])
            .await
            .expect("replacement OBS catalog should register");

        let replaced = state_manager.get_obs_actions().await.unwrap();
        assert_eq!(replaced.len(), 1);
        assert_eq!(replaced[0].action_id, "obs_stream_start");

        state_manager
            .disconnect_obs_companion("test disconnect")
            .await;
        assert!(state_manager.get_obs_actions().await.unwrap().is_empty());
    }
}

#[async_trait::async_trait]
impl AppIntegration for ObsIntegration {
    fn id(&self) -> &IntegrationId {
        &self.id
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn get_metadata(&self) -> IntegrationMetadata {
        IntegrationMetadata {
            id: self.id.clone(),
            name: self.name.clone(),
            version: "1.0.0".to_string(),
            description: self.description.clone(),
            author: "VortiDeck".to_string(),
            website: Some("https://obsproject.com".to_string()),
        }
    }
    fn get_requirements(&self) -> IntegrationRequirements {
        IntegrationRequirements {
            min_version: Some("28.0.0".to_string()),
            max_version: None,
            dependencies: vec![],
        }
    }
    async fn is_available(&self) -> bool {
        self.state_manager.get_obs_status()
    }
    async fn execute_action(
        &self,
        action: IntegrationAction,
    ) -> IntegrationResult<IntegrationActionResult> {
        let capability = self
            .resolve_capability_definition(&action.name, &action.parameters)
            .await?;
        let contract = capability.contract;
        // Check if OBS integration is activated, if not, activate it first
        let integration_guid = match self.state_manager.get_obs_integration_guid() {
            Some(guid) => guid,
            None => {
                // Integration not activated, activate it first
                println!("[OBS Integration] Integration not activated, activating now...");
                match self.activate().await {
                    Ok((guid, _)) => guid,
                    Err(e) => {
                        println!("[OBS Integration] Failed to activate integration: {:?}", e);
                        return Err(e);
                    }
                }
            }
        };

        let execution = action.execution.unwrap_or_default();
        let request_id = execution
            .request_id
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let receipt = match contract.receipt {
            ActionReceiptSemantics::Applied { .. } => Some(
                self.state_manager
                    .register_obs_action_receipt(request_id.clone())
                    .await,
            ),
            ActionReceiptSemantics::TransportAccepted => None,
        };

        // Use the real action name and parameters. Correlation metadata is kept
        // outside the action parameter map so it cannot collide with connector
        // input fields.
        let parameters = action.parameters;
        let invoke_message = json!({
            "path": "/api/v1/integration/sdk/action/invoke",
            "verb": "BROADCAST",
            "payload": {
                "actionId": action.name, // real action name
                "integrationGuid": integration_guid,
                "parameters": parameters,
                "requestId": request_id,
                "idempotencyKey": execution.idempotency_key,
                "contractVersion": contract.version,
            }
        });
        let message_str = invoke_message.to_string();
        println!(
            "[OBS Integration] Sending OBS action message: {}",
            message_str
        );
        // Send through the StateManager's OBS plugin sender
        match self.state_manager.get_obs_plugin_sender().await {
            Some(sender) => {
                match sender.send(Message::Text(message_str.into())) {
                    Err(e) => {
                        self.state_manager
                            .cancel_obs_action_receipt(&request_id)
                            .await;
                        println!("[OBS Integration] Failed to send OBS action: {}", e);
                        // 🚀 NEW ARCHITECTURE: Don't set status to false on action failure (like legacy)
                        // Only set to false when plugin actually disconnects
                        return Err(IntegrationError::ExecutionError(e.to_string()));
                    }
                    _ => {
                        let Some(receipt) = receipt else {
                            log::warn!(
                        "OBS companion accepted action {} via legacy transport without an applied receipt",
                        contract.action_id
                    );
                            return Ok(IntegrationActionResult {
                                status: IntegrationActionStatus::TransportAccepted,
                                request_id,
                                provider_receipt_id: None,
                                outputs: HashMap::new(),
                            });
                        };
                        let timeout_ms = match contract.receipt {
                            ActionReceiptSemantics::Applied { timeout_ms } => timeout_ms,
                            ActionReceiptSemantics::TransportAccepted => unreachable!(),
                        };
                        match tokio::time::timeout(
                            std::time::Duration::from_millis(timeout_ms),
                            receipt,
                        )
                        .await
                        {
                            Ok(Ok(Ok(()))) => {
                                return Ok(IntegrationActionResult {
                                    status: IntegrationActionStatus::Applied,
                                    request_id: request_id.clone(),
                                    provider_receipt_id: Some(request_id),
                                    outputs: HashMap::new(),
                                })
                            }
                            Ok(Ok(Err(error))) => {
                                return Err(IntegrationError::ActionFailed(error))
                            }
                            Ok(Err(_)) => {
                                return Err(IntegrationError::ExecutionError(
                                    "OBS action receipt waiter was cancelled".to_string(),
                                ))
                            }
                            Err(_) => {
                                self.state_manager
                                    .cancel_obs_action_receipt(&request_id)
                                    .await;
                                return Err(IntegrationError::ExecutionError(format!(
                                    "Timed out after {}ms waiting for OBS action receipt {}",
                                    timeout_ms, request_id
                                )));
                            }
                        }
                    }
                }
            }
            _ => {
                println!("[OBS Integration] No OBS plugin sender available in StateManager");
                // 🚀 NEW ARCHITECTURE: Only set status to false when no sender available (plugin disconnected)
                self.state_manager
                    .disconnect_obs_companion("OBS companion transport is unavailable")
                    .await;
                return Err(IntegrationError::NotConnected(
                    "No OBS plugin sender".to_string(),
                ));
            }
        }
    }
    async fn get_capabilities(&self) -> Vec<CapabilityDefinition> {
        let obs_actions_option = self.state_manager.get_obs_actions().await;

        let obs_actions = match obs_actions_option {
            Some(actions) => actions,
            None => {
                log::debug!("OBS action catalog has not been registered yet");
                return Vec::new();
            }
        };

        if obs_actions.is_empty() {
            log::debug!("OBS action catalog is empty while awaiting registration");
            return Vec::new();
        }

        let capabilities = obs_actions
            .iter()
            .map(|action| {
                self.capability_from_obs_action(
                    action,
                    Self::operational_contract(
                        &action.action_id,
                        &HashMap::new(),
                        if self.state_manager.obs_action_receipts_supported() {
                            ActionReceiptSemantics::Applied { timeout_ms: 15_000 }
                        } else {
                            ActionReceiptSemantics::TransportAccepted
                        },
                    ),
                )
            })
            .collect::<Vec<_>>();

        println!("✅ OBS: Loaded {} capabilities", capabilities.len());
        capabilities
    }

    async fn resolve_capability(
        &self,
        action_id: &str,
        parameters: &HashMap<String, Value>,
    ) -> IntegrationResult<CapabilityDefinition> {
        self.resolve_capability_definition(action_id, parameters)
            .await
    }

    async fn subscribe_to_events(
        &self,
        _callback: Box<dyn EventCallback>,
    ) -> IntegrationResult<()> {
        Ok(()) // TODO: Implement event subscription
    }
}

impl ObsIntegration {
    fn map_obs_parameter(&self, parameter: &ObsParameter) -> CapabilityParameter {
        let options = parameter
            .list_selection
            .iter()
            .map(|item| CapabilityOption {
                label: item.display_name.clone(),
                value: Value::String(item.value.clone()),
                parameters: item
                    .parameters
                    .as_ref()
                    .map(|parameters| {
                        parameters
                            .iter()
                            .map(|nested| self.map_obs_parameter(nested))
                            .collect()
                    })
                    .unwrap_or_default(),
            })
            .collect::<Vec<_>>();
        let value_type = Self::contract_value_type(&parameter.parameter_type);

        CapabilityParameter {
            id: parameter.parameter_id.clone(),
            label: parameter.assignment_message.clone(),
            description: parameter.assignment_message.clone(),
            value_type: value_type.clone(),
            required: parameter
                .required
                .unwrap_or_else(|| self.is_parameter_required(&parameter.parameter_id)),
            widget: if !options.is_empty() {
                CapabilityWidget::Select
            } else {
                match value_type {
                    ContractValueType::Boolean => CapabilityWidget::Toggle,
                    ContractValueType::Number | ContractValueType::Integer => {
                        CapabilityWidget::Number
                    }
                    ContractValueType::Object
                    | ContractValueType::Array
                    | ContractValueType::Any => CapabilityWidget::Json,
                    ContractValueType::String => CapabilityWidget::Text,
                }
            },
            default_value: self
                .get_parameter_default(&parameter.parameter_id)
                .map(Value::String),
            option_source: (!options.is_empty()).then(|| DynamicOptionSource {
                resolver_id: "obs.live_catalog".to_string(),
                depends_on: Vec::new(),
            }),
            options,
        }
    }

    fn capability_from_obs_action(
        &self,
        action: &ObsPluginAction,
        contract: ActionContract,
    ) -> CapabilityDefinition {
        CapabilityDefinition {
            integration_id: "obs".to_string(),
            capability_id: action.action_id.clone(),
            version: contract.version,
            kind: CapabilityKind::Action,
            presentation: CapabilityPresentation {
                name: action.name.clone(),
                description: action.name.clone(),
                category: "OBS".to_string(),
                icon: Some("simple-icons:obsstudio".to_string()),
                aliases: vec![action.action_id.clone()],
            },
            inputs: action
                .parameters
                .iter()
                .map(|parameter| self.map_obs_parameter(parameter))
                .collect(),
            outputs: vec![],
            allow_additional_inputs: false,
            contract,
        }
    }

    /// Determine if parameter is required based on action type
    fn is_parameter_required(&self, parameter_id: &str) -> bool {
        match parameter_id {
            "scene_name"
            | "source_name"
            | "collection_name"
            | "APPLET_OBS_SCENE_SELECTION"
            | "APPLET_OBS_SOURCE_SELECTION"
            | "APPLET_OBS_COLLECTION_SELECTION" => true,
            _ => false,
        }
    }

    /// Get default value for parameter
    fn get_parameter_default(&self, _parameter_id: &str) -> Option<String> {
        // Most OBS parameters don't have meaningful defaults
        None
    }

    /// Fallback actions when OBS plugin is not connected
    fn fallback_capabilities(&self) -> Vec<CapabilityDefinition> {
        [
            ("obs_stream_start", "Start stream", None),
            ("obs_stream_stop", "Stop stream", None),
            ("obs_recording_start", "Start recording", None),
            ("obs_recording_stop", "Stop recording", None),
            ("obs_scene_activate", "Activate scene", Some("scene_name")),
            (
                "obs_source_activate",
                "Activate source",
                Some("source_name"),
            ),
        ]
        .into_iter()
        .map(|(id, name, input)| {
            let contract = Self::operational_contract(
                id,
                &HashMap::new(),
                ActionReceiptSemantics::TransportAccepted,
            );
            CapabilityDefinition {
                integration_id: "obs".to_string(),
                capability_id: id.to_string(),
                version: contract.version,
                kind: CapabilityKind::Action,
                presentation: CapabilityPresentation {
                    name: name.to_string(),
                    description: name.to_string(),
                    category: "OBS".to_string(),
                    icon: Some("simple-icons:obsstudio".to_string()),
                    aliases: vec![],
                },
                inputs: input
                    .map(|id| CapabilityParameter {
                        id: id.to_string(),
                        label: if id.contains("scene") {
                            "Select Scene"
                        } else {
                            "Select Source"
                        }
                        .to_string(),
                        description: String::new(),
                        value_type: ContractValueType::String,
                        required: true,
                        widget: CapabilityWidget::Text,
                        default_value: None,
                        options: vec![],
                        option_source: None,
                    })
                    .into_iter()
                    .collect(),
                outputs: vec![],
                allow_additional_inputs: false,
                contract,
            }
        })
        .collect()
    }
}
