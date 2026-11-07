//! Durably accepts consequential stream events before projecting them onto the
//! shared live EventBus, and routes accepted envelopes into active flows.

use std::future::Future;
use std::{collections::HashMap, sync::Arc};

use chrono::Utc;
use serde_json::{json, Value};
use tokio::{sync::RwLock, task::JoinHandle};
use uuid::Uuid;

use crate::events::{
    EventBus, EventEnvelope, EventPayload, PluginEventPayload, RuntimeEventPayload,
    StreamEventPayload,
};
use crate::services::flow::executor::{
    ActiveTriggerDefinition, FlowExecutor, TriggerEvent, TriggerSource,
};

pub struct EventRouter {
    event_bus: Arc<EventBus>,
    ingress: DurableStreamEventIngress,
}

impl EventRouter {
    pub fn new(event_bus: Arc<EventBus>, executor: FlowExecutor) -> Self {
        Self {
            event_bus,
            ingress: DurableStreamEventIngress::new(executor),
        }
    }

    pub fn with_ingress(event_bus: Arc<EventBus>, ingress: DurableStreamEventIngress) -> Self {
        Self { event_bus, ingress }
    }

    /// Starts an EventBus subscriber which stays alive after a lag warning.
    pub fn spawn(self) -> JoinHandle<()> {
        tokio::spawn(self.run())
    }

    pub async fn run(self) {
        self.run_until(std::future::pending()).await;
    }

