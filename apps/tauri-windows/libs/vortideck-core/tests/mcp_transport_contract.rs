use std::{collections::HashMap, sync::Arc, time::Duration};

use rmcp::{
    model::{
        CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, ErrorData,
        ListToolsResult, PaginatedRequestParams, ServerCapabilities, ServerInfo, Tool,
    },
    service::{MaybeSendFuture, NotificationContext, RequestContext},
    RoleServer, ServerHandler, ServiceExt,
};
use serde_json::{json, Map, Value};
use vortideck_core::services::mcp::{
    config::McpServerTransportConfig,
    handler::{client_event_channel, McpClientEvent, VortiDeckClientHandler},
    tool::{get_mcp_tools, Tool as VortiDeckTool},
};
use vortideck_core::services::{
    execution::{ButtonExecutionMode, ExecutionContext, RetryConfig, TriggerSource},
    mcp::tool::ToolCallContext,
};

const FIXTURE_ENV: &str = "VORTIDECK_MCP_CONTRACT_FIXTURE";
const SCHEMA_ENV: &str = "VORTIDECK_MCP_FIXTURE_SCHEMA";
const CANCEL_MARKER_ENV: &str = "VORTIDECK_MCP_CANCEL_MARKER";

#[derive(Clone)]
struct ContractFixture {
    schema: String,
    cancel_marker: Option<String>,
}

impl ContractFixture {
    fn tool_name(&self) -> String {
        format!("scoped_echo_{}", self.schema)
    }
}

impl ServerHandler for ContractFixture {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_tool_list_changed()
                .build(),
        )
        .with_instructions("VortiDeck outbound MCP contract fixture")
    }

    fn on_initialized(
        &self,
        context: NotificationContext<RoleServer>,
    ) -> impl std::future::Future<Output = ()> + MaybeSendFuture + '_ {
        async move {
            let _ = context.peer.notify_tool_list_changed().await;
        }
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let object_schema = Arc::new(
            serde_json::from_value::<Map<String, Value>>(json!({
                "type": "object",
                "properties": {
                    "scope": { "type": "string" },
                    "value": { "type": "string" }
                },
                "required": ["scope", "value"]
            }))
            .expect("object schema"),
        );
        Ok(ListToolsResult {
            tools: vec![
                Tool::new(
                    self.tool_name(),
                    format!("Schema {} scoped echo", self.schema),
                    object_schema,
                ),
                Tool::new(
                    "wait_for_cancel",
                    "Wait until the MCP caller cancels this request",
                    Arc::new(Map::from_iter([(
                        "type".to_string(),
                        Value::String("object".to_string()),
                    )])),
                ),
            ],
            ..Default::default()
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        if request.name == "wait_for_cancel" {
            context.ct.cancelled().await;
            if let Some(path) = &self.cancel_marker {
                let _ = tokio::fs::write(path, b"cancelled").await;
            }
            return Ok(CallToolResult::success(vec![ContentBlock::text("cancelled")]).into());
        }
        if request.name != self.tool_name() {
            return Err(ErrorData::invalid_params("unknown fixture tool", None));
        }

        let arguments = request.arguments.unwrap_or_default();
        if arguments.get("scope").and_then(Value::as_str) != Some("allowed") {
            return Ok(CallToolResult::error(vec![ContentBlock::text("scope denied")]).into());
        }
        let value = arguments
            .get("value")
            .and_then(Value::as_str)
            .unwrap_or_default();
        Ok(
            CallToolResult::success(vec![ContentBlock::text(format!("{}:{value}", self.schema))])
                .into(),
        )
    }
}

#[tokio::test]
async fn mcp_contract_fixture() -> anyhow::Result<()> {
    if std::env::var(FIXTURE_ENV).as_deref() != Ok("1") {
        return Ok(());
    }
    let fixture = ContractFixture {
        schema: std::env::var(SCHEMA_ENV).unwrap_or_else(|_| "v1".to_string()),
        cancel_marker: std::env::var(CANCEL_MARKER_ENV).ok(),
    };
    fixture
        .serve(rmcp::transport::stdio())
        .await?
        .waiting()
        .await?;
    Ok(())
}

fn fixture_transport(schema: &str, marker: &std::path::Path) -> McpServerTransportConfig {
    let mut envs = HashMap::new();
    envs.insert(FIXTURE_ENV.to_string(), "1".to_string());
    envs.insert(SCHEMA_ENV.to_string(), schema.to_string());
    envs.insert(
        CANCEL_MARKER_ENV.to_string(),
        marker.to_string_lossy().into_owned(),
    );
    McpServerTransportConfig::Stdio {
        command: std::env::current_exe()
            .expect("current integration-test executable")
            .to_string_lossy()
            .into_owned(),
        args: vec![
            "--exact".to_string(),
            "mcp_contract_fixture".to_string(),
            "--quiet".to_string(),
            "--nocapture".to_string(),
            "--test-threads".to_string(),
            "1".to_string(),
        ],
        envs,
    }
}

