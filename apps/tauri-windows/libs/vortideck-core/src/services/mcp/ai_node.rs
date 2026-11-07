use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

use crate::services::mcp::tool::Tool;

/// Unified AI Node Configuration - replaces separate node types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AiNodeConfig {
    /// The behavior mode of this AI node
    pub behavior: AiNodeBehavior,
    /// Tool access policy
    pub tool_access: ToolAccessPolicy,
    /// Response format requirements
    pub response_format: ResponseFormat,
    /// Custom system prompt override
    pub custom_prompt: Option<String>,
    /// AI model to use for this node (e.g., "gpt-4", "gpt-3.5-turbo")
    pub model: Option<String>,
    /// How to interpret inputs for TextOnly behavior
    pub input_mode: Option<InputMode>,
    /// Custom prompt template for CustomTemplate mode
    pub prompt_template: Option<PromptTemplate>,
}

/// Different AI node behaviors
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AiNodeBehavior {
    /// Full autonomy - can use any available tools, make decisions
    FreeAgent,
    /// Orchestrator mode - restricted to connected tools, follows flow context
    Orchestrator,

    /// Text-only mode - no tool access, pure text generation
    TextOnly,
    /// Custom behavior defined by prompt
    Custom,
}

/// Tool access policies
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ToolAccessPolicy {
    /// Access to all available tools
    AllTools,
    /// Only tools connected in the flow
    ConnectedOnly,
    /// Specific whitelist of tools
    Whitelist(Vec<String>),
    /// No tools allowed
    NoTools,
}

/// Response format requirements
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ResponseFormat {
    /// Natural language response
    Natural,
    /// Structured JSON output
    Structured,
    /// Tool calls only
    ToolCallsOnly,
    /// Custom format
    Custom(String),
}

/// Input interpretation mode for AI nodes (modern template-first approach)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InputMode {
    /// Treat input as conversational - "Someone said: {input}"
    Conversational,
    /// Treat input as a task to execute - "Execute this task: {input}"
    TaskExecution,
    /// Use custom template for complete control over prompting (preferred)
    CustomTemplate,
}

/// Pre-defined prompt templates for common use cases
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PromptTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    /// Template with placeholders: {input}, {context}, {previous_outputs}
    pub template: String,
    /// Whether to include context and previous outputs automatically
    pub include_context: bool,
    /// Category for organization
    pub category: String,
}

/// Built-in templates following MCP best practices
impl PromptTemplate {
    pub fn direct_execution() -> Self {
        Self {
            id: "direct_execution".to_string(),
            name: "Direct Execution".to_string(),
            description: "Execute the task directly without conversational wrapper".to_string(),
            template: "{input}".to_string(),
            include_context: false,
            category: "Task Execution".to_string(),
        }
    }

    pub fn prompt_generator() -> Self {
        Self {
            id: "prompt_generator".to_string(),
            name: "AI Prompt Generator".to_string(),
            description: "Generate prompts - outputs ONLY the prompt text".to_string(),
            template: "Generate a detailed, creative prompt for: {input}\n\nIMPORTANT: Output ONLY the prompt text. No commentary, no explanations, no conversation - just the prompt itself.".to_string(),
            include_context: false,
            category: "Content Generation".to_string(),
        }
    }

    pub fn creative_executor() -> Self {
        Self {
            id: "creative_executor".to_string(),
            name: "Creative Content Executor".to_string(),
            description: "Execute creative writing tasks - outputs ONLY the content".to_string(),
            template: "Task: {input}\n\nExecute this creative task. Output ONLY the requested content. No commentary, no conversation, no meta-discussion - just create what was asked for.".to_string(),
            include_context: false,
            category: "Creative Writing".to_string(),
        }
    }

    pub fn story_writer() -> Self {
        Self {
            id: "story_writer".to_string(),
            name: "Story Writer".to_string(),
            description: "Write stories based on prompts - pure content output".to_string(),
            template: "Write a story: {input}\n\nWrite the story directly. No introduction, no commentary, no 'Here's a story...' - just start writing the story itself.".to_string(),
            include_context: false,
            category: "Creative Writing".to_string(),
        }
    }

