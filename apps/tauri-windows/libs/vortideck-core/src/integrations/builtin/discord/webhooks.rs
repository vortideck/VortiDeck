use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordWebhook {
    pub id: String,
    pub url: String,
    pub name: Option<String>,
    pub avatar: Option<String>,
    pub guild_id: Option<String>,
    pub channel_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordEmbed {
    pub title: Option<String>,
    pub description: Option<String>,
    pub url: Option<String>,
    pub timestamp: Option<String>,
    pub color: Option<u32>,
    pub footer: Option<DiscordEmbedFooter>,
    pub image: Option<DiscordEmbedImage>,
    pub thumbnail: Option<DiscordEmbedImage>,
    pub author: Option<DiscordEmbedAuthor>,
    pub fields: Option<Vec<DiscordEmbedField>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordEmbedFooter {
    pub text: String,
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordEmbedImage {
    pub url: String,
    pub proxy_url: Option<String>,
    pub height: Option<u32>,
    pub width: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordEmbedAuthor {
    pub name: String,
    pub url: Option<String>,
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordEmbedField {
    pub name: String,
    pub value: String,
    pub inline: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordWebhookMessage {
    pub content: Option<String>,
    pub username: Option<String>,
    pub avatar_url: Option<String>,
    pub tts: Option<bool>,
    pub embeds: Option<Vec<DiscordEmbed>>,
}

impl DiscordWebhook {
    pub fn new(url: String) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Parse webhook URL to extract ID and token
        if !url.starts_with("https://discord.com/api/webhooks/") {
            return Err("Invalid Discord webhook URL format".into());
        }

        // Extract webhook ID from URL
        let parts: Vec<&str> = url.split('/').collect();
        if parts.len() < 6 {
            return Err("Invalid Discord webhook URL format".into());
        }

        let id = parts[5].to_string();

        Ok(Self {
            id,
            url,
            name: None,
            avatar: None,
            guild_id: None,
            channel_id: "unknown".to_string(), // Would need to fetch from Discord API
        })
    }

    /// Send a message through the webhook
    /// Uses the global API client for optimized connection pooling
    pub async fn send_message(
        &self,
        message: DiscordWebhookMessage,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use crate::services::api::post;

        let response = post(&self.url)
            .json(serde_json::to_value(&message)?)
            .send()
            .await?;

        if response.status >= 200 && response.status < 300 {
            Ok(())
        } else {
            Err(format!(
                "Failed to send webhook message: {} (status: {})",
                response.body, response.status
            )
            .into())
        }
    }

    /// Send a simple text message
    pub async fn send_text(
        &self,
        content: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let message = DiscordWebhookMessage {
            content: Some(content.to_string()),
            username: None,
            avatar_url: None,
            tts: None,
            embeds: None,
        };

        self.send_message(message).await
    }

    /// Send an embed message
    pub async fn send_embed(
        &self,
        embed: DiscordEmbed,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let message = DiscordWebhookMessage {
            content: None,
            username: None,
            avatar_url: None,
            tts: None,
            embeds: Some(vec![embed]),
        };

        self.send_message(message).await
    }

    /// Send a notification with VortiDeck branding
    pub async fn send_vortideck_notification(
        &self,
        title: &str,
        description: &str,
        color: Option<u32>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let embed = DiscordEmbed {
            title: Some(title.to_string()),
            description: Some(description.to_string()),
            url: None,
            timestamp: Some(chrono::Utc::now().to_rfc3339()),
            color: color.or(Some(0x0099ff)),
            footer: Some(DiscordEmbedFooter {
                text: "VortiDeck".to_string(),
                icon_url: Some("https://vortideck.com/favicon.ico".to_string()),
            }),
            image: None,
            thumbnail: None,
            author: None,
            fields: None,
        };

        self.send_embed(embed).await
    }

    /// Send an alert message with appropriate styling
    pub async fn send_alert(
        &self,
        message: &str,
        severity: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (color, emoji, title) = match severity.to_lowercase().as_str() {
            "error" => (0xff0000, "🚨", "Error Alert"),
            "warning" => (0xffaa00, "⚠️", "Warning Alert"),
            "success" => (0x00ff00, "✅", "Success"),
            "info" => (0x0099ff, "ℹ️", "Information"),
            _ => (0x0099ff, "📢", "Alert"),
        };

        let embed = DiscordEmbed {
            title: Some(format!("{} {}", emoji, title)),
            description: Some(message.to_string()),
            url: None,
            timestamp: Some(chrono::Utc::now().to_rfc3339()),
            color: Some(color),
            footer: Some(DiscordEmbedFooter {
                text: "VortiDeck Alert System".to_string(),
                icon_url: None,
            }),
            image: None,
            thumbnail: None,
            author: None,
            fields: None,
        };

        self.send_embed(embed).await
    }

    /// Test webhook connectivity
    pub async fn test_connection(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let test_embed = DiscordEmbed {
            title: Some("🤖 VortiDeck Integration Test".to_string()),
            description: Some(
                "This is a test message to verify Discord webhook connectivity.".to_string(),
            ),
            url: None,
            timestamp: Some(chrono::Utc::now().to_rfc3339()),
            color: Some(0x00ff00),
            footer: Some(DiscordEmbedFooter {
                text: "VortiDeck Discord Integration".to_string(),
                icon_url: Some("https://vortideck.com/favicon.ico".to_string()),
            }),
            image: None,
            thumbnail: None,
            author: None,
            fields: Some(vec![
                DiscordEmbedField {
                    name: "Status".to_string(),
                    value: "✅ Connected".to_string(),
                    inline: Some(true),
                },
                DiscordEmbedField {
                    name: "Test Time".to_string(),
                    value: chrono::Utc::now()
                        .format("%Y-%m-%d %H:%M:%S UTC")
                        .to_string(),
                    inline: Some(true),
                },
            ]),
        };

        self.send_embed(test_embed).await
    }

    /// Get webhook information (requires additional API call)
    /// Uses the global API client for optimized connection pooling
    pub async fn get_info(
        &self,
    ) -> Result<DiscordWebhook, Box<dyn std::error::Error + Send + Sync>> {
        use crate::services::api::get;

        let response = get(&self.url).send().await?;

        if response.status >= 200 && response.status < 300 {
            let webhook_data: Value = serde_json::from_str(&response.body)?;

            Ok(DiscordWebhook {
                id: webhook_data["id"].as_str().unwrap_or(&self.id).to_string(),
                url: self.url.clone(),
                name: webhook_data["name"].as_str().map(|s| s.to_string()),
                avatar: webhook_data["avatar"].as_str().map(|s| s.to_string()),
                guild_id: webhook_data["guild_id"].as_str().map(|s| s.to_string()),
                channel_id: webhook_data["channel_id"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string(),
            })
        } else {
            Err(format!("Failed to get webhook info: {}", response.status).into())
        }
    }
}
