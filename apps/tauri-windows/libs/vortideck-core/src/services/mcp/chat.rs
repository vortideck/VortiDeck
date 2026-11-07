use anyhow::Result;
use chrono::Utc;
use rmcp::model::Role;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use uuid::Uuid;

use crate::services::mcp::{
    client::{ChatClient, OpenAIClient},
    conversation_file::{
        estimate_message_tokens, ConversationFile, ConversationMetadata, StoredMessage,
        TokenSource, ToolsMode,
    },
    model::{CompletionRequest, Function, Message, Tool, Usage, UsageEvidence},
    provider::AIProvider,
    tool::{Tool as McpTool, ToolSet},
};
use tracing;

// Chat session for managing conversations with AI
#[derive(Clone)]
pub struct ChatSession {
    client: Arc<dyn ChatClient>,
    tools: Arc<tokio::sync::Mutex<ToolSet>>, // Shared tool set - updates dynamically as MCP servers connect
    messages: Vec<Message>,
    model: String,
    provider: AIProvider,
    stream: bool, // Enable/disable streaming mode
    temperature: f32,
    max_tokens: u32,

    // Tool filtering settings
    tools_mode: String,          // "all", "selected", or "none"
    selected_tools: Vec<String>, // List of tool names when tools_mode is "selected"

    // User-customizable system prompt (None = use default)
    system_prompt: Option<String>,

    // Tool execution timeout (in seconds)
    tool_timeout: u64,

    // Persistence
    conversation_file: Option<Arc<tokio::sync::Mutex<ConversationFile>>>,

    // Track unpersisted assistant message with tool_calls
    // We delay persisting until tool results are ready to prevent history corruption
    has_pending_tool_assistant: bool,
    pending_assistant_usage: Option<Usage>,
}

impl std::fmt::Debug for ChatSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChatSession")
            .field("provider", &self.provider.provider_name())
            .field("model", &self.model)
            .field("tools", &self.tools)
            .field("messages", &self.messages.len())
            .finish()
    }
}