    pub fn content_only() -> Self {
        Self {
            id: "content_only".to_string(),
            name: "Content-Only Output".to_string(),
            description: "Generate requested content without any wrapper text".to_string(),
            template: "{input}\n\nGenerate exactly what was requested. No prefacing text, no explanations, no conversational elements. Output only the pure content.".to_string(),
            include_context: false,
            category: "Content Generation".to_string(),
        }
    }

    pub fn conversational() -> Self {
        Self {
            id: "conversational".to_string(),
            name: "Conversational".to_string(),
            description: "Natural conversation style".to_string(),
            template: "Someone said: \"{input}\"\n\nRespond naturally as if you're having a friendly conversation.".to_string(),
            include_context: true,
            category: "Conversation".to_string(),
        }
    }

    /// Get all built-in templates
    pub fn get_builtin_templates() -> Vec<PromptTemplate> {
        vec![
            Self::direct_execution(),
            Self::prompt_generator(),
            Self::creative_executor(),
            Self::story_writer(),
            Self::content_only(),
            Self::conversational(),
        ]
    }
}

/// Input structure for the Unified AI Node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiNodeInput {
    /// The user's message or goal
    pub user_message: String,
    /// Outputs from previous nodes in the flow
    pub previous_outputs: HashMap<String, Value>,
    /// Memory/context from previous flow steps
    pub context_memory: Vec<String>,
    /// Available tools (filtered based on access policy)
    pub available_tools: Vec<ToolSpec>,
    /// Expected branching conditions for IF nodes
    pub branching_conditions: Option<Vec<String>>,
    /// Flow metadata
    pub flow_context: FlowContext,
    /// Node configuration
    pub config: AiNodeConfig,
}

/// Specification of a tool available to the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String, // Display name for the AI
    pub description: String,
    pub parameters: Value,
    pub node_id: String,     // The flow node ID - this is the primary identifier
    pub tool_type: ToolType, // Whether it's a system tool, MCP tool, etc.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolType {
    SystemTool,
    McpTool { server_name: String },
    FlowNode, // Direct flow node execution
}

/// Flow context information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowContext {
    pub flow_run_id: String,
    pub current_node_id: String,
    pub next_node_types: Vec<String>, // "tool", "if", "summary", etc.
    pub expected_outputs: Vec<String>, // Variable names expected by next nodes
}

/// Output structure from the Unified AI Node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiNodeOutput {
    /// Tools the LLM wants to execute
    pub actions: Vec<ToolAction>,
    /// Variables to pass to next nodes
    pub outputs: HashMap<String, Value>,
    /// Reasoning/explanation for debugging
    pub reasoning: Option<String>,
    /// Flow control decisions
    pub flow_control: Option<FlowControl>,
    /// Execution metadata
    pub metadata: ExecutionMetadata,
}

/// Execution metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetadata {
    pub execution_time_ms: u64,
    pub tokens_used: Option<u32>,
    pub tools_called: u32,
    pub config_used: AiNodeConfig,
}

/// A tool action the LLM wants to execute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolAction {
    /// Stable identity of this individual model-planned call. Tool names are
    /// not identities because one response may invoke the same tool repeatedly.
    #[serde(default = "new_tool_call_id")]
    pub call_id: String,
    pub tool_name: String, // Tool name for logging/debugging
    pub args: Value,
    pub target_node_id: String, // The node ID to execute - this is required
}

fn new_tool_call_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Flow control decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowControl {
    pub branch_condition: Option<String>,
    pub should_continue: bool,
    pub next_node_override: Option<String>,
}

/// The Unified AI Node implementation
pub struct UnifiedAiNode {
    pub config: AiNodeConfig,
    pub available_tools: HashMap<String, Arc<dyn Tool>>,
    pub llm_client: Arc<dyn LlmClient>,
}

