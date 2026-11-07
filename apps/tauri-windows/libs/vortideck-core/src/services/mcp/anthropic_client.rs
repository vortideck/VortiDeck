//! Anthropic (Claude) chat client implementation

use crate::services::mcp::client::ChatClient;
use crate::services::mcp::model::{
    Choice, CompletionRequest, CompletionResponse, Function, Message, Tool, Usage, UsageEvidence,
};
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct AnthropicClient {
    api_key: String,
    base_url: String,
    client: Arc<HttpClient>,
}

impl AnthropicClient {
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        Self {
            api_key,
            base_url: base_url.unwrap_or_else(|| "https://api.anthropic.com".to_string()),
            client: Arc::new(HttpClient::new()),
        }
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// Convert universal messages to Anthropic format
    /// Anthropic separates system messages from the messages array
    fn convert_messages(&self, messages: Vec<Message>) -> (Option<String>, Vec<Value>) {
        let mut system_prompt = None;
        let mut anthropic_messages = Vec::new();

        for msg in messages {
            match msg.role.as_str() {
                "system" => {
                    // Anthropic puts system in a separate field
                    system_prompt = msg.content;
                }
                "user" | "assistant" => {
                    anthropic_messages.push(json!({
                        "role": msg.role,
                        "content": msg.content.unwrap_or_default(),
                    }));
                }
                "tool" => {
                    // Anthropic uses tool_result instead of tool role
                    anthropic_messages.push(json!({
                        "role": "user",
                        "content": [{
                            "type": "tool_result",
                            "tool_use_id": msg.tool_call_id.unwrap_or_default(),
                            "content": msg.content.unwrap_or_default(),
                        }]
                    }));
                }
                _ => {
                    // Default to user message
                    anthropic_messages.push(json!({
                        "role": "user",
                        "content": msg.content.unwrap_or_default(),
                    }));
                }
            }
        }

        (system_prompt, anthropic_messages)
    }
}

// Anthropic-specific response format
#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    id: String,
    content: Vec<AnthropicContent>,
    stop_reason: Option<String>,
    usage: AnthropicUsage,
    model: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum AnthropicContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: i32,
    output_tokens: i32,
}

#[async_trait]
impl ChatClient for AnthropicClient {
    fn provider_name(&self) -> &str {
        "Anthropic"
    }

    fn format_tools(&self, tools: &[Arc<dyn crate::services::mcp::tool::Tool>]) -> Vec<Tool> {
        // Anthropic format is different - they use "input_schema" instead of "parameters"
        // But we can still use the same Tool struct and convert during request
        tools
            .iter()
            .map(|tool| {
                Tool {
                    tool_type: "function".to_string(), // We'll convert this when making request
                    function: Function {
                        name: tool.name(),
                        description: tool.description(),
                        parameters: tool.parameters(),
                    },
                }
            })
            .collect()
    }

    fn validate_model(&self, model: &str) -> bool {
        // Allow common Claude models
        matches!(
            model,
            "claude-3-5-sonnet-20241022" | "claude-3-5-sonnet-20240620" |
            "claude-3-opus-20240229" | "claude-3-sonnet-20240229" | "claude-3-haiku-20240307" |
            _ if model.starts_with("claude-")  // Allow any claude-* model
        )
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        println!(
            "🤖 Anthropic: Sending completion request to {}",
            self.base_url
        );

        let max_tokens = request.max_tokens.unwrap_or(4096);
        // Convert messages to Anthropic format
        let (system_prompt, anthropic_messages) = self.convert_messages(request.messages);

        // Build Anthropic-specific request body
        let mut body = json!({
            "model": request.model,
            "messages": anthropic_messages,
            "max_tokens": max_tokens,
        });

        if let Some(ref system) = system_prompt {
            body["system"] = json!(system);
        }

        if let Some(temp) = request.temperature {
            body["temperature"] = json!(temp);
        }

        // Convert tools to Anthropic format
        if let Some(tools) = request.tools {
            if !tools.is_empty() {
                // Anthropic expects: { name, description, input_schema }
                let anthropic_tools: Vec<Value> = tools
                    .iter()
                    .map(|tool| {
                        json!({
                            "name": tool.function.name,
                            "description": tool.function.description,
                            "input_schema": tool.function.parameters,
                        })
                    })
                    .collect();

                body["tools"] = json!(anthropic_tools);
                println!("🔧 Anthropic: Sending {} tools", anthropic_tools.len());
            }
        }

        // Make API request
        let response = self
            .client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;

            // Graceful degradation: retry without unsupported parameter
            if crate::services::mcp::client::is_unsupported_param_error(&error_text) {
                println!("⚠️  Retrying without temperature parameter");

                // Rebuild request without temperature (no temp field in retry)
                let mut retry_body = json!({
                    "model": request.model,
                    "messages": anthropic_messages,
                    "max_tokens": max_tokens,
                });

                if let Some(system) = system_prompt {
                    retry_body["system"] = json!(system);
                }

                let retry_response = self
                    .client
                    .post(format!("{}/v1/messages", self.base_url))
                    .header("x-api-key", &self.api_key)
                    .header("anthropic-version", "2023-06-01")
                    .header("Content-Type", "application/json")
                    .json(&retry_body)
                    .send()
                    .await?;

                if !retry_response.status().is_success() {
                    let error = retry_response.text().await?;
                    return Err(anyhow::anyhow!("Anthropic retry error: {}", error));
                }

                let anthropic_response: AnthropicResponse = retry_response.json().await?;
                return self.convert_anthropic_response(anthropic_response, request.model);
            }

            return Err(anyhow::anyhow!(
                "Anthropic API error ({}): {}",
                status,
                error_text
            ));
        }

