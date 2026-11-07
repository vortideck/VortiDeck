//! Canonical durable invocation boundary shared by every user-facing surface.

use super::{
    convert_action_reference_to_universal_action, convert_action_to_universal_action,
    ActionReference, ActionRuntimeReadiness, ExecutionContext, ExecutionEngine, ExecutionError,
    ExecutionResult, UniversalAction, UniversalExecutionEngine,
};
use crate::db::{
    models::{Action, ActionInvocationRecord, NewActionInvocation},
    Database,
};
use crate::integrations::{ActionReceiptSemantics, ActionRecoverySemantics, CapabilityDefinition};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InvocationStatus {
    Applied,
    TransportAccepted,
    Partial,
    Failed,
    TimedOut,
    Uncertain,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvocationResult {
    pub invocation_id: String,
    pub status: InvocationStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub execution_time_ms: u64,
    pub outputs: HashMap<String, Value>,
    pub error: Option<ExecutionError>,
    pub retryable: bool,
    pub recovery: Option<ActionRecoverySemantics>,
}

/// Admission is durable before this handle is returned. Dropping a caller's
/// handle does not abort the invocation-owned execution/outcome writer.
pub struct InvocationSubmission {
    pub invocation_id: String,
    pub duplicate: bool,
    started_at: DateTime<Utc>,
    completion: Option<tokio::task::JoinHandle<InvocationResult>>,
    previous: Option<InvocationResult>,
}

impl InvocationSubmission {
    pub async fn wait(self) -> InvocationResult {
        if let Some(previous) = self.previous {
            return previous;
        }
        match self
            .completion
            .expect("new admission owns completion")
            .await
        {
            Ok(result) => result,
            Err(error) => ActionInvocationService::persistence_failure(
                self.invocation_id,
                self.started_at,
                format!("Action task ended without a terminal receipt: {}", error),
                true,
            ),
        }
    }
}

/// Start data for a specialized adapter whose receipt lifecycle cannot be
/// represented by a normal provider action (for example overlay visual ACKs).
#[derive(Debug, Clone)]
pub struct ExternalInvocationStart {
    pub invocation_id: String,
    pub source: String,
    pub source_reference: Option<String>,
    pub workflow_run_id: Option<String>,
    pub node_id: Option<String>,
    pub resolved_action: Value,
    pub idempotency_key: Option<String>,
    pub resource_key: Option<String>,
    pub receipt_semantics: String,
    pub recovery_decision: String,
}

#[derive(Debug, Clone)]
pub struct ExternalInvocationCompletion {
    pub dispatch_status: String,
    pub request_id: Option<String>,
    pub provider_receipt_id: Option<String>,
    pub outputs: HashMap<String, Value>,
    pub normalized_status: InvocationStatus,
    pub error: Option<String>,
    pub recovery_decision: String,
    pub execution_time_ms: u64,
}

impl InvocationResult {
    pub fn succeeded(&self) -> bool {
        matches!(
            self.status,
            InvocationStatus::Applied
                | InvocationStatus::TransportAccepted
                | InvocationStatus::Partial
                | InvocationStatus::Skipped
        )
    }
}

pub struct ActionInvocationService {
    engine: Arc<UniversalExecutionEngine>,
    database: Arc<Database>,
    readiness: Arc<ActionRuntimeReadiness>,
    admission: tokio::sync::RwLock<bool>,
    tasks: tokio_util::task::TaskTracker,
}

impl ActionInvocationService {
    pub async fn get_input_contract(
        &self,
        reference: &ActionReference,
    ) -> Result<super::input_contract::InputContractSnapshot, String> {
        let key = reference.storage_key()?;
        match self.database.get_action_input_contract(&key).await? {
            Some((revision, raw)) => Ok(super::input_contract::InputContractSnapshot {
                revision,
                contract: serde_json::from_str(&raw)
                    .map_err(|_| "Invalid stored action input contract")?,
            }),
            None => Ok(super::input_contract::InputContractSnapshot {
                revision: 0,
                contract: None,
            }),
        }
    }

    pub async fn save_input_contract(
        &self,
        reference: &ActionReference,
        expected_revision: i64,
        contract: Option<super::input_contract::ActionInputContract>,
    ) -> Result<super::input_contract::InputContractSnapshot, String> {
        // Resolve the creator-owned target before publishing a declaration.
        let template = self.resolve_reference(reference, None, None).await?;
        if let Some(contract) = &contract {
            contract.validate_for_template(&template)?;
        }
        let key = reference.storage_key()?;
        let raw = serde_json::to_string(&contract).map_err(|_| "Invalid action input contract")?;
        let revision = self
            .database
            .save_action_input_contract(&key, expected_revision, raw)
            .await?;
        Ok(super::input_contract::InputContractSnapshot { revision, contract })
    }

    pub async fn validate_reference_inputs(
        &self,
        reference: &ActionReference,
        inputs: &Value,
    ) -> Result<serde_json::Map<String, Value>, String> {
        match self.get_input_contract(reference).await?.contract {
            Some(contract) => contract.validate_inputs(inputs),
            None => {
                let values = inputs
                    .as_object()
                    .ok_or("Action inputs must be an object")?;
                if !values.is_empty() {
                    return Err("Saved action does not declare invocation inputs".into());
                }
                Ok(values.clone())
            }
        }
    }

    pub async fn prepare_reference(
        &self,
        reference: &ActionReference,
        value: Option<f64>,
        button_id: Option<String>,
        inputs: &Value,
    ) -> Result<UniversalAction, String> {
        let template = self.resolve_reference(reference, value, button_id).await?;
        match self.get_input_contract(reference).await?.contract {
            Some(contract) => contract.prepare(&template, inputs),
            None => {
                if !inputs.as_object().is_some_and(|values| values.is_empty()) {
                    return Err("Saved action does not declare invocation inputs".into());
                }
                Ok(template)
            }
        }
    }

    fn context_inputs(context: &ExecutionContext) -> Value {
        context
            .execution_config
            .as_ref()
            .and_then(|config| config.get("inputs"))
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}))
    }

    pub async fn prepare_database_action(
        &self,
        action: &Action,
        value: Option<f64>,
        button_id: Option<String>,
        inputs: &Value,
    ) -> Result<UniversalAction, String> {
        if let Some(reference) = ActionReference::from_database_action(action) {
            return self
                .prepare_reference(&reference, value, button_id, inputs)
                .await;
        }
        if !inputs.as_object().is_some_and(|inputs| inputs.is_empty()) {
            return Err("This action does not declare invocation inputs".into());
        }
        self.resolve_database_action(action, value, button_id).await
    }
    pub fn new(
        engine: Arc<UniversalExecutionEngine>,
        database: Arc<Database>,
        readiness: Arc<ActionRuntimeReadiness>,
    ) -> Self {
        Self {
            engine,
            database,
            readiness,
            admission: tokio::sync::RwLock::new(true),
            tasks: tokio_util::task::TaskTracker::new(),
        }
    }

    /// Close acceptance before shutdown checkpoints, while allowing already
    /// accepted work to persist its outcome.
    pub async fn close_admission(&self) {
        *self.admission.write().await = false;
        self.tasks.close();
    }

    pub async fn wait_for_idle(&self) {
        self.tasks.wait().await;
    }

    pub async fn resolve_reference(
        &self,
        reference: &ActionReference,
        value: Option<f64>,
        button_id: Option<String>,
    ) -> Result<UniversalAction, String> {
        convert_action_reference_to_universal_action(reference, value, button_id, &self.database)
            .await
    }

    pub async fn resolve_database_action(
        &self,
        action: &Action,
        value: Option<f64>,
        button_id: Option<String>,
    ) -> Result<UniversalAction, String> {
        convert_action_to_universal_action(action, value, button_id, &self.database).await
    }

    pub async fn resolve_capability(
        &self,
        action: &UniversalAction,
    ) -> Result<Option<CapabilityDefinition>, ExecutionError> {
        self.engine.resolve_integration_capability(action).await
    }

    pub async fn invoke_reference(
        &self,
        reference: &ActionReference,
        value: Option<f64>,
        button_id: Option<String>,
        context: &ExecutionContext,
    ) -> Result<InvocationResult, String> {
        self.readiness.wait_until_ready().await?;
        let action = self
            .prepare_reference(reference, value, button_id, &Self::context_inputs(context))
            .await?;
        Ok(self.invoke_resolved_action(&action, context).await)
    }

    pub async fn invoke_database_action(
        &self,
        action: &Action,
        value: Option<f64>,
        button_id: Option<String>,
        context: &ExecutionContext,
    ) -> Result<InvocationResult, String> {
        self.readiness.wait_until_ready().await?;
        let action = self
            .prepare_database_action(action, value, button_id, &Self::context_inputs(context))
            .await?;
        Ok(self.invoke_resolved_action(&action, context).await)
    }

    pub async fn invoke_resolved_action(
        &self,
        action: &UniversalAction,
        context: &ExecutionContext,
    ) -> InvocationResult {
        match self.submit_resolved_action(action, context).await {
            Ok(submission) => submission.wait().await,
            Err(result) => *result,
        }
    }

    /// Shared asynchronous admission for transports that must continue reading
    /// while a provider effect waits. Resolution and persistence still happen
    /// before acceptance, using the same boundary as synchronous callers.
    pub async fn submit_database_action(
        &self,
        action: &Action,
        context: &ExecutionContext,
    ) -> Result<InvocationSubmission, String> {
        self.readiness.wait_until_ready().await?;
        let resolved = self
            .prepare_database_action(action, None, None, &Self::context_inputs(context))
            .await?;
        self.submit_resolved_action(&resolved, context)
            .await
            .map_err(|result| {
                result
                    .error
                    .map(|error| error.message)
                    .unwrap_or_else(|| "Invocation admission failed".into())
            })
    }

    pub async fn submit_resolved_action(
        &self,
        action: &UniversalAction,
        context: &ExecutionContext,
    ) -> Result<InvocationSubmission, Box<InvocationResult>> {
        let invocation_id = context
            .execution_config
            .as_ref()
            .and_then(|value| value.get("invocationId"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let request_id = invocation_id.clone();
        let started_at = Utc::now();
        let admission = self.admission.read().await;
        if !*admission {
            return Err(Box::new(Self::runtime_unavailable_failure(
                invocation_id,
                started_at,
                "Action runtime is shutting down".into(),
            )));
        }
        if let Err(error) = self.readiness.wait_until_ready().await {
            return Err(Box::new(Self::runtime_unavailable_failure(
                invocation_id,
                started_at,
                error,
            )));
        }
        let capability = self.resolve_capability(action).await.ok().flatten();
        let resolved_action =
            match Self::action_snapshot(action).and_then(|action| serde_json::to_string(&action)) {
                Ok(action) => action,
                Err(error) => {
                    return Err(Box::new(Self::persistence_failure(
                        invocation_id,
                        started_at,
                        format!("Failed to serialize resolved action: {}", error),
                        false,
                    )));
                }
            };
        let source = Self::source_name(&context.trigger_source);
        let source_reference = Self::source_reference(&context.trigger_source);
        let (workflow_run_id, node_id) = match &context.trigger_source {
            super::TriggerSource::Flow { flow_id, node_id } => {
                (Some(flow_id.as_str()), Some(node_id.as_str()))
            }
            _ => (None, None),
        };
        let parent_invocation_id = context
            .execution_config
            .as_ref()
            .and_then(|value| value.get("parentInvocationId"))
            .and_then(Value::as_str);
        let idempotency_key = context
            .execution_config
            .as_ref()
            .and_then(|value| value.get("idempotencyKey"))
            .and_then(Value::as_str);
        let (integration_id, action_id) = match action {
            UniversalAction::Integration {
                integration_id,
                action_id,
                ..
            } => (Some(integration_id.as_str()), Some(action_id.as_str())),
            _ => (None, None),
        };
        let receipt_semantics = capability
            .as_ref()
            .and_then(|capability| serde_json::to_string(&capability.contract.receipt).ok());
        let recovery = capability
            .as_ref()
            .and_then(|capability| serde_json::to_string(&capability.contract.recovery).ok());
        let now = started_at.to_rfc3339();
        let claimed = self
            .database
            .claim_action_invocation(NewActionInvocation {
                invocation_id: &invocation_id,
                parent_invocation_id,
                source,
                source_reference: source_reference.as_deref(),
                workflow_run_id,
                node_id,
                resolved_action: &resolved_action,
                integration_id,
                action_id,
                capability_version: capability.as_ref().map(|value| value.version as i32),
                idempotency_key,
                resource_key: capability
                    .as_ref()
                    .and_then(|value| value.contract.resource_key.as_deref()),
                dispatch_status: "dispatching",
                receipt_semantics: receipt_semantics.as_deref(),
                request_id: Some(&request_id),
                provider_receipt_id: None,
                provider_outputs: None,
                normalized_status: None,
                error: None,
                recovery_decision: recovery.as_deref(),
                started_at: &now,
                completed_at: None,
                execution_time_ms: 0,
                created_at: &now,
                updated_at: &now,
            })
            .await;
        match claimed {
            Err(error) => {
                return Err(Box::new(Self::persistence_failure(
                    invocation_id,
                    started_at,
                    error,
                    false,
                )))
            }
            Ok(false) => {
                let record = match self.database.get_action_invocation(&invocation_id).await {
                    Ok(Some(record)) => record,
                    Ok(None) => {
                        return Err(Box::new(Self::persistence_failure(
                            invocation_id,
                            started_at,
                            "Existing invocation disappeared".into(),
                            false,
                        )))
                    }
                    Err(error) => {
                        return Err(Box::new(Self::persistence_failure(
                            invocation_id,
                            started_at,
                            error,
                            false,
                        )))
                    }
                };
                if record.source != source
                    || record.source_reference != source_reference
                    || record.workflow_run_id.as_deref() != workflow_run_id
                    || record.node_id.as_deref() != node_id
                    || record.idempotency_key.as_deref() != idempotency_key
                    || record.resolved_action != resolved_action
                {
                    return Err(Box::new(Self::persistence_failure(
                        invocation_id,
                        started_at,
                        "Invocation identity was already used for a different request".into(),
                        false,
                    )));
                }
                return Ok(InvocationSubmission {
                    invocation_id,
                    duplicate: true,
                    started_at,
                    completion: None,
                    previous: Some(Self::previous_result(record)),
                });
            }
            Ok(true) => {}
        }

        let mut durable_context = context.clone();
        let mut execution_config = durable_context
            .execution_config
            .take()
            .and_then(|value| value.as_object().cloned())
            .unwrap_or_default();
        execution_config.insert(
            "invocationId".to_string(),
            Value::String(invocation_id.clone()),
        );
        execution_config.insert("requestId".to_string(), Value::String(request_id));
        durable_context.execution_config = Some(Value::Object(execution_config));
        let engine = self.engine.clone();
        let database = self.database.clone();
        let owned_action = action.clone();
        let task_invocation_id = invocation_id.clone();
        let task_started_at = started_at.clone();
        let task = self.tasks.spawn(async move {
            Self::execute_and_persist(
                engine,
                database,
                owned_action,
                durable_context,
                task_invocation_id,
                task_started_at,
                capability,
                recovery,
            )
            .await
        });
        Ok(InvocationSubmission {
            invocation_id,
            duplicate: false,
            started_at,
            completion: Some(task),
            previous: None,
        })
    }

    /// History is diagnostic metadata, never a credential or HTTP payload store.
    /// Walk composites as well, since a sequence can contain HTTP actions.
    fn action_snapshot(action: &UniversalAction) -> Result<Value, serde_json::Error> {
        fn redact(value: &mut Value) {
            match value {
                Value::Object(fields) => {
                    if let Some(Value::Object(request)) = fields.get_mut("HttpRequest") {
                        if let Some(url) = request.remove("url") {
                            let origin = url
                                .as_str()
                                .and_then(|url| reqwest::Url::parse(url).ok())
                                .map(|url| url.origin().ascii_serialization());
                            request.insert("origin".into(), serde_json::json!(origin));
                        }
                        if let Some(Value::Object(headers)) = request.get_mut("headers") {
                            for value in headers.values_mut() {
                                if !value.as_str().is_some_and(
                                    crate::services::api::credentials::is_credential_reference,
                                ) {
                                    *value = Value::String("[redacted]".into());
                                }
                            }
                        }
                        let has_body = request.remove("body").is_some_and(|body| !body.is_null());
                        request.insert("bodyOmitted".into(), Value::Bool(has_body));
                    }
                    fields.values_mut().for_each(redact);
                }
                Value::Array(items) => items.iter_mut().for_each(redact),
                _ => {}
            }
        }
        let mut snapshot = serde_json::to_value(action)?;
        redact(&mut snapshot);
        Ok(snapshot)
    }

    fn contains_http(action: &UniversalAction) -> bool {
        match action {
            UniversalAction::HttpRequest { .. } => true,
            UniversalAction::Sequence { actions, .. } => actions.iter().any(Self::contains_http),
            UniversalAction::Conditional {
                if_true, if_false, ..
            } => {
                Self::contains_http(if_true) || if_false.as_deref().is_some_and(Self::contains_http)
            }
            _ => false,
        }
    }

    fn previous_result(record: ActionInvocationRecord) -> InvocationResult {
        let started_at = DateTime::parse_from_rfc3339(&record.started_at)
            .map(|time| time.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        let completed_at = record
            .completed_at
            .as_deref()
            .and_then(|time| DateTime::parse_from_rfc3339(time).ok())
            .map(|time| time.with_timezone(&Utc))
            .unwrap_or(started_at);
        let status = record
            .normalized_status
            .as_ref()
            .and_then(|status| serde_json::from_value(Value::String(status.clone())).ok())
            .unwrap_or(InvocationStatus::Uncertain);
        let mut outputs: HashMap<String, Value> = record
            .provider_outputs
            .as_deref()
            .and_then(|outputs| serde_json::from_str(outputs).ok())
            .unwrap_or_default();
        outputs.insert("deduplicated".into(), Value::Bool(true));
        outputs.insert(
            "dispatchStatus".into(),
            Value::String(record.dispatch_status),
        );
        let error = record.error.as_deref().and_then(|error| serde_json::from_str(error).ok()).or_else(|| {
            record.normalized_status.is_none().then(|| ExecutionError {
                error_type: super::ErrorType::SystemError,
                message: "Previously admitted invocation has no confirmed terminal outcome; it was not dispatched again".into(),
                details: None,
                recoverable: false,
            })
        });
        InvocationResult {
            invocation_id: record.invocation_id,
            status,
            started_at,
            completed_at,
            execution_time_ms: record.execution_time_ms.max(0) as u64,
            outputs,
            error,
            retryable: false,
            recovery: record
                .recovery_decision
                .as_deref()
                .and_then(|recovery| serde_json::from_str(recovery).ok()),
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn execute_and_persist(
        engine: Arc<UniversalExecutionEngine>,
        database: Arc<Database>,
        action: UniversalAction,
        context: ExecutionContext,
        invocation_id: String,
        started_at: DateTime<Utc>,
        capability: Option<CapabilityDefinition>,
        recovery: Option<String>,
    ) -> InvocationResult {
        let result = engine.execute_action(&action, &context).await;
        let mut normalized = Self::normalize(
            invocation_id,
            started_at,
            Utc::now(),
            result,
            capability.as_ref(),
        );
        let provider_request_id = normalized
            .outputs
            .get("invocation")
            .and_then(|value| value.get("requestId"))
            .and_then(Value::as_str);
        let provider_receipt_id = normalized
            .outputs
            .get("invocation")
            .and_then(|value| value.get("providerReceiptId"))
            .and_then(Value::as_str);
        let outputs = if Self::contains_http(&action) {
            serde_json::to_string(&serde_json::json!({
                "status": normalized.outputs.get("status"),
                "responseOmitted": true
            }))
            .ok()
        } else {
            serde_json::to_string(&normalized.outputs).ok()
        };
        let error_json = normalized
            .error
            .as_ref()
            .and_then(|error| serde_json::to_string(error).ok());
        let status = Self::status_name(&normalized.status);
        let dispatch_status = Self::dispatch_status_name(&normalized.status);
        let completed_at = normalized.completed_at.to_rfc3339();
        if let Err(error) = database
            .complete_action_invocation(
                &normalized.invocation_id,
                dispatch_status,
                provider_request_id,
                provider_receipt_id,
                outputs.as_deref(),
                status,
                error_json.as_deref(),
                recovery.as_deref(),
                &completed_at,
                normalized.execution_time_ms.min(i64::MAX as u64) as i64,
            )
            .await
        {
            normalized.status = InvocationStatus::Uncertain;
            normalized.retryable = false;
            normalized.error = Some(ExecutionError {
                error_type: super::ErrorType::SystemError,
                message: format!(
                    "Action completed but its durable outcome could not be stored: {}",
                    error
                ),
                details: Some(serde_json::json!({ "persistenceFailure": true })),
                recoverable: false,
            });
        }
        normalized
    }

    /// Persist a specialized invocation before dispatch. Callers must fail
    /// closed if this returns an error.
    pub async fn begin_external_invocation(
        &self,
        start: &ExternalInvocationStart,
    ) -> Result<(), String> {
        self.readiness.wait_until_ready().await?;
        let now = Utc::now().to_rfc3339();
        let resolved_action =
            serde_json::to_string(&start.resolved_action).map_err(|error| error.to_string())?;
        self.database
            .create_action_invocation(NewActionInvocation {
                invocation_id: &start.invocation_id,
                parent_invocation_id: None,
                source: &start.source,
                source_reference: start.source_reference.as_deref(),
                workflow_run_id: start.workflow_run_id.as_deref(),
                node_id: start.node_id.as_deref(),
                resolved_action: &resolved_action,
                integration_id: Some("overlay"),
                action_id: Some("command"),
                capability_version: Some(1),
                idempotency_key: start.idempotency_key.as_deref(),
                resource_key: start.resource_key.as_deref(),
                dispatch_status: "dispatching",
                receipt_semantics: Some(&start.receipt_semantics),
                request_id: Some(&start.invocation_id),
                provider_receipt_id: None,
                provider_outputs: None,
                normalized_status: None,
                error: None,
                recovery_decision: Some(&start.recovery_decision),
                started_at: &now,
                completed_at: None,
                execution_time_ms: 0,
                created_at: &now,
                updated_at: &now,
            })
            .await
    }

    pub async fn complete_external_invocation(
        &self,
        invocation_id: &str,
        completion: &ExternalInvocationCompletion,
    ) -> Result<(), String> {
        let outputs =
            serde_json::to_string(&completion.outputs).map_err(|error| error.to_string())?;
        let error = completion
            .error
            .as_ref()
            .map(|message| serde_json::json!({ "message": message }).to_string());
        let completed_at = Utc::now().to_rfc3339();
        self.database
            .complete_action_invocation(
                invocation_id,
                &completion.dispatch_status,
                completion.request_id.as_deref(),
                completion.provider_receipt_id.as_deref(),
                Some(&outputs),
                Self::status_name(&completion.normalized_status),
                error.as_deref(),
                Some(&completion.recovery_decision),
                &completed_at,
                completion.execution_time_ms.min(i64::MAX as u64) as i64,
            )
            .await
    }

    pub async fn get_invocation(
        &self,
        invocation_id: &str,
    ) -> Result<Option<ActionInvocationRecord>, String> {
        self.database.get_action_invocation(invocation_id).await
    }

    pub async fn list_invocations(
        &self,
        limit: i64,
        source: Option<&str>,
        status: Option<&str>,
        workflow_run_id: Option<&str>,
        node_id: Option<&str>,
    ) -> Result<Vec<ActionInvocationRecord>, String> {
        self.database
            .list_action_invocations(limit, source, status, workflow_run_id, node_id)
            .await
    }

    fn source_name(source: &super::TriggerSource) -> &'static str {
        match source {
            super::TriggerSource::Button { .. } => "button",
            super::TriggerSource::Macro { .. } => "macro",
            super::TriggerSource::Flow { .. } => "flow",
            super::TriggerSource::WebSocket { .. } => "websocket",
            super::TriggerSource::Schedule { .. } => "schedule",
            super::TriggerSource::Plugin { .. } => "plugin",
            super::TriggerSource::Manual { .. } => "manual",
            super::TriggerSource::ContextMenu { .. } => "context_menu",
        }
    }

    fn source_reference(source: &super::TriggerSource) -> Option<String> {
        match source {
            super::TriggerSource::Button { button_id, deck_id } => {
                Some(format!("{}:{}", deck_id, button_id))
            }
            super::TriggerSource::Macro { macro_id } => Some(macro_id.clone()),
            super::TriggerSource::Flow { flow_id, node_id } => {
                Some(format!("{}:{}", flow_id, node_id))
            }
            super::TriggerSource::WebSocket { client_id } => Some(client_id.clone()),
            super::TriggerSource::Schedule { schedule_id } => Some(schedule_id.clone()),
            super::TriggerSource::Plugin { plugin_id } => Some(plugin_id.clone()),
            super::TriggerSource::Manual { user_id } => Some(user_id.clone()),
            super::TriggerSource::ContextMenu { item_id } => Some(item_id.clone()),
        }
    }

    fn status_name(status: &InvocationStatus) -> &'static str {
        match status {
            InvocationStatus::Applied => "applied",
            InvocationStatus::TransportAccepted => "transport_accepted",
            InvocationStatus::Partial => "partial",
            InvocationStatus::Failed => "failed",
            InvocationStatus::TimedOut => "timed_out",
            InvocationStatus::Uncertain => "uncertain",
            InvocationStatus::Skipped => "skipped",
        }
    }

    fn dispatch_status_name(status: &InvocationStatus) -> &'static str {
        match status {
            InvocationStatus::Applied | InvocationStatus::Partial | InvocationStatus::Skipped => {
                "succeeded"
            }
            InvocationStatus::TransportAccepted => "transport_accepted",
            InvocationStatus::Uncertain => "uncertain",
            InvocationStatus::Failed | InvocationStatus::TimedOut => "failed",
        }
    }

    fn persistence_failure(
        invocation_id: String,
        started_at: DateTime<Utc>,
        message: String,
        uncertain: bool,
    ) -> InvocationResult {
        let completed_at = Utc::now();
        InvocationResult {
            invocation_id,
            status: if uncertain {
                InvocationStatus::Uncertain
            } else {
                InvocationStatus::Failed
            },
            started_at,
            completed_at,
            execution_time_ms: completed_at
                .signed_duration_since(started_at)
                .num_milliseconds()
                .max(0) as u64,
            outputs: HashMap::new(),
            error: Some(ExecutionError {
                error_type: super::ErrorType::SystemError,
                message,
                details: Some(serde_json::json!({ "persistenceFailure": true })),
                recoverable: false,
            }),
            retryable: false,
            recovery: None,
        }
    }

    fn runtime_unavailable_failure(
        invocation_id: String,
        started_at: DateTime<Utc>,
        message: String,
    ) -> InvocationResult {
        let completed_at = Utc::now();
        InvocationResult {
            invocation_id,
            status: InvocationStatus::Failed,
            started_at,
            completed_at,
            execution_time_ms: completed_at
                .signed_duration_since(started_at)
                .num_milliseconds()
                .max(0) as u64,
            outputs: HashMap::new(),
            error: Some(ExecutionError {
                error_type: super::ErrorType::SystemError,
                message,
                details: Some(serde_json::json!({ "runtimeReadinessFailure": true })),
                recoverable: true,
            }),
            retryable: true,
            recovery: None,
        }
    }

    fn normalize(
        invocation_id: String,
        started_at: DateTime<Utc>,
        completed_at: DateTime<Utc>,
        result: ExecutionResult,
        capability: Option<&CapabilityDefinition>,
    ) -> InvocationResult {
        let provider_status = result
            .outputs
            .get("invocation")
            .and_then(|value| value.get("status"))
            .and_then(Value::as_str);
        let timed_out = result
            .error
            .as_ref()
            .map(|error| {
                matches!(error.error_type, super::ErrorType::Timeout)
                    || error.message.to_ascii_lowercase().contains("timed out")
            })
            .unwrap_or(false);
        let status = if result.success {
            match provider_status {
                Some("transport_accepted") => InvocationStatus::TransportAccepted,
                Some("partial") => InvocationStatus::Partial,
                _ => InvocationStatus::Applied,
            }
        } else if timed_out {
            if matches!(
                capability.map(|capability| &capability.contract.receipt),
                Some(ActionReceiptSemantics::Applied { .. })
            ) {
                // The provider may have applied the side effect, but its correlated
                // receipt did not arrive. Retrying blindly would be unsafe.
                InvocationStatus::Uncertain
            } else {
                InvocationStatus::TimedOut
            }
        } else {
            InvocationStatus::Failed
        };
        let retryable = result
            .error
            .as_ref()
            .map(|error| error.recoverable)
            .unwrap_or(false);

        InvocationResult {
            invocation_id,
            status,
            started_at,
            completed_at,
            execution_time_ms: result.execution_time_ms,
            outputs: result.outputs,
            error: result.error,
            retryable,
            recovery: capability.map(|capability| capability.contract.recovery),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{ExecutionQueueState, QueuedExecution};
    use super::*;
    use crate::services::execution::{
        ButtonExecutionMode, ErrorType, ExecutionError, RetryConfig, SequenceMode, TriggerSource,
    };
    use diesel::r2d2::{ConnectionManager, Pool};
    use diesel::SqliteConnection;
    use diesel_migrations::MigrationHarness;
    use std::collections::VecDeque;
    use tokio::sync::oneshot;

    fn test_database() -> (tempfile::TempDir, Arc<Database>) {
        let directory = tempfile::tempdir().expect("temporary invocation database");
        let path = directory.path().join("invocations.db");
        let manager = ConnectionManager::<SqliteConnection>::new(path.to_string_lossy());
        let pool = Pool::builder()
            .max_size(2)
            .build(manager)
            .expect("invocation database pool");
        {
            let mut connection = pool.get().expect("invocation database connection");
            connection
                .run_pending_migrations(crate::db::db::MIGRATIONS)
                .expect("invocation database migrations");
        }
        (directory, Arc::new(Database::new(&pool)))
    }

    fn queue_context(invocation_id: &str) -> ExecutionContext {
        ExecutionContext {
            trigger_source: TriggerSource::Button {
                button_id: "button-1".into(),
                deck_id: "deck-1".into(),
            },
            user_id: None,
            session_id: "durable-queue-test".into(),
            variables: HashMap::new(),
            timeout_ms: None,
            retry_config: RetryConfig {
                max_retries: 0,
                retry_delay_ms: 0,
                exponential_backoff: false,
            },
            execution_mode: ButtonExecutionMode::Queue,
            execution_config: Some(serde_json::json!({
                "invocationId": invocation_id,
                "maxDepth": 8,
            })),
        }
    }

    #[test]
    fn http_history_omits_credentials_and_payloads_inside_composites() {
        let action = UniversalAction::Sequence {
            actions: vec![UniversalAction::HttpRequest {
                name: "Webhook".into(),
                method: "POST".into(),
                url: "https://example.com/private-path?token=seeded-url-secret".into(),
                headers: HashMap::from([(
                    "Authorization".into(),
                    "Bearer seeded-header-secret".into(),
                )]),
                body: Some(serde_json::json!({"password": "seeded-body-secret"})),
                timeout_ms: 1000,
                accepted_status_codes: None,
            }],
            execution_mode: super::super::SequenceMode::Sequential,
        };
        let snapshot = ActionInvocationService::action_snapshot(&action)
            .unwrap()
            .to_string();
        assert!(ActionInvocationService::contains_http(&action));
        assert!(!snapshot.contains("seeded-"));
        assert!(!snapshot.contains("private-path"));
        assert!(snapshot.contains("https://example.com"));
        assert!(snapshot.contains("bodyOmitted"));
    }

    #[test]
    fn normalization_does_not_upgrade_transport_acceptance_to_applied() {
        let result = ExecutionResult {
            success: true,
            outputs: HashMap::from([(
                "invocation".into(),
                serde_json::json!({ "status": "transport_accepted" }),
            )]),
            error: None,
            execution_time_ms: 3,
            retry_count: 0,
            logs: vec![],
        };
        let normalized =
            ActionInvocationService::normalize("test".into(), Utc::now(), Utc::now(), result, None);
        assert_eq!(normalized.status, InvocationStatus::TransportAccepted);
        assert_eq!(
            ActionInvocationService::dispatch_status_name(&normalized.status),
            "transport_accepted"
        );
    }

    #[test]
    fn normalization_preserves_partial_multi_target_outcomes() {
        let result = ExecutionResult {
            success: true,
            outputs: HashMap::from([(
                "invocation".into(),
                serde_json::json!({ "status": "partial" }),
            )]),
            error: None,
            execution_time_ms: 4,
            retry_count: 0,
            logs: vec![],
        };
        let normalized =
            ActionInvocationService::normalize("test".into(), Utc::now(), Utc::now(), result, None);
        assert_eq!(normalized.status, InvocationStatus::Partial);
        assert!(normalized.succeeded());
    }

    #[test]
    fn timeout_is_an_explicit_result_state() {
        let result = ExecutionResult {
            success: false,
            outputs: HashMap::new(),
            error: Some(ExecutionError {
                error_type: ErrorType::Timeout,
                message: "Timed out".into(),
                details: None,
                recoverable: true,
            }),
            execution_time_ms: 10,
            retry_count: 0,
            logs: vec![],
        };
        let normalized =
            ActionInvocationService::normalize("test".into(), Utc::now(), Utc::now(), result, None);
        assert_eq!(normalized.status, InvocationStatus::TimedOut);
        assert!(normalized.retryable);
    }

    #[tokio::test]
    async fn queued_terminal_success_and_failure_are_persisted_for_original_callers() {
        let (_directory, database) = test_database();
        let engine = Arc::new(
            UniversalExecutionEngine::new(
                Arc::new(crate::core::state_manager::StateManager::new()),
            )
            .with_database(database.clone()),
        );
        let readiness = Arc::new(ActionRuntimeReadiness::new());
        readiness.mark_ready(vec![]);
        let service = ActionInvocationService::new(engine, database.clone(), readiness);

        let successful = UniversalAction::Sequence {
            actions: vec![],
            execution_mode: SequenceMode::Sequential,
        };
        let success = service
            .invoke_resolved_action(&successful, &queue_context("queue-success"))
            .await;
        assert_eq!(success.invocation_id, "queue-success");
        assert_eq!(success.status, InvocationStatus::Applied);
        let persisted_success = database
            .get_action_invocation("queue-success")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(persisted_success.dispatch_status, "succeeded");
        assert_eq!(
            persisted_success.normalized_status.as_deref(),
            Some("applied")
        );
        assert!(persisted_success.completed_at.is_some());

        let failing = UniversalAction::Plugin {
            plugin_id: "test.plugin".into(),
            action_id: "fail".into(),
            parameters: HashMap::new(),
        };
        let failure = service
            .invoke_resolved_action(&failing, &queue_context("queue-failure"))
            .await;
        assert_eq!(failure.invocation_id, "queue-failure");
        assert_eq!(failure.status, InvocationStatus::Failed);
        let persisted_failure = database
            .get_action_invocation("queue-failure")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(persisted_failure.dispatch_status, "failed");
        assert_eq!(
            persisted_failure.normalized_status.as_deref(),
            Some("failed")
        );
        assert!(persisted_failure.error.is_some());
    }

    #[tokio::test]
    async fn blocked_queue_survives_caller_cancellation_and_persists_terminal_result() {
        let (_directory, database) = test_database();
        let engine = UniversalExecutionEngine::new(Arc::new(
            crate::core::state_manager::StateManager::new(),
        ))
        .with_database(database.clone());
        let (blocker_admission_tx, blocker_admission_rx) = oneshot::channel();
        let (blocker_completion_tx, blocker_completion_rx) = oneshot::channel();
        let successful = UniversalAction::Sequence {
            actions: vec![],
            execution_mode: SequenceMode::Sequential,
        };
        let mut blocker_context = queue_context("queue-blocker");
        blocker_context.execution_mode = ButtonExecutionMode::RunOnce;
        {
            let mut queues = engine.execution_queues.lock().await;
            queues.insert(
                "button-1".into(),
                ExecutionQueueState {
                    worker_running: true,
                    pending: VecDeque::from([QueuedExecution {
                        action: successful.clone(),
                        context: blocker_context,
                        admission: blocker_admission_rx,
                        completion: blocker_completion_tx,
                    }]),
                },
            );
        }
        let worker = engine.clone();
        tokio::spawn(async move {
            worker.drain_execution_queue("button-1".into()).await;
        });

        let readiness = Arc::new(ActionRuntimeReadiness::new());
        readiness.mark_ready(vec![]);
        let service = Arc::new(ActionInvocationService::new(
            Arc::new(engine),
            database.clone(),
            readiness,
        ));
        let pending_call = {
            let service = service.clone();
            tokio::spawn(async move {
                service
                    .invoke_resolved_action(&successful, &queue_context("queue-pending"))
                    .await
            })
        };

        let pending_record = tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                if let Some(record) = database
                    .get_action_invocation("queue-pending")
                    .await
                    .unwrap()
                {
                    if record.dispatch_status == "queued" {
                        break record;
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("queued phase must become observable");
        assert!(pending_record.normalized_status.is_none());
        assert!(pending_record.completed_at.is_none());
        assert!(!pending_call.is_finished());

        service.close_admission().await;
        assert!(service
            .submit_resolved_action(
                &UniversalAction::Sequence {
                    actions: vec![],
                    execution_mode: SequenceMode::Sequential,
                },
                &queue_context("after-shutdown")
            )
            .await
            .is_err());
        assert!(database
            .get_action_invocation("after-shutdown")
            .await
            .unwrap()
            .is_none());
        assert!(tokio::time::timeout(
            std::time::Duration::from_millis(20),
            service.wait_for_idle()
        )
        .await
        .is_err());

        pending_call.abort();
        assert!(pending_call.await.unwrap_err().is_cancelled());

        blocker_admission_tx.send(Ok(())).unwrap();
        assert!(blocker_completion_rx.await.unwrap().success);
        tokio::time::timeout(std::time::Duration::from_secs(2), service.wait_for_idle())
            .await
            .expect("shutdown joins the accepted invocation even after its caller is gone");
        let persisted = tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                let record = database
                    .get_action_invocation("queue-pending")
                    .await
                    .unwrap()
                    .unwrap();
                if record.completed_at.is_some() {
                    break record;
                }
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("detached accepted invocation must reach terminal persistence");
        assert_eq!(persisted.dispatch_status, "succeeded");
        assert_eq!(persisted.normalized_status.as_deref(), Some("applied"));
        assert!(persisted.completed_at.is_some());
    }
}
