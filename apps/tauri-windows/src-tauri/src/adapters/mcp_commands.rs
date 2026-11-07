use rmcp::service::RunningService;
use rmcp::RoleClient;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex as TokioMutex;
use vortideck_core::services::flow::commands::FlowRegistryState;
use vortideck_core::services::mcp::chat::ChatSession;
use vortideck_core::services::mcp::client::OpenAIClient;
use vortideck_core::services::mcp::commands::ChatState;
use vortideck_core::services::mcp::config::Config;
use vortideck_core::services::mcp::flow_adapter::McpToolProvider;
use vortideck_core::services::mcp::tool::{get_mcp_tools, Tool, ToolSet};

// Use the core ChatState directly - no duplication needed
pub type AdapterChatState = ChatState;

/// Request to initialize chat session with a specific provider
#[derive(Deserialize)]
pub struct InitRequest {
    /// Provider type to use (e.g., "openai", "anthropic", "custom")
    /// If not specified, uses the first provider from config
    pub provider: Option<String>,

    /// Model ID to use (optional)
    pub model_name: Option<String>,
}

#[derive(Serialize)]
pub struct InitResponse {
    success: bool,
    tools: Vec<ToolInfo>,
}

#[derive(Serialize)]
pub struct ToolInfo {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

// MCP Commands - Adapter Layer (Platform-specific Tauri implementations)

#[tauri::command]
pub async fn init_chat(
    app_handle: AppHandle,
    state: State<'_, AdapterChatState>,
    flow_state: State<'_, FlowRegistryState>,
    request: InitRequest,
) -> Result<InitResponse, String> {
    println!("🚀 Initializing MCP chat session...");

    // Send initialization started event
    app_handle
        .emit(
            "mcp-init-status",
            serde_json::json!({
                "status": "started",
                "message": "Chat initialization started"
            }),
        )
        .unwrap_or_else(|e| {
            println!("Failed to emit initialization status: {}", e);
        });

    // Load config using the same method as the working version
    let config_path = crate::utils::ensure_mcp_config_path(&app_handle)?;

    let config = Config::load(&config_path)
        .await
        .map_err(|e| format!("Failed to load config: {}", e))?;

    println!("✅ Loaded config with {} models", config.models.model.len());

    // Send config loaded event
    app_handle
        .emit(
            "mcp-init-status",
            serde_json::json!({
                "status": "config-loaded",
                "message": "Configuration loaded successfully"
            }),
        )
        .unwrap_or_else(|e| {
            println!("Failed to emit initialization status: {}", e);
        });

    // Check if we have any models configured
    if config.models.model.is_empty() {
        return Err("No models found in configuration".to_string());
    }

    // Get the model configuration
    println!(
        "🔍 [INIT_CHAT] Received request: provider={:?}, model_name={:?}",
        request.provider, request.model_name
    );
    println!("🔍 [INIT_CHAT] Available provider configs:");
    for (i, m) in config.models.model.iter().enumerate() {
        println!(
            "  [{}] name='{}', provider='{}', default_model={:?}",
            i, m.name, m.provider, m.default_model
        );
    }

    // Look up config by provider type (if specified), otherwise use first config
    let model_config = if let Some(ref provider_type) = request.provider {
        println!(
            "🔍 [INIT_CHAT] Looking for provider type: {}",
            provider_type
        );
        config.get_model_by_provider(provider_type).ok_or_else(|| {
            format!(
                "No configuration found for provider type '{}'",
                provider_type
            )
        })?
    } else {
        // Fallback: use first model from config
        println!("🔍 [INIT_CHAT] No provider specified, using first config");
        config
            .models
            .model
            .first()
            .ok_or_else(|| "No models found in configuration".to_string())?
    };

    // Get the actual URL (constructed from components if available)
    let chat_url = model_config.get_chat_url();

    println!(
        "✅ [INIT_CHAT] Selected config: name='{}', provider='{}', url='{}'",
        model_config.name, model_config.provider, chat_url
    );

    // Determine provider from model config (with backward compatibility)
    use vortideck_core::services::mcp::{AIProvider, AnthropicClient};

    // Handle provider type (new configs have provider field, old ones might not)
    let provider_type = if !model_config.provider.is_empty() {
        model_config.provider.clone()
    } else {
        // Legacy support: guess provider from URL or model name
        if chat_url.contains("anthropic") || model_config.name.to_lowercase().contains("claude") {
            "anthropic".to_string()
        } else if chat_url.contains("azure") {
            "azure".to_string()
        } else {
            "openai".to_string()
        }
    };

    let provider = match provider_type.to_lowercase().as_str() {
        "anthropic" | "claude" => {
            println!("✓ Using Anthropic provider: {}", model_config.name);
            AIProvider::Anthropic {
                api_key: model_config.api_key.clone(),
                base_url: Some(chat_url.clone()),
            }
        }
        "azure" | "azureopenai" => {
            println!("✓ Using Azure OpenAI provider: {}", model_config.name);
            AIProvider::AzureOpenAI {
                api_key: model_config.api_key.clone(),
                endpoint: chat_url.clone(),
                deployment: model_config
                    .deployment
                    .clone()
                    .or_else(|| model_config.default_model.clone())
                    .unwrap_or_else(|| "gpt-4".to_string()),
            }
        }
        "ollama" => {
            println!("✓ Using Ollama provider: {}", model_config.name);

            // Convert ModelCapability to ModelInfo
            let models = model_config
                .models
                .as_ref()
                .map(|caps| {
                    caps.iter()
                        .map(|cap| {
                            vortideck_core::services::mcp::provider::ModelInfo {
                                id: cap.id.clone(),
                                name: cap.name.clone().unwrap_or_else(|| cap.id.clone()),
                                description: String::new(),
                                context_window: cap.context_window.unwrap_or(4096),
                                supports_temperature: cap.supports_temperature,
                                supports_streaming: cap.supports_streaming,
                                supports_tools: false, // Ollama doesn't support MCP tools
                            }
                        })
                        .collect()
                })
                .unwrap_or_default();

            AIProvider::Ollama {
                base_url: chat_url.clone(),
                models,
            }
        }
        "custom" => {
            println!("✓ Using custom provider: {}", model_config.name);

            // Convert ModelCapability to ModelInfo
            let models = model_config
                .models
                .as_ref()
                .map(|caps| {
                    caps.iter()
                        .map(|cap| {
                            vortideck_core::services::mcp::provider::ModelInfo {
                                id: cap.id.clone(),
                                name: cap.name.clone().unwrap_or_else(|| cap.id.clone()),
                                description: String::new(),
                                context_window: cap.context_window.unwrap_or(4096),
                                supports_temperature: cap.supports_temperature,
                                supports_streaming: cap.supports_streaming,
                                supports_tools: true, // Custom providers are OpenAI-compatible
                            }
                        })
                        .collect()
                })
                .unwrap_or_default();

            AIProvider::Custom {
                name: model_config.name.clone(),
                base_url: chat_url.clone(),
                api_key: Some(model_config.api_key.clone()),
                models,
            }
        }
        _ => {
            println!("✓ Using OpenAI provider: {}", model_config.name);
            AIProvider::OpenAI {
                api_key: model_config.api_key.clone(),
                base_url: Some(chat_url.clone()),
            }
        }
    };

    // Create appropriate client based on provider
    let chat_client: Arc<dyn vortideck_core::services::mcp::ChatClient> = match &provider {
        AIProvider::OpenAI { api_key, base_url } => {
            println!("Creating OpenAI client");
            Arc::new(OpenAIClient::new(api_key.clone(), base_url.clone()))
        }
        AIProvider::Anthropic { api_key, base_url } => {
            println!("Creating Anthropic client");
            Arc::new(AnthropicClient::new(api_key.clone(), base_url.clone()))
        }
        AIProvider::AzureOpenAI {
            api_key, endpoint, ..
        } => {
            println!("Creating Azure OpenAI client");
            Arc::new(OpenAIClient::new(api_key.clone(), Some(endpoint.clone())))
        }
        AIProvider::Ollama { base_url, .. } => {
            println!("Creating Ollama client (native API)");
            Arc::new(vortideck_core::services::mcp::client::OllamaClient::new(
                base_url.clone(),
            ))
        }
        AIProvider::Custom {
            base_url, api_key, ..
        } => {
            println!("Creating custom client (OpenAI-compatible)");
            Arc::new(OpenAIClient::new(
                api_key.clone().unwrap_or_default(),
                Some(base_url.clone()),
            ))
        }
    };

    // ✅ REUSE GLOBAL MCP TOOLS: Use the globally initialized tools from ChatState
    // No need to reconnect - tools were loaded at app startup
    let tool_set = state.mcp_tools.clone();
    println!("📦 Reusing globally initialized MCP tools");

    // ✅ MCP tools already loaded globally at app startup - no reconnection needed!
    println!(
        "✅ Using {} globally initialized MCP tools",
        tool_set.lock().await.tools().len()
    );

    // Get tools info for response (sessions are created on-demand via SessionManager)
    let tools: Vec<ToolInfo> = tool_set
        .lock()
        .await
        .tools()
        .iter()
        .map(|tool| ToolInfo {
            name: tool.name(),
            description: tool.description(),
            parameters: tool.parameters(),
        })
        .collect();

    println!("✅ Chat system ready - sessions will be created on-demand per conversation");

    // Store config in state (if not already set)
    {
        let mut config_state = state.config.lock().await;
        if config_state.is_none() {
            *config_state = Some(config);
        }
    }

    // Sync flow registry with current MCP tools
    refresh_flow_registry_mcp_tools(&state, &flow_state).await;

    // Send initialization complete event
    let final_tool_count = tool_set.lock().await.tools().len();
    app_handle.emit("mcp-init-status", serde_json::json!({
        "status": "complete",
        "message": format!("Chat session initialized - {} MCP tools available (loaded at startup)", final_tool_count),
        "tools_count": final_tool_count,
    })).unwrap_or_else(|e| {
        println!("Failed to emit initialization status: {}", e);
    });

    Ok(InitResponse {
        success: true,
        tools,
    })
}

// Re-export McpServerConfig from core for frontend use
use vortideck_core::services::mcp::config::McpServerConfig;

#[derive(Serialize)]
pub struct McpServerResponse {
    pub success: bool,
    pub message: String,
    pub tools_count: usize,
}

pub fn spawn_mcp_client_event_forwarder(
    app_handle: AppHandle,
    chat_state: AdapterChatState,
    flow_state: FlowRegistryState,
) -> vortideck_core::services::mcp::handler::McpClientEventSender {
    use vortideck_core::services::mcp::handler::{client_event_channel, McpClientEvent};

    let (event_tx, mut event_rx) = client_event_channel();
    tauri::async_runtime::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            match event {
                McpClientEvent::Progress(progress) => {
                    let _ = app_handle.emit("mcp-tool-progress", progress);
                }
                McpClientEvent::ToolListChanged { server_name } => {
                    let peer = {
                        let clients = chat_state.mcp_clients.lock().await;
                        clients
                            .get(&server_name)
                            .map(|client| client.peer().clone())
                    };
                    let Some(peer) = peer else {
                        continue;
                    };
                    match get_mcp_tools(peer).await {
                        Ok(tools) => {
                            match chat_state.replace_server_tools(&server_name, &tools).await {
                                Ok(tools_count) => {
                                    refresh_flow_registry_mcp_tools(&chat_state, &flow_state).await;
                                    let _ = app_handle.emit(
                                        "mcp-server-tools-changed",
                                        serde_json::json!({
                                            "server_name": server_name,
                                            "tools_count": tools_count,
                                        }),
                                    );
                                }
                                Err(error) => {
                                    log::error!(
                                        "Rejected MCP schema refresh for {}: {}",
                                        server_name,
                                        error
                                    );
                                    let _ = app_handle.emit(
                                        "mcp-server-error",
                                        serde_json::json!({
                                            "server_name": server_name,
                                            "error": error,
                                        }),
                                    );
                                }
                            }
                        }
                        Err(error) => log::error!(
                            "Failed to rediscover MCP tools for {} after list change: {}",
                            server_name,
                            error
                        ),
                    }
                }
            }
        }
    });
    event_tx
}