    /// Run until the event bus closes or the owning runtime requests shutdown.
    /// The shutdown future belongs to the runtime lifecycle; dropping an
    /// arbitrary task handle is not treated as a graceful stop.
    pub async fn run_until<F>(self, shutdown: F)
    where
        F: Future<Output = ()>,
    {
        let mut receiver = self.event_bus.subscribe();
        tokio::pin!(shutdown);
        if let Err(error) = self.ingress.refresh_registry().await {
            log::error!("EventRouter could not refresh triggers: {}", error);
        }
        log::info!("EventRouter started: EventBus -> active flow triggers");
        loop {
            let received = tokio::select! {
                _ = &mut shutdown => {
                    log::info!("EventRouter stopped by runtime lifecycle");
                    break;
                }
                received = receiver.recv() => received,
            };
            match received {
                Ok(EventPayload::Stream(payload)) => {
                    if let Err(error) = self.ingress.accept(payload).await {
                        log::error!("EventRouter rejected stream ingress: {}", error);
                    }
                }
                Ok(EventPayload::Plugin(payload)) => {
                    self.ingress.record_plugin_event(payload).await
                }
                Ok(EventPayload::Runtime(RuntimeEventPayload::TriggerRegistryChanged {
                    ..
                })) => {
                    if let Err(error) = self.ingress.refresh_registry().await {
                        log::error!("EventRouter could not refresh triggers: {}", error);
                    }
                }
                Ok(_) => {}
                Err(tokio::sync::broadcast::error::RecvError::Lagged(count)) => {
                    log::warn!("EventRouter lagged by {} events; continuing", count);
                    if let Err(error) = self.ingress.refresh_registry().await {
                        log::error!("EventRouter could not refresh triggers: {}", error);
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    }
}

#[derive(Clone, Default)]
pub struct DurableStreamEventIngressState {
    ingress: Arc<RwLock<Option<DurableStreamEventIngress>>>,
}

impl DurableStreamEventIngressState {
    pub async fn install(&self, ingress: DurableStreamEventIngress) {
        *self.ingress.write().await = Some(ingress);
    }

    pub async fn clear(&self) {
        *self.ingress.write().await = None;
    }

    pub async fn accept(
        &self,
        payload: StreamEventPayload,
    ) -> Result<StreamIngressReceipt, String> {
        let ingress = self
            .ingress
            .read()
            .await
            .clone()
            .ok_or("Durable workflow stream ingress is not ready")?;
        ingress.accept(payload).await
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamIngressDisposition {
    ObservedOnly,
    Accepted,
    AcceptedExpired,
    Duplicate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamIngressReceipt {
    pub event_id: String,
    pub matched_routes: usize,
    pub disposition: StreamIngressDisposition,
}

#[derive(Clone)]
pub struct DurableStreamEventIngress {
    executor: FlowExecutor,
    route_index: Arc<RwLock<TriggerRouteIndex>>,
}

impl DurableStreamEventIngress {
    const MAX_TRIGGER_EVENT_AGE: chrono::Duration = chrono::Duration::minutes(5);

    pub fn new(executor: FlowExecutor) -> Self {
        Self {
            executor,
            route_index: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn refresh_registry(&self) -> Result<usize, String> {
        let triggers = self.executor.list_active_triggers().await?;
        let count = triggers.len();
        *self.route_index.write().await = index_active_triggers(triggers);
        log::info!("EventRouter loaded {} active trigger route(s)", count);
        Ok(count)
    }

    async fn record_plugin_event(&self, event: PluginEventPayload) {
        let received_at = Utc::now();
        let envelope = EventEnvelope {
            event_id: Uuid::new_v4().to_string(),
            event_type: format!("plugin.{}.{}", event.package_id, event.event_type),
            source: format!("plugin:{}", event.package_id),
            schema_version: 1,
            occurred_at: received_at,
            received_at,
            account_id: None,
            session_id: None,
            stream_id: None,
            correlation_id: None,
            causation_id: None,
            idempotency_key: None,
            payload: event.payload,
        };
        if let Err(error) = self.executor.record_event_envelope(&envelope).await {
            log::error!("Could not persist host-stamped plugin event: {}", error);
        }
    }

    pub async fn accept(
        &self,
        payload: StreamEventPayload,
    ) -> Result<StreamIngressReceipt, String> {
        let event_type = payload.event_type();
        let metadata = payload.metadata();
        let platform = metadata.platform.clone();
        let event_data = serde_json::to_value(&payload).unwrap_or(Value::Null);
        let source_event_id = metadata.event_id.clone();
        let event_id = source_event_id
            .as_ref()
            .map(|source_id| format!("{}:{}:{}", platform, event_type, source_id))
            .or_else(|| derived_stream_event_id(&platform, event_type, metadata, &event_data))
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let envelope = EventEnvelope {
            event_id,
            event_type: format!("stream.{}", event_type),
            source: platform.clone(),
            schema_version: 1,
            occurred_at: metadata.occurred_at.unwrap_or_else(Utc::now),
            received_at: Utc::now(),
            account_id: Some(metadata.account_id),
            session_id: None,
            stream_id: metadata.stream_id.clone(),
            correlation_id: None,
            causation_id: None,
            idempotency_key: source_event_id
                .map(|source_id| format!("stream:{}:{}:{}", platform, event_type, source_id)),
            payload: event_data.clone(),
        };

        let route_index = self.route_index.read().await;
        let triggers = matching_stream_triggers(&*route_index, &platform, event_type);
        drop(route_index);
        if triggers.is_empty()
            && crate::services::streaming::stream_values::observed_increment(&envelope).is_none()
        {
            return Ok(StreamIngressReceipt {
                event_id: envelope.event_id,
                matched_routes: 0,
                disposition: StreamIngressDisposition::ObservedOnly,
            });
        }

        // Consequential workflow inputs are durable before any live fan-out or
        // side effect. A storage failure is a rejection, never an accepted event.
        if !self.executor.record_event_envelope(&envelope).await? {
            return Ok(StreamIngressReceipt {
                event_id: envelope.event_id,
                matched_routes: triggers.len(),
                disposition: StreamIngressDisposition::Duplicate,
            });
        }

        let expired = envelope
            .received_at
            .signed_duration_since(envelope.occurred_at)
            > Self::MAX_TRIGGER_EVENT_AGE;
        if expired {
            for trigger in &triggers {
                self.executor
                    .record_workflow_dead_letter(
                        &envelope,
                        &trigger.flow_id,
                        &trigger.node_id,
                        "Stream event expired before workflow ingress".to_string(),
                    )
                    .await?;
            }
            return Ok(StreamIngressReceipt {
                event_id: envelope.event_id,
                matched_routes: triggers.len(),
                disposition: StreamIngressDisposition::AcceptedExpired,
            });
        }

        let matched_routes = triggers.len();
        for trigger in triggers {
            let trigger_event = TriggerEvent {
                source: TriggerSource::Backend,
                event_id: Some(envelope.event_id.clone()),
                correlation_id: envelope.correlation_id.clone(),
                flow_id: trigger.flow_id.clone(),
                node_id: trigger.node_id.clone(),
                data: json!({
                    "event": envelope.clone(),
                }),
                timestamp: envelope.occurred_at,
                dedup_key: envelope
                    .idempotency_key
                    .as_ref()
                    .map(|key| format!("{}:{}:{}", trigger.flow_id, trigger.node_id, key)),
            };

            match self.executor.trigger_flow_from_event(trigger_event).await {
                Ok(run_id) => log::info!(
                    "EventRouter started run {} from {}:{}",
                    run_id,
                    platform,
                    event_type
                ),
                Err(error) if error == "Duplicate event filtered" => log::debug!(
                    "EventRouter ignored duplicate {}:{} for flow {}",
                    platform,
                    event_type,
                    trigger.flow_id
                ),
                Err(error) => {
                    log::warn!(
                        "EventRouter failed to trigger flow {} from {}:{}: {}",
                        trigger.flow_id,
                        platform,
                        event_type,
                        error
                    );
                    if let Err(store_error) = self
                        .executor
                        .record_workflow_dead_letter(
                            &envelope,
                            &trigger.flow_id,
                            &trigger.node_id,
                            error,
                        )
                        .await
                    {
                        log::error!("EventRouter failed to persist dead letter: {}", store_error);
                    }
                }
            }
        }
        Ok(StreamIngressReceipt {
            event_id: envelope.event_id,
            matched_routes,
            disposition: StreamIngressDisposition::Accepted,
        })
    }
}

/// Deterministic identity for a stream event without a provider id.
///
/// The relay hands the same payload to direct ingress and to the EventBus,
/// whose EventRouter accepts it again; both must resolve to one inbox row.
/// Only timestamped payloads are hashed: without `occurred_at`, two genuinely
/// separate events (e.g. two equal viewer counts) would be indistinguishable,
/// so those keep a fresh id as before.
fn derived_stream_event_id(
    platform: &str,
    event_type: &str,
    metadata: &crate::events::StreamEventMetadata,
    event_data: &Value,
) -> Option<String> {
    use sha2::{Digest, Sha256};

    let occurred_at = metadata.occurred_at?;
    let mut hasher = Sha256::new();
    for part in [
        platform.to_string(),
        metadata.account_id.to_string(),
        metadata.stream_id.clone().unwrap_or_default(),
        event_type.to_string(),
        occurred_at.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true),
        event_data.to_string(),
    ] {
        hasher.update(part.as_bytes());
        hasher.update([0u8]);
    }
    Some(format!(
        "{}:{}:derived:{}",
        platform,
        event_type,
        hex::encode(hasher.finalize())
    ))
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct TriggerRouteKey {
    platform: String,
    event_type: String,
}

type TriggerRouteIndex = HashMap<TriggerRouteKey, Vec<ActiveTriggerDefinition>>;

fn index_active_triggers(triggers: Vec<ActiveTriggerDefinition>) -> TriggerRouteIndex {
    let mut index = HashMap::new();
    for trigger in triggers {
        if trigger.node_type != "stream-trigger" {
            continue;
        }
        let Some(platform) = canonical_platform(trigger.platform.as_deref()) else {
            continue;
        };
        let Some(event_type) = trigger.event_type.as_deref().map(canonical_event_name) else {
            continue;
        };
        index
            .entry(TriggerRouteKey {
                platform,
                event_type,
            })
            .or_insert_with(Vec::new)
            .push(trigger);
    }
    index
}

fn matching_stream_triggers(
    index: &TriggerRouteIndex,
    platform: &str,
    event_type: &str,
) -> Vec<ActiveTriggerDefinition> {
    let platform = platform.trim().to_ascii_lowercase();
    let event_type = canonical_event_name(event_type);
    [
        TriggerRouteKey {
            platform: platform.clone(),
            event_type: event_type.clone(),
        },
        TriggerRouteKey {
            platform: platform.clone(),
            event_type: "*".to_string(),
        },
        TriggerRouteKey {
            platform: "*".to_string(),
            event_type: event_type.clone(),
        },
        TriggerRouteKey {
            platform: "*".to_string(),
            event_type: "*".to_string(),
        },
    ]
    .into_iter()
    .filter_map(|key| index.get(&key))
    .flatten()
    .cloned()
    .collect()
}

fn canonical_platform(configured: Option<&str>) -> Option<String> {
    configured
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| match value.to_ascii_lowercase().as_str() {
            "any" | "custom" | "stream" | "*" => "*".to_string(),
            platform => platform.to_string(),
        })
}

fn canonical_event_name(value: &str) -> String {
    match value
        .trim()
        .to_ascii_lowercase()
        .replace(['.', '-', ' '], "_")
        .as_str()
    {
        "channel_raid" => "raid".to_string(),
        "channel_follow" => "follow".to_string(),
        "channel_subscribe" | "subscribe" | "sub" => "subscription".to_string(),
        "chat" | "message" => "chat_message".to_string(),
        "stream_online" | "online" => "stream_online".to_string(),
        "stream_offline" | "offline" => "stream_offline".to_string(),
        "viewer_count" | "viewers" => "viewer_count_updated".to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_supported_stream_event_aliases() {
        assert_eq!(canonical_event_name("channel.raid"), "raid");
        assert_eq!(canonical_event_name("stream.online"), "stream_online");
        assert_eq!(canonical_event_name("chat-message"), "chat_message");
    }

    #[test]
    fn preserves_connector_identity_for_durable_deduplication() {
        let occurred_at = Utc::now();
        let payload = StreamEventPayload::ChatMessage {
            metadata: crate::events::StreamEventMetadata {
                event_id: Some("message-123".to_string()),
                occurred_at: Some(occurred_at),
                stream_id: Some("stream-456".to_string()),
                account_id: 42,
                platform: "twitch".to_string(),
            },
            user_name: "viewer".to_string(),
            user_display_name: "Viewer".to_string(),
            message: "hello".to_string(),
            is_moderator: false,
            is_subscriber: true,
            platform_color: None,
        };

        assert_eq!(payload.event_type(), "chat_message");
        assert_eq!(payload.metadata().event_id.as_deref(), Some("message-123"));
        assert_eq!(payload.metadata().occurred_at, Some(occurred_at));
        assert_eq!(payload.metadata().stream_id.as_deref(), Some("stream-456"));
    }

    #[test]
    fn indexes_exact_and_wildcard_trigger_routes() {
        let trigger = |flow_id: &str, platform: &str, event_type: &str| ActiveTriggerDefinition {
            flow_id: flow_id.to_string(),
            node_id: format!("{flow_id}-trigger"),
            node_type: "stream-trigger".to_string(),
            platform: Some(platform.to_string()),
            event_type: Some(event_type.to_string()),
        };
        let mut webhook_notification = trigger("ignored-webhook", "twitch", "raid");
        webhook_notification.node_type = "webhook-trigger".to_string();
        let index = index_active_triggers(vec![
            trigger("exact", "twitch", "channel.raid"),
            trigger("all-platforms", "any", "raid"),
            trigger("all-events", "twitch", "*"),
            webhook_notification,
        ]);

        let matches = matching_stream_triggers(&index, "Twitch", "raid");
        let mut flow_ids: Vec<_> = matches.into_iter().map(|item| item.flow_id).collect();
        flow_ids.sort();
        assert_eq!(flow_ids, vec!["all-events", "all-platforms", "exact"]);
    }
}
