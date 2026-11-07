use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unified chat message from any platform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    #[serde(alias = "message_id")]
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_id: Option<u64>,
    pub platform: Platform,
    pub user_id: String,
    pub user_name: String,
    pub user_display_name: String,
    pub message: String,
    pub timestamp: String, // ISO 8601
    pub badges: Vec<String>,
    pub is_moderator: bool,
    pub is_subscriber: bool,
    pub is_vip: bool,
    pub platform_color: String,
    pub emotes: Option<Vec<Emote>>,
    pub mentions: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Twitch,
    YouTube,
    Kick,
    TikTok,
    Discord,
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Platform::Twitch => write!(f, "twitch"),
            Platform::YouTube => write!(f, "youtube"),
            Platform::Kick => write!(f, "kick"),
            Platform::TikTok => write!(f, "tiktok"),
            Platform::Discord => write!(f, "discord"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Emote {
    pub id: String,
    pub name: String,
    pub url: String,
}

/// Cloud events from backend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CloudEvent {
    ConnectionEstablished {
        streamer_id: String,
        display_name: String,
        timestamp: String,
        #[serde(default)]
        active_streams: Vec<ActiveStream>,
    },
    ChatMessage {
        account_id: u64,
        platform: Platform,
        data: ChatMessage,
    },
    StreamOnline {
        account_id: u64,
        platform: Platform,
        data: StreamData,
    },
    StreamOffline {
        account_id: u64,
        platform: Platform,
    },
    ChannelUpdate {
        account_id: u64,
        platform: Platform,
        data: ChannelUpdateData,
    },
    Subscription {
        account_id: u64,
        platform: Platform,
        data: serde_json::Value,
    },
    Raid {
        account_id: u64,
        platform: Platform,
        data: serde_json::Value,
    },
    Follow {
        account_id: u64,
        platform: Platform,
        data: serde_json::Value,
    },
    Bits {
        account_id: u64,
        platform: Platform,
        data: serde_json::Value,
    },
    PlatformStatus {
        platform: Platform,
        status: String,
        message: Option<String>,
    },
    SendChatResult {
        success: bool,
        platforms_sent: Option<Vec<Platform>>,
        errors: Option<Vec<serde_json::Value>>,
    },
    Error {
        message: String,
        status: Option<String>,
    },
    Ping {
        timestamp: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveStream {
    pub account_id: u64,
    pub platform: Platform,
    pub stream_id: String,
    pub title: String,
    pub started_at: String,
    pub game_name: Option<String>,
    pub viewer_count: Option<u32>,
    pub stream_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamData {
    pub stream_id: String,
    pub title: String,
    pub started_at: String,
    pub viewer_count: Option<u32>,
    pub game_name: Option<String>,
    pub stream_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelUpdateData {
    pub title: Option<String>,
    pub viewer_count: Option<u32>,
    pub game_name: Option<String>,
}

/// Stream session (reusable, managed in Rust)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")] // Always output camelCase when serializing
pub struct StreamSession {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub platforms: Vec<PlatformStream>,
    pub commands: Vec<SessionCommand>,
    pub settings: SessionSettings,
    pub status: SessionStatus,

    // Metadata
    pub created_at: String,
    pub updated_at: String,

    // Current run data (resets on each activation)
    pub current_run_started_at: Option<String>,
    pub paused_at: Option<String>,

    // Current run stats (resets on each end)
    pub duration: Option<u64>,
    pub message_count: Option<u64>,
    pub command_executions: Option<u64>,

    // Historical data (persists across runs)
    pub runs: Vec<StreamRun>,
    pub total_runs: u32,
    pub last_used_at: Option<String>,
    pub total_duration: Option<u64>,
    pub total_messages: Option<u64>,
    pub total_commands: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformStream {
    pub platform: Platform,
    pub account_id: String, // String for flexibility (some platforms use alphanumeric IDs)
    pub stream_id: Option<String>,
    pub title: Option<String>,
    pub started_at: Option<String>,
    pub viewer_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionCommand {
    /// Stable identity for delivery deduplication, independent of display text.
    #[serde(default)]
    pub id: String,
    /// Version 0 is migrated with its original public/default semantics.
    #[serde(default)]
    pub permission_version: u8,
    pub trigger: String,
    pub action: String, // JSON-encoded action (same format as button actions)
    pub cooldown: Option<u32>,
    pub enabled: bool,
    /// Platform-specific roles that can use this command
    /// Format: { "twitch": ["broadcaster", "moderator"], "youtube": ["owner"] }
    #[serde(default)]
    pub allowed_roles: Option<std::collections::HashMap<String, Vec<String>>>,
    /// Optional: Specific usernames allowed (whitelist)
    #[serde(default)]
    pub user_whitelist: Option<Vec<String>>,
    /// Optional: Specific usernames blocked (blacklist)
    #[serde(default)]
    pub user_blacklist: Option<Vec<String>>,
    #[serde(default)]
    pub input_bindings: std::collections::HashMap<String, CommandInputSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "camelCase", deny_unknown_fields)]
pub enum CommandInputSource {
    Args,
    ArgsNumber,
    ArgsBoolean,
    Message,
    Username,
    DisplayName,
    Platform,
    Literal { value: serde_json::Value },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSettings {
    pub auto_start: bool,
    pub notifications: bool,
    pub auto_resume: bool,
    #[serde(default = "default_grace_period")]
    pub grace_period_minutes: Option<u32>,
    #[serde(default)]
    pub priority: Option<u32>,
}

fn default_grace_period() -> Option<u32> {
    Some(30)
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Draft,
    WaitingForLive,
    Active,
    Paused,
    Inactive, // Reusable - can be activated again
}

/// Historical record of a single stream run
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamRun {
    pub started_at: String,
    pub ended_at: String,
    pub duration: u64, // Duration in ms
    pub message_count: u64,
    pub command_executions: u64,
    pub ended_by: EndedBy,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EndedBy {
    User,
    Auto,
}

/// Request to send chat message to platforms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendChatRequest {
    pub account_ids: Vec<u64>,
    pub message: String,
    pub reply_to: Option<String>,
}