/// Connect to a single MCP server and load its tools
#[tauri::command]
pub async fn connect_mcp_server(
    app_handle: AppHandle,
    state: State<'_, AdapterChatState>,
    flow_state: State<'_, FlowRegistryState>,
    server: McpServerConfig,
) -> Result<McpServerResponse, String> {
    use vortideck_core::services::mcp::handler::VortiDeckClientHandler;

    if server.name.trim().is_empty() {
        return Err("MCP server name cannot be empty".to_string());
    }

    println!("🔌 Connecting to MCP server: {}", server.name);

    let server_name = server.name.clone();
    let event_tx = spawn_mcp_client_event_forwarder(
        app_handle.clone(),
        state.inner().clone(),
        flow_state.inner().clone(),
    );

    let handler = VortiDeckClientHandler::for_server(server_name, event_tx);

    match server.transport.start(Some(handler)).await {
        Ok(client) => {
            println!("✅ Connected to MCP server: {}", server.name);

            // Load tools from this server
            match get_mcp_tools(client.peer().clone()).await {
                Ok(tools) => {
                    let tools_count = tools.len();
                    println!("✅ Loaded {} tools from {}", tools_count, server.name);

                    state
                        .replace_server_tools(&server.name, &tools)
                        .await
                        .map_err(|error| format!("Rejected tools from {}: {error}", server.name))?;

                    // 🔧 CRITICAL: Store the client to keep connection alive!
                    let mut mcp_clients = state.mcp_clients.lock().await;
                    mcp_clients.insert(server.name.clone(), client);
                    drop(mcp_clients);

                    // Refresh flow registry with updated tools
                    refresh_flow_registry_mcp_tools(&state, &flow_state).await;

                    app_handle
                        .emit(
                            "mcp-server-connected",
                            serde_json::json!({
                                "server_name": server.name,
                                "tools_count": tools_count,
                            }),
                        )
                        .ok();

                    Ok(McpServerResponse {
                        success: true,
                        message: format!(
                            "Connected to {} - {} tools loaded",
                            server.name, tools_count
                        ),
                        tools_count,
                    })
                }
                Err(e) => Err(format!("Failed to load tools from {}: {}", server.name, e)),
            }
        }
        Err(e) => Err(format!("Failed to connect to {}: {}", server.name, e)),
    }
}

