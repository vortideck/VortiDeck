use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;
use serde_json::{json, Value};
use tokio::sync::{watch, Barrier};
use vortideck_core::core::{
    NoopHttpClient, NoopStorageProvider, NoopSystemInterface, PlatformContext,
};
use vortideck_core::db::db::{establish_pool_at, initialize_database_at};
use vortideck_core::services::flow::traits::{FlowNode, FlowNodeProvider};
use vortideck_core::services::flow::{
    FlowDefinition, FlowMetadata, FlowNodeDefinition, FlowRuntime, FlowRuntimeStatus,
    HybridFlowStorage, NodeExecutionConfig, NodeUIData, StandardNodeProvider,
};

fn platform() -> Arc<PlatformContext> {
    Arc::new(PlatformContext::new_with_event_bus_size(
        Arc::new(NoopStorageProvider),
        Arc::new(NoopHttpClient),
        Arc::new(NoopSystemInterface),
        None,
        16,
    ))
}

fn isolated_storage() -> (
    tempfile::TempDir,
    Arc<HybridFlowStorage>,
    std::path::PathBuf,
) {
    let directory = tempfile::tempdir().expect("isolated runtime directory");
    let definitions_path = directory.path().join("vortideck.db");
    let executions_path = directory.path().join("flows.db");
    initialize_database_at(&definitions_path).expect("initialize definitions database");
    let pool = establish_pool_at(&definitions_path).expect("open definitions database");
    let storage = Arc::new(
        HybridFlowStorage::new(pool, executions_path.to_str().unwrap())
            .expect("open execution database"),
    );
    (directory, storage, executions_path)
}

fn single_node_flow(id: &str, node_type: &str, inputs: HashMap<String, Value>) -> FlowDefinition {
    let now = Utc::now();
    FlowDefinition {
        id: id.to_string(),
        name: format!("{id} shutdown fixture"),
        version: "1.0.0".to_string(),
        description: None,
        created_at: now,
        updated_at: now,
        created_by: None,
        tags: vec!["shutdown-test".to_string()],
        nodes: vec![FlowNodeDefinition {
            id: "only-node".to_string(),
            node_type: node_type.to_string(),
            name: "Only node".to_string(),
            inputs,
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
        start_node_id: "only-node".to_string(),
        metadata: FlowMetadata::default(),
        is_active: false,
    }
}

#[derive(Clone)]
struct BlockingEffectNode {
    entered: watch::Sender<bool>,
    active: Arc<AtomicBool>,
}

struct ActiveEffectGuard(Arc<AtomicBool>);

impl Drop for ActiveEffectGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

#[async_trait]
impl FlowNode for BlockingEffectNode {
    fn id(&self) -> String {
        "blocking-effect".to_string()
    }

    fn name(&self) -> String {
        "Blocking test effect".to_string()
    }

    fn description(&self) -> String {
        "A cancellation-safe delayed provider fixture".to_string()
    }

    fn category(&self) -> String {
        "TEST".to_string()
    }

    fn parameters_schema(&self) -> Value {
        json!({ "type": "object", "properties": {} })
    }

    fn type_color(&self) -> String {
        "#000000".to_string()
    }

    async fn execute(&self, _params: Value) -> Result<Value, String> {
        self.active.store(true, Ordering::Release);
        let _guard = ActiveEffectGuard(self.active.clone());
        self.entered.send_replace(true);
        std::future::pending().await
    }
}

struct BlockingEffectProvider(BlockingEffectNode);

impl FlowNodeProvider for BlockingEffectProvider {
    fn get_flow_nodes(&self) -> Vec<Box<dyn FlowNode>> {
        vec![Box::new(self.0.clone())]
    }

    fn provider_name(&self) -> &'static str {
        "BlockingEffectProvider"
    }
}

#[tokio::test]
async fn shutdown_waits_until_a_delayed_provider_future_is_dropped() {
    let (_directory, storage, _executions_path) = isolated_storage();
    let runtime = FlowRuntime::new_with_storage(platform(), storage.clone());
    let (entered_tx, mut entered_rx) = watch::channel(false);
    let active = Arc::new(AtomicBool::new(false));
    runtime
        .flow_registry()
        .registry
        .lock()
        .unwrap()
        .register_provider(Box::new(BlockingEffectProvider(BlockingEffectNode {
            entered: entered_tx,
            active: active.clone(),
        })));

    let flow = single_node_flow("owned-effect", "blocking-effect", HashMap::new());
    runtime
        .executor()
        .await
        .unwrap()
        .save_flow(&flow)
        .await
        .unwrap();
    runtime.start().await.unwrap();
    runtime
        .executor()
        .await
        .unwrap()
        .start_flow(flow)
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(2), entered_rx.changed())
        .await
        .expect("provider should start")
        .expect("provider signal should remain open");
    assert!(active.load(Ordering::Acquire));

    runtime.stop().await.expect("runtime shutdown");

    assert!(!active.load(Ordering::Acquire));
    assert_eq!(
        runtime
            .executor()
            .await
            .unwrap()
            .tracked_runtime_task_count(),
        0
    );
    assert_eq!(runtime.status(), FlowRuntimeStatus::Stopped);

    let replacement = FlowRuntime::new_with_storage(platform(), storage);
    let report = replacement
        .start()
        .await
        .expect("replacement runtime start");
    assert_eq!(report.recovered_runs, 1);
    tokio::time::sleep(Duration::from_millis(25)).await;
    assert!(
        !active.load(Ordering::Acquire),
        "the replacement owner must not leave the old provider future active"
    );
    replacement.stop().await.expect("replacement runtime stop");
}

