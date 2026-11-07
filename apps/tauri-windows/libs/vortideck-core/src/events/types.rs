use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::core::SystemInfo;
use crate::services::flow::executor::{ExecutionHistoryEntry, FlowExecutionState};

/// Versioned, transport-neutral metadata for workflow input and audit records.
///
/// `EventPayload` remains the low-latency EventBus message type. This envelope
/// is the canonical representation to persist, correlate, replay, and pass to
/// workflow triggers without coupling them to Tokio or a connector transport.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventEnvelope {
    pub event_id: String,
    pub event_type: String,
    pub source: String,
    pub schema_version: u16,
    pub occurred_at: DateTime<Utc>,
    pub received_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub causation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
    pub payload: Value,
}

/// Address for a workflow-originated overlay command. The desktop bridge may
/// broadcast the transport message, while the overlay host enforces the target
/// before forwarding it into widget iframes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum OverlayTarget {
    All,
    Widget {
        #[serde(rename = "widgetId")]
        widget_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum OverlayAcknowledgementMode {
    /// The authenticated overlay host accepted and forwarded the command.
    #[default]
    HostAccepted,
    /// Every targeted community-widget iframe explicitly reported that its
    /// visual update was committed. Unsupported widgets time out fail-closed.
    WidgetRendered,
}

/// Versioned command contract shared by workflows, the overlay bridge, and
/// widgets. Delivery remains live; command identity and expiry make retry and
/// acknowledgement support possible without changing this wire shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayCommandPayload {
    pub command_id: String,
    pub schema_version: u16,
    pub delivery_revision: u64,
    pub target: OverlayTarget,
    pub command: String,
    pub payload: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow_run_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    pub requires_ack: bool,
    #[serde(default)]
    pub acknowledgement_mode: OverlayAcknowledgementMode,
}

/// Receipt sent by the authenticated overlay host after it has accepted a
/// command and forwarded it to its eligible widget frame(s). This deliberately
/// does not claim that arbitrary third-party widget code finished rendering.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OverlayCommandAcknowledgement {
    pub command_id: String,
    pub status: OverlayCommandAckStatus,
    #[serde(default)]
    pub receipt_phase: OverlayCommandReceiptPhase,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applied_revision: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// The strongest lifecycle fact observed by the authenticated overlay host.
/// Older hosts omit this field and deserialize as `HostAccepted`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum OverlayCommandReceiptPhase {
    #[default]
    HostAccepted,
    WidgetRendered,
    PlaybackStarted,
    PlaybackCompleted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum OverlayCommandAckStatus {
    Accepted,
    Rejected,
}

/// Persisted delivery lifecycle for commands that must survive an overlay
/// reconnect. Acknowledged/rejected rows are retained as an audit trail.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum OverlayCommandDeliveryStatus {
    Pending,
    Acknowledged,
    Rejected,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayCommandDelivery {
    pub command: OverlayCommandPayload,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invocation_id: Option<String>,
    pub run_id: String,
    pub node_id: String,
    pub status: OverlayCommandDeliveryStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acknowledgement: Option<OverlayCommandAcknowledgement>,
}

/// Durable record around an externally visible workflow action. The executor
/// writes this before dispatching; an interrupted dispatch is intentionally
/// left for recovery rather than replayed blindly.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SideEffectOutboxEntry {
    pub idempotency_key: String,
    /// Links the workflow outbox claim to the canonical cross-surface invocation record.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invocation_id: Option<String>,
    pub run_id: String,
    pub node_id: String,
    pub action: Value,
    pub status: SideEffectOutboxStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SideEffectOutboxStatus {
    Dispatching,
    Succeeded,
    Failed,
    /// Dispatch may have reached the provider, but no durable applied receipt
    /// proves the external outcome. Operator reconciliation is required unless
    /// the declared recovery contract safely permits the same attempt.
    Uncertain,
}

