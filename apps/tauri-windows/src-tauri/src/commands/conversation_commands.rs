//! Conversation Management Commands
//!
//! Commands for managing persistent conversation history

use serde::Serialize;
use tauri::command;
use vortideck_core::services::mcp::{
    commands::ChatState, conversation_file::*, provider::AIProvider,
};

#[command]
pub async fn create_conversation(
    title: String,
    model: String,
    provider: String,
) -> Result<String, String> {
    println!("📝 Creating new conversation: {}", title);

    let conversations_dir = get_conversations_dir()?;

    let file = ConversationFile::new(&conversations_dir, title, model, provider)
        .map_err(|e| e.to_string())?;

    println!("✅ Conversation created: {}", file.id);
    Ok(file.id)
}

#[command]
pub async fn list_conversations() -> Result<Vec<ConversationMetadata>, String> {
    println!("📋 Listing conversations");

    let conversations_dir = get_conversations_dir()?;
    let conversations =
        vortideck_core::services::mcp::conversation_file::list_conversations(&conversations_dir)
            .map_err(|e| e.to_string())?;

    println!("✅ Found {} conversations", conversations.len());
    Ok(conversations)
}

#[command]
pub async fn load_conversation_messages(
    conversation_id: String,
    use_context_management: bool,
) -> Result<Vec<StoredMessage>, String> {
    println!("📖 Loading conversation: {}", conversation_id);

    let conversations_dir = get_conversations_dir()?;

    // Find conversation file by ID
    for entry in std::fs::read_dir(&conversations_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
            if let Ok(file) = ConversationFile::open(path) {
                if file.id == conversation_id {
                    let messages = if use_context_management {
                        file.load_for_context(100000, 30)
                    } else {
                        file.load_all()
                    };

                    let messages = messages.map_err(|e| e.to_string())?;

                    println!("✅ Loaded {} messages", messages.len());
                    return Ok(messages);
                }
            }
        }
    }

    Err(format!("Conversation {} not found", conversation_id))
}

#[command]
pub async fn delete_conversation(conversation_id: String) -> Result<(), String> {
    println!("🗑️ Deleting conversation: {}", conversation_id);

    let conversations_dir = get_conversations_dir()?;

    for entry in std::fs::read_dir(&conversations_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
            if let Ok(file) = ConversationFile::open(path.clone()) {
                if file.id == conversation_id {
                    std::fs::remove_file(path).map_err(|e| e.to_string())?;
                    println!("✅ Conversation deleted");
                    return Ok(());
                }
            }
        }
    }

    Err(format!("Conversation {} not found", conversation_id))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationSettings {
    provider: String,
    model: String,
    temperature: f32,
    max_tokens: u32,
    tools_mode: String,
    selected_tools: Vec<String>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_prompt: Option<String>,
    tool_timeout: u64,
}

/// Attach a conversation to a session
///
/// Gets or creates a session for the conversation, then attaches the conversation file.
#[command]
pub async fn attach_conversation_to_session(
    chat_state: tauri::State<'_, ChatState>,
    conversation_id: String,
    use_context_management: bool,
) -> Result<ConversationSettings, String> {
    println!("🔗 Attaching conversation {} to session", conversation_id);

    let conversations_dir = get_conversations_dir()?;

    // Find conversation file
    for entry in std::fs::read_dir(&conversations_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
            if let Ok(file) = ConversationFile::open(path) {
                if file.id == conversation_id {
                    // Get metadata before attaching (need provider info to create session)
                    let metadata = file.metadata().clone();

                    // Create provider from metadata
                    let provider = create_provider_from_metadata(&metadata, &chat_state).await?;

                    // Get or create session for this conversation
                    let session_key = chat_state
                        .session_manager
                        .get_or_create_conversation(
                            conversation_id.clone(),
                            provider,
                            Some(metadata.model.clone()),
                        )
                        .await?;

                    // Get session handle for attaching conversation
                    let handle = chat_state
                        .session_manager
                        .get(&session_key)
                        .await
                        .ok_or("Failed to get session after creation")?;

                    // Attach conversation file to session via actor
                    handle
                        .attach_conversation_file(file)
                        .await
                        .map_err(|e| format!("Failed to attach conversation: {}", e))?;

                    // Load messages via actor
                    handle
                        .load_from_file(use_context_management)
                        .await
                        .map_err(|e| e.to_string())?;

                    println!("✅ Conversation attached to session: {}", session_key);

                    return Ok(ConversationSettings {
                        provider: metadata.provider.clone(),
                        model: metadata.model.clone(),
                        temperature: metadata.temperature,
                        max_tokens: metadata.max_tokens,
                        tools_mode: metadata.tools_mode.as_str().to_string(),
                        selected_tools: metadata.selected_tools.clone(),
                        stream: metadata.stream,
                        system_prompt: metadata.system_prompt.clone(),
                        tool_timeout: metadata.tool_timeout,
                    });
                }
            }
        }
    }

    Err(format!("Conversation {} not found", conversation_id))
}

