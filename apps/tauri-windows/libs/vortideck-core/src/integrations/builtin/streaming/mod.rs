//! Provider-neutral streaming relay capabilities.
//!
//! Twitch, YouTube, Kick, TikTok, and Discord accounts remain provider-owned
//! relay targets. VortiDeck publishes one stable capability model and reports
//! only local transport acceptance until the relay supplies correlated applied
//! receipts.

use crate::integrations::{
    ActionContract, ActionReceiptSemantics, ActionRecoverySemantics, AppIntegration,
    CapabilityDefinition, CapabilityKind, CapabilityOption, CapabilityOutput, CapabilityParameter,
    CapabilityPresentation, CapabilityWidget, ContractValueType, DynamicOptionSource,
    EventCallback, IntegrationAction, IntegrationActionResult, IntegrationActionStatus,
    IntegrationError, IntegrationId, IntegrationMetadata, IntegrationRequirements,
    IntegrationResult,
};
use crate::services::streaming::{PlatformStream, StreamingState};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub struct StreamingIntegration {
    id: IntegrationId,
    state: Arc<StreamingState>,
}

impl StreamingIntegration {
    pub fn new(state: Arc<StreamingState>) -> Self {
        Self {
            id: IntegrationId::new("streaming"),
            state,
        }
    }

    fn accounts_parameter() -> CapabilityParameter {
        CapabilityParameter {
            id: "accountIds".to_string(),
            label: "Target accounts".to_string(),
            description: "One or more connected streaming accounts".to_string(),
            value_type: ContractValueType::Array,
            required: true,
            widget: CapabilityWidget::MultiSelect,
            default_value: Some(json!([])),
            options: vec![],
            option_source: Some(DynamicOptionSource {
                resolver_id: "streaming_accounts".to_string(),
                depends_on: vec![],
            }),
        }
    }

    fn string_parameter(id: &str, label: &str, required: bool) -> CapabilityParameter {
        CapabilityParameter {
            id: id.to_string(),
            label: label.to_string(),
            description: label.to_string(),
            value_type: ContractValueType::String,
            required,
            widget: if id == "message" || id == "reason" {
                CapabilityWidget::Textarea
            } else {
                CapabilityWidget::Text
            },
            default_value: None,
            options: vec![],
            option_source: None,
        }
    }

    fn capability(
        id: &str,
        name: &str,
        description: &str,
        mut inputs: Vec<CapabilityParameter>,
    ) -> CapabilityDefinition {
        inputs.insert(0, Self::accounts_parameter());
        CapabilityDefinition {
            integration_id: "streaming".to_string(),
            capability_id: id.to_string(),
            version: 1,
            kind: CapabilityKind::Action,
            presentation: CapabilityPresentation {
                name: name.to_string(),
                description: description.to_string(),
                category: "Streaming platforms".to_string(),
                icon: Some("lucide:radio-tower".to_string()),
                aliases: vec!["chat".to_string(), "moderation".to_string()],
            },
            inputs,
            outputs: vec![
                CapabilityOutput {
                    id: "overall".to_string(),
                    label: "Overall result".to_string(),
                    value_type: ContractValueType::String,
                },
                CapabilityOutput {
                    id: "targetResults".to_string(),
                    label: "Per-account results".to_string(),
                    value_type: ContractValueType::Array,
                },
                CapabilityOutput {
                    id: "acceptedCount".to_string(),
                    label: "Accepted targets".to_string(),
                    value_type: ContractValueType::Integer,
                },
                CapabilityOutput {
                    id: "failedCount".to_string(),
                    label: "Failed targets".to_string(),
                    value_type: ContractValueType::Integer,
                },
            ],
            allow_additional_inputs: false,
            contract: ActionContract {
                integration_id: "streaming".to_string(),
                action_id: id.to_string(),
                version: 1,
                required_permission: crate::plugins::bundle::PluginPermission::TriggerAction(
                    format!("integration:streaming:{}", id),
                ),
                timeout_ms: 10_000,
                recovery: ActionRecoverySemantics::ManualReview,
                resource_key: Some("streaming:accounts".to_string()),
                receipt: ActionReceiptSemantics::TransportAccepted,
            },
        }
    }

