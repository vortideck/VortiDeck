use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs::{create_dir_all, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Stored message format (optimized for JSONL)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StoredMessage {
    pub id: String,
    pub role: String,
    pub content: Option<String>,
    pub timestamp: String,
    pub tokens: Option<usize>,
    #[serde(default)]
    pub token_source: TokenSource,
    pub model: Option<String>,
    pub tool_calls: Option<serde_json::Value>,
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TokenSource {
    ProviderMeasured,
    Estimated,
    #[default]
    Unknown,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct ConversationTaskState {
    #[serde(default)]
    pub unresolved_calls: Vec<PendingToolCall>,
    #[serde(default)]
    pub completed_effects: Vec<CompletedEffectReceipt>,
    #[serde(default)]
    pub flow_revisions: BTreeMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct PendingToolCall {
    pub call_id: String,
    pub tool_name: String,
    pub arguments: Value,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct CompletedEffectReceipt {
    pub call_id: String,
    pub tool_name: String,
    pub invocation_id: String,
    pub status: String,
}

/// MCP tools mode for conversation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ToolsMode {
    /// Use all available tools
    All,
    /// Use only selected tools
    Selected,
    /// Disable all tools
    None,
}

impl Default for ToolsMode {
    fn default() -> Self {
        Self::All
    }
}

impl ToolsMode {
    /// Convert to string representation for legacy compatibility
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Selected => "selected",
            Self::None => "none",
        }
    }
}

/// Conversation metadata (stored as first line in JSONL)
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ConversationMetadata {
    /// Unique conversation ID
    pub id: String,
    /// Conversation title
    pub title: String,
    /// ISO 8601 timestamp when conversation was created
    pub created_at: String,
    /// ISO 8601 timestamp when conversation was last updated
    pub updated_at: String,
    /// AI model name (e.g., "gpt-4", "claude-3-opus")
    pub model: String,
    /// Provider name (e.g., "openai", "anthropic")
    pub provider: String,
    /// Number of messages in conversation
    pub message_count: usize,
    /// Total tokens used across all messages
    pub total_tokens: usize,
    /// Whether streaming is enabled for this conversation
    #[serde(default)]
    pub stream: bool,
    /// Temperature parameter for model sampling (0.0-2.0, typically 0.7)
    /// Lower values make output more focused, higher values more creative
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    /// Maximum tokens to generate in response
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    /// Tools mode for this conversation
    #[serde(default)]
    pub tools_mode: ToolsMode,
    /// Selected tool names (only used when tools_mode is Selected)
    #[serde(default)]
    pub selected_tools: Vec<String>,
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(default = "default_tool_timeout")]
    pub tool_timeout: u64,
    /// Machine-readable state that must survive lossy context compaction.
    #[serde(default)]
    pub task_state: ConversationTaskState,
}

impl Default for ConversationMetadata {
    fn default() -> Self {
        Self {
            id: String::new(),
            title: String::new(),
            created_at: String::new(),
            updated_at: String::new(),
            model: String::new(),
            provider: String::new(),
            message_count: 0,
            total_tokens: 0,
            stream: false,
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
            tools_mode: ToolsMode::default(),
            selected_tools: Vec::new(),
            system_prompt: None,
            tool_timeout: default_tool_timeout(),
            task_state: ConversationTaskState::default(),
        }
    }
}

impl ConversationMetadata {
    /// Validate metadata values are within acceptable ranges
    pub fn validate(&self) -> Result<(), String> {
        if self.temperature < 0.0 || self.temperature > 2.0 {
            return Err(format!(
                "Temperature {} out of range [0.0, 2.0]",
                self.temperature
            ));
        }
        if self.max_tokens == 0 {
            return Err("max_tokens must be greater than 0".to_string());
        }
        if self.model.is_empty() {
            return Err("model cannot be empty".to_string());
        }
        if self.provider.is_empty() {
            return Err("provider cannot be empty".to_string());
        }
        Ok(())
    }
}

fn default_temperature() -> f32 {
    0.7
}

fn default_max_tokens() -> u32 {
    4000
}

fn default_tool_timeout() -> u64 {
    30
}

/// Manages a single conversation file
pub struct ConversationFile {
    pub id: String,
    pub file_path: PathBuf,
    metadata: ConversationMetadata,
}

impl ConversationFile {
    /// Create new conversation
    pub fn new(
        conversations_dir: &Path,
        title: String,
        model: String,
        provider: String,
    ) -> Result<Self, std::io::Error> {
        create_dir_all(conversations_dir)?;

        let id = Uuid::new_v4().to_string();
        let short_id = &id[..8];
        let now = Utc::now();

        let filename = format!("{}-{}.jsonl", now.format("%Y-%m-%d-%H%M%S"), short_id);

        let file_path = conversations_dir.join(&filename);

        let metadata = ConversationMetadata {
            id: id.clone(),
            title,
            created_at: now.to_rfc3339(),
            updated_at: now.to_rfc3339(),
            model,
            provider,
            ..Default::default()
        };

        // Create file with metadata as first line
        let mut file = File::create(&file_path)?;
        writeln!(file, "{}", serde_json::to_string(&metadata)?)?;

        Ok(Self {
            id,
            file_path,
            metadata,
        })
    }

    /// Open existing conversation
    pub fn open(file_path: PathBuf) -> Result<Self, std::io::Error> {
        let file = File::open(&file_path)?;
        let mut reader = BufReader::new(file);

        let mut first_line = String::new();
        reader.read_line(&mut first_line)?;

        let metadata: ConversationMetadata = serde_json::from_str(first_line.trim())?;

        let mut conversation = Self {
            id: metadata.id.clone(),
            file_path,
            metadata,
        };
        // Migrate legacy records whose messages had no accounting or task
        // checkpoint. The transcript remains authoritative.
        let messages = conversation.load_all()?;
        let previous_metadata = conversation.metadata.clone();
        conversation.rebuild_derived_metadata(&messages);
        if conversation.metadata != previous_metadata {
            conversation.rewrite_metadata_line()?;
        }
        Ok(conversation)
    }

    /// Append message to conversation
    pub fn append_message(&mut self, msg: &StoredMessage) -> Result<(), std::io::Error> {
        // Append message to file
        let json = serde_json::to_string(msg)?;
        let mut file = OpenOptions::new().append(true).open(&self.file_path)?;
        writeln!(file, "{}", json)?;
        drop(file); // Close before rewriting

        // Update metadata in memory
        self.metadata.message_count += 1;
        self.metadata.total_tokens += effective_message_tokens(msg);
        self.metadata.updated_at = Utc::now().to_rfc3339();

        let messages = self.load_all()?;
        self.rebuild_derived_metadata(&messages);

        // Rewrite first line with updated metadata.
        self.rewrite_metadata_line()?;

        Ok(())
    }

    /// Rewrite the metadata line (first line) in the file
    fn rewrite_metadata_line(&self) -> Result<(), std::io::Error> {
        // Read all messages
        let all_messages = self.load_all()?;

        // Rewrite entire file
        let mut file = File::create(&self.file_path)?;

        // Write updated metadata as first line
        writeln!(file, "{}", serde_json::to_string(&self.metadata)?)?;

        // Write all messages back
        for msg in all_messages {
            writeln!(file, "{}", serde_json::to_string(&msg)?)?;
        }

        Ok(())
    }

    /// Load all messages
    pub fn load_all(&self) -> Result<Vec<StoredMessage>, std::io::Error> {
        let file = File::open(&self.file_path)?;
        let reader = BufReader::new(file);
        let mut messages = Vec::new();

        for (i, line) in reader.lines().enumerate() {
            if i == 0 {
                continue; // Skip metadata line
            }

            let line = line?;
            if let Ok(msg) = serde_json::from_str::<StoredMessage>(&line) {
                messages.push(msg);
            }
        }

        Ok(messages)
    }

    /// Load with context management (hybrid approach)
    pub fn load_for_context(
        &self,
        max_tokens: usize,
        keep_recent_count: usize,
    ) -> Result<Vec<StoredMessage>, std::io::Error> {
        let all_messages = self.load_all()?;

        if all_messages.is_empty() {
            return Ok(all_messages);
        }

        let total_tokens: usize = all_messages.iter().map(effective_message_tokens).sum();

        // If fits in context, return all
        if total_tokens < max_tokens {
            return Ok(all_messages);
        }

        let checkpoint_json = serde_json::to_string(&self.metadata.task_state)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
        let checkpoint_content = format!(
            "VortiDeck durable conversation checkpoint. Completed effects must not be repeated. Unresolved calls remain unfinished. Authoritative state: {checkpoint_json}"
        );
        let mut checkpoint = StoredMessage {
            id: "durable-task-checkpoint".to_string(),
            role: "system".to_string(),
            content: Some(checkpoint_content),
            timestamp: self.metadata.updated_at.clone(),
            tokens: None,
            token_source: TokenSource::Estimated,
            model: None,
            tool_calls: None,
            tool_call_id: None,
        };
        checkpoint.tokens = Some(effective_message_tokens(&checkpoint));
        let checkpoint_tokens = effective_message_tokens(&checkpoint);
        if checkpoint_tokens >= max_tokens {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "durable conversation checkpoint exceeds the model context budget",
            ));
        }

        let groups = protocol_groups(&all_messages);
        let mut selected_start = all_messages.len();
        let mut selected_tokens = checkpoint_tokens;
        let preferred_start = all_messages.len().saturating_sub(keep_recent_count);
        for (start, end) in groups.into_iter().rev() {
            let group_tokens: usize = all_messages[start..end]
                .iter()
                .map(effective_message_tokens)
                .sum();
            if selected_tokens + group_tokens > max_tokens {
                if selected_start == all_messages.len() {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "latest conversation task exceeds the model context budget",
                    ));
                }
                break;
            }
            selected_tokens += group_tokens;
            selected_start = start;
            if start <= preferred_start {
                break;
            }
        }

        let mut result = vec![checkpoint];
        result.extend_from_slice(&all_messages[selected_start..]);
        Ok(result)
    }

    fn rebuild_derived_metadata(&mut self, messages: &[StoredMessage]) {
        self.metadata.message_count = messages.len();
        self.metadata.total_tokens = messages.iter().map(effective_message_tokens).sum();
        self.metadata.task_state = derive_task_state(messages);
    }

    /// Get metadata
    pub fn metadata(&self) -> &ConversationMetadata {
        &self.metadata
    }

    /// Update title (rewrites metadata line in file)
    pub fn update_title(&mut self, title: String) -> Result<(), std::io::Error> {
        self.metadata.title = title;
        self.metadata.updated_at = Utc::now().to_rfc3339();

        // Rewrite the entire file with updated metadata
        let all_messages = self.load_all()?;

        // Recreate file with new metadata
        let mut file = File::create(&self.file_path)?;

        // Write metadata as first line
        writeln!(file, "{}", serde_json::to_string(&self.metadata)?)?;

        // Write all messages back
        for msg in all_messages {
            writeln!(file, "{}", serde_json::to_string(&msg)?)?;
        }

        println!("✏️ Updated conversation title: {}", self.metadata.title);
        Ok(())
    }

    /// Update stream setting (rewrites metadata line in file)
    pub fn update_stream(&mut self, stream: bool) -> Result<(), std::io::Error> {
        self.metadata.stream = stream;
        self.metadata.updated_at = Utc::now().to_rfc3339();
        self.rewrite_metadata_line()?;
        println!("🌊 Updated conversation stream mode: {}", stream);
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_runtime_settings(
        &mut self,
        model: String,
        temperature: f32,
        max_tokens: u32,
        tools_mode: ToolsMode,
        selected_tools: Vec<String>,
        system_prompt: Option<String>,
        tool_timeout: u64,
        stream: bool,
    ) -> Result<(), std::io::Error> {
        self.metadata.model = model;
        self.metadata.temperature = temperature;
        self.metadata.max_tokens = max_tokens;
        self.metadata.tools_mode = tools_mode;
        self.metadata.selected_tools = selected_tools;
        self.metadata.system_prompt = system_prompt;
        self.metadata.tool_timeout = tool_timeout;
        self.metadata.stream = stream;
        self.metadata.updated_at = Utc::now().to_rfc3339();
        self.rewrite_metadata_line()
    }
}