/// Create AI provider from conversation metadata
async fn create_provider_from_metadata(
    metadata: &ConversationMetadata,
    chat_state: &ChatState,
) -> Result<AIProvider, String> {
    // Try to find matching provider in config
    let config_guard = chat_state.config.lock().await;
    if let Some(config) = config_guard.as_ref() {
        if let Some(model_config) = config
            .models
            .model
            .iter()
            .find(|m| m.name == metadata.provider || m.provider == metadata.provider)
        {
            return AIProvider::from_model_config(model_config)
                .map_err(|e| format!("Failed to create provider: {}", e));
        }
    }

    // Fallback: use first configured provider
    if let Some(config) = config_guard.as_ref() {
        if let Some(first) = config.models.model.first() {
            return AIProvider::from_model_config(first)
                .map_err(|e| format!("Failed to create provider: {}", e));
        }
    }

    Err("No AI providers configured".to_string())
}

/// Create a new conversation and attach it to a session
///
/// Creates a new conversation file and creates a session for it.
#[command]
pub async fn create_and_attach_conversation(
    chat_state: tauri::State<'_, ChatState>,
    title: String,
    model: String,
    provider_name: String,
) -> Result<String, String> {
    println!("📝 Creating and attaching new conversation: {}", title);

    let conversations_dir = get_conversations_dir()?;

    // Create conversation file
    let file = ConversationFile::new(
        &conversations_dir,
        title.clone(),
        model.clone(),
        provider_name.clone(),
    )
    .map_err(|e| e.to_string())?;

    let conversation_id = file.id.clone();

    // Get provider from config
    let provider = {
        let config_guard = chat_state.config.lock().await;
        let config = config_guard.as_ref().ok_or("No configuration loaded")?;

        let model_config = config
            .models
            .model
            .iter()
            .find(|m| m.name == provider_name || m.provider == provider_name)
            .or_else(|| config.models.model.first())
            .ok_or("No AI providers configured")?;

        AIProvider::from_model_config(model_config)
            .map_err(|e| format!("Failed to create provider: {}", e))?
    };

    // Create session for this conversation
    let session_key = chat_state
        .session_manager
        .get_or_create_conversation(conversation_id.clone(), provider, Some(model.clone()))
        .await?;

    // Get session handle for attaching conversation
    let handle = chat_state
        .session_manager
        .get(&session_key)
        .await
        .ok_or("Failed to get session after creation")?;

    // Attach conversation file to session via actor
    handle
        .attach_conversation_file(file)
        .await
        .map_err(|e| format!("Failed to attach conversation: {}", e))?;

    // Load from the new (empty) file to initialize via actor
    handle
        .load_from_file(false)
        .await
        .map_err(|e| format!("Failed to initialize conversation: {}", e))?;

    println!("✅ Conversation created and attached: {}", conversation_id);
    println!(
        "📁 File: {:?}",
        conversations_dir.join(format!("{}.jsonl", conversation_id))
    );
    Ok(conversation_id)
}

#[command]
pub async fn update_conversation_title(
    conversation_id: String,
    new_title: String,
) -> Result<(), String> {
    println!(
        "✏️ Updating conversation {} title to: {}",
        conversation_id, new_title
    );

    let conversations_dir = get_conversations_dir()?;

    // Find conversation file by ID
    for entry in std::fs::read_dir(&conversations_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
            if let Ok(mut file) = ConversationFile::open(path) {
                if file.id == conversation_id {
                    file.update_title(new_title).map_err(|e| e.to_string())?;
                    println!("✅ Conversation title updated");
                    return Ok(());
                }
            }
        }
    }

    Err(format!("Conversation {} not found", conversation_id))
}