impl ChatSession {
    pub fn new(
        client: Arc<dyn ChatClient>,
        tools: Arc<tokio::sync::Mutex<ToolSet>>,
        provider: AIProvider,
    ) -> Self {
        let default_model = provider.default_model().to_string();

        let session = Self {
            client,
            tools,
            messages: Vec::new(),
            model: default_model,
            provider,
            stream: false, // Default to streaming disabled (requires SSE parsing)
            temperature: 0.7,
            max_tokens: 4000,
            tools_mode: "all".to_string(), // Default to all tools
            selected_tools: Vec::new(),
            system_prompt: None, // Default: use built-in prompt
            tool_timeout: 30,    // Default: 30 seconds
            conversation_file: None,
            has_pending_tool_assistant: false,
            pending_assistant_usage: None,
        };

        session
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    pub fn set_model(&mut self, model: impl Into<String>) {
        self.model = model.into();
    }

    pub fn with_stream(mut self, stream: bool) -> Self {
        self.stream = stream;
        self
    }

    pub fn set_stream(&mut self, stream: bool) {
        self.stream = stream;
    }

    pub fn set_temperature(&mut self, temperature: f32) {
        self.temperature = temperature;
    }

    pub fn set_max_tokens(&mut self, max_tokens: u32) {
        self.max_tokens = max_tokens;
    }

    /// Update stream setting and persist to file if attached
    pub async fn update_stream(&mut self, stream: bool) -> Result<()> {
        self.stream = stream;

        // Update conversation file metadata if attached
        if let Some(ref file_mutex) = self.conversation_file {
            let mut file = file_mutex.lock().await;
            file.update_stream(stream)
                .map_err(|e| anyhow::anyhow!("Failed to update stream in file: {}", e))?;
        }

        Ok(())
    }

    /// Set tool filtering mode and selected tools
    pub fn set_tools_filter(&mut self, tools_mode: String, selected_tools: Vec<String>) {
        println!(
            "🔧 Setting tools filter: mode={}, selected_count={}",
            tools_mode,
            selected_tools.len()
        );
        self.tools_mode = tools_mode;
        self.selected_tools = selected_tools;
    }

    /// Set custom system prompt
    pub fn set_system_prompt(&mut self, prompt: Option<String>) {
        self.system_prompt = prompt;
    }

    pub fn set_tool_timeout(&mut self, timeout: u64) {
        self.tool_timeout = timeout;
    }

    // Check if model supports temperature parameter (uses provider metadata)
    fn supports_temperature(&self) -> bool {
        self.provider
            .get_model_info(&self.model)
            .map(|info| info.supports_temperature)
            .unwrap_or(true) // Default to true for unknown models
    }

    pub fn effective_max_output_tokens(&self) -> u32 {
        let context_window = self
            .provider
            .get_model_info(&self.model)
            .map(|model| model.context_window)
            .unwrap_or(4096);
        self.max_tokens.min((context_window / 2).max(1) as u32)
    }

    pub fn effective_temperature(&self) -> Option<f32> {
        self.supports_temperature().then_some(self.temperature)
    }

    pub fn add_system_prompt(&mut self, content: String) {
        self.messages.push(Message::system(content));
    }

    pub fn add_system_message(&mut self, content: String) {
        self.messages.push(Message::system(content));
    }

    async fn format_tools_for_api(&self) -> Vec<Tool> {
        const MAX_TOOLS_OPENAI: usize = 128; // OpenAI API hard limit

        // Get all available tools
        let tools_guard = self.tools.lock().await;
        let all_tools = tools_guard.tools();

        // Filter tools based on tools_mode setting - keep as Arc<dyn Tool>
        let filtered_tools: Vec<std::sync::Arc<dyn crate::services::mcp::tool::Tool>> =
            match self.tools_mode.as_str() {
                "none" => {
                    println!("🔧 Tools mode: NONE - No tools will be sent to API");
                    vec![]
                }
                "selected" => {
                    println!(
                        "🔧 Tools mode: SELECTED - Filtering to {} selected tools",
                        self.selected_tools.len()
                    );
                    all_tools
                        .iter()
                        .filter(|tool| self.selected_tools.contains(&tool.name()))
                        .cloned()
                        .collect()
                }
                _ => {
                    // Default to "all"
                    println!(
                        "🔧 Tools mode: ALL - Using all {} available tools",
                        all_tools.len()
                    );
                    all_tools.clone()
                }
            };

        // Check if provider has tool limit (OpenAI/Azure only)
        let provider_name = self.provider.provider_name().to_lowercase();
        let has_tool_limit = provider_name.contains("openai") || provider_name.contains("azure");

        // Apply OpenAI limit if necessary
        let final_tools = if has_tool_limit && filtered_tools.len() > MAX_TOOLS_OPENAI {
            println!(
                "⚠️ Tool count ({}) exceeds {} API limit ({}), limiting to first {} tools",
                filtered_tools.len(),
                provider_name,
                MAX_TOOLS_OPENAI,
                MAX_TOOLS_OPENAI
            );
            filtered_tools.into_iter().take(MAX_TOOLS_OPENAI).collect()
        } else {
            filtered_tools
        };

        println!(
            "📤 Sending {} tools to {} API",
            final_tools.len(),
            provider_name
        );

        // Format tools for API
        self.client.format_tools(&final_tools)
    }

    pub async fn send_message(&mut self, content: String) -> Result<Message> {
        // Default behavior: save and send the same message
        let (message, _usage) = self
            .send_message_with_context(content.clone(), content)
            .await?;
        Ok(message)
    }

    /// Append and persist the user-visible message, then build the provider
    /// context with the optionally enriched final user message.
    ///
    /// Streaming and non-streaming turns both enter through this method so
    /// selecting a transport cannot change conversation persistence.
    pub async fn prepare_user_turn(
        &mut self,
        saved_content: String,
        ai_content: String,
    ) -> Result<Vec<Message>> {
        let user_message = Message::user(saved_content);
        self.messages.push(user_message.clone());

        if let Err(error) = self.persist_message(&user_message).await {
            println!("⚠️ Failed to persist user message: {error}");
        }

        let tools = self.get_tools().await;
        let mut api_messages = self.messages_for_api(&tools).await?;
        if let Some(last_message) = api_messages.last_mut() {
            *last_message = Message::user(ai_content);
        }
        Ok(api_messages)
    }

    /// Build provider context from committed conversation state.
    pub async fn messages_for_api(
        &self,
        tools: &[Arc<dyn crate::services::mcp::tool::Tool>],
    ) -> Result<Vec<Message>> {
        let context_window = self
            .provider
            .get_model_info(&self.model)
            .map(|model| model.context_window)
            .unwrap_or(4096);
        let output_reserve = self.effective_max_output_tokens() as usize;
        let system_tokens = 16 + self.effective_system_prompt().chars().count().div_ceil(3);
        let tool_tokens: usize = tools
            .iter()
            .map(|tool| {
                12 + (tool.name().len()
                    + tool.description().len()
                    + tool.parameters().to_string().len())
                .div_ceil(3)
            })
            .sum();
        let history_budget = context_window
            .saturating_sub(output_reserve)
            .saturating_sub(system_tokens)
            .saturating_sub(tool_tokens)
            .saturating_sub(128);
        if history_budget < 128 {
            return Err(anyhow::anyhow!(
                "model context window ({context_window}) is too small for the system prompt, tools and output reserve"
            ));
        }

        let mut messages = if let Some(file_mutex) = &self.conversation_file {
            let file = file_mutex.lock().await;
            file.load_for_context(history_budget, 30)?
                .into_iter()
                .map(|message| self.stored_to_message(message))
                .collect()
        } else {
            self.messages.clone()
        };
        if !messages.iter().any(|message| message.role == "system") {
            messages.insert(0, Message::system(self.effective_system_prompt()));
        }
        Ok(messages)
    }

    fn effective_system_prompt(&self) -> String {
        self.system_prompt.clone().unwrap_or_else(|| {
            "You are a helpful AI assistant integrated into VortiDeck, a stream deck application.

Your role is to help users by:

1. Suggesting appropriate commands, executables, and arguments
2. Filling form fields with intelligent defaults
3. Explaining program options and settings
4. Providing tips and best practices
5. Understanding context from the current form/input

When suggesting commands or values:
- Be specific and practical
- Consider the user's operating system context
- Provide executable paths that are commonly available
- Suggest arguments that are safe and useful
- Explain why you're suggesting something

You have access to MCP tools that you can use to help users. When asked about your available tools, you can list them. Use tools when they would be helpful to complete the user's request.

Keep responses concise but helpful. Be friendly and supportive."
                .to_string()
        })
    }

    /// Commit the complete provider message using the same atomic tool-call
    /// persistence policy for every response transport.
    pub async fn commit_assistant_response(&mut self, message: Message) {
        self.commit_assistant_response_with_usage(message, None)
            .await;
    }

    pub async fn commit_assistant_response_with_usage(
        &mut self,
        message: Message,
        usage: Option<Usage>,
    ) {
        self.messages.push(message.clone());

        let has_tool_calls = message
            .tool_calls
            .as_ref()
            .is_some_and(|tool_calls| !tool_calls.is_empty());

        if has_tool_calls {
            println!(
                "📌 Assistant message has tool_calls - deferring persistence until tool results ready"
            );
            self.has_pending_tool_assistant = true;
            self.pending_assistant_usage = usage;
            return;
        }

        if let Err(error) = self
            .persist_message_with_usage(&message, usage.as_ref())
            .await
        {
            println!("⚠️ Failed to persist assistant message: {error}");
        }

        if self.messages.len() == 2 {
            if let Err(error) = self.auto_generate_title_from_first_message().await {
                println!("⚠️ Failed to auto-generate title: {error}");
            }
        }
    }

    /// Send message with separate content for saving vs AI context
    /// - `saved_content`: Original message to save to conversation file
    /// - `ai_content`: Enhanced message with context to send to AI
    pub async fn send_message_with_context(
        &mut self,
        saved_content: String,
        ai_content: String,
    ) -> Result<(Message, crate::services::mcp::model::Usage)> {
        // Format tools for API
        let tools = self.format_tools_for_api().await;
        let api_messages = self.prepare_user_turn(saved_content, ai_content).await?;

        // Debug: Log what tools are being sent to the API
        println!(
            "🔧 Sending {} tools to {} API:",
            tools.len(),
            self.client.provider_name()
        );
        for tool in &tools {
            println!(
                "   • {} ({})",
                tool.function.name, tool.function.description
            );
        }

        // Get response from AI (using api_messages with context, not self.messages)
        let response = self
            .client
            .complete(CompletionRequest {
                model: self.model.clone(),
                messages: api_messages,
                temperature: if self.supports_temperature() {
                    Some(self.temperature)
                } else {
                    None
                },
                max_tokens: Some(self.effective_max_output_tokens()),
                tools: Some(tools),
                stream: Some(self.stream),
                stream_options: None,
            })
            .await?;

        // Add assistant's response to history
        let message = response
            .choices
            .first()
            .ok_or_else(|| anyhow::anyhow!("No response from AI"))?
            .message
            .clone();
        self.commit_assistant_response_with_usage(message.clone(), Some(response.usage.clone()))
            .await;

        Ok((message, response.usage))
    }

    /// Submit tool result WITHOUT getting a follow-up response from AI
    /// Used when processing multiple tool calls before getting the next AI response
    pub async fn submit_tool_result_only(&mut self, tool_call_id: String, result: String) {
        // CRITICAL: If we have a pending assistant message with tool_calls,
        // persist it FIRST before the tool result. This ensures atomic persistence:
        // assistant message with tool_calls is always followed by tool result in file.
        if self.has_pending_tool_assistant {
            let pending_usage = self.pending_assistant_usage.clone();
            // Find the last assistant message (should be the one with tool_calls)
            if let Some(assistant_msg) = self.messages.iter().rev().find(|m| {
                m.role == "assistant"
                    && m.tool_calls
                        .as_ref()
                        .map(|tc| !tc.is_empty())
                        .unwrap_or(false)
            }) {
                println!("📝 Persisting deferred assistant message with tool_calls");
                if let Err(e) = self
                    .persist_message_with_usage(assistant_msg, pending_usage.as_ref())
                    .await
                {
                    println!("⚠️ Failed to persist deferred assistant message: {}", e);
                }
            }
            self.has_pending_tool_assistant = false;
            self.pending_assistant_usage = None;
        }

        let tool_message = Message::tool(&tool_call_id, &result);
        self.messages.push(tool_message.clone());

        // Persist tool result to file (critical - prevents history corruption on reload)
        if let Err(e) = self.persist_message(&tool_message).await {
            println!("⚠️ Failed to persist tool result message: {}", e);
        }
    }

    /// Execute a tool safely with timeout and guaranteed response injection
    /// Returns the tool result string (either success or error)
    pub async fn execute_tool_safely(&self, tool_name: &str, args: serde_json::Value) -> String {
        use std::time::Duration;

        // Find the tool
        let tool = match self.get_tool(tool_name).await {
            Some(t) => t,
            None => {
                return serde_json::json!({
                    "error": true,
                    "message": format!("Tool '{}' not found", tool_name)
                })
                .to_string();
            }
        };

        // Execute with timeout from settings
        let result = tokio::time::timeout(
            Duration::from_secs(self.tool_timeout),
            tool.call(args.clone()),
        )
        .await;

        match result {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => {
                serde_json::json!({
                    "error": true,
                    "message": format!("Tool execution failed: {}", e)
                }).to_string()
            }
            Err(_) => {
                serde_json::json!({
                    "error": true,
                    "message": format!("Tool '{}' timed out after {} seconds", tool_name, self.tool_timeout)
                }).to_string()
            }
        }
    }

    pub async fn get_tool(&self, name: &str) -> Option<Arc<dyn McpTool>> {
        let tools_guard = self.tools.lock().await;
        tools_guard.get_tool(name).map(|t| t.clone())
    }

    /// Get tool by API function name (converts API name back to original tool name)
    pub async fn get_tool_by_api_name(&self, api_name: &str) -> Option<Arc<dyn McpTool>> {
        // Parse the index from the API name (format: tool_<index>)
        if let Some(index_str) = api_name.strip_prefix("tool_") {
            if let Ok(index) = index_str.parse::<usize>() {
                let tools_guard = self.tools.lock().await;
                return tools_guard.tools().get(index).cloned();
            }
        }
        None
    }

    pub async fn get_tools(&self) -> Vec<Arc<dyn McpTool>> {
        let tools_guard = self.tools.lock().await;
        tools_guard.tools()
    }

    // Public getters for creating fresh sessions
    pub fn get_client(&self) -> Arc<dyn ChatClient> {
        self.client.clone()
    }

    pub fn get_provider(&self) -> &AIProvider {
        &self.provider
    }

    pub fn get_tool_set(&self) -> Arc<tokio::sync::Mutex<ToolSet>> {
        self.tools.clone()
    }

    pub fn get_model(&self) -> String {
        self.model.clone()
    }

    // Add methods to access private fields
    pub fn get_messages(&self) -> &Vec<Message> {
        &self.messages
    }

    pub fn clear_messages(&mut self) {
        self.messages.clear();
    }

    pub fn messages_len(&self) -> usize {
        self.messages.len()
    }

    pub fn last_message(&self) -> Option<&Message> {
        self.messages.last()
    }

    pub async fn tools_len(&self) -> usize {
        let tools_guard = self.tools.lock().await;
        tools_guard.len()
    }

    pub fn is_streaming(&self) -> bool {
        self.stream
    }

    /// Check if there's a pending assistant message that hasn't been persisted yet
    pub fn has_pending_assistant(&self) -> bool {
        self.has_pending_tool_assistant
    }

    /// Close a deferred assistant response when its tools will not be executed.
    /// Useful text is persisted without the unfulfilled tool calls; an empty
    /// protocol-only item is removed. This keeps both memory and reopened
    /// history valid after cancellation or an iteration limit.
    pub async fn persist_pending_assistant_if_has_content(&mut self) {
        if !self.has_pending_tool_assistant {
            return;
        }

        let pending_index = self.messages.iter().rposition(|message| {
            message.role == "assistant"
                && message
                    .tool_calls
                    .as_ref()
                    .is_some_and(|tool_calls| !tool_calls.is_empty())
        });

        if let Some(index) = pending_index {
            let has_content = self.messages[index]
                .content
                .as_ref()
                .is_some_and(|content| !content.trim().is_empty());
            if has_content {
                let mut terminal_message = self.messages[index].clone();
                terminal_message.tool_calls = None;
                self.messages[index] = terminal_message.clone();
                println!("📝 Persisting terminal assistant content without unfulfilled tools");
                if let Err(error) = self.persist_message(&terminal_message).await {
                    println!("⚠️ Failed to persist terminal assistant message: {error}");
                }
            } else {
                self.messages.remove(index);
                println!("⏭️ Removed empty assistant item with unfulfilled tools");
            }
        }

        self.has_pending_tool_assistant = false;
        self.pending_assistant_usage = None;
    }

    // ========== Persistence Methods ==========

    /// Enable persistence for this session (builder pattern - consumes self)
    pub fn with_conversation_file(mut self, file: ConversationFile) -> Self {
        self.conversation_file = Some(Arc::new(tokio::sync::Mutex::new(file)));
        self
    }

    /// Apply conversation settings to this session
    /// Validates settings before applying them
    pub fn apply_settings(&mut self, metadata: &ConversationMetadata) -> Result<()> {
        // Validate settings first
        metadata
            .validate()
            .map_err(|e| anyhow::anyhow!("Invalid conversation settings: {}", e))?;

        // Apply validated settings
        self.stream = metadata.stream;
        self.model = metadata.model.clone();
        self.temperature = metadata.temperature;
        self.max_tokens = metadata.max_tokens;
        self.tools_mode = metadata.tools_mode.as_str().to_string();
        self.selected_tools = metadata.selected_tools.clone();
        self.system_prompt = metadata.system_prompt.clone();
        self.tool_timeout = metadata.tool_timeout;

        println!("⚙️  Applied conversation settings:");
        println!(
            "   Model: {}, Provider: {}",
            metadata.model, metadata.provider
        );
        println!(
            "   Stream: {}, Temp: {}, Max tokens: {}",
            metadata.stream, metadata.temperature, metadata.max_tokens
        );
        println!(
            "   Tools: {} ({} selected)",
            metadata.tools_mode.as_str(),
            metadata.selected_tools.len()
        );

        Ok(())
    }

    pub async fn persist_runtime_settings(&self) -> Result<()> {
        if let Some(file_mutex) = &self.conversation_file {
            let mut file = file_mutex.lock().await;
            let tools_mode = match self.tools_mode.as_str() {
                "none" => ToolsMode::None,
                "selected" => ToolsMode::Selected,
                _ => ToolsMode::All,
            };
            file.update_runtime_settings(
                self.model.clone(),
                self.temperature,
                self.max_tokens,
                tools_mode,
                self.selected_tools.clone(),
                self.system_prompt.clone(),
                self.tool_timeout,
                self.stream,
            )?;
        }
        Ok(())
    }

    /// Attach conversation file to existing session and restore its settings
    /// Returns the conversation metadata so frontend can update UI
    pub fn attach_conversation_file(
        &mut self,
        file: ConversationFile,
    ) -> Result<ConversationMetadata> {
        let metadata = file.metadata().clone();

        // Apply and validate settings
        self.apply_settings(&metadata)?;

        println!(
            "📎 Conversation file attached: {} (ID: {})",
            metadata.title,
            &metadata.id[..8]
        );

        self.conversation_file = Some(Arc::new(tokio::sync::Mutex::new(file)));
        Ok(metadata)
    }

    /// Load messages from file
    pub async fn load_from_file(
        &mut self,
        use_context_management: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref file_mutex) = self.conversation_file {
            let file = file_mutex.lock().await;

            // Keep the complete canonical transcript in the actor. Context is
            // projected against the current model budget for every request.
            let _ = use_context_management;
            let stored_messages = file.load_all()?;

            // Convert StoredMessage -> Message
            self.messages = stored_messages
                .into_iter()
                .map(|sm| self.stored_to_message(sm))
                .collect();
        }

        Ok(())
    }

