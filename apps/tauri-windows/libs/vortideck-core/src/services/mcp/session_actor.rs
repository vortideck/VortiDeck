//! Session Actor Pattern Implementation
//!
//! Each ChatSession runs in its own tokio task (actor).
//! Commands are sent via channels, ensuring:
//! - No shared mutable state (no Mutex needed)
//! - Concurrent access across different sessions
//! - Sequential processing within a session
//! - Clean ownership (actor owns the session)

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

use crate::services::mcp::{
    chat::ChatSession,
    client::ChatClient,
    conversation_file::{ConversationFile, ConversationMetadata},
    model::{Message, Usage},
    provider::AIProvider,
    tool::Tool as McpTool,
};

// ============================================================================
// Types
// ============================================================================

/// Information about a tool (for returning to callers)
#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
}

/// Session state information (for queries)
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub model: String,
    pub provider_name: String,
    pub is_streaming: bool,
    pub message_count: usize,
    pub has_pending_assistant: bool,
}

/// Settings update payload - None means don't change
#[derive(Debug, Clone, Default)]
pub struct SessionSettings {
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub stream: Option<bool>,
    pub tools_mode: Option<String>,
    pub selected_tools: Option<Vec<String>>,
    pub system_prompt: Option<Option<String>>, // Some(None) = clear, Some(Some(x)) = set
    pub tool_timeout: Option<u64>,
}

/// Context needed for streaming chat outside the actor
/// This allows streaming to happen without holding the actor
pub struct ModelRequestContext {
    pub client: Arc<dyn ChatClient>,
    pub messages: Vec<Message>,
    pub tools: Vec<Arc<dyn McpTool>>,
    pub model: String,
    pub provider: AIProvider,
    pub max_tokens: u32,
    pub temperature: Option<f32>,
}

// ============================================================================
// Commands
// ============================================================================

/// Commands that can be sent to a session actor
pub enum SessionCommand {
    // === Settings ===
    UpdateSettings {
        settings: SessionSettings,
        reply: oneshot::Sender<Result<(), String>>,
    },

    // === Core Chat Operations ===
    SubmitToolResultOnly {
        tool_call_id: String,
        result: String,
        reply: oneshot::Sender<()>,
    },

    // === Tool Execution ===
    ExecuteToolSafely {
        tool_name: String,
        args: serde_json::Value,
        reply: oneshot::Sender<String>,
    },

    // === Conversation File Operations ===
    AttachConversationFile {
        file: ConversationFile,
        reply: oneshot::Sender<Result<ConversationMetadata, String>>,
    },
    LoadFromFile {
        use_context_management: bool,
        reply: oneshot::Sender<Result<(), String>>,
    },
    PersistPendingAssistant {
        reply: oneshot::Sender<()>,
    },

    // === Queries ===
    GetSessionInfo {
        reply: oneshot::Sender<SessionInfo>,
    },
    GetMessages {
        reply: oneshot::Sender<Vec<Message>>,
    },
    GetTools {
        reply: oneshot::Sender<Vec<ToolInfo>>,
    },
    GetProvider {
        reply: oneshot::Sender<AIProvider>,
    },
    GetClient {
        reply: oneshot::Sender<Arc<dyn ChatClient>>,
    },
    GetTool {
        name: String,
        reply: oneshot::Sender<Option<Arc<dyn McpTool>>>,
    },
    GetModelRequestContext {
        reply: oneshot::Sender<Result<ModelRequestContext, String>>,
    },
    PrepareModelTurn {
        saved_content: String,
        ai_content: String,
        reply: oneshot::Sender<Result<ModelRequestContext, String>>,
    },
    CommitAssistantResponse {
        message: Message,
        usage: Option<Usage>,
        reply: oneshot::Sender<()>,
    },

    // === Message Management ===
    AddSystemMessage {
        content: String,
        reply: oneshot::Sender<()>,
    },
    ClearMessages {
        reply: oneshot::Sender<()>,
    },

