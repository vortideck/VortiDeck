use crate::services::execution::{
    ButtonExecutionMode, ExecutionContext, RetryConfig, TriggerSource,
};
use crate::services::flow::commands::FlowRegistryState;
use crate::services::flow::traits::FlowNode;
use crate::services::mcp::ai_node::{
    AiNodeBehavior, AiNodeConfig, AiNodeInput, AiNodeOutput, ExecutionMetadata, FlowControl,
    LlmClient, ResponseFormat, ToolAccessPolicy, ToolAction, ToolSpec, UnifiedAiNode,
};
use crate::services::mcp::chat::ChatSession;
use crate::services::mcp::commands::ChatState;
use crate::services::mcp::tool::Tool;
use crate::services::mcp::unified_tool_executor::UnifiedToolExecutor;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Centralized AI executor for all AI node types
pub struct AiExecutor {
    flow_registry: FlowRegistryState,
    chat_state: ChatState,
    tool_executor: UnifiedToolExecutor,
}

impl AiExecutor {
    pub fn new(flow_registry: FlowRegistryState, chat_state: ChatState) -> Self {
        let tool_executor = UnifiedToolExecutor::new(flow_registry.clone(), chat_state.clone());
        Self {
            flow_registry,
            chat_state,
            tool_executor,
        }
    }