#[tokio::test]
async fn stdio_discovers_invokes_scopes_cancels_and_reconnects() -> anyhow::Result<()> {
    if std::env::var(FIXTURE_ENV).as_deref() == Ok("1") {
        return Ok(());
    }
    let temp = tempfile::tempdir()?;
    let marker = temp.path().join("cancelled.marker");

    let (events_tx, mut events_rx) = client_event_channel();
    let first = fixture_transport("v1", &marker)
        .start(Some(VortiDeckClientHandler::for_server(
            "fixture", events_tx,
        )))
        .await?;
    let notification = tokio::time::timeout(Duration::from_secs(5), events_rx.recv())
        .await?
        .expect("notification channel remains open");
    assert!(matches!(
        notification,
        McpClientEvent::ToolListChanged { server_name } if server_name == "fixture"
    ));
    let tools = get_mcp_tools(first.peer().clone()).await?;
    let echo = tools
        .iter()
        .find(|tool| tool.name() == "scoped_echo_v1")
        .expect("independent client discovers v1 schema");
    let allowed: Value = serde_json::from_str(
        &echo
            .call(json!({ "scope": "allowed", "value": "hello" }))
            .await?,
    )?;
    assert_eq!(allowed["isError"], false);
    assert!(allowed.to_string().contains("v1:hello"));

    let denied: Value = serde_json::from_str(
        &echo
            .call(json!({ "scope": "forbidden", "value": "hello" }))
            .await?,
    )?;
    assert_eq!(denied["isError"], true);
    assert!(denied.to_string().contains("scope denied"));

    let missing = first
        .peer()
        .call_tool(CallToolRequestParams::new("not_advertised"))
        .await;
    assert!(missing.is_err(), "unknown tools must be protocol errors");

    let wait = tools
        .iter()
        .find(|tool| tool.name() == "wait_for_cancel")
        .expect("cancellable fixture tool");
    let timeout = wait
        .call_with_context(
            json!({}),
            &ToolCallContext {
                call_id: "mcp-cancel-1".to_string(),
                execution: ExecutionContext {
                    trigger_source: TriggerSource::Flow {
                        flow_id: "contract-flow".to_string(),
                        node_id: "mcp-node".to_string(),
                    },
                    user_id: None,
                    session_id: "mcp-contract".to_string(),
                    variables: HashMap::new(),
                    timeout_ms: Some(100),
                    retry_config: RetryConfig {
                        max_retries: 0,
                        retry_delay_ms: 0,
                        exponential_backoff: false,
                    },
                    execution_mode: ButtonExecutionMode::RunOnce,
                    execution_config: None,
                },
            },
        )
        .await
        .expect_err("bounded VortiDeck MCP calls must time out");
    assert!(timeout.to_string().to_lowercase().contains("timeout"));
    tokio::time::timeout(Duration::from_secs(5), async {
        while !marker.exists() {
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await?;
    first.cancel().await?;

    let second = fixture_transport("v2", &marker)
        .start(Some(VortiDeckClientHandler::without_progress()))
        .await?;
    let second_names: Vec<String> = get_mcp_tools(second.peer().clone())
        .await?
        .iter()
        .map(|tool| tool.name())
        .collect();
    assert!(second_names.iter().any(|name| name == "scoped_echo_v2"));
    assert!(!second_names.iter().any(|name| name == "scoped_echo_v1"));
    second.cancel().await?;
    Ok(())
}

#[tokio::test]
async fn streamable_http_connects_and_discovers_over_the_configured_transport() -> anyhow::Result<()>
{
    use rmcp::transport::streamable_http_server::{
        session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
    };

    let server_config = StreamableHttpServerConfig::default().with_sse_keep_alive(None);
    let shutdown = server_config.cancellation_token.clone();
    let service: StreamableHttpService<ContractFixture, LocalSessionManager> =
        StreamableHttpService::new(
            || {
                Ok(ContractFixture {
                    schema: "http".to_string(),
                    cancel_marker: None,
                })
            },
            Default::default(),
            server_config,
        );
    let router = axum::Router::new().nest_service("/mcp", service);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server_task = tokio::spawn({
        let shutdown = shutdown.clone();
        async move {
            let _ = axum::serve(listener, router)
                .with_graceful_shutdown(async move { shutdown.cancelled_owned().await })
                .await;
        }
    });

    let client = McpServerTransportConfig::Http {
        url: format!("http://{address}/mcp"),
        api_key: None,
    }
    .start(Some(VortiDeckClientHandler::without_progress()))
    .await?;
    let tools = get_mcp_tools(client.peer().clone()).await?;
    assert!(tools.iter().any(|tool| tool.name() == "scoped_echo_http"));
    client.cancel().await?;
    shutdown.cancel();
    server_task.await?;
    Ok(())
}
