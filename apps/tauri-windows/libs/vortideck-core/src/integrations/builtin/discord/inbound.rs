//! Permission-scoped Discord application-command ingress.
//!
//! Discord transport adapters normalize `INTERACTION_CREATE` into
//! [`DiscordInboundCommand`]. This module owns authorization, age checks,
//! durable deduplication and saved-flow dispatch; it intentionally knows
//! nothing about Gateway heartbeats or HTTP interaction signatures.

use std::collections::HashSet;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::{oneshot, Mutex};
use tokio::task::JoinHandle;

use crate::events::EventEnvelope;
use crate::integrations::interface::EventCallback;
use crate::services::flow::executor::{FlowExecutor, TriggerEvent, TriggerSource};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscordInboundPolicy {
    pub command_name: String,
    pub flow_id: String,
    pub guild_id: String,
    pub channel_id: String,
    pub allowed_user_ids: HashSet<String>,
    pub max_age_seconds: i64,
}

impl DiscordInboundPolicy {
    pub fn validate(&self) -> Result<(), String> {
        if self.command_name.is_empty()
            || !self.command_name.chars().all(|character| {
                character.is_ascii_lowercase()
                    || character.is_ascii_digit()
                    || matches!(character, '-' | '_')
            })
        {
            return Err(
                "Discord command name must use lowercase letters, digits, '-' or '_'".to_string(),
            );
        }
        if self.flow_id.trim().is_empty()
            || self.guild_id.trim().is_empty()
            || self.channel_id.trim().is_empty()
        {
            return Err("Discord inbound policy requires flow, guild and channel IDs".to_string());
        }
        if self.allowed_user_ids.is_empty() {
            return Err("Discord inbound policy requires at least one allowed user".to_string());
        }
        if !(1..=300).contains(&self.max_age_seconds) {
            return Err("Discord command max age must be between 1 and 300 seconds".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscordInboundCommand {
    pub interaction_id: String,
    pub command_name: String,
    pub guild_id: Option<String>,
    pub channel_id: String,
    pub user_id: String,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum DiscordInboundReceipt {
    Accepted { run_id: String },
    Duplicate { run_id: Option<String> },
    Rejected { reason: DiscordRejectionReason },
    Failed { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiscordRejectionReason {
    WrongCommand,
    DirectMessageNotAllowed,
    WrongGuild,
    WrongChannel,
    UserNotAllowed,
    Expired,
    FlowInactive,
}

#[derive(Clone)]
pub struct DiscordInboundProcessor {
    policy: DiscordInboundPolicy,
    executor: FlowExecutor,
}

impl DiscordInboundProcessor {
    pub fn new(policy: DiscordInboundPolicy, executor: FlowExecutor) -> Result<Self, String> {
        policy.validate()?;
        Ok(Self { policy, executor })
    }

    pub fn policy(&self) -> &DiscordInboundPolicy {
        &self.policy
    }

    /// Persist a normalized provider delivery before policy evaluation or
    /// execution. Re-delivery can therefore never launch a second run, while
    /// rejected requests remain available to redacted operator diagnostics.
    pub async fn process(&self, command: DiscordInboundCommand) -> DiscordInboundReceipt {
        let received_at = Utc::now();
        let event_id = format!("discord:{}", command.interaction_id);
        let envelope = EventEnvelope {
            event_id: event_id.clone(),
            event_type: "discord.application_command".to_string(),
            source: "discord".to_string(),
            schema_version: 1,
            occurred_at: command.occurred_at,
            received_at,
            account_id: None,
            session_id: None,
            stream_id: None,
            correlation_id: Some(command.interaction_id.clone()),
            causation_id: None,
            idempotency_key: Some(event_id.clone()),
            payload: json!({
                "commandName": command.command_name,
                "guildId": command.guild_id,
                "channelId": command.channel_id,
                "userId": command.user_id,
            }),
        };

        match self.executor.record_event_envelope(&envelope).await {
            Ok(false) => {
                let run_id = self
                    .executor
                    .trace_workflow_event(&event_id)
                    .await
                    .ok()
                    .flatten()
                    .and_then(|trace| trace.runs.first().map(|run| run.run_id.clone()));
                return DiscordInboundReceipt::Duplicate { run_id };
            }
            Err(message) => return DiscordInboundReceipt::Failed { message },
            Ok(true) => {}
        }

        if command.command_name != self.policy.command_name {
            return rejected(DiscordRejectionReason::WrongCommand);
        }
        let Some(guild_id) = command.guild_id.as_deref() else {
            return rejected(DiscordRejectionReason::DirectMessageNotAllowed);
        };
        if guild_id != self.policy.guild_id {
            return rejected(DiscordRejectionReason::WrongGuild);
        }
        if command.channel_id != self.policy.channel_id {
            return rejected(DiscordRejectionReason::WrongChannel);
        }
        if !self.policy.allowed_user_ids.contains(&command.user_id) {
            return rejected(DiscordRejectionReason::UserNotAllowed);
        }
        let age = received_at.signed_duration_since(command.occurred_at);
        if age.num_seconds() > self.policy.max_age_seconds || age.num_seconds() < -30 {
            return rejected(DiscordRejectionReason::Expired);
        }

        let flow = match self.executor.load_flow(&self.policy.flow_id).await {
            Ok(flow) => flow,
            Err(message) => {
                self.dead_letter(&envelope, "configured-entry", &message)
                    .await;
                return DiscordInboundReceipt::Failed { message };
            }
        };
        if !flow.is_active {
            return rejected(DiscordRejectionReason::FlowInactive);
        }

        let flow_id = flow.id;
        let start_node_id = flow.start_node_id;
        let trigger = TriggerEvent {
            source: TriggerSource::Integration,
            event_id: Some(event_id.clone()),
            correlation_id: Some(command.interaction_id),
            flow_id: flow_id.clone(),
            node_id: start_node_id.clone(),
            data: json!({ "event": envelope }),
            timestamp: command.occurred_at,
            dedup_key: Some(format!("discord:{flow_id}:{event_id}")),
        };
        match self.executor.trigger_flow_from_event(trigger).await {
            Ok(run_id) => DiscordInboundReceipt::Accepted { run_id },
            Err(message) if message == "Duplicate event filtered" => {
                DiscordInboundReceipt::Duplicate { run_id: None }
            }
            Err(message) => {
                self.dead_letter(&envelope, &start_node_id, &message).await;
                DiscordInboundReceipt::Failed { message }
            }
        }
    }

    async fn dead_letter(&self, envelope: &EventEnvelope, node_id: &str, message: &str) {
        if let Err(error) = self
            .executor
            .record_workflow_dead_letter(
                envelope,
                &self.policy.flow_id,
                node_id,
                message.to_string(),
            )
            .await
        {
            log::error!(
                "Failed to persist Discord command dead letter for {}: {}",
                envelope.event_id,
                error
            );
        }
    }
}

fn rejected(reason: DiscordRejectionReason) -> DiscordInboundReceipt {
    DiscordInboundReceipt::Rejected { reason }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscordResponseError {
    pub message: String,
    /// Provider-supplied delay. Callers must not invent fixed Discord rate
    /// limits because buckets may change independently per route.
    pub retry_after: Option<Duration>,
}

#[async_trait::async_trait]
pub trait DiscordInboundTransport: Send + Sync {
    async fn next_command(&self) -> Result<Option<DiscordInboundCommand>, String>;
    /// Acknowledge receipt before durable policy evaluation and dispatch. HTTP
    /// interaction transports should defer the response here; mock or
    /// non-interaction transports may retain the no-op default.
    async fn acknowledge(
        &self,
        _command: &DiscordInboundCommand,
    ) -> Result<(), DiscordResponseError> {
        Ok(())
    }
    async fn respond(
        &self,
        command: &DiscordInboundCommand,
        receipt: &DiscordInboundReceipt,
    ) -> Result<(), DiscordResponseError>;
    /// Forget any provider-only receipt state after response retries are
    /// exhausted. This prevents expiring response tokens from accumulating.
    async fn discard_response(&self, _command: &DiscordInboundCommand) {}
    async fn close(&self) -> Result<(), String>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscordInboundLifecycleStatus {
    Stopped,
    Running,
    Stopping,
}

const LIFECYCLE_STOPPED: u8 = 0;
const LIFECYCLE_RUNNING: u8 = 1;
const LIFECYCLE_STOPPING: u8 = 2;

struct InboundTask {
    shutdown: oneshot::Sender<()>,
    join: JoinHandle<()>,
    transport: Arc<dyn DiscordInboundTransport>,
}

/// Owns exactly one inbound transport task. Starting a replacement first
/// stops and joins the previous listener, so old credentials cannot continue
/// receiving commands after reconfiguration.
#[derive(Clone)]
pub struct DiscordInboundRuntime {
    status: Arc<AtomicU8>,
    task: Arc<Mutex<Option<InboundTask>>>,
}

impl Default for DiscordInboundRuntime {
    fn default() -> Self {
        Self {
            status: Arc::new(AtomicU8::new(LIFECYCLE_STOPPED)),
            task: Arc::new(Mutex::new(None)),
        }
    }
}

impl DiscordInboundRuntime {
    pub fn status(&self) -> DiscordInboundLifecycleStatus {
        match self.status.load(Ordering::Acquire) {
            LIFECYCLE_RUNNING => DiscordInboundLifecycleStatus::Running,
            LIFECYCLE_STOPPING => DiscordInboundLifecycleStatus::Stopping,
            _ => DiscordInboundLifecycleStatus::Stopped,
        }
    }

    pub async fn start(
        &self,
        processor: DiscordInboundProcessor,
        transport: Arc<dyn DiscordInboundTransport>,
        callback: Option<Arc<dyn EventCallback>>,
    ) -> Result<(), String> {
        self.stop().await?;
        let (shutdown, mut shutdown_receiver) = oneshot::channel();
        let task_transport = transport.clone();
        let status = self.status.clone();
        status.store(LIFECYCLE_RUNNING, Ordering::Release);
        let join = tokio::spawn(async move {
            loop {
                let command = tokio::select! {
                    _ = &mut shutdown_receiver => break,
                    command = task_transport.next_command() => command,
                };
                let command = match command {
                    Ok(Some(command)) => command,
                    Ok(None) => break,
                    Err(error) => {
                        log::error!("Discord inbound transport stopped: {error}");
                        break;
                    }
                };
                if let Err(error) = task_transport.acknowledge(&command).await {
                    log::warn!(
                        "Discord acknowledgement for interaction {} failed: {}",
                        command.interaction_id,
                        error.message
                    );
                }
                let receipt = processor.process(command.clone()).await;
                if let Some(callback) = &callback {
                    if let Err(error) = callback
                        .on_event(
                            "discord.inbound_command",
                            json!({
                                "interactionId": command.interaction_id,
                                "commandName": command.command_name,
                                "receipt": receipt,
                            }),
                        )
                        .await
                    {
                        log::warn!("Discord inbound event callback failed: {error}");
                    }
                }
                respond_with_provider_retry(task_transport.as_ref(), &command, &receipt).await;
            }
            status.store(LIFECYCLE_STOPPED, Ordering::Release);
        });
        *self.task.lock().await = Some(InboundTask {
            shutdown,
            join,
            transport,
        });
        Ok(())
    }

    pub async fn stop(&self) -> Result<(), String> {
        let Some(task) = self.task.lock().await.take() else {
            self.status.store(LIFECYCLE_STOPPED, Ordering::Release);
            return Ok(());
        };
        self.status.store(LIFECYCLE_STOPPING, Ordering::Release);
        let _ = task.shutdown.send(());
        let join_result = task
            .join
            .await
            .map_err(|error| format!("Discord inbound task failed during shutdown: {error}"));
        let close_result = task.transport.close().await;
        self.status.store(LIFECYCLE_STOPPED, Ordering::Release);
        join_result.and(close_result)
    }
}

async fn respond_with_provider_retry(
    transport: &dyn DiscordInboundTransport,
    command: &DiscordInboundCommand,
    receipt: &DiscordInboundReceipt,
) {
    match transport.respond(command, receipt).await {
        Ok(()) => {}
        Err(error) => {
            let Some(retry_after) = error.retry_after else {
                log::error!(
                    "Discord response for interaction {} failed: {}",
                    command.interaction_id,
                    error.message
                );
                transport.discard_response(command).await;
                return;
            };
            tokio::time::sleep(retry_after.min(Duration::from_secs(5))).await;
            if let Err(second_error) = transport.respond(command, receipt).await {
                log::error!(
                    "Discord response retry for interaction {} failed: {}",
                    command.interaction_id,
                    second_error.message
                );
                transport.discard_response(command).await;
            }
        }
    }
}
