use crate::services::mcp::handler::VortiDeckClientHandler;
use anyhow::{Context, Result};
use axum::{extract::Query, response::Html, routing::get, Router};
use base64;
use colored::Colorize;
use futures::{Sink, Stream};
use hyper::{
    self,
    header::{HeaderValue, UPGRADE},
    Request, StatusCode,
};
use hyper_util;
use pin_project_lite;
use rmcp::{
    service::RunningService,
    transport::auth::{AuthClient, AuthorizationRequest, OAuthState},
    RoleClient, ServiceExt,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite;
use tokio_tungstenite::tungstenite::Message;
use tracing;
use webpki_roots;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub models: ModelsConfig,
    pub mcp: Option<McpConfig>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ModelsConfig {
    #[serde(default)]
    pub model: Vec<ModelConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelConfig {
    pub name: String, // Provider name (e.g., "My OpenAI", "Claude API")
    #[serde(default)]
    pub provider: String, // "openai", "anthropic", "azure", "custom", "ollama"
    pub url: String,  // Base URL for the provider (or constructed from components)
    #[serde(rename = "apiKey")]
    pub api_key: String,
    #[serde(default)]
    pub default_model: Option<String>, // Default model for this provider (e.g., "gpt-4", "claude-3-5-sonnet")
    #[serde(default)]
    pub deployment: Option<String>, // For Azure: deployment name

    // Custom provider flexible configuration (new)
    #[serde(default, rename = "baseUrl")]
    pub base_url: Option<String>, // Base URL/hostname (e.g., "localhost", "192.168.1.100")
    #[serde(default)]
    pub port: Option<u16>, // Port number (e.g., 1234 for LM Studio)
    #[serde(default, rename = "chatEndpoint")]
    pub chat_endpoint: Option<String>, // Chat endpoint path (e.g., "/v1/chat/completions")
    #[serde(default, rename = "modelsEndpoint")]
    pub models_endpoint: Option<String>, // Models endpoint path (e.g., "/v1/models")
    #[serde(default, rename = "modelsUrl")]
    pub models_url: Option<String>, // Full models URL (for non-flexible configs)

    // Model capabilities (optional - for custom models)
    #[serde(default)]
    pub models: Option<Vec<ModelCapability>>,

    // Legacy field support for backward compatibility
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "model_name")]
    pub legacy_model_name: Option<String>,
}

/// Model capability definition for custom providers
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelCapability {
    pub id: String, // Model ID (e.g., "my-model-v1")
    #[serde(default)]
    pub name: Option<String>, // Display name
    #[serde(default = "default_true_capability")]
    pub supports_temperature: bool,
    #[serde(default = "default_true_capability")]
    pub supports_streaming: bool,
    #[serde(default)]
    pub context_window: Option<usize>,
}

fn default_true_capability() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize)]
pub struct McpConfig {
    pub server: Vec<McpServerConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct McpServerConfig {
    pub name: String,
    #[serde(flatten)]
    pub transport: McpServerTransportConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "protocol", rename_all = "lowercase")]
pub enum McpServerTransportConfig {
    Sse {
        url: String,
        #[serde(default)]
        oauth: Option<OAuthConfig>,
    },
    Stdio {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        envs: HashMap<String, String>,
    },
    Ws {
        url: String,
        #[serde(default)]
        api_key: Option<String>,
    },
    Http {
        url: String,
        #[serde(default)]
        api_key: Option<String>,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OAuthConfig {
    pub redirect_uri: String,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default)]
    pub callback_port: Option<u16>,
    #[serde(default)]
    pub reuse_token: Option<String>,
}

