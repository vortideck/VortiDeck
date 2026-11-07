use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

/// Data lineage tracking for flow executions
/// Tracks where data came from, how it was transformed, and where it went

// ============================================================================
// LINEAGE TYPES
// ============================================================================

/// Complete lineage record for a flow execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowLineage {
    /// Unique execution ID
    pub execution_id: String,

    /// Flow ID
    pub flow_id: String,

    /// Flow name
    pub flow_name: String,

    /// When execution started
    pub started_at: DateTime<Utc>,

    /// When execution completed (if finished)
    pub completed_at: Option<DateTime<Utc>>,

    /// Overall status
    pub status: LineageStatus,

    /// Lineage for each node execution
    pub node_lineages: Vec<NodeLineage>,

    /// Data transformations tracked
    pub transformations: Vec<DataTransformation>,

    /// Input data to the flow
    pub flow_inputs: HashMap<String, Value>,

    /// Final output data from the flow
    pub flow_outputs: Option<HashMap<String, Value>>,

    /// Metadata about execution environment
    pub metadata: LineageMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum LineageStatus {
    Running,
    Completed,
    Failed,
    PartiallyCompleted,
    Cancelled,
}

/// Lineage for a single node execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeLineage {
    /// Unique ID for this node execution
    pub node_execution_id: String,

    /// Node ID from flow definition
    pub node_id: String,

    /// Node type (e.g., "http-request", "ai-chat")
    pub node_type: String,

    /// Node name
    pub node_name: String,

    /// When node started
    pub started_at: DateTime<Utc>,

    /// When node completed
    pub completed_at: Option<DateTime<Utc>>,

    /// Execution duration in milliseconds
    pub duration_ms: Option<u64>,

    /// Node execution status
    pub status: NodeLineageStatus,

    /// Input data to this node
    pub inputs: HashMap<String, DataLineageValue>,

    /// Output data from this node
    pub outputs: HashMap<String, DataLineageValue>,

    /// Error information if failed
    pub error: Option<LineageError>,

    /// Retry information
    pub retry_info: Option<RetryInfo>,

    /// Parent node executions (what fed data into this node)
    pub parent_nodes: Vec<String>,

    /// Child node executions (what this node fed data into)
    pub child_nodes: Vec<String>,

    /// Additional metadata
    pub metadata: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum NodeLineageStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
    Retrying,
    TimedOut,
}

/// A value with its lineage information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataLineageValue {
    /// The actual value
    pub value: Value,

    /// Type of the value
    pub value_type: String,

    /// Where this value came from
    pub source: DataSource,

    /// Transformations applied to get this value
    pub transformations: Vec<String>,

    /// Hash of the value for change detection
    pub value_hash: Option<String>,

    /// Size in bytes (for large values)
    pub size_bytes: Option<usize>,
}

/// Where a piece of data originated
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum DataSource {
    /// Input to the flow
    FlowInput { input_name: String },

    /// Output from another node
    NodeOutput {
        node_id: String,
        node_name: String,
        output_port: String,
    },

    /// Constant value
    Constant {
        defined_at: String, // Where in the flow definition
    },

    /// Variable from flow state
    Variable {
        variable_name: String,
        scope: String,
    },

    /// External source (API, database, file, etc.)
    External {
        source_type: String,
        source_uri: String,
    },

    /// Computed/transformed value
    Computed {
        computation: String,
        inputs: Vec<String>,
    },
}