impl SideEffectOutboxStatus {
    /// Project a canonical terminal invocation receipt onto the workflow
    /// outbox. `None` means the invocation is still non-terminal or unknown.
    pub fn from_normalized_invocation(status: &str) -> Option<Self> {
        match status {
            "applied" | "partial" | "skipped" => Some(Self::Succeeded),
            "failed" | "timed_out" => Some(Self::Failed),
            "transport_accepted" | "uncertain" => Some(Self::Uncertain),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowRuntimeInspection {
    pub recovery_runs: usize,
    pub pending_overlay_commands: Vec<WorkflowOverlayDeliverySummary>,
    pub side_effects: Vec<WorkflowSideEffectTrace>,
    pub metrics: WorkflowRuntimeMetrics,
    pub dead_letters: Vec<WorkflowDeadLetterTrace>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowOverlayDeliverySummary {
    pub command: WorkflowOverlayCommandSummary,
    pub run_id: String,
    pub node_id: String,
    pub status: OverlayCommandDeliveryStatus,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowOverlayCommandSummary {
    pub command_id: String,
    pub command: String,
    pub delivery_revision: u64,
}

/// Redacted operator trace for one accepted source event. Payloads, resolved
/// actions, provider outputs and free-form errors are intentionally excluded.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowRuntimeTrace {
    pub event: WorkflowEventTrace,
    pub runs: Vec<WorkflowRunTrace>,
    pub side_effects: Vec<WorkflowSideEffectTrace>,
    pub invocations: Vec<WorkflowInvocationTrace>,
    pub dead_letters: Vec<WorkflowDeadLetterTrace>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowEventTrace {
    pub event_id: String,
    pub event_type: String,
    pub source: String,
    pub schema_version: u16,
    pub occurred_at: DateTime<Utc>,
    pub received_at: DateTime<Utc>,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowRunTrace {
    pub run_id: String,
    pub flow_id: String,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowSideEffectTrace {
    pub idempotency_key: String,
    pub invocation_id: Option<String>,
    pub run_id: String,
    pub node_id: String,
    pub status: SideEffectOutboxStatus,
    pub has_error: bool,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowInvocationTrace {
    pub invocation_id: String,
    pub run_id: Option<String>,
    pub node_id: Option<String>,
    pub integration_id: Option<String>,
    pub action_id: Option<String>,
    pub dispatch_status: String,
    pub normalized_status: Option<String>,
    pub has_error: bool,
    pub started_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowDeadLetterTrace {
    pub id: String,
    pub flow_id: String,
    pub node_id: String,
    pub attempts: u32,
    pub resolved_at: Option<DateTime<Utc>>,
    pub has_reason: bool,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowDeadLetter {
    pub id: String,
    pub event_id: String,
    pub flow_id: String,
    pub node_id: String,
    pub reason: String,
    pub attempts: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

/// Durable, process-restart-safe runtime counters. Live transport counters
/// (connected sockets, EventBus lag) belong to their owning adapters.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowRuntimeMetrics {
    pub canonical_events: u64,
    pub pending_overlay_commands: u64,
    pub acknowledged_overlay_commands: u64,
    pub rejected_overlay_commands: u64,
    pub expired_overlay_commands: u64,
    pub dispatching_side_effects: u64,
    pub succeeded_side_effects: u64,
    pub failed_side_effects: u64,
    pub uncertain_side_effects: u64,
    pub unresolved_dead_letters: u64,
    pub open_workflow_wakes: u64,
    pub waiting_approvals: u64,
    pub active_resource_leases: u64,
    pub incompatible_runtime_records: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oldest_dispatching_side_effect_at: Option<DateTime<Utc>>,
}

impl WorkflowRuntimeMetrics {
    /// Prometheus text exposition without a global metrics recorder. Values are
    /// read from the durable runtime database, so a process restart does not
    /// reset operational counters to a misleading zero.
    pub fn to_prometheus_text(&self) -> String {
        let values = [
            ("canonical_events", self.canonical_events),
            ("pending_overlay_commands", self.pending_overlay_commands),
            (
                "acknowledged_overlay_commands",
                self.acknowledged_overlay_commands,
            ),
            ("rejected_overlay_commands", self.rejected_overlay_commands),
            ("expired_overlay_commands", self.expired_overlay_commands),
            ("dispatching_side_effects", self.dispatching_side_effects),
            ("succeeded_side_effects", self.succeeded_side_effects),
            ("failed_side_effects", self.failed_side_effects),
            ("uncertain_side_effects", self.uncertain_side_effects),
            ("unresolved_dead_letters", self.unresolved_dead_letters),
            ("open_workflow_wakes", self.open_workflow_wakes),
            ("waiting_approvals", self.waiting_approvals),
            ("active_resource_leases", self.active_resource_leases),
            (
                "incompatible_runtime_records",
                self.incompatible_runtime_records,
            ),
        ];
        let mut output = String::new();
        for (name, value) in values {
            output.push_str(&format!(
                "# TYPE vortideck_workflow_{} gauge\nvortideck_workflow_{} {}\n",
                name, name, value
            ));
        }
        if let Some(timestamp) = self.oldest_dispatching_side_effect_at {
            output.push_str("# TYPE vortideck_workflow_oldest_dispatching_side_effect_timestamp_seconds gauge\n");
            output.push_str(&format!(
                "vortideck_workflow_oldest_dispatching_side_effect_timestamp_seconds {}\n",
                timestamp.timestamp()
            ));
        }
        output
    }
}

/// Internal runtime coordination notifications carried over the live bus.
/// These are typed so routing infrastructure does not depend on magic strings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum RuntimeEventPayload {
    TriggerRegistryChanged {
        #[serde(rename = "flowId")]
        flow_id: String,
    },
}

/// Host-stamped event emitted by an installed package. The guest supplies only
/// the local event name and payload; it cannot select another event category or
/// package principal.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginEventPayload {
    pub package_id: String,
    pub event_type: String,
    pub payload: Value,
}

/// Unified event payload for all event types in VortiDeck
/// This is the single event type used by EventBus
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "category", content = "payload", rename_all = "camelCase")]
pub enum EventPayload {
    /// Flow execution events
    Flow(FlowEventPayload),
    /// System-level events (volume, system info, etc.)
    System(SystemEventPayload),
    /// Webhook events (received, sent, etc.)
    Webhook(WebhookEventPayload),
    /// Relay events (commands from mobile via backend relay)
    Relay(RelayEventPayload),
    /// Stream events (for standard platforms: Twitch, YouTube, Kick)
    Stream(StreamEventPayload),
    /// Versioned workflow commands delivered to overlay widgets.
    Overlay(OverlayCommandPayload),
    /// Coordination events for runtime infrastructure.
    Runtime(RuntimeEventPayload),
    /// Event whose package principal was stamped by the WASM host.
    Plugin(PluginEventPayload),
    /// Session events (creation, deletion, activation, status changes)
    Session(SessionEventPayload),
    /// Auth events (login, logout, token saved/cleared)
    Auth(AuthEventPayload),
    /// Generic / Schemaless events (for custom webhooks, Ko-fi, new integrations, or plugins)
    Generic {
        source: String,
        #[serde(rename = "eventType")]
        event_type: String,
        payload: serde_json::Value,
    },
}

/// Canonical flow event type used internally and on the EventBus.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum FlowEventPayload {
    FlowStarted {
        #[serde(rename = "flowId")]
        flow_id: String,
        #[serde(rename = "runId")]
        run_id: String,
        #[serde(rename = "flowName")]
        flow_name: String,
    },
    NodeStarted {
        #[serde(rename = "flowId")]
        flow_id: String,
        #[serde(rename = "runId")]
        run_id: String,
        #[serde(rename = "nodeId")]
        node_id: String,
        #[serde(rename = "nodeName")]
        node_name: String,
    },
    NodeCompleted {
        #[serde(rename = "flowId")]
        flow_id: String,
        #[serde(rename = "runId")]
        run_id: String,
        #[serde(rename = "nodeId")]
        node_id: String,
        result: crate::services::flow::executor::NodeResult,
    },
    NodeProgress {
        #[serde(rename = "flowId")]
        flow_id: String,
        #[serde(rename = "runId")]
        run_id: String,
        #[serde(rename = "nodeId")]
        node_id: String,
        progress: f64,
        message: String,
    },
    NodeFailed {
        #[serde(rename = "flowId")]
        flow_id: String,
        #[serde(rename = "runId")]
        run_id: String,
        #[serde(rename = "nodeId")]
        node_id: String,
        error: String,
    },
    FlowCompleted {
        #[serde(rename = "flowId")]
        flow_id: String,
        #[serde(rename = "runId")]
        run_id: String,
        result: HashMap<String, Value>,
    },
    FlowFailed {
        #[serde(rename = "flowId")]
        flow_id: String,
        #[serde(rename = "runId")]
        run_id: String,
        error: String,
    },
    FlowCancelled {
        #[serde(rename = "flowId")]
        flow_id: String,
        #[serde(rename = "runId")]
        run_id: String,
    },
    FlowProgressUpdate {
        #[serde(rename = "flowId")]
        flow_id: String,
        #[serde(rename = "runId")]
        run_id: String,
        #[serde(rename = "progressPercentage")]
        progress_percentage: f64,
        #[serde(rename = "reachableNodes")]
        reachable_nodes: u32,
        #[serde(rename = "completedNodes")]
        completed_nodes: u32,
        #[serde(rename = "currentNode")]
        current_node: Option<String>,
    },
    ExecutionStateChanged {
        #[serde(rename = "flowId")]
        flow_id: String,
        #[serde(rename = "runId")]
        run_id: String,
        #[serde(rename = "executionState")]
        execution_state: FlowExecutionState,
    },
    ExecutionHistoryAdded {
        #[serde(rename = "flowId")]
        flow_id: String,
        #[serde(rename = "runId")]
        run_id: String,
        #[serde(rename = "historyEntry")]
        history_entry: ExecutionHistoryEntry,
    },
    ApprovalRequested {
        #[serde(rename = "flowId")]
        flow_id: String,
        #[serde(rename = "runId")]
        run_id: String,
        request: crate::services::flow::executor::WorkflowWake,
    },
    ApprovalResolved {
        #[serde(rename = "flowId")]
        flow_id: String,
        #[serde(rename = "runId")]
        run_id: String,
        #[serde(rename = "wakeId")]
        wake_id: String,
        resolution: crate::services::flow::executor::ApprovalResolution,
    },
    // Webhook response event (for ResponseNode mode)
    WebhookResponseReady {
        #[serde(rename = "flowId")]
        flow_id: String,
        #[serde(rename = "runId")]
        run_id: String,
        #[serde(rename = "nodeId")]
        node_id: String,
        #[serde(rename = "responseData")]
        response_data: Value,
    },
}

impl FlowEventPayload {
    pub(crate) fn run_id(&self) -> &str {
        match self {
            Self::FlowStarted { run_id, .. }
            | Self::NodeStarted { run_id, .. }
            | Self::NodeCompleted { run_id, .. }
            | Self::NodeProgress { run_id, .. }
            | Self::NodeFailed { run_id, .. }
            | Self::FlowCompleted { run_id, .. }
            | Self::FlowFailed { run_id, .. }
            | Self::FlowCancelled { run_id, .. }
            | Self::FlowProgressUpdate { run_id, .. }
            | Self::ExecutionStateChanged { run_id, .. }
            | Self::ExecutionHistoryAdded { run_id, .. }
            | Self::ApprovalRequested { run_id, .. }
            | Self::ApprovalResolved { run_id, .. }
            | Self::WebhookResponseReady { run_id, .. } => run_id,
        }
    }

    pub(crate) fn set_flow_id(&mut self, source_flow_id: String) {
        match self {
            Self::FlowStarted { flow_id, .. }
            | Self::NodeStarted { flow_id, .. }
            | Self::NodeCompleted { flow_id, .. }
            | Self::NodeProgress { flow_id, .. }
            | Self::NodeFailed { flow_id, .. }
            | Self::FlowCompleted { flow_id, .. }
            | Self::FlowFailed { flow_id, .. }
            | Self::FlowCancelled { flow_id, .. }
            | Self::FlowProgressUpdate { flow_id, .. }
            | Self::ExecutionStateChanged { flow_id, .. }
            | Self::ExecutionHistoryAdded { flow_id, .. }
            | Self::ApprovalRequested { flow_id, .. }
            | Self::ApprovalResolved { flow_id, .. }
            | Self::WebhookResponseReady { flow_id, .. } => *flow_id = source_flow_id,
        }
    }
}

/// System event types (volume, system info, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SystemEventPayload {
    /// Volume changed event
    VolumeChanged {
        volume: f32,
        button_id: Option<String>,
        timestamp: i64,
    },
    /// System information retrieved event
    SystemInfoRetrieved { system_info: SystemInfo },
}

/// Webhook event types (for routing and real-time UI updates)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum WebhookEventPayload {
    /// Webhook request received
    WebhookReceived {
        webhook_id: String,
        timestamp: i64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        event_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        service: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        flow_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        payload: Option<Value>,
        #[serde(default)]
        is_test: bool,
    },
}

/// Relay event types (commands and status updates from backend relay)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum RelayEventPayload {
    /// Relay connection status changed
    StatusChanged {
        status: String, // "connecting", "registered", "disconnected", "error"
        desktop_id: Option<String>,
        relay_session_id: Option<String>,
        error: Option<String>,
    },
    /// Command received from mobile client via relay
    CommandReceived {
        client_id: String,
        connection_session_id: String,
        payload: Value,
    },
    /// Mobile client connected to relay
    MobileConnected { client_id: String },
    /// Mobile client disconnected from relay
    MobileDisconnected { client_id: String },
}

/// Metadata shared by every normalized stream event. `flatten` keeps the
/// existing JSON wire contract while avoiding eight copies of these fields in
/// Rust and giving routing code one metadata accessor.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamEventMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub occurred_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream_id: Option<String>,
    pub account_id: u64,
    pub platform: String,
}

impl StreamEventMetadata {
    pub fn new(account_id: u64, platform: impl Into<String>) -> Self {
        Self {
            event_id: None,
            occurred_at: None,
            stream_id: None,
            account_id,
            platform: platform.into(),
        }
    }
}

/// Stream event types from platform relay (Twitch, YouTube, Kick, TikTok).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum StreamEventPayload {
    Follow {
        #[serde(flatten)]
        metadata: StreamEventMetadata,
        #[serde(rename = "userName")]
        user_name: String,
        #[serde(rename = "userDisplayName")]
        user_display_name: String,
    },
    Subscription {
        #[serde(flatten)]
        metadata: StreamEventMetadata,
        #[serde(rename = "userName")]
        user_name: String,
        tier: Option<String>,
        message: Option<String>,
    },
    Raid {
        #[serde(flatten)]
        metadata: StreamEventMetadata,
        raider: String,
        #[serde(rename = "viewerCount")]
        viewer_count: Option<u32>,
    },
    Bits {
        #[serde(flatten)]
        metadata: StreamEventMetadata,
        #[serde(rename = "userName")]
        user_name: String,
        amount: u32,
        message: Option<String>,
    },
    ChatMessage {
        #[serde(flatten)]
        metadata: StreamEventMetadata,
        #[serde(rename = "userName")]
        user_name: String,
        #[serde(rename = "userDisplayName")]
        user_display_name: String,
        message: String,
        #[serde(rename = "isModerator")]
        is_moderator: bool,
        #[serde(rename = "isSubscriber")]
        is_subscriber: bool,
        #[serde(rename = "platformColor", skip_serializing_if = "Option::is_none")]
        platform_color: Option<String>,
    },
    StreamOnline {
        #[serde(flatten)]
        metadata: StreamEventMetadata,
        title: Option<String>,
    },
    StreamOffline {
        #[serde(flatten)]
        metadata: StreamEventMetadata,
    },
    ViewerCountUpdated {
        #[serde(flatten)]
        metadata: StreamEventMetadata,
        count: u32,
    },
}

