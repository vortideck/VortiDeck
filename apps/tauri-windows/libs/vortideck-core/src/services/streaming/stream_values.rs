//! Read-only projections of accepted provider events. These are observed
//! cumulative values, not a substitute for a provider's lifetime totals.
use crate::events::EventEnvelope;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ObservedStreamValue {
    pub platform: String,
    pub account_id: u64,
    pub metric: String,
    pub total: u64,
    pub first_observed_at: String,
    pub last_observed_at: String,
}

pub fn observed_increment(envelope: &EventEnvelope) -> Option<(&'static str, u64)> {
    envelope.account_id?;
    // Provider time must be present; ingress's display-time fallback cannot
    // establish freshness for a cumulative value.
    let provider_time = envelope
        .payload
        .get("occurredAt")
        .and_then(serde_json::Value::as_str)?;
    chrono::DateTime::parse_from_rfc3339(provider_time).ok()?;
    if envelope
        .idempotency_key
        .as_ref()
        .is_none_or(|id| id.trim().is_empty())
    {
        return None;
    }
    let age = envelope
        .received_at
        .signed_duration_since(envelope.occurred_at);
    if age > chrono::Duration::minutes(5) || age < -chrono::Duration::seconds(120) {
        return None;
    }
    match envelope.event_type.as_str() {
        "stream.follow" => Some(("followers", 1)),
        "stream.subscription" => Some(("subscribers", 1)),
        "stream.bits" => envelope
            .payload
            .get("amount")
            .and_then(serde_json::Value::as_u64)
            .filter(|amount| *amount > 0 && *amount <= u32::MAX as u64)
            .map(|amount| ("bits", amount)),
        _ => None,
    }
}