impl McpServerTransportConfig {
    /// Start the MCP transport with an optional client handler
    /// If no handler is provided, uses a default handler without progress tracking
    pub async fn start(
        &self,
        handler: Option<VortiDeckClientHandler>,
    ) -> Result<RunningService<RoleClient, VortiDeckClientHandler>> {
        // Use provided handler or create default
        let handler = handler.unwrap_or_else(|| VortiDeckClientHandler::without_progress());

        let client = match self {
            McpServerTransportConfig::Sse { url, oauth } => {
                if let Some(oauth_config) = oauth {
                    println!(
                        "{}",
                        format!("==== INITIALIZING OAUTH FOR SSE CONNECTION ====")
                            .bright_cyan()
                            .bold()
                    );

                    // Validate and format the URL
                    let formatted_url =
                        if !url.starts_with("http://") && !url.starts_with("https://") {
                            format!("https://{}", url)
                        } else {
                            url.to_string()
                        };

                    println!(
                        "{}",
                        format!("Connecting to: {}", formatted_url).bright_blue()
                    );

                    // Initialize OAuth state machine with existing token if available
                    let oauth_state = if let Some(token) = &oauth_config.reuse_token {
                        println!("{}", format!("Reusing existing OAuth token").bright_green());
                        // Create authorization manager with existing token
                        let am = rmcp::transport::auth::AuthorizationManager::new(token.clone());
                        let client = reqwest::Client::default();
                        OAuthState::new(&formatted_url, Some(client))
                            .await
                            .context("Failed to initialize oauth state machine")?
                    } else {
                        println!("{}", format!("Starting new OAuth flow").bright_yellow());

                        // Create a new client for OAuth
                        let client = reqwest::Client::builder()
                            .build()
                            .context("Failed to create HTTP client")?;

                        // Initialize OAuth state
                        let mut state = OAuthState::new(&formatted_url, Some(client))
                            .await
                            .context("Failed to initialize oauth state machine")?;

                        // Start authorization flow
                        let scopes = if oauth_config.scopes.is_empty() {
                            vec!["mcp", "profile", "email"]
                        } else {
                            oauth_config
                                .scopes
                                .iter()
                                .map(|s| s.as_str())
                                .collect::<Vec<&str>>()
                        };

                        println!(
                            "{}",
                            format!("Starting authorization with scopes: {:?}", scopes)
                                .bright_blue()
                        );

                        // Create channel for receiving authorization code and state
                        let (code_sender, code_receiver) =
                            tokio::sync::oneshot::channel::<(String, String)>();

                        // Create app state for callback server
                        let app_state = Arc::new(Mutex::new(Some(code_sender)));

                        // Start callback server if port is specified
                        if let Some(port) = oauth_config.callback_port {
                            let callback_state = app_state.clone();
                            let callback_server_handle = tokio::spawn(async move {
                                let app = Router::new()
                                    .route("/callback", get(move |Query(params): Query<HashMap<String, String>>| async move {
                                        if let (Some(code), Some(state)) = (params.get("code"), params.get("state")) {
                                            if let Some(sender) = callback_state.lock().await.take() {
                                                let _ = sender.send((code.clone(), state.clone()));
                                            }
                                        }
                                        Html(include_str!("callback.html"))
                                    }));

                                // Try ports in sequence until we find an available one
                                let mut current_port = port;
                                let max_attempts = 10;
                                let mut attempts = 0;
                                let mut actual_port = None;

                                while attempts < max_attempts {
                                    let addr =
                                        std::net::SocketAddr::from(([127, 0, 0, 1], current_port));
                                    println!(
                                        "{}",
                                        format!(
                                            "Attempting to start callback server at: http://{}",
                                            addr
                                        )
                                        .bright_blue()
                                    );

                                    match tokio::net::TcpListener::bind(addr).await {
                                        Ok(listener) => {
                                            actual_port = Some(current_port);
                                            println!("{}", format!("Successfully started callback server at: http://{}", addr).bright_green());

                                            // Start the server in a separate task
                                            let server = axum::serve(listener, app);
                                            tokio::spawn(async move {
                                                if let Err(e) = server.await {
                                                    println!(
                                                        "{}",
                                                        format!("Callback server error: {}", e)
                                                            .bright_red()
                                                    );
                                                }
                                            });

                                            break;
                                        }
                                        Err(e) => {
                                            println!(
                                                "{}",
                                                format!(
                                                    "Failed to bind to port {}: {}",
                                                    current_port, e
                                                )
                                                .bright_yellow()
                                            );
                                            current_port += 1;
                                            attempts += 1;
                                        }
                                    }
                                }

                                if attempts >= max_attempts {
                                    println!(
                                        "{}",
                                        format!(
                                            "Failed to start callback server after {} attempts",
                                            max_attempts
                                        )
                                        .bright_red()
                                    );
                                }

                                actual_port
                            });

                            // Wait for the callback server to start and get the actual port
                            let actual_port = callback_server_handle
                                .await
                                .unwrap_or(None)
                                .ok_or_else(|| {
                                    anyhow::anyhow!("Failed to start callback server")
                                })?;

                            // Update the redirect URI with the actual port
                            let redirect_uri = format!("http://localhost:{}/callback", actual_port);
                            println!(
                                "{}",
                                format!("Using redirect URI: {}", redirect_uri).bright_blue()
                            );

                            // Start the authorization process with the updated redirect URI
                            let mut auth_req = AuthorizationRequest::new(redirect_uri.clone());
                            auth_req.scopes = scopes.iter().map(|s| s.to_string()).collect();
                            match state.start_authorization(auth_req).await {
                                Ok(_) => {
                                    println!(
                                        "{}",
                                        format!("Successfully started authorization process")
                                            .bright_green()
                                    );

                                    // Get and display the authorization URL
                                    match state.get_authorization_url().await {
                                        Ok(auth_url) => {
                                            println!("{}", format!("Please open the following URL in your browser to authorize:").bright_yellow());
                                            println!("{}", auth_url.bright_blue());
                                            println!(
                                                "{}",
                                                format!("Waiting for browser callback...")
                                                    .bright_yellow()
                                            );

                                            // Wait for authorization code with timeout
                                            match tokio::time::timeout(
                                                std::time::Duration::from_secs(60),
                                                code_receiver,
                                            )
                                            .await
                                            {
                                                Ok(Ok((auth_code, csrf_token))) => {
                                                    println!("{}", format!("Received authorization code, exchanging for access token...").bright_green());

                                                    // Exchange code for access token
                                                    match state
                                                        .handle_callback(&auth_code, &csrf_token)
                                                        .await
                                                    {
                                                        Ok(_) => {
                                                            println!("{}", format!("Successfully obtained access token").bright_green());
                                                        }
                                                        Err(e) => {
                                                            println!("{}", format!("Failed to exchange code for token: {}", e).bright_red());
                                                            return Err(anyhow::anyhow!("Failed to exchange code for token: {}", e));
                                                        }
                                                    }
                                                }
                                                Ok(Err(e)) => {
                                                    println!("{}", format!("Failed to receive authorization code: {}", e).bright_red());
                                                    return Err(anyhow::anyhow!(
                                                        "Failed to receive authorization code: {}",
                                                        e
                                                    ));
                                                }
                                                Err(_) => {
                                                    println!("{}", format!("Authorization timed out after 60 seconds").bright_red());
                                                    return Err(anyhow::anyhow!(
                                                        "Authorization timed out after 60 seconds"
                                                    ));
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            println!(
                                                "{}",
                                                format!("Failed to get authorization URL: {}", e)
                                                    .bright_red()
                                            );
                                            return Err(anyhow::anyhow!(
                                                "Failed to get authorization URL: {}",
                                                e
                                            ));
                                        }
                                    }
                                }
                                Err(e) => {
                                    println!(
                                        "{}",
                                        format!("Failed to start authorization: {}", e)
                                            .bright_red()
                                    );
                                    return Err(anyhow::anyhow!(
                                        "Failed to start authorization: {}",
                                        e
                                    ));
                                }
                            }
                        }

                        state
                    };

                    // Create authorized transport
                    let am = oauth_state
                        .into_authorization_manager()
                        .ok_or_else(|| anyhow::anyhow!("Failed to get authorization manager"))?;

                    let client = AuthClient::new(reqwest::Client::default(), am);
                    let transport = rmcp::transport::streamable_http_client::StreamableHttpClientTransport::with_client(
                        client,
                        rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig::with_uri(formatted_url.clone())
                    );

                    handler.serve(transport).await?
                } else {
                    // Regular HTTP connection without OAuth
                    let client = reqwest::Client::default();
                    let transport = rmcp::transport::streamable_http_client::StreamableHttpClientTransport::with_client(
                        client,
                        rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig::with_uri(url.to_string())
                    );
                    handler.serve(transport).await?
                }
            }
            McpServerTransportConfig::Stdio {
                command,
                args,
                envs,
            } => {
                // Build command - suppress stderr to prevent formatting issues from external servers
                // MCP protocol uses stdin/stdout for communication, stderr is only for server logging
                let mut cmd = tokio::process::Command::new(command);
                cmd.args(args).envs(envs).stderr(Stdio::null()); // Suppress stderr to prevent ANSI codes and formatting issues

                let transport = rmcp::transport::child_process::TokioChildProcess::new(cmd)?;
                handler.serve(transport).await?
            }
            McpServerTransportConfig::Ws { url, api_key } => {
                // Validate URL scheme
                if !url.starts_with("ws://") && !url.starts_with("wss://") {
                    return Err(anyhow::anyhow!(
                        "WebSocket URL must start with 'ws://' or 'wss://'. Got: {}",
                        url
                    ));
                }

                // Build the connection URL
                let mut connect_url = url.to_string();

                // For Smithery: ensure URL ends with /ws if it's a Smithery URL and append API key if provided
                if url.contains("server.smithery.ai") {
                    // Add /ws if missing
                    if !connect_url.ends_with("/ws") {
                        connect_url = format!("{}/ws", connect_url);
                    }

                    // Smithery requires specific parameters for MCP
                    // First add the basic parameters that are always needed
                    connect_url = if connect_url.contains('?') {
                        format!("{}&protocol=mcp&version=0.1", connect_url)
                    } else {
                        format!("{}?protocol=mcp&version=0.1", connect_url)
                    };

                    // Add API key as query parameter if provided
                    if let Some(key) = api_key {
                        connect_url = format!("{}&api_key={}", connect_url, key);
                    }
                }

                tracing::info!(
                    "Connecting to WebSocket server at: {}",
                    // Mask API key in logs
                    if connect_url.contains("api_key=") {
                        connect_url
                            .split("api_key=")
                            .next()
                            .unwrap_or("")
                            .to_owned()
                            + "api_key=********"
                    } else {
                        connect_url.clone()
                    }
                );

                // Also log to stdout for easier debugging
                println!(
                    "{}",
                    format!("==== ATTEMPTING TO CONNECT TO WEBSOCKET SERVER ====")
                        .bright_cyan()
                        .bold()
                );
                println!(
                    "{}",
                    format!(
                        "URL: {}",
                        if connect_url.contains("api_key=") {
                            connect_url
                                .split("api_key=")
                                .next()
                                .unwrap_or("")
                                .to_owned()
                                + "api_key=********"
                        } else {
                            connect_url.clone()
                        }
                    )
                    .bright_blue()
                );

                // Clone connect_url to prevent move issues
                let connect_url_for_ws = connect_url.clone();

                println!(
                    "{}",
                    format!("Making WebSocket connect_async call...").yellow()
                );
                let ws_result = tokio_tungstenite::connect_async(&connect_url_for_ws).await;

                match ws_result {
                    Ok((stream, response)) => {
                        println!(
                            "{}",
                            format!(
                                "WebSocket connection established with status: {}",
                                response.status()
                            )
                            .bright_green()
                        );
                        if response.status()
                            != tokio_tungstenite::tungstenite::http::StatusCode::SWITCHING_PROTOCOLS
                        {
                            let error_msg = format!(
                                "Failed to upgrade WebSocket connection: Status code {}",
                                response.status()
                            );
                            println!(
                                "{}",
                                format!("==== WEBSOCKET ERROR: {} ====", error_msg)
                                    .bright_red()
                                    .bold()
                            );
                            tracing::error!("{}", error_msg);
                            return Err(anyhow::anyhow!(error_msg));
                        }

                        println!(
                            "{}",
                            format!("Creating WebSocket transport...").bright_blue()
                        );
                        let transport = WebsocketTransport::new_client(stream);

                        println!(
                            "{}",
                            format!("Initializing MCP service with WebSocket transport...")
                                .bright_blue()
                        );
                        match handler.serve(transport).await {
                            Ok(service) => {
                                println!(
                                    "{}",
                                    format!("==== MCP SERVICE INITIALIZED SUCCESSFULLY ====")
                                        .bright_green()
                                        .bold()
                                );
                                service
                            }
                            Err(e) => {
                                let error_msg = format!("Failed to initialize MCP service: {}", e);
                                println!(
                                    "{}",
                                    format!("==== MCP SERVICE ERROR: {} ====", error_msg)
                                        .bright_red()
                                        .bold()
                                );
                                tracing::error!("{}", error_msg);
                                return Err(anyhow::anyhow!(error_msg));
                            }
                        }
                    }
                    Err(e) => {
                        let error_msg = format!("WebSocket connection error: {}", e);
                        println!(
                            "{}",
                            format!("==== WEBSOCKET CONNECTION ERROR: {} ====", error_msg)
                                .bright_red()
                                .bold()
                        );
                        tracing::error!("{}", error_msg);

                        // Add more details about common connection errors
                        if e.to_string().contains("tcp connect error")
                            || e.to_string().contains("connection refused")
                        {
                            println!(
                                "{}",
                                format!(
                                    "This appears to be a network connection issue. Please check:"
                                )
                                .bright_yellow()
                            );
                            println!(
                                "{}",
                                format!("1. The server is running and accessible").bright_yellow()
                            );
                            println!(
                                "{}",
                                format!("2. No firewall is blocking the connection")
                                    .bright_yellow()
                            );
                            println!("{}", format!("3. The URL is correct").bright_yellow());
                        } else if e.to_string().contains("dns error")
                            || e.to_string().contains("not found")
                        {
                            println!(
                                "{}",
                                format!("This appears to be a DNS resolution issue. Please check:")
                                    .bright_yellow()
                            );
                            println!(
                                "{}",
                                format!("1. The hostname in the URL is correct").bright_yellow()
                            );
                            println!(
                                "{}",
                                format!("2. Your internet connection is working").bright_yellow()
                            );
                            println!(
                                "{}",
                                format!("3. DNS servers are accessible").bright_yellow()
                            );
                        } else if e.to_string().contains("timed out") {
                            println!(
                                "{}",
                                format!("The connection attempt timed out. Please check:")
                                    .bright_yellow()
                            );
                            println!(
                                "{}",
                                format!("1. The server is running and not overloaded")
                                    .bright_yellow()
                            );
                            println!(
                                "{}",
                                format!("2. Network latency isn't too high").bright_yellow()
                            );
                            println!(
                                "{}",
                                format!("3. No firewall or proxy is blocking the connection")
                                    .bright_yellow()
                            );
                        }

                        return Err(anyhow::anyhow!(error_msg));
                    }
                }
            }
            McpServerTransportConfig::Http { url, api_key } => {
                // Validate URL scheme
                if !url.starts_with("http://") && !url.starts_with("https://") {
                    return Err(anyhow::anyhow!(
                        "HTTP URL must start with 'http://' or 'https://'. Got: {}",
                        url
                    ));
                }

                // Format the API key for log display
                let display_url = if url.contains("api_key=") {
                    url.split("api_key=").next().unwrap_or("").to_owned() + "api_key=********"
                } else {
                    url.clone()
                };

                println!(
                    "{}",
                    format!("==== ATTEMPTING TO CONNECT TO HTTP SERVER ====")
                        .bright_cyan()
                        .bold()
                );
                println!("{}", format!("URL: {}", display_url).bright_blue());

                // Handle different server types
                if url.contains("server.smithery.ai") {
                    // Special handling for Smithery servers
                    println!(
                        "{}",
                        format!("Detected Smithery server, using specialized connection...")
                            .bright_yellow()
                    );

                    // Parse the URL to extract the server path and API key
                    let server_path = url
                        .trim_start_matches("http://")
                        .trim_start_matches("https://")
                        .trim_start_matches("server.smithery.ai/")
                        .split('?')
                        .next()
                        .unwrap_or("");

                    println!(
                        "{}",
                        format!("Extracted server path: {}", server_path).bright_blue()
                    );

                    // Create a clean base URL following Smithery format
                    let base_url = format!("https://server.smithery.ai/{}", server_path);

                    // Extract API key from URL or use the provided one
                    let api_key_value = if url.contains("api_key=") {
                        url.split("api_key=")
                            .nth(1)
                            .and_then(|s| s.split('&').next())
                            .unwrap_or_else(|| api_key.as_deref().unwrap_or(""))
                    } else {
                        api_key.as_deref().unwrap_or("")
                    };

                    // For Smithery MCP servers, check if path already ends with /mcp to avoid duplication
                    let endpoint_path =
                        if server_path.ends_with("/mcp") || server_path.ends_with("mcp") {
                            "".to_string()
                        } else {
                            "/mcp".to_string()
                        };

                    // Simplified Smithery URL - only include API key parameter
                    let smithery_url = if !api_key_value.is_empty() {
                        format!(
                            "{}{}?api_key={}",
                            // Remove any trailing slashes
                            base_url.trim_end_matches('/'),
                            endpoint_path,
                            urlencoding::encode(api_key_value)
                        )
                    } else {
                        format!("{}{}", base_url.trim_end_matches('/'), endpoint_path)
                    };

                    println!(
                        "{}",
                        format!(
                            "Using simplified Smithery URL format: {}",
                            if smithery_url.contains("api_key=") {
                                smithery_url
                                    .split("api_key=")
                                    .next()
                                    .unwrap_or("")
                                    .to_owned()
                                    + "api_key=********"
                            } else {
                                smithery_url.clone()
                            }
                        )
                        .bright_blue()
                    );

                    // Try streamable HTTP client directly - this is what works best for other servers
                    println!(
                        "{}",
                        format!("Using streamable HTTP transport...").bright_blue()
                    );
                    let http_client = reqwest::Client::default();
                    let transport = rmcp::transport::streamable_http_client::StreamableHttpClientTransport::with_client(
                        http_client,
                        rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig::with_uri(smithery_url.clone())
                    );
                    match handler.serve(transport).await {
                        Ok(service) => {
                            println!(
                                "{}",
                                format!("==== MCP SERVICE INITIALIZED SUCCESSFULLY VIA HTTP ====")
                                    .bright_green()
                                    .bold()
                            );
                            return Ok(service);
                        }
                        Err(e) => {
                            let error_msg = format!("Streamable HTTP connection error: {}", e);
                            println!(
                                "{}",
                                format!("==== HTTP CONNECTION ERROR: {} ====", error_msg)
                                    .bright_red()
                                    .bold()
                            );
                            return Err(anyhow::anyhow!(
                                "HTTP connection failed for Smithery server: {}",
                                error_msg
                            ));
                        }
                    }
                } else {
                    // For non-Smithery servers, use standard connection methods
                    let connect_url = url.to_string();

                    // Try streamable HTTP transport first (confirmed enabled in Cargo.toml)
                    println!(
                        "{}",
                        format!("Using streamable HTTP transport...").bright_blue()
                    );
                    let http_client = if let Some(api_key) = api_key.as_deref() {
                        let mut headers = reqwest::header::HeaderMap::new();
                        headers.insert(
                            reqwest::header::AUTHORIZATION,
                            reqwest::header::HeaderValue::from_str(&format!("Bearer {api_key}"))
                                .context("MCP API key is not a valid HTTP header value")?,
                        );
                        reqwest::Client::builder()
                            .default_headers(headers)
                            .build()?
                    } else {
                        reqwest::Client::default()
                    };
                    let transport = rmcp::transport::streamable_http_client::StreamableHttpClientTransport::with_client(
                        http_client,
                        rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig::with_uri(connect_url.clone())
                    );
                    match handler.serve(transport).await {
                        Ok(service) => {
                            println!(
                                "{}",
                                format!("==== MCP SERVICE INITIALIZED SUCCESSFULLY VIA HTTP ====")
                                    .bright_green()
                                    .bold()
                            );
                            return Ok(service);
                        }
                        Err(e) => {
                            let error_msg = format!("Streamable HTTP connection error: {}", e);
                            println!(
                                "{}",
                                format!("==== HTTP CONNECTION ERROR: {} ====", error_msg)
                                    .bright_red()
                                    .bold()
                            );
                            return Err(anyhow::anyhow!("HTTP connection failed: {}", error_msg));
                        }
                    }
                }
            }
        };
        Ok(client)
    }
}

impl ModelConfig {
    /// Get the chat URL, constructing from components if available
    pub fn get_chat_url(&self) -> String {
        // For custom providers with flexible config, construct from components
        if self.provider == "custom" && self.base_url.is_some() && self.port.is_some() {
            let base_url = self.base_url.as_ref().unwrap();
            let port = self.port.unwrap();
            let endpoint = self
                .chat_endpoint
                .as_deref()
                .unwrap_or("/v1/chat/completions");
            format!("http://{}:{}{}", base_url, port, endpoint)
        } else {
            // Fallback to url field
            self.url.clone()
        }
    }

    /// Get the models URL, constructing from components if available
    pub fn get_models_url(&self) -> Option<String> {
        // For custom providers with flexible config, construct from components
        if self.provider == "custom" && self.base_url.is_some() && self.port.is_some() {
            let base_url = self.base_url.as_ref().unwrap();
            let port = self.port.unwrap();
            let endpoint = self.models_endpoint.as_deref().unwrap_or("/v1/models");
            Some(format!("http://{}:{}{}", base_url, port, endpoint))
        } else {
            // Fallback to modelsUrl field
            self.models_url.clone()
        }
    }
}

impl Config {
    pub async fn load(path: impl AsRef<Path>) -> Result<Self> {
        let content = tokio::fs::read_to_string(path).await?;
        let config: Self = serde_json::from_str(&content)?;
        Ok(config)
    }

    pub fn get_model_by_name(&self, name: &str) -> Option<&ModelConfig> {
        self.models.model.iter().find(|m| m.name == name)
    }

    /// Get the first model config matching the given provider type
    pub fn get_model_by_provider(&self, provider: &str) -> Option<&ModelConfig> {
        self.models.model.iter().find(|m| m.provider == provider)
    }
}

pin_project_lite::pin_project! {
    pub struct WebsocketTransport<R, S, E> {
        #[pin]
        stream: S,
        marker: std::marker::PhantomData<(fn() -> E, fn() -> R)>
    }
}

impl<R, S, E> WebsocketTransport<R, S, E> {
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            marker: std::marker::PhantomData,
        }
    }
}