    /// Universal AI execution entry point - handles ALL AI node behaviors
    pub async fn execute_ai_node(
        &self,
        config: AiNodeConfig,
        input: AiNodeInput,
    ) -> Result<AiNodeOutput, String> {
        let start_time = std::time::Instant::now();

        println!(
            "🤖 Universal AI Executor | Behavior: {:?} | Goal: {}",
            config.behavior, input.user_message
        );
        println!("🔧 Tool Access Policy: {:?}", config.tool_access);
        println!(
            "🔧 Available tools in input: {}",
            input.available_tools.len()
        );
        for tool in &input.available_tools {
            println!("   - {} ({})", tool.name, tool.description);
        }

        // Log the unified execution path
        match config.behavior {
            AiNodeBehavior::FreeAgent => {
                println!("🌐 Free Agent mode: Full autonomy with all tools")
            }
            AiNodeBehavior::Orchestrator => println!("🎯 Orchestrator mode: Connected tools only"),
            AiNodeBehavior::TextOnly => println!("📝 Text-only mode: No tool execution"),
            AiNodeBehavior::Custom => println!("🛠️ Custom mode: User-defined behavior"),
        }

        // Create ephemeral session for this flow node execution
        let node_id = &input.flow_context.current_node_id;
        let flow_instance_id = format!("flow-{}", uuid::Uuid::new_v4()); // Unique per execution

        let provider = self.get_provider_from_config(&config).await?;
        let session_key = self
            .chat_state
            .session_manager
            .create_ephemeral(
                flow_instance_id.clone(),
                node_id.clone(),
                provider,
                config.model.clone(),
            )
            .await?;

        println!(
            "🔑 Created ephemeral session: {} for node {}",
            session_key, node_id
        );

        // Get LLM client from ephemeral session
        let llm_client = self.get_llm_client(&session_key).await?;

        // Create unified AI node with configuration
        let mut ai_node = UnifiedAiNode::new(config.clone(), llm_client);

        // Add tools based on access policy
        self.configure_tools(&mut ai_node, &config, &input, &session_key)
            .await?;

        println!(
            "🧠 About to execute AI node with behavior: {:?}",
            config.behavior
        );

        // Execute the AI node
        let mut result = ai_node
            .execute(input.clone())
            .await
            .map_err(|e| format!("AI node execution failed: {}", e))?;

        println!(
            "🧠 AI Planning completed | Generated {} actions",
            result.actions.len()
        );

        if !result.actions.is_empty() {
            println!("📋 AI Action Plan:");
            for (i, action) in result.actions.iter().enumerate() {
                println!(
                    "   {}. {} with args: {}",
                    i + 1,
                    action.tool_name,
                    action.args
                );
            }
        } else {
            println!("⚠️ NO ACTIONS GENERATED - AI provided text response only");
            println!("🔍 AI Response: {:?}", result.outputs.get("response"));
        }

        // Execute tool actions if any using the unified tool executor
        if !result.actions.is_empty() {
            println!("🔧 Starting tool execution phase...");

            let execution_context = ExecutionContext {
                trigger_source: TriggerSource::Flow {
                    flow_id: input.flow_context.flow_run_id.clone(),
                    node_id: input.flow_context.current_node_id.clone(),
                },
                user_id: None,
                session_id: format!(
                    "flow-ai:{}:{}",
                    input.flow_context.flow_run_id, input.flow_context.current_node_id
                ),
                variables: input.previous_outputs.clone(),
                timeout_ms: Some(30_000),
                retry_config: RetryConfig {
                    max_retries: 0,
                    retry_delay_ms: 0,
                    exponential_backoff: false,
                },
                execution_mode: ButtonExecutionMode::RunOnce,
                execution_config: None,
            };

            let tool_results = self
                .tool_executor
                .execute_tool_actions(&result.actions, &execution_context)
                .await;

            println!("🔧 Tool execution phase completed");

            result.outputs.insert(
                "tool_results".to_string(),
                serde_json::to_value(&tool_results)
                    .map_err(|error| format!("Failed to serialize tool results: {error}"))?,
            );

            // Verify completion based on actual tool execution results
            let accepted_actions = tool_results
                .iter()
                .filter(|execution| execution.succeeded)
                .count();
            let applied_actions = tool_results
                .iter()
                .filter(|execution| {
                    execution.status
                        == crate::services::mcp::unified_tool_executor::ToolExecutionStatus::Applied
                })
                .count();

            println!("✅ TOOL RECEIPT SUMMARY:");
            println!("   Planned actions: {}", result.actions.len());
            println!("   Accepted actions: {}", accepted_actions);
            println!("   Confirmed applied actions: {}", applied_actions);
            println!(
                "   Acceptance rate: {:.1}%",
                (accepted_actions as f64 / result.actions.len() as f64) * 100.0
            );

            if applied_actions == result.actions.len() {
                println!("🎉 ALL STEPS CONFIRMED APPLIED!");
            } else if accepted_actions == result.actions.len() {
                println!("ℹ️ All tool calls were accepted; some effects await stronger receipts");
            } else {
                println!("⚠️ Some tool calls failed - check individual receipts");
            }
        } else {
            println!("ℹ️ No tool actions generated - AI provided text response only");
        }

        // Update execution metadata
        result.metadata.execution_time_ms = start_time.elapsed().as_millis() as u64;
        result.metadata.tools_called = result.actions.len() as u32;
        result.metadata.config_used = config;

        println!(
            "✅ AI Executor completed | Total time: {:?} | Actions: {} | Outputs: {}",
            start_time.elapsed(),
            result.actions.len(),
            result.outputs.len()
        );

        // ✅ CRITICAL: Cleanup ephemeral session after execution
        self.chat_state
            .session_manager
            .remove_ephemeral(&flow_instance_id, node_id)
            .await;
        println!("🗑️ Cleaned up ephemeral session for node {}", node_id);

        Ok(result)
    }