/// Trait for LLM clients (OpenAI, Anthropic, etc.)
#[async_trait::async_trait]
pub trait LlmClient: Send + Sync {
    async fn generate_response(&self, prompt: String, model: Option<&str>) -> Result<String>;
    async fn generate_with_tools(
        &self,
        prompt: String,
        tools: Vec<ToolSpec>,
        model: Option<&str>,
    ) -> Result<AiNodeOutput>;
}

impl UnifiedAiNode {
    pub fn new(config: AiNodeConfig, llm_client: Arc<dyn LlmClient>) -> Self {
        Self {
            config,
            available_tools: HashMap::new(),
            llm_client,
        }
    }

    /// Add a tool that this node can use
    pub fn add_tool(&mut self, tool: Arc<dyn Tool>) {
        self.available_tools.insert(tool.name(), tool);
    }

    /// Filter tools based on access policy
    pub fn filter_tools(&self, input: &AiNodeInput) -> Vec<ToolSpec> {
        match &self.config.tool_access {
            ToolAccessPolicy::AllTools => {
                // Return all available tools
                self.available_tools
                    .values()
                    .map(|tool| ToolSpec {
                        name: tool.name(),
                        description: tool.description(),
                        parameters: tool.parameters(),
                        node_id: format!("mcp_tool_{}", tool.name()), // Generate node ID for MCP tools
                        tool_type: ToolType::McpTool {
                            server_name: "default".to_string(),
                        },
                    })
                    .collect()
            }
            ToolAccessPolicy::ConnectedOnly => {
                // Return only tools connected in the flow
                input.available_tools.clone()
            }
            ToolAccessPolicy::Whitelist(allowed_tools) => {
                // Return only whitelisted tools
                input
                    .available_tools
                    .iter()
                    .filter(|tool| allowed_tools.contains(&tool.name))
                    .cloned()
                    .collect()
            }
            ToolAccessPolicy::NoTools => {
                // Return no tools
                Vec::new()
            }
        }
    }

