//! First-party overlay, alert, and media connector.
//!
//! Generic widget commands retain transport-level semantics. Alert and media
//! actions enter a persisted FIFO queue and complete only after the authenticated
//! overlay host reports correlated playback lifecycle receipts.

use crate::db::{Database, NewOverlayMediaQueueRecord, OverlayMediaQueueRecord};
use crate::events::{
    EventBus, EventPayload, OverlayAcknowledgementMode, OverlayCommandAckStatus,
    OverlayCommandAcknowledgement, OverlayCommandPayload, OverlayCommandReceiptPhase,
    OverlayTarget,
};
use crate::integrations::{
    ActionContract, ActionReceiptSemantics, ActionRecoverySemantics, AppIntegration,
    CapabilityDefinition, CapabilityKind, CapabilityOption, CapabilityOutput, CapabilityParameter,
    CapabilityPresentation, CapabilityWidget, ContractValueType, EventCallback, IntegrationAction,
    IntegrationActionResult, IntegrationActionStatus, IntegrationError, IntegrationId,
    IntegrationMetadata, IntegrationRequirements, IntegrationResult,
};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::sync::{mpsc, oneshot, Mutex, Notify};

#[derive(Debug)]
struct OverlayQueueCompletion {
    acknowledgement: OverlayCommandAcknowledgement,
    playback_started: bool,
}

struct OverlayMediaQueue {
    event_bus: Arc<EventBus>,
    database: Arc<Database>,
    pending_receipts: Mutex<HashMap<String, mpsc::UnboundedSender<OverlayCommandAcknowledgement>>>,
    waiting_invocations:
        Mutex<HashMap<String, oneshot::Sender<Result<OverlayQueueCompletion, String>>>>,
    alert_notify: Arc<Notify>,
    media_notify: Arc<Notify>,
    host_notify: Arc<Notify>,
    started: AtomicBool,
    connected_hosts: Arc<std::sync::atomic::AtomicUsize>,
}

impl OverlayMediaQueue {
    fn new(
        event_bus: Arc<EventBus>,
        database: Arc<Database>,
        connected_hosts: Arc<std::sync::atomic::AtomicUsize>,
    ) -> Arc<Self> {
        Arc::new(Self {
            event_bus,
            database,
            pending_receipts: Mutex::new(HashMap::new()),
            waiting_invocations: Mutex::new(HashMap::new()),
            alert_notify: Arc::new(Notify::new()),
            media_notify: Arc::new(Notify::new()),
            host_notify: Arc::new(Notify::new()),
            started: AtomicBool::new(false),
            connected_hosts,
        })
    }