    /// Configure tools for the AI node based on access policy
    async fn configure_tools(
        &self,
        ai_node: &mut UnifiedAiNode,
        config: &AiNodeConfig,
        input: &AiNodeInput,
        session_key: &str,
    ) -> Result<(), String> {
        match &config.tool_access {
            ToolAccessPolicy::AllTools => {
                println!("🌐 Configuring ALL available tools (Free Agent mode)");

                // Use unified tool executor to get all available tools
                let all_tools = self.tool_executor.get_all_available_tools().await?;

                for tool_spec in &all_tools {
                    if let Some(tool_adapter) = self
                        .create_tool_adapter_from_spec(tool_spec, session_key)
                        .await
                    {
                        ai_node.add_tool(tool_adapter);
                        println!(
                            "   ✅ Added tool: {} (Type: {:?})",
                            tool_spec.name, tool_spec.tool_type
                        );
                    }
                }

                println!("📋 Free Agent configured with {} tools", all_tools.len());
            }
            ToolAccessPolicy::ConnectedOnly => {
                println!("🔒 Configuring connected tools only (Orchestrator mode)");

                // Add only tools specified in input.available_tools
                for tool_spec in &input.available_tools {
                    if let Some(tool) = self.find_tool_by_spec(tool_spec, session_key).await {
                        ai_node.add_tool(tool);
                        println!("   ✅ Added connected tool: {}", tool_spec.name);
                    }
                }
            }
            ToolAccessPolicy::Whitelist(allowed_tools) => {
                println!("📋 Configuring whitelisted tools (Strict mode)");

                for tool_name in allowed_tools {
                    if let Some(tool) = self.find_tool_by_name(tool_name, session_key).await {
                        ai_node.add_tool(tool);
                        println!("   ✅ Added whitelisted tool: {}", tool_name);
                    }
                }
            }
            ToolAccessPolicy::NoTools => {
                println!("🚫 No tools configured (Text-only mode)");
            }
        }

        Ok(())
    }

    // Tool execution methods removed - now using UnifiedToolExecutor

    // Mapping functions removed - no longer needed since we use direct tool IDs

    /// Get AI provider from node config
    async fn get_provider_from_config(
        &self,
        _config: &AiNodeConfig,
    ) -> Result<crate::services::mcp::AIProvider, String> {
        // Get provider from the first configured model
        let config_guard = self.chat_state.config.lock().await;
        if let Some(mcp_config) = config_guard.as_ref() {
            // Get the first model config and convert to AIProvider
            if let Some(model_config) = mcp_config.models.model.first() {
                return crate::services::mcp::provider::AIProvider::from_model_config(model_config)
                    .map_err(|e| format!("Failed to create provider from config: {}", e));
            }
        }

        // Fallback: return error if no provider configured
        Err("No AI provider configured. Please set up a provider in settings.".to_string())
    }

    /// Get LLM client from session
    async fn get_llm_client(&self, session_key: &str) -> Result<Arc<dyn LlmClient>, String> {
        let handle = self
            .chat_state
            .session_manager
            .get(session_key)
            .await
            .ok_or_else(|| format!("Session not found: {}", session_key))?;

        let chat_client = handle.get_client().await?;
        let provider = handle.get_provider().await?;
        Ok(Arc::new(ChatClientWrapper::new(chat_client, provider)) as Arc<dyn LlmClient>)
    }

    /// Find tool by specification
    async fn find_tool_by_spec(
        &self,
        tool_spec: &ToolSpec,
        session_key: &str,
    ) -> Option<Arc<dyn Tool>> {
        // Try system tool first
        if let Some(system_tool) = self.create_system_tool_adapter(&tool_spec.name).await {
            return Some(system_tool);
        }

        // Try MCP tool from session
        let handle = self.chat_state.session_manager.get(session_key).await?;

        if let Ok(Some(mcp_tool)) = handle.get_tool(&tool_spec.name).await {
            return Some(mcp_tool);
        }

        None
    }

    /// Find tool by name
    async fn find_tool_by_name(&self, tool_name: &str, session_key: &str) -> Option<Arc<dyn Tool>> {
        self.find_tool_by_spec(
            &ToolSpec {
                name: tool_name.to_string(),
                description: String::new(),
                parameters: Value::Null,
                node_id: String::new(),
                tool_type: crate::services::mcp::ai_node::ToolType::SystemTool,
            },
            session_key,
        )
        .await
    }

    /// Create system tool adapter
    async fn create_system_tool_adapter(&self, tool_name: &str) -> Option<Arc<dyn Tool>> {
        // Get the system node without holding the lock across await
        let (system_node, parameters_schema) = {
            let registry = self.flow_registry.registry.lock().ok()?;
            let system_node = registry.get_node_by_name(tool_name)?;
            let parameters_schema = system_node.parameters_schema();
            (system_node, parameters_schema)
        };

        let adapter = crate::services::mcp::commands::SystemToolAdapter::new(
            tool_name.to_string(),
            format!("System tool: {}", tool_name),
            parameters_schema,
            system_node,
        );

        Some(Arc::new(adapter))
    }