    /// Generate the system prompt based on input and configuration
    pub fn generate_system_prompt(&self, input: &AiNodeInput) -> String {
        // Use custom prompt if provided
        if let Some(custom_prompt) = &self.config.custom_prompt {
            return custom_prompt.clone();
        }

        // Check if we should use a custom template (regardless of behavior)
        if let Some(input_mode) = &self.config.input_mode {
            if matches!(input_mode, InputMode::CustomTemplate) {
                if let Some(template) = &self.config.prompt_template {
                    let context_info = if !input.context_memory.is_empty() {
                        format!(
                            "\n## Previous Context:\n{}",
                            input.context_memory.join("\n")
                        )
                    } else {
                        String::new()
                    };

                    let data_info = if !input.previous_outputs.is_empty() {
                        format!(
                            "\n## Available Data:\n{}",
                            serde_json::to_string_pretty(&input.previous_outputs)
                                .unwrap_or_default()
                        )
                    } else {
                        String::new()
                    };

                    return self.apply_template(template, input, &context_info, &data_info);
                }
            }
        }

        // For TextOnly behavior, use conversational prompting
        if matches!(self.config.behavior, AiNodeBehavior::TextOnly) {
            return self.generate_conversational_prompt(input);
        }

        let base_prompt = format!(
            r#"You are an AI agent in a workflow system. Your goal: {}

## Previous Context:
{}

## Available Data:
{}

"#,
            input.user_message,
            input.context_memory.join("\n"),
            serde_json::to_string_pretty(&input.previous_outputs).unwrap_or_default()
        );

        let filtered_tools = self.filter_tools(input);

        match self.config.behavior {
            AiNodeBehavior::FreeAgent => {
                format!(
                    r#"{}
## Available Tools:
You have access to {} tools. Use any tools you think are appropriate to accomplish the goal.

## Multi-Step Task Execution:
- For tasks with timing requirements (e.g., "after X seconds", "wait X", "delay X"), use the Delay tool
- For sequential actions, call tools in the exact order they should execute
- Each tool will run sequentially in the order you specify them in your response
- Break down complex tasks into individual tool calls

## Tool Execution Rules:
1. **TIMING**: If the task mentions time delays, waiting, or "after X seconds", use the Delay tool
2. **SEQUENCE**: Call tools in the exact order they should be executed
3. **COMPLETENESS**: Include ALL necessary tool calls to fully complete the task
4. **PARAMETERS**: Use appropriate parameters for each tool call

## Examples:
- "set volume to 10 then after 5 seconds to 0" → SetVolume(10), Delay(5000ms), SetVolume(0)
- "play audio and wait 3 seconds" → PlayAudio(...), Delay(3000ms)
- "delay 2 seconds then set brightness" → Delay(2000ms), SetBrightness(...)

## IMPORTANT: For multi-step tasks, respond with JSON format:
```json
{{
  "actions": [
    {{"tool": "tool_name", "args": {{"param": "value"}}}},
    {{"tool": "tool_name_2", "args": {{"param": "value"}}}}
  ],
  "reasoning": "Why you chose these actions"
}}
```

If the task requires multiple steps, use the JSON format above. For single-step tasks, you can use either tool calls or JSON format.
"#,
                    base_prompt,
                    filtered_tools.len()
                )
            }
            AiNodeBehavior::Orchestrator => {
                let tools_desc = filtered_tools
                    .iter()
                    .map(|t| format!("- {}: {}", t.name, t.description))
                    .collect::<Vec<_>>()
                    .join("\n");

                let next_steps = input
                    .flow_context
                    .next_node_types
                    .iter()
                    .map(|t| match t.as_str() {
                        "tool" => "Execute connected tools",
                        "if" => "Make a branching decision",
                        "summary" => "Prepare summary data",
                        _ => "Continue to next step",
                    })
                    .collect::<Vec<_>>()
                    .join(", ");

                let response_format = match self.config.response_format {
                    ResponseFormat::Structured => {
                        r#"
Respond with JSON in this format:
{
  "actions": [
    {"tool": "tool_name", "args": {"param": "value"}, "target_node_id": "optional"}
  ],
  "outputs": {
    "variable_name": "value_for_next_nodes"
  },
  "reasoning": "Why you chose these actions",
  "flow_control": {
    "branch_condition": "value_for_if_node",
    "should_continue": true
  }
}
"#
                    }
                    _ => "\nProvide a natural language response with your reasoning.",
                };

                let tool_instructions = if filtered_tools.is_empty() {
                    "## ⚠️ NO TOOLS AVAILABLE
You have NO tools connected to this node. You CANNOT call any tools.
You can only provide outputs based on the available data and context.
Do NOT attempt to call any tools - they will fail."
                } else {
                    "## Instructions:
1. You can ONLY use the tools listed above - no other tools exist
2. Choose which connected tools to run based on the goal
3. You may call multiple connected tools if needed
4. Do NOT call tools that are not in the list above
5. **SEQUENTIAL EXECUTION**: Call tools in the exact order they should be executed
6. **COMPLETE TASK**: Include all necessary tool calls to fully complete the task"
                };

                let tool_names_list = if filtered_tools.is_empty() {
                    "No tools available".to_string()
                } else {
                    filtered_tools
                        .iter()
                        .map(|t| format!("\"{}\"", t.name))
                        .collect::<Vec<_>>()
                        .join(", ")
                };

                format!(
                    r#"{}
## Available Tools (connected to this node):
{}

## Next Steps in Flow:
{}

## Expected Outputs:
You should provide values for: {}

{}
7. Provide structured outputs for the next flow steps
8. If next step is an IF node, provide the branching condition value
{}

## Tool Execution Rules:
- Call the tools you need in the order they should execute
- Each tool will run sequentially in the order you specify
- All tool calls will be executed automatically
- Focus on the task requirements, not on detecting task complexity

## IMPORTANT - Tool Names in JSON:
When using JSON format, use these EXACT tool names: {}
"#,
                    base_prompt,
                    tools_desc,
                    next_steps,
                    input.flow_context.expected_outputs.join(", "),
                    tool_instructions,
                    response_format,
                    tool_names_list
                )
            }

            AiNodeBehavior::TextOnly => {
                // This case is now handled above
                unreachable!(
                    "TextOnly behavior should be handled by generate_conversational_prompt"
                )
            }
            AiNodeBehavior::Custom => {
                format!("{}\n## Custom Behavior:\nFollow the specific instructions provided in your configuration.", base_prompt)
            }
        }
    }