    async fn start(self: &Arc<Self>) -> Result<(), String> {
        if self
            .started
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Ok(());
        }
        let interrupted = match self.database.interrupt_dispatched_overlay_media().await {
            Ok(interrupted) => interrupted,
            Err(error) => {
                // Initialization may be retried after a transient database
                // failure. Never leave the queue marked started without workers.
                self.started.store(false, Ordering::Release);
                return Err(error);
            }
        };
        if interrupted > 0 {
            log::warn!(
                "Marked {} interrupted overlay playback item(s) for manual review",
                interrupted
            );
        }
        for kind in ["alert", "media"] {
            let queue = self.clone();
            let kind = kind.to_string();
            tokio::spawn(async move { queue.run_worker(kind).await });
        }
        self.alert_notify.notify_one();
        self.media_notify.notify_one();
        Ok(())
    }

    fn notify_for(&self, kind: &str) -> &Notify {
        if kind == "alert" {
            &self.alert_notify
        } else {
            &self.media_notify
        }
    }

    async fn enqueue_and_wait(
        &self,
        invocation_id: &str,
        kind: &str,
        command: &str,
        payload: Value,
        timeout_ms: u64,
    ) -> Result<OverlayQueueCompletion, String> {
        let item_id = uuid::Uuid::new_v4().to_string();
        let serialized = serde_json::to_string(&payload)
            .map_err(|error| format!("Unable to serialize overlay queue payload: {}", error))?;
        let now = chrono::Utc::now().to_rfc3339();
        let (sender, receiver) = oneshot::channel();
        self.waiting_invocations
            .lock()
            .await
            .insert(item_id.clone(), sender);
        if let Err(error) = self
            .database
            .enqueue_overlay_media(NewOverlayMediaQueueRecord {
                id: &item_id,
                invocation_id,
                queue_kind: kind,
                command,
                payload: &serialized,
                status: "queued",
                receipt_phase: None,
                provider_receipt_id: None,
                error: None,
                created_at: &now,
                updated_at: &now,
                started_at: None,
                completed_at: None,
            })
            .await
        {
            self.waiting_invocations.lock().await.remove(&item_id);
            return Err(error);
        }
        self.notify_for(kind).notify_one();
        match tokio::time::timeout(
            std::time::Duration::from_millis(timeout_ms.saturating_add(15_000)),
            receiver,
        )
        .await
        {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err("Overlay queue worker stopped before completion".to_string()),
            Err(_) => {
                self.waiting_invocations.lock().await.remove(&item_id);
                Err(format!(
                    "Timed out waiting for queued overlay playback '{}'",
                    item_id
                ))
            }
        }
    }

    async fn acknowledge(&self, acknowledgement: OverlayCommandAcknowledgement) -> bool {
        if acknowledgement
            .instance_id
            .as_deref()
            .is_none_or(str::is_empty)
            || (acknowledgement.status == OverlayCommandAckStatus::Accepted
                && acknowledgement.applied_revision != Some(1))
        {
            return false;
        }
        self.pending_receipts
            .lock()
            .await
            .get(&acknowledgement.command_id)
            .is_some_and(|sender| sender.send(acknowledgement).is_ok())
    }

    async fn run_worker(self: Arc<Self>, kind: String) {
        loop {
            let notified = self.notify_for(&kind).notified();
            let host_notified = self.host_notify.notified();
            if self.connected_hosts.load(Ordering::Acquire) == 0 {
                host_notified.await;
                continue;
            }
            match self.database.claim_next_overlay_media(&kind).await {
                Ok(Some(item)) => self.process_item(item).await,
                Ok(None) => notified.await,
                Err(error) => {
                    log::error!("Overlay {} queue worker failed: {}", kind, error);
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
        }
    }

    async fn process_item(&self, item: OverlayMediaQueueRecord) {
        if self.connected_hosts.load(Ordering::Acquire) == 0 {
            if let Err(error) = self.database.requeue_overlay_media(&item.id).await {
                self.finish_item(&item, Err(error)).await;
            }
            return;
        }
        let payload = match serde_json::from_str::<Value>(&item.payload) {
            Ok(payload) => payload,
            Err(error) => {
                self.finish_item(&item, Err(format!("Invalid persisted payload: {}", error)))
                    .await;
                return;
            }
        };
        let timeout_ms = payload
            .get("maxDurationMs")
            .and_then(Value::as_u64)
            .unwrap_or(60_000)
            .clamp(1_000, 300_000);
        let (receipt_sender, mut receipt_receiver) = mpsc::unbounded_channel();
        self.pending_receipts
            .lock()
            .await
            .insert(item.id.clone(), receipt_sender);
        let emitted = self
            .event_bus
            .emit(EventPayload::Overlay(OverlayCommandPayload {
                command_id: item.id.clone(),
                schema_version: 1,
                delivery_revision: 1,
                target: OverlayTarget::All,
                command: item.command.clone(),
                payload,
                workflow_run_id: None,
                event_id: None,
                expires_at: Some(
                    chrono::Utc::now()
                        + chrono::Duration::milliseconds(timeout_ms.saturating_add(15_000) as i64),
                ),
                requires_ack: true,
                acknowledgement_mode: OverlayAcknowledgementMode::HostAccepted,
            }));
        if emitted.as_ref().is_err() || emitted.as_ref().is_ok_and(|count| *count == 0) {
            self.pending_receipts.lock().await.remove(&item.id);
            self.finish_item(
                &item,
                Err(emitted
                    .err()
                    .map(|error| error.to_string())
                    .unwrap_or_else(|| "No overlay host is listening".to_string())),
            )
            .await;
            return;
        }

        let outcome = tokio::time::timeout(
            std::time::Duration::from_millis(timeout_ms.saturating_add(10_000)),
            async {
                let mut playback_started = false;
                while let Some(receipt) = receipt_receiver.recv().await {
                    if receipt.status == OverlayCommandAckStatus::Rejected {
                        return Err(receipt
                            .error
                            .clone()
                            .unwrap_or_else(|| "Overlay host rejected playback".to_string()));
                    }
                    match receipt.receipt_phase {
                        OverlayCommandReceiptPhase::PlaybackStarted => playback_started = true,
                        OverlayCommandReceiptPhase::PlaybackCompleted => {
                            if !playback_started {
                                return Err(
                                    "Overlay reported playback completion before playback start"
                                        .to_string(),
                                );
                            }
                            return Ok(OverlayQueueCompletion {
                                acknowledgement: receipt,
                                playback_started,
                            });
                        }
                        _ => {}
                    }
                }
                Err("Overlay receipt channel closed".to_string())
            },
        )
        .await
        .unwrap_or_else(|_| Err("Overlay playback completion timed out".to_string()));
        self.pending_receipts.lock().await.remove(&item.id);
        self.finish_item(&item, outcome).await;
    }

    async fn finish_item(
        &self,
        item: &OverlayMediaQueueRecord,
        outcome: Result<OverlayQueueCompletion, String>,
    ) {
        let persistence = match &outcome {
            Ok(completion) => {
                let receipt_id = format!(
                    "{}:{}",
                    completion
                        .acknowledgement
                        .instance_id
                        .as_deref()
                        .unwrap_or("overlay-host"),
                    item.id
                );
                self.database
                    .complete_overlay_media(
                        &item.id,
                        "completed",
                        Some("playback_completed"),
                        Some(&receipt_id),
                        None,
                    )
                    .await
            }
            Err(error) => {
                self.database
                    .complete_overlay_media(&item.id, "failed", None, None, Some(error))
                    .await
            }
        };
        let outcome = persistence.map_or_else(Err, |_| outcome);
        if let Some(waiter) = self.waiting_invocations.lock().await.remove(&item.id) {
            let _ = waiter.send(outcome);
        }
    }
}

pub struct OverlayIntegration {
    id: IntegrationId,
    event_bus: Arc<EventBus>,
    media_queue: Option<Arc<OverlayMediaQueue>>,
    connected_hosts: Arc<std::sync::atomic::AtomicUsize>,
}

impl OverlayIntegration {
    pub fn new(event_bus: Arc<EventBus>) -> Self {
        Self {
            id: IntegrationId::new("overlay"),
            event_bus,
            media_queue: None,
            connected_hosts: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }

    pub fn with_database(mut self, database: Arc<Database>) -> Self {
        self.media_queue = Some(OverlayMediaQueue::new(
            self.event_bus.clone(),
            database,
            self.connected_hosts.clone(),
        ));
        self
    }

    pub fn host_connected(&self) {
        self.connected_hosts.fetch_add(1, Ordering::AcqRel);
        if let Some(queue) = &self.media_queue {
            queue.host_notify.notify_waiters();
            queue.alert_notify.notify_one();
            queue.media_notify.notify_one();
        }
    }

    pub fn host_disconnected(&self) {
        // Rust 1.99 renamed fetch_update to try_update; keep fetch_update while the
        // declared minimum Rust version is older.
        #[allow(deprecated)]
        let _ = self
            .connected_hosts
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                Some(count.saturating_sub(1))
            });
    }

    pub async fn start(&self) -> Result<(), String> {
        match &self.media_queue {
            Some(queue) => queue.start().await,
            None => Ok(()),
        }
    }

    pub async fn acknowledge(&self, acknowledgement: OverlayCommandAcknowledgement) -> bool {
        match &self.media_queue {
            Some(queue) => queue.acknowledge(acknowledgement).await,
            None => false,
        }
    }

    fn dispatch_capability() -> CapabilityDefinition {
        CapabilityDefinition {
            integration_id: "overlay".to_string(),
            capability_id: "dispatch_command".to_string(),
            version: 1,
            kind: CapabilityKind::Action,
            presentation: CapabilityPresentation {
                name: "Dispatch overlay command".to_string(),
                description: "Send a versioned command to all overlays or one widget".to_string(),
                category: "Overlay".to_string(),
                icon: Some("lucide:panels-top-left".to_string()),
                aliases: vec!["widget".to_string(), "visual".to_string()],
            },
            inputs: vec![
                CapabilityParameter {
                    id: "command".to_string(),
                    label: "Command".to_string(),
                    description: "Command understood by the target widget".to_string(),
                    value_type: ContractValueType::String,
                    required: true,
                    widget: CapabilityWidget::Text,
                    default_value: None,
                    options: vec![],
                    option_source: None,
                },
                CapabilityParameter {
                    id: "widgetId".to_string(),
                    label: "Widget ID".to_string(),
                    description: "Leave empty to broadcast to all overlay widgets".to_string(),
                    value_type: ContractValueType::String,
                    required: false,
                    widget: CapabilityWidget::Text,
                    default_value: None,
                    options: vec![],
                    option_source: None,
                },
                CapabilityParameter {
                    id: "payload".to_string(),
                    label: "Payload".to_string(),
                    description: "JSON data delivered with the command".to_string(),
                    value_type: ContractValueType::Any,
                    required: false,
                    widget: CapabilityWidget::Json,
                    default_value: Some(json!({})),
                    options: vec![],
                    option_source: None,
                },
            ],
            outputs: vec![
                CapabilityOutput {
                    id: "commandId".to_string(),
                    label: "Command ID".to_string(),
                    value_type: ContractValueType::String,
                },
                CapabilityOutput {
                    id: "recipients".to_string(),
                    label: "Local recipients".to_string(),
                    value_type: ContractValueType::Integer,
                },
            ],
            allow_additional_inputs: false,
            contract: ActionContract {
                integration_id: "overlay".to_string(),
                action_id: "dispatch_command".to_string(),
                version: 1,
                required_permission: crate::plugins::bundle::PluginPermission::TriggerAction(
                    "integration:overlay:dispatch_command".to_string(),
                ),
                timeout_ms: 5_000,
                recovery: ActionRecoverySemantics::RetryWithIdempotency,
                resource_key: Some("overlay:all".to_string()),
                receipt: ActionReceiptSemantics::TransportAccepted,
            },
        }
    }

    fn parameter(
        id: &str,
        label: &str,
        value_type: ContractValueType,
        required: bool,
        widget: CapabilityWidget,
        default_value: Option<Value>,
    ) -> CapabilityParameter {
        CapabilityParameter {
            id: id.to_string(),
            label: label.to_string(),
            description: label.to_string(),
            value_type,
            required,
            widget,
            default_value,
            options: vec![],
            option_source: None,
        }
    }

    fn select_parameter(
        id: &str,
        label: &str,
        values: &[&str],
        default: &str,
    ) -> CapabilityParameter {
        let mut parameter = Self::parameter(
            id,
            label,
            ContractValueType::String,
            true,
            CapabilityWidget::Select,
            Some(json!(default)),
        );
        parameter.options = values
            .iter()
            .map(|value| CapabilityOption {
                label: value.to_string(),
                value: json!(value),
                parameters: vec![],
            })
            .collect();
        parameter
    }

    fn generated_capability(action_id: &str) -> IntegrationResult<CapabilityDefinition> {
        let (name, description, inputs, queued) = match action_id {
            "show_widget" => (
                "Show overlay widget",
                "Show one overlay widget",
                vec![
                    Self::parameter(
                        "widgetId",
                        "Widget ID",
                        ContractValueType::String,
                        true,
                        CapabilityWidget::Text,
                        None,
                    ),
                    Self::parameter(
                        "payload",
                        "Widget data",
                        ContractValueType::Any,
                        false,
                        CapabilityWidget::Json,
                        Some(json!({})),
                    ),
                ],
                false,
            ),
            "hide_widget" => (
                "Hide overlay widget",
                "Hide one overlay widget",
                vec![Self::parameter(
                    "widgetId",
                    "Widget ID",
                    ContractValueType::String,
                    true,
                    CapabilityWidget::Text,
                    None,
                )],
                false,
            ),
            "update_widget" => (
                "Update overlay widget",
                "Update data for one overlay widget",
                vec![
                    Self::parameter(
                        "widgetId",
                        "Widget ID",
                        ContractValueType::String,
                        true,
                        CapabilityWidget::Text,
                        None,
                    ),
                    Self::parameter(
                        "payload",
                        "Widget data",
                        ContractValueType::Any,
                        true,
                        CapabilityWidget::Json,
                        Some(json!({})),
                    ),
                ],
                false,
            ),
            "show_alert" => (
                "Queue overlay alert",
                "Queue a timed alert and wait for its visual playback to complete",
                vec![
                    Self::parameter(
                        "title",
                        "Title",
                        ContractValueType::String,
                        true,
                        CapabilityWidget::Text,
                        None,
                    ),
                    Self::parameter(
                        "message",
                        "Message",
                        ContractValueType::String,
                        false,
                        CapabilityWidget::Textarea,
                        None,
                    ),
                    Self::parameter(
                        "durationMs",
                        "Duration (ms)",
                        ContractValueType::Integer,
                        false,
                        CapabilityWidget::Number,
                        Some(json!(5000)),
                    ),
                    Self::parameter(
                        "mediaUrl",
                        "Optional HTTPS media URL",
                        ContractValueType::String,
                        false,
                        CapabilityWidget::Text,
                        None,
                    ),
                ],
                true,
            ),
            "play_media" => (
                "Queue overlay media",
                "Queue HTTPS audio, video, or image media and wait for completion",
                vec![
                    Self::parameter(
                        "url",
                        "HTTPS media URL",
                        ContractValueType::String,
                        true,
                        CapabilityWidget::Text,
                        None,
                    ),
                    Self::select_parameter(
                        "mediaType",
                        "Media type",
                        &["auto", "video", "audio", "image"],
                        "auto",
                    ),
                    Self::parameter(
                        "volume",
                        "Volume (0-1)",
                        ContractValueType::Number,
                        false,
                        CapabilityWidget::Number,
                        Some(json!(1.0)),
                    ),
                    Self::parameter(
                        "maxDurationMs",
                        "Maximum duration (ms)",
                        ContractValueType::Integer,
                        false,
                        CapabilityWidget::Number,
                        Some(json!(60000)),
                    ),
                ],
                true,
            ),
            "ingest_discord_media" => (
                "Moderate Discord media",
                "Queue an approved Discord attachment or URL for overlay playback",
                vec![
                    Self::parameter(
                        "url",
                        "Attachment or media URL",
                        ContractValueType::String,
                        true,
                        CapabilityWidget::Text,
                        None,
                    ),
                    Self::parameter(
                        "contentType",
                        "Content type",
                        ContractValueType::String,
                        true,
                        CapabilityWidget::Text,
                        None,
                    ),
                    Self::parameter(
                        "sizeBytes",
                        "Size in bytes",
                        ContractValueType::Integer,
                        true,
                        CapabilityWidget::Number,
                        None,
                    ),
                    Self::select_parameter(
                        "moderationDecision",
                        "Moderation decision",
                        &["approved", "rejected"],
                        "rejected",
                    ),
                    Self::parameter(
                        "authorId",
                        "Discord author ID",
                        ContractValueType::String,
                        false,
                        CapabilityWidget::Text,
                        None,
                    ),
                    Self::parameter(
                        "channelId",
                        "Discord channel ID",
                        ContractValueType::String,
                        false,
                        CapabilityWidget::Text,
                        None,
                    ),
                    Self::parameter(
                        "maxDurationMs",
                        "Maximum duration (ms)",
                        ContractValueType::Integer,
                        false,
                        CapabilityWidget::Number,
                        Some(json!(60000)),
                    ),
                ],
                true,
            ),
            _ => return Err(IntegrationError::UnsupportedAction(action_id.to_string())),
        };
        Ok(CapabilityDefinition {
            integration_id: "overlay".to_string(),
            capability_id: action_id.to_string(),
            version: 1,
            kind: CapabilityKind::Action,
            presentation: CapabilityPresentation {
                name: name.to_string(),
                description: description.to_string(),
                category: if queued { "Overlay media" } else { "Overlay" }.to_string(),
                icon: Some(
                    if queued {
                        "lucide:play"
                    } else {
                        "lucide:panel-top"
                    }
                    .to_string(),
                ),
                aliases: vec!["overlay".to_string(), "stream".to_string()],
            },
            inputs,
            outputs: if queued {
                vec![
                    CapabilityOutput {
                        id: "queueItemId".to_string(),
                        label: "Queue item ID".to_string(),
                        value_type: ContractValueType::String,
                    },
                    CapabilityOutput {
                        id: "playbackStarted".to_string(),
                        label: "Playback started".to_string(),
                        value_type: ContractValueType::Boolean,
                    },
                    CapabilityOutput {
                        id: "playbackCompleted".to_string(),
                        label: "Playback completed".to_string(),
                        value_type: ContractValueType::Boolean,
                    },
                    CapabilityOutput {
                        id: "receiptPhase".to_string(),
                        label: "Strongest receipt phase".to_string(),
                        value_type: ContractValueType::String,
                    },
                ]
            } else {
                Self::dispatch_capability().outputs
            },
            allow_additional_inputs: false,
            contract: ActionContract {
                integration_id: "overlay".to_string(),
                action_id: action_id.to_string(),
                version: 1,
                required_permission: crate::plugins::bundle::PluginPermission::TriggerAction(
                    format!("integration:overlay:{}", action_id),
                ),
                timeout_ms: if queued { 330_000 } else { 5_000 },
                recovery: if queued {
                    ActionRecoverySemantics::ManualReview
                } else {
                    ActionRecoverySemantics::RetryWithIdempotency
                },
                resource_key: Some(
                    if queued {
                        if action_id == "show_alert" {
                            "overlay:queue:alert"
                        } else {
                            "overlay:queue:media"
                        }
                    } else {
                        "overlay:all"
                    }
                    .to_string(),
                ),
                receipt: if queued {
                    ActionReceiptSemantics::Applied {
                        timeout_ms: 330_000,
                    }
                } else {
                    ActionReceiptSemantics::TransportAccepted
                },
            },
        })
    }

    fn validate_media_url(raw: &str) -> IntegrationResult<url::Url> {
        let parsed = url::Url::parse(raw).map_err(|error| {
            IntegrationError::InvalidParameter(format!("Invalid URL: {}", error))
        })?;
        if parsed.scheme() != "https" || parsed.host_str().is_none() {
            return Err(IntegrationError::InvalidParameter(
                "Overlay media must use an absolute HTTPS URL".to_string(),
            ));
        }
        let host = parsed.host_str().unwrap_or_default().to_ascii_lowercase();
        if host == "localhost"
            || host.ends_with(".localhost")
            || host.parse::<std::net::IpAddr>().is_ok()
        {
            return Err(IntegrationError::InvalidParameter(
                "Local and numeric hosts are not allowed for overlay media".to_string(),
            ));
        }
        Ok(parsed)
    }

    fn validate_discord_ingestion(parameters: &HashMap<String, Value>) -> IntegrationResult<()> {
        if parameters.get("moderationDecision").and_then(Value::as_str) != Some("approved") {
            return Err(IntegrationError::PermissionDenied(
                "Discord media must be explicitly approved before playback".to_string(),
            ));
        }
        let size = parameters
            .get("sizeBytes")
            .and_then(Value::as_u64)
            .unwrap_or(u64::MAX);
        if size > 100 * 1024 * 1024 {
            return Err(IntegrationError::InvalidParameter(
                "Discord media exceeds the 100 MiB ingestion limit".to_string(),
            ));
        }
        let content_type = parameters
            .get("contentType")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_ascii_lowercase();
        if !["video/", "audio/", "image/"]
            .iter()
            .any(|prefix| content_type.starts_with(prefix))
        {
            return Err(IntegrationError::InvalidParameter(
                "Discord media content type must be video, audio, or image".to_string(),
            ));
        }
        Ok(())
    }

    fn dispatch_live_command(
        &self,
        request_id: String,
        target: OverlayTarget,
        command: String,
        payload: Value,
    ) -> IntegrationResult<IntegrationActionResult> {
        if self.connected_hosts.load(Ordering::Acquire) == 0 {
            return Err(IntegrationError::NotConnected(
                "No authenticated overlay host is connected".to_string(),
            ));
        }
        let recipients = self
            .event_bus
            .emit(EventPayload::Overlay(OverlayCommandPayload {
                command_id: request_id.clone(),
                schema_version: 1,
                delivery_revision: 1,
                target,
                command,
                payload,
                workflow_run_id: None,
                event_id: None,
                expires_at: Some(chrono::Utc::now() + chrono::Duration::seconds(5)),
                requires_ack: false,
                acknowledgement_mode: OverlayAcknowledgementMode::HostAccepted,
            }))
            .map_err(|error| IntegrationError::ExecutionError(error.to_string()))?;
        if recipients == 0 {
            return Err(IntegrationError::NotConnected(
                "No overlay bridge is listening".to_string(),
            ));
        }
        Ok(IntegrationActionResult {
            status: IntegrationActionStatus::TransportAccepted,
            request_id: request_id.clone(),
            provider_receipt_id: None,
            outputs: HashMap::from([
                ("commandId".to_string(), json!(request_id)),
                ("recipients".to_string(), json!(recipients)),
            ]),
        })
    }
}