    /// Save message after sending (called internally)
    async fn persist_message(&self, msg: &Message) -> Result<(), Box<dyn std::error::Error>> {
        self.persist_message_with_usage(msg, None).await
    }

    async fn persist_message_with_usage(
        &self,
        msg: &Message,
        usage: Option<&Usage>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref file_mutex) = self.conversation_file {
            let mut file = file_mutex.lock().await;
            let stored = self.message_to_stored(msg, usage);
            file.append_message(&stored)?;
        }
        Ok(())
    }

    /// Convert Message to StoredMessage
    fn message_to_stored(&self, msg: &Message, usage: Option<&Usage>) -> StoredMessage {
        let measured_tokens = usage
            .filter(|usage| usage.evidence == UsageEvidence::ProviderMeasured)
            .and_then(|usage| usize::try_from(usage.completion_tokens).ok())
            .filter(|tokens| *tokens > 0);
        let mut stored = StoredMessage {
            id: Uuid::new_v4().to_string(),
            role: msg.role.clone(),
            content: msg.content.clone(),
            timestamp: Utc::now().to_rfc3339(),
            tokens: None,
            token_source: if measured_tokens.is_some() {
                TokenSource::ProviderMeasured
            } else {
                TokenSource::Estimated
            },
            model: Some(self.model.clone()),
            tool_calls: msg
                .tool_calls
                .as_ref()
                .and_then(|tc| serde_json::to_value(tc).ok()),
            tool_call_id: msg.tool_call_id.clone(),
        };
        stored.tokens = measured_tokens.or_else(|| Some(estimate_message_tokens(&stored)));
        stored
    }

    /// Convert StoredMessage to Message
    fn stored_to_message(&self, sm: StoredMessage) -> Message {
        Message {
            role: sm.role,
            content: sm.content,
            name: None,
            function_call: None,
            tool_calls: sm.tool_calls.and_then(|tc| serde_json::from_value(tc).ok()),
            tool_call_id: sm.tool_call_id,
            refusal: None,
            annotations: vec![],
        }
    }

    /// Get conversation ID if persistence is enabled
    pub async fn get_conversation_id(&self) -> Option<String> {
        if let Some(ref file_mutex) = self.conversation_file {
            let file = file_mutex.lock().await;
            Some(file.id.clone())
        } else {
            None
        }
    }

    /// Update conversation title
    pub async fn update_conversation_title(
        &self,
        title: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref file_mutex) = self.conversation_file {
            let mut file = file_mutex.lock().await;
            file.update_title(title)?;
        }
        Ok(())
    }

    /// Auto-generate title from first user message (industry standard)
    pub async fn auto_generate_title_from_first_message(
        &self,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Only generate if we have a conversation file and this is the first message
        if self.messages.len() >= 2 {
            // Need at least user + assistant message
            if let Some(first_user_msg) = self.messages.iter().find(|m| m.role == "user") {
                if let Some(content) = &first_user_msg.content {
                    // Generate AI-powered title
                    let title = self.generate_smart_title(content).await?;
                    self.update_conversation_title(title).await?;
                    println!("📝 Auto-generated conversation title with AI");
                }
            }
        }
        Ok(())
    }

    async fn generate_smart_title(
        &self,
        first_message: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // Use AI to generate a concise, descriptive title
        let title_prompt = format!(
            "Generate a very short, concise title (max 5-6 words) for a conversation that starts with: \"{}\"\n\nRespond with ONLY the title, no quotes, no explanation.",
            if first_message.len() > 200 {
                &first_message[..200]
            } else {
                first_message
            }
        );

        // Create title generation message
        let title_messages = vec![Message::user(title_prompt)];

        // Use a reliable model for title generation (fallback to gpt-4 if current model might fail)
        let title_model = if self.model.to_lowercase().contains("codex")
            || self.model.to_lowercase().contains("embedding")
            || self.model.to_lowercase().contains("tts")
            || self.model.to_lowercase().contains("whisper")
        {
            // Use safe fallback for non-chat models
            "gpt-4".to_string()
        } else {
            // Use current model
            self.model.clone()
        };

        // Send request to AI provider (separate session - no tools)
        let response = self
            .client
            .complete(CompletionRequest {
                model: title_model.clone(),
                messages: title_messages,
                temperature: if self.supports_temperature() {
                    Some(0.3)
                } else {
                    None
                }, // Lower temperature for more consistent titles (if supported)
                max_tokens: Some(32),
                tools: None,         // No tools needed for title generation
                stream: Some(false), // Disable streaming for title generation
                stream_options: None,
            })
            .await?;

        // Extract title from response
        let title = response
            .choices
            .first()
            .and_then(|choice| choice.message.content.clone())
            .unwrap_or_else(|| "New Chat".to_string())
            .trim()
            .to_string();

        // Fallback to truncation if AI fails or returns empty
        if title.is_empty() || title.len() > 100 {
            Ok(if first_message.len() > 50 {
                format!("{}...", &first_message[..50])
            } else {
                first_message.to_string()
            })
        } else {
            Ok(title)
        }
    }
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;
    use crate::services::mcp::model::{CompletionResponse, FunctionCall, ToolCallInfo};
    use async_trait::async_trait;

    #[derive(Debug)]
    struct UnusedClient;

    #[async_trait]
    impl ChatClient for UnusedClient {
        fn provider_name(&self) -> &str {
            "fixture"
        }

        async fn complete(&self, _request: CompletionRequest) -> Result<CompletionResponse> {
            panic!("lifecycle fixture does not call the provider")
        }

        async fn complete_streaming(
            &self,
            _request: CompletionRequest,
        ) -> Result<(
            tokio::sync::mpsc::Receiver<String>,
            tokio::task::JoinHandle<Result<CompletionResponse>>,
        )> {
            panic!("lifecycle fixture does not call the provider")
        }

        fn format_tools(&self, _tools: &[Arc<dyn McpTool>]) -> Vec<Tool> {
            Vec::new()
        }
    }

    fn session_with_file(directory: &Path) -> (ChatSession, std::path::PathBuf) {
        let file = ConversationFile::new(
            directory,
            "Lifecycle fixture".to_string(),
            "fixture-model".to_string(),
            "fixture".to_string(),
        )
        .expect("conversation file");
        let path = file.file_path.clone();
        let mut session = ChatSession::new(
            Arc::new(UnusedClient),
            Arc::new(tokio::sync::Mutex::new(ToolSet::new())),
            AIProvider::Custom {
                name: "fixture".to_string(),
                base_url: "http://localhost".to_string(),
                api_key: None,
                models: Vec::new(),
            },
        )
        .with_conversation_file(file);
        // Avoid title generation; this system entry is request context only and
        // is deliberately not persisted by the fixture.
        session.add_system_message("fixture system prompt".to_string());
        (session, path)
    }

    fn assistant_message(content: &str) -> Message {
        Message {
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

    #[tokio::test]
    async fn prepared_and_committed_turn_survives_reopen() {
        let directory = tempfile::tempdir().expect("temporary conversation directory");
        let (mut session, path) = session_with_file(directory.path());

        let request_messages = session
            .prepare_user_turn(
                "visible user text".to_string(),
                "enriched AI text".to_string(),
            )
            .await
            .expect("prepare user turn");
        assert_eq!(
            request_messages
                .last()
                .and_then(|message| message.content.as_deref()),
            Some("enriched AI text")
        );

        session
            .commit_assistant_response_with_usage(
                assistant_message("final answer"),
                Some(Usage {
                    prompt_tokens: 8,
                    completion_tokens: 3,
                    total_tokens: 11,
                    evidence: UsageEvidence::ProviderMeasured,
                }),
            )
            .await;

        let reopened = ConversationFile::open(path).expect("reopen conversation");
        let messages = reopened.load_all().expect("load persisted messages");
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[0].content.as_deref(), Some("visible user text"));
        assert_eq!(messages[1].role, "assistant");
        assert_eq!(messages[1].content.as_deref(), Some("final answer"));
        assert_eq!(messages[1].tokens, Some(3));
        assert_eq!(messages[1].token_source, TokenSource::ProviderMeasured);
    }

    #[tokio::test]
    async fn tool_call_and_result_are_persisted_as_one_valid_sequence() {
        let directory = tempfile::tempdir().expect("temporary conversation directory");
        let (mut session, path) = session_with_file(directory.path());
        session
            .prepare_user_turn("use a tool".to_string(), "use a tool".to_string())
            .await
            .expect("prepare user turn");

        let mut assistant = assistant_message("");
        assistant.tool_calls = Some(vec![ToolCallInfo {
            id: "call-1".to_string(),
            function: FunctionCall {
                name: "fixture.read".to_string(),
                arguments: "{}".to_string(),
            },
            call_type: "function".to_string(),
        }]);
        session.commit_assistant_response(assistant).await;

        let before_result = ConversationFile::open(path.clone())
            .expect("reopen before result")
            .load_all()
            .expect("load before result");
        assert_eq!(before_result.len(), 1);

        session
            .submit_tool_result_only("call-1".to_string(), "{\"ok\":true}".to_string())
            .await;

        let messages = ConversationFile::open(path)
            .expect("reopen after result")
            .load_all()
            .expect("load after result");
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[1].role, "assistant");
        assert_eq!(
            messages[1]
                .tool_calls
                .as_ref()
                .and_then(serde_json::Value::as_array)
                .map(Vec::len),
            Some(1)
        );
        assert_eq!(messages[2].role, "tool");
        assert_eq!(messages[2].tool_call_id.as_deref(), Some("call-1"));
    }

    #[tokio::test]
    async fn cancelled_pending_tool_call_never_reopens_as_an_orphan() {
        let directory = tempfile::tempdir().expect("temporary conversation directory");
        let (mut session, path) = session_with_file(directory.path());
        session
            .prepare_user_turn("cancel this".to_string(), "cancel this".to_string())
            .await
            .expect("prepare user turn");

        let mut assistant = assistant_message("I could not finish this request.");
        assistant.tool_calls = Some(vec![ToolCallInfo {
            id: "unfulfilled".to_string(),
            function: FunctionCall {
                name: "fixture.effect".to_string(),
                arguments: "{}".to_string(),
            },
            call_type: "function".to_string(),
        }]);
        session.commit_assistant_response(assistant).await;
        session.persist_pending_assistant_if_has_content().await;

        let messages = ConversationFile::open(path)
            .expect("reopen cancelled turn")
            .load_all()
            .expect("load cancelled turn");
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[1].role, "assistant");
        assert!(messages[1].tool_calls.is_none());
        assert_eq!(
            messages[1].content.as_deref(),
            Some("I could not finish this request.")
        );
    }

    #[tokio::test]
    async fn model_change_recomputes_context_budget_and_keeps_current_task() {
        use crate::services::mcp::provider::ModelInfo;

        let directory = tempfile::tempdir().expect("temporary conversation directory");
        let file = ConversationFile::new(
            directory.path(),
            "Model switch".to_string(),
            "large".to_string(),
            "fixture".to_string(),
        )
        .expect("conversation file");
        let conversation_path = file.file_path.clone();
        let provider = AIProvider::Custom {
            name: "fixture".to_string(),
            base_url: "http://localhost".to_string(),
            api_key: None,
            models: vec![
                ModelInfo {
                    id: "large".to_string(),
                    name: "Large".to_string(),
                    description: String::new(),
                    context_window: 8192,
                    supports_temperature: true,
                    supports_streaming: true,
                    supports_tools: true,
                },
                ModelInfo {
                    id: "small".to_string(),
                    name: "Small".to_string(),
                    description: String::new(),
                    context_window: 1024,
                    supports_temperature: true,
                    supports_streaming: true,
                    supports_tools: true,
                },
            ],
        };
        let mut session = ChatSession::new(
            Arc::new(UnusedClient),
            Arc::new(tokio::sync::Mutex::new(ToolSet::new())),
            provider,
        )
        .with_model("large")
        .with_conversation_file(file);
        session.add_system_message("fixture system prompt".to_string());
        session.max_tokens = 256;
        for index in 0..18 {
            session
                .prepare_user_turn(
                    format!("historical task {index} {}", "x".repeat(120)),
                    format!("historical task {index} {}", "x".repeat(120)),
                )
                .await
                .expect("large model accepts turn");
            session
                .commit_assistant_response(assistant_message(&format!(
                    "historical answer {index} {}",
                    "y".repeat(120)
                )))
                .await;
        }
        session
            .prepare_user_turn(
                "CURRENT TASK: finish the unfinished patch".to_string(),
                "CURRENT TASK: finish the unfinished patch".to_string(),
            )
            .await
            .expect("current task");

        let large = session.messages_for_api(&[]).await.expect("large context");
        session.set_model("small");
        let small = session.messages_for_api(&[]).await.expect("small context");

        assert!(small.len() < large.len());
        let small_context_tokens: usize = small
            .iter()
            .map(|message| {
                let stored = session.message_to_stored(message, None);
                estimate_message_tokens(&stored)
            })
            .sum();
        assert!(
            small_context_tokens + session.effective_max_output_tokens() as usize + 128 <= 1024
        );
        assert!(small.iter().any(|message| {
            message
                .content
                .as_deref()
                .is_some_and(|content| content.contains("CURRENT TASK"))
        }));
        assert!(small.iter().any(|message| {
            message
                .content
                .as_deref()
                .is_some_and(|content| content.contains("durable conversation checkpoint"))
        }));

        let reopened = ConversationFile::open(conversation_path).expect("reopen with provider");
        let smaller_provider = AIProvider::Custom {
            name: "replacement-provider".to_string(),
            base_url: "http://localhost".to_string(),
            api_key: None,
            models: vec![ModelInfo {
                id: "large".to_string(),
                name: "Same model ID, smaller provider window".to_string(),
                description: String::new(),
                context_window: 1024,
                supports_temperature: true,
                supports_streaming: true,
                supports_tools: true,
            }],
        };
        let mut replacement_session = ChatSession::new(
            Arc::new(UnusedClient),
            Arc::new(tokio::sync::Mutex::new(ToolSet::new())),
            smaller_provider,
        )
        .with_model("large")
        .with_conversation_file(reopened);
        replacement_session.max_tokens = 256;
        replacement_session
            .load_from_file(false)
            .await
            .expect("restore canonical transcript");
        let provider_limited = replacement_session
            .messages_for_api(&[])
            .await
            .expect("replacement provider context");
        assert!(provider_limited.len() < large.len());
        assert!(provider_limited.iter().any(|message| {
            message
                .content
                .as_deref()
                .is_some_and(|content| content.contains("CURRENT TASK"))
        }));
    }
}