/// Disconnect from an MCP server (removes its tools)
#[tauri::command]
pub async fn disconnect_mcp_server(
    app_handle: AppHandle,
    state: State<'_, AdapterChatState>,
    flow_state: State<'_, FlowRegistryState>,
    server_name: String,
) -> Result<McpServerResponse, String> {
    println!("🔌 Disconnecting MCP server: {}", server_name);

    // Remove the client connection (this will close the connection)
    let mut mcp_clients = state.mcp_clients.lock().await;
    mcp_clients.remove(&server_name);
    drop(mcp_clients);

    let removed_count = state.remove_server_tools(&server_name).await;

    if removed_count > 0 {
        // Refresh flow registry after removing tools
        refresh_flow_registry_mcp_tools(&state, &flow_state).await;

        app_handle
            .emit(
                "mcp-server-disconnected",
                serde_json::json!({
                    "server_name": server_name,
                    "tools_removed": removed_count,
                }),
            )
            .ok();

        println!(
            "✅ Disconnected from {} - {} tools removed",
            server_name, removed_count
        );

        Ok(McpServerResponse {
            success: true,
            message: format!(
                "Disconnected from {} - {} tools removed",
                server_name, removed_count
            ),
            tools_count: removed_count,
        })
    } else {
        println!("⚠️ No tools found for server: {}", server_name);

        app_handle
            .emit(
                "mcp-server-disconnected",
                serde_json::json!({
                    "server_name": server_name,
                    "tools_removed": 0,
                }),
            )
            .ok();

        Ok(McpServerResponse {
            success: true,
            message: format!("Disconnected from {} (no tools to remove)", server_name),
            tools_count: 0,
        })
    }
}

/// Helper to refresh flow registry with current MCP tools
async fn refresh_flow_registry_mcp_tools(
    chat_state: &AdapterChatState,
    flow_state: &FlowRegistryState,
) {
    // Get current tools from single source of truth
    let tool_set_guard = chat_state.mcp_tools.lock().await;
    let tools_count = tool_set_guard.tools().len();

    if tools_count > 0 {
        // Create MCP provider from current tools
        let mcp_provider = McpToolProvider::new(tool_set_guard.clone());
        drop(tool_set_guard);

        // Update flow registry
        let mut registry = flow_state.registry.lock().unwrap();
        registry.clear_provider_nodes("McpToolProvider");
        registry.register_provider(Box::new(mcp_provider));

        println!("🔄 Flow registry refreshed with {} MCP tools", tools_count);
    } else {
        drop(tool_set_guard);

        // No tools - just clear the provider
        let mut registry = flow_state.registry.lock().unwrap();
        registry.clear_provider_nodes("McpToolProvider");

        println!("🔄 Flow registry cleared (no MCP tools)");
    }
}
