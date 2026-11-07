use chrono::{DateTime, Duration as ChronoDuration, Utc};
use hmac::{Hmac, Mac};
use serde::Serialize;
use sha2::Sha256;
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::Duration;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

/// An editor-held, webhook-scoped capability for the real-effect test route.
/// The raw token is returned once and is never persisted by this authority.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebhookTestCapability {
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub expires_in_seconds: u64,
}

#[derive(Debug, Clone)]
struct SessionRecord {
    token_mac: [u8; 32],
    expires_at: DateTime<Utc>,
}

/// Process-local authority for editor test sessions.
///
/// Durable webhook flags remain useful UI/lifecycle metadata, but never grant
/// access on their own. Restarting the host therefore revokes every test
/// capability even when an old database flag has not yet been reconciled.
#[derive(Debug)]
pub struct WebhookTestSessionAuthority {
    signing_key: [u8; 32],
    sessions: RwLock<HashMap<String, SessionRecord>>,
}

impl Default for WebhookTestSessionAuthority {
    fn default() -> Self {
        Self::new()
    }
}

impl WebhookTestSessionAuthority {
    pub fn new() -> Self {
        let first = Uuid::new_v4();
        let second = Uuid::new_v4();
        let mut signing_key = [0_u8; 32];
        signing_key[..16].copy_from_slice(first.as_bytes());
        signing_key[16..].copy_from_slice(second.as_bytes());

        Self {
            signing_key,
            sessions: RwLock::new(HashMap::new()),
        }
    }

    pub fn issue(&self, webhook_id: &str, ttl: Duration) -> WebhookTestCapability {
        let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
        let ttl = ttl.max(Duration::from_millis(1));
        let expires_in_seconds = ttl.as_secs().max(1);
        let expires_at = Utc::now() + ChronoDuration::from_std(ttl).unwrap_or(ChronoDuration::MAX);
        let token_mac = self.token_mac(&token);

        self.sessions
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(
                webhook_id.to_string(),
                SessionRecord {
                    token_mac,
                    expires_at,
                },
            );

        WebhookTestCapability {
            token,
            expires_at,
            expires_in_seconds,
        }
    }

    pub fn authorize(&self, webhook_id: &str, token: Option<&str>) -> bool {
        let Some(token) = token.filter(|value| !value.is_empty()) else {
            return false;
        };

        let mut sessions = self
            .sessions
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        sessions.retain(|_, session| session.expires_at > Utc::now());
        let Some(session) = sessions.get(webhook_id) else {
            return false;
        };

        let Ok(mut verifier) = HmacSha256::new_from_slice(&self.signing_key) else {
            return false;
        };
        verifier.update(token.as_bytes());
        verifier.verify_slice(&session.token_mac).is_ok()
    }

    pub fn revoke(&self, webhook_id: &str) {
        self.sessions
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(webhook_id);
    }

    fn token_mac(&self, token: &str) -> [u8; 32] {
        let mut mac = HmacSha256::new_from_slice(&self.signing_key)
            .expect("SHA-256 HMAC accepts a 32-byte key");
        mac.update(token.as_bytes());
        mac.finalize().into_bytes().into()
    }
}

#[cfg(test)]
mod tests {
    use super::WebhookTestSessionAuthority;
    use std::time::Duration;

    #[test]
    fn capability_is_scoped_rotated_and_revocable() {
        let authority = WebhookTestSessionAuthority::new();
        let first = authority.issue("webhook-a", Duration::from_secs(120));
        assert!(authority.authorize("webhook-a", Some(&first.token)));
        assert!(!authority.authorize("webhook-b", Some(&first.token)));
        assert!(!authority.authorize("webhook-a", Some("wrong")));

        let second = authority.issue("webhook-a", Duration::from_secs(120));
        assert!(!authority.authorize("webhook-a", Some(&first.token)));
        assert!(authority.authorize("webhook-a", Some(&second.token)));

        authority.revoke("webhook-a");
        assert!(!authority.authorize("webhook-a", Some(&second.token)));
    }

    #[test]
    fn expired_capability_is_rejected() {
        let authority = WebhookTestSessionAuthority::new();
        let capability = authority.issue("webhook-a", Duration::from_nanos(1));
        std::thread::sleep(Duration::from_millis(10));
        assert!(!authority.authorize("webhook-a", Some(&capability.token)));
    }
}