#[async_trait]
impl AppIntegration for OverlayIntegration {
    fn id(&self) -> &IntegrationId {
        &self.id
    }

    fn name(&self) -> &str {
        "Overlay"
    }

    fn description(&self) -> &str {
        "Dispatch commands to VortiDeck overlay hosts and widgets"
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
        self.connected_hosts.load(Ordering::Acquire) > 0
    }

    async fn execute_action(
        &self,
        action: IntegrationAction,
    ) -> IntegrationResult<IntegrationActionResult> {
        if !self.is_available().await {
            return Err(IntegrationError::NotConnected(
                "No authenticated overlay host is connected".to_string(),
            ));
        }
        self.resolve_capability(&action.name, &action.parameters)
            .await?;

        let request_id = action
            .execution
            .as_ref()
            .and_then(|execution| execution.request_id.clone())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        match action.name.as_str() {
            "dispatch_command" => {
                let target = action
                    .parameters
                    .get("widgetId")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(|widget_id| OverlayTarget::Widget {
                        widget_id: widget_id.to_string(),
                    })
                    .unwrap_or(OverlayTarget::All);
                self.dispatch_live_command(
                    request_id,
                    target,
                    action.parameters["command"]
                        .as_str()
                        .expect("validated command")
                        .to_string(),
                    action
                        .parameters
                        .get("payload")
                        .cloned()
                        .unwrap_or_else(|| json!({})),
                )
            }
            "show_widget" | "hide_widget" | "update_widget" => {
                let widget_id = action.parameters["widgetId"]
                    .as_str()
                    .expect("validated widget ID")
                    .to_string();
                let command = match action.name.as_str() {
                    "show_widget" => "show",
                    "hide_widget" => "hide",
                    _ => "update",
                };
                self.dispatch_live_command(
                    request_id,
                    OverlayTarget::Widget { widget_id },
                    command.to_string(),
                    action
                        .parameters
                        .get("payload")
                        .cloned()
                        .unwrap_or_else(|| json!({})),
                )
            }
            "show_alert" | "play_media" | "ingest_discord_media" => {
                let queue = self.media_queue.as_ref().ok_or_else(|| {
                    IntegrationError::ConfigurationError(
                        "Overlay media queue repository is unavailable".to_string(),
                    )
                })?;
                queue
                    .start()
                    .await
                    .map_err(IntegrationError::ExecutionError)?;
                let (kind, command, mut payload, timeout_ms) = if action.name == "show_alert" {
                    let duration = action
                        .parameters
                        .get("durationMs")
                        .and_then(Value::as_u64)
                        .unwrap_or(5_000)
                        .clamp(500, 60_000);
                    (
                        "alert",
                        "overlay.alert.play",
                        json!({
                            "title": action.parameters["title"],
                            "message": action.parameters.get("message").cloned().unwrap_or(Value::Null),
                            "mediaUrl": action.parameters.get("mediaUrl").cloned().unwrap_or(Value::Null),
                            "durationMs": duration,
                            "maxDurationMs": duration.saturating_add(2_000),
                        }),
                        duration.saturating_add(2_000),
                    )
                } else {
                    let timeout = action
                        .parameters
                        .get("maxDurationMs")
                        .and_then(Value::as_u64)
                        .unwrap_or(60_000)
                        .clamp(1_000, 300_000);
                    let payload = if action.name == "ingest_discord_media" {
                        let content_type = action.parameters["contentType"]
                            .as_str()
                            .expect("validated content type");
                        json!({
                            "url": action.parameters["url"],
                            "mediaType": content_type.split('/').next().unwrap_or("auto"),
                            "volume": 1.0,
                            "maxDurationMs": timeout,
                            "source": "discord",
                            "authorId": action.parameters.get("authorId").cloned().unwrap_or(Value::Null),
                            "channelId": action.parameters.get("channelId").cloned().unwrap_or(Value::Null),
                            "contentType": content_type,
                            "sizeBytes": action.parameters["sizeBytes"],
                        })
                    } else {
                        action
                            .parameters
                            .clone()
                            .into_iter()
                            .collect::<serde_json::Map<_, _>>()
                            .into()
                    };
                    ("media", "overlay.media.play", payload, timeout)
                };
                if payload.get("maxDurationMs").is_none() {
                    payload["maxDurationMs"] = json!(timeout_ms);
                }
                let completion = queue
                    .enqueue_and_wait(&request_id, kind, command, payload, timeout_ms)
                    .await
                    .map_err(IntegrationError::ActionFailed)?;
                let queue_item_id = completion.acknowledgement.command_id.clone();
                let provider_receipt_id = Some(format!(
                    "{}:{}",
                    completion
                        .acknowledgement
                        .instance_id
                        .as_deref()
                        .unwrap_or("overlay-host"),
                    queue_item_id
                ));
                Ok(IntegrationActionResult {
                    status: IntegrationActionStatus::Applied,
                    request_id,
                    provider_receipt_id,
                    outputs: HashMap::from([
                        ("queueItemId".to_string(), json!(queue_item_id)),
                        (
                            "playbackStarted".to_string(),
                            json!(completion.playback_started),
                        ),
                        ("playbackCompleted".to_string(), json!(true)),
                        ("receiptPhase".to_string(), json!("playbackCompleted")),
                    ]),
                })
            }
            unsupported => Err(IntegrationError::UnsupportedAction(unsupported.to_string())),
        }
    }

