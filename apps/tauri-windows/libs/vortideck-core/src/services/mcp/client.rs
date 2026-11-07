use crate::services::mcp::model::{
    Choice, CompletionRequest, CompletionResponse, Message, ToolCall, ToolCallInfo, ToolResult,
    Usage, UsageEvidence,
};
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client as HttpClient;
use serde_json::json;
use std::sync::Arc;

/// Check if API error indicates unsupported parameter
#[inline]
pub fn is_unsupported_param_error(error_text: &str) -> bool {
    error_text.contains("temperature")
        || error_text.contains("Unsupported")
        || error_text.contains("not supported")
        || error_text.contains("doesn't support")
}

/// Remove temperature from request (used for retry logic)
#[inline]
pub fn remove_temperature(request: &mut CompletionRequest) {
    request.temperature = None;
}

/// Core trait for AI chat providers
///
/// Supports multiple AI providers (OpenAI, Anthropic, Azure, Custom) through a unified interface.
#[async_trait]
pub trait ChatClient: Send + Sync {
    /// Provider name (for logging/debugging)
    fn provider_name(&self) -> &str;

    /// Make a completion request
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse>;

    /// Make a streaming completion request - returns channel for real-time chunks
    /// Returns (receiver for chunks, task handle for final response)
    async fn complete_streaming(
        &self,
        request: CompletionRequest,
    ) -> Result<(
        tokio::sync::mpsc::Receiver<String>,
        tokio::task::JoinHandle<Result<CompletionResponse>>,
    )>;

    /// Format tools for this provider's specific schema
    ///
    /// Different providers expect different tool formats:
    /// - OpenAI: `{ type: "function", function: { name, description, parameters } }`
    /// - Anthropic: `{ name, description, input_schema }`
    fn format_tools(
        &self,
        tools: &[std::sync::Arc<dyn crate::services::mcp::tool::Tool>],
    ) -> Vec<crate::services::mcp::model::Tool>;

    /// Validate model name for this provider
    fn validate_model(&self, model: &str) -> bool {
        !model.is_empty() // Default implementation
    }
}

#[derive(Clone, Debug)]
pub struct OpenAIClient {
    api_key: String,
    url: String,
    client: Arc<HttpClient>,
}

impl OpenAIClient {
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        Self {
            api_key,
            url: base_url.unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
            client: Arc::new(HttpClient::new()),
        }
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.url = base_url.into();
        self
    }

    fn get_completion_url(&self) -> String {
        // Frontend already provides full URLs, use as-is
        self.url.clone()
    }

    pub async fn send_messages(&self, messages: Vec<Message>, model: &str) -> Result<Message> {
        let completion_url = self.get_completion_url();
        let response = self
            .client
            .post(&completion_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&json!({
                "model": model,
                "messages": messages,
                "temperature": 0.7,
                "stream": false
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("OpenAI API request failed: {}", error_text));
        }

        let response_data = response.json::<serde_json::Value>().await?;
        let choices = response_data["choices"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("No choices in response"))?;

        if choices.is_empty() {
            return Err(anyhow::anyhow!("Empty choices in response"));
        }

        let message = serde_json::from_value(choices[0]["message"].clone().into())?;

        Ok(message)
    }
}

#[async_trait]
impl ChatClient for OpenAIClient {
    fn provider_name(&self) -> &str {
        "OpenAI"
    }

    fn format_tools(
        &self,
        tools: &[std::sync::Arc<dyn crate::services::mcp::tool::Tool>],
    ) -> Vec<crate::services::mcp::model::Tool> {
        use crate::services::mcp::model::{Function, Tool};
        // OpenAI format: { type: "function", function: { name, description, parameters } }
        tools
            .iter()
            .map(|tool| {
                // OpenAI requires tool names to match pattern: ^[a-zA-Z0-9_-]+$
                // Replace dots with underscores to make tool names API-compliant
                let sanitized_name = tool.name().replace('.', "_");

                Tool {
                    tool_type: "function".to_string(),
                    function: Function {
                        name: sanitized_name,
                        description: tool.description(),
                        parameters: tool.parameters(),
                    },
                }
            })
            .collect()
    }

