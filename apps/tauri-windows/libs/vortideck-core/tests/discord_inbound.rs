use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration as StdDuration;

use chrono::{Duration, Utc};
use serde_json::json;
use tokio::sync::{mpsc, Mutex};
use vortideck_core::core::{
    NoopHttpClient, NoopStorageProvider, NoopSystemInterface, PlatformContext,
};
use vortideck_core::db::db::{establish_pool_at, initialize_database_at};
use vortideck_core::integrations::builtin::discord::inbound::{
    DiscordInboundCommand, DiscordInboundPolicy, DiscordInboundProcessor, DiscordInboundReceipt,
    DiscordRejectionReason,
};
use vortideck_core::integrations::builtin::discord::inbound::{
    DiscordInboundLifecycleStatus, DiscordInboundRuntime, DiscordInboundTransport,
    DiscordResponseError,
};
use vortideck_core::services::flow::{
    FlowDefinition, FlowMetadata, FlowNodeDefinition, FlowRuntime, HybridFlowStorage,
    NodeExecutionConfig, NodeUIData, StandardNodeProvider,
};

fn command(id: &str) -> DiscordInboundCommand {
    DiscordInboundCommand {
        interaction_id: id.to_string(),
        command_name: "vortideck-break".to_string(),
        guild_id: Some("guild-1".to_string()),
        channel_id: "channel-1".to_string(),
        user_id: "moderator-1".to_string(),
        occurred_at: Utc::now(),
    }
}

fn policy(flow_id: &str) -> DiscordInboundPolicy {
    DiscordInboundPolicy {
        command_name: "vortideck-break".to_string(),
        flow_id: flow_id.to_string(),
        guild_id: "guild-1".to_string(),
        channel_id: "channel-1".to_string(),
        allowed_user_ids: HashSet::from(["moderator-1".to_string()]),
        max_age_seconds: 15,
    }
}

fn active_flow() -> FlowDefinition {
    let now = Utc::now();
    FlowDefinition {
        id: "break-scene".to_string(),
        name: "Break scene".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        created_at: now,
        updated_at: now,
        created_by: None,
        tags: Vec::new(),
        nodes: vec![FlowNodeDefinition {
            id: "delay".to_string(),
            node_type: "delay".to_string(),
            name: "Safe fixture".to_string(),
            inputs: HashMap::from([("duration".to_string(), json!(5))]),
            outputs: Vec::new(),
            ai_behavior: None,
            ai_model: None,
            prompt_template: None,
            tool_name: None,
            position: (0.0, 0.0),
            ui_data: NodeUIData::default(),
            execution_config: NodeExecutionConfig::default(),
        }],
        connections: Vec::new(),
        start_node_id: "delay".to_string(),
        metadata: FlowMetadata::default(),
        is_active: true,
    }
}

async fn runtime() -> (tempfile::TempDir, FlowRuntime) {
    let directory = tempfile::tempdir().unwrap();
    let definitions = directory.path().join("vortideck.db");
    initialize_database_at(&definitions).unwrap();
    let storage = Arc::new(
        HybridFlowStorage::new(
            establish_pool_at(&definitions).unwrap(),
            directory.path().join("flows.db").to_str().unwrap(),
        )
        .unwrap(),
    );
    let platform = Arc::new(PlatformContext::new_with_event_bus_size(
        Arc::new(NoopStorageProvider),
        Arc::new(NoopHttpClient),
        Arc::new(NoopSystemInterface),
        None,
        16,
    ));
    let runtime = FlowRuntime::new_with_storage(platform, storage);
    runtime
        .flow_registry()
        .registry
        .lock()
        .unwrap()
        .register_provider(Box::new(StandardNodeProvider::new()));
    runtime
        .executor()
        .await
        .unwrap()
        .save_flow(&active_flow())
        .await
        .unwrap();
    (directory, runtime)
}

#[tokio::test]
async fn discord_command_is_scoped_durable_and_deduplicated() {
    let (_directory, runtime) = runtime().await;
    runtime.start().await.unwrap();
    let processor =
        DiscordInboundProcessor::new(policy("break-scene"), runtime.executor().await.unwrap())
            .unwrap();

    let mut wrong_command = command("wrong-command");
    wrong_command.command_name = "something-else".to_string();
    assert_eq!(
        processor.process(wrong_command).await,
        DiscordInboundReceipt::Rejected {
            reason: DiscordRejectionReason::WrongCommand
        }
    );

    let mut direct_message = command("direct-message");
    direct_message.guild_id = None;
    assert_eq!(
        processor.process(direct_message).await,
        DiscordInboundReceipt::Rejected {
            reason: DiscordRejectionReason::DirectMessageNotAllowed
        }
    );

    let mut wrong_guild = command("wrong-guild");
    wrong_guild.guild_id = Some("guild-2".to_string());
    assert_eq!(
        processor.process(wrong_guild).await,
        DiscordInboundReceipt::Rejected {
            reason: DiscordRejectionReason::WrongGuild
        }
    );

    let mut wrong_channel = command("wrong-channel");
    wrong_channel.channel_id = "channel-2".to_string();
    assert_eq!(
        processor.process(wrong_channel).await,
        DiscordInboundReceipt::Rejected {
            reason: DiscordRejectionReason::WrongChannel
        }
    );

    let mut wrong_user = command("wrong-user");
    wrong_user.user_id = "viewer-1".to_string();
    assert_eq!(
        processor.process(wrong_user).await,
        DiscordInboundReceipt::Rejected {
            reason: DiscordRejectionReason::UserNotAllowed
        }
    );

    let mut expired = command("expired");
    expired.occurred_at = Utc::now() - Duration::seconds(30);
    assert_eq!(
        processor.process(expired).await,
        DiscordInboundReceipt::Rejected {
            reason: DiscordRejectionReason::Expired
        }
    );

    let accepted_command = command("accepted");
    let accepted = processor.process(accepted_command.clone()).await;
    let run_id = match accepted {
        DiscordInboundReceipt::Accepted { run_id } => run_id,
        other => panic!("expected accepted command, got {other:?}"),
    };
    assert_eq!(
        processor.process(accepted_command).await,
        DiscordInboundReceipt::Duplicate {
            run_id: Some(run_id)
        }
    );
    assert!(runtime
        .executor()
        .await
        .unwrap()
        .load_event_envelope("discord:accepted")
        .await
        .unwrap()
        .is_some());

    let missing =
        DiscordInboundProcessor::new(policy("missing-flow"), runtime.executor().await.unwrap())
            .unwrap()
            .process(command("offline-target"))
            .await;
    assert!(matches!(missing, DiscordInboundReceipt::Failed { .. }));
    runtime.stop().await.unwrap();
}