impl<S, E> WebsocketTransport<rmcp::RoleClient, S, E> {
    pub fn new_client(stream: S) -> Self {
        Self {
            stream,
            marker: std::marker::PhantomData,
        }
    }
}

impl<S, E> WebsocketTransport<rmcp::RoleServer, S, E> {
    pub fn new_server(stream: S) -> Self {
        Self {
            stream,
            marker: std::marker::PhantomData,
        }
    }
}

impl<R, S, E> futures::Stream for WebsocketTransport<R, S, E>
where
    S: futures::Stream<Item = Result<tokio_tungstenite::tungstenite::Message, E>>,
    R: rmcp::service::ServiceRole,
    E: std::error::Error,
{
    type Item = rmcp::service::RxJsonRpcMessage<R>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let this = self.as_mut().project();
        match this.stream.poll_next(cx) {
            std::task::Poll::Ready(Some(Ok(message))) => {
                // Log raw message for debugging
                tracing::debug!("Received WebSocket message: {:?}", message);

                let message = match message {
                    tokio_tungstenite::tungstenite::Message::Text(json) => {
                        tracing::debug!("Received JSON: {}", json);
                        json
                    }
                    tokio_tungstenite::tungstenite::Message::Binary(data) => {
                        tracing::debug!("Received binary data, length: {}", data.len());
                        return self.poll_next(cx);
                    }
                    tokio_tungstenite::tungstenite::Message::Ping(_) => {
                        tracing::debug!("Received ping");
                        return self.poll_next(cx);
                    }
                    tokio_tungstenite::tungstenite::Message::Pong(_) => {
                        tracing::debug!("Received pong");
                        return self.poll_next(cx);
                    }
                    tokio_tungstenite::tungstenite::Message::Close(frame) => {
                        tracing::warn!("Received close frame: {:?}", frame);
                        return self.poll_next(cx);
                    }
                    _ => return self.poll_next(cx),
                };

                let parse_result =
                    serde_json::from_str::<rmcp::service::RxJsonRpcMessage<R>>(&message);
                let message = match parse_result {
                    Ok(message) => {
                        tracing::debug!("Successfully parsed JSON-RPC message: {:?}", message);
                        message
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, json = %message, "Failed to parse JSON-RPC message");
                        return self.poll_next(cx);
                    }
                };
                std::task::Poll::Ready(Some(message))
            }
            std::task::Poll::Ready(Some(Err(e))) => {
                tracing::warn!(error = %e, "websocket error");
                self.poll_next(cx)
            }
            std::task::Poll::Ready(None) => std::task::Poll::Ready(None),
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}

impl<R, S, E> futures::Sink<rmcp::service::TxJsonRpcMessage<R>> for WebsocketTransport<R, S, E>
where
    S: futures::Sink<tokio_tungstenite::tungstenite::Message, Error = E>,
    R: rmcp::service::ServiceRole,
{
    type Error = E;

    fn poll_ready(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.stream.poll_ready(cx)
    }

    fn start_send(
        self: std::pin::Pin<&mut Self>,
        item: rmcp::service::TxJsonRpcMessage<R>,
    ) -> Result<(), Self::Error> {
        let this = self.project();

        // Convert to JSON string
        let json_str = serde_json::to_string(&item).expect("jsonrpc should be valid json");

        // Log the outgoing message
        tracing::debug!("Sending JSON-RPC message: {}", json_str);

        let message = tokio_tungstenite::tungstenite::Message::Text(json_str.into());
        this.stream.start_send(message)
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.stream.poll_flush(cx)
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.stream.poll_close(cx)
    }
}
