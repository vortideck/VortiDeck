//! AI Assistant Commands
//!
//! Backend commands for managing the AI Assistant window

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{atomic::AtomicBool, Arc, LazyLock, Mutex};
use tauri::{command, AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder, Window};
use vortideck_core::services::mcp::{
    commands::ChatState,
    model::{CompletionRequest, Message as McpMessage, Usage, UsageEvidence},
    provider::{AIProvider, AssistantAIConfig, ModelInfo},
    session_actor::{ModelRequestContext, SessionHandle},
    session_manager::SessionContext,
};

#[derive(Debug, Deserialize)]
pub struct AssistantMessage {
    pub message: String,
    pub context: Option<String>,
    pub system_prompt: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AssistantResponse {
    pub message: String,
    pub suggestions: Option<Vec<AssistantSuggestion>>,
    pub model: String,
    pub provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<TokenUsage>,
    #[serde(rename = "taskId")]
    pub task_id: String,
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct AssistantChunk {
    task_id: String,
    conversation_id: String,
    sequence: u64,
    content: String,
    done: bool,
    cancelled: bool,
}

static ACTIVE_CHAT_TASKS: LazyLock<tokio::sync::Mutex<HashMap<String, String>>> =
    LazyLock::new(|| tokio::sync::Mutex::new(HashMap::new()));

async fn claim_chat_task(conversation_id: &str, task_id: &str) -> Result<(), String> {
    let mut active_tasks = ACTIVE_CHAT_TASKS.lock().await;
    if let Some(active_task) = active_tasks.get(conversation_id) {
        return Err(format!(
            "Conversation already has an active task ({active_task})"
        ));
    }
    active_tasks.insert(conversation_id.to_string(), task_id.to_string());
    Ok(())
}

async fn release_chat_task(conversation_id: &str, task_id: &str) {
    let mut active_tasks = ACTIVE_CHAT_TASKS.lock().await;
    if active_tasks
        .get(conversation_id)
        .is_some_and(|active| active == task_id)
    {
        active_tasks.remove(conversation_id);
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct TokenUsage {
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
    pub evidence: UsageEvidence,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AssistantSuggestion {
    pub label: String,
    pub value: String,
    pub action: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AssistantContext {
    pub context: Option<String>,
    pub context_info: Option<String>,
    pub current_value: Option<String>,
    pub form_data: Option<serde_json::Value>,
    pub conversation_history: Vec<ConversationMessage>,
    pub ai_config: AssistantAIConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConversationMessage {
    pub role: String,
    pub content: String,
    pub timestamp: i64,
}

// Store assistant contexts for each window
type AssistantContextStore = Arc<Mutex<HashMap<String, AssistantContext>>>;

// Global conversation history (shared across all assistant windows)
type GlobalConversationHistory = Arc<Mutex<Vec<ConversationMessage>>>;

/// Open AI Assistant window (supports both primary 'assistant' and multi-window 'assistant-<id>')
#[command]
pub async fn open_assistant_window(
    app: AppHandle,
    window_label: Option<String>,
    conversation_id: Option<String>,
    context: Option<String>,
    context_info: Option<String>,
    current_value: Option<String>,
    form_data: Option<serde_json::Value>,
) -> Result<String, String> {
    println!("🤖 Opening AI Assistant window");

    let label = window_label.unwrap_or_else(|| "assistant".to_string());

    // Store context for this assistant window
    if let Some(store) = app.try_state::<AssistantContextStore>() {
        let assistant_context = AssistantContext {
            context: context.clone(),
            context_info: context_info.clone(),
            current_value: current_value.clone(),
            form_data: form_data.clone(),
            conversation_history: Vec::new(),
            ai_config: AssistantAIConfig::default(),
        };

        let mut store = store.lock().unwrap();
        store.insert(label.clone(), assistant_context);
    }

    // If window already exists, bring to front and emit new context
    if let Some(existing_window) = app.get_webview_window(&label) {
        let _ = existing_window.show();
        let _ = existing_window.set_focus();
        let _ = existing_window.emit(
            "assistant-context-update",
            serde_json::json!({
                "context": context,
                "contextInfo": context_info,
                "currentValue": current_value,
                "formData": form_data,
                "conversationId": conversation_id,
            }),
        );
        println!("✅ Reused existing assistant window: {}", label);
        return Ok(label);
    }

    // Build the URL with optional conversationId query parameter
    let url_path = match &conversation_id {
        Some(cid) if !cid.is_empty() => format!("/assistant-window?conversationId={}", cid),
        _ => "/assistant-window".to_string(),
    };

    // Create window (hidden initially until mounted to prevent white flash)
    let window = WebviewWindowBuilder::new(&app, &label, WebviewUrl::App(url_path.into()))
        .title("AI Assistant")
        .inner_size(400.0, 600.0)
        .resizable(true)
        .decorations(false) // Custom title bar
        .transparent(false)
        .always_on_top(true)
        .visible(false) // Make it initially hidden until theme loads
        .build()
        .map_err(|e| format!("Failed to create assistant window: {}", e))?;

    // Emit initial context to the window after a short delay
    let window_clone = window.clone();
    let ctx_clone = context.clone();
    let ctx_info_clone = context_info.clone();
    let val_clone = current_value.clone();
    let data_clone = form_data.clone();
    let conv_clone = conversation_id.clone();

    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        let _ = window_clone.emit(
            "assistant-context-update",
            serde_json::json!({
                "context": ctx_clone,
                "contextInfo": ctx_info_clone,
                "currentValue": val_clone,
                "formData": data_clone,
                "conversationId": conv_clone,
            }),
        );
    });

    println!("✅ Assistant window opened: {}", label);
    Ok(label)
}

/// Get assistant context for a window
#[command]
pub async fn get_assistant_context(
    app: AppHandle,
    window_label: String,
) -> Result<AssistantContext, String> {
    println!("📝 Getting context for window: {}", window_label);

    if let Some(store) = app.try_state::<AssistantContextStore>() {
        let store = store.lock().unwrap();
        if let Some(context) = store.get(&window_label) {
            return Ok(context.clone());
        }
    }

    // Return empty context if not found
    Ok(AssistantContext {
        context: None,
        context_info: None,
        current_value: None,
        form_data: None,
        conversation_history: Vec::new(),
        ai_config: AssistantAIConfig::default(),
    })
}

/// Send a message to the AI Assistant
///
/// Uses session pools to support multiple concurrent conversations.
/// Each conversation_id gets its own isolated session with separate history.
#[command]
pub async fn send_chat_message(
    app: AppHandle,
    chat_state: tauri::State<'_, ChatState>,
    message: String,
    context: Option<String>,
    conversation_id: String,
    provider_name: Option<String>,
    model_name: Option<String>,
    task_id: Option<String>,
) -> Result<AssistantResponse, String> {
    use std::sync::atomic::Ordering;

    println!(
        "🤖 AI Assistant message (conv: {}): {}",
        conversation_id,
        message
            .char_indices()
            .nth(100)
            .map(|(index, _)| &message[..index])
            .unwrap_or(message.as_str())
    );

    // 📊 Track assistant usage
    crate::analytics::capture(
        &app,
        crate::analytics::AnalyticsEvent::new("assistant_message_sent")
            .prop("provider", provider_name.clone().unwrap_or_default())
            .prop("model", model_name.clone().unwrap_or_default()),
    );

    // Build context-enhanced prompt for AI (original message saved to history)
    let ai_prompt = match &context {
        Some(ctx) if !ctx.trim().is_empty() => format!("{}\n\n{}", ctx, message),
        _ => message.clone(),
    };

    let task_id = task_id
        .filter(|value| !value.trim().is_empty() && value.len() <= 128)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    // Initialize conversation session
    let provider = get_provider_for_conversation(&chat_state, provider_name.as_deref()).await?;
    let session_key = chat_state
        .session_manager
        .get_or_create_conversation(conversation_id.clone(), provider, model_name)
        .await?;

    // Get session handle for chat execution
    let handle = chat_state
        .session_manager
        .get(&session_key)
        .await
        .ok_or("Session not found after creation")?;

    // Check streaming mode via session info
    let session_info = handle
        .get_session_info()
        .await
        .map_err(|e| format!("Failed to get session info: {}", e))?;

    claim_chat_task(&conversation_id, &task_id).await?;

    // Reset cancellation only after this task owns the conversation. A rejected
    // concurrent request must not accidentally revive the task already running.
    chat_state
        .session_manager
        .reset_cancellation(&session_key)
        .await;
    let cancellation_flag = match chat_state
        .session_manager
        .get_cancellation_flag(&session_key)
        .await
    {
        Some(flag) => flag,
        None => {
            release_chat_task(&conversation_id, &task_id).await;
            return Err("Failed to get cancellation flag".to_string());
        }
    };

    let emit_chunk = |chunk| {
        let _ = app.emit("assistant-chunk", chunk);
    };
    let result = execute_chat_with_handle(
        &emit_chunk,
        &handle,
        &conversation_id,
        &task_id,
        message,
        ai_prompt,
        session_info.is_streaming,
        cancellation_flag,
    )
    .await;

    release_chat_task(&conversation_id, &task_id).await;
    result
}

// ============================================================================
// SessionHandle-based Chat Execution (Actor Pattern)
// ============================================================================

/// Execute one actor-owned chat task. Streaming changes only how model progress
/// is projected; persistence and tool execution remain shared.
#[allow(clippy::too_many_arguments)]
async fn execute_chat_with_handle(
    emit_chunk: &(dyn Fn(AssistantChunk) + Sync),
    handle: &SessionHandle,
    conversation_id: &str,
    task_id: &str,
    saved_message: String,
    ai_prompt: String,
    streaming: bool,
    cancellation_flag: Arc<std::sync::atomic::AtomicBool>,
) -> Result<AssistantResponse, String> {
    use std::sync::atomic::Ordering;

    if cancellation_flag.load(Ordering::Relaxed) {
        return Err("Generation cancelled by user".to_string());
    }

    let context = handle
        .prepare_model_turn(saved_message, ai_prompt)
        .await
        .map_err(|error| format!("Failed to prepare model turn: {error}"))?;
    let (mut response_message, mut usage) = request_model_response(
        emit_chunk,
        handle,
        context,
        conversation_id,
        task_id,
        streaming,
        &cancellation_flag,
    )
    .await?;

    // Tool execution loop
    let mut iteration = 0u32;
    let mut had_tool_error = false;
    const MAX_ITERATIONS: u32 = 10;

    while let Some(ref tool_calls) = response_message.tool_calls {
        // Check cancellation
        if cancellation_flag.load(Ordering::Relaxed) {
            let _ = handle.persist_pending_assistant().await;
            return Err("Generation cancelled by user".to_string());
        }

        // Skip empty tool_calls arrays
        if tool_calls.is_empty() {
            break;
        }

        iteration += 1;
        if had_tool_error || iteration > MAX_ITERATIONS {
            break;
        }

        println!(
            "🔧 Processing {} tool call(s) (iteration {})",
            tool_calls.len(),
            iteration
        );

        // Execute each tool call
        for tool_call in tool_calls {
            let tool_name = &tool_call.function.name;
            let tool_args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
                .unwrap_or(serde_json::json!({}));

            println!("  🔨 Executing: {} (id: {})", tool_name, tool_call.id);

            // Once a tool begins, await and record its real result. Cancellation
            // cannot honestly undo an external effect that may already have run.
            let result = handle
                .execute_tool_safely(tool_name, tool_args)
                .await
                .unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e));

            if result.contains("\"error\"") || result.contains("\"isError\":true") {
                println!(
                    "  ❌ Tool error: {}...",
                    &result.chars().take(100).collect::<String>()
                );
                had_tool_error = true;
            } else {
                println!(
                    "  ✅ Success: {}...",
                    &result.chars().take(100).collect::<String>()
                );
            }

            handle
                .submit_tool_result_only(tool_call.id.clone(), result)
                .await;

            if cancellation_flag.load(Ordering::Relaxed) {
                let _ = handle.persist_pending_assistant().await;
                return Err("Generation cancelled after the active tool completed".to_string());
            }
        }

        let context = handle
            .get_model_request_context()
            .await
            .map_err(|error| format!("Failed to prepare follow-up model request: {error}"))?;
        let (mut new_message, new_usage) = request_model_response(
            emit_chunk,
            handle,
            context,
            conversation_id,
            task_id,
            streaming,
            &cancellation_flag,
        )
        .await?;

        if had_tool_error && new_message.tool_calls.is_some() {
            new_message.tool_calls = None;
        }
        response_message = new_message;
        usage.merge(&new_usage);
    }

    // Persist partial response on early exit
    if had_tool_error || iteration >= MAX_ITERATIONS {
        let _ = handle.persist_pending_assistant().await;
    }

    // Get session info for response
    let session_info = handle
        .get_session_info()
        .await
        .map_err(|e| format!("Failed to get session info: {}", e))?;
    let provider = handle
        .get_provider()
        .await
        .map_err(|e| format!("Failed to get provider: {}", e))?;

    // Build final response
    let response_text = extract_response_text(&response_message, had_tool_error);

    if iteration > 0 {
        println!("✅ Response received (after {} tool iterations)", iteration);
    } else {
        println!("✅ Response received");
    }

    Ok(AssistantResponse {
        message: response_text,
        suggestions: None,
        model: session_info.model,
        provider: provider.provider_name().to_string(),
        usage: Some(TokenUsage {
            prompt_tokens: usage.prompt_tokens,
            completion_tokens: usage.completion_tokens,
            total_tokens: usage.total_tokens,
            evidence: usage.evidence,
        }),
        task_id: task_id.to_string(),
        conversation_id: conversation_id.to_string(),
    })
}

async fn request_model_response(
    emit_chunk: &(dyn Fn(AssistantChunk) + Sync),
    handle: &SessionHandle,
    context: ModelRequestContext,
    conversation_id: &str,
    task_id: &str,
    streaming: bool,
    cancellation_flag: &Arc<AtomicBool>,
) -> Result<(McpMessage, Usage), String> {
    let formatted_tools = context.client.format_tools(&context.tools);
    let request = CompletionRequest {
        model: context.model.clone(),
        messages: context.messages,
        temperature: context.temperature,
        max_tokens: Some(context.max_tokens),
        tools: if formatted_tools.is_empty() {
            None
        } else {
            Some(formatted_tools)
        },
        stream: Some(streaming),
        stream_options: streaming.then_some(vortideck_core::services::mcp::model::StreamOptions {
            include_usage: true,
        }),
    };

    let (message, usage) = if streaming {
        perform_streaming(
            emit_chunk,
            &context.client,
            request,
            conversation_id,
            task_id,
            cancellation_flag,
        )
        .await?
    } else {
        let flag = cancellation_flag.clone();
        let response = tokio::select! {
            response = context.client.complete(request) => response
                .map_err(|error| format!("Model response error: {error}"))?,
            _ = wait_for_cancellation(&flag) => {
                return Err("Generation cancelled by user".to_string());
            }
        };
        let message = response
            .choices
            .first()
            .ok_or_else(|| "No response from AI".to_string())?
            .message
            .clone();
        (message, response.usage)
    };
    handle
        .commit_assistant_response(message.clone(), Some(usage.clone()))
        .await
        .map_err(|error| format!("Failed to commit model response: {error}"))?;
    Ok((message, usage))
}

/// Perform streaming request and emit chunks to frontend
async fn perform_streaming(
    emit_chunk: &(dyn Fn(AssistantChunk) + Sync),
    client: &Arc<dyn vortideck_core::services::mcp::ChatClient>,
    request: CompletionRequest,
    conversation_id: &str,
    task_id: &str,
    cancellation_flag: &Arc<AtomicBool>,
) -> Result<(McpMessage, Usage), String> {
    // Start streaming request
    let (mut chunk_rx, mut response_handle) = client
        .complete_streaming(request)
        .await
        .map_err(|e| format!("Streaming error: {}", e))?;

    let mut sequence = 0u64;

    loop {
        let flag = cancellation_flag.clone();
        let chunk = tokio::select! {
            chunk = chunk_rx.recv() => chunk,
            _ = wait_for_cancellation(&flag) => {
                response_handle.abort();
                emit_assistant_chunk(emit_chunk, conversation_id, task_id, sequence, "", true, true);
                return Err("Generation cancelled by user".to_string());
            }
        };
        let Some(chunk) = chunk else { break };
        sequence += 1;
        emit_assistant_chunk(
            emit_chunk,
            conversation_id,
            task_id,
            sequence,
            &chunk,
            false,
            false,
        );
    }

    let flag = cancellation_flag.clone();
    let final_response = tokio::select! {
        response = &mut response_handle => response
            .map_err(|error| format!("Streaming task error: {error}"))?
            .map_err(|error| format!("Streaming response error: {error}"))?,
        _ = wait_for_cancellation(&flag) => {
            response_handle.abort();
            emit_assistant_chunk(emit_chunk, conversation_id, task_id, sequence, "", true, true);
            return Err("Generation cancelled by user".to_string());
        }
    };

    let message = final_response
        .choices
        .first()
        .ok_or_else(|| "No response from AI".to_string())?
        .message
        .clone();
    emit_assistant_chunk(
        emit_chunk,
        conversation_id,
        task_id,
        sequence + 1,
        "",
        true,
        false,
    );
    Ok((message, final_response.usage))
}

fn emit_assistant_chunk(
    emit_chunk: &(dyn Fn(AssistantChunk) + Sync),
    conversation_id: &str,
    task_id: &str,
    sequence: u64,
    content: &str,
    done: bool,
    cancelled: bool,
) {
    emit_chunk(AssistantChunk {
        task_id: task_id.to_string(),
        conversation_id: conversation_id.to_string(),
        sequence,
        content: content.to_string(),
        done,
        cancelled,
    });
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Wait for cancellation flag to be set
async fn wait_for_cancellation(flag: &Arc<std::sync::atomic::AtomicBool>) {
    use std::sync::atomic::Ordering;
    loop {
        if flag.load(Ordering::Relaxed) {
            break;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}

/// Extract response text from AI message, handling error cases
fn extract_response_text(message: &McpMessage, had_error: bool) -> String {
    match &message.content {
        Some(content) if !content.trim().is_empty() => content.clone(),
        Some(_) if had_error => "The tool encountered an error. Please try again.".to_string(),
        None if had_error => {
            "The tool encountered an error and couldn't complete the request.".to_string()
        }
        _ => "No response from AI".to_string(),
    }
}

/// Get AI provider for a conversation from config
async fn get_provider_for_conversation(
    chat_state: &ChatState,
    provider_name: Option<&str>,
) -> Result<AIProvider, String> {
    let config_guard = chat_state.config.lock().await;
    let config = config_guard
        .as_ref()
        .ok_or("No configuration loaded. Please configure AI providers in settings.")?;

    // Find matching provider or use first available
    let model_config = if let Some(name) = provider_name {
        config
            .models
            .model
            .iter()
            .find(|m| m.name == name || m.provider == name)
            .ok_or_else(|| format!("Provider '{}' not found in config", name))?
    } else {
        config
            .models
            .model
            .first()
            .ok_or("No AI providers configured. Please add a provider in settings.")?
    };

    AIProvider::from_model_config(model_config)
        .map_err(|e| format!("Failed to create provider: {}", e))
}

// ============================================================================
// Assistant Window Commands
// ============================================================================

/// Fill value back to parent window
#[command]
pub async fn assistant_fill_value(
    app: AppHandle,
    target_window: String,
    value: String,
    action: String,
) -> Result<(), String> {
    println!("📝 Filling value to window {}: {}", target_window, value);

    // Emit event to parent window (or all windows)
    app.emit(
        "assistant-fill-value",
        serde_json::json!({
            "value": value,
            "action": action,
        }),
    )
    .map_err(|e| format!("Failed to emit fill event: {}", e))?;

    Ok(())
}

/// Get context-aware suggestions (quick presets based on input type)
#[command]
pub async fn get_assistant_suggestions(
    context: String,
    current_value: Option<String>,
) -> Result<Vec<AssistantSuggestion>, String> {
    println!("📝 Getting suggestions for context: {}", context);

    let suggestions = match context.as_str() {
        "command_input" => vec![
            AssistantSuggestion {
                label: "List files".to_string(),
                value: if cfg!(windows) { "dir" } else { "ls -la" }.to_string(),
                action: Some("fill".to_string()),
            },
            AssistantSuggestion {
                label: "Open VS Code".to_string(),
                value: "code .".to_string(),
                action: Some("fill".to_string()),
            },
            AssistantSuggestion {
                label: "Git status".to_string(),
                value: "git status".to_string(),
                action: Some("fill".to_string()),
            },
        ],

        "executable_path" => {
            if cfg!(windows) {
                vec![
                    AssistantSuggestion {
                        label: "Chrome".to_string(),
                        value: "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe"
                            .to_string(),
                        action: Some("fill".to_string()),
                    },
                    AssistantSuggestion {
                        label: "VS Code".to_string(),
                        value: "C:\\Program Files\\Microsoft VS Code\\Code.exe".to_string(),
                        action: Some("fill".to_string()),
                    },
                    AssistantSuggestion {
                        label: "PowerShell".to_string(),
                        value: "C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe"
                            .to_string(),
                        action: Some("fill".to_string()),
                    },
                ]
            } else {
                vec![
                    AssistantSuggestion {
                        label: "Chrome".to_string(),
                        value: "/usr/bin/google-chrome".to_string(),
                        action: Some("fill".to_string()),
                    },
                    AssistantSuggestion {
                        label: "VS Code".to_string(),
                        value: "/usr/bin/code".to_string(),
                        action: Some("fill".to_string()),
                    },
                    AssistantSuggestion {
                        label: "Terminal".to_string(),
                        value: "/usr/bin/gnome-terminal".to_string(),
                        action: Some("fill".to_string()),
                    },
                ]
            }
        }

        "program_args" => vec![
            AssistantSuggestion {
                label: "Help flag".to_string(),
                value: "--help".to_string(),
                action: Some("fill".to_string()),
            },
            AssistantSuggestion {
                label: "Version flag".to_string(),
                value: "--version".to_string(),
                action: Some("fill".to_string()),
            },
            AssistantSuggestion {
                label: "Verbose mode".to_string(),
                value: "--verbose".to_string(),
                action: Some("fill".to_string()),
            },
        ],

        _ => Vec::new(),
    };

    Ok(suggestions)
}

// ========== New Chat Settings Commands ==========

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
}

/// Get available MCP tools for the chat settings modal
///
/// Returns tools from the global MCP tool registry (shared across all sessions)
#[command]
pub async fn get_available_mcp_tools(
    chat_state: tauri::State<'_, ChatState>,
) -> Result<Vec<ToolInfo>, String> {
    println!("🔧 Getting available MCP tools");

    let tools_guard = chat_state.mcp_tools.lock().await;
    let tools = tools_guard.tools();

    let tool_infos: Vec<ToolInfo> = tools
        .iter()
        .map(|tool| ToolInfo {
            name: tool.name().to_string(),
            description: tool.description().to_string(),
        })
        .collect();

    println!("✅ Found {} MCP tools", tool_infos.len());
    Ok(tool_infos)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSettings {
    pub provider: String,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub tools_mode: String, // "all", "selected", "none"
    pub selected_tools: Vec<String>,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    #[serde(default = "default_tool_timeout")]
    pub tool_timeout: u64, // Tool execution timeout in seconds
}

fn default_tool_timeout() -> u64 {
    30 // 30 seconds default
}

/// Update chat settings for a specific conversation session
///
/// Settings are applied to the session identified by conversation_id.
/// If no session exists yet, settings will be applied when the session is created.
#[command]
pub async fn update_chat_settings(
    chat_state: tauri::State<'_, ChatState>,
    conversation_id: String,
    provider: String,
    model: String,
    temperature: f32,
    max_tokens: u32,
    tools_mode: String,
    selected_tools: Vec<String>,
    stream: bool,
    system_prompt: Option<String>,
    tool_timeout: u64,
) -> Result<(), String> {
    println!(
        "⚙️ Updating chat settings for {}: model={}, tools_mode={}, stream={}",
        conversation_id, model, tools_mode, stream
    );

    let session_key = format!("conv:{}", conversation_id);

    // Get the session handle and update settings via actor message
    match chat_state.session_manager.get(&session_key).await {
        Some(handle) => {
            let settings = vortideck_core::services::mcp::SessionSettings {
                model: Some(model.clone()),
                temperature: Some(temperature),
                max_tokens: Some(max_tokens),
                stream: Some(stream),
                tools_mode: Some(tools_mode.clone()),
                selected_tools: Some(selected_tools.clone()),
                system_prompt: Some(system_prompt.clone()),
                tool_timeout: Some(tool_timeout),
            };
            handle.update_settings(settings).await?;
            println!(
                "✅ Chat settings updated for conversation: {}",
                conversation_id
            );
            Ok(())
        }
        None => {
            // No session yet - this is fine, settings will be applied when session is created
            println!(
                "ℹ️ No active session for {}. Settings will apply on first message.",
                conversation_id
            );
            Ok(())
        }
    }
}

/// Get default chat settings from saved config or first configured provider
#[command]
pub async fn get_default_chat_settings(app: AppHandle) -> Result<ChatSettings, String> {
    use std::fs;
    use tauri::Manager;

    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    // 1. Check user-saved chat-settings.json
    let settings_path = app_data_dir.join("chat-settings.json");
    if settings_path.exists() {
        if let Ok(content) = fs::read_to_string(&settings_path) {
            if let Ok(settings) = serde_json::from_str::<ChatSettings>(&content) {
                println!(
                    "📋 Loaded user saved chat settings: provider={}, model={}",
                    settings.provider, settings.model
                );
                return Ok(settings);
            }
        }
    }

    // 2. Otherwise inspect MCP config to get first configured provider
    if let Ok(config_path) = crate::utils::ensure_mcp_config_path(&app) {
        if let Ok(config_content) = fs::read_to_string(&config_path) {
            if let Ok(config) = serde_json::from_str::<serde_json::Value>(&config_content) {
                if let Some(models) = config
                    .get("models")
                    .and_then(|m| m.get("model"))
                    .and_then(|m| m.as_array())
                {
                    if let Some(first_model) = models.first() {
                        let provider = first_model
                            .get("provider")
                            .and_then(|p| p.as_str())
                            .unwrap_or("openai")
                            .to_string();

                        let model = first_model
                            .get("defaultModel")
                            .or_else(|| first_model.get("default_model"))
                            .and_then(|m| m.as_str())
                            .unwrap_or("gpt-4o")
                            .to_string();

                        println!(
                            "📋 Loaded default chat settings from MCP config: provider={}, model={}",
                            provider, model
                        );
                        return Ok(ChatSettings {
                            provider,
                            model,
                            temperature: 0.7,
                            max_tokens: 4000,
                            tools_mode: "all".to_string(),
                            selected_tools: vec![],
                            stream: false,
                            system_prompt: None,
                            tool_timeout: 30,
                        });
                    }
                }
            }
        }
    }

    // 3. Fallback standard default
    Ok(ChatSettings {
        provider: "openai".to_string(),
        model: "gpt-4o".to_string(),
        temperature: 0.7,
        max_tokens: 4000,
        tools_mode: "all".to_string(),
        selected_tools: vec![],
        stream: false,
        system_prompt: None,
        tool_timeout: 30,
    })
}

/// Save default chat settings to app config
#[command]
pub async fn save_default_chat_settings(
    app: AppHandle,
    provider: String,
    model: String,
    temperature: f32,
    max_tokens: u32,
    tools_mode: String,
    selected_tools: Vec<String>,
    system_prompt: Option<String>,
    tool_timeout: u64,
) -> Result<(), String> {
    use std::fs;
    use tauri::Manager;

    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    fs::create_dir_all(&app_data_dir)
        .map_err(|e| format!("Failed to create app data directory: {}", e))?;

    let settings = ChatSettings {
        provider,
        model,
        temperature,
        max_tokens,
        tools_mode,
        selected_tools,
        stream: false,
        system_prompt,
        tool_timeout,
    };

    let settings_path = app_data_dir.join("chat-settings.json");
    let json_content = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("Failed to serialize chat settings: {}", e))?;

    fs::write(&settings_path, json_content)
        .map_err(|e| format!("Failed to write chat-settings.json: {}", e))?;

    println!("✅ Saved default chat settings to {:?}", settings_path);
    Ok(())
}

/// Read MCP configuration file
#[command]
pub async fn read_mcp_config(app: AppHandle) -> Result<serde_json::Value, String> {
    use std::fs;

    if let Ok(config_path) = crate::utils::ensure_mcp_config_path(&app) {
        if config_path.exists() {
            if let Ok(content) = fs::read_to_string(&config_path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    return Ok(json);
                }
            }
        }
    }

    Ok(serde_json::json!({
        "models": { "model": [] },
        "mcp": { "server": [] }
    }))
}

/// Initialize assistant context store
pub fn init_assistant_store(app: &AppHandle) {
    app.manage(
        Arc::new(Mutex::new(HashMap::<String, AssistantContext>::new())) as AssistantContextStore,
    );
    println!("✅ Assistant context store initialized");
}

// ========== Model Cache Commands ==========

/// Read model cache from file
#[command]
pub async fn read_model_cache(app: AppHandle) -> Result<String, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let cache_path = app_data_dir.join("model-cache.json");

    if !cache_path.exists() {
        return Err("Cache file does not exist".to_string());
    }

    std::fs::read_to_string(&cache_path).map_err(|e| format!("Failed to read cache: {}", e))
}

/// Write model cache to file
#[command]
pub async fn write_model_cache(app: AppHandle, content: String) -> Result<(), String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    // Create directory if it doesn't exist
    std::fs::create_dir_all(&app_data_dir)
        .map_err(|e| format!("Failed to create app data dir: {}", e))?;

    let cache_path = app_data_dir.join("model-cache.json");

    std::fs::write(&cache_path, content).map_err(|e| format!("Failed to write cache: {}", e))?;

    println!("✅ Model cache saved to: {:?}", cache_path);
    Ok(())
}

/// Abort AI generation for a specific conversation
///
/// Cancels the entire chat response including all tool calls for the given conversation.
#[command]
pub async fn abort_generation(
    chat_state: tauri::State<'_, ChatState>,
    conversation_id: String,
    task_id: Option<String>,
) -> Result<(), String> {
    println!(
        "🛑 Aborting AI generation for conversation: {}",
        conversation_id
    );

    // Build session key and cancel
    let session_key = format!("conv:{}", conversation_id);
    if let Some(requested_task) = task_id {
        // Keep ownership stable until the cancellation flag is set. Otherwise a
        // completed old task could release and a new task could claim/reset the
        // same session between validation and cancellation.
        let active_tasks = ACTIVE_CHAT_TASKS.lock().await;
        if active_tasks.get(&conversation_id) != Some(&requested_task) {
            return Err("Refusing to cancel an inactive or newer chat task".to_string());
        }
        chat_state.session_manager.cancel(&session_key).await?;
    } else {
        chat_state.session_manager.cancel(&session_key).await?;
    }

    println!("✅ Cancellation flag set for: {}", conversation_id);
    Ok(())
}

#[cfg(test)]
mod task_ownership_tests {
    use super::{claim_chat_task, release_chat_task};

    #[tokio::test]
    async fn conversations_have_independent_task_owners() {
        let suffix = uuid::Uuid::new_v4();
        let first_conversation = format!("first-{suffix}");
        let second_conversation = format!("second-{suffix}");

        claim_chat_task(&first_conversation, "task-a")
            .await
            .expect("claim first conversation");
        claim_chat_task(&second_conversation, "task-b")
            .await
            .expect("claim second conversation");

        release_chat_task(&first_conversation, "task-a").await;
        release_chat_task(&second_conversation, "task-b").await;
    }

    #[tokio::test]
    async fn stale_release_cannot_replace_or_release_current_owner() {
        let conversation = format!("conversation-{}", uuid::Uuid::new_v4());
        claim_chat_task(&conversation, "current")
            .await
            .expect("claim conversation");

        assert!(claim_chat_task(&conversation, "concurrent").await.is_err());
        release_chat_task(&conversation, "stale").await;
        assert!(claim_chat_task(&conversation, "replacement").await.is_err());

        release_chat_task(&conversation, "current").await;
        claim_chat_task(&conversation, "replacement")
            .await
            .expect("claim after current owner releases");
        release_chat_task(&conversation, "replacement").await;
    }
}

#[cfg(test)]
mod chat_lifecycle_tests {
    use super::*;
    use async_trait::async_trait;
    use std::collections::VecDeque;
    use vortideck_core::services::mcp::{
        chat::ChatSession,
        model::{Choice, CompletionResponse, Function, FunctionCall, Tool, ToolCallInfo},
        session_actor::spawn_session_actor,
        tool::{Tool as McpTool, ToolSet},
        ChatClient,
    };

    struct ScriptedClient {
        responses: Mutex<VecDeque<CompletionResponse>>,
    }

    impl ScriptedClient {
        fn new() -> Self {
            Self {
                responses: Mutex::new(VecDeque::from([
                    completion(tool_message()),
                    completion(assistant_message("fixture complete")),
                ])),
            }
        }

        fn next_response(&self) -> CompletionResponse {
            self.responses
                .lock()
                .expect("script lock")
                .pop_front()
                .expect("scripted response")
        }
    }

    #[async_trait]
    impl ChatClient for ScriptedClient {
        fn provider_name(&self) -> &str {
            "fixture"
        }

        async fn complete(
            &self,
            _request: CompletionRequest,
        ) -> anyhow::Result<CompletionResponse> {
            Ok(self.next_response())
        }

        async fn complete_streaming(
            &self,
            _request: CompletionRequest,
        ) -> anyhow::Result<(
            tokio::sync::mpsc::Receiver<String>,
            tokio::task::JoinHandle<anyhow::Result<CompletionResponse>>,
        )> {
            let response = self.next_response();
            let content = response
                .choices
                .first()
                .and_then(|choice| choice.message.content.clone())
                .unwrap_or_default();
            let (sender, receiver) = tokio::sync::mpsc::channel(2);
            let handle = tokio::spawn(async move {
                if !content.is_empty() {
                    sender.send(content).await.expect("stream receiver");
                }
                drop(sender);
                Ok(response)
            });
            Ok((receiver, handle))
        }

        fn format_tools(&self, _tools: &[Arc<dyn McpTool>]) -> Vec<Tool> {
            vec![Tool {
                tool_type: "function".to_string(),
                function: Function {
                    name: "fixture.read".to_string(),
                    description: "deterministic read".to_string(),
                    parameters: serde_json::json!({"type": "object"}),
                },
            }]
        }
    }

    struct FixtureRead;

    #[async_trait]
    impl McpTool for FixtureRead {
        fn name(&self) -> String {
            "fixture.read".to_string()
        }

        fn description(&self) -> String {
            "deterministic read".to_string()
        }

        fn parameters(&self) -> serde_json::Value {
            serde_json::json!({"type": "object"})
        }

        async fn call(&self, _args: serde_json::Value) -> anyhow::Result<String> {
            Ok("{\"value\":\"fixture\"}".to_string())
        }
    }

    struct SlowEffect;

    #[async_trait]
    impl McpTool for SlowEffect {
        fn name(&self) -> String {
            "fixture.read".to_string()
        }
        fn description(&self) -> String {
            "slow deterministic effect".to_string()
        }
        fn parameters(&self) -> serde_json::Value {
            serde_json::json!({"type": "object"})
        }

        async fn call(&self, _args: serde_json::Value) -> anyhow::Result<String> {
            tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
            Ok("{\"applied\":true}".to_string())
        }
    }

    struct DelayedStreamingClient;

    #[async_trait]
    impl ChatClient for DelayedStreamingClient {
        fn provider_name(&self) -> &str {
            "fixture"
        }

        async fn complete(
            &self,
            _request: CompletionRequest,
        ) -> anyhow::Result<CompletionResponse> {
            Ok(completion(assistant_message("late response")))
        }

        async fn complete_streaming(
            &self,
            _request: CompletionRequest,
        ) -> anyhow::Result<(
            tokio::sync::mpsc::Receiver<String>,
            tokio::task::JoinHandle<anyhow::Result<CompletionResponse>>,
        )> {
            let (sender, receiver) = tokio::sync::mpsc::channel(1);
            let handle = tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                drop(sender);
                Ok(completion(assistant_message("late response")))
            });
            Ok((receiver, handle))
        }

        fn format_tools(&self, _tools: &[Arc<dyn McpTool>]) -> Vec<Tool> {
            Vec::new()
        }
    }