pub fn estimate_message_tokens(message: &StoredMessage) -> usize {
    effective_message_tokens(message)
}

fn effective_message_tokens(message: &StoredMessage) -> usize {
    message.tokens.unwrap_or_else(|| {
        let mut characters = message.role.chars().count() + 16;
        characters += message
            .content
            .as_deref()
            .map(str::chars)
            .map(Iterator::count)
            .unwrap_or(0);
        characters += message
            .tool_calls
            .as_ref()
            .map(Value::to_string)
            .map(|value| value.chars().count())
            .unwrap_or(0);
        characters += message.tool_call_id.as_deref().map(str::len).unwrap_or(0);
        // A conservative, tokenizer-independent fallback. UTF-8/code/JSON often
        // tokenize less efficiently than prose, so use three characters/token.
        4 + characters.div_ceil(3)
    })
}

fn protocol_groups(messages: &[StoredMessage]) -> Vec<(usize, usize)> {
    let mut groups = Vec::new();
    let mut index = 0;
    while index < messages.len() {
        let start = index;
        index += 1;
        if messages[start].role == "assistant"
            && messages[start]
                .tool_calls
                .as_ref()
                .and_then(Value::as_array)
                .is_some_and(|calls| !calls.is_empty())
        {
            while index < messages.len() && messages[index].role == "tool" {
                index += 1;
            }
        }
        groups.push((start, index));
    }
    groups
}

