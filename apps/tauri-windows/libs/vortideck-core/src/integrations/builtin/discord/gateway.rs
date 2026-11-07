//! Discord Gateway transport for permission-scoped application commands.
//!
//! Provider response tokens are retained only in this adapter's in-memory
//! receipt map. They are never included in normalized commands, persisted
//! event envelopes, callbacks, or logs.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{TimeZone, Utc};
use tokio::sync::Mutex;
use twilight_gateway::{
    CloseFrame, Event, EventTypeFlags, Intents, Shard, ShardId, StreamExt as _,
};
use twilight_http::Client as DiscordHttpClient;
use twilight_model::application::interaction::InteractionData;
use twilight_model::channel::message::MessageFlags;
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_model::id::marker::{ApplicationMarker, InteractionMarker};
use twilight_model::id::Id;

use super::inbound::{
    DiscordInboundCommand, DiscordInboundReceipt, DiscordInboundTransport, DiscordRejectionReason,
    DiscordResponseError,
};

#[derive(Debug, Clone)]
struct ResponseContext {
    interaction_id: Id<InteractionMarker>,
    token: String,
    acknowledged: bool,
}

/// A single-shard Discord Gateway client. The shard performs heartbeat and
/// reconnect handling; this adapter only selects application-command events
/// and maps provider identity into the transport-neutral ingress contract.
pub struct TwilightDiscordTransport {
    application_id: Id<ApplicationMarker>,
    shard: Mutex<Shard>,
    http: DiscordHttpClient,
    responses: Mutex<HashMap<String, ResponseContext>>,
}

impl TwilightDiscordTransport {
    pub fn new(bot_token: String, application_id: u64) -> Result<Arc<Self>, String> {
        if bot_token.trim().is_empty() {
            return Err("Discord bot token cannot be empty".to_string());
        }
        let application_id = Id::new_checked(application_id)
            .ok_or_else(|| "Discord application ID cannot be zero".to_string())?;
        Ok(Arc::new(Self {
            application_id,
            shard: Mutex::new(Shard::new(ShardId::ONE, bot_token.clone(), Intents::GUILDS)),
            http: DiscordHttpClient::new(bot_token),
            responses: Mutex::new(HashMap::new()),
        }))
    }

    fn occurred_at(interaction_id: u64) -> chrono::DateTime<Utc> {
        // Discord snowflakes encode milliseconds since 2015-01-01 in their
        // upper 42 bits. Falling back to receipt time is fail-safe because the
        // core still applies its narrow maximum-age policy.
        const DISCORD_EPOCH_MS: i64 = 1_420_070_400_000;
        let timestamp_ms = ((interaction_id >> 22) as i64).saturating_add(DISCORD_EPOCH_MS);
        Utc.timestamp_millis_opt(timestamp_ms)
            .single()
            .unwrap_or_else(Utc::now)
    }

    fn receipt_message(receipt: &DiscordInboundReceipt) -> String {
        match receipt {
            DiscordInboundReceipt::Accepted { run_id } => {
                format!("VortiDeck accepted the command as workflow run `{run_id}`.")
            }
            DiscordInboundReceipt::Duplicate {
                run_id: Some(run_id),
            } => {
                format!("This command was already accepted as workflow run `{run_id}`.")
            }
            DiscordInboundReceipt::Duplicate { run_id: None } => {
                "This command delivery was already handled.".to_string()
            }
            DiscordInboundReceipt::Rejected { reason } => match reason {
                DiscordRejectionReason::WrongCommand => "This command is not bound to VortiDeck.",
                DiscordRejectionReason::DirectMessageNotAllowed => {
                    "This VortiDeck command is not allowed in direct messages."
                }
                DiscordRejectionReason::WrongGuild => {
                    "This command is not authorized for this Discord server."
                }
                DiscordRejectionReason::WrongChannel => {
                    "This command is not authorized in this channel."
                }
                DiscordRejectionReason::UserNotAllowed => {
                    "Your Discord account is not authorized for this command."
                }
                DiscordRejectionReason::Expired => "This command delivery expired.",
                DiscordRejectionReason::FlowInactive => {
                    "The configured VortiDeck workflow is inactive."
                }
            }
            .to_string(),
            DiscordInboundReceipt::Failed { .. } => {
                "VortiDeck could not accept this command. Check the desktop logs.".to_string()
            }
        }
    }

    fn response_error(_error: impl std::fmt::Display) -> DiscordResponseError {
        DiscordResponseError {
            // Twilight request errors may include request context. Keep the
            // provider token out of application logs even on failure.
            message: "Discord HTTP response failed".to_string(),
            // Twilight's HTTP client owns Discord bucket/global rate-limit
            // handling. There is no provider delay to reinterpret here.
            retry_after: None,
        }
    }
}