        // Parse response
        let anthropic_response: AnthropicResponse = response.json().await?;
        self.convert_anthropic_response(anthropic_response, request.model)
    }

    async fn complete_streaming(
        &self,
        request: CompletionRequest,
    ) -> Result<(
        tokio::sync::mpsc::Receiver<String>,
        tokio::task::JoinHandle<Result<CompletionResponse>>,
    )> {
        println!(
            "🤖 Anthropic: Sending streaming completion request to {}",
            self.base_url
        );

        let max_tokens = request.max_tokens.unwrap_or(4096);
        // Convert messages to Anthropic format
        let (system_prompt, anthropic_messages) = self.convert_messages(request.messages.clone());

        // Build Anthropic-specific request body
        let mut body = json!({
            "model": request.model.clone(),
            "messages": anthropic_messages,
            "max_tokens": max_tokens,
            "stream": true,  // Enable streaming
        });

        if let Some(system) = system_prompt {
            body["system"] = json!(system);
        }

        if let Some(temp) = request.temperature {
            body["temperature"] = json!(temp);
        }

        // Convert tools to Anthropic format
        if let Some(tools) = request.tools.clone() {
            if !tools.is_empty() {
                let anthropic_tools: Vec<Value> = tools
                    .iter()
                    .map(|tool| {
                        json!({
                            "name": tool.function.name,
                            "description": tool.function.description,
                            "input_schema": tool.function.parameters,
                        })
                    })
                    .collect();
                body["tools"] = json!(anthropic_tools);
            }
        }

        // Make streaming API request
        let response = self
            .client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!(
                "Anthropic API error ({}): {}",
                status,
                error_text
            ));
        }

        // Parse SSE streaming response
        self.parse_anthropic_streaming_response(response, request.model)
            .await
    }
}

// Helper methods for AnthropicClient
impl AnthropicClient {
    /// Convert Anthropic response to universal CompletionResponse format
    fn convert_anthropic_response(
        &self,
        anthropic_response: AnthropicResponse,
        model: String,
    ) -> Result<CompletionResponse> {
        let mut message_content = String::new();
        let mut tool_calls_vec = Vec::new();

        for content in anthropic_response.content {
            match content {
                AnthropicContent::Text { text } => {
                    if !message_content.is_empty() {
                        message_content.push('\n');
                    }
                    message_content.push_str(&text);
                }
                AnthropicContent::ToolUse { id, name, input } => {
                    use crate::services::mcp::model::{FunctionCall, ToolCallInfo};
                    tool_calls_vec.push(ToolCallInfo {
                        id,
                        function: FunctionCall {
                            name,
                            arguments: serde_json::to_string(&input)?,
                        },
                        call_type: "function".to_string(),
                    });
                }
            }
        }

        Ok(CompletionResponse {
            id: anthropic_response.id,
            object: "chat.completion".to_string(),
            created: chrono::Utc::now().timestamp(),
            model,
            choices: vec![Choice {
                index: 0,
                message: Message {
                    role: "assistant".to_string(),
                    content: if message_content.is_empty() {
                        None
                    } else {
                        Some(message_content)
                    },
                    name: None,
                    function_call: None,
                    tool_calls: if tool_calls_vec.is_empty() {
                        None
                    } else {
                        Some(tool_calls_vec)
                    },
                    tool_call_id: None,
                    refusal: None,
                    annotations: Vec::new(),
                },
                finish_reason: anthropic_response
                    .stop_reason
                    .unwrap_or_else(|| "stop".to_string()),
            }],
            usage: Usage {
                prompt_tokens: anthropic_response.usage.input_tokens,
                completion_tokens: anthropic_response.usage.output_tokens,
                total_tokens: anthropic_response.usage.input_tokens
                    + anthropic_response.usage.output_tokens,
                evidence: UsageEvidence::ProviderMeasured,
            },
        })
    }