    async fn resolve_capability(
        &self,
        action_id: &str,
        parameters: &HashMap<String, Value>,
    ) -> IntegrationResult<CapabilityDefinition> {
        let mut capability = if action_id == "dispatch_command" {
            Self::dispatch_capability()
        } else {
            Self::generated_capability(action_id)?
        };
        capability
            .validate_parameters(parameters)
            .map_err(IntegrationError::ValidationError)?;
        if matches!(
            action_id,
            "dispatch_command" | "show_widget" | "hide_widget" | "update_widget"
        ) {
            let target = parameters
                .get("widgetId")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("all");
            capability.contract.resource_key = Some(format!("overlay:{}", target));
        }
        if action_id == "play_media" || action_id == "ingest_discord_media" {
            Self::validate_media_url(
                parameters
                    .get("url")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
            )?;
        }
        if action_id == "show_alert" {
            if let Some(url) = parameters
                .get("mediaUrl")
                .and_then(Value::as_str)
                .filter(|url| !url.trim().is_empty())
            {
                Self::validate_media_url(url)?;
            }
        }
        if action_id == "ingest_discord_media" {
            Self::validate_discord_ingestion(parameters)?;
        }
        if action_id == "play_media" {
            let volume = parameters
                .get("volume")
                .and_then(Value::as_f64)
                .unwrap_or(1.0);
            if !(0.0..=1.0).contains(&volume) {
                return Err(IntegrationError::InvalidParameter(
                    "Media volume must be between 0 and 1".to_string(),
                ));
            }
        }
        if matches!(action_id, "play_media" | "ingest_discord_media") {
            let duration = parameters
                .get("maxDurationMs")
                .and_then(Value::as_u64)
                .unwrap_or(60_000);
            if !(1_000..=300_000).contains(&duration) {
                return Err(IntegrationError::InvalidParameter(
                    "Maximum media duration must be between 1000 and 300000 milliseconds"
                        .to_string(),
                ));
            }
        }
        if action_id == "show_alert" {
            let duration = parameters
                .get("durationMs")
                .and_then(Value::as_u64)
                .unwrap_or(5_000);
            if !(500..=60_000).contains(&duration) {
                return Err(IntegrationError::InvalidParameter(
                    "Alert duration must be between 500 and 60000 milliseconds".to_string(),
                ));
            }
        }
        Ok(capability)
    }

