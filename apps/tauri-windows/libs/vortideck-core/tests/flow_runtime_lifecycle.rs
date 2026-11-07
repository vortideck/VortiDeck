use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use vortideck_core::core::{
    NoopHttpClient, NoopStorageProvider, NoopSystemInterface, PlatformContext,
};
use vortideck_core::db::db::{establish_pool_at, initialize_database_at};
use vortideck_core::services::flow::{
    FlowDefinition, FlowMetadata, FlowNodeDefinition, FlowRuntime, FlowRuntimeStatus,
    HybridFlowStorage, LocalFlowHost, NodeExecutionConfig, NodeUIData, StandardNodeProvider,
};

const TOKEN: &str = "test-only-local-host-token-32-bytes-minimum";

fn platform() -> Arc<PlatformContext> {
    Arc::new(PlatformContext::new_with_event_bus_size(
        Arc::new(NoopStorageProvider),
        Arc::new(NoopHttpClient),
        Arc::new(NoopSystemInterface),
        None,
        16,
    ))
}

fn saved_delay_flow(id: &str, duration_ms: u64) -> FlowDefinition {
    let now = Utc::now();
    FlowDefinition {
        id: id.to_string(),
        name: "Local host lifecycle proof".to_string(),
        version: "1.0.0".to_string(),
        description: Some("Executes through the shared background runtime".to_string()),
        created_at: now,
        updated_at: now,
        created_by: None,
        tags: vec!["integration-test".to_string()],
        nodes: vec![FlowNodeDefinition {
            id: "delay".to_string(),
            node_type: "delay".to_string(),
            name: "Short delay".to_string(),
            inputs: HashMap::from([("duration".to_string(), json!(duration_ms))]),
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
        is_active: false,
    }
}

async fn request(address: std::net::SocketAddr, request: Value) -> Value {
    let mut stream = TcpStream::connect(address)
        .await
        .expect("connect to local flow host");
    stream
        .write_all(&serde_json::to_vec(&request).unwrap())
        .await
        .expect("write local-host request");
    stream.shutdown().await.expect("finish request body");
    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .await
        .expect("read local-host response");
    serde_json::from_slice(&response).expect("valid local-host response")
}

#[tokio::test]
async fn shared_runtime_executes_across_client_reconnect_and_stops_cleanly() {
    let directory = tempfile::tempdir().expect("isolated runtime directory");
    let definitions_path = directory.path().join("vortideck.db");
    let executions_path = directory.path().join("flows.db");
    initialize_database_at(&definitions_path).expect("initialize isolated definitions database");
    let pool = establish_pool_at(&definitions_path).expect("open isolated definitions database");
    let storage = Arc::new(
        HybridFlowStorage::new(pool, executions_path.to_str().unwrap())
            .expect("open isolated execution database"),
    );

    let runtime = FlowRuntime::new_with_storage(platform(), storage);
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
        .save_flow(&saved_delay_flow("local-host-delay", 10))
        .await
        .expect("save flow before host starts");
    runtime
        .executor()
        .await
        .unwrap()
        .save_flow(&saved_delay_flow("pending-at-shutdown", 30_000))
        .await
        .expect("save pending-work fixture");

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind isolated loopback host");
    let address = listener.local_addr().unwrap();
    let host = LocalFlowHost::new(runtime.clone(), TOKEN).unwrap();
    let host_task = tokio::spawn(host.serve(listener));

    let capabilities = request(
        address,
        json!({ "requestId": "capabilities", "token": TOKEN, "operation": "capabilities" }),
    )
    .await;
    assert_eq!(capabilities["ok"], true);
    assert_eq!(capabilities["result"]["flowExecution"], true);
    assert_eq!(capabilities["result"]["desktopSystemActions"], false);
    assert_eq!(runtime.status(), FlowRuntimeStatus::Running);
    assert!(
        runtime.start().await.is_err(),
        "a second owner cannot start"
    );

    let unsupported = request(
        address,
        json!({
            "requestId": "desktop-capability",
            "token": TOKEN,
            "operation": "invokeDesktopCapability",
            "capability": "window.focus"
        }),
    )
    .await;
    assert_eq!(unsupported["ok"], false);
    assert_eq!(unsupported["error"]["code"], "capability_unavailable");

    let started = request(
        address,
        json!({
            "requestId": "execute",
            "token": TOKEN,
            "operation": "executeSavedFlow",
            "flowId": "local-host-delay"
        }),
    )
    .await;
    assert_eq!(started["ok"], true, "{started}");
    let run_id = started["result"]["runId"].as_str().unwrap().to_string();

    // The first client is gone. A fresh connection observes the same run from
    // the single host-owned executor rather than starting a second engine.
    let terminal = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let status = request(
                address,
                json!({
                    "requestId": "reconnected-status",
                    "token": TOKEN,
                    "operation": "getRun",
                    "runId": run_id
                }),
            )
            .await;
            assert_eq!(status["ok"], true, "{status}");
            let flow_status = status["result"]["status"].as_str().unwrap_or_default();
            if matches!(
                flow_status,
                "Completed" | "CompletedWithErrors" | "Failed" | "Cancelled"
            ) {
                break flow_status.to_string();
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("flow reaches terminal state");
    assert_eq!(terminal, "Completed");

    let unauthorized_shutdown = request(
        address,
        json!({ "requestId": "bad-stop", "token": "wrong", "operation": "shutdown" }),
    )
    .await;
    assert_eq!(unauthorized_shutdown["error"]["code"], "unauthorized");
    assert_eq!(runtime.status(), FlowRuntimeStatus::Running);

    let pending = request(
        address,
        json!({
            "requestId": "pending",
            "token": TOKEN,
            "operation": "executeSavedFlow",
            "flowId": "pending-at-shutdown"
        }),
    )
    .await;
    assert_eq!(pending["ok"], true, "{pending}");
    let pending_run_id = pending["result"]["runId"].as_str().unwrap().to_string();

    let shutdown = request(
        address,
        json!({ "requestId": "stop", "token": TOKEN, "operation": "shutdown" }),
    )
    .await;
    assert_eq!(shutdown["result"]["shuttingDown"], true);
    host_task
        .await
        .expect("host task joins")
        .expect("host stops cleanly");
    assert_eq!(runtime.status(), FlowRuntimeStatus::Stopped);
    runtime.stop().await.expect("repeated stop is idempotent");

    let reopened = FlowRuntime::new_with_storage(
        platform(),
        Arc::new(
            HybridFlowStorage::new(
                establish_pool_at(&definitions_path).unwrap(),
                executions_path.to_str().unwrap(),
            )
            .unwrap(),
        ),
    );
    let report = reopened.start().await.expect("start replacement owner");
    assert_eq!(report.recovered_runs, 1);
    let recovery = reopened
        .executor()
        .await
        .unwrap()
        .list_recovery_required_runs()
        .await;
    assert_eq!(recovery.len(), 1);
    assert_eq!(recovery[0].run_id, pending_run_id);
    reopened.stop().await.expect("stop replacement owner");
}