impl StreamEventPayload {
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::Follow { .. } => "follow",
            Self::Subscription { .. } => "subscription",
            Self::Raid { .. } => "raid",
            Self::Bits { .. } => "bits",
            Self::ChatMessage { .. } => "chat_message",
            Self::StreamOnline { .. } => "stream_online",
            Self::StreamOffline { .. } => "stream_offline",
            Self::ViewerCountUpdated { .. } => "viewer_count_updated",
        }
    }

    pub fn metadata(&self) -> &StreamEventMetadata {
        match self {
            Self::Follow { metadata, .. }
            | Self::Subscription { metadata, .. }
            | Self::Raid { metadata, .. }
            | Self::Bits { metadata, .. }
            | Self::ChatMessage { metadata, .. }
            | Self::StreamOnline { metadata, .. }
            | Self::StreamOffline { metadata }
            | Self::ViewerCountUpdated { metadata, .. } => metadata,
        }
    }
}

/// Session event types (creation, deletion, activation, status changes)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SessionEventPayload {
    /// Session created
    SessionCreated {
        #[serde(rename = "sessionId")]
        session_id: String,
    },
    /// Session updated
    SessionUpdated {
        #[serde(rename = "sessionId")]
        session_id: String,
    },
    /// Session deleted
    SessionDeleted {
        #[serde(rename = "sessionId")]
        session_id: String,
    },
    /// Session activated
    SessionActivated {
        #[serde(rename = "sessionId")]
        session_id: String,
        #[serde(rename = "accountIds")]
        account_ids: Vec<u64>,
    },
    /// Session deactivated
    SessionDeactivated {
        #[serde(rename = "sessionId")]
        session_id: String,
    },
    /// General session state change
    SessionStateChanged,
}

/// Auth event types (token saved, token cleared)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AuthEventPayload {
    /// Token saved
    TokenSaved,
    /// Token cleared
    TokenCleared,
}