    async fn get_capabilities(&self) -> Vec<CapabilityDefinition> {
        let mut capabilities = vec![Self::dispatch_capability()];
        capabilities.extend(
            [
                "show_widget",
                "hide_widget",
                "update_widget",
                "show_alert",
                "play_media",
                "ingest_discord_media",
            ]
            .into_iter()
            .filter_map(|id| Self::generated_capability(id).ok()),
        );
        capabilities
    }

    async fn subscribe_to_events(
        &self,
        _callback: Box<dyn EventCallback>,
    ) -> IntegrationResult<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::connection::SimpleConnection;
    use diesel::r2d2::{ConnectionManager, Pool};
    use diesel::SqliteConnection;

    #[tokio::test]
    async fn dispatch_reports_transport_acceptance_without_claiming_render() {
        let bus = Arc::new(EventBus::new_default());
        let _receiver = bus.subscribe();
        let integration = OverlayIntegration::new(bus);
        integration.host_connected();
        let result = integration
            .execute_action(IntegrationAction {
                name: "dispatch_command".to_string(),
                parameters: HashMap::from([("command".to_string(), json!("show"))]),
                execution: None,
            })
            .await
            .unwrap();
        assert_eq!(result.status, IntegrationActionStatus::TransportAccepted);
    }

    #[tokio::test]
    async fn publishes_generated_widget_alert_media_and_discord_ingestion_actions() {
        let capabilities = OverlayIntegration::new(Arc::new(EventBus::new_default()))
            .get_capabilities()
            .await;
        assert_eq!(capabilities.len(), 7);
        for id in ["show_alert", "play_media", "ingest_discord_media"] {
            let capability = capabilities
                .iter()
                .find(|capability| capability.capability_id == id)
                .unwrap();
            assert!(matches!(
                capability.contract.receipt,
                ActionReceiptSemantics::Applied { .. }
            ));
        }
    }