/// A data transformation operation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataTransformation {
    /// Unique ID for this transformation
    pub transformation_id: String,

    /// Which node performed this transformation
    pub node_id: String,

    /// Type of transformation
    pub transformation_type: TransformationType,

    /// Description of what changed
    pub description: String,

    /// Input value(s)
    pub inputs: Vec<TransformationValue>,

    /// Output value
    pub output: TransformationValue,

    /// When transformation occurred
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TransformationType {
    /// Mapping: Transform structure
    Map,

    /// Filtering: Remove/select data
    Filter,

    /// Aggregation: Combine multiple values
    Aggregate,

    /// Enrichment: Add additional data
    Enrich,

    /// Validation: Check and potentially modify
    Validate,

    /// Conversion: Change type/format
    Convert,

    /// Extraction: Pull out specific fields
    Extract,

    /// Merging: Combine data sources
    Merge,

    /// Custom transformation
    Custom { operation: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformationValue {
    /// Sample of the value (truncated if large)
    pub sample: Value,

    /// Full value hash
    pub hash: String,

    /// Type of the value
    pub value_type: String,

    /// Size in bytes
    pub size_bytes: usize,
}

/// Error information in lineage
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LineageError {
    /// Error message
    pub message: String,

    /// Error code/type
    pub code: Option<String>,

    /// Stack trace
    pub stack: Option<String>,

    /// When error occurred
    pub timestamp: DateTime<Utc>,

    /// Whether error was recovered
    pub recovered: bool,

    /// Recovery mechanism used
    pub recovery_method: Option<String>,
}

/// Retry information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetryInfo {
    /// Number of attempts made
    pub attempt_count: u32,

    /// Maximum attempts allowed
    pub max_attempts: u32,

    /// Delays between attempts (ms)
    pub delays: Vec<u64>,

    /// Errors from each attempt
    pub attempt_errors: Vec<String>,

    /// Whether retry succeeded
    pub succeeded: bool,
}

/// Metadata about execution environment
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LineageMetadata {
    /// User who triggered execution
    pub triggered_by: Option<String>,

    /// Trigger type (manual, scheduled, webhook, etc.)
    pub trigger_type: String,

    /// Execution environment
    pub environment: String,

    /// Version of flow definition
    pub flow_version: String,

    /// Platform/runtime info
    pub platform: String,

    /// Tags for categorization
    pub tags: Vec<String>,

    /// Custom metadata
    pub custom: HashMap<String, Value>,
}

// ============================================================================
// LINEAGE TRACKER
// ============================================================================

/// Tracks lineage during flow execution
#[derive(Debug, Clone)]
pub struct LineageTracker {
    current_lineage: FlowLineage,
    node_start_times: HashMap<String, DateTime<Utc>>,
}

impl LineageTracker {
    /// Create a new lineage tracker for a flow execution
    pub fn new(
        flow_id: String,
        flow_name: String,
        flow_inputs: HashMap<String, Value>,
        trigger_type: String,
        triggered_by: Option<String>,
    ) -> Self {
        let execution_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let lineage = FlowLineage {
            execution_id: execution_id.clone(),
            flow_id,
            flow_name,
            started_at: now,
            completed_at: None,
            status: LineageStatus::Running,
            node_lineages: Vec::new(),
            transformations: Vec::new(),
            flow_inputs,
            flow_outputs: None,
            metadata: LineageMetadata {
                triggered_by,
                trigger_type,
                environment: std::env::var("ENV").unwrap_or_else(|_| "development".to_string()),
                flow_version: "1.0.0".to_string(), // TODO: Get from flow definition
                platform: format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
                tags: Vec::new(),
                custom: HashMap::new(),
            },
        };

        Self {
            current_lineage: lineage,
            node_start_times: HashMap::new(),
        }
    }

    /// Record the start of a node execution
    pub fn start_node(
        &mut self,
        node_id: String,
        node_type: String,
        node_name: String,
        inputs: HashMap<String, Value>,
    ) -> String {
        let node_execution_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        self.node_start_times.insert(node_execution_id.clone(), now);

        // Convert inputs to lineage values
        let input_lineages: HashMap<String, DataLineageValue> = inputs
            .into_iter()
            .map(|(k, v)| {
                let lineage_value = DataLineageValue {
                    value_type: type_name_of(&v),
                    value: v,
                    source: DataSource::FlowInput {
                        input_name: k.clone(),
                    },
                    transformations: Vec::new(),
                    value_hash: None,
                    size_bytes: None,
                };
                (k, lineage_value)
            })
            .collect();

        let node_lineage = NodeLineage {
            node_execution_id: node_execution_id.clone(),
            node_id,
            node_type,
            node_name,
            started_at: now,
            completed_at: None,
            duration_ms: None,
            status: NodeLineageStatus::Running,
            inputs: input_lineages,
            outputs: HashMap::new(),
            error: None,
            retry_info: None,
            parent_nodes: Vec::new(),
            child_nodes: Vec::new(),
            metadata: HashMap::new(),
        };

        self.current_lineage.node_lineages.push(node_lineage);
        node_execution_id
    }

    /// Record the completion of a node execution
    pub fn complete_node(&mut self, node_execution_id: &str, outputs: HashMap<String, Value>) {
        let now = Utc::now();

        if let Some(node) = self
            .current_lineage
            .node_lineages
            .iter_mut()
            .find(|n| n.node_execution_id == node_execution_id)
        {
            node.completed_at = Some(now);
            node.status = NodeLineageStatus::Completed;

            // Calculate duration
            if let Some(start_time) = self.node_start_times.get(node_execution_id) {
                let duration = now.signed_duration_since(*start_time);
                node.duration_ms = Some(duration.num_milliseconds() as u64);
            }

            // Convert outputs to lineage values
            node.outputs = outputs
                .into_iter()
                .map(|(k, v)| {
                    let lineage_value = DataLineageValue {
                        value_type: type_name_of(&v),
                        value: v,
                        source: DataSource::NodeOutput {
                            node_id: node.node_id.clone(),
                            node_name: node.node_name.clone(),
                            output_port: k.clone(),
                        },
                        transformations: Vec::new(),
                        value_hash: None,
                        size_bytes: None,
                    };
                    (k, lineage_value)
                })
                .collect();
        }
    }

    /// Record a node failure
    pub fn fail_node(
        &mut self,
        node_execution_id: &str,
        error_message: String,
        error_code: Option<String>,
    ) {
        let now = Utc::now();

        if let Some(node) = self
            .current_lineage
            .node_lineages
            .iter_mut()
            .find(|n| n.node_execution_id == node_execution_id)
        {
            node.completed_at = Some(now);
            node.status = NodeLineageStatus::Failed;

            if let Some(start_time) = self.node_start_times.get(node_execution_id) {
                let duration = now.signed_duration_since(*start_time);
                node.duration_ms = Some(duration.num_milliseconds() as u64);
            }

            node.error = Some(LineageError {
                message: error_message,
                code: error_code,
                stack: None,
                timestamp: now,
                recovered: false,
                recovery_method: None,
            });
        }
    }

    /// Record a data transformation
    pub fn record_transformation(
        &mut self,
        node_id: String,
        transformation_type: TransformationType,
        description: String,
        inputs: Vec<Value>,
        output: Value,
    ) {
        let transformation = DataTransformation {
            transformation_id: Uuid::new_v4().to_string(),
            node_id,
            transformation_type,
            description,
            inputs: inputs
                .into_iter()
                .map(|v| TransformationValue {
                    sample: v.clone(),
                    hash: format!("{:x}", md5::compute(v.to_string())),
                    value_type: type_name_of(&v),
                    size_bytes: v.to_string().len(),
                })
                .collect(),
            output: TransformationValue {
                sample: output.clone(),
                hash: format!("{:x}", md5::compute(output.to_string())),
                value_type: type_name_of(&output),
                size_bytes: output.to_string().len(),
            },
            timestamp: Utc::now(),
        };

        self.current_lineage.transformations.push(transformation);
    }

    /// Complete the flow execution
    pub fn complete_flow(&mut self, outputs: HashMap<String, Value>) {
        self.current_lineage.completed_at = Some(Utc::now());
        self.current_lineage.status = LineageStatus::Completed;
        self.current_lineage.flow_outputs = Some(outputs);
    }

    /// Fail the flow execution
    pub fn fail_flow(&mut self) {
        self.current_lineage.completed_at = Some(Utc::now());
        self.current_lineage.status = LineageStatus::Failed;
    }

    /// Get the current lineage record
    pub fn get_lineage(&self) -> &FlowLineage {
        &self.current_lineage
    }

    /// Export lineage as JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.current_lineage)
    }
}