fn derive_task_state(messages: &[StoredMessage]) -> ConversationTaskState {
    let mut pending = BTreeMap::<String, PendingToolCall>::new();
    let mut completed_effects = Vec::new();
    let mut flow_revisions = BTreeMap::new();

    for message in messages {
        if message.role == "assistant" {
            for call in message
                .tool_calls
                .as_ref()
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let Some(call_id) = call.get("id").and_then(Value::as_str) else {
                    continue;
                };
                let function = call.get("function").unwrap_or(&Value::Null);
                let tool_name = function
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string();
                let arguments = function
                    .get("arguments")
                    .and_then(Value::as_str)
                    .and_then(|value| serde_json::from_str(value).ok())
                    .unwrap_or_else(|| function.get("arguments").cloned().unwrap_or(Value::Null));
                pending.insert(
                    call_id.to_string(),
                    PendingToolCall {
                        call_id: call_id.to_string(),
                        tool_name,
                        arguments,
                    },
                );
            }
        }

        if message.role != "tool" {
            continue;
        }
        let Some(call_id) = message.tool_call_id.as_deref() else {
            continue;
        };
        let tool_name = pending
            .remove(call_id)
            .map(|call| call.tool_name)
            .unwrap_or_else(|| "unknown".to_string());
        let result = message
            .content
            .as_deref()
            .and_then(|content| serde_json::from_str::<Value>(content).ok())
            .unwrap_or(Value::Null);

        if let (Some(invocation_id), Some(status)) = (
            result.get("invocation_id").and_then(Value::as_str),
            result.get("status").and_then(Value::as_str),
        ) {
            completed_effects.push(CompletedEffectReceipt {
                call_id: call_id.to_string(),
                tool_name: tool_name.clone(),
                invocation_id: invocation_id.to_string(),
                status: status.to_string(),
            });
        }

        if let (Some(flow_id), Some(revision)) = (
            result
                .get("flow")
                .and_then(|flow| flow.get("id"))
                .and_then(Value::as_str)
                .or_else(|| result.get("flow_id").and_then(Value::as_str)),
            result.get("revision").and_then(Value::as_str),
        ) {
            flow_revisions.insert(flow_id.to_string(), revision.to_string());
        }
        if let Some(receipt) = result.get("receipt") {
            if let (Some(flow_id), Some(revision)) = (
                receipt
                    .get("flowId")
                    .or_else(|| receipt.get("flow_id"))
                    .and_then(Value::as_str),
                receipt
                    .get("afterRevision")
                    .or_else(|| receipt.get("after_revision"))
                    .and_then(Value::as_str),
            ) {
                flow_revisions.insert(flow_id.to_string(), revision.to_string());
            }
        }
    }

    if completed_effects.len() > 64 {
        completed_effects.drain(..completed_effects.len() - 64);
    }
    ConversationTaskState {
        unresolved_calls: pending.into_values().collect(),
        completed_effects,
        flow_revisions,
    }
}