    // === Lifecycle ===
    Shutdown,
}

// ============================================================================
// Session Handle
// ============================================================================

/// Handle for communicating with a session actor
/// Clone is cheap - just clones the channel sender
#[derive(Clone)]
pub struct SessionHandle {
    sender: mpsc::Sender<SessionCommand>,
    cancellation_flag: Arc<AtomicBool>,
    created_at: Instant,
}

impl SessionHandle {
    fn new(sender: mpsc::Sender<SessionCommand>, cancellation_flag: Arc<AtomicBool>) -> Self {
        Self {
            sender,
            cancellation_flag,
            created_at: Instant::now(),
        }
    }

    /// Check if the actor is still alive
    pub fn is_alive(&self) -> bool {
        !self.sender.is_closed()
    }

    /// Get age of this session
    pub fn age(&self) -> Duration {
        self.created_at.elapsed()
    }

    /// Request cancellation of current operation
    pub fn cancel(&self) {
        self.cancellation_flag.store(true, Ordering::SeqCst);
    }

    /// Reset cancellation flag (call before new operation)
    pub fn reset_cancellation(&self) {
        self.cancellation_flag.store(false, Ordering::SeqCst);
    }

    /// Check if cancellation was requested
    pub fn is_cancelled(&self) -> bool {
        self.cancellation_flag.load(Ordering::SeqCst)
    }