    fn validate_model(&self, model: &str) -> bool {
        // Allow common OpenAI models
        matches!(
            model,
            "gpt-4" | "gpt-4-turbo" | "gpt-4-turbo-preview" |
            "gpt-3.5-turbo" | "gpt-3.5-turbo-16k" |
            _ if model.starts_with("gpt-")  // Allow any gpt-* model
        )
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let completion_url = self.get_completion_url();
        println!("🤖 Sending chat completion request to: {}", completion_url);

        // Log each message in the request for debugging
        if let Some(last_message) = request.messages.last() {
            println!(
                "Last message in conversation - role: {}, content: {:?}",
                last_message.role, last_message.content
            );

            if last_message.role == "tool" {
                println!(
                    "Tool message with tool_call_id: {:?}",
                    last_message.tool_call_id
                );
            }

            if let Some(tool_calls) = &last_message.tool_calls {
                println!("Message contains {} tool calls", tool_calls.len());
                for call in tool_calls {
                    println!("Tool call - id: {}, name: {}", call.id, call.function.name);
                }
            }
        }

        let request_json = serde_json::to_string(&request)?;
        println!("request content: {}", request_json);

        let is_streaming = request.stream.unwrap_or(false);

        let response = self
            .client
            .post(&completion_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            println!("API error: {}", error_text);

            // Graceful degradation: retry without unsupported parameter
            if is_unsupported_param_error(&error_text) {
                println!("⚠️  Retrying without temperature parameter");

                let mut retry_request = request.clone();
                remove_temperature(&mut retry_request);

                let response = self
                    .client
                    .post(&completion_url)
                    .header("Authorization", format!("Bearer {}", self.api_key))
                    .header("Content-Type", "application/json")
                    .json(&retry_request)
                    .send()
                    .await?;

                if !response.status().is_success() {
                    let error = response.text().await?;
                    return Err(anyhow::anyhow!("API Error (retry): {}", error));
                }

                return self.parse_non_streaming_response(response).await;
            }

            return Err(anyhow::anyhow!("API Error: {}", error_text));
        }

        // Handle streaming vs non-streaming responses
        if is_streaming {
            println!("🌊 Parsing SSE streaming response...");
            // For streaming, accumulate all chunks and return final response
            let (mut rx, handle) = self.complete_streaming_impl(request).await?;

            // Consume all chunks (adapter layer can override this for real-time updates)
            while let Some(_chunk) = rx.recv().await {
                // Chunks are logged in parse_streaming_response
            }

            // Wait for final response
            handle
                .await
                .map_err(|e| anyhow::anyhow!("Join error: {}", e))?
        } else {
            self.parse_non_streaming_response(response).await
        }
    }

    async fn complete_streaming(
        &self,
        request: CompletionRequest,
    ) -> Result<(
        tokio::sync::mpsc::Receiver<String>,
        tokio::task::JoinHandle<Result<CompletionResponse>>,
    )> {
        self.complete_streaming_impl(request).await
    }
}

// OpenAIClient-specific helper methods
impl OpenAIClient {
    /// Parse non-streaming response
    async fn parse_non_streaming_response(
        &self,
        response: reqwest::Response,
    ) -> Result<CompletionResponse> {
        let response_text = response.text().await?;
        println!("Raw API response: {}", response_text);

        // Try to parse the response
        match serde_json::from_str::<CompletionResponse>(&response_text) {
            Ok(mut completion) => {
                if serde_json::from_str::<serde_json::Value>(&response_text)
                    .ok()
                    .and_then(|value| value.get("usage").cloned())
                    .is_some()
                {
                    completion.usage.evidence = UsageEvidence::ProviderMeasured;
                }
                Ok(completion)
            }
            Err(e) => {
                println!("Error parsing API response: {}", e);
                println!("Response was: {}", response_text);
                Err(anyhow::anyhow!("Failed to parse API response: {}", e))
            }
        }
    }

    /// Complete with streaming support - returns channel for real-time chunks
    async fn complete_streaming_impl(
        &self,
        request: CompletionRequest,
    ) -> Result<(
        tokio::sync::mpsc::Receiver<String>,
        tokio::task::JoinHandle<Result<CompletionResponse>>,
    )> {
        let completion_url = self.get_completion_url();
        println!("🤖 OpenAI: Sending streaming request to {}", completion_url);

        let response = self
            .client
            .post(&completion_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("API Error: {}", error_text));
        }