    fn assistant_message(content: &str) -> McpMessage {
        McpMessage {
            role: "assistant".to_string(),
            content: Some(content.to_string()),
            name: None,
            function_call: None,
            tool_calls: None,
            tool_call_id: None,
            refusal: None,
            annotations: Vec::new(),
        }
    }

    fn tool_message() -> McpMessage {
        let mut message = assistant_message("");
        message.tool_calls = Some(vec![ToolCallInfo {
            id: "call-1".to_string(),
            function: FunctionCall {
                name: "fixture.read".to_string(),
                arguments: "{}".to_string(),
            },
            call_type: "function".to_string(),
        }]);
        message
    }

    fn completion(message: McpMessage) -> CompletionResponse {
        CompletionResponse {
            id: uuid::Uuid::new_v4().to_string(),
            object: "chat.completion".to_string(),
            created: 0,
            model: "fixture-model".to_string(),
            choices: vec![Choice {
                index: 0,
                message,
                finish_reason: "stop".to_string(),
            }],
            usage: Usage {
                prompt_tokens: 2,
                completion_tokens: 3,
                total_tokens: 5,
                evidence: UsageEvidence::ProviderMeasured,
            },
        }
    }

    async fn run_fixture(
        streaming: bool,
    ) -> (AssistantResponse, Vec<McpMessage>, Vec<AssistantChunk>) {
        let mut tools = ToolSet::new();
        tools.add_tool(FixtureRead);
        let session = ChatSession::new(
            Arc::new(ScriptedClient::new()),
            Arc::new(tokio::sync::Mutex::new(tools)),
            AIProvider::Custom {
                name: "fixture".to_string(),
                base_url: "http://localhost".to_string(),
                api_key: None,
                models: Vec::new(),
            },
        );
        let (handle, actor) = spawn_session_actor(session);
        let chunks = Mutex::new(Vec::new());
        let emit = |chunk| chunks.lock().expect("chunk lock").push(chunk);
        let response = execute_chat_with_handle(
            &emit,
            &handle,
            if streaming {
                "streaming"
            } else {
                "nonstreaming"
            },
            if streaming {
                "task-stream"
            } else {
                "task-plain"
            },
            "run fixture".to_string(),
            "run fixture".to_string(),
            streaming,
            Arc::new(AtomicBool::new(false)),
        )
        .await
        .expect("chat lifecycle");
        let messages = handle.get_messages().await.expect("actor messages");
        handle.shutdown().await;
        actor.await.expect("actor shutdown");
        let chunks = chunks.into_inner().expect("chunk results");
        (response, messages, chunks)
    }