#[test]
fn discord_policy_refuses_an_unscoped_user_set() {
    let mut invalid = policy("break-scene");
    invalid.allowed_user_ids.clear();
    assert!(invalid.validate().is_err());
}

struct MockTransport {
    commands: Mutex<mpsc::UnboundedReceiver<DiscordInboundCommand>>,
    acknowledgements: AtomicUsize,
    responses: AtomicUsize,
    fail_first_response: AtomicBool,
    closed: AtomicBool,
}

#[async_trait::async_trait]
impl DiscordInboundTransport for MockTransport {
    async fn next_command(&self) -> Result<Option<DiscordInboundCommand>, String> {
        Ok(self.commands.lock().await.recv().await)
    }

    async fn acknowledge(
        &self,
        _command: &DiscordInboundCommand,
    ) -> Result<(), DiscordResponseError> {
        self.acknowledgements.fetch_add(1, Ordering::AcqRel);
        Ok(())
    }

    async fn respond(
        &self,
        _command: &DiscordInboundCommand,
        _receipt: &DiscordInboundReceipt,
    ) -> Result<(), DiscordResponseError> {
        self.responses.fetch_add(1, Ordering::AcqRel);
        if self.fail_first_response.swap(false, Ordering::AcqRel) {
            return Err(DiscordResponseError {
                message: "provider rate limit".to_string(),
                retry_after: Some(StdDuration::from_millis(1)),
            });
        }
        Ok(())
    }

    async fn close(&self) -> Result<(), String> {
        self.commands.lock().await.close();
        self.closed.store(true, Ordering::Release);
        Ok(())
    }
}

fn mock_transport(
    fail_first_response: bool,
) -> (
    mpsc::UnboundedSender<DiscordInboundCommand>,
    Arc<MockTransport>,
) {
    let (sender, receiver) = mpsc::unbounded_channel();
    (
        sender,
        Arc::new(MockTransport {
            commands: Mutex::new(receiver),
            acknowledgements: AtomicUsize::new(0),
            responses: AtomicUsize::new(0),
            fail_first_response: AtomicBool::new(fail_first_response),
            closed: AtomicBool::new(false),
        }),
    )
}

#[tokio::test]
async fn discord_listener_reconfiguration_joins_old_transport_and_retries_provider_delay() {
    let (_directory, flow_runtime) = runtime().await;
    let processor = DiscordInboundProcessor::new(
        policy("break-scene"),
        flow_runtime.executor().await.unwrap(),
    )
    .unwrap();
    let lifecycle = DiscordInboundRuntime::default();
    let (first_sender, first_transport) = mock_transport(false);
    lifecycle
        .start(processor.clone(), first_transport.clone(), None)
        .await
        .unwrap();
    first_sender.send(command("first-listener")).unwrap();
    tokio::time::timeout(StdDuration::from_secs(2), async {
        while first_transport.responses.load(Ordering::Acquire) == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(first_transport.acknowledgements.load(Ordering::Acquire), 1);

    let (second_sender, second_transport) = mock_transport(true);
    lifecycle
        .start(processor, second_transport.clone(), None)
        .await
        .unwrap();
    assert!(first_transport.closed.load(Ordering::Acquire));
    assert_eq!(lifecycle.status(), DiscordInboundLifecycleStatus::Running);

    second_sender.send(command("second-listener")).unwrap();
    tokio::time::timeout(StdDuration::from_secs(2), async {
        while second_transport.responses.load(Ordering::Acquire) < 2 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(second_transport.acknowledgements.load(Ordering::Acquire), 1);
    assert_eq!(second_transport.responses.load(Ordering::Acquire), 2);

    lifecycle.stop().await.unwrap();
    assert!(second_transport.closed.load(Ordering::Acquire));
    assert_eq!(lifecycle.status(), DiscordInboundLifecycleStatus::Stopped);
    lifecycle.stop().await.unwrap();
}
