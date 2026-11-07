//! Durable routing metadata for workflow ingress triggers.

use serde::{Deserialize, Serialize};

/// Persisted routing metadata for one trigger in an active flow.
///
/// The compact registry keeps EventRouter independent from full flow scans and
/// is rebuilt from active flow definitions at deterministic startup.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ActiveTriggerDefinition {
    pub flow_id: String,
    pub node_id: String,
    pub node_type: String,
    pub platform: Option<String>,
    pub event_type: Option<String>,
}