    #[tokio::test]
    async fn streaming_and_nonstreaming_share_tool_and_message_lifecycle() {
        let (plain_response, plain_messages, plain_chunks) = run_fixture(false).await;
        let (stream_response, stream_messages, stream_chunks) = run_fixture(true).await;

        assert_eq!(plain_response.message, "fixture complete");
        assert_eq!(stream_response.message, plain_response.message);
        assert_eq!(
            serde_json::to_value(stream_messages).expect("stream messages"),
            serde_json::to_value(plain_messages).expect("plain messages")
        );
        assert!(plain_chunks.is_empty());
        assert!(stream_chunks.iter().all(|chunk| {
            chunk.task_id == "task-stream" && chunk.conversation_id == "streaming"
        }));
        assert!(stream_chunks
            .last()
            .is_some_and(|chunk| chunk.done && !chunk.cancelled));
    }

    #[tokio::test]
    async fn cancelling_stream_aborts_without_inventing_assistant_message() {
        let session = ChatSession::new(
            Arc::new(DelayedStreamingClient),
            Arc::new(tokio::sync::Mutex::new(ToolSet::new())),
            AIProvider::Custom {
                name: "fixture".to_string(),
                base_url: "http://localhost".to_string(),
                api_key: None,
                models: Vec::new(),
            },
        );
        let (handle, actor) = spawn_session_actor(session);
        let cancellation = Arc::new(AtomicBool::new(false));
        let cancellation_trigger = cancellation.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            cancellation_trigger.store(true, std::sync::atomic::Ordering::Relaxed);
        });
        let chunks = Mutex::new(Vec::new());
        let emit = |chunk| chunks.lock().expect("chunk lock").push(chunk);

        let result = execute_chat_with_handle(
            &emit,
            &handle,
            "cancel-conversation",
            "cancel-task",
            "cancel fixture".to_string(),
            "cancel fixture".to_string(),
            true,
            cancellation,
        )
        .await;
        assert_eq!(result.unwrap_err(), "Generation cancelled by user");
        let messages = handle.get_messages().await.expect("actor messages");
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].role, "user");
        assert!(chunks
            .lock()
            .expect("chunk results")
            .last()
            .is_some_and(|chunk| chunk.done && chunk.cancelled));
        handle.shutdown().await;
        actor.await.expect("actor shutdown");
    }

    #[tokio::test]
    async fn cancelling_during_tool_records_real_effect_result_before_stopping() {
        let mut tools = ToolSet::new();
        tools.add_tool(SlowEffect);
        let client = ScriptedClient {
            responses: Mutex::new(VecDeque::from([completion(tool_message())])),
        };
        let session = ChatSession::new(
            Arc::new(client),
            Arc::new(tokio::sync::Mutex::new(tools)),
            AIProvider::Custom {
                name: "fixture".to_string(),
                base_url: "http://localhost".to_string(),
                api_key: None,
                models: Vec::new(),
            },
        );
        let (handle, actor) = spawn_session_actor(session);
        let cancellation = Arc::new(AtomicBool::new(false));
        let cancellation_trigger = cancellation.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
            cancellation_trigger.store(true, std::sync::atomic::Ordering::Relaxed);
        });

        let result = execute_chat_with_handle(
            &|_| {},
            &handle,
            "effect-conversation",
            "effect-task",
            "apply fixture".to_string(),
            "apply fixture".to_string(),
            false,
            cancellation,
        )
        .await;
        assert_eq!(
            result.unwrap_err(),
            "Generation cancelled after the active tool completed"
        );
        let messages = handle.get_messages().await.expect("actor messages");
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[1].role, "assistant");
        assert_eq!(messages[2].role, "tool");
        assert_eq!(messages[2].tool_call_id.as_deref(), Some("call-1"));
        handle.shutdown().await;
        actor.await.expect("actor shutdown");
    }
}