    /// Parse Anthropic SSE streaming response
    async fn parse_anthropic_streaming_response(
        &self,
        response: reqwest::Response,
        model: String,
    ) -> Result<(
        tokio::sync::mpsc::Receiver<String>,
        tokio::task::JoinHandle<Result<CompletionResponse>>,
    )> {
        use futures_util::StreamExt;

        let (tx, rx) = tokio::sync::mpsc::channel::<String>(100);

        let handle = tokio::spawn(async move {
            let mut stream = response.bytes_stream();
            let mut buffer = String::new();
            let mut accumulated_content = String::new();
            let mut message_id = String::new();
            let mut usage = Usage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
                evidence: UsageEvidence::Unknown,
            };

            while let Some(chunk_result) = stream.next().await {
                let chunk = chunk_result?;
                let text = String::from_utf8_lossy(&chunk);
                buffer.push_str(&text);

                // Process complete lines
                while let Some(line_end) = buffer.find('\n') {
                    let line = buffer[..line_end].trim().to_string();
                    buffer = buffer[line_end + 1..].to_string();

                    if line.is_empty() || line.starts_with(':') {
                        continue;
                    }

                    // Anthropic SSE format: "data: {...}"
                    if let Some(data_str) = line.strip_prefix("data: ") {
                        if data_str == "[DONE]" {
                            break;
                        }

                        if let Ok(event) = serde_json::from_str::<Value>(data_str) {
                            let event_type = event["type"].as_str().unwrap_or("");

                            match event_type {
                                "message_start" => {
                                    if let Some(id) = event["message"]["id"].as_str() {
                                        message_id = id.to_string();
                                    }
                                    // Extract initial usage from message_start
                                    if let Some(usage_data) = event["message"]["usage"].as_object()
                                    {
                                        usage.evidence = UsageEvidence::ProviderMeasured;
                                        if let Some(input) =
                                            usage_data.get("input_tokens").and_then(|v| v.as_i64())
                                        {
                                            usage.prompt_tokens = input as i32;
                                        }
                                        if let Some(output) =
                                            usage_data.get("output_tokens").and_then(|v| v.as_i64())
                                        {
                                            usage.completion_tokens = output as i32;
                                        }
                                        usage.total_tokens =
                                            usage.prompt_tokens + usage.completion_tokens;
                                    }
                                }
                                "content_block_delta" => {
                                    if let Some(text) = event["delta"]["text"].as_str() {
                                        accumulated_content.push_str(text);
                                        let _ = tx.send(text.to_string()).await;
                                    }
                                }
                                "message_delta" => {
                                    // Update usage as it changes during streaming
                                    if let Some(usage_data) = event["usage"].as_object() {
                                        if usage.evidence == UsageEvidence::Unknown {
                                            usage.evidence = UsageEvidence::Partial;
                                        }
                                        if let Some(output) =
                                            usage_data.get("output_tokens").and_then(|v| v.as_i64())
                                        {
                                            usage.completion_tokens = output as i32;
                                        }
                                        usage.total_tokens =
                                            usage.prompt_tokens + usage.completion_tokens;
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }

            // Build final response
            println!(
                "✅ Anthropic streaming complete: {} chars",
                accumulated_content.len()
            );
            println!(
                "📊 Anthropic usage - Prompt: {}, Completion: {}, Total: {}",
                usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
            );

            Ok(CompletionResponse {
                id: message_id,
                object: "chat.completion".to_string(),
                created: chrono::Utc::now().timestamp(),
                model,
                choices: vec![Choice {
                    index: 0,
                    message: Message {
                        role: "assistant".to_string(),
                        content: Some(accumulated_content),
                        name: None,
                        function_call: None,
                        tool_calls: None,
                        tool_call_id: None,
                        refusal: None,
                        annotations: Vec::new(),
                    },
                    finish_reason: "stop".to_string(),
                }],
                usage,
            })
        });

        Ok((rx, handle))
    }
}
