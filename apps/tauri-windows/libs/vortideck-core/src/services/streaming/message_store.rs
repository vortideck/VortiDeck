use super::error::{StreamingError, StreamingResult};
use super::models::{ChatMessage, Platform};
use crate::db::{models::NewChatMessage, Database};
use std::sync::Arc;

/// Message store - Database-backed persistence
///
/// Messages are stored in SQLite database only (no in-memory HashMap).
/// This provides persistence across app restarts while keeping the architecture simple.
/// Messages are kept for the last 2 session runs per session.
#[derive(Clone, Debug)]
pub struct MessageStore {
    /// Database for message persistence
    database: Arc<Database>,
}

impl MessageStore {
    /// Create a new MessageStore with database backend
    pub fn new(database: Arc<Database>) -> Self {
        Self { database }
    }

    /// Store a chat message without session context
    pub async fn store_message(&self, message: ChatMessage) -> StreamingResult<()> {
        self.store_message_with_session(message, None, None).await
    }

    /// Store a chat message with session context (saves to database)
    pub async fn store_message_with_session(
        &self,
        message: ChatMessage,
        session_id: Option<String>,
        session_run_index: Option<i32>,
    ) -> StreamingResult<()> {
        let db = self.database.clone();
        let session_id_opt = session_id.clone();

        // The pool checkout and Diesel insert are synchronous, so they run on the
        // blocking pool instead of a Tokio worker. A panic there (e.g. pool
        // exhaustion) is confined to that task and logged.
        tokio::spawn(async move {
            let saved = tokio::task::spawn_blocking(move || {
                tokio::runtime::Handle::current().block_on(Self::save_to_db(
                    &db,
                    &message,
                    session_id_opt.as_deref(),
                    session_run_index,
                ))
            })
            .await;
            match saved {
                Ok(Ok(())) => {}
                Ok(Err(e)) => log::error!("❌ Failed to save message to database: {}", e),
                Err(e) => log::error!("❌ Chat message persistence task failed: {}", e),
            }
        });

        Ok(())
    }

    /// Save a message to the database
    async fn save_to_db(
        db: &Database,
        message: &ChatMessage,
        session_id: Option<&str>,
        session_run_index: Option<i32>,
    ) -> Result<(), diesel::result::Error> {
        // Convert ChatMessage to NewChatMessage for insertion
        let badges_json = serde_json::to_string(&message.badges).ok();
        let emotes_json = message
            .emotes
            .as_ref()
            .and_then(|e| serde_json::to_string(e).ok());
        let mentions_json = message
            .mentions
            .as_ref()
            .and_then(|m| serde_json::to_string(m).ok());

        let new_msg = NewChatMessage {
            id: &message.id,
            account_id: message.account_id.unwrap_or(0) as i32,
            session_id,
            session_run_index,
            platform: &message.platform.to_string(),
            user_id: &message.user_id,
            user_name: &message.user_name,
            user_display_name: &message.user_display_name,
            message: &message.message,
            timestamp: &message.timestamp,
            badges: badges_json.as_deref(),
            is_moderator: message.is_moderator as i32,
            is_subscriber: message.is_subscriber as i32,
            is_vip: message.is_vip as i32,
            platform_color: Some(&message.platform_color),
            emotes: emotes_json.as_deref(),
            mentions: mentions_json.as_deref(),
        };

        db.save_chat_message(new_msg).await
    }

    /// Get messages for a specific account from database
    pub async fn get_messages(
        &self,
        account_id: u64,
        limit: Option<usize>,
    ) -> StreamingResult<Vec<ChatMessage>> {
        let db_messages = self
            .database
            .load_messages_for_accounts(&[account_id], limit.map(|l| l as i32))
            .await
            .map_err(|e| StreamingError::DatabaseError(e.to_string()))?;

        // Convert ChatMessageDb to ChatMessage
        Ok(db_messages
            .into_iter()
            .map(Self::db_to_chat_message)
            .collect())
    }

    /// Get messages for multiple accounts (aggregated) from database
    pub async fn get_messages_for_accounts(
        &self,
        account_ids: Vec<u64>,
        limit: Option<usize>,
    ) -> StreamingResult<Vec<ChatMessage>> {
        let db_messages = self
            .database
            .load_messages_for_accounts(&account_ids, limit.map(|l| l as i32))
            .await
            .map_err(|e| StreamingError::DatabaseError(e.to_string()))?;

        // Convert ChatMessageDb to ChatMessage
        Ok(db_messages
            .into_iter()
            .map(Self::db_to_chat_message)
            .collect())
    }

    /// Convert ChatMessageDb (from database) to ChatMessage (service model)
    fn db_to_chat_message(db_msg: crate::db::models::ChatMessageDb) -> ChatMessage {
        ChatMessage {
            id: db_msg.id,
            account_id: Some(db_msg.account_id as u64),
            platform: match db_msg.platform.as_str() {
                "twitch" => Platform::Twitch,
                "youtube" => Platform::YouTube,
                "kick" => Platform::Kick,
                "tiktok" => Platform::TikTok,
                "discord" => Platform::Discord,
                _ => Platform::Twitch, // fallback
            },
            user_id: db_msg.user_id,
            user_name: db_msg.user_name,
            user_display_name: db_msg.user_display_name,
            message: db_msg.message,
            timestamp: db_msg.timestamp,
            badges: db_msg
                .badges
                .and_then(|b| serde_json::from_str(&b).ok())
                .unwrap_or_default(),
            is_moderator: db_msg.is_moderator != 0,
            is_subscriber: db_msg.is_subscriber != 0,
            is_vip: db_msg.is_vip != 0,
            platform_color: db_msg
                .platform_color
                .unwrap_or_else(|| "#FFFFFF".to_string()),
            emotes: db_msg.emotes.and_then(|e| serde_json::from_str(&e).ok()),
            mentions: db_msg.mentions.and_then(|m| serde_json::from_str(&m).ok()),
        }
    }

    /// Cleanup old session runs (keep only last 2 runs per session)
    pub async fn cleanup_old_runs(
        &self,
        session_id: &str,
        current_run_index: i32,
    ) -> StreamingResult<usize> {
        let keep_from = current_run_index.saturating_sub(1);

        self.database
            .cleanup_old_session_runs(session_id, keep_from)
            .await
            .map_err(|e| StreamingError::DatabaseError(e.to_string()))
    }
}

use serde::{Deserialize, Serialize};