        self.parse_streaming_response(response).await
    }

    /// Parse SSE (Server-Sent Events) streaming response
    /// Returns immediately with a channel receiver for chunks and a future for the final response
    async fn parse_streaming_response(
        &self,
        response: reqwest::Response,
    ) -> Result<(
        tokio::sync::mpsc::Receiver<String>,
        tokio::task::JoinHandle<Result<CompletionResponse>>,
    )> {
        use futures_util::StreamExt;

        let (tx, rx) = tokio::sync::mpsc::channel::<String>(100);

        // Spawn async task to parse stream and send chunks
        let handle = tokio::spawn(async move {
            let mut stream = response.bytes_stream();
            let mut buffer = String::new();
            let mut accumulated_content = String::new();
            let mut response_id = String::new();
            let mut model = String::new();
            let mut created = 0i64;
            let mut tool_calls: Option<Vec<ToolCallInfo>> = None;
            let mut usage = Usage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
                evidence: UsageEvidence::Unknown,
            };

            while let Some(chunk_result) = stream.next().await {
                let chunk = chunk_result.map_err(|e| anyhow::anyhow!("Stream error: {}", e))?;
                let chunk_str = String::from_utf8_lossy(&chunk);
                buffer.push_str(&chunk_str);

                // Process complete lines
                while let Some(newline_pos) = buffer.find('\n') {
                    let line = buffer[..newline_pos].trim().to_string();
                    buffer = buffer[newline_pos + 1..].to_string();

                    // Skip empty lines
                    if line.is_empty() {
                        continue;
                    }

                    // Parse SSE data lines
                    if let Some(data) = line.strip_prefix("data: ") {
                        // Check for end marker
                        if data == "[DONE]" {
                            break;
                        }

                        // Parse JSON chunk
                        if let Ok(chunk_json) = serde_json::from_str::<serde_json::Value>(data) {
                            // Extract metadata from first chunk
                            if response_id.is_empty() {
                                response_id = chunk_json["id"].as_str().unwrap_or("").to_string();
                                model = chunk_json["model"].as_str().unwrap_or("").to_string();
                                created = chunk_json["created"].as_i64().unwrap_or(0);
                            }

                            // Extract usage data (OpenAI sends this in streaming chunks)
                            if let Some(usage_json) = chunk_json.get("usage") {
                                usage.evidence = UsageEvidence::ProviderMeasured;
                                println!("📊 Found usage in chunk: {:?}", usage_json);
                                if let Some(prompt_tokens) =
                                    usage_json.get("prompt_tokens").and_then(|v| v.as_i64())
                                {
                                    usage.prompt_tokens = prompt_tokens as i32;
                                }
                                if let Some(completion_tokens) =
                                    usage_json.get("completion_tokens").and_then(|v| v.as_i64())
                                {
                                    usage.completion_tokens = completion_tokens as i32;
                                }
                                if let Some(total_tokens) =
                                    usage_json.get("total_tokens").and_then(|v| v.as_i64())
                                {
                                    usage.total_tokens = total_tokens as i32;
                                }
                            }

                            // Extract content delta
                            if let Some(choices) = chunk_json["choices"].as_array() {
                                if let Some(first_choice) = choices.first() {
                                    if let Some(delta) = first_choice["delta"].as_object() {
                                        // Accumulate content and emit chunk
                                        if let Some(content) =
                                            delta.get("content").and_then(|c| c.as_str())
                                        {
                                            accumulated_content.push_str(content);

                                            // Send chunk to channel (non-blocking)
                                            let _ = tx.send(content.to_string()).await;
                                        }

                                        // Handle tool calls (if any)
                                        if let Some(tool_calls_json) = delta.get("tool_calls") {
                                            // Parse tool calls from streaming delta
                                            if let Ok(calls) =
                                                serde_json::from_value::<Vec<ToolCallInfo>>(
                                                    tool_calls_json.clone(),
                                                )
                                            {
                                                tool_calls = Some(calls);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            println!(
                "✅ Streaming complete. Accumulated content: {} chars",
                accumulated_content.len()
            );
            println!(
                "📊 Final usage - Prompt: {}, Completion: {}, Total: {}",
                usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
            );

            // Build CompletionResponse from accumulated data
            let message = Message {
                role: "assistant".to_string(),
                content: if accumulated_content.is_empty() {
                    None
                } else {
                    Some(accumulated_content)
                },
                name: None,
                function_call: None,
                tool_calls,
                tool_call_id: None,
                refusal: None,
                annotations: vec![],
            };

            Ok(CompletionResponse {
                id: response_id,
                object: "chat.completion".to_string(),
                created,
                model,
                choices: vec![Choice {
                    index: 0,
                    message,
                    finish_reason: "stop".to_string(),
                }],
                usage,
            })
        });

        Ok((rx, handle))
    }
}

// ============================================================================
// Ollama Client - Native Ollama API support
// ============================================================================

/// Ollama client for local LLM inference using Ollama's native API
#[derive(Clone, Debug)]
pub struct OllamaClient {
    base_url: String,
    client: Arc<HttpClient>,
}

impl OllamaClient {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client: Arc::new(HttpClient::new()),
        }
    }

    fn get_completion_url(&self) -> String {
        // Use URL exactly as provided from config - no modifications
        self.base_url.clone()
    }
}

#[async_trait]
impl ChatClient for OllamaClient {
    fn provider_name(&self) -> &str {
        "Ollama"
    }

    fn format_tools(
        &self,
        tools: &[Arc<dyn crate::services::mcp::tool::Tool>],
    ) -> Vec<crate::services::mcp::model::Tool> {
        use crate::services::mcp::model::{Function, Tool};
        tools
            .iter()
            .map(|tool| {
                let sanitized_name = tool.name().replace('.', "_");
                Tool {
                    tool_type: "function".to_string(),
                    function: Function {
                        name: sanitized_name,
                        description: tool.description(),
                        parameters: tool.parameters(),
                    },
                }
            })
            .collect()
    }

    fn validate_model(&self, _model: &str) -> bool {
        // Ollama accepts any model that's pulled locally
        true
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let completion_url = self.get_completion_url();
        println!("🦙 Ollama: Sending request to {}", completion_url);

        // Convert to Ollama native format
        let ollama_request = json!({
            "model": request.model,
            "messages": request.messages,
            "stream": false,
            "options": {
                "num_predict": request.max_tokens.unwrap_or(2048)
            }
        });

        let response = self
            .client
            .post(&completion_url)
            .header("Content-Type", "application/json")
            .json(&ollama_request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Ollama API Error: {}", error_text));
        }

        let response_text = response.text().await?;
        let json: serde_json::Value = serde_json::from_str(&response_text)?;

        // Parse Ollama response: {"model": "...", "message": {"content": "..."}, "done": true}
        let content = json
            .get("message")
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string();

        let model = json
            .get("model")
            .and_then(|m| m.as_str())
            .unwrap_or("unknown")
            .to_string();

        // Convert to CompletionResponse format
        let prompt_tokens = json
            .get("prompt_eval_count")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
        let completion_tokens = json.get("eval_count").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let usage_evidence = match (
            json.get("prompt_eval_count").and_then(|v| v.as_i64()),
            json.get("eval_count").and_then(|v| v.as_i64()),
        ) {
            (Some(_), Some(_)) => UsageEvidence::ProviderMeasured,
            (None, None) => UsageEvidence::Unknown,
            _ => UsageEvidence::Partial,
        };

        Ok(CompletionResponse {
            id: "ollama-response".to_string(),
            object: "chat.completion".to_string(),
            created: chrono::Utc::now().timestamp(),
            model,
            choices: vec![Choice {
                index: 0,
                message: Message {
                    role: "assistant".to_string(),
                    content: Some(content),
                    name: None,
                    function_call: None,
                    tool_calls: None,
                    tool_call_id: None,
                    refusal: None,
                    annotations: vec![],
                },
                finish_reason: "stop".to_string(),
            }],
            usage: Usage {
                prompt_tokens,
                completion_tokens,
                total_tokens: prompt_tokens + completion_tokens,
                evidence: usage_evidence,
            },
        })
    }

    async fn complete_streaming(
        &self,
        _request: CompletionRequest,
    ) -> Result<(
        tokio::sync::mpsc::Receiver<String>,
        tokio::task::JoinHandle<Result<CompletionResponse>>,
    )> {
        Err(anyhow::anyhow!("Ollama streaming not yet implemented"))
    }
}
