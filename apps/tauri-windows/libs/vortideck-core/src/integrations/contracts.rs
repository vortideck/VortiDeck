//! Versioned contracts shared by native and package-provided integrations.
//!
//! A contract describes what an action accepts and how the runtime may safely
//! execute/recover it. Authorization still belongs to the invocation boundary;
//! connectors never grant permissions to themselves.

use crate::plugins::bundle::PluginPermission;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContractValueType {
    Any,
    String,
    Number,
    Integer,
    Boolean,
    Object,
    Array,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityKind {
    Action,
    Query,
    Trigger,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityWidget {
    Text,
    Textarea,
    Number,
    Color,
    Toggle,
    Select,
    MultiSelect,
    Json,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityOption {
    pub label: String,
    pub value: Value,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<CapabilityParameter>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DynamicOptionSource {
    pub resolver_id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityParameter {
    pub id: String,
    pub label: String,
    pub description: String,
    pub value_type: ContractValueType,
    pub required: bool,
    pub widget: CapabilityWidget,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_value: Option<Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<CapabilityOption>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub option_source: Option<DynamicOptionSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityOutput {
    pub id: String,
    pub label: String,
    pub value_type: ContractValueType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityPresentation {
    pub name: String,
    pub description: String,
    pub category: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionRecoverySemantics {
    Reconcile,
    RetryWithIdempotency,
    ManualReview,
    FailClosed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionReceiptSemantics {
    /// The legacy connector only confirms that the local channel accepted the
    /// command. This is never sufficient proof for automatic reconciliation.
    TransportAccepted,
    /// A capable connector must return a correlated applied/failed receipt.
    Applied { timeout_ms: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ActionContract {
    pub integration_id: String,
    pub action_id: String,
    pub version: u32,
    pub required_permission: PluginPermission,
    pub timeout_ms: u64,
    pub recovery: ActionRecoverySemantics,
    pub resource_key: Option<String>,
    pub receipt: ActionReceiptSemantics,
}

/// The single provider-owned definition used by discovery, editors,
/// validation, preflight, and execution. UI schema and operational behavior
/// cannot drift because providers publish them together.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityDefinition {
    pub integration_id: String,
    pub capability_id: String,
    pub version: u32,
    pub kind: CapabilityKind,
    pub presentation: CapabilityPresentation,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<CapabilityParameter>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outputs: Vec<CapabilityOutput>,
    pub allow_additional_inputs: bool,
    pub contract: ActionContract,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionPreflightStatus {
    Ready,
    /// Provider is registered but still establishing or refreshing its connection.
    Syncing,
    Disconnected,
    MissingIntegration,
    MissingTarget,
    PermissionDenied,
    InvalidAction,
    ContractDrift,
    /// Executable, but the provider can only confirm transport acceptance.
    ReceiptDegraded,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionRepairKind {
    RetryPreflight,
    OpenIntegrationSettings,
    RebindAction,
    ReplaceReferences,
    DisableAffectedFlows,
    ReviewReceiptSemantics,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ActionPreflightIssue {
    pub code: String,
    pub message: String,
    pub recoverable: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub repairs: Vec<ActionRepairKind>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ActionPreflight {
    pub integration_id: String,
    pub action_id: String,
    pub status: ActionPreflightStatus,
    pub capability: Option<CapabilityDefinition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub issues: Vec<ActionPreflightIssue>,
}

impl ActionPreflight {
    pub fn is_ready(&self) -> bool {
        matches!(
            self.status,
            ActionPreflightStatus::Ready | ActionPreflightStatus::ReceiptDegraded
        )
    }
}

impl ActionContract {
    pub fn permission_wire_name(&self) -> String {
        format!("integration:{}:{}", self.integration_id, self.action_id)
    }
}

impl CapabilityDefinition {
    pub fn validate_parameters(&self, parameters: &HashMap<String, Value>) -> Result<(), String> {
        let mut visible = Vec::new();
        collect_visible_parameters(&self.inputs, parameters, &mut visible);
        for parameter in &visible {
            let value = parameters.get(&parameter.id);
            if parameter.required && value.is_none() {
                return Err(format!(
                    "Action '{}:{}' requires parameter '{}'",
                    self.integration_id, self.capability_id, parameter.id
                ));
            }
            let Some(value) = value else { continue };
            if !value_matches_type(value, &parameter.value_type) {
                return Err(format!(
                    "Parameter '{}' for action '{}:{}' must be {:?}",
                    parameter.id, self.integration_id, self.capability_id, parameter.value_type
                ));
            }
            if !parameter.options.is_empty()
                && !parameter
                    .options
                    .iter()
                    .any(|option| &option.value == value)
            {
                return Err(format!(
                    "Parameter '{}' for action '{}:{}' is not an allowed value",
                    parameter.id, self.integration_id, self.capability_id
                ));
            }
        }

        if !self.allow_additional_inputs {
            if let Some(unknown) = parameters
                .keys()
                .find(|id| !visible.iter().any(|parameter| &parameter.id == *id))
            {
                return Err(format!(
                    "Action '{}:{}' does not declare parameter '{}'",
                    self.integration_id, self.capability_id, unknown
                ));
            }
        }
        Ok(())
    }
}

fn collect_visible_parameters<'a>(
    declared: &'a [CapabilityParameter],
    values: &HashMap<String, Value>,
    visible: &mut Vec<&'a CapabilityParameter>,
) {
    for parameter in declared {
        visible.push(parameter);
        let Some(value) = values.get(&parameter.id) else {
            continue;
        };
        if let Some(option) = parameter
            .options
            .iter()
            .find(|option| &option.value == value)
        {
            collect_visible_parameters(&option.parameters, values, visible);
        }
    }
}

fn value_matches_type(value: &Value, value_type: &ContractValueType) -> bool {
    match value_type {
        ContractValueType::Any => true,
        ContractValueType::String => value.is_string(),
        ContractValueType::Number => value.is_number(),
        ContractValueType::Integer => value.as_i64().is_some() || value.as_u64().is_some(),
        ContractValueType::Boolean => value.is_boolean(),
        ContractValueType::Object => value.is_object(),
        ContractValueType::Array => value.is_array(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn contract_rejects_missing_wrong_and_unknown_parameters() {
        let capability = CapabilityDefinition {
            integration_id: "test".into(),
            capability_id: "set".into(),
            version: 1,
            kind: CapabilityKind::Action,
            presentation: CapabilityPresentation {
                name: "Set".into(),
                description: String::new(),
                category: "Test".into(),
                icon: None,
                aliases: vec![],
            },
            inputs: vec![CapabilityParameter {
                id: "enabled".into(),
                label: "Enabled".into(),
                description: String::new(),
                value_type: ContractValueType::Boolean,
                required: true,
                widget: CapabilityWidget::Toggle,
                default_value: None,
                options: vec![],
                option_source: None,
            }],
            outputs: vec![],
            allow_additional_inputs: false,
            contract: ActionContract {
                integration_id: "test".into(),
                action_id: "set".into(),
                version: 1,
                required_permission: PluginPermission::TriggerAction("integration:test:set".into()),
                timeout_ms: 1_000,
                recovery: ActionRecoverySemantics::Reconcile,
                resource_key: Some("test:set".into()),
                receipt: ActionReceiptSemantics::TransportAccepted,
            },
        };

        assert!(capability.validate_parameters(&HashMap::new()).is_err());
        assert!(capability
            .validate_parameters(&HashMap::from([("enabled".into(), json!("yes"))]))
            .is_err());
        assert!(capability
            .validate_parameters(&HashMap::from([
                ("enabled".into(), json!(true)),
                ("extra".into(), json!(1)),
            ]))
            .is_err());
        assert!(capability
            .validate_parameters(&HashMap::from([("enabled".into(), json!(true))]))
            .is_ok());
    }

    #[test]
    fn capability_validates_only_nested_inputs_selected_by_the_parent_option() {
        let mut capability = test_capability_with_nested_input();
        let valid = HashMap::from([
            ("scene".into(), json!("Main")),
            ("source".into(), json!("Camera")),
        ]);
        assert!(capability.validate_parameters(&valid).is_ok());

        capability.inputs[0].options[0].value = json!("Other");
        assert!(capability.validate_parameters(&valid).is_err());
    }

    fn test_capability_with_nested_input() -> CapabilityDefinition {
        let mut capability = CapabilityDefinition {
            integration_id: "test".into(),
            capability_id: "nested".into(),
            version: 1,
            kind: CapabilityKind::Action,
            presentation: CapabilityPresentation {
                name: "Nested".into(),
                description: String::new(),
                category: "Test".into(),
                icon: None,
                aliases: vec![],
            },
            inputs: vec![],
            outputs: vec![],
            allow_additional_inputs: false,
            contract: ActionContract {
                integration_id: "test".into(),
                action_id: "nested".into(),
                version: 1,
                required_permission: PluginPermission::TriggerAction(
                    "integration:test:nested".into(),
                ),
                timeout_ms: 1_000,
                recovery: ActionRecoverySemantics::Reconcile,
                resource_key: None,
                receipt: ActionReceiptSemantics::TransportAccepted,
            },
        };
        capability.inputs.push(CapabilityParameter {
            id: "scene".into(),
            label: "Scene".into(),
            description: String::new(),
            value_type: ContractValueType::String,
            required: true,
            widget: CapabilityWidget::Select,
            default_value: None,
            option_source: None,
            options: vec![CapabilityOption {
                label: "Main".into(),
                value: json!("Main"),
                parameters: vec![CapabilityParameter {
                    id: "source".into(),
                    label: "Source".into(),
                    description: String::new(),
                    value_type: ContractValueType::String,
                    required: true,
                    widget: CapabilityWidget::Text,
                    default_value: None,
                    options: vec![],
                    option_source: None,
                }],
            }],
        });
        capability
    }
}