#[tokio::test]
async fn start_racing_stop_is_either_rejected_or_owned_by_shutdown() {
    let (_directory, storage, _executions_path) = isolated_storage();
    let runtime = FlowRuntime::new_with_storage(platform(), storage);
    runtime
        .flow_registry()
        .registry
        .lock()
        .unwrap()
        .register_provider(Box::new(StandardNodeProvider::new()));
    runtime.start().await.unwrap();
    let flow = single_node_flow(
        "racing-start",
        "delay",
        HashMap::from([("duration".to_string(), json!(30_000))]),
    );
    let barrier = Arc::new(Barrier::new(3));
    let start_executor = runtime.executor().await.unwrap();
    let start_barrier = barrier.clone();
    let start_task = tokio::spawn(async move {
        start_barrier.wait().await;
        start_executor.start_flow(flow).await
    });
    let stop_runtime = runtime.clone();
    let stop_barrier = barrier.clone();
    let stop_task = tokio::spawn(async move {
        stop_barrier.wait().await;
        stop_runtime.stop().await
    });

    barrier.wait().await;
    let start_result = start_task.await.unwrap();
    stop_task
        .await
        .unwrap()
        .expect("shutdown should own accepted work");

    if let Err(error) = start_result {
        assert!(error.contains("not accepting new work"), "{error}");
    }
    assert_eq!(
        runtime
            .executor()
            .await
            .unwrap()
            .tracked_runtime_task_count(),
        0
    );
    assert_eq!(runtime.status(), FlowRuntimeStatus::Stopped);
    let rejected = runtime
        .executor()
        .await
        .unwrap()
        .start_flow(single_node_flow("after-stop", "delay", HashMap::new()))
        .await
        .expect_err("stopped owner must reject new runs");
    assert!(rejected.contains("not accepting new work"));
}

#[tokio::test]
async fn checkpoint_failure_still_joins_tasks_and_a_repeated_stop_retries() {
    let (_directory, storage, executions_path) = isolated_storage();
    let runtime = FlowRuntime::new_with_storage(platform(), storage);
    runtime
        .flow_registry()
        .registry
        .lock()
        .unwrap()
        .register_provider(Box::new(StandardNodeProvider::new()));
    runtime.start().await.unwrap();
    runtime
        .executor()
        .await
        .unwrap()
        .start_flow(single_node_flow(
            "checkpoint-failure",
            "delay",
            HashMap::from([("duration".to_string(), json!(30_000))]),
        ))
        .await
        .unwrap();

    rusqlite::Connection::open(&executions_path)
        .unwrap()
        .execute("DROP TABLE execution_states", [])
        .unwrap();

    let first_error = runtime.stop().await.expect_err("checkpoint must fail");
    assert!(
        first_error.contains("Failed to checkpoint"),
        "{first_error}"
    );
    assert_eq!(
        runtime
            .executor()
            .await
            .unwrap()
            .tracked_runtime_task_count(),
        0
    );
    assert_eq!(runtime.status(), FlowRuntimeStatus::Stopping);

    let retry_error = runtime
        .stop()
        .await
        .expect_err("retry still sees broken storage");
    assert!(
        retry_error.contains("Failed to checkpoint"),
        "{retry_error}"
    );
    assert!(!retry_error.contains("cannot stop while"), "{retry_error}");
}

#[tokio::test]
async fn failed_start_closes_admission_and_returns_to_stopped() {
    let (_directory, storage, executions_path) = isolated_storage();
    rusqlite::Connection::open(&executions_path)
        .unwrap()
        .execute("DROP TABLE execution_states", [])
        .unwrap();
    let runtime = FlowRuntime::new_with_storage(platform(), storage);

    let start_error = runtime
        .start()
        .await
        .expect_err("recovery lookup must fail when its schema is unavailable");
    assert!(start_error.contains("execution_states"), "{start_error}");
    assert_eq!(runtime.status(), FlowRuntimeStatus::Stopped);
    assert_eq!(
        runtime
            .executor()
            .await
            .unwrap()
            .tracked_runtime_task_count(),
        0
    );
    let rejected = runtime
        .executor()
        .await
        .unwrap()
        .start_flow(single_node_flow(
            "after-failed-start",
            "delay",
            HashMap::new(),
        ))
        .await
        .expect_err("failed startup must leave admission closed");
    assert!(rejected.contains("not accepting new work"), "{rejected}");
}