    /// Generate conversational prompt for TextOnly behavior
    fn generate_conversational_prompt(&self, input: &AiNodeInput) -> String {
        let context_info = if !input.context_memory.is_empty() {
            format!(
                "\n## Previous Context:\n{}",
                input.context_memory.join("\n")
            )
        } else {
            String::new()
        };

        let data_info = if !input.previous_outputs.is_empty() {
            format!(
                "\n## Available Data:\n{}",
                serde_json::to_string_pretty(&input.previous_outputs).unwrap_or_default()
            )
        } else {
            String::new()
        };

        // Determine input mode (default to TaskExecution for modern template-first approach)
        let input_mode = self
            .config
            .input_mode
            .as_ref()
            .unwrap_or(&InputMode::TaskExecution);

        match input_mode {
            InputMode::TaskExecution => {
                // Always treat as a task to execute
                format!(
                    r#"You are a creative AI assistant. Your task is to follow the instructions or prompt given to you.{}{} 

Your task/prompt: "{}"

Execute this task directly. Be creative and comprehensive in your response."#,
                    context_info, data_info, input.user_message
                )
            }
            InputMode::Conversational => {
                // Always treat as conversation
                format!(
                    r#"You are having a casual conversation.{}{} 

Someone just said: "{}"

Respond naturally as if you're chatting with a friend. Just give a normal, conversational response."#,
                    context_info, data_info, input.user_message
                )
            }

            InputMode::CustomTemplate => {
                // Use custom template with placeholder replacement
                if let Some(template) = &self.config.prompt_template {
                    self.apply_template(template, input, &context_info, &data_info)
                } else {
                    // Fallback to direct execution if no template provided
                    format!(r#"{}"#, input.user_message)
                }
            }
        }
    }

    /// Apply a custom template with placeholder replacement
    fn apply_template(
        &self,
        template: &PromptTemplate,
        input: &AiNodeInput,
        context_info: &str,
        data_info: &str,
    ) -> String {
        let mut result = template.template.clone();

        // Replace placeholders
        result = result.replace("{input}", &input.user_message);

        if template.include_context {
            result = result.replace("{context}", &input.context_memory.join("\n"));
            result = result.replace(
                "{previous_outputs}",
                &serde_json::to_string_pretty(&input.previous_outputs).unwrap_or_default(),
            );
        } else {
            // Remove unused placeholders
            result = result.replace("{context}", "");
            result = result.replace("{previous_outputs}", "");
        }

        // Clean up extra whitespace
        result = result.trim().to_string();

        result
    }

    /// Execute the unified AI node
    pub async fn execute(&self, input: AiNodeInput) -> Result<AiNodeOutput> {
        let start_time = std::time::Instant::now();
        let prompt = self.generate_system_prompt(&input);
        let filtered_tools = self.filter_tools(&input);

        println!(
            "🧠 UnifiedAiNode | Behavior: {:?} | Response Format: {:?}",
            self.config.behavior, self.config.response_format
        );
        println!("🔧 Filtered tools count: {}", filtered_tools.len());
        println!(
            "📝 Generated prompt (first 500 chars): {}",
            if prompt.len() > 500 {
                &prompt[..500]
            } else {
                &prompt
            }
        );

        let result = match self.config.response_format {
            ResponseFormat::ToolCallsOnly | ResponseFormat::Structured => {
                println!(
                    "🚀 Using structured generation with {} tools",
                    filtered_tools.len()
                );
                // Use structured generation with tools
                self.llm_client
                    .generate_with_tools(prompt, filtered_tools, self.config.model.as_deref())
                    .await
            }
            ResponseFormat::Natural => {
                println!("🚀 Using natural language generation (no tools)");
                // Use simple text generation
                let response = self
                    .llm_client
                    .generate_response(prompt, self.config.model.as_deref())
                    .await?;
                Ok(AiNodeOutput {
                    actions: Vec::new(),
                    outputs: {
                        let mut outputs = HashMap::new();
                        outputs.insert("response".to_string(), Value::String(response.clone()));
                        outputs
                    },
                    reasoning: Some(response),
                    flow_control: Some(FlowControl {
                        branch_condition: None,
                        should_continue: true,
                        next_node_override: None,
                    }),
                    metadata: ExecutionMetadata {
                        execution_time_ms: start_time.elapsed().as_millis() as u64,
                        tokens_used: None,
                        tools_called: 0,
                        config_used: self.config.clone(),
                    },
                })
            }
            ResponseFormat::Custom(ref format) => {
                println!("🚀 Using custom format: {}", format);
                // Use custom format
                let custom_prompt = format!(
                    "{}\n\nPlease respond in the following format: {}",
                    prompt, format
                );
                let response = self
                    .llm_client
                    .generate_response(custom_prompt, self.config.model.as_deref())
                    .await?;
                self.parse_custom_response(response, start_time)
            }
        };

        match &result {
            Ok(output) => {
                println!(
                    "✅ UnifiedAiNode execution completed | Actions: {} | Outputs: {}",
                    output.actions.len(),
                    output.outputs.len()
                );
                if output.actions.is_empty() {
                    println!("⚠️ No actions generated - check if tools were properly provided");
                }
            }
            Err(e) => {
                println!("❌ UnifiedAiNode execution failed: {}", e);
            }
        }

        result
    }

    fn parse_custom_response(
        &self,
        response: String,
        start_time: std::time::Instant,
    ) -> Result<AiNodeOutput> {
        // Simple response parsing - outputs response as-is
        Ok(AiNodeOutput {
            actions: Vec::new(),
            outputs: {
                let mut outputs = HashMap::new();
                outputs.insert("response".to_string(), Value::String(response.clone()));
                outputs
            },
            reasoning: Some(response),
            flow_control: Some(FlowControl {
                branch_condition: None,
                should_continue: true,
                next_node_override: None,
            }),
            metadata: ExecutionMetadata {
                execution_time_ms: start_time.elapsed().as_millis() as u64,
                tokens_used: None,
                tools_called: 0,
                config_used: self.config.clone(),
            },
        })
    }
}

/// Factory for creating AI nodes with different configurations
pub struct AiNodeFactory;

impl AiNodeFactory {
    /// Create a Text-Only AI node with a specific template
    pub fn create_with_template(
        template: PromptTemplate,
        llm_client: Arc<dyn LlmClient>,
    ) -> UnifiedAiNode {
        UnifiedAiNode::new(
            AiNodeConfig {
                behavior: AiNodeBehavior::TextOnly,
                tool_access: ToolAccessPolicy::NoTools,
                response_format: ResponseFormat::Natural,
                custom_prompt: None,
                model: None,
                input_mode: Some(InputMode::CustomTemplate),
                prompt_template: Some(template),
            },
            llm_client,
        )
    }

    /// Create a custom AI node
    pub fn create_custom(
        behavior: AiNodeBehavior,
        tool_access: ToolAccessPolicy,
        response_format: ResponseFormat,
        custom_prompt: Option<String>,
        model: Option<String>,
        llm_client: Arc<dyn LlmClient>,
    ) -> UnifiedAiNode {
        UnifiedAiNode::new(
            AiNodeConfig {
                behavior,
                tool_access,
                response_format,
                custom_prompt,
                model,
                input_mode: None,
                prompt_template: None,
            },
            llm_client,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ai_node_config_serialization() {
        let config = AiNodeConfig {
            behavior: AiNodeBehavior::Orchestrator,
            tool_access: ToolAccessPolicy::ConnectedOnly,
            response_format: ResponseFormat::Structured,
            custom_prompt: Some("Custom prompt".to_string()),
            model: None,
            input_mode: None,
            prompt_template: None,
        };

        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: AiNodeConfig = serde_json::from_str(&serialized).unwrap();
        assert_eq!(config, deserialized);
    }
}