    async fn configured_accounts(&self) -> Vec<PlatformStream> {
        let mut seen = HashSet::new();
        let mut accounts = Vec::new();
        if let Ok(sessions) = self.state.session_manager.get_all_sessions().await {
            for account in sessions.iter().flat_map(|session| session.platforms.iter()) {
                if seen.insert(account.account_id.clone()) {
                    accounts.push(account.clone());
                }
            }
        }
        accounts.sort_by(|left, right| {
            left.platform
                .to_string()
                .cmp(&right.platform.to_string())
                .then(left.account_id.cmp(&right.account_id))
        });
        accounts
    }

    fn selected_account_ids(parameters: &HashMap<String, Value>) -> IntegrationResult<Vec<String>> {
        let values = parameters
            .get("accountIds")
            .and_then(Value::as_array)
            .ok_or_else(|| IntegrationError::InvalidParameter("accountIds".to_string()))?;
        if values.is_empty() {
            return Err(IntegrationError::InvalidParameter(
                "At least one streaming account is required".to_string(),
            ));
        }
        let mut account_ids: Vec<_> = values
            .iter()
            .map(|value| match value {
                Value::String(value) if !value.trim().is_empty() => Ok(value.clone()),
                Value::Number(value) => Ok(value.to_string()),
                _ => Err(IntegrationError::InvalidParameter(
                    "accountIds must contain strings or integers".to_string(),
                )),
            })
            .collect::<IntegrationResult<_>>()?;
        account_ids.sort();
        account_ids.dedup();
        Ok(account_ids)
    }

    async fn dispatch(
        &self,
        action: &IntegrationAction,
    ) -> IntegrationResult<IntegrationActionResult> {
        let request_id = action
            .execution
            .as_ref()
            .and_then(|execution| execution.request_id.clone())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let selected = Self::selected_account_ids(&action.parameters)?;
        let catalog: HashMap<_, _> = self
            .configured_accounts()
            .await
            .into_iter()
            .map(|account| (account.account_id.clone(), account))
            .collect();
        let mut target_results = Vec::with_capacity(selected.len());
        let mut accepted = 0usize;

        for account_id in selected {
            let Some(account) = catalog.get(&account_id) else {
                target_results.push(json!({
                    "accountId": account_id,
                    "status": "failed",
                    "error": "account is not configured in a streaming session"
                }));
                continue;
            };
            let target_request_id = format!("{}:{}", request_id, account_id);
            let mut payload = json!({
                "type": action.name,
                "request_id": target_request_id,
                "account_id": account_id,
                "platform": account.platform,
            });
            let object = payload.as_object_mut().expect("object literal");
            match action.name.as_str() {
                "send_chat" => {
                    let Ok(numeric_account_id) = account.account_id.parse::<u64>() else {
                        target_results.push(json!({
                            "accountId": account.account_id,
                            "platform": account.platform,
                            "status": "failed",
                            "requestId": target_request_id,
                            "error": "relay chat transport requires a numeric account ID",
                        }));
                        continue;
                    };
                    object.insert("account_ids".to_string(), json!([numeric_account_id]));
                    object.insert("message".to_string(), action.parameters["message"].clone());
                    object.insert(
                        "reply_to".to_string(),
                        action
                            .parameters
                            .get("replyTo")
                            .cloned()
                            .unwrap_or(Value::Null),
                    );
                }
                "ban_user" => {
                    object.insert("user_id".to_string(), action.parameters["userId"].clone());
                    object.insert(
                        "reason".to_string(),
                        action
                            .parameters
                            .get("reason")
                            .cloned()
                            .unwrap_or(Value::Null),
                    );
                }
                "timeout_user" => {
                    object.insert("user_id".to_string(), action.parameters["userId"].clone());
                    object.insert(
                        "duration_seconds".to_string(),
                        action.parameters["durationSeconds"].clone(),
                    );
                    object.insert(
                        "reason".to_string(),
                        action
                            .parameters
                            .get("reason")
                            .cloned()
                            .unwrap_or(Value::Null),
                    );
                }
                "delete_message" => {
                    object.insert(
                        "message_id".to_string(),
                        action.parameters["messageId"].clone(),
                    );
                }
                unsupported => {
                    return Err(IntegrationError::UnsupportedAction(unsupported.to_string()))
                }
            }
            match self.state.send_payload(payload).await {
                Ok(()) => {
                    accepted += 1;
                    target_results.push(json!({
                        "accountId": account.account_id,
                        "platform": account.platform,
                        "status": "transport_accepted",
                        "requestId": target_request_id,
                    }));
                }
                Err(error) => target_results.push(json!({
                    "accountId": account.account_id,
                    "platform": account.platform,
                    "status": "failed",
                    "requestId": target_request_id,
                    "error": error,
                })),
            }
        }

        let failed = target_results.len().saturating_sub(accepted);
        if accepted == 0 {
            return Err(IntegrationError::ActionFailed(
                "Streaming command failed for every selected account".to_string(),
            ));
        }
        let overall = if failed == 0 {
            "transport_accepted"
        } else {
            "partial"
        };
        Ok(IntegrationActionResult {
            status: if failed == 0 {
                IntegrationActionStatus::TransportAccepted
            } else {
                IntegrationActionStatus::Partial
            },
            request_id,
            provider_receipt_id: None,
            outputs: HashMap::from([
                ("overall".to_string(), json!(overall)),
                ("targetResults".to_string(), json!(target_results)),
                ("acceptedCount".to_string(), json!(accepted)),
                ("failedCount".to_string(), json!(failed)),
            ]),
        })
    }
}