    /// Get the cancellation flag for external use
    pub fn cancellation_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.cancellation_flag)
    }

    // ========================================================================
    // Settings
    // ========================================================================

    pub async fn update_settings(&self, settings: SessionSettings) -> Result<(), String> {
        let (reply, rx) = oneshot::channel();
        self.sender
            .send(SessionCommand::UpdateSettings { settings, reply })
            .await
            .map_err(|_| "Session actor closed".to_string())?;
        rx.await
            .map_err(|_| "Session actor dropped response".to_string())?
    }

    // ========================================================================
    // Core Chat Operations
    // ========================================================================

    pub async fn submit_tool_result_only(&self, tool_call_id: String, result: String) {
        let (reply, rx) = oneshot::channel();
        let _ = self
            .sender
            .send(SessionCommand::SubmitToolResultOnly {
                tool_call_id,
                result,
                reply,
            })
            .await;
        let _ = rx.await;
    }

    // ========================================================================
    // Conversation File Operations
    // ========================================================================

    pub async fn attach_conversation_file(
        &self,
        file: ConversationFile,
    ) -> Result<ConversationMetadata, String> {
        let (reply, rx) = oneshot::channel();
        self.sender
            .send(SessionCommand::AttachConversationFile { file, reply })
            .await
            .map_err(|_| "Session actor closed".to_string())?;
        rx.await
            .map_err(|_| "Session actor dropped response".to_string())?
    }

    pub async fn load_from_file(&self, use_context_management: bool) -> Result<(), String> {
        let (reply, rx) = oneshot::channel();
        self.sender
            .send(SessionCommand::LoadFromFile {
                use_context_management,
                reply,
            })
            .await
            .map_err(|_| "Session actor closed".to_string())?;
        rx.await
            .map_err(|_| "Session actor dropped response".to_string())?
    }

    // ========================================================================
    // Queries
    // ========================================================================

    pub async fn get_session_info(&self) -> Result<SessionInfo, String> {
        let (reply, rx) = oneshot::channel();
        self.sender
            .send(SessionCommand::GetSessionInfo { reply })
            .await
            .map_err(|_| "Session actor closed".to_string())?;
        rx.await
            .map_err(|_| "Session actor dropped response".to_string())
    }

    pub async fn get_messages(&self) -> Result<Vec<Message>, String> {
        let (reply, rx) = oneshot::channel();
        self.sender
            .send(SessionCommand::GetMessages { reply })
            .await
            .map_err(|_| "Session actor closed".to_string())?;
        rx.await
            .map_err(|_| "Session actor dropped response".to_string())
    }

    pub async fn get_tools(&self) -> Result<Vec<ToolInfo>, String> {
        let (reply, rx) = oneshot::channel();
        self.sender
            .send(SessionCommand::GetTools { reply })
            .await
            .map_err(|_| "Session actor closed".to_string())?;
        rx.await
            .map_err(|_| "Session actor dropped response".to_string())
    }

    pub async fn get_provider(&self) -> Result<AIProvider, String> {
        let (reply, rx) = oneshot::channel();
        self.sender
            .send(SessionCommand::GetProvider { reply })
            .await
            .map_err(|_| "Session actor closed".to_string())?;
        rx.await
            .map_err(|_| "Session actor dropped response".to_string())
    }

    pub async fn get_client(&self) -> Result<Arc<dyn ChatClient>, String> {
        let (reply, rx) = oneshot::channel();
        self.sender
            .send(SessionCommand::GetClient { reply })
            .await
            .map_err(|_| "Session actor closed".to_string())?;
        rx.await
            .map_err(|_| "Session actor dropped response".to_string())
    }

    pub async fn get_tool(&self, name: &str) -> Result<Option<Arc<dyn McpTool>>, String> {
        let (reply, rx) = oneshot::channel();
        self.sender
            .send(SessionCommand::GetTool {
                name: name.to_string(),
                reply,
            })
            .await
            .map_err(|_| "Session actor closed".to_string())?;
        rx.await
            .map_err(|_| "Session actor dropped response".to_string())
    }

    pub async fn get_model_request_context(&self) -> Result<ModelRequestContext, String> {
        let (reply, rx) = oneshot::channel();
        self.sender
            .send(SessionCommand::GetModelRequestContext { reply })
            .await
            .map_err(|_| "Session actor closed".to_string())?;
        rx.await
            .map_err(|_| "Session actor dropped response".to_string())?
    }

    pub async fn prepare_model_turn(
        &self,
        saved_content: String,
        ai_content: String,
    ) -> Result<ModelRequestContext, String> {
        let (reply, rx) = oneshot::channel();
        self.sender
            .send(SessionCommand::PrepareModelTurn {
                saved_content,
                ai_content,
                reply,
            })
            .await
            .map_err(|_| "Session actor closed".to_string())?;
        rx.await
            .map_err(|_| "Session actor dropped response".to_string())?
    }

    pub async fn commit_assistant_response(
        &self,
        message: Message,
        usage: Option<Usage>,
    ) -> Result<(), String> {
        let (reply, rx) = oneshot::channel();
        self.sender
            .send(SessionCommand::CommitAssistantResponse {
                message,
                usage,
                reply,
            })
            .await
            .map_err(|_| "Session actor closed".to_string())?;
        rx.await
            .map_err(|_| "Session actor dropped response".to_string())
    }

    // ========================================================================
    // Tool Execution
    // ========================================================================

    pub async fn execute_tool_safely(
        &self,
        tool_name: &str,
        args: serde_json::Value,
    ) -> Result<String, String> {
        let (reply, rx) = oneshot::channel();
        self.sender
            .send(SessionCommand::ExecuteToolSafely {
                tool_name: tool_name.to_string(),
                args,
                reply,
            })
            .await
            .map_err(|_| "Session actor closed".to_string())?;
        rx.await
            .map_err(|_| "Session actor dropped response".to_string())
    }

    pub async fn persist_pending_assistant(&self) -> Result<(), String> {
        let (reply, rx) = oneshot::channel();
        self.sender
            .send(SessionCommand::PersistPendingAssistant { reply })
            .await
            .map_err(|_| "Session actor closed".to_string())?;
        rx.await
            .map_err(|_| "Session actor dropped response".to_string())?;
        Ok(())
    }

    // ========================================================================
    // Message Management
    // ========================================================================

    pub async fn add_system_message(&self, content: String) -> Result<(), String> {
        let (reply, rx) = oneshot::channel();
        self.sender
            .send(SessionCommand::AddSystemMessage { content, reply })
            .await
            .map_err(|_| "Session actor closed".to_string())?;
        rx.await
            .map_err(|_| "Session actor dropped response".to_string())?;
        Ok(())
    }

    pub async fn clear_messages(&self) -> Result<(), String> {
        let (reply, rx) = oneshot::channel();
        self.sender
            .send(SessionCommand::ClearMessages { reply })
            .await
            .map_err(|_| "Session actor closed".to_string())?;
        rx.await
            .map_err(|_| "Session actor dropped response".to_string())?;
        Ok(())
    }

    // ========================================================================
    // Lifecycle
    // ========================================================================

    pub async fn shutdown(&self) {
        let _ = self.sender.send(SessionCommand::Shutdown).await;
    }
}