    /// Create system tool adapter with specific ID (for consistent tool naming)
    async fn create_system_tool_adapter_with_id(
        &self,
        tool_id: &str,
        display_name: &str,
    ) -> Option<Arc<dyn Tool>> {
        // Get the system node without holding the lock across await
        let (system_node, parameters_schema) = {
            let registry = self.flow_registry.registry.lock().ok()?;
            let system_node = registry.get_node(tool_id)?;
            let parameters_schema = system_node.parameters_schema();
            (system_node, parameters_schema)
        };

        let adapter = crate::services::mcp::commands::SystemToolAdapter::new(
            tool_id.to_string(), // Use the actual tool ID as the name
            format!("System tool: {}", display_name),
            parameters_schema,
            system_node,
        );

        Some(Arc::new(adapter))
    }

    /// Create a tool adapter from a ToolSpec (unified approach)
    async fn create_tool_adapter_from_spec(
        &self,
        tool_spec: &ToolSpec,
        session_key: &str,
    ) -> Option<Arc<dyn Tool>> {
        match &tool_spec.tool_type {
            crate::services::mcp::ai_node::ToolType::SystemTool => {
                // Create system tool adapter
                let (system_node, parameters_schema) = {
                    let registry = self.flow_registry.registry.lock().ok()?;
                    let system_node = registry.get_node(&tool_spec.name)?;
                    let parameters_schema = system_node.parameters_schema();
                    (system_node, parameters_schema)
                };

                let adapter = crate::services::mcp::commands::SystemToolAdapter::new(
                    tool_spec.name.clone(),
                    tool_spec.description.clone(),
                    parameters_schema,
                    system_node,
                );

                Some(Arc::new(adapter))
            }
            crate::services::mcp::ai_node::ToolType::McpTool { server_name: _ } => {
                // Get MCP tool from session using actor handle
                match self.chat_state.session_manager.get(session_key).await {
                    Some(handle) => handle.get_tool(&tool_spec.name).await.ok().flatten(),
                    _ => None,
                }
            }
            crate::services::mcp::ai_node::ToolType::FlowNode => {
                let schema_tool = crate::services::mcp::commands::ToolSchemaAdapter::new(
                    tool_spec.name.clone(),
                    tool_spec.description.clone(),
                    tool_spec.parameters.clone(),
                );
                Some(Arc::new(schema_tool))
            }
        }
    }
}

/// Chat client wrapper that implements LlmClient
struct ChatClientWrapper {
    client: Arc<dyn crate::services::mcp::ChatClient>,
    provider: crate::services::mcp::AIProvider,
}

impl ChatClientWrapper {
    fn new(
        client: Arc<dyn crate::services::mcp::ChatClient>,
        provider: crate::services::mcp::AIProvider,
    ) -> Self {
        Self { client, provider }
    }
}

#[async_trait::async_trait]
impl LlmClient for ChatClientWrapper {
    async fn generate_response(
        &self,
        prompt: String,
        model: Option<&str>,
    ) -> anyhow::Result<String> {
        // Create a temporary session for simple text generation with the specified model
        let tool_set = Arc::new(tokio::sync::Mutex::new(
            crate::services::mcp::tool::ToolSet::new(),
        ));
        let mut session = if let Some(model_name) = model {
            println!("🤖 Using AI model: {}", model_name);
            ChatSession::new(self.client.clone(), tool_set, self.provider.clone())
                .with_model(model_name)
        } else {
            ChatSession::new(self.client.clone(), tool_set, self.provider.clone())
        };
        let response = session.send_message(prompt).await?;
        Ok(response.content.unwrap_or_default())
    }

