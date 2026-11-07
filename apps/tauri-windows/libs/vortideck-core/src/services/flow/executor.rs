use crate::services::flow::ai_executor::AiExecutor;
use crate::services::flow::commands::FlowRegistryState;
use crate::services::flow::traits::{ExecutionContext, FlowNode};
use crate::services::mcp::ai_node::{
    AiNodeBehavior, AiNodeConfig, AiNodeInput, FlowContext, ResponseFormat, ToolAccessPolicy,
    ToolAction,
};
use crate::services::mcp::commands::ChatState;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

// Note: Tauri dependencies removed - executor is now platform-agnostic
use crate::core::{PlatformContext, PlatformError};
use crate::db::database::DbPool;
use crate::db::db::get_vortideck_data_root;
use crate::events::FlowEventPayload;
use crate::services::execution::{
    ActionInvocationService, ButtonExecutionMode, ExecutionContext as UniversalExecutionContext,
    RetryConfig as UniversalRetryConfig, TriggerSource as UniversalTriggerSource, UniversalAction,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tokio::sync::{Mutex, Notify, RwLock};
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use uuid::Uuid;

pub use super::durable_scheduler::{
    wait_until_wall_clock_deadline, ApprovalActor, ApprovalDecision, ApprovalDecisionPolicy,
    ApprovalDecisionRequest, ApprovalRequestDetails, ApprovalResolution, ApprovalResolveOutcome,
    ApprovedEffectBinding, SystemWorkflowClock, WorkflowClock, WorkflowResourceLease,
    WorkflowResumePolicy, WorkflowWake, WorkflowWakeKind, WorkflowWakeStatus,
};
pub use super::runtime_storage::HybridFlowStorage;
pub use super::trigger_registry::ActiveTriggerDefinition;

mod scheduler;
mod trigger_registry;

// ============================================================================
// FLOW DEFINITION TYPES
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowDefinition {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<String>,
    pub tags: Vec<String>,
    pub nodes: Vec<FlowNodeDefinition>,
    pub connections: Vec<FlowConnection>,
    pub start_node_id: String,
    pub metadata: FlowMetadata,

    // Lifecycle: Whether this flow is active (triggers enabled)
    #[serde(default)]
    pub is_active: bool,
}

pub const CURRENT_FLOW_DOCUMENT_VERSION: &str = "1.0.0";
pub const LEGACY_FLOW_DOCUMENT_VERSION: &str = "1";

pub fn validate_flow_document_version(version: &str) -> Result<(), String> {
    if matches!(
        version,
        CURRENT_FLOW_DOCUMENT_VERSION | LEGACY_FLOW_DOCUMENT_VERSION
    ) {
        Ok(())
    } else {
        Err(format!(
            "Unsupported flow document version '{}'; this build supports '{}'",
            version, CURRENT_FLOW_DOCUMENT_VERSION
        ))
    }
}

/// Stable semantic identity used for optimistic flow edits. Wall-clock save
/// time is deliberately excluded; every other persisted field participates.
pub fn flow_revision(flow: &FlowDefinition) -> Result<String, String> {
    let mut document = serde_json::to_value(flow).map_err(|error| error.to_string())?;
    if let Some(object) = document.as_object_mut() {
        object.remove("updatedAt");
    }
    let canonical =
        serde_json_canonicalizer::to_string(&document).map_err(|error| error.to_string())?;
    Ok(hex::encode(Sha256::digest(canonical.as_bytes())))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowEditReceipt {
    pub flow_id: String,
    pub before_revision: String,
    pub after_revision: String,
    pub changed_paths: Vec<String>,
    pub flow: FlowDefinition,
}

fn collect_changed_paths(before: &Value, after: &Value, path: &str, output: &mut Vec<String>) {
    match (before, after) {
        (Value::Object(before), Value::Object(after)) => {
            let mut keys = before.keys().chain(after.keys()).collect::<Vec<_>>();
            keys.sort_unstable();
            keys.dedup();
            for key in keys {
                if key == "updatedAt" && path.is_empty() {
                    continue;
                }
                let child_path = format!("{}/{}", path, key.replace('~', "~0").replace('/', "~1"));
                match (before.get(key), after.get(key)) {
                    (Some(before), Some(after)) => {
                        collect_changed_paths(before, after, &child_path, output)
                    }
                    _ => output.push(child_path),
                }
            }
        }
        (Value::Array(before), Value::Array(after)) => {
            let length = before.len().max(after.len());
            for index in 0..length {
                let child_path = format!("{}/{}", path, index);
                match (before.get(index), after.get(index)) {
                    (Some(before), Some(after)) => {
                        collect_changed_paths(before, after, &child_path, output)
                    }
                    _ => output.push(child_path),
                }
            }
        }
        _ if before != after => output.push(if path.is_empty() {
            "/".to_string()
        } else {
            path.to_string()
        }),
        _ => {}
    }
}

pub fn flow_changed_paths(
    before: &FlowDefinition,
    after: &FlowDefinition,
) -> Result<Vec<String>, String> {
    let before = serde_json::to_value(before).map_err(|error| error.to_string())?;
    let after = serde_json::to_value(after).map_err(|error| error.to_string())?;
    let mut output = Vec::new();
    collect_changed_paths(&before, &after, "", &mut output);
    Ok(output)
}

fn prepare_inactive_flow_edit(
    flow_id: &str,
    expected_revision: &str,
    current: &FlowDefinition,
    mut proposed: FlowDefinition,
) -> Result<(FlowDefinition, String, Vec<String>), String> {
    validate_flow_document_version(&proposed.version)?;
    let before_revision = flow_revision(current)?;
    if before_revision != expected_revision {
        return Err(format!(
            "Stale flow revision: expected '{}', current is '{}'",
            expected_revision, before_revision
        ));
    }
    if proposed.id != flow_id || proposed.id != current.id {
        return Err("Flow identity cannot be changed by an editor patch".to_string());
    }
    if proposed.created_at != current.created_at || proposed.created_by != current.created_by {
        return Err("Flow creation identity cannot be changed by an editor patch".to_string());
    }
    if current.is_active || proposed.is_active != current.is_active {
        return Err(
            "Assistant edits are limited to inactive flows; activation uses a separate authority"
                .to_string(),
        );
    }
    if proposed.metadata.permissions != current.metadata.permissions {
        return Err("Flow permissions cannot be changed by an editor patch".to_string());
    }
    if proposed.name.trim().is_empty() {
        return Err("Flow name cannot be empty".to_string());
    }
    proposed.validate_graph()?;
    proposed.updated_at = Utc::now();
    let changed_paths = flow_changed_paths(current, &proposed)?;
    Ok((proposed, before_revision, changed_paths))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowNodeDefinition {
    pub id: String,
    pub node_type: String,
    pub name: String,
    pub inputs: HashMap<String, Value>,
    pub outputs: Vec<FlowNodeOutput>,
    pub ai_behavior: Option<AiNodeBehavior>,
    pub ai_model: Option<String>,        // Per-node AI model selection
    pub prompt_template: Option<String>, // Per-node prompt template selection
    pub tool_name: Option<String>,
    pub position: (f64, f64),
    pub ui_data: NodeUIData, // Frontend-specific data
    pub execution_config: NodeExecutionConfig,
}

/// A compiled graph edge. Control scheduling and value transport are different
/// operations and cannot be confused by provider or UI port names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum FlowConnection {
    Control {
        #[serde(rename = "fromNode")]
        from_node: String,
        #[serde(rename = "toNode")]
        to_node: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        route: Option<String>,
    },
    Data {
        #[serde(rename = "fromNode")]
        from_node: String,
        #[serde(rename = "fromOutput")]
        from_output: String,
        #[serde(rename = "toNode")]
        to_node: String,
        #[serde(rename = "toInput")]
        to_input: String,
    },
}

impl FlowConnection {
    pub fn control(from_node: impl Into<String>, to_node: impl Into<String>) -> Self {
        Self::Control {
            from_node: from_node.into(),
            to_node: to_node.into(),
            route: None,
        }
    }

    pub fn control_route(
        from_node: impl Into<String>,
        to_node: impl Into<String>,
        route: impl Into<String>,
    ) -> Self {
        Self::Control {
            from_node: from_node.into(),
            to_node: to_node.into(),
            route: Some(route.into()),
        }
    }

    pub fn data(
        from_node: impl Into<String>,
        from_output: impl Into<String>,
        to_node: impl Into<String>,
        to_input: impl Into<String>,
    ) -> Self {
        Self::Data {
            from_node: from_node.into(),
            from_output: from_output.into(),
            to_node: to_node.into(),
            to_input: to_input.into(),
        }
    }

    pub fn from_node(&self) -> &str {
        match self {
            Self::Control { from_node, .. } | Self::Data { from_node, .. } => from_node,
        }
    }

    pub fn to_node(&self) -> &str {
        match self {
            Self::Control { to_node, .. } | Self::Data { to_node, .. } => to_node,
        }
    }

    pub fn is_control(&self) -> bool {
        matches!(self, Self::Control { .. })
    }

    pub fn is_data(&self) -> bool {
        matches!(self, Self::Data { .. })
    }
}

impl FlowDefinition {
    /// Compile-time graph validation. Runtime scheduling receives only edges
    /// whose kind and endpoints have already been proven coherent.
    pub fn validate_graph(&self) -> Result<(), String> {
        if !self.nodes.iter().any(|node| node.id == self.start_node_id) {
            return Err(format!(
                "Start node '{}' does not exist",
                self.start_node_id
            ));
        }

        let nodes: HashMap<&str, &FlowNodeDefinition> = self
            .nodes
            .iter()
            .map(|node| (node.id.as_str(), node))
            .collect();
        if nodes.len() != self.nodes.len() {
            return Err("Flow contains duplicate node IDs".to_string());
        }

        let mut edge_keys = HashSet::new();
        let mut control_in_degree: HashMap<&str, usize> = nodes.keys().map(|id| (*id, 0)).collect();
        let mut control_successors: HashMap<&str, Vec<&str>> = HashMap::new();

        for edge in &self.connections {
            let source = nodes.get(edge.from_node()).ok_or_else(|| {
                format!(
                    "Connection references missing source node '{}'",
                    edge.from_node()
                )
            })?;
            let target = nodes.get(edge.to_node()).ok_or_else(|| {
                format!(
                    "Connection references missing target node '{}'",
                    edge.to_node()
                )
            })?;
            if edge.from_node() == edge.to_node() {
                return Err(format!(
                    "Node '{}' cannot connect to itself",
                    edge.from_node()
                ));
            }

            match edge {
                FlowConnection::Control {
                    from_node,
                    to_node,
                    route,
                } => {
                    let declared_routes: Vec<&str> = source
                        .outputs
                        .iter()
                        .filter(|output| output.output_type == "flow" && output.id != "output")
                        .map(|output| output.id.as_str())
                        .collect();
                    match route {
                        Some(route) if route.trim().is_empty() => {
                            return Err(format!(
                                "Control edge {} -> {} has an empty route",
                                from_node, to_node
                            ));
                        }
                        Some(route) if !declared_routes.contains(&route.as_str()) => {
                            return Err(format!(
                                "Node '{}' does not declare control route '{}'",
                                source.id, route
                            ));
                        }
                        None if !declared_routes.is_empty()
                            && !source.outputs.iter().any(|output| {
                                output.output_type == "flow" && output.id == "output"
                            }) =>
                        {
                            return Err(format!(
                                "Node '{}' declares routed control outputs; select an explicit route",
                                source.id
                            ));
                        }
                        _ => {}
                    }
                    let key = format!(
                        "control:{from_node}:{to_node}:{}",
                        route.as_deref().unwrap_or("")
                    );
                    if !edge_keys.insert(key) {
                        return Err(format!(
                            "Duplicate control edge {} -> {}",
                            from_node, to_node
                        ));
                    }
                    *control_in_degree
                        .get_mut(to_node.as_str())
                        .expect("target existence checked") += 1;
                    control_successors
                        .entry(from_node.as_str())
                        .or_default()
                        .push(to_node.as_str());
                }
                FlowConnection::Data {
                    from_node,
                    from_output,
                    to_node,
                    to_input,
                } => {
                    if from_output.trim().is_empty() || to_input.trim().is_empty() {
                        return Err(format!(
                            "Data edge {} -> {} must name both data ports",
                            from_node, to_node
                        ));
                    }
                    if !source
                        .outputs
                        .iter()
                        .any(|output| output.id == *from_output && output.output_type != "flow")
                    {
                        return Err(format!(
                            "Node '{}' does not declare data output '{}'",
                            source.id, from_output
                        ));
                    }
                    if !target.inputs.contains_key(to_input) {
                        return Err(format!(
                            "Node '{}' does not declare input '{}'",
                            target.id, to_input
                        ));
                    }
                    let key = format!("data:{from_node}:{from_output}:{to_node}:{to_input}");
                    if !edge_keys.insert(key) {
                        return Err(format!(
                            "Duplicate data edge {}:{} -> {}:{}",
                            from_node, from_output, to_node, to_input
                        ));
                    }
                }
            }
        }

        let mut ready: Vec<&str> = control_in_degree
            .iter()
            .filter_map(|(id, degree)| (*degree == 0).then_some(*id))
            .collect();
        let mut visited = 0usize;
        while let Some(node_id) = ready.pop() {
            visited += 1;
            for successor in control_successors.get(node_id).into_iter().flatten() {
                let degree = control_in_degree
                    .get_mut(successor)
                    .expect("successor existence checked");
                *degree -= 1;
                if *degree == 0 {
                    ready.push(successor);
                }
            }
        }
        if visited != nodes.len() {
            return Err("Control graph contains a cycle".to_string());
        }

        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
enum TypedFlowConnection {
    Control {
        #[serde(rename = "fromNode")]
        from_node: String,
        #[serde(rename = "toNode")]
        to_node: String,
        #[serde(default)]
        route: Option<String>,
    },
    Data {
        #[serde(rename = "fromNode")]
        from_node: String,
        #[serde(rename = "fromOutput")]
        from_output: String,
        #[serde(rename = "toNode")]
        to_node: String,
        #[serde(rename = "toInput")]
        to_input: String,
    },
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MigratingFlowConnection {
    from_node: String,
    from_port: String,
    to_node: String,
    to_port: String,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum FlowConnectionWire {
    Typed(TypedFlowConnection),
    Migrating(MigratingFlowConnection),
}

impl<'de> Deserialize<'de> for FlowConnection {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;

        match FlowConnectionWire::deserialize(deserializer)? {
            FlowConnectionWire::Typed(TypedFlowConnection::Control {
                from_node,
                to_node,
                route,
            }) => Ok(Self::Control {
                from_node,
                to_node,
                route,
            }),
            FlowConnectionWire::Typed(TypedFlowConnection::Data {
                from_node,
                from_output,
                to_node,
                to_input,
            }) => Ok(Self::Data {
                from_node,
                from_output,
                to_node,
                to_input,
            }),
            FlowConnectionWire::Migrating(edge) => {
                Self::from_migrating_edge(edge).map_err(D::Error::custom)
            }
        }
    }
}

impl FlowConnection {
    fn from_migrating_edge(edge: MigratingFlowConnection) -> Result<Self, String> {
        fn route(port: &str) -> Option<String> {
            port.strip_prefix("flow-output-")
                .filter(|route| *route != "output")
                .map(str::to_string)
        }
        fn value_port(port: &str, prefix: &str) -> String {
            port.strip_prefix(prefix).unwrap_or(port).to_string()
        }

        if edge.from_port.starts_with("flow-output") && edge.to_port == "flow-input" {
            return Ok(Self::Control {
                from_node: edge.from_node,
                to_node: edge.to_node,
                route: route(&edge.from_port),
            });
        }
        if edge.from_port == "flow-input" && edge.to_port.starts_with("flow-output") {
            return Ok(Self::Control {
                from_node: edge.to_node,
                to_node: edge.from_node,
                route: route(&edge.to_port),
            });
        }
        if edge.from_port.starts_with("value-output-") && edge.to_port.starts_with("value-input-") {
            return Ok(Self::data(
                edge.from_node,
                value_port(&edge.from_port, "value-output-"),
                edge.to_node,
                value_port(&edge.to_port, "value-input-"),
            ));
        }
        if edge.from_port.starts_with("value-input-") && edge.to_port.starts_with("value-output-") {
            return Ok(Self::data(
                edge.to_node,
                value_port(&edge.to_port, "value-output-"),
                edge.from_node,
                value_port(&edge.from_port, "value-input-"),
            ));
        }
        if !edge.from_port.contains("flow") && !edge.to_port.contains("flow") {
            return Ok(Self::data(
                edge.from_node,
                edge.from_port,
                edge.to_node,
                edge.to_port,
            ));
        }

        Err(format!(
            "Cannot migrate connection {}:{} -> {}:{}",
            edge.from_node, edge.from_port, edge.to_node, edge.to_port
        ))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeUIData {
    pub color: Option<String>,
    pub icon: Option<String>,
    pub collapsed: bool,
    pub selected: bool,
    pub z_index: i32,
    pub custom_properties: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeExecutionConfig {
    pub timeout_ms: Option<u64>,
    pub retry_count: u32,
    pub retry_delay_ms: u64,
    pub skip_on_error: bool,
    pub parallel_execution: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowNodeOutput {
    pub id: String,
    pub name: String,
    pub output_type: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowMetadata {
    pub canvas_settings: CanvasSettings,
    pub variables: HashMap<String, FlowVariable>,
    pub permissions: FlowPermissions,
    pub execution_settings: FlowExecutionSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanvasSettings {
    pub zoom: f64,
    pub pan_x: f64,
    pub pan_y: f64,
    pub grid_enabled: bool,
    pub snap_to_grid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowVariable {
    pub name: String,
    pub value: Value,
    pub variable_type: String,
    pub description: Option<String>,
    pub scope: VariableScope,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum VariableScope {
    #[default]
    Local, // Runtime only - cleared after execution
    // TODO: Future scopes (uncomment when needed)
    Session, // Persists across executions until app closes
    Global,  // Saved with flow definition permanently
}

// ============================================================================
// RUNTIME VARIABLES
// ============================================================================

/// Runtime variable storage during flow execution
/// Variables can store ANY JSON value (strings, numbers, objects, arrays, etc.)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuntimeVariables {
    /// Local variables - exist only during current execution
    /// Cleared when flow completes or fails
    local: HashMap<String, Value>,
    #[serde(default)]
    read_only: HashSet<String>,
    // TODO: Future scopes (uncomment when needed)
    // /// Session variables - persist across multiple executions until app closes
    // session: HashMap<String, Value>,
    //
    // /// Global variables - loaded from/saved to flow definition
    // global: HashMap<String, Value>,
}

impl RuntimeVariables {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a variable by name
    /// Priority: Local (→ Session → Global when implemented)
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.local.get(name)
        // TODO: When other scopes are implemented:
        // .or_else(|| self.session.get(name))
        // .or_else(|| self.global.get(name))
    }

    /// Set a variable with the given scope
    pub fn set(&mut self, name: impl Into<String>, value: Value, _scope: VariableScope) {
        let name = name.into();
        if self.read_only.contains(&name) {
            return;
        }
        // For now, all variables are local (runtime only)
        self.local.insert(name, value);

        // TODO: When other scopes are implemented:
        // match scope {
        //     VariableScope::Local => self.local.insert(name.into(), value),
        //     VariableScope::Session => self.session.insert(name.into(), value),
        //     VariableScope::Global => self.global.insert(name.into(), value),
        // };
    }

    /// Delete a variable
    pub fn delete(&mut self, name: &str) -> Option<Value> {
        if self.is_read_only(name) {
            return None;
        }
        self.local.remove(name)
    }

    pub fn is_read_only(&self, name: &str) -> bool {
        self.read_only.contains(name)
    }

    fn set_run_inputs(&mut self, inputs: &HashMap<String, Value>) {
        if inputs.is_empty() {
            return;
        } // Preserve legacy flows using an ordinary variable named inputs.
        self.local.insert("inputs".into(), json!(inputs));
        self.read_only.insert("inputs".into());
    }

    /// Check if a variable exists
    pub fn exists(&self, name: &str) -> bool {
        self.local.contains_key(name)
    }

    /// Get all variables (for debugging/inspection)
    pub fn all(&self) -> &HashMap<String, Value> {
        &self.local
    }

    /// Clear all local variables (called after execution completes)
    pub fn clear_local(&mut self) {
        self.local.clear();
        self.read_only.clear();
    }

    /// Get variable count
    pub fn len(&self) -> usize {
        self.local.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.local.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowPermissions {
    pub public: bool,
    pub collaborators: Vec<String>,
    pub execution_roles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowExecutionSettings {
    pub max_execution_time_ms: Option<u64>,
    pub max_concurrent_nodes: u32,
    pub auto_save_interval_ms: u64,
}

// ============================================================================
// EXECUTION STATE TYPES
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowExecutionState {
    /// Immutable identifier for one execution of a flow definition.
    ///
    /// This must never be confused with `flow_id`: a flow can have many
    /// concurrent or historical runs.
    #[serde(default)]
    pub run_id: String,
    pub flow_id: String,
    pub flow_definition: FlowDefinition,
    pub status: FlowStatus,
    pub current_node: Option<String>,
    pub completed_nodes: HashSet<String>,
    pub failed_nodes: HashSet<String>,
    pub node_results: HashMap<String, NodeResult>,
    pub execution_log: Vec<ExecutionLogEntry>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    /// First time a replacement process detected this non-terminal snapshot.
    /// This is not a completion timestamp and never implies side effects ended
    /// at this exact instant.
    #[serde(default)]
    pub recovery_detected_at: Option<DateTime<Utc>>,
    // 🆕 Queue for nodes waiting to execute due to concurrency limits
    pub pending_nodes: Vec<FlowNodeDefinition>,
    // 🆕 Pre-sorted node results for frontend (in execution order)
    pub sorted_node_results: Vec<NodeResult>,
    // 🆕 Runtime variables - for passing data between nodes without connections
    #[serde(default)]
    pub runtime_variables: RuntimeVariables,
    /// Action hashes authorized by completed approval nodes. Old checkpoints
    /// deserialize to an empty map and therefore never gain implicit consent.
    #[serde(default)]
    pub approved_effect_bindings: HashMap<String, String>,
    // 🆕 Lineage tracker for data provenance (not serialized)
    #[serde(skip)]
    pub lineage_tracker: Option<crate::services::flow::lineage::LineageTracker>,
    // 🆕 Final lineage record (serialized for storage)
    pub lineage: Option<crate::services::flow::lineage::FlowLineage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RecoveryPolicy {
    /// Resume after a persisted receipt proves the desired state was applied.
    Reconcile,
    /// Retry only with the exact same externally propagated idempotency key.
    RetryWithIdempotency,
    /// An operator must inspect the external system before choosing an action.
    ManualReview,
    /// No automatic or user-triggered retry is permitted.
    FailClosed,
}

enum LedgerRecoveryResolution {
    Overlay(crate::events::OverlayCommandDelivery),
    CompletedSideEffect(crate::events::SideEffectOutboxEntry),
    RetrySideEffect(crate::events::SideEffectOutboxEntry),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FlowStatus {
    Pending,
    Running,
    /// The process stopped while this run was pending/running. Explicit
    /// recovery is required before side effects may continue.
    RecoveryRequired,
    Completed,
    CompletedWithErrors, // Execution finished but some nodes failed
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeResult {
    pub node_id: String,
    pub node_name: String,
    pub node_type: String,
    pub status: NodeStatus,
    pub inputs: HashMap<String, Value>,
    pub outputs: HashMap<String, Value>,
    pub error: Option<String>,
    pub execution_time_ms: u64,
    pub retry_count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionLogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub node_id: Option<String>,
    pub message: String,
    pub details: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

// ============================================================================
// FLOW EVENTS
// ============================================================================

// New execution history entry for backend storage
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionHistoryEntry {
    pub id: String,
    pub run_id: String,
    pub flow_id: String,
    pub flow_name: String,
    pub status: FlowStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub reachable_node_count: u32,
    pub completed_nodes: u32,
    pub failed_nodes: u32,
    pub error_message: Option<String>,
    pub final_results: Option<HashMap<String, Value>>,
    /// Detailed execution data for each node including inputs and outputs
    pub node_execution_details: Vec<NodeExecutionDetail>,
}

/// Detailed execution information for a single node
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeExecutionDetail {
    pub node_id: String,
    pub node_name: String,
    pub node_type: String,
    pub status: NodeStatus,
    pub inputs: HashMap<String, Value>,
    pub outputs: HashMap<String, Value>,
    pub error: Option<String>,
    pub execution_time_ms: u64,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub retry_count: u32,
}

// ============================================================================
// TRIGGER LIFECYCLE TYPES
// ============================================================================

/// Source of a trigger event (for de-duplication and routing)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerSource {
    /// Event from backend server (streaming events)
    Backend,

    /// Event from local webhook server
    LocalWebhook,

    /// Event from local polling
    LocalPolling,

    /// Event from a locally connected integration provider (for example a
    /// Discord Gateway application command).
    Integration,

    /// Manual trigger (button click)
    Manual,

    /// Scheduled trigger (cron)
    Scheduled,
}

/// Natural source preference for different service types
///
/// Note: De-duplication applies to all events regardless of source preference.
/// This is primarily for documentation and future routing logic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NaturalSource {
    /// Server webhook or backend-relayed events (e.g., Twitch, Kick, YouTube)
    /// These typically come from server deployments with public URLs
    ServerBased,

    /// Local-only events (e.g., Discord bot, GitHub, file watching, polling)
    /// These come from desktop deployments with direct integrations
    LocalOnly,

    /// Either source (e.g., custom webhooks, manual triggers)
    Any,
}

/// Webhook deployment mode - determines how webhooks are registered
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WebhookMode {
    /// Server deployment with public URL - direct registration (like n8n)
    /// Example: https://vortideck.yourdomain.com/webhook/abc123
    ServerDirect { public_base_url: String },

    /// Desktop with tunnel service (ngrok, cloudflared)
    /// Example: https://abc123.ngrok.io/webhook/abc123
    DesktopWithTunnel { tunnel_url: String },

    /// Manual registration - user registers webhook themselves
    /// Desktop app only provides webhook endpoint, no auto-registration
    Manual,
}

/// Event that triggers a flow execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerEvent {
    pub source: TriggerSource,
    /// Stable identity of the source event when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    /// Links the run to related events and downstream action records.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    pub flow_id: String,
    pub node_id: String,
    pub data: Value,
    pub timestamp: DateTime<Utc>,

    /// Platform-specific ID for de-duplication (e.g., "twitch:stream.online:12345")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dedup_key: Option<String>,
}

/// Context provided to trigger nodes for managing persistent state
#[derive(Clone)]
pub struct TriggerContext {
    pub flow_id: String,
    pub node_id: String,
    storage: Arc<dyn FlowStorage>,
}

impl TriggerContext {
    pub fn new(flow_id: String, node_id: String, storage: Arc<dyn FlowStorage>) -> Self {
        Self {
            flow_id,
            node_id,
            storage,
        }
    }

    /// Get persistent state for this trigger (survives restarts)
    pub async fn get_state(&self, key: &str) -> Result<Option<Value>, String> {
        self.storage
            .load_trigger_state(&self.flow_id, &self.node_id, key)
            .await
    }

    /// Set persistent state for this trigger
    pub async fn set_state(&self, key: &str, value: Value) -> Result<(), String> {
        self.storage
            .save_trigger_state(&self.flow_id, &self.node_id, key, value)
            .await
    }

    /// Helper: Get last poll time for polling triggers
    pub async fn get_last_poll_time(&self) -> Result<Option<DateTime<Utc>>, String> {
        match self.get_state("last_poll_time").await? {
            Some(Value::String(s)) => DateTime::parse_from_rfc3339(&s)
                .map(|dt| Some(dt.with_timezone(&Utc)))
                .map_err(|e| format!("Failed to parse datetime: {}", e)),
            _ => Ok(None),
        }
    }

    /// Helper: Set last poll time for polling triggers
    pub async fn set_last_poll_time(&self, time: DateTime<Utc>) -> Result<(), String> {
        self.set_state("last_poll_time", Value::String(time.to_rfc3339()))
            .await
    }

    /// Get webhook deployment mode (configured globally or per-flow)
    /// TODO: Load from configuration file or environment
    pub async fn get_webhook_mode(&self) -> Result<WebhookMode, String> {
        // For now, default to Manual mode
        // In production, this would read from config:
        // - Check environment variable WEBHOOK_MODE
        // - Check flow-specific override
        // - Fall back to global config

        // Example future implementation:
        // if let Ok(public_url) = std::env::var("VORTIDECK_PUBLIC_URL") {
        //     return Ok(WebhookMode::ServerDirect { public_base_url: public_url });
        // }

        Ok(WebhookMode::Manual)
    }

    /// Generate webhook URL for this trigger based on deployment mode
    pub async fn get_webhook_url(&self) -> Result<String, String> {
        let mode = self.get_webhook_mode().await?;
        let path = format!("/webhook/{}-{}", self.flow_id, self.node_id);

        match mode {
            WebhookMode::ServerDirect { public_base_url } => {
                Ok(format!("{}{}", public_base_url.trim_end_matches('/'), path))
            }
            WebhookMode::DesktopWithTunnel { tunnel_url } => {
                Ok(format!("{}{}", tunnel_url.trim_end_matches('/'), path))
            }
            WebhookMode::Manual => {
                // Return localhost URL for manual setup
                Ok(format!("http://localhost:3000{}", path))
            }
        }
    }

    /// Helper: Get stored webhook subscription ID (for external services like Twitch)
    pub async fn get_subscription_id(&self) -> Result<Option<String>, String> {
        match self.get_state("subscription_id").await? {
            Some(Value::String(id)) => Ok(Some(id)),
            _ => Ok(None),
        }
    }

    /// Helper: Set webhook subscription ID
    pub async fn set_subscription_id(&self, id: String) -> Result<(), String> {
        self.set_state("subscription_id", Value::String(id)).await
    }

    /// Helper: Clear webhook subscription ID
    pub async fn clear_subscription_id(&self) -> Result<(), String> {
        // Set to null/empty instead of deleting
        self.set_state("subscription_id", Value::Null).await
    }
}

// ============================================================================
// FLOW EXECUTOR
// ============================================================================

/// 🚀 UNIVERSAL ARCHITECTURE: Event emitter abstraction
#[derive(Clone)]
pub enum EventEmitter {
    Universal(Arc<PlatformContext>),
}

/// Run-scoped webhook response signals. Unlike the application EventBus, this
/// single-consumer channel is created before execution starts and cannot lag
/// behind unrelated application traffic.
#[derive(Debug, Clone)]
pub enum FlowRunResponseSignal {
    ResponseNode(Value),
    Completed(Value),
    Failed(String),
}

pub struct FlowExecutor {
    flow_registry: Arc<FlowRegistryState>,
    chat_state: Arc<ChatState>,
    event_emitter: EventEmitter,
    active_flows: Arc<Mutex<HashMap<String, FlowExecutionState>>>,
    run_flow_ids: Arc<RwLock<HashMap<String, String>>>,
    ai_executor: AiExecutor,
    storage: Arc<dyn FlowStorage>,
    clock: Arc<dyn WorkflowClock>,
    action_invocation_service: Option<Arc<ActionInvocationService>>,

    /// Per-run cancellation signals. Cancelling a run drops the currently
    /// executing node future and prevents any downstream scheduling.
    cancellation_signals: Arc<Mutex<HashMap<String, tokio::sync::watch::Sender<bool>>>>,

    /// Recent trigger events for de-duplication (key -> timestamp)
    recent_triggers: Arc<RwLock<HashMap<String, DateTime<Utc>>>>,

    /// Live waiters for authenticated overlay-host receipts. The command and
    /// run checkpoint are persisted separately; this map only coordinates the
    /// current process and is intentionally never treated as durable state.
    pending_overlay_acks: Arc<
        Mutex<
            HashMap<
                String,
                tokio::sync::oneshot::Sender<crate::events::OverlayCommandAcknowledgement>,
            >,
        >,
    >,
    /// One-shot operator approvals for retrying a durable side effect with the
    /// exact same idempotency key. Never persisted as implicit permission.
    approved_side_effect_retries: Arc<Mutex<HashSet<String>>>,
    /// Live process waiters for durable approval records. The durable wake row,
    /// never this map, is the source of truth across restarts.
    pending_approval_wakes:
        Arc<Mutex<HashMap<String, tokio::sync::oneshot::Sender<ApprovalResolution>>>>,
    run_response_senders:
        Arc<Mutex<HashMap<String, tokio::sync::mpsc::UnboundedSender<FlowRunResponseSignal>>>>,
    run_response_receivers:
        Arc<Mutex<HashMap<String, tokio::sync::mpsc::UnboundedReceiver<FlowRunResponseSignal>>>>,
    /// Linearizes new run registration with runtime shutdown. A read guard is
    /// held until the run task is tracked; shutdown takes the write guard
    /// before collecting snapshots.
    runtime_admission_gate: Arc<RwLock<()>>,
    accepting_runtime_work: Arc<AtomicBool>,
    runtime_admission_notify: Arc<Notify>,
    runtime_tasks: TaskTracker,
    runtime_shutdown: Arc<std::sync::RwLock<CancellationToken>>,
}

impl FlowExecutor {
    /// 🚀 UNIVERSAL ARCHITECTURE: Create FlowExecutor with platform abstractions
    /// Uses main Diesel database for flow definitions, separate flows.db for execution history
    pub fn new_universal(
        flow_registry: Arc<FlowRegistryState>,
        chat_state: Arc<ChatState>,
        platform_context: Arc<PlatformContext>,
        db_pool: DbPool,
    ) -> Result<Self, PlatformError> {
        // Use proper AppData directory path for execution history
        let flows_db_path = get_vortideck_data_root().join("flows.db");

        // Ensure the directory exists
        if let Some(parent) = flows_db_path.parent() {
            std::fs::create_dir_all(parent).expect("Failed to create VortiDeck directory");
        }

        let flows_db_path = flows_db_path.to_string_lossy().to_string();

        // Hybrid storage: main DB for flows, flows.db for execution history
        let storage = Arc::new(
            HybridFlowStorage::new(db_pool.clone(), &flows_db_path)
                .map_err(PlatformError::StorageError)?,
        );
        Ok(Self::new_with_storage(
            flow_registry,
            chat_state,
            platform_context,
            storage,
        ))
    }

    /// Construct the executor with an explicit storage boundary. Production
    /// uses `new_universal`; integration tests and headless embedders can use an
    /// isolated durable store without mutating the process-wide VortiDeck data
    /// directory.
    pub fn new_with_storage(
        flow_registry: Arc<FlowRegistryState>,
        chat_state: Arc<ChatState>,
        platform_context: Arc<PlatformContext>,
        storage: Arc<dyn FlowStorage>,
    ) -> Self {
        Self {
            flow_registry: flow_registry.clone(),
            chat_state: chat_state.clone(),
            event_emitter: EventEmitter::Universal(platform_context.clone()),
            active_flows: Arc::new(Mutex::new(HashMap::new())),
            run_flow_ids: Arc::new(RwLock::new(HashMap::new())),
            ai_executor: AiExecutor::new((*flow_registry).clone(), (*chat_state).clone()),
            storage,
            clock: Arc::new(SystemWorkflowClock),
            action_invocation_service: None,
            cancellation_signals: Arc::new(Mutex::new(HashMap::new())),
            recent_triggers: Arc::new(RwLock::new(HashMap::new())),
            pending_overlay_acks: Arc::new(Mutex::new(HashMap::new())),
            approved_side_effect_retries: Arc::new(Mutex::new(HashSet::new())),
            pending_approval_wakes: Arc::new(Mutex::new(HashMap::new())),
            run_response_senders: Arc::new(Mutex::new(HashMap::new())),
            run_response_receivers: Arc::new(Mutex::new(HashMap::new())),
            runtime_admission_gate: Arc::new(RwLock::new(())),
            accepting_runtime_work: Arc::new(AtomicBool::new(true)),
            runtime_admission_notify: Arc::new(Notify::new()),
            runtime_tasks: TaskTracker::new(),
            runtime_shutdown: Arc::new(std::sync::RwLock::new(CancellationToken::new())),
        }
    }

    pub async fn begin_runtime_startup(&self) {
        let _admission = self.runtime_admission_gate.write().await;
        self.accepting_runtime_work.store(false, Ordering::Release);
        self.runtime_tasks.reopen();
        *self.runtime_shutdown.write().unwrap() = CancellationToken::new();
    }

    /// Used only while a newly constructed executor is being handed to its
    /// runtime owner, before the handle is published to request adapters.
    pub fn mark_runtime_owned(&self) {
        self.accepting_runtime_work.store(false, Ordering::Release);
    }

    pub async fn open_runtime_admission(&self) {
        let _admission = self.runtime_admission_gate.write().await;
        self.accepting_runtime_work.store(true, Ordering::Release);
        self.runtime_admission_notify.notify_waiters();
    }

    pub async fn close_runtime_admission(
        &self,
        deadline: std::time::Duration,
    ) -> Result<(), String> {
        // Flip the fast-path first so readers queued behind the writer cannot
        // admit more work. The write lock is the barrier for readers that were
        // already inside an accepted operation.
        self.accepting_runtime_work.store(false, Ordering::Release);
        let _admission = tokio::time::timeout(deadline, self.runtime_admission_gate.write())
            .await
            .map_err(|_| "Timed out closing workflow runtime admission".to_string())?;
        Ok(())
    }

    fn ensure_runtime_admission(&self) -> Result<(), String> {
        if self.accepting_runtime_work.load(Ordering::Acquire) {
            Ok(())
        } else {
            Err("Flow runtime is not accepting new work".to_string())
        }
    }

    fn runtime_shutdown_token(&self) -> CancellationToken {
        self.runtime_shutdown.read().unwrap().clone()
    }

    async fn wait_for_runtime_admission(&self) -> bool {
        loop {
            if self.accepting_runtime_work.load(Ordering::Acquire) {
                return true;
            }
            let notified = self.runtime_admission_notify.notified();
            if self.accepting_runtime_work.load(Ordering::Acquire) {
                return true;
            }
            tokio::select! {
                _ = notified => {}
                _ = self.runtime_shutdown_token().cancelled_owned() => return false,
            }
        }
    }

    pub fn tracked_runtime_task_count(&self) -> usize {
        self.runtime_tasks.len()
    }

    pub async fn finish_runtime_shutdown(
        &self,
        deadline: std::time::Duration,
    ) -> Result<(), String> {
        self.runtime_tasks.close();
        self.runtime_shutdown_token().cancel();
        tokio::time::timeout(deadline, self.runtime_tasks.wait())
            .await
            .map_err(|_| {
                format!(
                    "Timed out waiting for {} workflow runtime task(s) to stop",
                    self.runtime_tasks.len()
                )
            })
    }

    pub fn set_action_invocation_service(
        &mut self,
        invocation_service: Arc<ActionInvocationService>,
    ) {
        self.action_invocation_service = Some(invocation_service);
    }

    /// Override the workflow wall clock before execution. This is primarily for
    /// deterministic recovery tests and headless hosts with an explicit clock.
    pub fn set_workflow_clock(&mut self, clock: Arc<dyn WorkflowClock>) {
        self.clock = clock;
    }

    async fn resume_policy_for_run(&self, run_id: &str) -> Result<WorkflowResumePolicy, String> {
        let active = self.active_flows.lock().await;
        let state = active
            .get(run_id)
            .ok_or_else(|| format!("Run {run_id} is not loaded"))?;
        Ok(WorkflowResumePolicy::pinned(flow_revision(
            &state.flow_definition,
        )?))
    }

    fn validate_wake_resume_policy(
        wake: &WorkflowWake,
        state: &FlowExecutionState,
    ) -> Result<(), String> {
        if !wake.resume_policy.is_current() {
            return Err(format!(
                "Wake {} uses a legacy or unsupported resume policy; manual recovery is required",
                wake.id
            ));
        }
        let snapshot_revision = flow_revision(&state.flow_definition)?;
        if wake.resume_policy.flow_revision != snapshot_revision {
            return Err(format!(
                "Wake {} belongs to flow revision {}, but run {} contains {}; automatic resume is refused",
                wake.id, wake.resume_policy.flow_revision, state.run_id, snapshot_revision
            ));
        }
        Ok(())
    }

    /// Save a flow definition to storage
    pub async fn save_flow(&self, flow: &FlowDefinition) -> Result<(), String> {
        // Inactive flows are editable drafts and may be incomplete. Active
        // flows must always remain executable because their triggers can fire
        // immediately after this save.
        if flow.is_active {
            flow.validate_graph()?;
        }
        self.storage.save_flow(flow).await?;
        self.synchronize_trigger_registry(flow).await?;
        self.emit_trigger_registry_changed(&flow.id).await;
        Ok(())
    }

    /// Save through optimistic concurrency and refresh trigger state. Unlike
    /// the assistant-specific edit operation, this is the general editor path
    /// and may intentionally change activation or permissions.
    pub async fn save_flow_at_revision(
        &self,
        expected_revision: &str,
        flow: &FlowDefinition,
    ) -> Result<FlowDefinition, String> {
        validate_flow_document_version(&flow.version)?;
        if flow.is_active {
            flow.validate_graph()?;
        }
        let saved = self
            .storage
            .replace_flow_at_revision(expected_revision, flow)
            .await?;
        self.synchronize_trigger_registry(&saved).await?;
        self.emit_trigger_registry_changed(&saved.id).await;
        Ok(saved)
    }

    /// Replace an inactive saved flow only when the caller observed the current
    /// semantic revision. Identity, permissions and activation remain owned by
    /// dedicated host/UI operations; this boundary cannot execute the graph.
    pub async fn replace_inactive_flow_at_revision(
        &self,
        flow_id: &str,
        expected_revision: &str,
        proposed: FlowDefinition,
    ) -> Result<FlowEditReceipt, String> {
        let current = self.load_flow(flow_id).await?;
        let (proposed, before_revision, changed_paths) =
            prepare_inactive_flow_edit(flow_id, expected_revision, &current, proposed)?;
        if changed_paths.is_empty() {
            return Ok(FlowEditReceipt {
                flow_id: flow_id.to_string(),
                before_revision: before_revision.clone(),
                after_revision: before_revision,
                changed_paths,
                flow: current,
            });
        }

        let saved = self
            .storage
            .replace_flow_at_revision(expected_revision, &proposed)
            .await?;
        self.synchronize_trigger_registry(&saved).await?;
        self.emit_trigger_registry_changed(flow_id).await;
        let after_revision = flow_revision(&saved)?;
        Ok(FlowEditReceipt {
            flow_id: flow_id.to_string(),
            before_revision,
            after_revision,
            changed_paths,
            flow: saved,
        })
    }

    /// Load a flow definition from storage  
    pub async fn load_flow(&self, flow_id: &str) -> Result<FlowDefinition, String> {
        self.storage.load_flow(flow_id).await
    }

    /// List all saved flows
    pub async fn list_flows(&self) -> Result<Vec<FlowSummary>, String> {
        self.storage.list_flows().await
    }

    /// Return the compact persisted trigger registry used by EventRouter.
    pub async fn list_active_triggers(&self) -> Result<Vec<ActiveTriggerDefinition>, String> {
        self.storage.list_active_triggers().await
    }

    /// Persist a canonical source event before it can launch workflow side
    /// effects. Returns false when that exact event ID is already present.
    pub async fn record_event_envelope(
        &self,
        envelope: &crate::events::EventEnvelope,
    ) -> Result<bool, String> {
        self.storage.store_event_envelope(envelope).await
    }

    /// Load a recorded source event for diagnostics or replay tooling.
    pub async fn observed_stream_values(
        &self,
    ) -> Result<Vec<crate::services::streaming::stream_values::ObservedStreamValue>, String> {
        self.storage.observed_stream_values().await
    }

    /// Load a recorded source event for diagnostics or replay tooling.
    pub async fn load_event_envelope(
        &self,
        event_id: &str,
    ) -> Result<Option<crate::events::EventEnvelope>, String> {
        self.storage.load_event_envelope(event_id).await
    }

    pub async fn list_event_envelopes(
        &self,
        event_type: Option<&str>,
        since: Option<DateTime<Utc>>,
        limit: u32,
    ) -> Result<Vec<crate::events::EventEnvelope>, String> {
        self.storage
            .list_event_envelopes(event_type, since, limit)
            .await
    }

    /// Rebuild the registry from source-of-truth active flow definitions.
    /// This repairs stale registry rows after crashes or upgrades.
    pub async fn rebuild_active_trigger_registry(&self) -> Result<usize, String> {
        self.storage.clear_active_triggers().await?;
        let summaries = self.storage.list_flows().await?;
        let mut count = 0;

        for summary in summaries {
            let flow = self.storage.load_flow(&summary.id).await?;
            if flow.is_active {
                let definitions = Self::active_trigger_definitions(&flow);
                count += definitions.len();
                self.storage
                    .replace_active_triggers(&flow.id, &definitions)
                    .await?;
            }
        }

        self.emit_trigger_registry_changed("startup-rebuild").await;
        Ok(count)
    }

    /// Reconcile runs left pending/running by a previous process. We mark them
    /// as recovery-required rather than automatically replaying side effects.
    /// A future recovery policy can explicitly resume safe nodes from the saved
    /// checkpoint.
    pub async fn recover_interrupted_runs(&self) -> Result<usize, String> {
        let interrupted_runs = self.storage.list_recoverable_execution_states().await?;
        let count = interrupted_runs
            .iter()
            .filter(|state| !matches!(state.status, FlowStatus::RecoveryRequired))
            .count();
        let mut durable_timers = Vec::new();
        let mut durable_retries = Vec::new();
        let mut approval_timeouts = Vec::new();
        let mut ready_approvals = Vec::new();

        for mut state in interrupted_runs {
            if state.run_id.is_empty() {
                // Compatibility with states written before run IDs existed.
                state.run_id = state.flow_id.clone();
            }
            state.status = FlowStatus::RecoveryRequired;
            if state.recovery_detected_at.is_none() {
                state.recovery_detected_at = Some(self.clock.now());
            }
            let run_id = state.run_id.clone();

            self.run_flow_ids
                .write()
                .await
                .insert(run_id.clone(), state.flow_id.clone());

            self.storage.save_execution_state(&state).await?;
            {
                let mut active_flows = self.active_flows.lock().await;
                active_flows.insert(run_id.clone(), state.clone());
            }
            let wakes = self.storage.list_open_workflow_wakes(Some(&run_id)).await?;
            if let Some(error) = wakes
                .iter()
                .find_map(|wake| Self::validate_wake_resume_policy(wake, &state).err())
            {
                state.execution_log.push(ExecutionLogEntry {
                    timestamp: self.clock.now(),
                    level: LogLevel::Error,
                    node_id: None,
                    message: "Automatic recovery refused an incompatible wake policy".to_string(),
                    details: Some(json!({ "error": error })),
                });
                self.storage.save_execution_state(&state).await?;
                self.active_flows
                    .lock()
                    .await
                    .insert(run_id.clone(), state.clone());
                self.emit_event(FlowEventPayload::ExecutionStateChanged {
                    flow_id: run_id.clone(),
                    run_id,
                    execution_state: state,
                })
                .await;
                continue;
            }
            for approval in wakes.iter().filter(|wake| {
                wake.kind == WorkflowWakeKind::Approval
                    && wake.status == WorkflowWakeStatus::Waiting
            }) {
                self.emit_event(FlowEventPayload::ApprovalRequested {
                    flow_id: state.flow_id.clone(),
                    run_id: run_id.clone(),
                    request: approval.clone(),
                })
                .await;
            }
            ready_approvals.extend(
                wakes
                    .iter()
                    .filter(|wake| {
                        wake.kind == WorkflowWakeKind::Approval
                            && wake.status == WorkflowWakeStatus::Ready
                            && wake.payload.get("resolution").is_some()
                    })
                    .map(|wake| wake.id.clone()),
            );
            approval_timeouts.extend(wakes.iter().filter_map(|wake| {
                (wake.kind == WorkflowWakeKind::Approval
                    && wake.status == WorkflowWakeStatus::Waiting)
                    .then_some(wake.due_at.map(|deadline| (wake.id.clone(), deadline)))
                    .flatten()
            }));
            let timer_wakes: Vec<_> = wakes
                .iter()
                .filter(|wake| wake.kind == WorkflowWakeKind::Timer)
                .cloned()
                .collect();
            if !timer_wakes.is_empty() {
                durable_timers.push((run_id.clone(), timer_wakes));
            }
            let retry_wakes: Vec<_> = wakes
                .iter()
                .filter(|wake| wake.kind == WorkflowWakeKind::Retry)
                .cloned()
                .collect();
            if !retry_wakes.is_empty() {
                let deadline = retry_wakes
                    .iter()
                    .filter_map(|wake| wake.due_at)
                    .max()
                    .unwrap_or_else(|| self.clock.now());
                durable_retries.push((run_id.clone(), retry_wakes, deadline));
            }
            self.emit_event(FlowEventPayload::ExecutionStateChanged {
                flow_id: run_id.clone(),
                run_id,
                execution_state: state,
            })
            .await;
        }

        for (run_id, wakes) in durable_timers {
            self.schedule_durable_timer_recovery(run_id, wakes);
        }
        for (run_id, wakes, deadline) in durable_retries {
            self.schedule_durable_retry_recovery(run_id, wakes, deadline);
        }
        for (wake_id, deadline) in approval_timeouts {
            let executor = self.clone();
            let runtime_shutdown = self.runtime_shutdown_token();
            self.runtime_tasks.spawn(async move {
                tokio::select! {
                    _ = wait_until_wall_clock_deadline(executor.clock.clone(), deadline) => {}
                    _ = runtime_shutdown.cancelled() => return,
                }
                if !executor.wait_for_runtime_admission().await {
                    return;
                }
                let _ = executor
                    .resolve_approval_wake(
                        &wake_id,
                        ApprovalDecisionRequest {
                            decision: ApprovalDecision::TimedOut,
                            actor: ApprovalActor {
                                id: "workflow-runtime".to_string(),
                                display_name: Some("Workflow runtime".to_string()),
                                channel: "system".to_string(),
                            },
                            comment: Some("Approval deadline elapsed".to_string()),
                        },
                    )
                    .await;
            });
        }
        for wake_id in ready_approvals {
            let executor = self.clone();
            let runtime_shutdown = self.runtime_shutdown_token();
            self.runtime_tasks.spawn(async move {
                if runtime_shutdown.is_cancelled() {
                    return;
                }
                if !executor.wait_for_runtime_admission().await {
                    return;
                }
                let existing = executor
                    .storage
                    .list_open_workflow_wakes(None)
                    .await
                    .ok()
                    .and_then(|wakes| wakes.into_iter().find(|wake| wake.id == wake_id))
                    .and_then(|wake| Self::approval_resolution_from_wake(&wake).ok());
                let Some(resolution) = existing else {
                    log::error!("Decided approval {} has no valid resolution", wake_id);
                    return;
                };
                let request = ApprovalDecisionRequest {
                    decision: resolution.decision,
                    actor: resolution.decided_by,
                    comment: resolution.comment,
                };
                if let Err(error) = executor.resolve_approval_wake(&wake_id, request).await {
                    log::error!("Failed to restore decided approval {}: {}", wake_id, error);
                }
            });
        }

        Ok(count)
    }

    /// Seed one approval-gated run for the packaged process-kill acceptance
    /// harness. This API does not exist in ordinary development or production
    /// builds and therefore cannot be activated by an environment variable
    /// alone.
    #[cfg(feature = "acceptance-test-hooks")]
    pub async fn seed_packaged_recovery_acceptance_run(&self) -> Result<(), String> {
        const RUN_ID: &str = "acceptance-run-before-process-kill";
        const FLOW_ID: &str = "acceptance-flow";
        const NODE_ID: &str = "approval-node";

        let now = self.clock.now();
        let approval_node = FlowNodeDefinition {
            id: NODE_ID.to_string(),
            node_type: "approval".to_string(),
            name: "Packaged recovery approval".to_string(),
            inputs: HashMap::new(),
            outputs: Vec::new(),
            ai_behavior: None,
            ai_model: None,
            prompt_template: None,
            tool_name: None,
            position: (0.0, 0.0),
            ui_data: NodeUIData::default(),
            execution_config: NodeExecutionConfig::default(),
        };
        let flow_definition = FlowDefinition {
            id: FLOW_ID.to_string(),
            name: "Packaged crash recovery acceptance".to_string(),
            version: "1".to_string(),
            description: Some("Feature-gated acceptance fixture".to_string()),
            created_at: now,
            updated_at: now,
            created_by: None,
            tags: vec!["acceptance".to_string()],
            nodes: vec![approval_node.clone()],
            connections: Vec::new(),
            start_node_id: NODE_ID.to_string(),
            metadata: FlowMetadata::default(),
            is_active: false,
        };
        let resume_policy = WorkflowResumePolicy::pinned(flow_revision(&flow_definition)?);
        let node_result = NodeResult {
            node_id: NODE_ID.to_string(),
            node_name: approval_node.name.clone(),
            node_type: approval_node.node_type.clone(),
            status: NodeStatus::Running,
            inputs: HashMap::new(),
            outputs: HashMap::new(),
            error: None,
            execution_time_ms: 0,
            retry_count: 0,
        };
        let state = FlowExecutionState {
            run_id: RUN_ID.to_string(),
            flow_id: FLOW_ID.to_string(),
            flow_definition,
            status: FlowStatus::Running,
            current_node: Some(NODE_ID.to_string()),
            completed_nodes: HashSet::new(),
            failed_nodes: HashSet::new(),
            node_results: HashMap::from([(NODE_ID.to_string(), node_result.clone())]),
            execution_log: Vec::new(),
            started_at: now,
            completed_at: None,
            recovery_detected_at: None,
            pending_nodes: Vec::new(),
            sorted_node_results: vec![node_result],
            runtime_variables: RuntimeVariables::default(),
            approved_effect_bindings: HashMap::new(),
            lineage_tracker: None,
            lineage: None,
        };
        let wake = WorkflowWake {
            id: "acceptance-approval-wake".to_string(),
            run_id: RUN_ID.to_string(),
            node_id: NODE_ID.to_string(),
            kind: WorkflowWakeKind::Approval,
            status: WorkflowWakeStatus::Waiting,
            due_at: None,
            attempt: 0,
            payload: serde_json::to_value(ApprovalRequestDetails {
                title: "Packaged recovery acceptance".to_string(),
                message: "This fixture must survive a forced host termination.".to_string(),
                severity: "high".to_string(),
                requested_channels: vec!["desktop".to_string(), "mobile".to_string()],
                request_snapshot: json!({ "operation": "packaged recovery acceptance" }),
                approved_effects: Vec::new(),
                decision_policy: ApprovalDecisionPolicy::default(),
            })
            .map_err(|error| error.to_string())?,
            resume_policy,
            created_at: now,
            updated_at: now,
        };

        self.storage.save_execution_state(&state).await?;
        self.storage.save_workflow_wake(&wake).await?;
        Ok(())
    }

    /// Delete a flow
    pub async fn delete_flow(&self, flow_id: &str) -> Result<(), String> {
        self.storage.remove_active_triggers(flow_id).await?;
        self.storage.delete_flow(flow_id).await?;
        self.emit_trigger_registry_changed(flow_id).await;
        Ok(())
    }

    /// Start executing a flow and return a unique workflow run ID.
    pub async fn start_flow(&self, flow_definition: FlowDefinition) -> Result<String, String> {
        self.start_flow_with_inputs(flow_definition, HashMap::new())
            .await
    }

    pub async fn start_flow_with_inputs(
        &self,
        flow_definition: FlowDefinition,
        inputs: HashMap<String, Value>,
    ) -> Result<String, String> {
        let _admission = self.runtime_admission_gate.read().await;
        self.ensure_runtime_admission()?;
        let inputs = if let Some(service) = &self.action_invocation_service {
            let reference = crate::services::execution::ActionReference {
                kind: crate::services::execution::ActionReferenceKind::Flow,
                id: flow_definition.id.clone(),
            };
            service
                .validate_reference_inputs(&reference, &json!(inputs))
                .await?
                .into_iter()
                .collect()
        } else if inputs.is_empty() {
            inputs
        } else {
            return Err("Action input service is unavailable".into());
        };
        self.start_flow_admitted(flow_definition, false, inputs)
            .await
    }

    async fn start_flow_admitted(
        &self,
        flow_definition: FlowDefinition,
        capture_run_response: bool,
        inputs: HashMap<String, Value>,
    ) -> Result<String, String> {
        flow_definition.validate_graph()?;
        // A manually run, unsaved graph needs the same credential boundary as
        // saved graphs before its first resumable checkpoint is constructed.
        let flow_definition = tokio::task::spawn_blocking(move || {
            super::http_credentials::protect_flow(&flow_definition)
                .map(|protected| protected.commit())
        })
        .await
        .map_err(|_| "HTTP credential preparation interrupted".to_string())??;
        let flow_id = flow_definition.id.clone();
        let run_id = Uuid::new_v4().to_string();

        // Note: Flows are NOT auto-saved during execution
        // Users must explicitly save flows via save_flow command
        // This prevents temporary/untitled flows from cluttering the database

        // Initialize lineage tracker for data provenance
        let lineage_tracker = crate::services::flow::lineage::LineageTracker::new(
            flow_id.clone(),
            flow_definition.name.clone(),
            inputs.clone(),
            "manual".to_string(), // Trigger type
            None,                 // Triggered by user (can be extended later)
        );

        let mut runtime_variables = RuntimeVariables::default();
        runtime_variables.set_run_inputs(&inputs);
        let execution_state = FlowExecutionState {
            run_id: run_id.clone(),
            flow_id: flow_id.clone(),
            flow_definition: flow_definition.clone(),
            status: FlowStatus::Pending,
            current_node: None,
            completed_nodes: HashSet::new(),
            failed_nodes: HashSet::new(),
            node_results: HashMap::new(),
            execution_log: Vec::new(),
            started_at: self.clock.now(),
            completed_at: None,
            recovery_detected_at: None,
            pending_nodes: Vec::new(),
            sorted_node_results: Vec::new(),
            runtime_variables,
            approved_effect_bindings: HashMap::new(),
            lineage_tracker: Some(lineage_tracker),
            lineage: None,
        };

        // Store the execution state in memory and persistent storage
        {
            let mut active_flows = self.active_flows.lock().await;
            active_flows.insert(run_id.clone(), execution_state.clone());
        }
        self.run_flow_ids
            .write()
            .await
            .insert(run_id.clone(), flow_id.clone());

        // Save execution state for resumability
        if let Err(error) = self.storage.save_execution_state(&execution_state).await {
            self.active_flows.lock().await.remove(&run_id);
            self.run_flow_ids.write().await.remove(&run_id);
            return Err(error);
        }

        if capture_run_response {
            let (response_tx, response_rx) = tokio::sync::mpsc::unbounded_channel();
            self.run_response_senders
                .lock()
                .await
                .insert(run_id.clone(), response_tx);
            self.run_response_receivers
                .lock()
                .await
                .insert(run_id.clone(), response_rx);
        }

        let (cancellation_tx, mut cancellation_rx) = tokio::sync::watch::channel(false);
        self.cancellation_signals
            .lock()
            .await
            .insert(run_id.clone(), cancellation_tx);

        // Emit flow started event
        self.emit_event(FlowEventPayload::FlowStarted {
            flow_id: run_id.clone(),
            run_id: run_id.clone(),
            flow_name: flow_definition.name.clone(),
        })
        .await;

        // Register execution with the runtime owner before releasing the
        // admission guard held by the caller.
        let executor = self.clone();
        let run_id_clone = run_id.clone();
        let runtime_shutdown = self.runtime_shutdown_token();
        self.runtime_tasks.spawn(async move {
            tokio::select! {
                result = executor.execute_flow_internal(run_id_clone.clone()) => {
                    if let Err(error) = result {
                        log::error!("Workflow run {} failed: {}", run_id_clone, error);
                        executor.fail_run_if_active(&run_id_clone, error).await;
                    }
                }
                changed = cancellation_rx.changed() => {
                    if changed.is_ok() && *cancellation_rx.borrow() {
                        log::info!("Cancelled in-flight workflow task {}", run_id_clone);
                    }
                }
                _ = runtime_shutdown.cancelled() => {
                    log::info!("Stopped in-flight workflow task {} with its runtime owner", run_id_clone);
                }
            }
            executor
                .cancellation_signals
                .lock()
                .await
                .remove(&run_id_clone);
            executor.run_flow_ids.write().await.remove(&run_id_clone);
        });

        Ok(run_id)
    }

    /// Claim the single run-scoped response stream created before execution.
    /// A webhook response adapter calls this immediately after triggering; any
    /// signal produced by an extremely fast run is already queued.
    pub async fn take_run_response_receiver(
        &self,
        run_id: &str,
    ) -> Option<tokio::sync::mpsc::UnboundedReceiver<FlowRunResponseSignal>> {
        self.run_response_receivers.lock().await.remove(run_id)
    }

    async fn send_run_response_signal(&self, run_id: &str, signal: FlowRunResponseSignal) {
        let sender = self.run_response_senders.lock().await.get(run_id).cloned();
        if let Some(sender) = sender {
            let _ = sender.send(signal);
        }
    }

    /// Get the current status of a flow
    pub async fn get_flow_status(&self, flow_id: &str) -> Result<FlowExecutionState, String> {
        let active_flows = self.active_flows.lock().await;
        active_flows
            .get(flow_id)
            .cloned()
            .ok_or_else(|| format!("Flow {} not found", flow_id))
    }

    /// Persist non-terminal runs and stop their in-process futures without
    /// converting them into user cancellations. The next runtime owner will
    /// classify these snapshots as recovery-required before accepting work.
    pub async fn prepare_for_runtime_shutdown(&self) -> Result<usize, String> {
        let snapshots = self
            .active_flows
            .lock()
            .await
            .values()
            .filter(|state| matches!(state.status, FlowStatus::Pending | FlowStatus::Running))
            .cloned()
            .collect::<Vec<_>>();

        let mut checkpoint_errors = Vec::new();
        for state in &snapshots {
            if let Err(error) = self.storage.save_execution_state(state).await {
                checkpoint_errors.push(format!("{}: {error}", state.run_id));
            }
        }

        let signals = self
            .cancellation_signals
            .lock()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        for signal in signals {
            let _ = signal.send(true);
        }

        if checkpoint_errors.is_empty() {
            Ok(snapshots.len())
        } else {
            Err(format!(
                "Failed to checkpoint {} workflow run(s): {}",
                checkpoint_errors.len(),
                checkpoint_errors.join("; ")
            ))
        }
    }

    /// Persist the current snapshot for one workflow run. Checkpoint failures
    /// are logged but never hide the original execution error.
    async fn checkpoint_run(&self, run_id: &str) {
        let state = {
            let active_flows = self.active_flows.lock().await;
            active_flows.get(run_id).cloned()
        };

        if let Some(state) = state {
            if let Err(error) = self.storage.save_execution_state(&state).await {
                log::error!("Failed to checkpoint flow run {}: {}", run_id, error);
            }
        }
    }

    /// Finalize errors that happen before execute_flow_internal reaches its
    /// normal terminal-state branch (for example an invalid entry point).
    /// If the run is already terminal this is a no-op, preventing duplicate
    /// failure events.
    async fn fail_run_if_active(&self, run_id: &str, error: String) {
        let failed_state = {
            let mut active_flows = self.active_flows.lock().await;
            let Some(state) = active_flows.get_mut(run_id) else {
                return;
            };
            if !matches!(
                state.status,
                FlowStatus::Pending | FlowStatus::Running | FlowStatus::RecoveryRequired
            ) {
                return;
            }
            state.status = FlowStatus::Failed;
            state.current_node = None;
            state.completed_at = Some(self.clock.now());
            state.pending_nodes.clear();
            for node_result in state.node_results.values_mut() {
                if matches!(
                    node_result.status,
                    NodeStatus::Pending | NodeStatus::Running
                ) {
                    node_result.status = NodeStatus::Skipped;
                    node_result.error = Some("Run failed before completion".to_string());
                }
            }
            state.execution_log.push(ExecutionLogEntry {
                timestamp: self.clock.now(),
                level: LogLevel::Error,
                node_id: None,
                message: error.clone(),
                details: None,
            });
            state.clone()
        };

        if let Err(storage_error) = self.storage.save_execution_state(&failed_state).await {
            log::error!(
                "Failed to checkpoint failed run {}: {}",
                run_id,
                storage_error
            );
        }
        let finalized = self.archive_terminal_state(&failed_state).await;
        self.emit_event(FlowEventPayload::ExecutionStateChanged {
            flow_id: run_id.to_string(),
            run_id: run_id.to_string(),
            execution_state: failed_state,
        })
        .await;
        self.send_run_response_signal(run_id, FlowRunResponseSignal::Failed(error.clone()))
            .await;
        self.emit_event(FlowEventPayload::FlowFailed {
            flow_id: run_id.to_string(),
            run_id: run_id.to_string(),
            error,
        })
        .await;
        if finalized {
            self.release_terminal_run(run_id).await;
        }
    }

    async fn release_terminal_run(&self, run_id: &str) {
        self.active_flows.lock().await.remove(run_id);
        self.run_flow_ids.write().await.remove(run_id);
        self.cancellation_signals.lock().await.remove(run_id);

        // Keep an unclaimed response queue briefly so an extremely fast run
        // cannot finish and erase its terminal signal between `start_flow`
        // returning and the webhook adapter claiming the receiver.
        let has_response_queue = self.run_response_senders.lock().await.contains_key(run_id)
            || self
                .run_response_receivers
                .lock()
                .await
                .contains_key(run_id);
        if !has_response_queue {
            return;
        }
        let run_id = run_id.to_string();
        let senders = self.run_response_senders.clone();
        let receivers = self.run_response_receivers.clone();
        let runtime_shutdown = self.runtime_shutdown_token();
        self.runtime_tasks.spawn(async move {
            tokio::select! {
                _ = tokio::time::sleep(std::time::Duration::from_secs(60)) => {}
                _ = runtime_shutdown.cancelled() => {}
            }
            senders.lock().await.remove(&run_id);
            receivers.lock().await.remove(&run_id);
        });
    }

    async fn archive_terminal_state(&self, state: &FlowExecutionState) -> bool {
        match self.persist_terminal_history(state).await {
            Ok(()) => true,
            Err(error) => {
                log::error!(
                    "Failed to archive terminal workflow run {}: {}",
                    state.run_id,
                    error
                );
                false
            }
        }
    }

    async fn is_run_cancelled(&self, run_id: &str) -> bool {
        let active_flows = self.active_flows.lock().await;
        active_flows
            .get(run_id)
            .is_some_and(|state| matches!(state.status, FlowStatus::Cancelled))
    }

    /// Cancel a running flow
    pub async fn cancel_flow(&self, flow_id: &str) -> Result<(), String> {
        let _admission = self.runtime_admission_gate.read().await;
        self.ensure_runtime_admission()?;
        let cancelled_state = {
            let mut active_flows = self.active_flows.lock().await;
            if let Some(state) = active_flows.get_mut(flow_id) {
                state.status = FlowStatus::Cancelled;
                state.current_node = None;
                state.completed_at = Some(self.clock.now());
                state.pending_nodes.clear();
                for node_result in state.node_results.values_mut() {
                    if matches!(
                        node_result.status,
                        NodeStatus::Pending | NodeStatus::Running
                    ) {
                        node_result.status = NodeStatus::Skipped;
                        node_result.error = Some("Run cancelled".to_string());
                    }
                }
                state.clone()
            } else {
                return Err(format!("Flow {} not found", flow_id));
            }
        };

        if let Some(signal) = self.cancellation_signals.lock().await.get(flow_id).cloned() {
            let _ = signal.send(true);
        }
        // A cancelled run must not leave visual work eligible for browser
        // reconnect replay. Expire first, then drop any in-process waiter so a
        // late ACK cannot revive or complete the cancelled node.
        let overlay_cancellation_error =
            self.cancel_pending_overlay_deliveries(flow_id).await.err();
        for wake in self.storage.list_open_workflow_wakes(Some(flow_id)).await? {
            if wake.kind == WorkflowWakeKind::Approval && wake.status == WorkflowWakeStatus::Waiting
            {
                let resolution = ApprovalResolution {
                    decision: ApprovalDecision::Cancelled,
                    decided_at: self.clock.now(),
                    decided_by: ApprovalActor {
                        id: "workflow-runtime".to_string(),
                        display_name: Some("Workflow runtime".to_string()),
                        channel: "system".to_string(),
                    },
                    comment: Some("Workflow run was cancelled".to_string()),
                };
                if let Ok(outcome) = self
                    .storage
                    .resolve_workflow_approval(&wake.id, &resolution)
                    .await
                {
                    let persisted = Self::approval_resolution_from_wake(outcome.wake())?;
                    if outcome.was_applied() {
                        self.emit_event(FlowEventPayload::ApprovalResolved {
                            flow_id: wake.run_id.clone(),
                            run_id: wake.run_id.clone(),
                            wake_id: wake.id.clone(),
                            resolution: persisted.clone(),
                        })
                        .await;
                    }
                    if let Some(sender) = self.pending_approval_wakes.lock().await.remove(&wake.id)
                    {
                        let _ = sender.send(persisted);
                    }
                }
                let _ = self
                    .storage
                    .set_workflow_wake_status(&wake.id, WorkflowWakeStatus::Cancelled)
                    .await;
            } else {
                let _ = self
                    .storage
                    .set_workflow_wake_status(&wake.id, WorkflowWakeStatus::Cancelled)
                    .await;
            }
        }

        // Add to execution history for cancelled flows
        let finalized = self.archive_terminal_state(&cancelled_state).await;

        // Emit final execution state change for cancellation
        self.emit_event(FlowEventPayload::ExecutionStateChanged {
            flow_id: flow_id.to_string(),
            run_id: flow_id.to_string(),
            execution_state: cancelled_state,
        })
        .await;

        self.emit_event(FlowEventPayload::FlowCancelled {
            flow_id: flow_id.to_string(),
            run_id: flow_id.to_string(),
        })
        .await;

        if finalized {
            self.release_terminal_run(flow_id).await;
        }

        if let Some(error) = overlay_cancellation_error {
            Err(format!(
                "Flow was cancelled, but pending overlay delivery cleanup failed: {error}"
            ))
        } else {
            Ok(())
        }
    }

    async fn cancel_pending_overlay_deliveries(&self, run_id: &str) -> Result<(), String> {
        let command_ids = self
            .storage
            .expire_pending_overlay_command_deliveries_for_run(run_id)
            .await?;
        let mut waiters = self.pending_overlay_acks.lock().await;
        for command_id in command_ids {
            waiters.remove(&command_id);
        }
        Ok(())
    }
    // EVENT DE-DUPLICATION & ROUTING
    // ============================================================================

    /// Check if event should trigger (de-duplication + cleanup)
    async fn should_trigger_deduplicated(&self, event: &TriggerEvent) -> Result<bool, String> {
        // Generate dedup key
        let key = event.dedup_key.as_ref().cloned().unwrap_or_else(|| {
            // Default key: source:flow:node
            format!(
                "{}:{}:{}:{}",
                match event.source {
                    TriggerSource::Backend => "backend",
                    TriggerSource::LocalWebhook => "local_webhook",
                    TriggerSource::LocalPolling => "local_polling",
                    TriggerSource::Integration => "integration",
                    TriggerSource::Manual => "manual",
                    TriggerSource::Scheduled => "scheduled",
                },
                event.flow_id,
                event.node_id,
                event
                    .event_id
                    .clone()
                    .unwrap_or_else(|| event.timestamp.timestamp_millis().to_string())
            )
        });

        let cutoff = self.clock.now() - chrono::Duration::seconds(10);

        // Persist connector/webhook idempotency keys before starting side
        // effects. This survives restarts; the in-memory map above remains a
        // cheap fast-path for bursty duplicate delivery.
        if event.dedup_key.is_some()
            && !self
                .storage
                .claim_trigger_dedup(&key, event.event_id.as_deref(), event.timestamp)
                .await?
        {
            log::info!("⏭️ Skipping durable duplicate event: {}", key);
            return Ok(false);
        }

        let mut triggers = self.recent_triggers.write().await;
        // Check if triggered recently (within 10 seconds)
        if let Some(last_trigger) = triggers.get(&key) {
            if *last_trigger > cutoff {
                log::info!(
                    "⏭️ Skipping duplicate event: {} (last: {:?})",
                    key,
                    last_trigger
                );
                return Ok(false);
            }
        }

        // Record this trigger
        triggers.insert(key, self.clock.now());

        // Cleanup old entries (older than 10 seconds)
        triggers.retain(|_, timestamp| *timestamp > cutoff);

        Ok(true)
    }

    /// Trigger a flow execution from an event (with source checking and de-duplication)
    pub async fn trigger_flow_from_event(&self, event: TriggerEvent) -> Result<String, String> {
        self.trigger_flow_from_event_internal(event, false).await
    }

    async fn trigger_flow_from_event_internal(
        &self,
        event: TriggerEvent,
        capture_run_response: bool,
    ) -> Result<String, String> {
        let _admission = self.runtime_admission_gate.read().await;
        self.ensure_runtime_admission()?;
        log::info!(
            "🎯 Triggering flow from event: {} (source: {:?})",
            event.flow_id,
            event.source
        );

        // Check de-duplication
        if !self.should_trigger_deduplicated(&event).await? {
            return Err("Duplicate event filtered".to_string());
        }

        // Load the flow definition
        let mut flow_definition = self.storage.load_flow(&event.flow_id).await?;

        // Check if flow is active (skip for LocalWebhook - manual webhooks don't need activation)
        if !flow_definition.is_active && event.source != TriggerSource::LocalWebhook {
            return Err(format!("Flow {} is not active", event.flow_id));
        }

        // Inject event data into the trigger node
        if let Some(trigger_node) = flow_definition
            .nodes
            .iter_mut()
            .find(|n| n.id == event.node_id)
        {
            // Add event data to node inputs
            trigger_node
                .inputs
                .insert("trigger_data".to_string(), event.data.clone());
            trigger_node
                .inputs
                .insert("trigger_source".to_string(), json!(event.source));
            trigger_node
                .inputs
                .insert("trigger_timestamp".to_string(), json!(event.timestamp));
            if let Some(event_id) = &event.event_id {
                trigger_node
                    .inputs
                    .insert("trigger_event_id".to_string(), json!(event_id));
            }
            if let Some(correlation_id) = &event.correlation_id {
                trigger_node
                    .inputs
                    .insert("trigger_correlation_id".to_string(), json!(correlation_id));
            }
        } else {
            return Err(format!(
                "Trigger node {} not found in flow {}",
                event.node_id, event.flow_id
            ));
        }

        // Start flow execution
        self.start_flow_admitted(flow_definition, capture_run_response, HashMap::new())
            .await
    }

    /// Trigger a flow execution from a webhook (legacy method - converts to TriggerEvent)
    /// This is called when an external webhook is received
    pub async fn trigger_flow_from_webhook(
        &self,
        flow_id: &str,
        webhook_data: Value,
        event_id: Option<String>,
        capture_run_response: bool,
    ) -> Result<String, String> {
        log::info!("🔗 Triggering flow from webhook: {}", flow_id);

        // Load flow to find webhook node
        let flow_definition = self.storage.load_flow(flow_id).await?;

        // Find webhook trigger node
        let webhook_node_id = flow_definition
            .nodes
            .iter()
            .find(|n| n.node_type.contains("webhook") || n.node_type == "webhook-trigger")
            .map(|n| n.id.clone())
            .ok_or_else(|| "No webhook trigger node found in flow".to_string())?;

        // Only provider-supplied delivery IDs are safe durable deduplication
        // keys. Hashing a payload would incorrectly collapse two legitimate
        // identical webhook invocations.
        let dedup_key = event_id
            .as_ref()
            .map(|event_id| format!("local_webhook:{}:{}:{}", flow_id, webhook_node_id, event_id));

        // Convert to TriggerEvent
        let event = TriggerEvent {
            source: TriggerSource::LocalWebhook,
            event_id,
            correlation_id: None,
            flow_id: flow_id.to_string(),
            node_id: webhook_node_id,
            data: webhook_data,
            timestamp: self.clock.now(),
            dedup_key,
        };

        // Trigger via event system
        self.trigger_flow_from_event_internal(event, capture_run_response)
            .await
    }

    /// Internal flow execution logic
    async fn execute_flow_internal(&self, flow_id: String) -> Result<(), String> {
        // Update status to running
        {
            let mut active_flows = self.active_flows.lock().await;
            if let Some(state) = active_flows.get_mut(&flow_id) {
                state.status = FlowStatus::Running;
            }
        }
        self.checkpoint_run(&flow_id).await;

        // Get the flow definition
        let flow_definition = {
            let active_flows = self.active_flows.lock().await;
            active_flows
                .get(&flow_id)
                .ok_or_else(|| "Flow not found".to_string())?
                .flow_definition
                .clone()
        };

        // Find all entry points (nodes with no incoming connections OR explicit triggers)
        let entry_points = self.find_flow_entry_points(&flow_definition);

        if entry_points.is_empty() {
            return Err("No entry points found in flow. Add a trigger node or ensure nodes are properly connected.".to_string());
        }

        println!("🚀 Found {} flow entry point(s):", entry_points.len());
        for entry in &entry_points {
            println!("   - {} ({})", entry.name, entry.node_type);
        }

        let final_result = self.execute_flow_graph(&flow_id, &flow_definition).await;

        match final_result {
            Ok(_) => {
                // Mark flow as completed (or completed with errors if any nodes failed)
                let (final_result, completed_state) = {
                    let mut active_flows = self.active_flows.lock().await;
                    if let Some(state) = active_flows.get_mut(&flow_id) {
                        // Check if any nodes failed during execution
                        state.status = if state.failed_nodes.is_empty() {
                            FlowStatus::Completed
                        } else {
                            FlowStatus::CompletedWithErrors
                        };
                        state.current_node = None;
                        state.completed_at = Some(Utc::now());

                        // Collect final outputs
                        let result = state
                            .node_results
                            .values()
                            .flat_map(|result| result.outputs.clone())
                            .collect::<HashMap<String, Value>>();

                        // 🆕 Finalize lineage tracking
                        if let Some(mut tracker) = state.lineage_tracker.take() {
                            tracker.complete_flow(result.clone());
                            state.lineage = Some(tracker.get_lineage().clone());
                        }

                        (result, state.clone())
                    } else {
                        (
                            HashMap::new(),
                            FlowExecutionState {
                                run_id: flow_id.clone(),
                                flow_id: flow_id.clone(),
                                flow_definition: flow_definition.clone(),
                                status: FlowStatus::Completed,
                                current_node: None,
                                completed_nodes: HashSet::new(),
                                failed_nodes: HashSet::new(),
                                node_results: HashMap::new(),
                                execution_log: Vec::new(),
                                started_at: Utc::now(),
                                completed_at: Some(Utc::now()),
                                recovery_detected_at: None,
                                pending_nodes: Vec::new(),
                                sorted_node_results: Vec::new(),
                                runtime_variables: RuntimeVariables::default(),
                                approved_effect_bindings: HashMap::new(),
                                lineage_tracker: None,
                                lineage: None,
                            },
                        )
                    }
                };

                // Add to execution history
                let finalized = self.archive_terminal_state(&completed_state).await;

                // Emit final execution state change for completion
                self.emit_event(FlowEventPayload::ExecutionStateChanged {
                    flow_id: flow_id.clone(),
                    run_id: flow_id.clone(),
                    execution_state: completed_state,
                })
                .await;

                self.send_run_response_signal(
                    &flow_id,
                    FlowRunResponseSignal::Completed(
                        serde_json::to_value(&final_result).unwrap_or(Value::Null),
                    ),
                )
                .await;
                self.emit_event(FlowEventPayload::FlowCompleted {
                    flow_id: flow_id.clone(),
                    run_id: flow_id.clone(),
                    result: final_result,
                })
                .await;
                if finalized {
                    self.release_terminal_run(&flow_id).await;
                }

                Ok(())
            }
            Err(e) => {
                // Mark flow as failed
                let failed_state = {
                    let mut active_flows = self.active_flows.lock().await;
                    if let Some(state) = active_flows.get_mut(&flow_id) {
                        state.status = FlowStatus::Failed;
                        state.current_node = None;
                        state.completed_at = Some(Utc::now());
                        state.pending_nodes.clear();
                        for node_result in state.node_results.values_mut() {
                            if matches!(
                                node_result.status,
                                NodeStatus::Pending | NodeStatus::Running
                            ) {
                                node_result.status = NodeStatus::Skipped;
                                node_result.error =
                                    Some("Run stopped after another node failed".to_string());
                            }
                        }
                        state.execution_log.push(ExecutionLogEntry {
                            timestamp: Utc::now(),
                            level: LogLevel::Error,
                            node_id: None,
                            message: e.clone(),
                            details: None,
                        });

                        // 🆕 Finalize lineage tracking for failed flow
                        if let Some(mut tracker) = state.lineage_tracker.take() {
                            tracker.fail_flow();
                            state.lineage = Some(tracker.get_lineage().clone());
                        }

                        state.clone()
                    } else {
                        FlowExecutionState {
                            run_id: flow_id.clone(),
                            flow_id: flow_id.clone(),
                            flow_definition: flow_definition.clone(),
                            status: FlowStatus::Failed,
                            current_node: None,
                            completed_nodes: HashSet::new(),
                            failed_nodes: HashSet::new(),
                            node_results: HashMap::new(),
                            execution_log: Vec::new(),
                            started_at: Utc::now(),
                            completed_at: Some(Utc::now()),
                            recovery_detected_at: None,
                            pending_nodes: Vec::new(),
                            sorted_node_results: Vec::new(),
                            runtime_variables: RuntimeVariables::default(),
                            approved_effect_bindings: HashMap::new(),
                            lineage_tracker: None,
                            lineage: None,
                        }
                    }
                };

                // Add to execution history for failed flows too
                let finalized = self.archive_terminal_state(&failed_state).await;

                // Emit final execution state change for failure
                self.emit_event(FlowEventPayload::ExecutionStateChanged {
                    flow_id: flow_id.clone(),
                    run_id: flow_id.clone(),
                    execution_state: failed_state,
                })
                .await;

                self.send_run_response_signal(&flow_id, FlowRunResponseSignal::Failed(e.clone()))
                    .await;
                self.emit_event(FlowEventPayload::FlowFailed {
                    flow_id: flow_id.clone(),
                    run_id: flow_id.clone(),
                    error: e.clone(),
                })
                .await;
                if finalized {
                    self.release_terminal_run(&flow_id).await;
                }

                Err(e)
            }
        }
    }

    async fn execute_single_node(
        &self,
        flow_id: &str,
        node_def: &FlowNodeDefinition,
    ) -> Result<NodeResult, String> {
        // Ensure connected value dependencies are ready. Value-only nodes can be evaluated lazily.
        self.ensure_value_dependencies_ready(flow_id, node_def)
            .await?;
        self.execute_single_node_without_value_dependency_resolution(flow_id, node_def)
            .await
    }

    /// Execute a single node assuming value dependencies are already resolved.
    async fn execute_single_node_without_value_dependency_resolution(
        &self,
        flow_id: &str,
        node_def: &FlowNodeDefinition,
    ) -> Result<NodeResult, String> {
        let start_time = std::time::Instant::now();
        let started_at = Utc::now();
        tracing::info!(
            target: "vortideck.workflow",
            event = "node_started",
            run_id = flow_id,
            node_id = %node_def.id,
            node_type = %node_def.node_type,
            "workflow node started"
        );

        // Build complete inputs for this node (including previous outputs)
        let mut node_inputs = self.build_node_inputs(flow_id, node_def).await?;
        if node_def.node_type == "http_request" {
            node_inputs = tokio::task::spawn_blocking(move || {
                super::http_credentials::protect_resolved_inputs(node_inputs)
                    .map(|inputs| inputs.commit())
            })
            .await
            .map_err(|_| "HTTP credential preparation interrupted".to_string())??;
        }

        // Emit node started event
        self.emit_event(FlowEventPayload::NodeStarted {
            flow_id: flow_id.to_string(),
            run_id: flow_id.to_string(),
            node_id: node_def.id.clone(),
            node_name: node_def.name.clone(),
        })
        .await;

        let mut retry_count = 0;
        // Update current node
        {
            let mut active_flows = self.active_flows.lock().await;
            if let Some(state) = active_flows.get_mut(flow_id) {
                state.current_node = Some(node_def.id.clone());
                retry_count = state
                    .node_results
                    .get(&node_def.id)
                    .map(|result| result.retry_count)
                    .unwrap_or(0);

                // 🆕 Mark node as Running in node_results for concurrent tracking
                let running_node_result = NodeResult {
                    node_id: node_def.id.clone(),
                    node_name: node_def.name.clone(),
                    node_type: node_def.node_type.clone(),
                    status: NodeStatus::Running,
                    inputs: node_inputs.clone(),
                    outputs: HashMap::new(),
                    error: None,
                    execution_time_ms: 0,
                    retry_count,
                };
                state
                    .node_results
                    .insert(node_def.id.clone(), running_node_result);

                // Emit execution state change to update frontend with current node
                self.emit_event(FlowEventPayload::ExecutionStateChanged {
                    flow_id: flow_id.to_string(),
                    run_id: flow_id.to_string(),
                    execution_state: state.clone(),
                })
                .await;
            }
        }
        // A long-running operation (including an ACK wait) must leave a
        // recoverable checkpoint before it starts, not only after it finishes.
        self.checkpoint_run(flow_id).await;

        let result = loop {
            let operation = self.execute_node_operation(flow_id, node_def, &node_inputs);
            let operation_timeout =
                if node_def.node_type == "approval" || node_def.node_type == "invoke-action" {
                    None
                } else {
                    node_def.execution_config.timeout_ms
                };
            let attempt = if let Some(timeout_ms) = operation_timeout.filter(|value| *value > 0) {
                match tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), operation)
                    .await
                {
                    Ok(result) => result,
                    Err(_) => Err(format!(
                        "Node '{}' timed out after {}ms",
                        node_def.name, timeout_ms
                    )),
                }
            } else {
                operation.await
            };

            match attempt {
                Ok(outputs) => break Ok(outputs),
                Err(error) if retry_count < node_def.execution_config.retry_count => {
                    if node_def.node_type == "invoke-action" {
                        if let Err(policy_error) = self
                            .authorize_automatic_action_retry(flow_id, node_def, &node_inputs)
                            .await
                        {
                            break Err(format!(
                                "{}; automatic retry was blocked: {}",
                                error, policy_error
                            ));
                        }
                    }
                    retry_count += 1;
                    log::warn!(
                        "Retrying node '{}' ({}/{}) after error: {}",
                        node_def.name,
                        retry_count,
                        node_def.execution_config.retry_count,
                        error
                    );
                    {
                        let mut active_flows = self.active_flows.lock().await;
                        if let Some(state) = active_flows.get_mut(flow_id) {
                            if let Some(node_result) = state.node_results.get_mut(&node_def.id) {
                                node_result.retry_count = retry_count;
                                node_result.error = Some(error.clone());
                            }
                        }
                    }
                    self.checkpoint_run(flow_id).await;
                    let delay_ms = node_def.execution_config.retry_delay_ms;
                    let due_at =
                        self.clock
                            .now()
                            .checked_add_signed(
                                chrono::Duration::try_milliseconds(
                                    delay_ms.min(i64::MAX as u64) as i64
                                )
                                .ok_or("Retry delay is outside the supported range")?,
                            )
                            .ok_or("Retry deadline is outside the supported date range")?;
                    let now = self.clock.now();
                    let resume_policy = self.resume_policy_for_run(flow_id).await?;
                    let wake = WorkflowWake {
                        id: format!("retry:{}:{}:{}", flow_id, node_def.id, retry_count),
                        run_id: flow_id.to_string(),
                        node_id: node_def.id.clone(),
                        kind: WorkflowWakeKind::Retry,
                        status: WorkflowWakeStatus::Waiting,
                        due_at: Some(due_at),
                        attempt: retry_count,
                        payload: json!({ "lastError": error }),
                        resume_policy,
                        created_at: now,
                        updated_at: now,
                    };
                    self.storage.save_workflow_wake(&wake).await?;
                    wait_until_wall_clock_deadline(self.clock.clone(), due_at).await;
                    self.storage
                        .set_workflow_wake_status(&wake.id, WorkflowWakeStatus::Ready)
                        .await?;
                }
                Err(error) => break Err(error),
            }
        };

        let execution_time = start_time.elapsed().as_millis() as u64;

        let (node_result, terminal_error) = match result {
            Ok(mut outputs) => {
                // Check if outputs contain an execution error (added by execute_flow_node)
                let error_msg = outputs
                    .remove("__execution_error")
                    .and_then(|v| v.as_str().map(|s| s.to_string()));

                let has_error = error_msg.is_some();
                if has_error
                    && node_def.execution_config.skip_on_error
                    && node_def
                        .outputs
                        .iter()
                        .any(|output| output.output_type == "flow" && output.id == "error")
                {
                    let message = error_msg
                        .as_deref()
                        .unwrap_or("Node execution failed")
                        .to_string();
                    outputs.insert("selectedRoute".to_string(), json!("error"));
                    outputs.insert("errorMessage".to_string(), json!(message));
                    outputs.insert(
                        "errorDetails".to_string(),
                        json!({
                            "nodeId": node_def.id,
                            "nodeType": node_def.node_type,
                            "message": error_msg,
                            "retryCount": retry_count,
                        }),
                    );
                }

                let node_result = NodeResult {
                    node_id: node_def.id.clone(),
                    node_name: node_def.name.clone(),
                    node_type: node_def.node_type.clone(),
                    status: if has_error {
                        NodeStatus::Failed
                    } else {
                        NodeStatus::Completed
                    },
                    inputs: node_inputs.clone(),
                    outputs, // ✅ Outputs preserved even on error!
                    error: error_msg.clone(),
                    execution_time_ms: execution_time,
                    retry_count,
                };

                // Emit appropriate event based on whether there was an error
                if has_error {
                    self.emit_event(FlowEventPayload::NodeFailed {
                        flow_id: flow_id.to_string(),
                        run_id: flow_id.to_string(),
                        node_id: node_def.id.clone(),
                        error: error_msg
                            .clone()
                            .unwrap_or_else(|| "Unknown error".to_string()),
                    })
                    .await;
                } else {
                    self.emit_event(FlowEventPayload::NodeCompleted {
                        flow_id: flow_id.to_string(),
                        run_id: flow_id.to_string(),
                        node_id: node_def.id.clone(),
                        result: node_result.clone(),
                    })
                    .await;
                }

                // Check if this is a RespondToWebhookNode (for ResponseNode mode)
                if node_def.node_type == "respond-to-webhook" {
                    log::info!(
                        "📤 RespondToWebhookNode executed, emitting WebhookResponseReady event"
                    );

                    // Convert outputs HashMap to JSON Value
                    let response_data = serde_json::to_value(&node_result.outputs)
                        .unwrap_or_else(|_| serde_json::json!({}));

                    self.emit_event(FlowEventPayload::WebhookResponseReady {
                        flow_id: flow_id.to_string(),
                        run_id: flow_id.to_string(),
                        node_id: node_def.id.clone(),
                        response_data: response_data.clone(),
                    })
                    .await;
                    self.send_run_response_signal(
                        flow_id,
                        FlowRunResponseSignal::ResponseNode(response_data),
                    )
                    .await;
                }

                (node_result, error_msg)
            }
            Err(error) => {
                let mut outputs = HashMap::new();
                if node_def.execution_config.skip_on_error
                    && node_def
                        .outputs
                        .iter()
                        .any(|output| output.output_type == "flow" && output.id == "error")
                {
                    outputs.insert("selectedRoute".to_string(), json!("error"));
                    outputs.insert("errorMessage".to_string(), json!(error.clone()));
                    outputs.insert(
                        "errorDetails".to_string(),
                        json!({
                            "nodeId": node_def.id,
                            "nodeType": node_def.node_type,
                            "message": error.clone(),
                            "retryCount": retry_count,
                        }),
                    );
                }
                let node_result = NodeResult {
                    node_id: node_def.id.clone(),
                    node_name: node_def.name.clone(),
                    node_type: node_def.node_type.clone(),
                    status: NodeStatus::Failed,
                    inputs: node_inputs.clone(),
                    outputs,
                    error: Some(error.clone()),
                    execution_time_ms: execution_time,
                    retry_count,
                };

                // Emit failure event
                self.emit_event(FlowEventPayload::NodeFailed {
                    flow_id: flow_id.to_string(),
                    run_id: flow_id.to_string(),
                    node_id: node_def.id.clone(),
                    error: error.clone(),
                })
                .await;

                (node_result, Some(error))
            }
        };

        // Store the result
        let updated_state = {
            let mut active_flows = self.active_flows.lock().await;
            if let Some(state) = active_flows.get_mut(flow_id) {
                state.completed_nodes.insert(node_def.id.clone());
                state
                    .node_results
                    .insert(node_def.id.clone(), node_result.clone());

                // Track failed nodes for CompletedWithErrors status
                if node_result.status == NodeStatus::Failed {
                    state.failed_nodes.insert(node_def.id.clone());
                }

                // 🆕 Add to sorted results in execution order
                state.sorted_node_results.push(node_result.clone());

                Some(state.clone())
            } else {
                None
            }
        };

        if let Some(state) = updated_state {
            self.emit_progress_update(flow_id, &state).await;
            self.emit_event(FlowEventPayload::ExecutionStateChanged {
                flow_id: flow_id.to_string(),
                run_id: flow_id.to_string(),
                execution_state: state,
            })
            .await;
        }

        self.checkpoint_run(flow_id).await;

        // The node result is durable before its scheduler records are closed.
        // If the process dies between these steps, recovery sees both the
        // completed node and the still-open wake and consumes it idempotently.
        if let Ok(wakes) = self.storage.list_open_workflow_wakes(Some(flow_id)).await {
            for wake in wakes.into_iter().filter(|wake| wake.node_id == node_def.id) {
                if let Err(error) = self
                    .storage
                    .set_workflow_wake_status(&wake.id, WorkflowWakeStatus::Consumed)
                    .await
                {
                    log::error!("Failed to close workflow wake {}: {}", wake.id, error);
                }
            }
        }

        tracing::info!(
            target: "vortideck.workflow",
            event = "node_finished",
            run_id = flow_id,
            node_id = %node_def.id,
            node_type = %node_def.node_type,
            status = ?node_result.status,
            retry_count = node_result.retry_count,
            duration_ms = node_result.execution_time_ms,
            error = node_result.error.as_deref().unwrap_or(""),
            "workflow node finished"
        );

        if let Some(error) = terminal_error {
            if !node_def.execution_config.skip_on_error {
                return Err(error);
            }
        }

        Ok(node_result)
    }

    async fn execute_node_operation(
        &self,
        flow_id: &str,
        node_def: &FlowNodeDefinition,
        node_inputs: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, String> {
        match node_def.node_type.as_str() {
            "hybrid-llm-free" | "hybrid-llm-guided" | "hybrid-llm-strict" | "ai-node"
            | "ai-orchestrator" | "ai-free-agent" | "ai-text-generator" | "ai-strict-executor" => {
                self.execute_ai_node(flow_id, node_def, node_inputs).await
            }
            node_type if node_type.starts_with("tool") => {
                self.execute_tool_node(flow_id, node_def, node_inputs).await
            }
            _ => {
                println!(
                    "Routing '{}' to standard node execution",
                    node_def.node_type
                );
                self.execute_standard_node(flow_id, node_def, node_inputs)
                    .await
            }
        }
    }

    /// Execute an AI node using the new AI executor
    async fn execute_ai_node(
        &self,
        flow_id: &str,
        node_def: &FlowNodeDefinition,
        node_inputs: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, String> {
        println!(
            "🤖 EXECUTING AI NODE: {} (Type: {})",
            node_def.name, node_def.node_type
        );
        println!("🧠 AI Behavior: {:?}", node_def.ai_behavior);

        // Get connected tools (for non-Free Agent nodes)
        let connected_tools = self.get_connected_tools(flow_id, node_def).await?;

        println!("🔧 Connected tools count: {}", connected_tools.len());
        for tool in &connected_tools {
            println!(
                "   - {} (ID: {}, Type: {:?})",
                tool.name, tool.node_id, tool.tool_type
            );
        }

        // Build user message from merged inputs
        let user_message = node_inputs
            .get("user_message")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .unwrap_or("Complete the requested task")
            .to_string();

        println!("💬 User Goal: {}", user_message);

        // Convert template ID to PromptTemplate object
        let prompt_template = if let Some(template_id) = &node_def.prompt_template {
            // Get built-in template by ID
            crate::services::mcp::ai_node::PromptTemplate::get_builtin_templates()
                .into_iter()
                .find(|t| t.id == *template_id)
        } else {
            None
        };

        // Create AI node configuration
        let config = AiNodeConfig {
            behavior: if prompt_template.is_some() {
                // For template-based nodes, use TextOnly behavior for clean text generation
                crate::services::mcp::ai_node::AiNodeBehavior::TextOnly
            } else {
                node_def
                    .ai_behavior
                    .clone()
                    .unwrap_or(AiNodeBehavior::Orchestrator)
            },
            tool_access: if prompt_template.is_some() {
                // Template nodes don't need tools
                crate::services::mcp::ai_node::ToolAccessPolicy::NoTools
            } else {
                ToolAccessPolicy::ConnectedOnly
            },
            response_format: if prompt_template.is_some() {
                // Template nodes should use natural language output
                crate::services::mcp::ai_node::ResponseFormat::Natural
            } else {
                ResponseFormat::Structured
            },
            custom_prompt: None,
            model: node_def.ai_model.clone(),
            input_mode: Some(if prompt_template.is_some() {
                crate::services::mcp::ai_node::InputMode::CustomTemplate
            } else {
                crate::services::mcp::ai_node::InputMode::TaskExecution
            }),
            prompt_template,
        };

        println!(
            "⚙️ AI Config: Behavior={:?}, ToolAccess={:?}, ResponseFormat={:?}",
            config.behavior, config.tool_access, config.response_format
        );

        // Build AI node input
        let ai_input = AiNodeInput {
            user_message,
            previous_outputs: node_inputs.clone(),
            context_memory: Vec::new(), // TODO: Implement context memory
            available_tools: connected_tools,
            branching_conditions: None,
            flow_context: FlowContext {
                flow_run_id: flow_id.to_string(),
                current_node_id: node_def.id.clone(),
                next_node_types: vec!["tool".to_string()], // TODO: Get actual next types
                expected_outputs: vec!["result".to_string(), "success".to_string()],
            },
            config: config.clone(),
        };

        println!(
            "🚀 Calling AI executor with {} available tools...",
            ai_input.available_tools.len()
        );

        // Execute using the new AI executor
        let output = self.ai_executor.execute_ai_node(config, ai_input).await?;

        println!("✅ AI node completed with {} outputs", output.outputs.len());

        // Map AI executor outputs to node schema output keys
        let mut mapped_outputs = HashMap::new();

        // Get the node's output schema to map to correct keys
        let node_registry = self
            .flow_registry
            .registry
            .lock()
            .map_err(|e| e.to_string())?;
        match node_registry.get_node(&node_def.node_type) {
            Some(node) => {
                let output_schema = node.outputs_schema();

                // For AI text generators, map "response" to the defined output key
                if let Some(response) = output.outputs.get("response") {
                    if let Some(first_output) = output_schema.first() {
                        println!(
                            "🔗 Mapping AI response to output key: {} -> {}",
                            "response", first_output.id
                        );
                        mapped_outputs.insert(first_output.id.clone(), response.clone());
                    } else {
                        // Fallback to "result" if no schema found
                        mapped_outputs.insert("result".to_string(), response.clone());
                    }
                }

                // Also include any other outputs as-is
                for (key, value) in output.outputs {
                    if key != "response" {
                        mapped_outputs.insert(key, value);
                    }
                }
            }
            _ => {
                // Fallback: use outputs as-is if no schema found
                mapped_outputs = output.outputs;
            }
        }

        println!(
            "📤 Final mapped outputs: {:?}",
            mapped_outputs.keys().collect::<Vec<_>>()
        );
        Ok(mapped_outputs)
    }

    /// Check if a tool node is connected to an AI node
    async fn is_tool_connected_to_ai_node(
        &self,
        flow_id: &str,
        tool_node_id: &str,
    ) -> Result<bool, String> {
        let active_flows = self.active_flows.lock().await;
        if let Some(state) = active_flows.get(flow_id) {
            // Check if any AI node connects to this tool node
            for connection in &state.flow_definition.connections {
                if connection.is_control() && connection.to_node() == tool_node_id {
                    // Find the source node
                    if let Some(source_node) = state
                        .flow_definition
                        .nodes
                        .iter()
                        .find(|n| n.id == connection.from_node())
                    {
                        // Check if source is an AI node with Orchestrator behavior
                        let is_ai_node = source_node.node_type.starts_with("hybrid-llm")
                            || source_node.node_type == "ai-node"
                            || source_node.node_type.starts_with("ai-")
                            || source_node.node_type == "ai-orchestrator";

                        if is_ai_node {
                            // Only skip if the AI node has Orchestrator behavior
                            if let Some(ai_behavior) = &source_node.ai_behavior {
                                match ai_behavior {
                                    AiNodeBehavior::Orchestrator => {
                                        println!(
                                            "🎯 Tool '{}' connected to Orchestrator '{}' - will skip independent execution",
                                            tool_node_id, source_node.name
                                        );
                                        return Ok(true);
                                    }
                                    _ => {
                                        println!(
                                            "🔄 Tool '{}' connected to {:?} AI '{}' - will NOT skip independent execution",
                                            tool_node_id, ai_behavior, source_node.name
                                        );
                                        return Ok(false);
                                    }
                                }
                            } else {
                                // Default to Orchestrator behavior if not specified for backward compatibility
                                println!(
                                    "🎯 Tool '{}' connected to AI '{}' (no behavior specified, defaulting to Orchestrator) - will skip independent execution",
                                    tool_node_id, source_node.name
                                );
                                return Ok(true);
                            }
                        }
                    }
                }
            }
        }
        Ok(false)
    }

    /// Execute a tool node
    async fn execute_tool_node(
        &self,
        flow_id: &str,
        node_def: &FlowNodeDefinition,
        node_inputs: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, String> {
        // Check if this tool node is connected to an AI node
        if self
            .is_tool_connected_to_ai_node(flow_id, &node_def.id)
            .await?
        {
            println!(
                "⏭️ Skipping tool node '{}' - already executed by connected AI node",
                node_def.name
            );
            // Return a placeholder result since the tool was already executed by the hybrid LLM node
            let mut outputs = HashMap::new();
            outputs.insert(
                "result".to_string(),
                serde_json::json!({
                    "success": true,
                    "message": "Tool executed by AI node",
                    "skipped": true
                }),
            );
            return Ok(outputs);
        }
        if let Some(tool_name) = &node_def.tool_name {
            let args = serde_json::to_value(node_inputs).map_err(|e| e.to_string())?;

            // First try to find the tool in the system registry (for system tools like "Set System Volume")
            let registry_result = {
                let registry = self
                    .flow_registry
                    .registry
                    .lock()
                    .map_err(|e| e.to_string())?;
                registry.get_node_by_name(tool_name)
            };

            if let Some(system_node) = registry_result {
                // Execute as system tool
                println!("🔧 Executing system tool: {}", tool_name);
                let result = system_node.execute(args).await?;

                // Check for error indicators in the result
                Self::check_tool_result_for_error(&result)?;

                // Map result to declared outputs using the node's output schema
                let outputs = Self::map_result_to_outputs(&result, &*system_node);
                Ok(outputs)
            } else {
                // Fall back to MCP tool execution
                println!("🌐 Executing MCP tool: {}", tool_name);
                let result = crate::services::mcp::commands::execute_mcp_tool_core(
                    &self.chat_state,
                    tool_name.clone(),
                    args,
                )
                .await?;

                // Check for error indicators in the result
                Self::check_tool_result_for_error(&result)?;

                // For MCP tools without schema, use default result wrapping
                let mut outputs = HashMap::new();
                outputs.insert("result".to_string(), result);
                Ok(outputs)
            }
        } else {
            Err("Tool node missing tool_name".to_string())
        }
    }

    /// Execute a standard node
    async fn execute_standard_node(
        &self,
        flow_id: &str,
        node_def: &FlowNodeDefinition,
        node_inputs: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, String> {
        if node_def.node_type == "overlay-command" {
            return self
                .execute_overlay_command_node(flow_id, node_def, node_inputs)
                .await;
        }
        if node_def.node_type == "delay" {
            return self
                .execute_durable_delay_node(flow_id, node_def, node_inputs)
                .await;
        }
        if node_def.node_type == "approval" {
            return self
                .execute_durable_approval_node(flow_id, node_def, node_inputs)
                .await;
        }

        // Check if this node is connected to an AI node and should be skipped
        if self
            .is_tool_connected_to_ai_node(flow_id, &node_def.id)
            .await?
        {
            println!(
                "⏭️ Skipping standard node '{}' - already executed by connected AI node",
                node_def.name
            );
            // Return a placeholder result since the tool was already executed by the AI node
            let mut outputs = HashMap::new();
            outputs.insert(
                "result".to_string(),
                serde_json::json!({
                    "success": true,
                    "message": "Tool executed by AI node",
                    "skipped": true
                }),
            );
            return Ok(outputs);
        }

        if node_def.node_type == "invoke-action" {
            return self
                .execute_invoke_action_node(flow_id, node_def, node_inputs)
                .await;
        }

        // Try to find the node in the registry
        let node = {
            let registry = self
                .flow_registry
                .registry
                .lock()
                .map_err(|e| e.to_string())?;
            registry.get_node(&node_def.node_type)
        };

        if let Some(node) = node {
            let args = serde_json::to_value(node_inputs).map_err(|e| e.to_string())?;

            // Extract runtime_variables for context
            let (mut runtime_vars, source_flow_id) = {
                let mut active_flows = self.active_flows.lock().await;
                if let Some(state) = active_flows.get_mut(flow_id) {
                    (
                        std::mem::take(&mut state.runtime_variables),
                        state.flow_id.clone(),
                    )
                } else {
                    (RuntimeVariables::new(), flow_id.to_string())
                }
            };

            // Create progress callback for node progress reporting
            let flow_id_owned = flow_id.to_string();
            let source_flow_id_owned = source_flow_id;
            let node_id_owned = node_def.id.clone();
            let event_emitter = self.event_emitter.clone();

            let progress_callback: crate::services::flow::traits::ProgressCallback =
                Arc::new(move |progress: f64, message: String| {
                    let flow_id = flow_id_owned.clone();
                    let source_flow_id = source_flow_id_owned.clone();
                    let node_id = node_id_owned.clone();
                    let event_emitter = event_emitter.clone();

                    let event = FlowEventPayload::NodeProgress {
                        flow_id: source_flow_id,
                        run_id: flow_id,
                        node_id,
                        progress,
                        message,
                    };

                    match &event_emitter {
                        EventEmitter::Universal(platform_context) => {
                            let payload = crate::events::EventPayload::Flow(event);
                            if let Err(e) = platform_context.event_bus.emit(payload) {
                                log::error!("Failed to emit progress event: {}", e);
                            }
                        }
                    }
                });

            // Create execution context and execute
            // All nodes get context; default implementation delegates to execute()
            let mut ctx = ExecutionContext {
                flow_id,
                node_id: &node_def.id,
                variables: &mut runtime_vars,
                progress_callback: Some(&progress_callback),
            };

            // Emit initial progress for ALL nodes (nodes can override with detailed progress)
            progress_callback(0.0, "Processing...".to_string());

            let result = node.execute_with_context(args, &mut ctx).await;

            // Restore runtime_variables
            {
                let mut active_flows = self.active_flows.lock().await;
                if let Some(state) = active_flows.get_mut(flow_id) {
                    state.runtime_variables = runtime_vars;
                }
            }

            let result_value = result?;

            // Map result to declared outputs FIRST (always extract outputs for debugging)
            let mut outputs = Self::map_result_to_outputs(&result_value, &*node);

            // Check if the result indicates an error (common patterns in MCP/tool responses)
            // If error found, add it to outputs but still return Ok so outputs are preserved
            if let Err(error_msg) = Self::check_tool_result_for_error(&result_value) {
                outputs.insert("__execution_error".to_string(), Value::String(error_msg));
            }

            Ok(outputs)
        } else {
            Err(format!(
                "Flow node type '{}' is not registered",
                node_def.node_type
            ))
        }
    }

    async fn execute_durable_delay_node(
        &self,
        run_id: &str,
        node_def: &FlowNodeDefinition,
        node_inputs: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, String> {
        let duration_ms = node_inputs
            .get("duration")
            .and_then(Value::as_u64)
            .unwrap_or(1_000);
        let wake_id = format!("timer:{}:{}:{}", run_id, node_def.id, Uuid::new_v4());
        let duration = chrono::Duration::try_milliseconds(duration_ms.min(i64::MAX as u64) as i64)
            .ok_or("Delay duration is outside the supported range")?;
        let deadline = self
            .clock
            .now()
            .checked_add_signed(duration)
            .ok_or("Delay deadline is outside the supported date range")?;
        let now = self.clock.now();
        let resume_policy = self.resume_policy_for_run(run_id).await?;
        self.storage
            .save_workflow_wake(&WorkflowWake {
                id: wake_id.clone(),
                run_id: run_id.to_string(),
                node_id: node_def.id.clone(),
                kind: WorkflowWakeKind::Timer,
                status: WorkflowWakeStatus::Waiting,
                due_at: Some(deadline),
                attempt: 0,
                payload: json!({ "durationMs": duration_ms }),
                resume_policy,
                created_at: now,
                updated_at: now,
            })
            .await?;
        self.checkpoint_run(run_id).await;
        wait_until_wall_clock_deadline(self.clock.clone(), deadline).await;
        Ok(HashMap::from([
            ("success".to_string(), json!(true)),
            ("duration".to_string(), json!(duration_ms)),
            ("wakeId".to_string(), json!(wake_id)),
            (
                "completed_at".to_string(),
                json!(self.clock.now().to_rfc3339()),
            ),
        ]))
    }

    fn approval_timeout_ms(node_inputs: &HashMap<String, Value>) -> Option<u64> {
        node_inputs
            .get("timeoutMs")
            .or_else(|| node_inputs.get("timeout_ms"))
            .and_then(Value::as_u64)
            .filter(|timeout| *timeout > 0)
    }

    async fn execute_durable_approval_node(
        &self,
        run_id: &str,
        node_def: &FlowNodeDefinition,
        node_inputs: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, String> {
        let wake_id = format!("approval:{}:{}:{}", run_id, node_def.id, Uuid::new_v4());
        let timeout_ms = Self::approval_timeout_ms(node_inputs);
        let now = self.clock.now();
        let due_at = timeout_ms
            .map(|timeout| {
                chrono::Duration::try_milliseconds(timeout.min(i64::MAX as u64) as i64)
                    .and_then(|duration| now.checked_add_signed(duration))
                    .ok_or("Approval timeout is outside the supported range")
            })
            .transpose()?;
        let requested_channels = node_inputs
            .get("channels")
            .and_then(Value::as_array)
            .map(|channels| {
                channels
                    .iter()
                    .filter_map(Value::as_str)
                    .filter(|channel| matches!(*channel, "desktop" | "mobile"))
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .filter(|channels| !channels.is_empty())
            .unwrap_or_else(|| vec!["desktop".to_string(), "mobile".to_string()]);
        let allowed_channels = node_inputs
            .get("allowedDecisionChannels")
            .or_else(|| node_inputs.get("allowed_decision_channels"))
            .and_then(Value::as_array)
            .map(|channels| {
                channels
                    .iter()
                    .filter_map(Value::as_str)
                    .filter(|channel| {
                        matches!(*channel, "desktop" | "mobile_direct" | "mobile_relay")
                    })
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .filter(|channels| !channels.is_empty())
            .unwrap_or_else(|| ApprovalDecisionPolicy::default().allowed_channels);
        let approved_effects = self
            .collect_approval_effect_bindings(run_id, &node_def.id)
            .await?;
        let request_details = ApprovalRequestDetails {
            title: node_inputs
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or(&node_def.name)
                .to_string(),
            message: node_inputs
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("Workflow approval required")
                .to_string(),
            severity: node_inputs
                .get("severity")
                .and_then(Value::as_str)
                .filter(|severity| matches!(*severity, "normal" | "high" | "critical"))
                .unwrap_or("normal")
                .to_string(),
            requested_channels,
            request_snapshot: node_inputs
                .get("requestSnapshot")
                .cloned()
                .unwrap_or(Value::Null),
            approved_effects,
            decision_policy: ApprovalDecisionPolicy { allowed_channels },
        };
        let resume_policy = self.resume_policy_for_run(run_id).await?;
        let wake = WorkflowWake {
            id: wake_id.clone(),
            run_id: run_id.to_string(),
            node_id: node_def.id.clone(),
            kind: WorkflowWakeKind::Approval,
            status: WorkflowWakeStatus::Waiting,
            due_at,
            attempt: 0,
            payload: serde_json::to_value(request_details)
                .map_err(|error| format!("Failed to serialize approval request: {}", error))?,
            resume_policy,
            created_at: now,
            updated_at: now,
        };
        self.storage.save_workflow_wake(&wake).await?;
        self.checkpoint_run(run_id).await;
        self.emit_event(FlowEventPayload::ApprovalRequested {
            flow_id: run_id.to_string(),
            run_id: run_id.to_string(),
            request: wake.clone(),
        })
        .await;

        let (sender, mut receiver) = tokio::sync::oneshot::channel();
        self.pending_approval_wakes
            .lock()
            .await
            .insert(wake_id.clone(), sender);

        // A decision can arrive after the durable row is committed but before
        // the live waiter is registered. Re-read once to close that race.
        if let Some(decided) = self
            .storage
            .list_open_workflow_wakes(Some(run_id))
            .await?
            .into_iter()
            .find(|candidate| {
                candidate.id == wake_id && candidate.status == WorkflowWakeStatus::Ready
            })
        {
            let resolution = Self::approval_resolution_from_wake(&decided)?;
            if let Some(sender) = self.pending_approval_wakes.lock().await.remove(&wake_id) {
                let _ = sender.send(resolution);
            }
        }

        let resolution = if let Some(deadline) = due_at {
            let remaining = deadline.signed_duration_since(self.clock.now());
            match remaining.to_std() {
                Ok(_) => tokio::select! {
                    received = &mut receiver => received.map_err(|_| "Approval waiter closed before a durable decision".to_string())?,
                    _ = wait_until_wall_clock_deadline(self.clock.clone(), deadline) => {
                        let outcome = self.resolve_live_approval_timeout(&wake_id).await?;
                        Self::approval_resolution_from_wake(outcome.wake())?
                    }
                },
                Err(_) => {
                    let outcome = self.resolve_live_approval_timeout(&wake_id).await?;
                    Self::approval_resolution_from_wake(outcome.wake())?
                }
            }
        } else {
            receiver
                .await
                .map_err(|_| "Approval waiter closed before a durable decision".to_string())?
        };
        self.pending_approval_wakes.lock().await.remove(&wake_id);
        if resolution.decision == ApprovalDecision::Approved {
            let details = Self::approval_request_details(&wake)?;
            self.remember_approved_effect_bindings(run_id, &details)
                .await?;
        }
        Ok(HashMap::from([
            (
                "isApproved".to_string(),
                json!(resolution.decision == ApprovalDecision::Approved),
            ),
            ("decision".to_string(), json!(resolution.decision.route())),
            (
                "selectedRoute".to_string(),
                json!(resolution.decision.route()),
            ),
            ("wakeId".to_string(), json!(wake_id)),
            (
                "decidedAt".to_string(),
                json!(resolution.decided_at.to_rfc3339()),
            ),
            ("decidedBy".to_string(), json!(resolution.decided_by)),
            ("comment".to_string(), json!(resolution.comment)),
        ]))
    }

    /// Claim a timeout while the node future itself is still alive. This does
    /// not enter restart recovery, avoiding a recursive executor lifecycle.
    async fn resolve_live_approval_timeout(
        &self,
        wake_id: &str,
    ) -> Result<ApprovalResolveOutcome, String> {
        let resolution = ApprovalResolution {
            decision: ApprovalDecision::TimedOut,
            decided_at: self.clock.now(),
            decided_by: ApprovalActor {
                id: "workflow-runtime".to_string(),
                display_name: Some("Workflow runtime".to_string()),
                channel: "system".to_string(),
            },
            comment: Some("Approval deadline elapsed".to_string()),
        };
        let outcome = self
            .storage
            .resolve_workflow_approval(wake_id, &resolution)
            .await?;
        if outcome.was_applied() {
            let wake = outcome.wake();
            self.emit_event(FlowEventPayload::ApprovalResolved {
                flow_id: wake.run_id.clone(),
                run_id: wake.run_id.clone(),
                wake_id: wake.id.clone(),
                resolution,
            })
            .await;
        }
        Ok(outcome)
    }

    async fn execute_overlay_command_node(
        &self,
        run_id: &str,
        node_def: &FlowNodeDefinition,
        node_inputs: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, String> {
        use crate::services::execution::{
            ExternalInvocationCompletion, ExternalInvocationStart, InvocationStatus,
        };

        let invocation_service = self
            .action_invocation_service
            .as_ref()
            .ok_or_else(|| "Action invocation service is unavailable".to_string())?;
        let invocation_id = Uuid::new_v4().to_string();
        let started_at = std::time::Instant::now();
        let requires_ack = node_inputs
            .get("requires_ack")
            .or_else(|| node_inputs.get("requiresAck"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let start = ExternalInvocationStart {
            invocation_id: invocation_id.clone(),
            source: "flow".to_string(),
            source_reference: Some(format!("{}:{}", run_id, node_def.id)),
            workflow_run_id: Some(run_id.to_string()),
            node_id: Some(node_def.id.clone()),
            resolved_action: json!({
                "type": "overlayCommand",
                "nodeId": node_def.id,
                "inputs": node_inputs,
            }),
            idempotency_key: Some(format!("overlay-command:{}:{}", run_id, node_def.id)),
            resource_key: Some(
                node_inputs
                    .get("widget_id")
                    .or_else(|| node_inputs.get("widgetId"))
                    .and_then(Value::as_str)
                    .map(|id| format!("overlay:widget:{}", id))
                    .unwrap_or_else(|| "overlay:all".to_string()),
            ),
            receipt_semantics: if requires_ack {
                "overlay_acknowledged".to_string()
            } else {
                "transport_accepted".to_string()
            },
            recovery_decision: "manual_review".to_string(),
        };
        invocation_service.begin_external_invocation(&start).await?;

        let result = self
            .execute_overlay_command_node_inner(run_id, node_def, node_inputs, &invocation_id)
            .await;
        let outputs = result.clone().unwrap_or_default();
        let error = result.as_ref().err().cloned();
        let provider_receipt_id = outputs
            .get("commandId")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        let normalized_status = if result.is_ok() {
            if requires_ack {
                InvocationStatus::Applied
            } else {
                InvocationStatus::TransportAccepted
            }
        } else if error
            .as_deref()
            .is_some_and(|message| message.contains("within") || message.contains("timed out"))
        {
            InvocationStatus::TimedOut
        } else {
            InvocationStatus::Failed
        };
        let completion = ExternalInvocationCompletion {
            dispatch_status: if result.is_ok() {
                "succeeded".to_string()
            } else {
                "failed".to_string()
            },
            request_id: Some(invocation_id.clone()),
            provider_receipt_id,
            outputs,
            normalized_status,
            error,
            recovery_decision: "manual_review".to_string(),
            execution_time_ms: started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
        };
        if let Err(persistence_error) = invocation_service
            .complete_external_invocation(&invocation_id, &completion)
            .await
        {
            return Err(format!(
                "Overlay dispatch finished but its durable outcome could not be stored: {}",
                persistence_error
            ));
        }
        result
    }

    async fn execute_overlay_command_node_inner(
        &self,
        run_id: &str,
        node_def: &FlowNodeDefinition,
        node_inputs: &HashMap<String, Value>,
        invocation_id: &str,
    ) -> Result<HashMap<String, Value>, String> {
        use crate::events::{
            EventPayload, OverlayAcknowledgementMode, OverlayCommandAckStatus,
            OverlayCommandAcknowledgement, OverlayCommandPayload, OverlayTarget,
        };

        let requires_ack = node_inputs
            .get("requires_ack")
            .or_else(|| node_inputs.get("requiresAck"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let acknowledgement_mode = match node_inputs
            .get("ack_mode")
            .or_else(|| node_inputs.get("ackMode"))
            .and_then(Value::as_str)
            .unwrap_or("hostAccepted")
        {
            "hostAccepted" | "host" => OverlayAcknowledgementMode::HostAccepted,
            "widgetRendered" | "visual" => OverlayAcknowledgementMode::WidgetRendered,
            unsupported => return Err(format!("Unsupported overlay ACK mode '{}'", unsupported)),
        };
        let target = match node_inputs
            .get("target")
            .and_then(Value::as_str)
            .unwrap_or("all")
        {
            "all" => OverlayTarget::All,
            "widget" => {
                let widget_id = node_inputs
                    .get("widget_id")
                    .or_else(|| node_inputs.get("widgetId"))
                    .and_then(Value::as_str)
                    .filter(|value| !value.trim().is_empty())
                    .ok_or_else(|| {
                        "overlay-command target 'widget' requires widgetId".to_string()
                    })?;
                OverlayTarget::Widget {
                    widget_id: widget_id.to_string(),
                }
            }
            unsupported => {
                return Err(format!(
                    "Unsupported overlay target '{}'; expected 'all' or 'widget'",
                    unsupported
                ));
            }
        };

        let event_id = {
            let active_flows = self.active_flows.lock().await;
            active_flows.get(run_id).and_then(|state| {
                state.flow_definition.nodes.iter().find_map(|node| {
                    node.inputs
                        .get("trigger_event_id")
                        .and_then(Value::as_str)
                        .map(str::to_string)
                })
            })
        };
        let ack_timeout_ms = node_inputs
            .get("ack_timeout_ms")
            .or_else(|| node_inputs.get("ackTimeoutMs"))
            .and_then(Value::as_u64)
            .unwrap_or(10_000)
            .clamp(100, 300_000);
        let expires_at = node_inputs
            .get("expires_in_ms")
            .or_else(|| node_inputs.get("expiresInMs"))
            .and_then(Value::as_u64)
            .map(|milliseconds| {
                self.clock.now() + chrono::Duration::milliseconds(milliseconds as i64)
            })
            // An ACK-required command must never remain replayable forever.
            .or_else(|| {
                requires_ack.then(|| {
                    self.clock.now() + chrono::Duration::milliseconds(ack_timeout_ms as i64)
                })
            });
        let command_id = Uuid::new_v4().to_string();
        let command = OverlayCommandPayload {
            command_id: command_id.clone(),
            schema_version: 1,
            delivery_revision: 1,
            target,
            command: node_inputs
                .get("command")
                .and_then(Value::as_str)
                .unwrap_or("show")
                .to_string(),
            payload: node_inputs.get("payload").cloned().unwrap_or(Value::Null),
            workflow_run_id: Some(run_id.to_string()),
            event_id,
            expires_at,
            requires_ack,
            acknowledgement_mode,
        };

        if requires_ack {
            let now = self.clock.now();
            self.storage
                .save_overlay_command_delivery(&crate::events::OverlayCommandDelivery {
                    command: command.clone(),
                    invocation_id: Some(invocation_id.to_string()),
                    run_id: run_id.to_string(),
                    node_id: node_def.id.clone(),
                    status: crate::events::OverlayCommandDeliveryStatus::Pending,
                    created_at: now,
                    updated_at: now,
                    acknowledgement: None,
                })
                .await?;
        }

        // Register before publishing so a fast local browser-source ACK cannot
        // race ahead of the workflow waiter.
        let ack_receiver = if requires_ack {
            let (sender, receiver) = tokio::sync::oneshot::channel();
            self.pending_overlay_acks
                .lock()
                .await
                .insert(command_id.clone(), sender);
            Some(receiver)
        } else {
            None
        };
        let expected_delivery_revision = command.delivery_revision;

        let emit_result = match &self.event_emitter {
            EventEmitter::Universal(platform_context) => platform_context
                .event_bus
                .emit(EventPayload::Overlay(command))
                .map_err(|error| format!("Failed to emit overlay command: {}", error)),
        };
        let receiver_count = match emit_result {
            Ok(count) => count,
            Err(error) => {
                self.pending_overlay_acks.lock().await.remove(&command_id);
                if requires_ack {
                    let rejected = crate::events::OverlayCommandAcknowledgement {
                        command_id: command_id.clone(),
                        status: OverlayCommandAckStatus::Rejected,
                        receipt_phase: crate::events::OverlayCommandReceiptPhase::HostAccepted,
                        instance_id: Some("runtime:event-bus".to_string()),
                        applied_revision: None,
                        error: Some(error.clone()),
                    };
                    let _ = self
                        .storage
                        .resolve_overlay_command_delivery(&rejected)
                        .await;
                }
                return Err(error);
            }
        };

        let acknowledgement = if let Some(receiver) = ack_receiver {
            let receipt_result =
                tokio::time::timeout(std::time::Duration::from_millis(ack_timeout_ms), receiver)
                    .await;
            self.pending_overlay_acks.lock().await.remove(&command_id);
            let receipt = match receipt_result {
                Ok(Ok(receipt)) => receipt,
                Ok(Err(_)) => {
                    let error = format!(
                        "Overlay acknowledgement channel closed for command {}",
                        command_id
                    );
                    let _ = self
                        .storage
                        .expire_overlay_command_delivery(&command_id)
                        .await;
                    return Err(error);
                }
                Err(_) => {
                    let error = format!(
                        "Overlay command {} was not acknowledged within {}ms",
                        command_id, ack_timeout_ms
                    );
                    let _ = self
                        .storage
                        .expire_overlay_command_delivery(&command_id)
                        .await;
                    return Err(error);
                }
            };
            if receipt.instance_id.as_deref().is_none_or(str::is_empty) {
                return Err(format!(
                    "Overlay command {} ACK did not identify its browser-source instance",
                    command_id
                ));
            }
            if receipt.status == OverlayCommandAckStatus::Accepted
                && receipt.applied_revision != Some(expected_delivery_revision)
            {
                return Err(format!(
                    "Overlay command {} ACK revision mismatch (expected {}, got {:?})",
                    command_id, expected_delivery_revision, receipt.applied_revision
                ));
            }
            if receipt.status == OverlayCommandAckStatus::Rejected {
                return Err(receipt
                    .error
                    .unwrap_or_else(|| format!("Overlay rejected command {}", command_id)));
            }
            Some(receipt)
        } else {
            None
        };

        let mut outputs = HashMap::from([
            ("commandId".to_string(), json!(command_id)),
            ("receiverCount".to_string(), json!(receiver_count)),
            ("nodeId".to_string(), json!(node_def.id)),
        ]);
        if let Some(receipt) = acknowledgement {
            outputs.insert("acknowledged".to_string(), json!(true));
            outputs.insert("ackStatus".to_string(), json!(receipt.status));
        }
        Ok(outputs)
    }

    /// Resolves one receipt waiter. Only the desktop overlay WebSocket bridge
    /// calls this after authenticating the browser-source capability token.
    pub async fn acknowledge_overlay_command(
        &self,
        acknowledgement: crate::events::OverlayCommandAcknowledgement,
    ) -> bool {
        let _admission = self.runtime_admission_gate.read().await;
        if self.ensure_runtime_admission().is_err() {
            return false;
        }
        let delivery = match self
            .storage
            .load_overlay_command_delivery(&acknowledgement.command_id)
            .await
        {
            Ok(Some(delivery)) => delivery,
            Ok(None) => return false,
            Err(error) => {
                log::error!("Failed to validate overlay acknowledgement: {}", error);
                return false;
            }
        };
        if acknowledgement
            .instance_id
            .as_deref()
            .is_none_or(str::is_empty)
        {
            log::warn!("Rejected overlay acknowledgement without an instance ID");
            return false;
        }
        if acknowledgement.status == crate::events::OverlayCommandAckStatus::Accepted
            && acknowledgement.applied_revision != Some(delivery.command.delivery_revision)
        {
            log::warn!("Rejected overlay acknowledgement with a mismatched delivery revision");
            return false;
        }
        if acknowledgement.status == crate::events::OverlayCommandAckStatus::Accepted
            && delivery.command.acknowledgement_mode
                == crate::events::OverlayAcknowledgementMode::WidgetRendered
            && acknowledgement.receipt_phase
                != crate::events::OverlayCommandReceiptPhase::WidgetRendered
        {
            log::warn!("Rejected host-only acknowledgement for a widget-rendered command");
            return false;
        }
        match self
            .storage
            .resolve_overlay_command_delivery(&acknowledgement)
            .await
        {
            Ok(true) => {}
            Ok(false) => return false,
            Err(error) => {
                log::error!("Failed to persist overlay acknowledgement: {}", error);
                return false;
            }
        }
        let sender = self
            .pending_overlay_acks
            .lock()
            .await
            .remove(&acknowledgement.command_id);
        sender.map_or(true, |sender| sender.send(acknowledgement).is_ok())
    }

    /// Commands still waiting for an ACK, used by the authenticated overlay
    /// bridge to reconcile a browser-source reconnect.
    pub async fn pending_overlay_commands(
        &self,
    ) -> Result<Vec<crate::events::OverlayCommandPayload>, String> {
        Ok(self
            .storage
            .list_pending_overlay_command_deliveries()
            .await?
            .into_iter()
            .map(|delivery| delivery.command)
            .collect())
    }

    pub async fn inspect_workflow_runtime(
        &self,
    ) -> Result<crate::events::WorkflowRuntimeInspection, String> {
        let recovery_runs = self.list_recovery_required_runs().await.len();
        Ok(crate::events::WorkflowRuntimeInspection {
            recovery_runs,
            pending_overlay_commands: self
                .storage
                .list_pending_overlay_command_deliveries()
                .await?
                .into_iter()
                .map(|delivery| crate::events::WorkflowOverlayDeliverySummary {
                    command: crate::events::WorkflowOverlayCommandSummary {
                        command_id: delivery.command.command_id,
                        command: delivery.command.command,
                        delivery_revision: delivery.command.delivery_revision,
                    },
                    run_id: delivery.run_id,
                    node_id: delivery.node_id,
                    status: delivery.status,
                    updated_at: delivery.updated_at,
                })
                .collect(),
            side_effects: self
                .storage
                .list_side_effect_outbox()
                .await?
                .into_iter()
                .map(|entry| crate::events::WorkflowSideEffectTrace {
                    idempotency_key: entry.idempotency_key,
                    invocation_id: entry.invocation_id,
                    run_id: entry.run_id,
                    node_id: entry.node_id,
                    status: entry.status,
                    has_error: entry.error.is_some(),
                    updated_at: entry.updated_at,
                })
                .collect(),
            metrics: self.storage.workflow_runtime_metrics().await?,
            dead_letters: self
                .storage
                .list_workflow_dead_letters()
                .await?
                .into_iter()
                .map(|record| crate::events::WorkflowDeadLetterTrace {
                    id: record.id,
                    flow_id: record.flow_id,
                    node_id: record.node_id,
                    attempts: record.attempts,
                    resolved_at: record.resolved_at,
                    has_reason: !record.reason.is_empty(),
                    updated_at: record.updated_at,
                })
                .collect(),
        })
    }

    /// Correlate one canonical event with its routed runs and durable effects.
    /// The returned shape is deliberately redacted for diagnostics/export.
    pub async fn trace_workflow_event(
        &self,
        event_id: &str,
    ) -> Result<Option<crate::events::WorkflowRuntimeTrace>, String> {
        let Some(mut trace) = self.storage.trace_workflow_event(event_id).await? else {
            return Ok(None);
        };
        if let Some(service) = &self.action_invocation_service {
            for run in &trace.runs {
                let records = service
                    .list_invocations(500, None, None, Some(&run.run_id), None)
                    .await?;
                trace.invocations.extend(records.into_iter().map(|record| {
                    crate::events::WorkflowInvocationTrace {
                        invocation_id: record.invocation_id,
                        run_id: record.workflow_run_id,
                        node_id: record.node_id,
                        integration_id: record.integration_id,
                        action_id: record.action_id,
                        dispatch_status: record.dispatch_status,
                        normalized_status: record.normalized_status,
                        has_error: record.error.is_some(),
                        started_at: record.started_at,
                        completed_at: record.completed_at,
                    }
                }));
            }
        }
        trace
            .invocations
            .sort_by(|left, right| left.started_at.cmp(&right.started_at));
        trace
            .invocations
            .dedup_by(|left, right| left.invocation_id == right.invocation_id);
        Ok(Some(trace))
    }

    pub async fn record_workflow_dead_letter(
        &self,
        envelope: &crate::events::EventEnvelope,
        flow_id: &str,
        node_id: &str,
        reason: String,
    ) -> Result<String, String> {
        let now = self.clock.now();
        let id = uuid::Uuid::new_v4().to_string();
        self.storage
            .save_workflow_dead_letter(&crate::events::WorkflowDeadLetter {
                id: id.clone(),
                event_id: envelope.event_id.clone(),
                flow_id: flow_id.to_string(),
                node_id: node_id.to_string(),
                reason,
                attempts: 0,
                created_at: now,
                updated_at: now,
                resolved_at: None,
            })
            .await?;
        Ok(id)
    }

    /// Explicit operator replay of a failed ingress route. A new dedup key is
    /// used for each attempt while the original canonical envelope remains the
    /// audit source. Callers must separately require side-effect confirmation.
    pub async fn replay_workflow_dead_letter(&self, id: &str) -> Result<String, String> {
        let mut record = self
            .storage
            .load_workflow_dead_letter(id)
            .await?
            .ok_or_else(|| format!("Dead letter {} not found", id))?;
        if record.resolved_at.is_some() {
            return Err(format!("Dead letter {} is already resolved", id));
        }
        let envelope = self
            .storage
            .load_event_envelope(&record.event_id)
            .await?
            .ok_or_else(|| format!("Canonical event {} is missing", record.event_id))?;
        record.attempts = record.attempts.saturating_add(1);
        record.updated_at = Utc::now();
        let event = TriggerEvent {
            source: TriggerSource::Backend,
            event_id: Some(envelope.event_id.clone()),
            correlation_id: envelope.correlation_id.clone(),
            flow_id: record.flow_id.clone(),
            node_id: record.node_id.clone(),
            data: json!({ "event": envelope }),
            timestamp: Utc::now(),
            dedup_key: Some(format!(
                "dead-letter-replay:{}:{}",
                record.id, record.attempts
            )),
        };
        match self.trigger_flow_from_event(event).await {
            Ok(run_id) => {
                record.resolved_at = Some(Utc::now());
                record.reason = format!("Replayed as run {}", run_id);
                record.updated_at = Utc::now();
                self.storage.save_workflow_dead_letter(&record).await?;
                Ok(run_id)
            }
            Err(error) => {
                record.reason = error.clone();
                self.storage.save_workflow_dead_letter(&record).await?;
                Err(error)
            }
        }
    }

    fn recovery_policy_from_semantics(
        recovery: crate::integrations::ActionRecoverySemantics,
    ) -> RecoveryPolicy {
        match recovery {
            crate::integrations::ActionRecoverySemantics::Reconcile => RecoveryPolicy::Reconcile,
            crate::integrations::ActionRecoverySemantics::RetryWithIdempotency => {
                RecoveryPolicy::RetryWithIdempotency
            }
            crate::integrations::ActionRecoverySemantics::ManualReview => {
                RecoveryPolicy::ManualReview
            }
            crate::integrations::ActionRecoverySemantics::FailClosed => RecoveryPolicy::FailClosed,
        }
    }

    async fn action_contract(
        &self,
        action: &UniversalAction,
    ) -> Result<Option<crate::integrations::ActionContract>, String> {
        let service = self.action_invocation_service.as_ref().ok_or_else(|| {
            "Action invocation service is not wired into FlowExecutor".to_string()
        })?;
        service
            .resolve_capability(action)
            .await
            .map(|capability| capability.map(|capability| capability.contract))
            .map_err(|error| error.message)
    }

    async fn action_recovery_policy(
        &self,
        action: &UniversalAction,
    ) -> Result<RecoveryPolicy, String> {
        if matches!(action, UniversalAction::HttpRequest { .. }) {
            return Ok(RecoveryPolicy::RetryWithIdempotency);
        }
        Ok(self
            .action_contract(action)
            .await?
            .map(|contract| Self::recovery_policy_from_semantics(contract.recovery))
            .unwrap_or(RecoveryPolicy::FailClosed))
    }

    fn action_reference_for_node(
        node_def: &FlowNodeDefinition,
        node_inputs: &HashMap<String, Value>,
    ) -> Result<crate::services::execution::ActionReference, String> {
        let value = node_inputs
            .get("actionReference")
            .or_else(|| node_def.ui_data.custom_properties.get("actionReference"))
            .ok_or_else(|| {
                format!(
                    "Invoke Action node '{}' is missing actionReference",
                    node_def.name
                )
            })?;

        serde_json::from_value(value.clone()).map_err(|error| {
            format!(
                "Invoke Action node '{}' has an invalid actionReference: {}",
                node_def.name, error
            )
        })
    }

    fn resolved_action_hash(action: &UniversalAction) -> Result<String, String> {
        let document = serde_json::to_value(action)
            .map_err(|error| format!("Failed to serialize resolved action: {error}"))?;
        let canonical = serde_json_canonicalizer::to_string(&document)
            .map_err(|error| format!("Failed to canonicalize resolved action: {error}"))?;
        Ok(hex::encode(Sha256::digest(canonical.as_bytes())))
    }

    fn validate_resolved_action_binding(
        node_id: &str,
        action: &UniversalAction,
        expected_hash: &str,
    ) -> Result<(), String> {
        if Self::resolved_action_hash(action)? != expected_hash {
            return Err(format!(
                "Effect {node_id} changed after approval was requested; create a new approval request"
            ));
        }
        Ok(())
    }

    async fn collect_approval_effect_bindings(
        &self,
        run_id: &str,
        approval_node_id: &str,
    ) -> Result<Vec<ApprovedEffectBinding>, String> {
        let flow = {
            let active = self.active_flows.lock().await;
            active
                .get(run_id)
                .ok_or_else(|| format!("Run {run_id} is not loaded"))?
                .flow_definition
                .clone()
        };
        let mut queue = flow
            .connections
            .iter()
            .filter_map(|connection| match connection {
                FlowConnection::Control {
                    from_node,
                    to_node,
                    route,
                } if from_node == approval_node_id
                    && route.as_deref().is_none_or(|route| route == "approved") =>
                {
                    Some(to_node.clone())
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        let mut visited = HashSet::new();
        let mut bindings = Vec::new();
        while let Some(node_id) = queue.pop() {
            if !visited.insert(node_id.clone()) {
                continue;
            }
            let node = flow
                .nodes
                .iter()
                .find(|candidate| candidate.id == node_id)
                .ok_or_else(|| format!("Approval branch references missing node {node_id}"))?;
            // A later approval owns all effects beyond its own approved route.
            if node.node_type == "approval" {
                continue;
            }
            if node.node_type == "invoke-action" {
                let inputs = self.build_node_inputs(run_id, node).await?;
                let reference = Self::action_reference_for_node(node, &inputs)?;
                let action = self
                    .action_invocation_service
                    .as_ref()
                    .ok_or_else(|| {
                        "Action invocation service is required to bind approval effects".to_string()
                    })?
                    .prepare_reference(
                        &reference,
                        None,
                        None,
                        &inputs.get("inputs").cloned().unwrap_or_else(|| json!({})),
                    )
                    .await?;
                bindings.push(ApprovedEffectBinding {
                    node_id: node.id.clone(),
                    action_hash: Self::resolved_action_hash(&action)?,
                });
            }
            queue.extend(
                flow.connections
                    .iter()
                    .filter_map(|connection| match connection {
                        FlowConnection::Control {
                            from_node, to_node, ..
                        } if from_node == &node_id => Some(to_node.clone()),
                        _ => None,
                    }),
            );
        }
        bindings.sort_by(|left, right| left.node_id.cmp(&right.node_id));
        Ok(bindings)
    }

    async fn validate_approval_effect_bindings(
        &self,
        run_id: &str,
        details: &ApprovalRequestDetails,
    ) -> Result<(), String> {
        for expected in &details.approved_effects {
            let node = {
                let active = self.active_flows.lock().await;
                active
                    .get(run_id)
                    .and_then(|state| {
                        state
                            .flow_definition
                            .nodes
                            .iter()
                            .find(|node| node.id == expected.node_id)
                    })
                    .cloned()
                    .ok_or_else(|| {
                        format!(
                            "Approved effect node {} is absent from run {}",
                            expected.node_id, run_id
                        )
                    })?
            };
            let inputs = self.build_node_inputs(run_id, &node).await?;
            let reference = Self::action_reference_for_node(&node, &inputs)?;
            let action = self
                .action_invocation_service
                .as_ref()
                .ok_or_else(|| {
                    "Action invocation service is required to validate approval effects".to_string()
                })?
                .prepare_reference(
                    &reference,
                    None,
                    None,
                    &inputs.get("inputs").cloned().unwrap_or_else(|| json!({})),
                )
                .await?;
            Self::validate_resolved_action_binding(
                &expected.node_id,
                &action,
                &expected.action_hash,
            )?;
        }
        Ok(())
    }

    async fn remember_approved_effect_bindings(
        &self,
        run_id: &str,
        details: &ApprovalRequestDetails,
    ) -> Result<(), String> {
        let mut active = self.active_flows.lock().await;
        let state = active
            .get_mut(run_id)
            .ok_or_else(|| format!("Approval run {run_id} is no longer loaded"))?;
        for binding in &details.approved_effects {
            if let Some(previous) = state
                .approved_effect_bindings
                .insert(binding.node_id.clone(), binding.action_hash.clone())
            {
                if previous != binding.action_hash {
                    return Err(format!(
                        "Effect {} has conflicting approval bindings",
                        binding.node_id
                    ));
                }
            }
        }
        Ok(())
    }

    async fn authorize_automatic_action_retry(
        &self,
        flow_id: &str,
        node_def: &FlowNodeDefinition,
        node_inputs: &HashMap<String, Value>,
    ) -> Result<(), String> {
        let action_reference = Self::action_reference_for_node(node_def, node_inputs)?;
        let invocation_service = self.action_invocation_service.as_ref().ok_or_else(|| {
            "Action invocation service is not wired into FlowExecutor yet".to_string()
        })?;
        let action = invocation_service
            .prepare_reference(
                &action_reference,
                None,
                None,
                &node_inputs
                    .get("inputs")
                    .cloned()
                    .unwrap_or_else(|| json!({})),
            )
            .await?;
        let policy = self.action_recovery_policy(&action).await?;
        if policy != RecoveryPolicy::RetryWithIdempotency {
            return Err(format!(
                "provider recovery policy is {:?}, not RetryWithIdempotency",
                policy
            ));
        }

        self.approved_side_effect_retries
            .lock()
            .await
            .insert(format!("flow-action:{}:{}", flow_id, node_def.id));
        Ok(())
    }

    async fn action_resource_key(
        &self,
        action: &UniversalAction,
    ) -> Result<Option<String>, String> {
        Ok(self
            .action_contract(action)
            .await?
            .and_then(|contract| contract.resource_key))
    }

    /// Close the crash window where the canonical invocation reached a durable
    /// terminal receipt but the workflow outbox was not updated yet.
    async fn reconcile_side_effect_invocation(
        &self,
        entry: &crate::events::SideEffectOutboxEntry,
    ) -> Result<crate::events::SideEffectOutboxEntry, String> {
        let (Some(invocation_service), Some(invocation_id)) = (
            self.action_invocation_service.as_ref(),
            entry.invocation_id.as_deref(),
        ) else {
            return Ok(entry.clone());
        };
        let Some(invocation) = invocation_service.get_invocation(invocation_id).await? else {
            return Ok(entry.clone());
        };
        let Some(status) = invocation.normalized_status.as_deref() else {
            return Ok(entry.clone());
        };

        let Some(outbox_status) =
            crate::events::SideEffectOutboxStatus::from_normalized_invocation(status)
        else {
            return Ok(entry.clone());
        };
        let error = match outbox_status {
            crate::events::SideEffectOutboxStatus::Failed => invocation.error.clone(),
            crate::events::SideEffectOutboxStatus::Uncertain => Some(format!(
                "Invocation {} ended as {}; no applied provider receipt is durable",
                invocation_id, status
            )),
            _ => None,
        };
        if entry.status != outbox_status || entry.error != error {
            self.storage
                .complete_side_effect_outbox(
                    &entry.idempotency_key,
                    outbox_status.clone(),
                    error.clone(),
                )
                .await?;
        }
        let mut reconciled = entry.clone();
        reconciled.status = outbox_status;
        reconciled.error = error;
        reconciled.updated_at = Utc::now();
        Ok(reconciled)
    }

    async fn acquire_action_resource_lease(
        &self,
        run_id: &str,
        node_id: &str,
        action: &UniversalAction,
        node_def: &FlowNodeDefinition,
    ) -> Result<Option<WorkflowResourceLease>, String> {
        let Some(resource_key) = self.action_resource_key(action).await? else {
            return Ok(None);
        };
        let now = self.clock.now();
        let ttl_ms = node_def
            .execution_config
            .timeout_ms
            .unwrap_or(60_000)
            .saturating_add(30_000);
        let expires_at = now
            .checked_add_signed(
                chrono::Duration::try_milliseconds(ttl_ms.min(i64::MAX as u64) as i64)
                    .ok_or("Resource lease duration is outside the supported range")?,
            )
            .ok_or("Resource lease expiry is outside the supported date range")?;
        let lease = WorkflowResourceLease {
            resource_key: resource_key.clone(),
            owner_run_id: run_id.to_string(),
            owner_node_id: node_id.to_string(),
            lease_token: Uuid::new_v4().to_string(),
            expires_at,
            updated_at: now,
        };
        if !self.storage.acquire_resource_lease(&lease, now).await? {
            return Err(format!(
                "Resource '{}' is busy; the scheduler will retry according to the node policy",
                resource_key
            ));
        }
        Ok(Some(lease))
    }

    async fn release_action_resource_lease(&self, lease: Option<&WorkflowResourceLease>) {
        let Some(lease) = lease else {
            return;
        };
        if let Err(error) = self
            .storage
            .release_resource_lease(&lease.resource_key, &lease.lease_token)
            .await
        {
            log::warn!("Failed to release workflow resource lease: {}", error);
        }
    }

    async fn execute_invoke_action_node(
        &self,
        flow_id: &str,
        node_def: &FlowNodeDefinition,
        node_inputs: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, String> {
        let action_reference = Self::action_reference_for_node(node_def, node_inputs)?;

        println!(
            "🎯 invoke-action '{}' -> {:?}",
            node_def.name, action_reference
        );

        let invocation_service = self
            .action_invocation_service
            .as_ref()
            .ok_or_else(|| {
                "Action invocation service is not wired into FlowExecutor yet".to_string()
            })?
            .clone();

        let universal_action = invocation_service
            .prepare_reference(
                &action_reference,
                None,
                None,
                &node_inputs
                    .get("inputs")
                    .cloned()
                    .unwrap_or_else(|| json!({})),
            )
            .await?;

        let approved_hash = self
            .active_flows
            .lock()
            .await
            .get(flow_id)
            .and_then(|state| state.approved_effect_bindings.get(&node_def.id).cloned());
        if let Some(expected_hash) = approved_hash {
            Self::validate_resolved_action_binding(
                &node_def.id,
                &universal_action,
                &expected_hash,
            )
            .map_err(|_| {
                format!(
                    "Action '{}' changed after approval; execution is blocked until it is approved again",
                    node_def.name
                )
            })?;
        }

        let resource_lease = self
            .acquire_action_resource_lease(flow_id, &node_def.id, &universal_action, node_def)
            .await?;

        let outbox_key = format!("flow-action:{}:{}", flow_id, node_def.id);
        let invocation_id = Uuid::new_v4().to_string();
        let now = self.clock.now();
        let outbox_entry = crate::events::SideEffectOutboxEntry {
            idempotency_key: outbox_key.clone(),
            invocation_id: Some(invocation_id.clone()),
            run_id: flow_id.to_string(),
            node_id: node_def.id.clone(),
            action: serde_json::to_value(&universal_action)
                .map_err(|error| format!("Failed to serialize action outbox entry: {}", error))?,
            status: crate::events::SideEffectOutboxStatus::Dispatching,
            created_at: now,
            updated_at: now,
            error: None,
        };
        let claimed = match self.storage.claim_side_effect_outbox(&outbox_entry).await {
            Ok(claimed) => claimed,
            Err(error) => {
                self.release_action_resource_lease(resource_lease.as_ref())
                    .await;
                return Err(error);
            }
        };
        let explicitly_approved = if claimed {
            false
        } else {
            self.approved_side_effect_retries
                .lock()
                .await
                .remove(&outbox_key)
        };
        if !claimed && !explicitly_approved {
            self.release_action_resource_lease(resource_lease.as_ref())
                .await;
            return Err(format!(
                "Side-effect action '{}' has an existing outbox record; recovery review is required before retrying it",
                node_def.name
            ));
        }
        if explicitly_approved {
            // The workflow outbox tracks the current approved attempt; the
            // canonical invocation repository retains every prior attempt.
            self.storage
                .replace_side_effect_outbox_attempt(&outbox_entry)
                .await?;
        }

        let context = UniversalExecutionContext {
            trigger_source: UniversalTriggerSource::Flow {
                flow_id: flow_id.to_string(),
                node_id: node_def.id.clone(),
            },
            user_id: None,
            session_id: format!("flow-node-{}-{}", flow_id, node_def.id),
            variables: HashMap::new(),
            timeout_ms: node_def.execution_config.timeout_ms,
            retry_config: UniversalRetryConfig {
                // FlowExecutor owns node-level retry accounting/checkpoints.
                max_retries: 0,
                retry_delay_ms: node_def.execution_config.retry_delay_ms,
                exponential_backoff: false,
            },
            execution_mode: ButtonExecutionMode::RunOnce,
            execution_config: Some(json!({
                "idempotencyKey": outbox_key,
                "invocationId": invocation_id
            })),
        };

        let execution_result = invocation_service
            .invoke_resolved_action(&universal_action, &context)
            .await;
        self.release_action_resource_lease(resource_lease.as_ref())
            .await;

        if matches!(
            execution_result.status,
            crate::services::execution::InvocationStatus::TransportAccepted
                | crate::services::execution::InvocationStatus::Uncertain
        ) {
            let err = format!(
                "Invoke Action node '{}' has no durable applied receipt (status: {:?}); external outcome requires recovery review",
                node_def.name, execution_result.status
            );
            self.storage
                .complete_side_effect_outbox(
                    &outbox_key,
                    crate::events::SideEffectOutboxStatus::Uncertain,
                    Some(err.clone()),
                )
                .await?;
            return Err(err);
        }

        if !execution_result.succeeded() {
            let err = execution_result
                .error
                .as_ref()
                .map(|e| e.message.clone())
                .unwrap_or_else(|| "Action invocation failed".to_string());
            self.storage
                .complete_side_effect_outbox(
                    &outbox_key,
                    crate::events::SideEffectOutboxStatus::Failed,
                    Some(err.clone()),
                )
                .await?;
            return Err(format!(
                "Invoke Action node '{}' failed: {}",
                node_def.name, err
            ));
        }
        self.storage
            .complete_side_effect_outbox(
                &outbox_key,
                crate::events::SideEffectOutboxStatus::Succeeded,
                None,
            )
            .await?;
        println!("✅ invoke-action '{}' executed successfully", node_def.name);

        let mut outputs = HashMap::new();
        outputs.insert("status".to_string(), Value::Bool(true));
        let outcome = serde_json::to_value(&execution_result.status)
            .unwrap_or_else(|_| Value::String("unknown".to_string()));
        outputs.insert("outcome".to_string(), outcome);
        outputs.insert(
            "partial".to_string(),
            Value::Bool(matches!(
                execution_result.status,
                crate::services::execution::InvocationStatus::Partial
            )),
        );
        outputs.insert(
            "targetResults".to_string(),
            execution_result
                .outputs
                .get("targetResults")
                .cloned()
                .unwrap_or_else(|| json!([])),
        );
        outputs.insert(
            "result".to_string(),
            json!({
                "success": true,
                "actionReference": action_reference,
                "outputs": execution_result.outputs,
            }),
        );

        Ok(outputs)
    }

    /// Execute a tool action directly within the flow context
    async fn execute_tool_action(
        &self,
        flow_id: &str,
        action: &ToolAction,
    ) -> Result<Value, String> {
        println!(
            "🔧 Executing flow node: {} (ID: {})",
            action.tool_name, action.target_node_id
        );

        // Get the target node definition by cloning it to avoid borrow issues
        let target_node = {
            let active_flows = self.active_flows.lock().await;
            if let Some(state) = active_flows.get(flow_id) {
                // Find and clone the target node in the flow definition
                state
                    .flow_definition
                    .nodes
                    .iter()
                    .find(|n| n.id == action.target_node_id)
                    .cloned()
            } else {
                return Err(format!("Flow not found: {}", flow_id));
            }
        };

        if let Some(target_node) = target_node {
            // Execute the flow node directly
            match self.execute_single_node(flow_id, &target_node).await {
                Ok(node_result) => {
                    // Return the primary result
                    if let Some(result) = node_result.outputs.get("result") {
                        Ok(result.clone())
                    } else {
                        Ok(serde_json::json!({
                            "success": true,
                            "outputs": node_result.outputs
                        }))
                    }
                }
                Err(e) => Err(format!(
                    "Failed to execute flow node {}: {}",
                    action.target_node_id, e
                )),
            }
        } else {
            Err(format!("Flow node not found: {}", action.target_node_id))
        }
    }

    /// Get connected tools for a node
    async fn get_connected_tools(
        &self,
        flow_id: &str,
        node_def: &FlowNodeDefinition,
    ) -> Result<Vec<crate::services::mcp::ai_node::ToolSpec>, String> {
        let active_flows = self.active_flows.lock().await;
        if let Some(state) = active_flows.get(flow_id) {
            let mut connected_tools = Vec::new();

            // Check if this is a Free Agent behavior
            let is_free_agent = node_def
                .ai_behavior
                .as_ref()
                .map(|b| matches!(b, AiNodeBehavior::FreeAgent))
                .unwrap_or(false);

            if is_free_agent {
                println!("🌐 Free Agent detected - providing access to ALL system tools");

                // For Free Agents, provide ALL system tools from registry as unified MCP tools
                let registry = self
                    .flow_registry
                    .registry
                    .lock()
                    .map_err(|e| e.to_string())?;
                let all_system_tools = registry.get_all_node_info();

                for tool_info in &all_system_tools {
                    if let Some(system_node) = registry.get_node_by_name(&tool_info.name) {
                        // ✅ Present ALL tools as unified MCP tools
                        connected_tools.push(crate::services::mcp::ai_node::ToolSpec {
                            name: system_node.id().clone(), // Use actual tool ID
                            description: format!(
                                "{} - {}",
                                tool_info.name,
                                system_node.description()
                            ),
                            parameters: system_node.parameters_schema(),
                            node_id: system_node.id().clone(), // Direct execution via tool ID
                            tool_type: crate::services::mcp::ai_node::ToolType::SystemTool,
                        });
                        println!(
                            "✅ Added unified tool: {} ({})",
                            system_node.id(),
                            tool_info.name
                        );
                    }
                }

                println!(
                    "📋 Free Agent has access to {} unified tools",
                    connected_tools.len()
                );
                return Ok(connected_tools);
            }

            // For Orchestrators: Find connected tools and present them as unified MCP tools
            println!("🔍 Looking for tools connected to AI node: {}", node_def.id);

            // Find all nodes connected TO this AI node (tools the AI can execute)
            for connection in &state.flow_definition.connections {
                if connection.is_control() && connection.from_node() == node_def.id {
                    println!(
                        "✅ Found connection FROM AI node: {} -> {}",
                        connection.from_node(),
                        connection.to_node()
                    );

                    // Find the target node (the tool to be executed)
                    if let Some(target_node) = state
                        .flow_definition
                        .nodes
                        .iter()
                        .find(|n| n.id == connection.to_node())
                    {
                        println!(
                            "📦 Target node: {} (type: {})",
                            target_node.id, target_node.node_type
                        );

                        // ✅ UNIFIED APPROACH: Present ALL connected nodes as MCP tools
                        let tool_spec = self.create_unified_tool_spec(target_node).await?;
                        if let Some(spec) = tool_spec {
                            connected_tools.push(spec);
                            println!("✅ Added connected tool: {}", target_node.node_type);
                        }
                    } else {
                        println!("❌ Target node not found: {}", connection.to_node());
                    }
                }
            }

            println!(
                "📋 AI node {} has {} connected tools",
                node_def.id,
                connected_tools.len()
            );
            Ok(connected_tools)
        } else {
            println!("❌ Flow not found: {}", flow_id);
            Ok(Vec::new())
        }
    }

    /// Create a unified MCP tool spec for any type of node
    async fn create_unified_tool_spec(
        &self,
        target_node: &FlowNodeDefinition,
    ) -> Result<Option<crate::services::mcp::ai_node::ToolSpec>, String> {
        // Determine the tool identifier and get schema
        let (tool_id, tool_name, description, parameters) =
            if let Some(mcp_tool_name) = &target_node.tool_name {
                // MCP Tool: Use explicit tool_name
                let registry = self
                    .flow_registry
                    .registry
                    .lock()
                    .map_err(|e| e.to_string())?;
                match registry.get_node_by_name(mcp_tool_name) {
                    Some(system_node) => (
                        mcp_tool_name.clone(),
                        system_node.id().clone(),
                        format!("MCP Tool: {}", system_node.description()),
                        system_node.parameters_schema(),
                    ),
                    _ => (
                        mcp_tool_name.clone(),
                        mcp_tool_name.clone(),
                        "MCP Tool".to_string(),
                        serde_json::json!({"type": "object", "properties": {}, "required": []}),
                    ),
                }
            } else if target_node.node_type.starts_with("system.") {
                // System Tool: Use node_type as identifier
                let registry = self
                    .flow_registry
                    .registry
                    .lock()
                    .map_err(|e| e.to_string())?;
                match registry.get_node(&target_node.node_type) {
                    Some(system_node) => (
                        target_node.node_type.clone(),
                        system_node.id().clone(),
                        format!("System Tool: {}", system_node.description()),
                        system_node.parameters_schema(),
                    ),
                    _ => {
                        println!(
                            "❌ System tool not found in registry: {}",
                            target_node.node_type
                        );
                        return Ok(None);
                    }
                }
            } else {
                // Generic Flow Node: Use node_type
                (
                    target_node.node_type.clone(),
                    target_node.name.clone(),
                    format!("Flow Tool: {}", target_node.name),
                    serde_json::json!({"type": "object", "properties": {}, "required": []}),
                )
            };

        // ✅ Return unified MCP tool spec
        Ok(Some(crate::services::mcp::ai_node::ToolSpec {
            name: tool_id.clone(), // AI calls this name
            description,
            parameters,
            node_id: target_node.id.clone(), // Flow executor uses this for execution
            tool_type: crate::services::mcp::ai_node::ToolType::FlowNode, // All treated as flow execution
        }))
    }

    /// Build complete inputs for a node (including previous outputs and node's own inputs)
    async fn build_node_inputs(
        &self,
        flow_id: &str,
        node_def: &FlowNodeDefinition,
    ) -> Result<HashMap<String, Value>, String> {
        let mut all_inputs = HashMap::new();

        // Start with the node's configured input values
        for (key, value) in &node_def.inputs {
            all_inputs.insert(key.clone(), value.clone());
        }

        // Override with connected values from previous nodes
        let previous_outputs = self.get_previous_outputs(flow_id, node_def).await?;
        for (key, value) in previous_outputs {
            all_inputs.insert(key, value);
        }

        Ok(all_inputs)
    }

    /// Value-driven nodes can be evaluated from value links without an explicit flow edge.
    /// Keep this allowlist tight: nodes here should be safe to execute as data dependencies.
    fn is_value_driven_node_type(node_type: &str) -> bool {
        matches!(node_type, "get-variable" | "set-variable")
    }

    /// Build an execution plan (topological order) for unresolved value dependencies.
    /// Returns node IDs that should be executed before `target_node`.
    fn collect_value_dependency_plan(
        flow_definition: &FlowDefinition,
        node_results: &HashMap<String, NodeResult>,
        target_node: &FlowNodeDefinition,
    ) -> Result<Vec<String>, String> {
        let node_lookup: HashMap<&str, &FlowNodeDefinition> = flow_definition
            .nodes
            .iter()
            .map(|node| (node.id.as_str(), node))
            .collect();

        fn dfs(
            current_node_id: &str,
            flow_definition: &FlowDefinition,
            node_lookup: &HashMap<&str, &FlowNodeDefinition>,
            node_results: &HashMap<String, NodeResult>,
            visiting: &mut HashSet<String>,
            visited: &mut HashSet<String>,
            scheduled: &mut HashSet<String>,
            plan: &mut Vec<String>,
        ) -> Result<(), String> {
            if visited.contains(current_node_id) {
                return Ok(());
            }

            let current_node = node_lookup.get(current_node_id).copied().ok_or_else(|| {
                format!(
                    "Missing node '{}' while resolving dependencies",
                    current_node_id
                )
            })?;

            if !visiting.insert(current_node_id.to_string()) {
                return Err(format!(
                    "Detected cyclic value dependency while resolving node '{}'",
                    current_node_id
                ));
            }

            for connection in flow_definition
                .connections
                .iter()
                .filter(|conn| conn.to_node() == current_node_id && conn.is_data())
            {
                let FlowConnection::Data {
                    from_node,
                    to_input,
                    ..
                } = connection
                else {
                    unreachable!("value-edge filter admitted a control edge")
                };
                let source_id = from_node.as_str();
                let source_node = node_lookup.get(source_id).copied().ok_or_else(|| {
                    format!("Missing source node '{}' for value connection", source_id)
                })?;

                match node_results.get(source_id) {
                    Some(source_result)
                        if matches!(&source_result.status, NodeStatus::Completed) =>
                    {
                        continue;
                    }
                    Some(source_result) if matches!(&source_result.status, NodeStatus::Failed) => {
                        let reason = source_result
                            .error
                            .clone()
                            .unwrap_or_else(|| "upstream node failed".to_string());
                        return Err(format!(
                            "Cannot execute '{}': dependency '{}' failed ({})",
                            current_node_id, source_node.name, reason
                        ));
                    }
                    _ => {}
                }

                if !FlowExecutor::is_value_driven_node_type(&source_node.node_type) {
                    return Err(format!(
                        "Cannot execute '{}' yet: input '{}' depends on '{}' which is not value-driven. Add a flow link to run it first.",
                        current_node_id, to_input, source_node.name
                    ));
                }

                dfs(
                    source_id,
                    flow_definition,
                    node_lookup,
                    node_results,
                    visiting,
                    visited,
                    scheduled,
                    plan,
                )?;

                if !scheduled.contains(source_id) {
                    scheduled.insert(source_id.to_string());
                    plan.push(source_id.to_string());
                }
            }

            // Variable UX: for Get Variable, implicitly resolve matching Set Variable producers.
            if current_node.node_type == "get-variable" {
                let variable_name = current_node
                    .inputs
                    .get("variableName")
                    .and_then(|v| v.as_str())
                    .map(str::trim)
                    .filter(|v| !v.is_empty());

                if let Some(variable_name) = variable_name {
                    for setter in flow_definition.nodes.iter().filter(|node| {
                        node.node_type == "set-variable"
                            && node
                                .inputs
                                .get("variableName")
                                .and_then(|v| v.as_str())
                                .map(str::trim)
                                == Some(variable_name)
                    }) {
                        match node_results.get(&setter.id) {
                            Some(result) if matches!(result.status, NodeStatus::Completed) => {
                                continue;
                            }
                            Some(result) if matches!(result.status, NodeStatus::Failed) => {
                                let reason = result
                                    .error
                                    .clone()
                                    .unwrap_or_else(|| "upstream node failed".to_string());
                                return Err(format!(
                                    "Cannot execute '{}': matching Set Variable '{}' failed ({})",
                                    current_node.name, setter.name, reason
                                ));
                            }
                            _ => {}
                        }

                        dfs(
                            setter.id.as_str(),
                            flow_definition,
                            node_lookup,
                            node_results,
                            visiting,
                            visited,
                            scheduled,
                            plan,
                        )?;

                        if !scheduled.contains(&setter.id) {
                            scheduled.insert(setter.id.clone());
                            plan.push(setter.id.clone());
                        }
                    }
                }
            }

            visiting.remove(current_node_id);
            visited.insert(current_node_id.to_string());
            Ok(())
        }

        let mut visiting = HashSet::new();
        let mut visited = HashSet::new();
        let mut scheduled = HashSet::new();
        let mut plan = Vec::new();

        dfs(
            target_node.id.as_str(),
            flow_definition,
            &node_lookup,
            node_results,
            &mut visiting,
            &mut visited,
            &mut scheduled,
            &mut plan,
        )?;

        Ok(plan)
    }

    /// Ensure all value dependencies are available before node execution.
    /// If dependency nodes are value-driven, execute them lazily.
    async fn ensure_value_dependencies_ready(
        &self,
        flow_id: &str,
        node_def: &FlowNodeDefinition,
    ) -> Result<(), String> {
        let (flow_definition, node_results) = {
            let active_flows = self.active_flows.lock().await;
            let Some(state) = active_flows.get(flow_id) else {
                return Ok(());
            };
            (state.flow_definition.clone(), state.node_results.clone())
        };

        let dependency_plan =
            Self::collect_value_dependency_plan(&flow_definition, &node_results, node_def)?;

        for dep_node_id in dependency_plan {
            if dep_node_id == node_def.id {
                continue;
            }

            let dep_node = {
                let active_flows = self.active_flows.lock().await;
                let Some(state) = active_flows.get(flow_id) else {
                    return Ok(());
                };

                if let Some(existing) = state.node_results.get(&dep_node_id) {
                    match &existing.status {
                        NodeStatus::Completed => continue,
                        NodeStatus::Failed => {
                            let reason = existing
                                .error
                                .clone()
                                .unwrap_or_else(|| "upstream node failed".to_string());
                            return Err(format!(
                                "Cannot execute '{}': dependency '{}' failed ({})",
                                node_def.name, dep_node_id, reason
                            ));
                        }
                        NodeStatus::Running | NodeStatus::Pending => {
                            return Err(format!(
                                "Cannot execute '{}': dependency '{}' is still {:?}",
                                node_def.name, dep_node_id, existing.status
                            ));
                        }
                        NodeStatus::Skipped => {}
                    }
                }

                state
                    .flow_definition
                    .nodes
                    .iter()
                    .find(|node| node.id == dep_node_id)
                    .cloned()
            };

            if let Some(dep_node_def) = dep_node {
                if !Self::is_value_driven_node_type(&dep_node_def.node_type) {
                    continue;
                }

                println!(
                    "🔄 Resolving value dependency '{}' before '{}'",
                    dep_node_def.name, node_def.name
                );
                self.execute_single_node_without_value_dependency_resolution(
                    flow_id,
                    &dep_node_def,
                )
                .await?;
            }
        }

        Ok(())
    }

    /// Recovery runs are intentionally inspected and resolved by an explicit
    /// operator/API decision; they are never auto-resumed after a crash.
    pub async fn list_recovery_required_runs(&self) -> Vec<FlowExecutionState> {
        self.active_flows
            .lock()
            .await
            .values()
            .filter(|state| matches!(state.status, FlowStatus::RecoveryRequired))
            .cloned()
            .collect()
    }

    pub async fn list_execution_history(
        &self,
        flow_id: Option<&str>,
        limit: u32,
    ) -> Result<Vec<ExecutionHistoryEntry>, String> {
        self.storage
            .list_execution_history(flow_id, limit.clamp(1, 500))
            .await
    }

    pub async fn list_open_workflow_wakes(&self) -> Result<Vec<WorkflowWake>, String> {
        self.storage.list_open_workflow_wakes(None).await
    }

    pub fn approval_request_details(wake: &WorkflowWake) -> Result<ApprovalRequestDetails, String> {
        if wake.kind != WorkflowWakeKind::Approval {
            return Err(format!("Wake {} is not an approval", wake.id));
        }
        serde_json::from_value(wake.payload.clone()).map_err(|error| {
            format!(
                "Approval wake {} has an invalid request payload: {}",
                wake.id, error
            )
        })
    }

    fn approval_resolution_from_wake(wake: &WorkflowWake) -> Result<ApprovalResolution, String> {
        serde_json::from_value(
            wake.payload
                .get("resolution")
                .cloned()
                .ok_or_else(|| format!("Approval wake {} has no resolution", wake.id))?,
        )
        .map_err(|error| {
            format!(
                "Approval wake {} has an invalid resolution: {}",
                wake.id, error
            )
        })
    }

    fn approval_outputs(wake_id: &str, resolution: &ApprovalResolution) -> HashMap<String, Value> {
        HashMap::from([
            (
                "isApproved".to_string(),
                json!(resolution.decision == ApprovalDecision::Approved),
            ),
            ("decision".to_string(), json!(resolution.decision.route())),
            (
                "selectedRoute".to_string(),
                json!(resolution.decision.route()),
            ),
            ("wakeId".to_string(), json!(wake_id)),
            (
                "decidedAt".to_string(),
                json!(resolution.decided_at.to_rfc3339()),
            ),
            ("decidedBy".to_string(), json!(resolution.decided_by)),
            ("comment".to_string(), json!(resolution.comment)),
        ])
    }

    /// Resolve one durable approval through the transport-neutral decision
    /// contract. Persistence wins the race before any live waiter or recovered
    /// graph is notified, so retries are idempotent and conflicting answers are
    /// rejected.
    pub async fn resolve_approval_wake(
        &self,
        wake_id: &str,
        request: ApprovalDecisionRequest,
    ) -> Result<ApprovalResolveOutcome, String> {
        let _admission = self.runtime_admission_gate.read().await;
        self.ensure_runtime_admission()?;
        if request.actor.id.trim().is_empty() || request.actor.channel.trim().is_empty() {
            return Err(
                "Approval decisions require an authenticated actor and channel".to_string(),
            );
        }
        if request
            .comment
            .as_ref()
            .is_some_and(|comment| comment.len() > 2_000)
        {
            return Err("Approval comments cannot exceed 2000 characters".to_string());
        }
        let wake = self
            .storage
            .load_workflow_wake(wake_id)
            .await?
            .ok_or_else(|| format!("Approval wake {} does not exist", wake_id))?;
        let details = Self::approval_request_details(&wake)?;
        {
            let active = self.active_flows.lock().await;
            let state = active.get(&wake.run_id);
            if let Some(state) = state {
                Self::validate_wake_resume_policy(&wake, state)?;
            } else if wake.payload.get("resolution").is_none() {
                return Err(format!(
                    "Approval run {} is no longer active; the late decision was not stored",
                    wake.run_id
                ));
            }
            if let Some(state) = state {
                if !matches!(
                    state.status,
                    FlowStatus::Pending | FlowStatus::Running | FlowStatus::RecoveryRequired
                ) {
                    return Err(format!(
                        "Approval run {} is terminal; the late decision was not stored",
                        wake.run_id
                    ));
                }
            }
        }
        match &request.decision {
            ApprovalDecision::Approved | ApprovalDecision::Denied => {
                if request.actor.channel == "system" {
                    return Err(
                        "The workflow runtime cannot approve or deny on behalf of a person"
                            .to_string(),
                    );
                }
                if !details.decision_policy.permits(&request.actor) {
                    return Err(format!(
                        "Approval channel '{}' is not authorized for {}",
                        request.actor.channel, wake_id
                    ));
                }
            }
            ApprovalDecision::TimedOut | ApprovalDecision::Cancelled => {
                if request.actor.channel != "system" {
                    return Err(
                        "Only the workflow runtime may time out or cancel an approval".to_string(),
                    );
                }
            }
        }
        if request.decision == ApprovalDecision::Approved
            && wake.payload.get("resolution").is_none()
        {
            self.validate_approval_effect_bindings(&wake.run_id, &details)
                .await?;
        }
        let resolution = ApprovalResolution {
            decision: request.decision,
            decided_at: self.clock.now(),
            decided_by: request.actor,
            comment: request.comment,
        };
        let outcome = self
            .storage
            .resolve_workflow_approval(wake_id, &resolution)
            .await?;
        let wake = outcome.wake().clone();
        let persisted_resolution = Self::approval_resolution_from_wake(&wake)?;
        if persisted_resolution.decision != resolution.decision {
            return Err(format!(
                "Approval {} was already resolved as {}",
                wake_id,
                persisted_resolution.decision.route()
            ));
        }

        if persisted_resolution.decision == ApprovalDecision::Approved
            && self.active_flows.lock().await.contains_key(&wake.run_id)
        {
            self.remember_approved_effect_bindings(&wake.run_id, &details)
                .await?;
        }

        if outcome.was_applied() {
            self.emit_event(FlowEventPayload::ApprovalResolved {
                flow_id: wake.run_id.clone(),
                run_id: wake.run_id.clone(),
                wake_id: wake.id.clone(),
                resolution: persisted_resolution.clone(),
            })
            .await;
        }

        if let Some(sender) = self.pending_approval_wakes.lock().await.remove(wake_id) {
            let _ = sender.send(persisted_resolution);
            return Ok(outcome);
        }

        let run_status = self
            .active_flows
            .lock()
            .await
            .get(&wake.run_id)
            .map(|state| state.status.clone());
        if matches!(run_status, Some(FlowStatus::Running | FlowStatus::Pending)) {
            // The durable row may be resolved in the narrow interval before a
            // newly executing node installs its in-memory waiter. The node
            // closes this race by re-reading the row after registration.
            return Ok(outcome);
        }
        if !outcome.was_applied() && !matches!(run_status, Some(FlowStatus::RecoveryRequired)) {
            // The caller may be retrying because its acknowledgement was lost
            // after the run already continued or completed. Returning the
            // persisted winner is the idempotent response.
            return Ok(outcome);
        }
        if !matches!(run_status, Some(FlowStatus::RecoveryRequired)) {
            return Err(format!(
                "Approval run {} is not awaiting a decision",
                wake.run_id
            ));
        }

        // After restart there is no live future. Materialize the completed node
        // from the durable resolution, checkpoint it, then continue the graph.
        {
            let mut active = self.active_flows.lock().await;
            let state = active
                .get_mut(&wake.run_id)
                .ok_or_else(|| format!("Approval run {} is not loaded", wake.run_id))?;
            let result = state
                .node_results
                .get_mut(&wake.node_id)
                .ok_or("Approval node result is missing")?;
            result.status = NodeStatus::Completed;
            result.error = None;
            result.outputs = Self::approval_outputs(&wake.id, &persisted_resolution);
            state.completed_nodes.insert(wake.node_id.clone());
            state.current_node = None;
        }
        self.checkpoint_run(&wake.run_id).await;
        self.storage
            .set_workflow_wake_status(&wake.id, WorkflowWakeStatus::Consumed)
            .await?;
        self.try_start_recovered_run_if_ready_admitted(&wake.run_id)
            .await?;
        Ok(outcome)
    }

    pub async fn fail_recovery_run(&self, run_id: &str, reason: String) -> Result<(), String> {
        let _admission = self.runtime_admission_gate.read().await;
        self.ensure_runtime_admission()?;
        let is_recovery = self
            .active_flows
            .lock()
            .await
            .get(run_id)
            .is_some_and(|state| matches!(state.status, FlowStatus::RecoveryRequired));
        if !is_recovery {
            return Err(format!("Run {} is not awaiting recovery", run_id));
        }
        self.fail_run_if_active(run_id, reason).await;
        Ok(())
    }

    /// Reconcile every in-flight node from its per-operation durable ledger.
    /// Planning completes before state mutation, so one unsafe concurrent
    /// operation prevents a partial recovery of the run.
    pub async fn resume_recovery_run(&self, run_id: &str) -> Result<(), String> {
        let _admission = self.runtime_admission_gate.read().await;
        self.ensure_runtime_admission()?;
        let running_nodes: Vec<String> = {
            let active = self.active_flows.lock().await;
            let state = active
                .get(run_id)
                .ok_or_else(|| format!("Run {} not found", run_id))?;
            if !matches!(state.status, FlowStatus::RecoveryRequired) {
                return Err(format!("Run {} is not awaiting recovery", run_id));
            }
            state
                .node_results
                .values()
                .filter(|result| result.status == NodeStatus::Running)
                .map(|result| result.node_id.clone())
                .collect()
        };

        if running_nodes.is_empty() {
            {
                let mut active = self.active_flows.lock().await;
                let state = active
                    .get_mut(run_id)
                    .ok_or_else(|| format!("Run {} not found", run_id))?;
                state.status = FlowStatus::Running;
            }
            self.checkpoint_run(run_id).await;
            return self.spawn_recovered_run_admitted(run_id).await;
        }

        let wakes = self.storage.list_open_workflow_wakes(Some(run_id)).await?;
        {
            let active = self.active_flows.lock().await;
            let state = active
                .get(run_id)
                .ok_or_else(|| format!("Run {} not found", run_id))?;
            for wake in &wakes {
                Self::validate_wake_resume_policy(wake, state)?;
            }
        }
        if let Some(wake) = wakes
            .iter()
            .find(|wake| running_nodes.contains(&wake.node_id))
        {
            return Err(format!(
                "Node {} is controlled by an open {:?} wake and cannot be manually replayed",
                wake.node_id, wake.kind
            ));
        }

        let deliveries = self.storage.list_overlay_command_deliveries(run_id).await?;
        let outbox = self.storage.list_side_effect_outbox().await?;
        let mut resolutions = Vec::with_capacity(running_nodes.len());

        for node_id in &running_nodes {
            if let Some(delivery) = deliveries
                .iter()
                .rev()
                .find(|delivery| delivery.node_id == *node_id)
            {
                if delivery.status != crate::events::OverlayCommandDeliveryStatus::Acknowledged {
                    return Err(format!(
                        "Overlay node {} has status {:?}; resume is unsafe",
                        node_id, delivery.status
                    ));
                }
                let receipt = delivery
                    .acknowledgement
                    .as_ref()
                    .ok_or_else(|| format!("Overlay node {} has no applied receipt", node_id))?;
                if receipt.status != crate::events::OverlayCommandAckStatus::Accepted {
                    return Err(format!("Overlay node {} was not accepted", node_id));
                }
                resolutions.push(LedgerRecoveryResolution::Overlay(delivery.clone()));
                continue;
            }

            if let Some(entry) = outbox
                .iter()
                .find(|entry| entry.run_id == run_id && entry.node_id == *node_id)
            {
                let entry = self.reconcile_side_effect_invocation(entry).await?;
                if entry.status == crate::events::SideEffectOutboxStatus::Succeeded {
                    resolutions.push(LedgerRecoveryResolution::CompletedSideEffect(entry));
                    continue;
                }
                let action: UniversalAction = serde_json::from_value(entry.action.clone())
                    .map_err(|error| format!("Stored side effect is invalid: {}", error))?;
                let policy = self.action_recovery_policy(&action).await?;
                if !matches!(
                    policy,
                    RecoveryPolicy::RetryWithIdempotency | RecoveryPolicy::Reconcile
                ) {
                    return Err(format!(
                        "Side-effect node {} is fail-closed and cannot be retried",
                        node_id
                    ));
                }
                resolutions.push(LedgerRecoveryResolution::RetrySideEffect(entry));
                continue;
            }

            return Err(format!(
                "Running node {} has no durable wake, overlay receipt, or outbox record; recovery is fail-closed",
                node_id
            ));
        }

        let retry_keys: Vec<String> = resolutions
            .iter()
            .filter_map(|resolution| match resolution {
                LedgerRecoveryResolution::RetrySideEffect(entry) => {
                    Some(entry.idempotency_key.clone())
                }
                _ => None,
            })
            .collect();
        self.approved_side_effect_retries
            .lock()
            .await
            .extend(retry_keys);

        {
            let mut active = self.active_flows.lock().await;
            let state = active
                .get_mut(run_id)
                .ok_or_else(|| format!("Run {} not found", run_id))?;
            for resolution in resolutions {
                match resolution {
                    LedgerRecoveryResolution::Overlay(delivery) => {
                        let receipt = delivery
                            .acknowledgement
                            .expect("validated overlay acknowledgement");
                        let result = state
                            .node_results
                            .get_mut(&delivery.node_id)
                            .ok_or("Recovery node result is missing")?;
                        result.status = NodeStatus::Completed;
                        result.error = None;
                        result.outputs.extend([
                            ("commandId".to_string(), json!(delivery.command.command_id)),
                            ("acknowledged".to_string(), json!(true)),
                            ("ackStatus".to_string(), json!(receipt.status)),
                            ("overlayInstanceId".to_string(), json!(receipt.instance_id)),
                            (
                                "appliedRevision".to_string(),
                                json!(receipt.applied_revision),
                            ),
                        ]);
                        state.completed_nodes.insert(delivery.node_id);
                    }
                    LedgerRecoveryResolution::CompletedSideEffect(entry) => {
                        let result = state
                            .node_results
                            .get_mut(&entry.node_id)
                            .ok_or("Recovery node result is missing")?;
                        result.status = NodeStatus::Completed;
                        result.error = None;
                        result.outputs.insert("status".to_string(), json!(true));
                        result
                            .outputs
                            .insert("recoveredFromOutbox".to_string(), json!(true));
                        state.completed_nodes.insert(entry.node_id);
                    }
                    LedgerRecoveryResolution::RetrySideEffect(entry) => {
                        state.node_results.remove(&entry.node_id);
                        state.completed_nodes.remove(&entry.node_id);
                        state.failed_nodes.remove(&entry.node_id);
                    }
                }
            }
            state.current_node = None;
            state.status = FlowStatus::Running;
        }
        self.checkpoint_run(run_id).await;
        self.spawn_recovered_run_admitted(run_id).await
    }

    fn schedule_durable_timer_recovery(&self, run_id: String, wakes: Vec<WorkflowWake>) {
        for wake in wakes {
            let executor = self.clone();
            let run_id = run_id.clone();
            let runtime_shutdown = self.runtime_shutdown_token();
            self.runtime_tasks.spawn(async move {
                tokio::select! {
                    _ = async {
                        if let Some(deadline) = wake.due_at {
                            wait_until_wall_clock_deadline(executor.clock.clone(), deadline).await;
                        }
                    } => {}
                    _ = runtime_shutdown.cancelled() => return,
                }
                if !executor.wait_for_runtime_admission().await {
                    return;
                }
                let _admission = executor.runtime_admission_gate.read().await;
                if executor.ensure_runtime_admission().is_err() {
                    return;
                }
                let open = match executor
                    .storage
                    .list_open_workflow_wakes(Some(&run_id))
                    .await
                {
                    Ok(open) => open,
                    Err(error) => {
                        executor.fail_run_if_active(&run_id, error).await;
                        return;
                    }
                };
                if !open.iter().any(|candidate| candidate.id == wake.id) {
                    return;
                }
                {
                    let mut active = executor.active_flows.lock().await;
                    let Some(state) = active.get_mut(&run_id) else {
                        return;
                    };
                    if !matches!(state.status, FlowStatus::RecoveryRequired) {
                        return;
                    }
                    if let Err(error) = Self::validate_wake_resume_policy(&wake, state) {
                        drop(active);
                        executor.fail_run_if_active(&run_id, error).await;
                        return;
                    }
                    let Some(result) = state.node_results.get_mut(&wake.node_id) else {
                        return;
                    };
                    if result.status == NodeStatus::Running {
                        result.status = NodeStatus::Completed;
                        result.error = None;
                        result.outputs = HashMap::from([
                            ("success".to_string(), json!(true)),
                            ("recoveredDurableTimer".to_string(), json!(true)),
                            (
                                "completed_at".to_string(),
                                json!(executor.clock.now().to_rfc3339()),
                            ),
                        ]);
                    }
                    state.completed_nodes.insert(wake.node_id.clone());
                    state.current_node = None;
                }
                executor.checkpoint_run(&run_id).await;
                if let Err(error) = executor
                    .storage
                    .set_workflow_wake_status(&wake.id, WorkflowWakeStatus::Consumed)
                    .await
                {
                    executor.fail_run_if_active(&run_id, error).await;
                    return;
                }
                if let Err(error) = executor
                    .try_start_recovered_run_if_ready_admitted(&run_id)
                    .await
                {
                    executor.fail_run_if_active(&run_id, error).await;
                }
            });
        }
    }

    async fn try_start_recovered_run_if_ready_admitted(&self, run_id: &str) -> Result<(), String> {
        let ready = {
            let mut active = self.active_flows.lock().await;
            let state = active
                .get_mut(run_id)
                .ok_or_else(|| format!("Run {} not found", run_id))?;
            if !matches!(state.status, FlowStatus::RecoveryRequired) {
                return Ok(());
            }
            if state
                .node_results
                .values()
                .any(|result| result.status == NodeStatus::Running)
            {
                false
            } else {
                state.status = FlowStatus::Running;
                state.current_node = None;
                true
            }
        };
        if ready {
            self.checkpoint_run(run_id).await;
            self.spawn_recovered_run_admitted(run_id).await?;
        }
        Ok(())
    }

    fn schedule_durable_retry_recovery(
        &self,
        run_id: String,
        wakes: Vec<WorkflowWake>,
        deadline: DateTime<Utc>,
    ) {
        let executor = self.clone();
        let runtime_shutdown = self.runtime_shutdown_token();
        self.runtime_tasks.spawn(async move {
            tokio::select! {
                _ = wait_until_wall_clock_deadline(executor.clock.clone(), deadline) => {}
                _ = runtime_shutdown.cancelled() => return,
            }
            if !executor.wait_for_runtime_admission().await {
                return;
            }
            let _admission = executor.runtime_admission_gate.read().await;
            if executor.ensure_runtime_admission().is_err() {
                return;
            }
            let open = match executor
                .storage
                .list_open_workflow_wakes(Some(&run_id))
                .await
            {
                Ok(open) => open,
                Err(error) => {
                    executor.fail_run_if_active(&run_id, error).await;
                    return;
                }
            };
            let open_ids: HashSet<_> = open.iter().map(|wake| wake.id.as_str()).collect();
            if !wakes.iter().all(|wake| open_ids.contains(wake.id.as_str())) {
                return;
            }
            {
                let mut active = executor.active_flows.lock().await;
                let Some(state) = active.get_mut(&run_id) else {
                    return;
                };
                if !matches!(state.status, FlowStatus::RecoveryRequired) {
                    return;
                }
                for wake in &wakes {
                    if let Err(error) = Self::validate_wake_resume_policy(wake, state) {
                        drop(active);
                        executor.fail_run_if_active(&run_id, error).await;
                        return;
                    }
                }
                state.current_node = None;
                for wake in &wakes {
                    let still_running = state
                        .node_results
                        .get(&wake.node_id)
                        .is_some_and(|result| result.status == NodeStatus::Running);
                    if still_running {
                        state.node_results.remove(&wake.node_id);
                        state.completed_nodes.remove(&wake.node_id);
                        state.failed_nodes.remove(&wake.node_id);
                    }
                }
                state.execution_log.push(ExecutionLogEntry {
                    timestamp: Utc::now(),
                    level: LogLevel::Info,
                    node_id: None,
                    message: "Resuming scheduler-owned durable retry waits".to_string(),
                    details: Some(json!({
                        "wakeIds": wakes.iter().map(|wake| &wake.id).collect::<Vec<_>>()
                    })),
                });
            }
            executor.checkpoint_run(&run_id).await;
            for wake in &wakes {
                if let Err(error) = executor
                    .storage
                    .set_workflow_wake_status(&wake.id, WorkflowWakeStatus::Consumed)
                    .await
                {
                    executor.fail_run_if_active(&run_id, error).await;
                    return;
                }
            }
            if let Err(error) = executor
                .try_start_recovered_run_if_ready_admitted(&run_id)
                .await
            {
                executor.fail_run_if_active(&run_id, error).await;
            }
        });
    }

    async fn spawn_recovered_run_admitted(&self, run_id: &str) -> Result<(), String> {
        let (cancellation_tx, mut cancellation_rx) = tokio::sync::watch::channel(false);
        self.cancellation_signals
            .lock()
            .await
            .insert(run_id.to_string(), cancellation_tx);
        let executor = self.clone();
        let run_id_owned = run_id.to_string();
        let runtime_shutdown = self.runtime_shutdown_token();
        self.runtime_tasks.spawn(async move {
            tokio::select! {
                result = executor.execute_flow_internal(run_id_owned.clone()) => {
                    if let Err(error) = result {
                        executor.fail_run_if_active(&run_id_owned, error).await;
                    }
                }
                changed = cancellation_rx.changed() => {
                    if changed.is_ok() && *cancellation_rx.borrow() {
                        log::info!("Cancelled recovered workflow task {}", run_id_owned);
                    }
                }
                _ = runtime_shutdown.cancelled() => {
                    log::info!("Stopped recovered workflow task {} with its runtime owner", run_id_owned);
                }
            }
            executor
                .cancellation_signals
                .lock()
                .await
                .remove(&run_id_owned);
            executor.run_flow_ids.write().await.remove(&run_id_owned);
        });
        Ok(())
    }

    /// Get previous outputs for a node
    async fn get_previous_outputs(
        &self,
        flow_id: &str,
        node_def: &FlowNodeDefinition,
    ) -> Result<HashMap<String, Value>, String> {
        let active_flows = self.active_flows.lock().await;
        if let Some(state) = active_flows.get(flow_id) {
            let mut previous_outputs = HashMap::new();

            // Find all connections TO this node (incoming value connections)
            for connection in &state.flow_definition.connections {
                if connection.to_node() == node_def.id && connection.is_data() {
                    let FlowConnection::Data {
                        from_node,
                        from_output,
                        to_input,
                        ..
                    } = connection
                    else {
                        unreachable!("value-edge filter admitted a control edge")
                    };

                    // Find the source node's result
                    if let Some(source_result) = state.node_results.get(from_node) {
                        if matches!(source_result.status, NodeStatus::Completed) {
                            if let Some(output_value) = source_result.outputs.get(from_output) {
                                // For AI text generators, we want the response text, not the full object
                                let processed_value =
                                    if let Some(response_obj) = output_value.as_object() {
                                        if let Some(response_text) = response_obj.get("response") {
                                            response_text.clone()
                                        } else {
                                            output_value.clone()
                                        }
                                    } else {
                                        output_value.clone()
                                    };

                                previous_outputs.insert(to_input.clone(), processed_value);
                            }
                        }
                    }
                }
            }

            Ok(previous_outputs)
        } else {
            Ok(HashMap::new())
        }
    }

    /// Emit one canonical public flow event to the EventBus.
    ///
    /// Callers key work by run ID. This method resolves and writes the source
    /// flow-definition ID before broadcasting, so there is only one flow-event
    /// representation throughout the runtime.
    async fn emit_event(&self, mut event: FlowEventPayload) {
        let run_id = event.run_id().to_string();
        let source_flow_id = self
            .run_flow_ids
            .read()
            .await
            .get(&run_id)
            .cloned()
            .unwrap_or_else(|| run_id.clone());
        event.set_flow_id(source_flow_id);

        match &self.event_emitter {
            EventEmitter::Universal(platform_context) => {
                if let Err(error) = platform_context
                    .event_bus
                    .emit(crate::events::EventPayload::Flow(event))
                {
                    log::error!("Failed to emit flow event to EventBus: {}", error);
                }
            }
        }
    }
    /// Calculate progress based on reachable nodes only (excludes disconnected nodes)
    fn calculate_progress(
        &self,
        flow_definition: &FlowDefinition,
        completed_nodes: &HashSet<String>,
        node_results: &HashMap<String, NodeResult>,
    ) -> (f64, u32, u32) {
        let reachable_nodes = Self::reachable_for_resolved_routes(flow_definition, node_results);
        let reachable_count = reachable_nodes.len() as u32;
        let completed_count = completed_nodes
            .iter()
            .filter(|node_id| reachable_nodes.contains(*node_id))
            .count() as u32;

        let progress_percentage = if reachable_count > 0 {
            (completed_count as f64 / reachable_count as f64) * 100.0
        } else {
            0.0
        };

        (progress_percentage, reachable_count, completed_count)
    }

    /// Find all nodes reachable from trigger node
    /// Find the entry point for flow execution using start_node_id
    fn find_flow_entry_points<'a>(
        &self,
        flow_definition: &'a FlowDefinition,
    ) -> Vec<&'a FlowNodeDefinition> {
        let nodes = &flow_definition.nodes;

        // Use start_node_id as the ONLY entry point
        if let Some(start_node) = nodes
            .iter()
            .find(|node| node.id == flow_definition.start_node_id)
        {
            println!("🔍 Flow entry point analysis:");
            println!("   Total nodes: {}", nodes.len());
            println!("   Start node ID: {}", flow_definition.start_node_id);
            println!(
                "   🎯 Entry point: {} ({})",
                start_node.name, start_node.node_type
            );

            vec![start_node]
        } else {
            println!(
                "❌ Start node '{}' not found in flow definition",
                flow_definition.start_node_id
            );
            println!(
                "   Available nodes: {:?}",
                nodes.iter().map(|n| &n.id).collect::<Vec<_>>()
            );

            // Fallback: find trigger nodes only
            let trigger_entries: Vec<&FlowNodeDefinition> = nodes
                .iter()
                .filter(|node| self.is_trigger_node(&node.node_type))
                .collect();

            if !trigger_entries.is_empty() {
                println!("🔄 Fallback: Using trigger node as entry point");
                for entry in &trigger_entries {
                    println!("   🎯 Trigger: {} ({})", entry.name, entry.node_type);
                }
                trigger_entries
            } else {
                println!("❌ No valid entry point found - flow cannot execute");
                vec![]
            }
        }
    }

    /// Check if a node type is a trigger node
    fn is_trigger_node(&self, node_type: &str) -> bool {
        matches!(
            node_type,
            "webhook-trigger"
                | "button-trigger"
                | "timer-trigger"
                | "file-trigger"
                | "mqtt-trigger"
        )
    }

    fn find_reachable_nodes_from_trigger(
        &self,
        flow_definition: &FlowDefinition,
    ) -> HashSet<String> {
        let connections = &flow_definition.connections;
        let nodes = &flow_definition.nodes;

        // Start from the start_node_id only
        let mut reachable = HashSet::new();
        let mut queue = vec![flow_definition.start_node_id.clone()];

        // Traverse from start node following connections
        while let Some(current_node_id) = queue.pop() {
            if reachable.contains(&current_node_id) {
                continue; // Already processed
            }

            // Only add node if it actually exists in the flow
            if nodes.iter().any(|n| n.id == current_node_id) {
                reachable.insert(current_node_id.clone());

                // Find all nodes connected from this node
                for connection in connections
                    .iter()
                    .filter(|conn| conn.from_node() == current_node_id && conn.is_control())
                {
                    if !reachable.contains(connection.to_node()) {
                        queue.push(connection.to_node().to_string());
                    }
                }
            }
        }

        println!(
            "🔍 Reachable nodes from start node '{}': {} nodes",
            flow_definition.start_node_id,
            reachable.len()
        );

        reachable
    }

    /// Build the terminal timeline and atomically replace the live snapshot
    /// with its archived history record.
    async fn persist_terminal_history(
        &self,
        execution_state: &FlowExecutionState,
    ) -> Result<(), String> {
        let duration_ms = execution_state
            .completed_at
            .map(|completed| (completed - execution_state.started_at).num_milliseconds() as u64);

        let (_, reachable_count, completed_count) = self.calculate_progress(
            &execution_state.flow_definition,
            &execution_state.completed_nodes,
            &execution_state.node_results,
        );

        // Build detailed node execution information
        println!(
            "📋 Building execution history with {} node results",
            execution_state.sorted_node_results.len()
        );
        let node_execution_details: Vec<NodeExecutionDetail> = execution_state
            .sorted_node_results
            .iter()
            .map(|node_result| {
                let node_def = execution_state
                    .flow_definition
                    .nodes
                    .iter()
                    .find(|n| n.id == node_result.node_id)
                    .unwrap_or(&FlowNodeDefinition {
                        id: node_result.node_id.clone(),
                        node_type: node_result.node_type.clone(),
                        name: node_result.node_name.clone(),
                        inputs: HashMap::new(),
                        outputs: vec![],
                        ai_behavior: None,
                        ai_model: None,
                        prompt_template: None,
                        tool_name: None,
                        position: (0.0, 0.0),
                        ui_data: NodeUIData::default(),
                        execution_config: NodeExecutionConfig::default(),
                    });

                // Calculate start time from execution time
                let started_at = execution_state.started_at
                    + chrono::Duration::milliseconds(
                        (node_result.execution_time_ms as i64).saturating_sub(1000),
                    );

                let completed_at = if matches!(
                    node_result.status,
                    NodeStatus::Completed | NodeStatus::Failed
                ) {
                    Some(
                        started_at
                            + chrono::Duration::milliseconds(node_result.execution_time_ms as i64),
                    )
                } else {
                    None
                };

                let detail = NodeExecutionDetail {
                    node_id: node_result.node_id.clone(),
                    node_name: node_result.node_name.clone(),
                    node_type: node_result.node_type.clone(),
                    status: node_result.status.clone(),
                    inputs: node_result.inputs.clone(),
                    outputs: node_result.outputs.clone(),
                    error: node_result.error.clone(),
                    execution_time_ms: node_result.execution_time_ms,
                    started_at,
                    completed_at,
                    retry_count: node_result.retry_count,
                };

                println!(
                    "   📊 Node '{}' - {} inputs, {} outputs",
                    detail.node_name,
                    detail.inputs.len(),
                    detail.outputs.len()
                );

                detail
            })
            .collect();

        let history_entry = ExecutionHistoryEntry {
            id: format!("history-{}", execution_state.run_id),
            run_id: execution_state.run_id.clone(),
            flow_id: execution_state.flow_id.clone(),
            flow_name: execution_state.flow_definition.name.clone(),
            status: execution_state.status.clone(),
            started_at: execution_state.started_at,
            completed_at: execution_state.completed_at,
            duration_ms,
            reachable_node_count: reachable_count,
            completed_nodes: completed_count,
            failed_nodes: execution_state.failed_nodes.len() as u32,
            error_message: execution_state
                .execution_log
                .iter()
                .find(|log| matches!(log.level, LogLevel::Error))
                .map(|log| log.message.clone()),
            final_results: if matches!(execution_state.status, FlowStatus::Completed) {
                Some(
                    execution_state
                        .node_results
                        .iter()
                        .map(|(id, result)| {
                            (
                                id.clone(),
                                serde_json::to_value(&result.outputs).unwrap_or(Value::Null),
                            )
                        })
                        .collect(),
                )
            } else {
                None
            },
            node_execution_details,
        };

        // History and the removal of the resumable snapshot are one commit.
        // A crash can therefore never leave both a terminal history row and a
        // snapshot that startup later misclassifies as interrupted.
        self.storage.finalize_execution(&history_entry).await?;

        self.emit_event(FlowEventPayload::ExecutionHistoryAdded {
            flow_id: history_entry.flow_id.clone(),
            run_id: history_entry.run_id.clone(),
            history_entry,
        })
        .await;

        Ok(())
    }

    /// Map a tool execution result to the declared outputs schema
    /// This allows nodes to expose structured outputs instead of generic "result"
    fn map_result_to_outputs(
        result: &serde_json::Value,
        node: &dyn crate::services::flow::traits::FlowNode,
    ) -> HashMap<String, Value> {
        let mut outputs = HashMap::new();
        let output_schema = node.outputs_schema();

        // Always include full result as catch-all
        outputs.insert("result".to_string(), result.clone());

        // Extract schema-defined outputs (those that aren't "result")
        let schema_outputs: Vec<_> = output_schema.iter().filter(|o| o.id != "result").collect();

        if !schema_outputs.is_empty() {
            // Check if result has an "outputs" array (WASM plugin format)
            if let Some(outputs_array) = result.get("outputs").and_then(|v| v.as_array()) {
                // Map array indices to schema-defined output IDs
                for (idx, output_def) in schema_outputs.iter().enumerate() {
                    let value = outputs_array.get(idx).cloned().unwrap_or(Value::Null);
                    outputs.insert(output_def.id.clone(), value);
                }
            } else {
                // Fall back to field-based extraction for non-array formats
                for output_def in &schema_outputs {
                    // Try to extract field from result:
                    // 1. First try top level (for regular nodes)
                    // 2. Then try structuredContent (for MCP tools)
                    // 3. Otherwise set to null

                    let value = if let Some(val) = result.get(&output_def.id) {
                        // Found at top level (regular nodes)
                        val.clone()
                    } else if let Some(structured) = result.get("structuredContent") {
                        // Try in structuredContent (MCP tools)
                        structured
                            .get(&output_def.id)
                            .cloned()
                            .unwrap_or(Value::Null)
                    } else {
                        // Not found anywhere
                        Value::Null
                    };

                    outputs.insert(output_def.id.clone(), value);
                }
            }
        }

        outputs
    }

    /// Check if a tool result indicates an error and return the error if so
    fn check_tool_result_for_error(result: &serde_json::Value) -> Result<(), String> {
        if let Some(obj) = result.as_object() {
            // Check for explicit error field
            if let Some(error_val) = obj.get("error") {
                if !error_val.is_null() {
                    if let Some(error_str) = error_val.as_str() {
                        return Err(error_str.to_string());
                    } else {
                        return Err(format!("Tool execution failed: {}", error_val));
                    }
                }
            }

            // Check for success: false pattern
            if let Some(success_val) = obj.get("success") {
                if success_val == &serde_json::Value::Bool(false) {
                    let error_msg = obj
                        .get("error")
                        .or_else(|| obj.get("message"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("Tool execution failed");
                    return Err(error_msg.to_string());
                }
            }

            // Check for isError: true pattern
            if let Some(is_error) = obj.get("isError") {
                if is_error == &serde_json::Value::Bool(true) {
                    let error_msg = obj
                        .get("message")
                        .or_else(|| obj.get("error"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("Tool execution failed");
                    return Err(error_msg.to_string());
                }
            }

            // 🔥 CRITICAL: Check nested structuredContent.result for errors
            // Many MCP tools (like HexStrike) nest errors here even when isError: false
            if let Some(structured) = obj.get("structuredContent").and_then(|v| v.as_object()) {
                if let Some(result_obj) = structured.get("result").and_then(|v| v.as_object()) {
                    // Check nested error field
                    if let Some(error_val) = result_obj.get("error") {
                        if !error_val.is_null() {
                            if let Some(error_str) = error_val.as_str() {
                                return Err(error_str.to_string());
                            } else {
                                return Err(format!("Tool execution failed: {}", error_val));
                            }
                        }
                    }

                    // Check nested success: false
                    if let Some(success_val) = result_obj.get("success") {
                        if success_val == &serde_json::Value::Bool(false) {
                            let error_msg = result_obj
                                .get("error")
                                .or_else(|| result_obj.get("message"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("Tool execution failed");
                            return Err(error_msg.to_string());
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Emit progress update event
    async fn emit_progress_update(&self, flow_id: &str, execution_state: &FlowExecutionState) {
        let (progress_percentage, reachable_count, completed_count) = self.calculate_progress(
            &execution_state.flow_definition,
            &execution_state.completed_nodes,
            &execution_state.node_results,
        );

        self.emit_event(FlowEventPayload::FlowProgressUpdate {
            flow_id: flow_id.to_string(),
            run_id: flow_id.to_string(),
            progress_percentage,
            reachable_nodes: reachable_count,
            completed_nodes: completed_count,
            current_node: execution_state.current_node.clone(),
        })
        .await;
    }

    /// Save execution history to database
    pub async fn get_database_stats(&self) -> Result<DatabaseStats, String> {
        self.storage.get_database_stats().await
    }

    pub async fn cleanup_database(
        &self,
        retention_days: u32,
        max_size_mb: u32,
    ) -> Result<usize, String> {
        self.storage
            .cleanup_execution_history(retention_days, max_size_mb)
            .await
    }

    pub async fn vacuum_database(&self) -> Result<(), String> {
        self.storage.vacuum_database().await
    }
}

// Implement Clone for FlowExecutor
impl Clone for FlowExecutor {
    fn clone(&self) -> Self {
        Self {
            flow_registry: self.flow_registry.clone(),
            chat_state: self.chat_state.clone(),
            event_emitter: match &self.event_emitter {
                EventEmitter::Universal(platform_context) => {
                    EventEmitter::Universal(platform_context.clone())
                }
            },
            active_flows: self.active_flows.clone(),
            run_flow_ids: self.run_flow_ids.clone(),
            ai_executor: AiExecutor::new((*self.flow_registry).clone(), (*self.chat_state).clone()),
            storage: self.storage.clone(),
            clock: self.clock.clone(),
            action_invocation_service: self.action_invocation_service.clone(),
            cancellation_signals: self.cancellation_signals.clone(),
            recent_triggers: self.recent_triggers.clone(),
            pending_overlay_acks: self.pending_overlay_acks.clone(),
            approved_side_effect_retries: self.approved_side_effect_retries.clone(),
            pending_approval_wakes: self.pending_approval_wakes.clone(),
            run_response_senders: self.run_response_senders.clone(),
            run_response_receivers: self.run_response_receivers.clone(),
            runtime_admission_gate: self.runtime_admission_gate.clone(),
            accepting_runtime_work: self.accepting_runtime_work.clone(),
            runtime_admission_notify: self.runtime_admission_notify.clone(),
            runtime_tasks: self.runtime_tasks.clone(),
            runtime_shutdown: self.runtime_shutdown.clone(),
        }
    }
}

// ============================================================================
// FLOW STORAGE TRAITS
// ============================================================================

#[async_trait]
pub trait FlowStorage: Send + Sync {
    async fn observed_stream_values(
        &self,
    ) -> Result<Vec<crate::services::streaming::stream_values::ObservedStreamValue>, String> {
        Err("Observed stream values are unavailable in this storage adapter".into())
    }
    /// Save a flow definition
    async fn save_flow(&self, flow: &FlowDefinition) -> Result<(), String>;

    /// Compare and replace a saved flow atomically at the storage boundary.
    async fn replace_flow_at_revision(
        &self,
        expected_revision: &str,
        flow: &FlowDefinition,
    ) -> Result<FlowDefinition, String>;

    /// Load a flow definition
    async fn load_flow(&self, flow_id: &str) -> Result<FlowDefinition, String>;

    /// Save flow execution state
    async fn save_execution_state(&self, state: &FlowExecutionState) -> Result<(), String>;

    /// Load flow execution state  
    async fn load_execution_state(&self, flow_id: &str) -> Result<FlowExecutionState, String>;

    /// Return runs that were active when the process last stopped.
    async fn list_recoverable_execution_states(&self) -> Result<Vec<FlowExecutionState>, String>;

    /// Atomically archive a terminal run and remove its resumable snapshot.
    async fn finalize_execution(&self, history_entry: &ExecutionHistoryEntry)
        -> Result<(), String>;

    /// Read archived terminal runs, newest first.
    async fn list_execution_history(
        &self,
        flow_id: Option<&str>,
        limit: u32,
    ) -> Result<Vec<ExecutionHistoryEntry>, String>;

    /// List all flows
    async fn list_flows(&self) -> Result<Vec<FlowSummary>, String>;

    /// Delete a flow
    async fn delete_flow(&self, flow_id: &str) -> Result<(), String>;

    /// Archive completed flow
    async fn archive_flow(&self, flow_id: &str) -> Result<(), String>;

    // 🆕 DATABASE CLEANUP AND RETENTION METHODS
    /// Clean up old execution history based on retention policy and database size
    async fn cleanup_execution_history(
        &self,
        retention_days: u32,
        max_size_mb: u32,
    ) -> Result<usize, String>;

    /// Get database size statistics
    async fn get_database_stats(&self) -> Result<DatabaseStats, String>;

    /// Vacuum database to reclaim space
    async fn vacuum_database(&self) -> Result<(), String>;

    // 🆕 TRIGGER LIFECYCLE STATE METHODS
    /// Save persistent state for a trigger node (survives restarts)
    async fn save_trigger_state(
        &self,
        flow_id: &str,
        node_id: &str,
        key: &str,
        value: Value,
    ) -> Result<(), String>;

    /// Load persistent state for a trigger node
    async fn load_trigger_state(
        &self,
        flow_id: &str,
        node_id: &str,
        key: &str,
    ) -> Result<Option<Value>, String>;

    /// Delete trigger state (used during deactivation cleanup)
    async fn delete_trigger_state(&self, flow_id: &str, node_id: &str) -> Result<(), String>;

    /// Atomically claim a trigger idempotency key. Returns false when the key
    /// was already accepted by a previous process/run.
    async fn claim_trigger_dedup(
        &self,
        dedup_key: &str,
        event_id: Option<&str>,
        occurred_at: DateTime<Utc>,
    ) -> Result<bool, String>;

    /// Replace every routing entry for one active flow in a transaction.
    async fn replace_active_triggers(
        &self,
        flow_id: &str,
        triggers: &[ActiveTriggerDefinition],
    ) -> Result<(), String>;

    async fn remove_active_triggers(&self, flow_id: &str) -> Result<(), String>;

    async fn clear_active_triggers(&self) -> Result<(), String>;

    async fn list_active_triggers(&self) -> Result<Vec<ActiveTriggerDefinition>, String>;

    /// Persist canonical source events for audit and replay tooling.
    async fn store_event_envelope(
        &self,
        envelope: &crate::events::EventEnvelope,
    ) -> Result<bool, String>;

    async fn load_event_envelope(
        &self,
        event_id: &str,
    ) -> Result<Option<crate::events::EventEnvelope>, String>;

    async fn trace_workflow_event(
        &self,
        event_id: &str,
    ) -> Result<Option<crate::events::WorkflowRuntimeTrace>, String> {
        Err(format!(
            "Workflow event tracing is unavailable for storage backend ({event_id})"
        ))
    }

    async fn list_event_envelopes(
        &self,
        event_type: Option<&str>,
        since: Option<DateTime<Utc>>,
        limit: u32,
    ) -> Result<Vec<crate::events::EventEnvelope>, String>;

    async fn save_overlay_command_delivery(
        &self,
        delivery: &crate::events::OverlayCommandDelivery,
    ) -> Result<(), String>;
    async fn resolve_overlay_command_delivery(
        &self,
        acknowledgement: &crate::events::OverlayCommandAcknowledgement,
    ) -> Result<bool, String>;
    async fn expire_overlay_command_delivery(&self, command_id: &str) -> Result<bool, String>;
    /// Expire every pending command owned by a cancelled run and return the
    /// command IDs whose in-process receipt waiters must be dropped.
    async fn expire_pending_overlay_command_deliveries_for_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<String>, String>;
    async fn list_pending_overlay_command_deliveries(
        &self,
    ) -> Result<Vec<crate::events::OverlayCommandDelivery>, String>;
    async fn list_overlay_command_deliveries(
        &self,
        run_id: &str,
    ) -> Result<Vec<crate::events::OverlayCommandDelivery>, String>;
    async fn load_overlay_command_delivery(
        &self,
        command_id: &str,
    ) -> Result<Option<crate::events::OverlayCommandDelivery>, String>;
    async fn claim_side_effect_outbox(
        &self,
        entry: &crate::events::SideEffectOutboxEntry,
    ) -> Result<bool, String>;
    async fn replace_side_effect_outbox_attempt(
        &self,
        entry: &crate::events::SideEffectOutboxEntry,
    ) -> Result<(), String>;
    async fn complete_side_effect_outbox(
        &self,
        key: &str,
        status: crate::events::SideEffectOutboxStatus,
        error: Option<String>,
    ) -> Result<(), String>;
    async fn list_side_effect_outbox(
        &self,
    ) -> Result<Vec<crate::events::SideEffectOutboxEntry>, String>;
    async fn load_side_effect_outbox(
        &self,
        key: &str,
    ) -> Result<Option<crate::events::SideEffectOutboxEntry>, String>;
    async fn workflow_runtime_metrics(
        &self,
    ) -> Result<crate::events::WorkflowRuntimeMetrics, String>;
    async fn save_workflow_dead_letter(
        &self,
        record: &crate::events::WorkflowDeadLetter,
    ) -> Result<(), String>;
    async fn load_workflow_dead_letter(
        &self,
        id: &str,
    ) -> Result<Option<crate::events::WorkflowDeadLetter>, String>;
    async fn list_workflow_dead_letters(
        &self,
    ) -> Result<Vec<crate::events::WorkflowDeadLetter>, String>;
    async fn save_workflow_wake(&self, wake: &WorkflowWake) -> Result<(), String>;
    async fn load_workflow_wake(&self, wake_id: &str) -> Result<Option<WorkflowWake>, String>;
    async fn list_open_workflow_wakes(
        &self,
        run_id: Option<&str>,
    ) -> Result<Vec<WorkflowWake>, String>;
    async fn set_workflow_wake_status(
        &self,
        wake_id: &str,
        status: WorkflowWakeStatus,
    ) -> Result<bool, String>;
    async fn resolve_workflow_approval(
        &self,
        wake_id: &str,
        resolution: &ApprovalResolution,
    ) -> Result<ApprovalResolveOutcome, String>;
    async fn acquire_resource_lease(
        &self,
        lease: &WorkflowResourceLease,
        now: DateTime<Utc>,
    ) -> Result<bool, String>;
    async fn renew_resource_lease(
        &self,
        resource_key: &str,
        lease_token: &str,
        expires_at: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Result<bool, String>;
    async fn release_resource_lease(
        &self,
        resource_key: &str,
        lease_token: &str,
    ) -> Result<bool, String>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowSummary {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub status: Option<FlowStatus>,
    pub tags: Vec<String>,
}

// ============================================================================
// HYBRID STORAGE IMPLEMENTATION
// Uses main Diesel DB for flow definitions, separate SQLite for execution history
// ============================================================================

// ============================================================================
// DEFAULT IMPLEMENTATIONS
// ============================================================================

impl Default for FlowMetadata {
    fn default() -> Self {
        Self {
            canvas_settings: CanvasSettings::default(),
            variables: HashMap::new(),
            permissions: FlowPermissions::default(),
            execution_settings: FlowExecutionSettings::default(),
        }
    }
}

impl Default for CanvasSettings {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
            grid_enabled: true,
            snap_to_grid: true,
        }
    }
}

impl Default for FlowPermissions {
    fn default() -> Self {
        Self {
            public: false,
            collaborators: vec![],
            execution_roles: vec![],
        }
    }
}

impl Default for FlowExecutionSettings {
    fn default() -> Self {
        Self {
            max_execution_time_ms: Some(300_000), // 5 minutes
            max_concurrent_nodes: 10,
            auto_save_interval_ms: 30_000, // 30 seconds
        }
    }
}

impl Default for NodeUIData {
    fn default() -> Self {
        Self {
            color: None,
            icon: None,
            collapsed: false,
            selected: false,
            z_index: 0,
            custom_properties: HashMap::new(),
        }
    }
}

impl Default for NodeExecutionConfig {
    fn default() -> Self {
        Self {
            timeout_ms: Some(60_000), // 1 minute
            retry_count: 0,
            retry_delay_ms: 1000,
            skip_on_error: false,
            parallel_execution: false,
        }
    }
}

// 🆕 Add database statistics struct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseStats {
    pub total_size_bytes: u64,
    pub flow_count: u32,
    pub execution_history_count: u32,
    pub execution_states_count: u32,
    pub oldest_execution: Option<DateTime<Utc>>,
    pub newest_execution: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod executor_tests {
    use super::*;
    use crate::services::execution::{SequenceMode, UniversalExecutionEngine};
    use diesel::r2d2::{ConnectionManager, Pool};
    use diesel::{r2d2, SqliteConnection};
    use diesel_migrations::MigrationHarness;

    #[test]
    fn run_inputs_remain_immutable_after_checkpoint_recovery() {
        let mut variables = RuntimeVariables::new();
        variables.set_run_inputs(&HashMap::from([("text".into(), json!("original"))]));
        variables.set("scratch", json!(1), VariableScope::Local);
        let mut recovered: RuntimeVariables =
            serde_json::from_value(serde_json::to_value(&variables).unwrap()).unwrap();
        recovered.set("inputs", json!({"text":"modified"}), VariableScope::Local);
        assert!(recovered.delete("inputs").is_none());
        assert_eq!(recovered.get("inputs"), Some(&json!({"text":"original"})));
        recovered.set("scratch", json!(2), VariableScope::Local);
        assert_eq!(recovered.get("scratch"), Some(&json!(2)));
        recovered.clear_local();
        assert!(!recovered.is_read_only("inputs"));
        let legacy: RuntimeVariables =
            serde_json::from_value(json!({"local":{"inputs":"legacy"}})).unwrap();
        assert!(!legacy.is_read_only("inputs"));
        assert_eq!(legacy.get("inputs"), Some(&json!("legacy")));
    }

    fn test_node(id: &str) -> FlowNodeDefinition {
        FlowNodeDefinition {
            id: id.to_string(),
            node_type: "test".to_string(),
            name: id.to_string(),
            inputs: HashMap::new(),
            outputs: Vec::new(),
            ai_behavior: None,
            ai_model: None,
            prompt_template: None,
            tool_name: None,
            position: (0.0, 0.0),
            ui_data: NodeUIData::default(),
            execution_config: NodeExecutionConfig::default(),
        }
    }

    fn execution_edge(from: &str, to: &str) -> FlowConnection {
        FlowConnection::control(from, to)
    }

    fn test_flow(
        nodes: Vec<FlowNodeDefinition>,
        connections: Vec<FlowConnection>,
    ) -> FlowDefinition {
        FlowDefinition {
            id: "flow".to_string(),
            name: "typed edges".to_string(),
            version: "1".to_string(),
            description: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            created_by: None,
            tags: Vec::new(),
            start_node_id: nodes
                .first()
                .map(|node| node.id.clone())
                .unwrap_or_default(),
            nodes,
            connections,
            metadata: FlowMetadata::default(),
            is_active: false,
        }
    }

    fn test_execution_state(flow_definition: FlowDefinition) -> FlowExecutionState {
        FlowExecutionState {
            run_id: "run".to_string(),
            flow_id: flow_definition.id.clone(),
            flow_definition,
            status: FlowStatus::RecoveryRequired,
            current_node: None,
            completed_nodes: HashSet::new(),
            failed_nodes: HashSet::new(),
            node_results: HashMap::new(),
            execution_log: Vec::new(),
            started_at: Utc::now(),
            completed_at: None,
            recovery_detected_at: Some(Utc::now()),
            pending_nodes: Vec::new(),
            sorted_node_results: Vec::new(),
            runtime_variables: RuntimeVariables::default(),
            approved_effect_bindings: HashMap::new(),
            lineage_tracker: None,
            lineage: None,
        }
    }

    #[test]
    fn wake_policy_accepts_only_its_pinned_graph_snapshot() {
        let original = test_flow(vec![test_node("start")], Vec::new());
        let state = test_execution_state(original.clone());
        let now = Utc::now();
        let mut wake = WorkflowWake {
            id: "wake".to_string(),
            run_id: state.run_id.clone(),
            node_id: "start".to_string(),
            kind: WorkflowWakeKind::Timer,
            status: WorkflowWakeStatus::Waiting,
            due_at: Some(now),
            attempt: 0,
            payload: Value::Null,
            resume_policy: WorkflowResumePolicy::pinned(flow_revision(&original).unwrap()),
            created_at: now,
            updated_at: now,
        };
        assert!(FlowExecutor::validate_wake_resume_policy(&wake, &state).is_ok());

        let mut edited = original;
        edited.nodes[0].name = "edited after wait".to_string();
        wake.resume_policy = WorkflowResumePolicy::pinned(flow_revision(&edited).unwrap());
        assert!(FlowExecutor::validate_wake_resume_policy(&wake, &state).is_err());

        wake.resume_policy = WorkflowResumePolicy::default();
        assert!(FlowExecutor::validate_wake_resume_policy(&wake, &state).is_err());
    }

    #[test]
    fn approval_binding_rejects_a_changed_resolved_effect() {
        let original = UniversalAction::SystemControl {
            command: "system.volume.set".to_string(),
            parameters: Some(HashMap::from([("volume".to_string(), json!(20))])),
        };
        let changed = UniversalAction::SystemControl {
            command: "system.volume.set".to_string(),
            parameters: Some(HashMap::from([("volume".to_string(), json!(80))])),
        };
        let binding = FlowExecutor::resolved_action_hash(&original).unwrap();

        assert!(
            FlowExecutor::validate_resolved_action_binding("volume", &original, &binding).is_ok()
        );
        assert!(
            FlowExecutor::validate_resolved_action_binding("volume", &changed, &binding).is_err()
        );
    }

    #[test]
    fn approval_binding_hash_is_independent_of_map_insertion_order() {
        let mut first = HashMap::new();
        first.insert("alpha".to_string(), json!(1));
        first.insert("beta".to_string(), json!({ "nested": true }));
        let mut second = HashMap::new();
        second.insert("beta".to_string(), json!({ "nested": true }));
        second.insert("alpha".to_string(), json!(1));

        let first = UniversalAction::SystemControl {
            command: "test".to_string(),
            parameters: Some(first),
        };
        let second = UniversalAction::SystemControl {
            command: "test".to_string(),
            parameters: Some(second),
        };
        assert_eq!(
            FlowExecutor::resolved_action_hash(&first).unwrap(),
            FlowExecutor::resolved_action_hash(&second).unwrap()
        );
    }

    #[tokio::test]
    async fn cancelling_a_run_closes_approval_and_rejects_late_decisions() {
        let directory = tempfile::tempdir().expect("approval cancellation directory");
        let manager = ConnectionManager::<SqliteConnection>::new(":memory:");
        let pool = Pool::builder()
            .max_size(1)
            .build(manager)
            .expect("flow definition pool");
        let storage = Arc::new(
            HybridFlowStorage::new(
                pool,
                directory
                    .path()
                    .join("workflow-runtime.db")
                    .to_str()
                    .unwrap(),
            )
            .expect("workflow runtime storage"),
        );
        let platform = Arc::new(crate::core::PlatformContext::new_with_event_bus_size(
            Arc::new(crate::core::NoopStorageProvider),
            Arc::new(crate::core::NoopHttpClient),
            Arc::new(crate::core::NoopSystemInterface),
            None,
            8,
        ));
        let executor = FlowExecutor::new_with_storage(
            Arc::new(FlowRegistryState::new()),
            Arc::new(ChatState::new()),
            platform,
            storage.clone(),
        );
        let mut approval = test_node("approval");
        approval.node_type = "approval".to_string();
        let flow = test_flow(vec![approval.clone()], Vec::new());
        let mut state = test_execution_state(flow.clone());
        state.status = FlowStatus::Running;
        state.current_node = Some(approval.id.clone());
        state.node_results.insert(
            approval.id.clone(),
            NodeResult {
                node_id: approval.id.clone(),
                node_name: approval.name.clone(),
                node_type: approval.node_type.clone(),
                status: NodeStatus::Running,
                inputs: HashMap::new(),
                outputs: HashMap::new(),
                error: None,
                execution_time_ms: 0,
                retry_count: 0,
            },
        );
        let now = Utc::now();
        let wake = WorkflowWake {
            id: "approval-cancel".to_string(),
            run_id: state.run_id.clone(),
            node_id: approval.id,
            kind: WorkflowWakeKind::Approval,
            status: WorkflowWakeStatus::Waiting,
            due_at: None,
            attempt: 0,
            payload: serde_json::to_value(ApprovalRequestDetails {
                title: "Approve".to_string(),
                message: "Approve effect".to_string(),
                severity: "normal".to_string(),
                requested_channels: vec!["desktop".to_string()],
                request_snapshot: Value::Null,
                approved_effects: Vec::new(),
                decision_policy: ApprovalDecisionPolicy::default(),
            })
            .unwrap(),
            resume_policy: WorkflowResumePolicy::pinned(flow_revision(&flow).unwrap()),
            created_at: now,
            updated_at: now,
        };
        executor
            .active_flows
            .lock()
            .await
            .insert(state.run_id.clone(), state.clone());
        executor
            .run_flow_ids
            .write()
            .await
            .insert(state.run_id.clone(), state.flow_id.clone());
        storage.save_execution_state(&state).await.unwrap();
        storage.save_workflow_wake(&wake).await.unwrap();

        executor.cancel_flow(&state.run_id).await.unwrap();
        let persisted = storage.load_workflow_wake(&wake.id).await.unwrap().unwrap();
        assert_eq!(persisted.status, WorkflowWakeStatus::Cancelled);
        assert_eq!(persisted.payload["resolution"]["decision"], "cancelled");
        assert!(executor
            .resolve_approval_wake(
                &wake.id,
                ApprovalDecisionRequest {
                    decision: ApprovalDecision::Approved,
                    actor: ApprovalActor {
                        id: "late-user".to_string(),
                        display_name: None,
                        channel: "desktop".to_string(),
                    },
                    comment: None,
                },
            )
            .await
            .is_err());
    }

    #[test]
    fn revision_edit_accepts_valid_graph_and_reports_actual_paths() {
        let current = test_flow(vec![test_node("start")], Vec::new());
        let revision = flow_revision(&current).unwrap();
        let mut proposed = current.clone();
        proposed.name = "Updated draft".to_string();
        proposed.nodes[0]
            .inputs
            .insert("message".to_string(), json!("hello"));

        let (prepared, before_revision, changed_paths) =
            prepare_inactive_flow_edit("flow", &revision, &current, proposed).unwrap();

        assert_eq!(before_revision, revision);
        assert_eq!(prepared.name, "Updated draft");
        assert!(changed_paths.contains(&"/name".to_string()));
        assert!(changed_paths.contains(&"/nodes/0/inputs/message".to_string()));
        assert!(!changed_paths.contains(&"/updatedAt".to_string()));
    }

    #[test]
    fn revision_edit_rejects_stale_invalid_and_authority_changes() {
        let current = test_flow(vec![test_node("start")], Vec::new());
        let revision = flow_revision(&current).unwrap();

        let stale =
            prepare_inactive_flow_edit("flow", "stale", &current, current.clone()).unwrap_err();
        assert!(stale.contains("Stale flow revision"));

        let mut invalid = current.clone();
        invalid.start_node_id = "missing".to_string();
        assert!(
            prepare_inactive_flow_edit("flow", &revision, &current, invalid)
                .unwrap_err()
                .contains("does not exist")
        );

        let mut permissions = current.clone();
        permissions.metadata.permissions.public = true;
        assert!(
            prepare_inactive_flow_edit("flow", &revision, &current, permissions)
                .unwrap_err()
                .contains("permissions")
        );

        let mut active = current.clone();
        active.is_active = true;
        assert!(
            prepare_inactive_flow_edit("flow", &revision, &current, active)
                .unwrap_err()
                .contains("inactive flows")
        );
    }

    #[test]
    fn typed_edges_serialize_with_the_frontend_contract() {
        assert_eq!(
            serde_json::to_value(FlowConnection::control_route(
                "approval", "action", "approved"
            ))
            .unwrap(),
            serde_json::json!({
                "kind": "control",
                "fromNode": "approval",
                "toNode": "action",
                "route": "approved"
            })
        );
        assert_eq!(
            serde_json::to_value(FlowConnection::data(
                "source",
                "display-text",
                "target",
                "body"
            ))
            .unwrap(),
            serde_json::json!({
                "kind": "data",
                "fromNode": "source",
                "fromOutput": "display-text",
                "toNode": "target",
                "toInput": "body"
            })
        );
    }

    #[test]
    fn graph_validation_checks_semantic_routes_and_data_ports() {
        let mut source = test_node("source");
        source.outputs = vec![
            FlowNodeOutput {
                id: "approved".to_string(),
                name: "Approved".to_string(),
                output_type: "flow".to_string(),
                description: None,
            },
            FlowNodeOutput {
                id: "message".to_string(),
                name: "Message".to_string(),
                output_type: "string".to_string(),
                description: None,
            },
        ];
        let mut target = test_node("target");
        target.inputs.insert("body".to_string(), Value::Null);
        let flow = test_flow(
            vec![source, target],
            vec![
                FlowConnection::control_route("source", "target", "approved"),
                FlowConnection::data("source", "message", "target", "body"),
            ],
        );

        assert_eq!(flow.validate_graph(), Ok(()));
    }

    #[test]
    fn graph_validation_rejects_control_cycles() {
        let flow = test_flow(
            vec![test_node("one"), test_node("two")],
            vec![execution_edge("one", "two"), execution_edge("two", "one")],
        );

        assert_eq!(
            flow.validate_graph(),
            Err("Control graph contains a cycle".to_string())
        );
    }

    #[test]
    fn join_waits_for_every_execution_predecessor() {
        let connections = vec![
            execution_edge("left", "join"),
            execution_edge("right", "join"),
        ];
        let scheduled = HashSet::new();
        let mut finished = HashSet::from(["left".to_string()]);

        assert!(FlowExecutor::ready_execution_successors(
            &connections,
            "left",
            &finished,
            &scheduled,
        )
        .is_empty());

        finished.insert("right".to_string());
        assert_eq!(
            FlowExecutor::ready_execution_successors(&connections, "right", &finished, &scheduled,),
            vec!["join".to_string()]
        );
    }

    #[test]
    fn data_edges_do_not_schedule_node_execution() {
        let connections = vec![FlowConnection::data(
            "value-source",
            "result",
            "consumer",
            "text",
        )];

        assert!(FlowExecutor::ready_execution_successors(
            &connections,
            "value-source",
            &HashSet::from(["value-source".to_string()]),
            &HashSet::new(),
        )
        .is_empty());
    }

    #[test]
    fn recovered_graph_starts_after_durably_completed_nodes() {
        let flow = FlowDefinition {
            id: "flow".to_string(),
            name: "recovery".to_string(),
            version: "1".to_string(),
            description: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            created_by: None,
            tags: Vec::new(),
            nodes: vec![
                test_node("trigger"),
                test_node("side-effect"),
                test_node("after"),
            ],
            connections: vec![
                execution_edge("trigger", "side-effect"),
                execution_edge("side-effect", "after"),
            ],
            start_node_id: "trigger".to_string(),
            metadata: FlowMetadata::default(),
            is_active: true,
        };
        let reachable = HashSet::from([
            "trigger".to_string(),
            "side-effect".to_string(),
            "after".to_string(),
        ]);
        let finished = HashSet::from(["trigger".to_string(), "side-effect".to_string()]);
        let scheduled = finished.clone();

        assert_eq!(
            FlowExecutor::ready_nodes_at_frontier(&flow, &reachable, &finished, &scheduled,),
            vec!["after".to_string()]
        );
    }

    #[test]
    fn migrating_default_output_alias_becomes_unconditional_control_edge() {
        let edge: FlowConnection = serde_json::from_value(json!({
            "fromNode": "trigger",
            "fromPort": "flow-output-output",
            "toNode": "after",
            "toPort": "flow-input"
        }))
        .unwrap();
        assert_eq!(edge, FlowConnection::control("trigger", "after"));
        let flow = FlowDefinition {
            id: "flow".to_string(),
            name: "default output".to_string(),
            version: "1".to_string(),
            description: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            created_by: None,
            tags: Vec::new(),
            nodes: vec![test_node("trigger"), test_node("after")],
            connections: vec![edge],
            start_node_id: "trigger".to_string(),
            metadata: FlowMetadata::default(),
            is_active: true,
        };
        let completed = NodeResult {
            node_id: "trigger".to_string(),
            node_name: "trigger".to_string(),
            node_type: "test".to_string(),
            status: NodeStatus::Completed,
            inputs: HashMap::new(),
            outputs: HashMap::new(),
            error: None,
            execution_time_ms: 0,
            retry_count: 0,
        };

        let reachable = FlowExecutor::reachable_for_resolved_routes(
            &flow,
            &HashMap::from([("trigger".to_string(), completed)]),
        );
        assert!(reachable.contains("after"));
    }

    #[test]
    fn zero_approval_timeout_means_wait_indefinitely() {
        assert_eq!(
            FlowExecutor::approval_timeout_ms(&HashMap::from([(
                "timeoutMs".to_string(),
                json!(0),
            )])),
            None
        );
        assert_eq!(
            FlowExecutor::approval_timeout_ms(&HashMap::from([(
                "timeoutMs".to_string(),
                json!(2500),
            )])),
            Some(2500)
        );
    }

    #[test]
    fn named_control_routes_schedule_only_the_selected_branch() {
        let approved_edge = FlowConnection::control_route("approval", "allow-action", "approved");
        let denied_edge = FlowConnection::control_route("approval", "deny-action", "denied");
        let flow = FlowDefinition {
            id: "flow".to_string(),
            name: "approval routing".to_string(),
            version: "1".to_string(),
            description: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            created_by: None,
            tags: Vec::new(),
            nodes: vec![
                test_node("trigger"),
                test_node("approval"),
                test_node("allow-action"),
                test_node("deny-action"),
            ],
            connections: vec![
                execution_edge("trigger", "approval"),
                approved_edge,
                denied_edge,
            ],
            start_node_id: "trigger".to_string(),
            metadata: FlowMetadata::default(),
            is_active: true,
        };
        let completed = |node_id: &str, outputs: HashMap<String, Value>| NodeResult {
            node_id: node_id.to_string(),
            node_name: node_id.to_string(),
            node_type: "test".to_string(),
            status: NodeStatus::Completed,
            inputs: HashMap::new(),
            outputs,
            error: None,
            execution_time_ms: 0,
            retry_count: 0,
        };
        let node_results = HashMap::from([
            ("trigger".to_string(), completed("trigger", HashMap::new())),
            (
                "approval".to_string(),
                completed(
                    "approval",
                    HashMap::from([("selectedRoute".to_string(), json!("denied"))]),
                ),
            ),
        ]);
        let finished = HashSet::from(["trigger".to_string(), "approval".to_string()]);
        let reachable = FlowExecutor::reachable_for_resolved_routes(&flow, &node_results);

        assert!(reachable.contains("deny-action"));
        assert!(!reachable.contains("allow-action"));
        assert_eq!(
            FlowExecutor::ready_nodes_for_resolved_routes(
                &flow,
                &reachable,
                &finished,
                &finished,
                &node_results,
            ),
            vec!["deny-action".to_string()]
        );
    }

    #[test]
    fn error_route_can_coexist_with_the_default_success_output() {
        let mut source = test_node("source");
        source.outputs = vec![
            FlowNodeOutput {
                id: "output".to_string(),
                name: "Success".to_string(),
                output_type: "flow".to_string(),
                description: None,
            },
            FlowNodeOutput {
                id: "error".to_string(),
                name: "On Error".to_string(),
                output_type: "flow".to_string(),
                description: None,
            },
            FlowNodeOutput {
                id: "errorMessage".to_string(),
                name: "Error Message".to_string(),
                output_type: "string".to_string(),
                description: None,
            },
        ];
        let flow = test_flow(
            vec![source, test_node("success"), test_node("catch")],
            vec![
                FlowConnection::control("source", "success"),
                FlowConnection::control_route("source", "catch", "error"),
            ],
        );

        assert_eq!(flow.validate_graph(), Ok(()));
    }

    #[test]
    fn failed_node_selects_only_its_error_branch() {
        let flow = test_flow(
            vec![
                test_node("source"),
                test_node("success"),
                test_node("catch"),
            ],
            vec![
                FlowConnection::control("source", "success"),
                FlowConnection::control_route("source", "catch", "error"),
            ],
        );
        let failed = NodeResult {
            node_id: "source".to_string(),
            node_name: "source".to_string(),
            node_type: "test".to_string(),
            status: NodeStatus::Failed,
            inputs: HashMap::new(),
            outputs: HashMap::from([
                ("selectedRoute".to_string(), json!("error")),
                ("errorMessage".to_string(), json!("failed")),
            ]),
            error: Some("failed".to_string()),
            execution_time_ms: 0,
            retry_count: 2,
        };
        let node_results = HashMap::from([("source".to_string(), failed)]);
        let reachable = FlowExecutor::reachable_for_resolved_routes(&flow, &node_results);

        assert!(reachable.contains("catch"));
        assert!(!reachable.contains("success"));
    }

    #[test]
    fn connector_recovery_semantics_map_without_provider_branches() {
        assert_eq!(
            FlowExecutor::recovery_policy_from_semantics(
                crate::integrations::ActionRecoverySemantics::Reconcile,
            ),
            RecoveryPolicy::Reconcile
        );
        assert_eq!(
            FlowExecutor::recovery_policy_from_semantics(
                crate::integrations::ActionRecoverySemantics::FailClosed,
            ),
            RecoveryPolicy::FailClosed
        );
    }

    #[tokio::test]
    async fn rendered_overlay_node_waits_for_persisted_rendered_receipt() {
        let directory = tempfile::tempdir().expect("overlay receipt directory");
        let manager = ConnectionManager::<SqliteConnection>::new(":memory:");
        let pool = Pool::builder()
            .max_size(1)
            .build(manager)
            .expect("flow definition pool");
        let storage = Arc::new(
            HybridFlowStorage::new(
                pool,
                directory
                    .path()
                    .join("workflow-runtime.db")
                    .to_str()
                    .unwrap(),
            )
            .expect("workflow runtime storage"),
        );
        let platform = Arc::new(crate::core::PlatformContext::new_with_event_bus_size(
            Arc::new(crate::core::NoopStorageProvider),
            Arc::new(crate::core::NoopHttpClient),
            Arc::new(crate::core::NoopSystemInterface),
            None,
            8,
        ));
        let mut events = platform.event_bus.subscribe();
        let executor = FlowExecutor::new_with_storage(
            Arc::new(FlowRegistryState::new()),
            Arc::new(ChatState::new()),
            platform,
            storage.clone(),
        );
        let mut node = test_node("rendered-overlay");
        node.node_type = "overlay-command".to_string();
        let inputs = HashMap::from([
            ("requiresAck".to_string(), json!(true)),
            ("ackMode".to_string(), json!("widgetRendered")),
            ("ackTimeoutMs".to_string(), json!(2_000)),
            ("target".to_string(), json!("widget")),
            ("widgetId".to_string(), json!("placed-widget")),
            ("command".to_string(), json!("show")),
        ]);
        let execution = executor.clone();
        let mut task = tokio::spawn(async move {
            execution
                .execute_overlay_command_node_inner(
                    "run-rendered",
                    &node,
                    &inputs,
                    "invocation-rendered",
                )
                .await
        });
        let command = match tokio::time::timeout(std::time::Duration::from_secs(1), events.recv())
            .await
            .expect("overlay command emitted")
            .expect("event bus open")
        {
            crate::events::EventPayload::Overlay(command) => command,
            event => panic!("unexpected event: {event:?}"),
        };
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(20), &mut task)
                .await
                .is_err(),
            "the node must still be waiting before its declared receipt"
        );
        assert!(
            !executor
                .acknowledge_overlay_command(crate::events::OverlayCommandAcknowledgement {
                    command_id: command.command_id.clone(),
                    status: crate::events::OverlayCommandAckStatus::Accepted,
                    receipt_phase: crate::events::OverlayCommandReceiptPhase::HostAccepted,
                    instance_id: Some("browser-source".to_string()),
                    applied_revision: Some(command.delivery_revision),
                    error: None,
                })
                .await
        );
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(20), &mut task)
                .await
                .is_err(),
            "a host-only receipt must not complete rendered-mode work"
        );
        assert!(
            executor
                .acknowledge_overlay_command(crate::events::OverlayCommandAcknowledgement {
                    command_id: command.command_id.clone(),
                    status: crate::events::OverlayCommandAckStatus::Accepted,
                    receipt_phase: crate::events::OverlayCommandReceiptPhase::WidgetRendered,
                    instance_id: Some("browser-source".to_string()),
                    applied_revision: Some(command.delivery_revision),
                    error: None,
                })
                .await
        );
        let outputs = tokio::time::timeout(std::time::Duration::from_secs(1), task)
            .await
            .expect("rendered receipt wakes the node")
            .expect("overlay task joined")
            .expect("overlay node succeeded");
        assert_eq!(outputs.get("acknowledged"), Some(&json!(true)));
        assert_eq!(
            storage
                .load_overlay_command_delivery(&command.command_id)
                .await
                .unwrap()
                .unwrap()
                .status,
            crate::events::OverlayCommandDeliveryStatus::Acknowledged
        );
    }

    #[tokio::test]
    async fn persisted_flow_reopens_and_executes_the_same_canonical_graph() {
        let directory = tempfile::tempdir().expect("flow reopen directory");
        let definition_path = directory.path().join("vortideck.db");
        let runtime_path = directory.path().join("workflow-runtime.db");
        let open_storage = || {
            let manager =
                ConnectionManager::<SqliteConnection>::new(definition_path.to_string_lossy());
            let pool = Pool::builder()
                .max_size(2)
                .build(manager)
                .expect("flow definition pool");
            {
                let mut connection = pool.get().expect("flow definition connection");
                connection
                    .run_pending_migrations(crate::db::db::MIGRATIONS)
                    .expect("flow definition migrations");
            }
            Arc::new(
                HybridFlowStorage::new(pool, runtime_path.to_str().unwrap())
                    .expect("workflow runtime storage"),
            )
        };
        let platform = || {
            Arc::new(crate::core::PlatformContext::new_with_event_bus_size(
                Arc::new(crate::core::NoopStorageProvider),
                Arc::new(crate::core::NoopHttpClient),
                Arc::new(crate::core::NoopSystemInterface),
                None,
                8,
            ))
        };

        let mut delay = test_node("delay");
        delay.node_type = "delay".to_string();
        delay.inputs.insert("duration".to_string(), json!(5));
        delay.ai_model = Some("deterministic-test-model".to_string());
        delay.prompt_template = Some("template-support-reply".to_string());
        delay
            .ui_data
            .custom_properties
            .insert("futureField".to_string(), json!({ "retained": true }));
        let mut flow = test_flow(vec![delay], Vec::new());
        flow.id = "persisted-flow".to_string();
        flow.name = "Persisted deterministic flow".to_string();
        flow.version = "1.0.0".to_string();

        {
            let storage = open_storage();
            let executor = FlowExecutor::new_with_storage(
                Arc::new(FlowRegistryState::new()),
                Arc::new(ChatState::new()),
                platform(),
                storage,
            );
            executor
                .save_flow(&flow)
                .await
                .expect("save canonical flow");
        }

        let reopened_storage = open_storage();
        let registry = Arc::new(FlowRegistryState::new());
        registry
            .registry
            .lock()
            .unwrap()
            .register_provider(Box::new(
                crate::services::flow::standard_nodes::StandardNodeProvider::new(),
            ));
        let reopened_executor = FlowExecutor::new_with_storage(
            registry,
            Arc::new(ChatState::new()),
            platform(),
            reopened_storage,
        );
        let loaded = reopened_executor
            .load_flow("persisted-flow")
            .await
            .expect("reopen canonical flow");
        assert_eq!(
            serde_json::to_value(&loaded).unwrap(),
            serde_json::to_value(&flow).unwrap()
        );
        assert_eq!(
            loaded.nodes[0].prompt_template.as_deref(),
            Some("template-support-reply")
        );
        assert_eq!(
            loaded.nodes[0].ui_data.custom_properties["futureField"],
            json!({ "retained": true })
        );

        let shared_executor = Arc::new(tokio::sync::Mutex::new(Some(reopened_executor)));
        let assistant_state = ChatState::new_with_flow_executor(Some(shared_executor.clone()));
        let get_result = crate::services::mcp::commands::execute_mcp_tool_core(
            &assistant_state,
            "vortideck_get_flow".to_string(),
            json!({ "flow_id": "persisted-flow" }),
        )
        .await
        .expect("assistant reads the authoritative graph");
        assert_eq!(get_result["available"], true);
        assert_eq!(
            get_result["provenance"]["content_trust"],
            "untrusted_user_authored_data"
        );
        let revision = get_result["revision"].as_str().unwrap().to_string();
        let mut proposed = loaded;
        proposed.name = "Assistant verified edit".to_string();
        proposed.nodes[0]
            .inputs
            .insert("duration".to_string(), json!(7));
        let replace_args = json!({
            "flow_id": "persisted-flow",
            "expected_revision": revision,
            "proposed_flow": proposed,
        });
        let replace_result = crate::services::mcp::commands::execute_mcp_tool_core(
            &assistant_state,
            "vortideck_replace_inactive_flow".to_string(),
            replace_args.clone(),
        )
        .await
        .expect("assistant edit persists atomically");
        assert_eq!(replace_result["available"], true);
        assert_eq!(replace_result["provenance"]["read_back"], true);
        assert!(replace_result["receipt"]["changedPaths"]
            .as_array()
            .unwrap()
            .iter()
            .any(|path| path == "/nodes/0/inputs/duration"));
        let stale_error = crate::services::mcp::commands::execute_mcp_tool_core(
            &assistant_state,
            "vortideck_replace_inactive_flow".to_string(),
            replace_args,
        )
        .await
        .expect_err("the consumed revision must become stale");
        assert!(stale_error.contains("Stale flow revision"));
        let logs = crate::services::mcp::commands::execute_mcp_tool_core(
            &assistant_state,
            "vortideck_get_system_logs".to_string(),
            json!({}),
        )
        .await
        .expect("unavailable diagnostics are a truthful typed result");
        assert_eq!(logs["available"], false);
        assert_eq!(logs["provenance"]["reason"], "log_reader_not_configured");

        drop(assistant_state);
        drop(shared_executor);

        let final_registry = Arc::new(FlowRegistryState::new());
        final_registry
            .registry
            .lock()
            .unwrap()
            .register_provider(Box::new(
                crate::services::flow::standard_nodes::StandardNodeProvider::new(),
            ));
        let reopened_executor = FlowExecutor::new_with_storage(
            final_registry,
            Arc::new(ChatState::new()),
            platform(),
            open_storage(),
        );
        let loaded = reopened_executor
            .load_flow("persisted-flow")
            .await
            .expect("read back assistant edit after executor restart");
        assert_eq!(loaded.name, "Assistant verified edit");
        assert_eq!(loaded.nodes[0].inputs["duration"], 7);

        let run_id = reopened_executor
            .start_flow(loaded)
            .await
            .expect("execute reopened flow");
        let history = tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                if let Some(entry) = reopened_executor
                    .list_execution_history(Some("persisted-flow"), 10)
                    .await
                    .unwrap()
                    .into_iter()
                    .find(|entry| entry.run_id == run_id)
                {
                    break entry;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("reopened flow reaches terminal history");
        assert!(matches!(history.status, FlowStatus::Completed));
        assert_eq!(history.completed_nodes, 1);
        assert_eq!(history.failed_nodes, 0);
        assert_eq!(history.node_execution_details[0].outputs["duration"], 7);
    }

    #[tokio::test]
    async fn durable_applied_invocation_closes_outbox_crash_window_without_redispatch() {
        let directory = tempfile::tempdir().expect("reconciliation directory");
        let database_path = directory.path().join("vortideck.db");
        let manager = ConnectionManager::<SqliteConnection>::new(database_path.to_string_lossy());
        let pool: r2d2::Pool<ConnectionManager<SqliteConnection>> = Pool::builder()
            .max_size(2)
            .build(manager)
            .expect("main database pool");
        {
            let mut connection = pool.get().expect("main database connection");
            connection
                .run_pending_migrations(crate::db::db::MIGRATIONS)
                .expect("main database migrations");
        }
        let database = Arc::new(crate::db::Database::new(&pool));
        let storage = Arc::new(
            HybridFlowStorage::new(
                pool,
                directory
                    .path()
                    .join("workflow-runtime.db")
                    .to_str()
                    .unwrap(),
            )
            .expect("workflow runtime storage"),
        );
        let platform = Arc::new(crate::core::PlatformContext::new_with_event_bus_size(
            Arc::new(crate::core::NoopStorageProvider),
            Arc::new(crate::core::NoopHttpClient),
            Arc::new(crate::core::NoopSystemInterface),
            None,
            8,
        ));
        let engine = Arc::new(
            UniversalExecutionEngine::new(
                Arc::new(crate::core::state_manager::StateManager::new()),
            )
            .with_database(database.clone()),
        );
        let readiness = Arc::new(crate::services::execution::ActionRuntimeReadiness::new());
        readiness.mark_ready(vec![]);
        let invocation_service = Arc::new(ActionInvocationService::new(
            engine,
            database.clone(),
            readiness,
        ));
        let mut executor = FlowExecutor::new_with_storage(
            Arc::new(FlowRegistryState::new()),
            Arc::new(ChatState::new()),
            platform,
            storage.clone(),
        );
        executor.set_action_invocation_service(invocation_service);

        let now = Utc::now().to_rfc3339();
        database
            .create_action_invocation(crate::db::NewActionInvocation {
                invocation_id: "reconcile-invocation",
                parent_invocation_id: None,
                source: "flow",
                source_reference: Some("run:node"),
                workflow_run_id: Some("run"),
                node_id: Some("node"),
                resolved_action: r#"{"Sequence":{"actions":[],"execution_mode":"Sequential"}}"#,
                integration_id: None,
                action_id: None,
                capability_version: None,
                idempotency_key: Some("flow-action:run:node"),
                resource_key: None,
                dispatch_status: "succeeded",
                receipt_semantics: None,
                request_id: Some("reconcile-invocation"),
                provider_receipt_id: Some("receipt"),
                provider_outputs: Some("{}"),
                normalized_status: Some("applied"),
                error: None,
                recovery_decision: Some("retry_with_idempotency"),
                started_at: &now,
                completed_at: Some(&now),
                execution_time_ms: 1,
                created_at: &now,
                updated_at: &now,
            })
            .await
            .unwrap();
        let entry = crate::events::SideEffectOutboxEntry {
            idempotency_key: "flow-action:run:node".into(),
            invocation_id: Some("reconcile-invocation".into()),
            run_id: "run".into(),
            node_id: "node".into(),
            action: serde_json::to_value(UniversalAction::Sequence {
                actions: vec![],
                execution_mode: SequenceMode::Sequential,
            })
            .unwrap(),
            status: crate::events::SideEffectOutboxStatus::Dispatching,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            error: None,
        };
        assert!(storage.claim_side_effect_outbox(&entry).await.unwrap());

        let reconciled = executor
            .reconcile_side_effect_invocation(&entry)
            .await
            .unwrap();
        assert_eq!(
            reconciled.status,
            crate::events::SideEffectOutboxStatus::Succeeded
        );
        assert_eq!(
            storage
                .load_side_effect_outbox(&entry.idempotency_key)
                .await
                .unwrap()
                .unwrap()
                .status,
            crate::events::SideEffectOutboxStatus::Succeeded
        );
    }
}
