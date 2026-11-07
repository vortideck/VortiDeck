// Event system module - provides platform-agnostic event broadcasting
pub mod bus;
pub mod types;

pub use bus::EventBus;
pub use types::{
    AuthEventPayload, EventEnvelope, EventPayload, FlowEventPayload, OverlayAcknowledgementMode,
    OverlayCommandAckStatus, OverlayCommandAcknowledgement, OverlayCommandDelivery,
    OverlayCommandDeliveryStatus, OverlayCommandPayload, OverlayCommandReceiptPhase, OverlayTarget,
    PluginEventPayload, RelayEventPayload, RuntimeEventPayload, SessionEventPayload,
    SideEffectOutboxEntry, SideEffectOutboxStatus, StreamEventMetadata, StreamEventPayload,
    SystemEventPayload, WebhookEventPayload, WorkflowDeadLetter, WorkflowDeadLetterTrace,
    WorkflowEventTrace, WorkflowInvocationTrace, WorkflowOverlayCommandSummary,
    WorkflowOverlayDeliverySummary, WorkflowRunTrace, WorkflowRuntimeInspection,
    WorkflowRuntimeMetrics, WorkflowRuntimeTrace, WorkflowSideEffectTrace,
};