    async fn generate_with_tools(
        &self,
        prompt: String,
        tools: Vec<ToolSpec>,
        model: Option<&str>,
    ) -> anyhow::Result<AiNodeOutput> {
        println!("🔧 Sending {} tools to OpenAI API:", tools.len());
        if let Some(model_name) = model {
            println!("🤖 Using AI model: {}", model_name);
        }
        for tool in &tools {
            println!("   - {} ({})", tool.name, tool.description);
        }

        if tools.is_empty() {
            println!("⚠️ WARNING: No tools provided to OpenAI API - AI will not be able to call any tools!");
        }

        // Create a temporary session with tools and specified model
        let mut tool_set = crate::services::mcp::tool::ToolSet::new();

        // Store the mapping from tool index to original tool spec for later lookup
        let mut tool_specs_by_index = Vec::new();
        // Also create reverse mapping from sanitized API names to original tool specs
        let mut tool_specs_by_sanitized_name: std::collections::HashMap<String, ToolSpec> =
            std::collections::HashMap::new();

        // Add tools to the tool set using their actual names
        for (index, tool_spec) in tools.iter().enumerate() {
            // The chat session needs schemas only. Action execution is performed
            // later by UnifiedToolExecutor against the authoritative registries.
            let schema_tool = crate::services::mcp::commands::ToolSchemaAdapter::new(
                tool_spec.name.clone(),
                tool_spec.description.clone(),
                tool_spec.parameters.clone(),
            );
            tool_set.add_tool(schema_tool);

            // Store the tool spec by index for later mapping (legacy format)
            tool_specs_by_index.push(tool_spec.clone());

            // Store by sanitized name (dots replaced with underscores for OpenAI compliance)
            let sanitized_name = tool_spec.name.replace('.', "_");
            tool_specs_by_sanitized_name.insert(sanitized_name.clone(), tool_spec.clone());

            println!(
                "📋 Added tool to AI: {} (sanitized: {})",
                tool_spec.name, sanitized_name
            );
        }

        let tool_set_wrapped = Arc::new(tokio::sync::Mutex::new(tool_set));
        let mut session = if let Some(model_name) = model {
            ChatSession::new(self.client.clone(), tool_set_wrapped, self.provider.clone())
                .with_model(model_name)
        } else {
            ChatSession::new(self.client.clone(), tool_set_wrapped, self.provider.clone())
        };

        println!(
            "📤 Sending prompt to OpenAI (first 300 chars): {}",
            if prompt.len() > 300 {
                &prompt[..300]
            } else {
                &prompt
            }
        );

        // Send message (tools are automatically included by ChatSession)
        let response = session.send_message(prompt).await?;

        println!("📥 Received response from OpenAI");
        println!(
            "   Content: {:?}",
            response
                .content
                .as_ref()
                .map(|c| if c.len() > 200 { &c[..200] } else { c })
        );
        println!(
            "   Tool calls: {:?}",
            response.tool_calls.as_ref().map(|tc| tc.len())
        );
        println!(
            "   Function call: {:?}",
            response.function_call.as_ref().map(|fc| &fc.name)
        );

        // Parse tool actions with proper index mapping and sanitized name mapping
        let actions = self.parse_tool_actions_from_response(
            &response,
            &session,
            &tool_specs_by_index,
            &tool_specs_by_sanitized_name,
        );

        println!("🔧 Parsed {} tool actions from response", actions.len());
        for action in &actions {
            println!("   - {} -> {}", action.tool_name, action.target_node_id);
        }

        let mut outputs = HashMap::new();
        let content = response.content.unwrap_or_default();
        outputs.insert("response".to_string(), Value::String(content.clone()));

        let tools_called_count = actions.len() as u32;

        Ok(AiNodeOutput {
            actions,
            outputs,
            reasoning: Some(content),
            flow_control: Some(FlowControl {
                branch_condition: None,
                should_continue: true,
                next_node_override: None,
            }),
            metadata: ExecutionMetadata {
                execution_time_ms: 0,
                tokens_used: None,
                tools_called: tools_called_count,
                config_used: AiNodeConfig {
                    behavior: AiNodeBehavior::FreeAgent,
                    tool_access: ToolAccessPolicy::AllTools,
                    response_format: ResponseFormat::Structured,
                    custom_prompt: None,
                    model: model.map(|s| s.to_string()),
                    input_mode: None,
                    prompt_template: None,
                },
            },
        })
    }
}

/// Canonical format for tool calls - all API formats normalize to this
#[derive(Debug, Clone)]
struct NormalizedToolCall {
    id: String,
    name: String,
    arguments: Value,
}