/// Get the type name of a JSON value
fn type_name_of(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(_) => "boolean".to_string(),
        Value::Number(_) => "number".to_string(),
        Value::String(_) => "string".to_string(),
        Value::Array(_) => "array".to_string(),
        Value::Object(_) => "object".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lineage_tracker_creation() {
        let mut inputs = HashMap::new();
        inputs.insert("userId".to_string(), serde_json::json!("user123"));

        let tracker = LineageTracker::new(
            "flow-1".to_string(),
            "Test Flow".to_string(),
            inputs,
            "manual".to_string(),
            Some("test_user".to_string()),
        );

        assert_eq!(tracker.current_lineage.flow_id, "flow-1");
        assert_eq!(tracker.current_lineage.status, LineageStatus::Running);
    }

    #[test]
    fn test_node_lineage() {
        let mut tracker = LineageTracker::new(
            "flow-1".to_string(),
            "Test Flow".to_string(),
            HashMap::new(),
            "manual".to_string(),
            None,
        );

        let mut inputs = HashMap::new();
        inputs.insert("param1".to_string(), serde_json::json!("value1"));

        let node_id = tracker.start_node(
            "node-1".to_string(),
            "http-request".to_string(),
            "Fetch Data".to_string(),
            inputs,
        );

        let mut outputs = HashMap::new();
        outputs.insert(
            "result".to_string(),
            serde_json::json!({"status": "success"}),
        );

        tracker.complete_node(&node_id, outputs);

        let lineage = tracker.get_lineage();
        assert_eq!(lineage.node_lineages.len(), 1);
        assert_eq!(
            lineage.node_lineages[0].status,
            NodeLineageStatus::Completed
        );
    }

    #[test]
    fn test_transformation_tracking() {
        let mut tracker = LineageTracker::new(
            "flow-1".to_string(),
            "Test Flow".to_string(),
            HashMap::new(),
            "manual".to_string(),
            None,
        );

        tracker.record_transformation(
            "node-1".to_string(),
            TransformationType::Map,
            "Convert user data to profile".to_string(),
            vec![serde_json::json!({"name": "John", "age": 30})],
            serde_json::json!({"displayName": "John", "userAge": 30}),
        );

        let lineage = tracker.get_lineage();
        assert_eq!(lineage.transformations.len(), 1);
        assert_eq!(
            lineage.transformations[0].description,
            "Convert user data to profile"
        );
    }
}
