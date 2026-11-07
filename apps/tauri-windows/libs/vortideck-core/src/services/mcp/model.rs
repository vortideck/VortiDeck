use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: Option<String>,
    pub name: Option<String>,
    #[serde(rename = "function_call")]
    pub function_call: Option<FunctionCall>,
    #[serde(rename = "tool_calls")]
    pub tool_calls: Option<Vec<ToolCallInfo>>,
    #[serde(rename = "tool_call_id")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub annotations: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallInfo {
    pub id: String,
    pub function: FunctionCall,
    #[serde(rename = "type")]
    pub call_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
    pub ai_command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamOptions {
    pub include_usage: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_options: Option<StreamOptions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<Choice>,
    #[serde(default)]
    pub usage: Usage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub index: i32,
    pub message: Message,
    pub finish_reason: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
    #[serde(default)]
    pub evidence: UsageEvidence,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UsageEvidence {
    ProviderMeasured,
    Partial,
    #[default]
    Unknown,
}

impl Usage {
    pub fn merge(&mut self, next: &Self) {
        self.prompt_tokens += next.prompt_tokens;
        self.completion_tokens += next.completion_tokens;
        self.total_tokens += next.total_tokens;
        self.evidence = match (self.evidence, next.evidence) {
            (UsageEvidence::Unknown, UsageEvidence::Unknown) => UsageEvidence::Unknown,
            (UsageEvidence::ProviderMeasured, UsageEvidence::ProviderMeasured) => {
                UsageEvidence::ProviderMeasured
            }
            _ => UsageEvidence::Partial,
        };
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    pub content: String,
    pub is_final: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: Function,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

impl Message {
    pub fn system(content: String) -> Self {
        Self {
            role: "system".to_string(),
            content: Some(content),
            name: None,
            function_call: None,
            tool_calls: None,
            tool_call_id: None,
            refusal: None,
            annotations: Vec::new(),
        }
    }

    pub fn user(content: String) -> Self {
        Self {
            role: "user".to_string(),
            content: Some(content),
            name: None,
            function_call: None,
            tool_calls: None,
            tool_call_id: None,
            refusal: None,
            annotations: Vec::new(),
        }
    }

    pub fn tool(tool_call_id: &str, result: &str) -> Self {
        Self {
            role: "tool".to_string(),
            content: Some(result.to_string()),
            name: None,
            function_call: None,
            tool_calls: None,
            tool_call_id: Some(tool_call_id.to_string()),
            refusal: None,
            annotations: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub contents: Vec<Content>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Content {
    pub content_type: String,
    pub body: String,
}

impl Content {
    pub fn text(content: impl ToString) -> Self {
        Self {
            content_type: "text/plain".to_string(),
            body: content.to_string(),
        }
    }
}

#[cfg(test)]
mod usage_tests {
    use super::*;

    #[test]
    fn absent_provider_usage_is_explicitly_unknown() {
        let response: CompletionResponse = serde_json::from_value(serde_json::json!({
            "id": "response-1",
            "object": "chat.completion",
            "created": 0,
            "model": "fixture",
            "choices": []
        }))
        .expect("response without usage remains readable");

        assert_eq!(response.usage.total_tokens, 0);
        assert_eq!(response.usage.evidence, UsageEvidence::Unknown);
    }

    #[test]
    fn mixed_measured_and_unknown_turns_are_labelled_partial() {
        let mut aggregate = Usage {
            prompt_tokens: 10,
            completion_tokens: 5,
            total_tokens: 15,
            evidence: UsageEvidence::ProviderMeasured,
        };
        aggregate.merge(&Usage::default());

        assert_eq!(aggregate.total_tokens, 15);
        assert_eq!(aggregate.evidence, UsageEvidence::Partial);
    }
}