/// Normalize OpenAI tool_calls format to canonical format
fn normalize_openai_tool_calls(
    tool_calls: &[crate::services::mcp::model::ToolCallInfo],
) -> Vec<NormalizedToolCall> {
    tool_calls
        .iter()
        .map(|call| {
            let args: Value = serde_json::from_str(&call.function.arguments)
                .unwrap_or_else(|_| Value::String(call.function.arguments.clone()));

            NormalizedToolCall {
                id: call.id.clone(),
                name: call.function.name.clone(),
                arguments: args,
            }
        })
        .collect()
}

/// Normalize OpenAI function_call format to canonical format
fn normalize_openai_function_call(
    function_call: &crate::services::mcp::model::FunctionCall,
) -> Vec<NormalizedToolCall> {
    let args: Value = serde_json::from_str(&function_call.arguments)
        .unwrap_or_else(|_| Value::String(function_call.arguments.clone()));

    vec![NormalizedToolCall {
        id: uuid::Uuid::new_v4().to_string(),
        name: function_call.name.clone(),
        arguments: args,
    }]
}

/// Normalize JSON content format to canonical format
fn normalize_json_content(content: &str) -> Vec<NormalizedToolCall> {
    // Extract JSON from markdown code blocks or plain text
    let json_content = if content.contains("```json") {
        if let Some(start) = content.find("```json") {
            let search_from = start + 7;
            if let Some(end_relative) = content[search_from..].find("```") {
                let json_start = search_from;
                let json_end = search_from + end_relative;
                content[json_start..json_end].trim()
            } else {
                content.trim()
            }
        } else {
            content.trim()
        }
    } else {
        content.trim()
    };

    // Parse JSON and extract actions
    if let Ok(parsed_json) = serde_json::from_str::<Value>(json_content) {
        if let Some(actions_array) = parsed_json.get("actions").and_then(|v| v.as_array()) {
            return actions_array
                .iter()
                .filter_map(|action| {
                    let tool_name = action.get("tool")?.as_str()?;
                    let args = action
                        .get("args")
                        .cloned()
                        .unwrap_or(Value::Object(serde_json::Map::new()));

                    Some(NormalizedToolCall {
                        id: action
                            .get("id")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned)
                            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
                        name: tool_name.to_string(),
                        arguments: args,
                    })
                })
                .collect();
        }
    }

    Vec::new()
}

/// Parse normalized tool calls into ToolActions using sanitized name mapping
/// This is the SINGLE place where tool name mapping logic lives
fn parse_normalized_tool_calls(
    normalized_calls: Vec<NormalizedToolCall>,
    tool_specs_by_index: &[ToolSpec],
    tool_specs_by_sanitized_name: &std::collections::HashMap<String, ToolSpec>,
) -> Vec<ToolAction> {
    normalized_calls
        .into_iter()
        .filter_map(|call| {
            let tool_name = &call.name;

            // Handle tool names that might have "functions." prefix
            let clean_tool_name = if tool_name.starts_with("functions.") {
                &tool_name[10..] // Remove "functions." prefix
            } else {
                tool_name.as_str()
            };

            println!("🔍 Mapping tool name: {} -> {}", tool_name, clean_tool_name);

            // Try sanitized name mapping first (for tools with dots converted to underscores)
            if let Some(tool_spec) = tool_specs_by_sanitized_name.get(clean_tool_name) {
                println!(
                    "✅ Found tool by sanitized name: {} -> {}",
                    clean_tool_name, tool_spec.name
                );
                return Some(ToolAction {
                    call_id: call.id,
                    tool_name: tool_spec.name.clone(),
                    args: call.arguments,
                    target_node_id: tool_spec.node_id.clone(),
                });
            }

            // Fallback: Try legacy indexed name format (tool_0, tool_1, etc.)
            if let Some(index_str) = clean_tool_name.strip_prefix("tool_") {
                if let Ok(index) = index_str.parse::<usize>() {
                    if let Some(tool_spec) = tool_specs_by_index.get(index) {
                        println!(
                            "✅ Mapped indexed name {} -> {}",
                            clean_tool_name, tool_spec.name
                        );
                        return Some(ToolAction {
                            call_id: call.id,
                            tool_name: tool_spec.name.clone(),
                            args: call.arguments,
                            target_node_id: tool_spec.node_id.clone(),
                        });
                    }
                }
            }

            // Fallback: Try direct name/ID matching
            for tool_spec in tool_specs_by_index {
                if tool_spec.name == clean_tool_name || tool_spec.node_id == clean_tool_name {
                    println!(
                        "✅ Found tool by direct name/ID: {} -> {}",
                        clean_tool_name, tool_spec.name
                    );
                    return Some(ToolAction {
                        call_id: call.id,
                        tool_name: tool_spec.name.clone(),
                        args: call.arguments,
                        target_node_id: tool_spec.node_id.clone(),
                    });
                }
            }

            println!(
                "❌ Tool name '{}' not found in available tools",
                clean_tool_name
            );
            None
        })
        .collect()
}