#[async_trait]
impl AppIntegration for StreamingIntegration {
    fn id(&self) -> &IntegrationId {
        &self.id
    }
    fn name(&self) -> &str {
        "Streaming platforms"
    }
    fn description(&self) -> &str {
        "Provider-neutral chat and moderation through the VortiDeck relay"
    }
    fn get_metadata(&self) -> IntegrationMetadata {
        IntegrationMetadata {
            id: self.id.clone(),
            name: self.name().to_string(),
            version: "1.0.0".to_string(),
            description: self.description().to_string(),
            author: "VortiDeck".to_string(),
            website: None,
        }
    }
    fn get_requirements(&self) -> IntegrationRequirements {
        IntegrationRequirements {
            min_version: None,
            max_version: None,
            dependencies: vec![],
        }
    }
    async fn is_available(&self) -> bool {
        self.state.is_connected().await
    }
    async fn execute_action(
        &self,
        action: IntegrationAction,
    ) -> IntegrationResult<IntegrationActionResult> {
        self.resolve_capability(&action.name, &action.parameters)
            .await?;
        self.dispatch(&action).await
    }
    async fn subscribe_to_events(
        &self,
        _callback: Box<dyn EventCallback>,
    ) -> IntegrationResult<()> {
        Ok(())
    }
    async fn get_capabilities(&self) -> Vec<CapabilityDefinition> {
        vec![
            Self::capability(
                "send_chat",
                "Send chat message",
                "Send a message to one or more platform accounts",
                vec![
                    Self::string_parameter("message", "Message", true),
                    Self::string_parameter("replyTo", "Reply to message ID", false),
                ],
            ),
            Self::capability(
                "ban_user",
                "Ban user",
                "Ban one user across selected platform accounts",
                vec![
                    Self::string_parameter("userId", "User ID", true),
                    Self::string_parameter("reason", "Reason", false),
                ],
            ),
            Self::capability(
                "timeout_user",
                "Timeout user",
                "Temporarily restrict one user across selected accounts",
                vec![
                    Self::string_parameter("userId", "User ID", true),
                    CapabilityParameter {
                        id: "durationSeconds".to_string(),
                        label: "Duration (seconds)".to_string(),
                        description: "Timeout duration".to_string(),
                        value_type: ContractValueType::Integer,
                        required: true,
                        widget: CapabilityWidget::Number,
                        default_value: Some(json!(600)),
                        options: vec![],
                        option_source: None,
                    },
                    Self::string_parameter("reason", "Reason", false),
                ],
            ),
            Self::capability(
                "delete_message",
                "Delete message",
                "Delete a message across selected accounts",
                vec![Self::string_parameter("messageId", "Message ID", true)],
            ),
        ]
    }
    async fn resolve_capability(
        &self,
        capability_id: &str,
        parameters: &HashMap<String, Value>,
    ) -> IntegrationResult<CapabilityDefinition> {
        let mut capability = self
            .get_capabilities()
            .await
            .into_iter()
            .find(|capability| capability.capability_id == capability_id)
            .ok_or_else(|| IntegrationError::UnsupportedAction(capability_id.to_string()))?;
        capability
            .validate_parameters(parameters)
            .map_err(IntegrationError::ValidationError)?;
        let accounts = Self::selected_account_ids(parameters)?;
        let configured: HashSet<_> = self
            .configured_accounts()
            .await
            .into_iter()
            .map(|account| account.account_id)
            .collect();
        if let Some(missing) = accounts
            .iter()
            .find(|account| !configured.contains(*account))
        {
            return Err(IntegrationError::NotFound(format!(
                "Streaming account '{}' is not configured",
                missing
            )));
        }
        capability.contract.resource_key =
            Some(format!("streaming:accounts:{}", accounts.join(",")));
        Ok(capability)
    }
    async fn resolve_capability_options(
        &self,
        _capability_id: &str,
        parameter_id: &str,
        _current_values: &HashMap<String, Value>,
    ) -> IntegrationResult<Vec<CapabilityOption>> {
        if parameter_id != "accountIds" {
            return Err(IntegrationError::InvalidParameter(parameter_id.to_string()));
        }
        Ok(self
            .configured_accounts()
            .await
            .into_iter()
            .map(|account| CapabilityOption {
                label: format!(
                    "{} · {}",
                    account.platform,
                    account.title.unwrap_or_else(|| account.account_id.clone())
                ),
                value: Value::String(account.account_id),
                parameters: vec![],
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::r2d2::{ConnectionManager, Pool};
    use diesel::SqliteConnection;

    fn integration() -> StreamingIntegration {
        let manager = ConnectionManager::<SqliteConnection>::new(":memory:");
        let pool = Pool::builder().max_size(1).build(manager).unwrap();
        StreamingIntegration::new(Arc::new(StreamingState::with_db(Arc::new(pool))))
    }

    #[tokio::test]
    async fn publishes_provider_neutral_chat_and_moderation_capabilities() {
        let capabilities = integration().get_capabilities().await;
        assert_eq!(capabilities.len(), 4);
        assert!(capabilities.iter().all(|capability| {
            capability.integration_id == "streaming"
                && capability.contract.receipt == ActionReceiptSemantics::TransportAccepted
                && capability.inputs.first().is_some_and(|input| {
                    input.id == "accountIds"
                        && input.widget == CapabilityWidget::MultiSelect
                        && input.option_source.is_some()
                })
        }));
    }

    #[test]
    fn account_selection_accepts_normalizes_and_deduplicates_ids() {
        let parameters = HashMap::from([("accountIds".to_string(), json!([34, "12", "12"]))]);
        assert_eq!(
            StreamingIntegration::selected_account_ids(&parameters).unwrap(),
            vec!["12".to_string(), "34".to_string()]
        );
    }

    #[tokio::test]
    async fn rejects_missing_accounts_during_capability_resolution() {
        let parameters = HashMap::from([
            ("accountIds".to_string(), json!(["missing"])),
            ("message".to_string(), json!("hello")),
        ]);
        let error = integration()
            .resolve_capability("send_chat", &parameters)
            .await
            .unwrap_err();
        assert!(matches!(error, IntegrationError::NotFound(_)));
    }
}