#[async_trait::async_trait]
impl DiscordInboundTransport for TwilightDiscordTransport {
    async fn next_command(&self) -> Result<Option<DiscordInboundCommand>, String> {
        loop {
            let event = self
                .shard
                .lock()
                .await
                .next_event(EventTypeFlags::INTERACTION_CREATE)
                .await
                .ok_or_else(|| "Discord Gateway stream ended".to_string())?
                .map_err(|error| format!("Discord Gateway receive failed: {error}"))?;
            let Event::InteractionCreate(event) = event else {
                continue;
            };
            let interaction = event.0;
            let Some(InteractionData::ApplicationCommand(data)) = interaction.data.as_ref() else {
                continue;
            };
            let Some(user_id) = interaction.author_id() else {
                log::warn!(
                    "Ignoring Discord interaction {} without an author identity",
                    interaction.id
                );
                continue;
            };
            let Some(channel_id) = interaction.channel.as_ref().map(|channel| channel.id) else {
                log::warn!(
                    "Ignoring Discord interaction {} without a channel identity",
                    interaction.id
                );
                continue;
            };
            let interaction_id = interaction.id.to_string();
            self.responses.lock().await.insert(
                interaction_id.clone(),
                ResponseContext {
                    interaction_id: interaction.id,
                    token: interaction.token,
                    acknowledged: false,
                },
            );
            return Ok(Some(DiscordInboundCommand {
                interaction_id,
                command_name: data.name.clone(),
                guild_id: interaction.guild_id.map(|id| id.to_string()),
                channel_id: channel_id.to_string(),
                user_id: user_id.to_string(),
                occurred_at: Self::occurred_at(interaction.id.get()),
            }));
        }
    }

    async fn acknowledge(
        &self,
        command: &DiscordInboundCommand,
    ) -> Result<(), DiscordResponseError> {
        let mut responses = self.responses.lock().await;
        let context = responses.get_mut(&command.interaction_id).ok_or_else(|| {
            Self::response_error("Discord response context is no longer available")
        })?;
        if context.acknowledged {
            return Ok(());
        }
        let response = InteractionResponse {
            kind: InteractionResponseType::DeferredChannelMessageWithSource,
            data: Some(InteractionResponseData {
                flags: Some(MessageFlags::EPHEMERAL),
                ..InteractionResponseData::default()
            }),
        };
        self.http
            .interaction(self.application_id)
            .create_response(context.interaction_id, &context.token, &response)
            .await
            .map_err(Self::response_error)?;
        context.acknowledged = true;
        Ok(())
    }

    async fn respond(
        &self,
        command: &DiscordInboundCommand,
        receipt: &DiscordInboundReceipt,
    ) -> Result<(), DiscordResponseError> {
        let context = self
            .responses
            .lock()
            .await
            .get(&command.interaction_id)
            .cloned()
            .ok_or_else(|| {
                Self::response_error("Discord response context is no longer available")
            })?;
        let content = Self::receipt_message(receipt);
        let interaction = self.http.interaction(self.application_id);
        if context.acknowledged {
            interaction
                .update_response(&context.token)
                .content(Some(&content))
                .await
                .map_err(Self::response_error)?;
        } else {
            let response = InteractionResponse {
                kind: InteractionResponseType::ChannelMessageWithSource,
                data: Some(InteractionResponseData {
                    content: Some(content),
                    flags: Some(MessageFlags::EPHEMERAL),
                    ..InteractionResponseData::default()
                }),
            };
            interaction
                .create_response(context.interaction_id, &context.token, &response)
                .await
                .map_err(Self::response_error)?;
        }
        self.responses.lock().await.remove(&command.interaction_id);
        Ok(())
    }

    async fn close(&self) -> Result<(), String> {
        self.shard.lock().await.close(CloseFrame::NORMAL);
        self.responses.lock().await.clear();
        Ok(())
    }

    async fn discard_response(&self, command: &DiscordInboundCommand) {
        self.responses.lock().await.remove(&command.interaction_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_text_never_exposes_internal_failure_details() {
        let text = TwilightDiscordTransport::receipt_message(&DiscordInboundReceipt::Failed {
            message: "secret database path and token".to_string(),
        });
        assert!(!text.contains("secret"));
        assert!(text.contains("desktop logs"));
    }

    #[test]
    fn discord_snowflake_time_is_stable() {
        let timestamp = TwilightDiscordTransport::occurred_at(175_928_847_299_117_063);
        assert_eq!(timestamp.timestamp_millis(), 1_462_015_105_796);
    }
}