#[cfg(test)]
mod tool_call_identity_tests {
    use super::*;
    use crate::services::mcp::model::{FunctionCall, ToolCallInfo};

    #[test]
    fn repeated_tool_names_preserve_provider_call_ids_and_arguments() {
        let calls = vec![
            ToolCallInfo {
                id: "call-a".to_string(),
                function: FunctionCall {
                    name: "repeat_tool".to_string(),
                    arguments: "{\"value\":1}".to_string(),
                },
                call_type: "function".to_string(),
            },
            ToolCallInfo {
                id: "call-b".to_string(),
                function: FunctionCall {
                    name: "repeat_tool".to_string(),
                    arguments: "{\"value\":2}".to_string(),
                },
                call_type: "function".to_string(),
            },
        ];
        let spec = ToolSpec {
            name: "repeat.tool".to_string(),
            description: "fixture".to_string(),
            parameters: serde_json::json!({}),
            node_id: "repeat.tool".to_string(),
            tool_type: crate::services::mcp::ai_node::ToolType::McpTool {
                server_name: "fixture".to_string(),
            },
        };
        let mapping = std::collections::HashMap::from([("repeat_tool".to_string(), spec)]);

        let actions =
            parse_normalized_tool_calls(normalize_openai_tool_calls(&calls), &[], &mapping);

        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].call_id, "call-a");
        assert_eq!(actions[0].args, serde_json::json!({"value": 1}));
        assert_eq!(actions[1].call_id, "call-b");
        assert_eq!(actions[1].args, serde_json::json!({"value": 2}));
    }
}

impl ChatClientWrapper {
    /// Parse tool actions from AI response using industry-standard normalization pattern
    /// All API formats are normalized to a canonical format, then parsed once
    fn parse_tool_actions_from_response(
        &self,
        response: &crate::services::mcp::model::Message,
        session: &ChatSession,
        tool_specs_by_index: &[ToolSpec],
        tool_specs_by_sanitized_name: &std::collections::HashMap<String, ToolSpec>,
    ) -> Vec<ToolAction> {
        // Step 1: Normalize different API formats to canonical format
        let normalized_calls = if let Some(api_tool_calls) = &response.tool_calls {
            println!("📞 Normalizing tool_calls format");
            normalize_openai_tool_calls(api_tool_calls)
        } else if let Some(function_call) = &response.function_call {
            println!("📞 Normalizing function_call format");
            normalize_openai_function_call(function_call)
        } else if let Some(content) = &response.content {
            println!("📞 Normalizing JSON content format");
            normalize_json_content(content)
        } else {
            println!("⚠️ No tool_calls, function_call, or content found in response");
            Vec::new()
        };

        // Step 2: Parse canonical format once (all mapping logic in one place)
        if normalized_calls.is_empty() {
            println!("⚠️ No normalized tool calls extracted");
            return Vec::new();
        }

        println!(
            "📋 Parsing {} normalized tool calls",
            normalized_calls.len()
        );
        parse_normalized_tool_calls(
            normalized_calls,
            tool_specs_by_index,
            tool_specs_by_sanitized_name,
        )
    }
}

// Mapping functions removed - no longer needed since we use direct tool IDs