    #[tokio::test]
    async fn discord_ingestion_is_deny_by_default() {
        let integration = OverlayIntegration::new(Arc::new(EventBus::new_default()));
        let parameters = HashMap::from([
            (
                "url".to_string(),
                json!("https://cdn.discordapp.com/file.mp4"),
            ),
            ("contentType".to_string(), json!("video/mp4")),
            ("sizeBytes".to_string(), json!(1024)),
            ("moderationDecision".to_string(), json!("rejected")),
        ]);
        assert!(matches!(
            integration
                .resolve_capability("ingest_discord_media", &parameters)
                .await,
            Err(IntegrationError::PermissionDenied(_))
        ));
    }

    #[tokio::test]
    async fn queued_media_requires_started_and_completed_receipts() {
        let manager = ConnectionManager::<SqliteConnection>::new(":memory:");
        let pool = Pool::builder().max_size(2).build(manager).unwrap();
        pool.get()
            .unwrap()
            .batch_execute(include_str!(
                "../../../db/migrations/2026-08-30-070000_create_overlay_media_queue/up.sql"
            ))
            .unwrap();
        let bus = Arc::new(EventBus::new_default());
        let mut receiver = bus.subscribe();
        let integration =
            Arc::new(OverlayIntegration::new(bus).with_database(Arc::new(Database::new(&pool))));
        integration.host_connected();
        integration.start().await.unwrap();
        let action_integration = integration.clone();
        let invocation = tokio::spawn(async move {
            action_integration
                .execute_action(IntegrationAction {
                    name: "play_media".to_string(),
                    parameters: HashMap::from([
                        (
                            "url".to_string(),
                            json!("https://cdn.example.com/video.mp4"),
                        ),
                        ("mediaType".to_string(), json!("video")),
                        ("maxDurationMs".to_string(), json!(2000)),
                    ]),
                    execution: None,
                })
                .await
        });
        let command = match receiver.recv().await.unwrap() {
            EventPayload::Overlay(command) => command,
            _ => panic!("expected overlay command"),
        };
        for receipt_phase in [
            OverlayCommandReceiptPhase::PlaybackStarted,
            OverlayCommandReceiptPhase::PlaybackCompleted,
        ] {
            assert!(
                integration
                    .acknowledge(OverlayCommandAcknowledgement {
                        command_id: command.command_id.clone(),
                        status: OverlayCommandAckStatus::Accepted,
                        receipt_phase,
                        instance_id: Some("overlay-test".to_string()),
                        applied_revision: Some(1),
                        error: None,
                    })
                    .await
            );
        }
        let result = invocation.await.unwrap().unwrap();
        assert_eq!(result.status, IntegrationActionStatus::Applied);
        assert_eq!(result.outputs["playbackStarted"], json!(true));
        assert_eq!(result.outputs["playbackCompleted"], json!(true));
    }
}