impl std::fmt::Debug for SessionHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionHandle")
            .field("alive", &self.is_alive())
            .field("age", &self.age())
            .finish()
    }
}

// ============================================================================
// Actor Spawning
// ============================================================================

/// Spawn a session actor task
/// Returns a handle for communicating with the actor
pub fn spawn_session_actor(session: ChatSession) -> (SessionHandle, JoinHandle<()>) {
    let (tx, rx) = mpsc::channel(32); // Buffer up to 32 commands
    let cancellation_flag = Arc::new(AtomicBool::new(false));
    let flag_clone = Arc::clone(&cancellation_flag);

    let handle = SessionHandle::new(tx, cancellation_flag);

    let task = tokio::spawn(async move {
        run_session_actor(session, rx, flag_clone).await;
    });

    (handle, task)
}

// ============================================================================
// Actor Event Loop
// ============================================================================

/// The actor event loop - processes commands sequentially
async fn run_session_actor(
    mut session: ChatSession,
    mut rx: mpsc::Receiver<SessionCommand>,
    _cancellation_flag: Arc<AtomicBool>,
) {
    println!(
        "🎭 Session actor started for provider: {}",
        session.get_provider().provider_name()
    );

    while let Some(cmd) = rx.recv().await {
        match cmd {
            // === Settings ===
            SessionCommand::UpdateSettings { settings, reply } => {
                let result = match apply_settings(&mut session, settings) {
                    Ok(()) => session
                        .persist_runtime_settings()
                        .await
                        .map_err(|error| error.to_string()),
                    Err(error) => Err(error),
                };
                let _ = reply.send(result);
            }

            // === Core Chat Operations ===
            SessionCommand::SubmitToolResultOnly {
                tool_call_id,
                result,
                reply,
            } => {
                session.submit_tool_result_only(tool_call_id, result).await;
                let _ = reply.send(());
            }

            // === Tool Execution ===
            SessionCommand::ExecuteToolSafely {
                tool_name,
                args,
                reply,
            } => {
                let result = session.execute_tool_safely(&tool_name, args).await;
                let _ = reply.send(result);
            }

            // === Conversation File Operations ===
            SessionCommand::AttachConversationFile { file, reply } => {
                let result = session
                    .attach_conversation_file(file)
                    .map_err(|e| e.to_string());
                let _ = reply.send(result);
            }
            SessionCommand::LoadFromFile {
                use_context_management,
                reply,
            } => {
                let result = session
                    .load_from_file(use_context_management)
                    .await
                    .map_err(|e| e.to_string());
                let _ = reply.send(result);
            }
            SessionCommand::PersistPendingAssistant { reply } => {
                session.persist_pending_assistant_if_has_content().await;
                let _ = reply.send(());
            }

            // === Queries ===
            SessionCommand::GetSessionInfo { reply } => {
                let info = SessionInfo {
                    model: session.get_model(),
                    provider_name: session.get_provider().provider_name().to_string(),
                    is_streaming: session.is_streaming(),
                    message_count: session.messages_len(),
                    has_pending_assistant: session.has_pending_assistant(),
                };
                let _ = reply.send(info);
            }
            SessionCommand::GetMessages { reply } => {
                let messages = session.get_messages().clone();
                let _ = reply.send(messages);
            }
            SessionCommand::GetTools { reply } => {
                let tools = session.get_tools().await;
                let tool_infos: Vec<ToolInfo> = tools
                    .iter()
                    .map(|t| ToolInfo {
                        name: t.name(),
                        description: t.description(),
                    })
                    .collect();
                let _ = reply.send(tool_infos);
            }
            SessionCommand::GetProvider { reply } => {
                let provider = session.get_provider().clone();
                let _ = reply.send(provider);
            }
            SessionCommand::GetClient { reply } => {
                let client = session.get_client();
                let _ = reply.send(client);
            }
            SessionCommand::GetTool { name, reply } => {
                let tool = session.get_tool(&name).await;
                let _ = reply.send(tool);
            }
            SessionCommand::GetModelRequestContext { reply } => {
                let client = session.get_client();
                let tools = session.get_tools().await;
                let model = session.get_model();
                let provider = session.get_provider().clone();
                let result = session
                    .messages_for_api(&tools)
                    .await
                    .map(|messages| ModelRequestContext {
                        client,
                        messages,
                        tools,
                        model,
                        provider,
                        max_tokens: session.effective_max_output_tokens(),
                        temperature: session.effective_temperature(),
                    })
                    .map_err(|error| error.to_string());
                let _ = reply.send(result);
            }
            SessionCommand::PrepareModelTurn {
                saved_content,
                ai_content,
                reply,
            } => {
                let tools = session.get_tools().await;
                let result = session
                    .prepare_user_turn(saved_content, ai_content)
                    .await
                    .map(|messages| ModelRequestContext {
                        client: session.get_client(),
                        messages,
                        tools,
                        model: session.get_model(),
                        provider: session.get_provider().clone(),
                        max_tokens: session.effective_max_output_tokens(),
                        temperature: session.effective_temperature(),
                    })
                    .map_err(|error| error.to_string());
                let _ = reply.send(result);
            }
            SessionCommand::CommitAssistantResponse {
                message,
                usage,
                reply,
            } => {
                session
                    .commit_assistant_response_with_usage(message, usage)
                    .await;
                let _ = reply.send(());
            }

            // === Message Management ===
            SessionCommand::AddSystemMessage { content, reply } => {
                session.add_system_message(content);
                let _ = reply.send(());
            }
            SessionCommand::ClearMessages { reply } => {
                session.clear_messages();
                let _ = reply.send(());
            }

            // === Lifecycle ===
            SessionCommand::Shutdown => {
                println!("🎭 Session actor received shutdown command");
                break;
            }
        }
    }

    println!("🎭 Session actor stopped");
}

/// Apply settings to a session
fn apply_settings(session: &mut ChatSession, settings: SessionSettings) -> Result<(), String> {
    if let Some(model) = settings.model {
        session.set_model(model);
    }
    if let Some(temperature) = settings.temperature {
        if !(0.0..=2.0).contains(&temperature) {
            return Err("temperature must be between 0 and 2".to_string());
        }
        session.set_temperature(temperature);
    }
    if let Some(max_tokens) = settings.max_tokens {
        if max_tokens == 0 {
            return Err("max_tokens must be greater than zero".to_string());
        }
        session.set_max_tokens(max_tokens);
    }
    if let Some(stream) = settings.stream {
        session.set_stream(stream);
    }
    if let Some(tools_mode) = settings.tools_mode {
        let selected = settings.selected_tools.unwrap_or_default();
        session.set_tools_filter(tools_mode, selected);
    } else if let Some(selected) = settings.selected_tools {
        // Only selected_tools provided without mode - use "selected" mode
        session.set_tools_filter("selected".to_string(), selected);
    }
    if let Some(prompt) = settings.system_prompt {
        session.set_system_prompt(prompt);
    }
    if let Some(timeout) = settings.tool_timeout {
        session.set_tool_timeout(timeout);
    }
    Ok(())
}