/// List all conversations in directory
pub fn list_conversations(
    conversations_dir: &Path,
) -> Result<Vec<ConversationMetadata>, std::io::Error> {
    create_dir_all(conversations_dir)?;

    let mut conversations = Vec::new();

    for entry in std::fs::read_dir(conversations_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
            if let Ok(conv_file) = ConversationFile::open(path) {
                conversations.push(conv_file.metadata().clone());
            }
        }
    }

    // Sort by updated_at (most recent first)
    conversations.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    Ok(conversations)
}

/// Get conversations directory path
pub fn get_conversations_dir() -> Result<PathBuf, String> {
    let app_data = dirs::data_dir().ok_or("Could not find app data directory")?;

    let dir = app_data.join("VortiDeck").join("conversations");
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(role: &str, content: &str) -> StoredMessage {
        StoredMessage {
            id: Uuid::new_v4().to_string(),
            role: role.to_string(),
            content: Some(content.to_string()),
            timestamp: Utc::now().to_rfc3339(),
            tokens: None,
            token_source: TokenSource::Unknown,
            model: Some("fixture".to_string()),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    #[test]
    fn legacy_missing_tokens_are_estimated_and_migrated_on_open() {
        let directory = tempfile::tempdir().unwrap();
        let mut file = ConversationFile::new(
            directory.path(),
            "Legacy".into(),
            "gpt-4".into(),
            "OpenAI".into(),
        )
        .unwrap();
        file.append_message(&message("user", &"context ".repeat(200)))
            .unwrap();
        let path = file.file_path.clone();

        let reopened = ConversationFile::open(path).unwrap();
        assert!(reopened.metadata().total_tokens > 400);
        assert_eq!(reopened.metadata().message_count, 1);
    }

    #[test]
    fn compaction_preserves_tool_protocol_and_durable_effect_checkpoint() {
        let directory = tempfile::tempdir().unwrap();
        let mut file = ConversationFile::new(
            directory.path(),
            "Long task".into(),
            "gpt-4".into(),
            "OpenAI".into(),
        )
        .unwrap();
        for index in 0..12 {
            file.append_message(&message(
                if index % 2 == 0 { "user" } else { "assistant" },
                &format!("old context {index} {}", "x".repeat(180)),
            ))
            .unwrap();
        }
        let mut proposal = message("assistant", "I will update OBS.");
        proposal.tool_calls = Some(serde_json::json!([{
            "id": "call-obs-1",
            "type": "function",
            "function": {
                "name": "vortideck_obs_action",
                "arguments": "{\"action_id\":\"set_scene\"}"
            }
        }]));
        file.append_message(&proposal).unwrap();
        let mut result = message("tool", r#"{"status":"applied","invocation_id":"inv-1"}"#);
        result.tool_call_id = Some("call-obs-1".into());
        file.append_message(&result).unwrap();
        file.append_message(&message("user", "What happened?"))
            .unwrap();

        let compacted = file.load_for_context(650, 4).unwrap();
        assert_eq!(compacted[0].id, "durable-task-checkpoint");
        let has_proposal = compacted.iter().any(|item| {
            item.tool_calls
                .as_ref()
                .and_then(Value::as_array)
                .is_some_and(|calls| !calls.is_empty())
        });
        let has_result = compacted
            .iter()
            .any(|item| item.tool_call_id.as_deref() == Some("call-obs-1"));
        assert_eq!(has_proposal, has_result);
        assert_eq!(file.metadata().task_state.unresolved_calls.len(), 0);
        assert_eq!(file.metadata().task_state.completed_effects.len(), 1);
        assert_eq!(
            file.metadata().task_state.completed_effects[0].invocation_id,
            "inv-1"
        );

        let reopened = ConversationFile::open(file.file_path.clone()).unwrap();
        assert_eq!(reopened.metadata().task_state, file.metadata().task_state);
    }

    #[test]
    fn unresolved_call_is_machine_readable_after_reload() {
        let directory = tempfile::tempdir().unwrap();
        let mut file = ConversationFile::new(
            directory.path(),
            "Interrupted".into(),
            "gpt-4".into(),
            "OpenAI".into(),
        )
        .unwrap();
        let mut proposal = message("assistant", "");
        proposal.tool_calls = Some(serde_json::json!([{
            "id": "pending-call",
            "type": "function",
            "function": { "name": "vortideck_get_flow", "arguments": "{\"flow_id\":\"f1\"}" }
        }]));
        file.append_message(&proposal).unwrap();

        let reopened = ConversationFile::open(file.file_path.clone()).unwrap();
        let pending = &reopened.metadata().task_state.unresolved_calls[0];
        assert_eq!(pending.call_id, "pending-call");
        assert_eq!(pending.tool_name, "vortideck_get_flow");
        assert_eq!(pending.arguments["flow_id"], "f1");
    }
}
