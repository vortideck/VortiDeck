//! Shared verification and replay-safe acceptance for direct and relay peers.

use crate::crypto::verify_hmac_signature;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use vortideck_core::core::StateManager;
use vortideck_core::db::{
    Database, NewRemoteCommandReceipt, RemoteCommandClaim, RemoteCommandReceiptRecord,
};

const REMOTE_PROTOCOL_VERSION: u64 = 2;
const MAX_REMOTE_ENVELOPE_LIFETIME_MS: u64 = 60_000;
const REMOTE_CLOCK_SKEW_MS: u64 = 5 * 60_000;
const MAX_REMOTE_ID_BYTES: usize = 128;

#[derive(Debug, Clone, PartialEq)]
pub enum RemoteCommandDisposition {
    /// Read-only/status traffic uses an atomic in-memory nonce claim and is not
    /// persisted as a consequential input.
    Transient,
    /// This process won the durable claim and may create one effect intent.
    Accepted(Value),
    /// A prior process/route already accepted this logical command. The stored
    /// receipt is returned and the command must not be dispatched again.
    Duplicate(Value),
}

impl RemoteCommandDisposition {
    pub fn should_dispatch(&self) -> bool {
        !matches!(self, Self::Duplicate(_))
    }

    pub fn receipt(&self) -> Option<&Value> {
        match self {
            Self::Accepted(receipt) | Self::Duplicate(receipt) => Some(receipt),
            Self::Transient => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VerifiedRemoteMessage {
    pub client_id: String,
    pub payload: Value,
    pub disposition: RemoteCommandDisposition,
}

#[derive(Debug, Clone)]
struct EnvelopeIdentity {
    message_id: String,
    nonce: String,
    sent_at: u64,
    expires_at: u64,
}

fn unix_time_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn validate_envelope_identity(json: &Value) -> Result<EnvelopeIdentity, String> {
    let protocol_version = json
        .get("protocol_version")
        .and_then(Value::as_u64)
        .ok_or("Missing protocol_version in remote command")?;
    if protocol_version != REMOTE_PROTOCOL_VERSION {
        return Err(format!(
            "Unsupported remote command protocol version: {protocol_version}"
        ));
    }
    let message_id = json
        .get("message_id")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty() && value.len() <= MAX_REMOTE_ID_BYTES)
        .ok_or("Missing or invalid message_id in remote command")?
        .to_string();
    let nonce = json
        .get("nonce")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty() && value.len() <= MAX_REMOTE_ID_BYTES)
        .ok_or("Missing or invalid nonce in remote command")?
        .to_string();
    let sent_at = json
        .get("sent_at")
        .and_then(Value::as_u64)
        .ok_or("Missing sent_at in remote command")?;
    let expires_at = json
        .get("expires_at")
        .and_then(Value::as_u64)
        .ok_or("Missing expires_at in remote command")?;
    let now = unix_time_millis();
    if expires_at <= sent_at
        || expires_at - sent_at > MAX_REMOTE_ENVELOPE_LIFETIME_MS
        || sent_at > now.saturating_add(REMOTE_CLOCK_SKEW_MS)
        || expires_at <= now
    {
        return Err("Expired or invalid remote command lifetime".to_string());
    }
    Ok(EnvelopeIdentity {
        message_id,
        nonce,
        sent_at,
        expires_at,
    })
}

fn is_consequential(payload: &Value) -> bool {
    !matches!(
        payload.get("action").and_then(Value::as_str),
        Some(
            "ping"
                | "subscribe_volume_updates"
                | "deck_widget_cache_inventory"
                | "list_workflow_approvals"
                | "get_current_deck"
                | "get_all_decks"
        )
    )
}

fn key_fingerprint(secret: &[u8; 32]) -> String {
    hex::encode(Sha256::digest(secret))
}

fn digest_json(value: &Value) -> Result<String, String> {
    let canonical = serde_json_canonicalizer::to_string(value)
        .map_err(|error| format!("Canonical JSON error: {error}"))?;
    Ok(hex::encode(Sha256::digest(canonical.as_bytes())))
}

fn acceptance_id(client_id: &str, key_fingerprint: &str, message_id: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(client_id.as_bytes());
    digest.update([0]);
    digest.update(key_fingerprint.as_bytes());
    digest.update([0]);
    digest.update(message_id.as_bytes());
    format!("remote:{}", hex::encode(digest.finalize()))
}

fn stored_receipt(record: &RemoteCommandReceiptRecord) -> Result<Value, String> {
    serde_json::from_str(&record.receipt_json)
        .map_err(|error| format!("Stored remote command receipt is invalid: {error}"))
}

async fn accept_verified_payload(
    client_id: &str,
    shared_secret: &[u8; 32],
    payload: &Value,
    identity: Option<EnvelopeIdentity>,
    signed_nonce: &str,
    transport: &str,
    state_manager: &Arc<StateManager>,
    database: &Database,
) -> Result<RemoteCommandDisposition, String> {
    if !is_consequential(payload) {
        if !state_manager
            .claim_transient_nonce(signed_nonce.to_string(), client_id.to_string())
            .await
        {
            return Err("Replay detected: transient nonce was already used".to_string());
        }
        return Ok(RemoteCommandDisposition::Transient);
    }

    let identity = identity
        .ok_or("Consequential direct commands require a versioned, expiring message identity")?;

    let key_fingerprint = key_fingerprint(shared_secret);
    let acceptance_id = acceptance_id(client_id, &key_fingerprint, &identity.message_id);
    let payload_digest = digest_json(payload)?;
    let now = chrono::Utc::now().to_rfc3339();
    let receipt = json!({
        "action": "remote_command_receipt",
        "acceptanceId": acceptance_id,
        "messageId": identity.message_id,
        "status": "accepted",
        "outcome": "unresolved",
    });
    let receipt_json = serde_json::to_string(&receipt)
        .map_err(|error| format!("Could not serialize remote command receipt: {error}"))?;
    let sent_at = i64::try_from(identity.sent_at)
        .map_err(|_| "Remote command sent_at is outside the supported range".to_string())?;
    let expires_at = i64::try_from(identity.expires_at)
        .map_err(|_| "Remote command expires_at is outside the supported range".to_string())?;

    match database
        .claim_remote_command(NewRemoteCommandReceipt {
            acceptance_id: &acceptance_id,
            peer_client_id: client_id,
            key_fingerprint: &key_fingerprint,
            message_id: &identity.message_id,
            nonce: &identity.nonce,
            payload_digest: &payload_digest,
            transport,
            status: "accepted",
            sent_at,
            expires_at,
            receipt_json: &receipt_json,
            accepted_at: &now,
            updated_at: &now,
        })
        .await?
    {
        RemoteCommandClaim::Accepted(record) => {
            Ok(RemoteCommandDisposition::Accepted(stored_receipt(&record)?))
        }
        RemoteCommandClaim::Duplicate(record) => Ok(RemoteCommandDisposition::Duplicate(
            stored_receipt(&record)?,
        )),
    }
}

fn direct_payload(mut json: Value) -> Result<Value, String> {
    let object = json.as_object_mut().ok_or("Message is not a JSON object")?;
    for field in [
        "signature",
        "nonce",
        "protocol_version",
        "message_id",
        "sent_at",
        "expires_at",
        "client_id",
    ] {
        object.remove(field);
    }
    Ok(json)
}

/// Verify one direct peer message. Consequential commands require the same
/// versioned, expiring identity fields as relay commands.
pub async fn verify_e2ee_message(
    json: Value,
    client_id: &str,
    state_manager: &Arc<StateManager>,
    database: &Database,
) -> Result<VerifiedRemoteMessage, String> {
    let shared_secret = state_manager
        .get_shared_secret(client_id)
        .await
        .ok_or_else(|| format!("No shared secret for E2EE client: {client_id}"))?;
    let signature = json
        .get("signature")
        .and_then(Value::as_str)
        .ok_or("Missing signature in E2EE message")?;
    let signed_nonce = json
        .get("nonce")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty() && value.len() <= MAX_REMOTE_ID_BYTES)
        .ok_or("Missing or invalid nonce in E2EE message")?
        .to_string();
    let has_versioned_identity = ["protocol_version", "message_id", "sent_at", "expires_at"]
        .iter()
        .any(|field| json.get(field).is_some());
    let identity = has_versioned_identity
        .then(|| validate_envelope_identity(&json))
        .transpose()?;

    let mut unsigned = json.clone();
    unsigned
        .as_object_mut()
        .ok_or("Message is not a JSON object")?
        .remove("signature");
    let canonical = serde_json_canonicalizer::to_string(&unsigned)
        .map_err(|error| format!("Canonical JSON error: {error}"))?;
    if !verify_hmac_signature(&shared_secret, &canonical, signature) {
        return Err(format!("Invalid HMAC signature from client: {client_id}"));
    }

    let payload = direct_payload(json)?;
    let disposition = accept_verified_payload(
        client_id,
        &shared_secret,
        &payload,
        identity,
        &signed_nonce,
        "direct",
        state_manager,
        database,
    )
    .await?;
    Ok(VerifiedRemoteMessage {
        client_id: client_id.to_string(),
        payload,
        disposition,
    })
}

/// Verify one message delivered through the account relay. Relay routing
/// metadata is not trusted; the HMAC-bound client and payload identity are.
pub async fn verify_relay_message(
    json: &Value,
    state_manager: &Arc<StateManager>,
    database: &Database,
) -> Result<VerifiedRemoteMessage, String> {
    let client_id = json
        .get("client_id")
        .and_then(Value::as_str)
        .ok_or("Missing client_id in relay message")?
        .to_string();
    let payload = json
        .get("payload")
        .filter(|value| value.is_object())
        .cloned()
        .ok_or("Payload is not a JSON object")?;
    let signature = json
        .get("signature")
        .and_then(Value::as_str)
        .ok_or("Missing signature in relay message")?;
    let identity = validate_envelope_identity(json)?;
    let signed_nonce = identity.nonce.clone();
    let shared_secret = state_manager
        .get_shared_secret(&client_id)
        .await
        .ok_or_else(|| format!("No shared secret for client: {client_id}"))?;

    let unsigned = json!({
        "protocol_version": REMOTE_PROTOCOL_VERSION,
        "message_id": identity.message_id,
        "sent_at": identity.sent_at,
        "expires_at": identity.expires_at,
        "payload": payload,
        "nonce": identity.nonce,
    });
    let canonical = serde_json_canonicalizer::to_string(&unsigned)
        .map_err(|error| format!("Canonical JSON error: {error}"))?;
    if !verify_hmac_signature(&shared_secret, &canonical, signature) {
        return Err(format!(
            "Invalid HMAC signature from relay client: {client_id}"
        ));
    }

    let disposition = accept_verified_payload(
        &client_id,
        &shared_secret,
        &payload,
        Some(identity),
        &signed_nonce,
        "relay",
        state_manager,
        database,
    )
    .await?;
    Ok(VerifiedRemoteMessage {
        client_id,
        payload,
        disposition,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::create_hmac_signature;
    use diesel::r2d2::{ConnectionManager, Pool};
    use diesel::SqliteConnection;

    fn test_database(migrate: bool) -> (tempfile::TempDir, Database) {
        let directory = tempfile::tempdir().expect("temporary database directory");
        let path = directory.path().join("remote-commands.db");
        let pool = if migrate {
            vortideck_core::db::db::initialize_database_at(&path)
                .expect("remote command migrations");
            vortideck_core::db::db::establish_pool_at(&path).expect("migrated database pool")
        } else {
            std::fs::File::create(&path).expect("empty database file");
            let manager = ConnectionManager::<SqliteConnection>::new(path.to_string_lossy());
            Pool::builder()
                .max_size(8)
                .build(manager)
                .expect("unmigrated database pool")
        };
        (directory, Database::new(&pool))
    }

    async fn state_with_secret(client_id: &str, secret: [u8; 32]) -> Arc<StateManager> {
        let state = Arc::new(StateManager::new());
        state
            .store_shared_secret(client_id.to_string(), secret)
            .await;
        state
    }

    fn signed_direct(
        secret: &[u8; 32],
        message_id: &str,
        nonce: &str,
        payload: Value,
        sent_at: u64,
    ) -> Value {
        let mut object = payload.as_object().expect("object payload").clone();
        object.insert("protocol_version".into(), json!(REMOTE_PROTOCOL_VERSION));
        object.insert("message_id".into(), json!(message_id));
        object.insert("sent_at".into(), json!(sent_at));
        object.insert("expires_at".into(), json!(sent_at + 30_000));
        object.insert("nonce".into(), json!(nonce));
        let mut envelope = Value::Object(object);
        let canonical = serde_json_canonicalizer::to_string(&envelope).unwrap();
        envelope["signature"] = json!(create_hmac_signature(secret, &canonical));
        envelope
    }

    fn signed_legacy_direct(secret: &[u8; 32], nonce: &str, payload: Value) -> Value {
        let mut object = payload.as_object().expect("object payload").clone();
        object.insert("nonce".into(), json!(nonce));
        let mut envelope = Value::Object(object);
        let canonical = serde_json_canonicalizer::to_string(&envelope).unwrap();
        envelope["signature"] = json!(create_hmac_signature(secret, &canonical));
        envelope
    }

    fn signed_relay(
        client_id: &str,
        secret: &[u8; 32],
        message_id: &str,
        nonce: &str,
        payload: Value,
        sent_at: u64,
    ) -> Value {
        let unsigned = json!({
            "protocol_version": REMOTE_PROTOCOL_VERSION,
            "message_id": message_id,
            "sent_at": sent_at,
            "expires_at": sent_at + 30_000,
            "payload": payload,
            "nonce": nonce,
        });
        let canonical = serde_json_canonicalizer::to_string(&unsigned).unwrap();
        json!({
            "client_id": client_id,
            "action": "relay_message",
            "protocol_version": REMOTE_PROTOCOL_VERSION,
            "message_id": message_id,
            "sent_at": sent_at,
            "expires_at": sent_at + 30_000,
            "payload": unsigned["payload"],
            "nonce": nonce,
            "signature": create_hmac_signature(secret, &canonical),
        })
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_direct_copies_create_one_durable_effect_intent() {
        let (_directory, database) = test_database(true);
        let database = Arc::new(database);
        let client_id = "mobile:concurrent";
        let secret = [7u8; 32];
        let state = state_with_secret(client_id, secret).await;
        let envelope = signed_direct(
            &secret,
            "command-concurrent",
            "nonce-concurrent",
            json!({ "action": "command", "command": "effect" }),
            unix_time_millis(),
        );

        let mut tasks = Vec::new();
        for _ in 0..16 {
            let state = state.clone();
            let database = database.clone();
            let envelope = envelope.clone();
            tasks.push(tokio::spawn(async move {
                verify_e2ee_message(envelope, client_id, &state, &database)
                    .await
                    .expect("valid concurrent command")
                    .disposition
            }));
        }
        let mut accepted = 0;
        let mut duplicates = 0;
        for task in tasks {
            match task.await.unwrap() {
                RemoteCommandDisposition::Accepted(_) => accepted += 1,
                RemoteCommandDisposition::Duplicate(_) => duplicates += 1,
                RemoteCommandDisposition::Transient => panic!("command must be durable"),
            }
        }
        assert_eq!(accepted, 1);
        assert_eq!(duplicates, 15);
    }

    #[tokio::test]
    async fn duplicate_survives_process_state_reconstruction_and_cross_route_retry() {
        let (directory, database) = test_database(true);
        let client_id = "mobile:restart";
        let secret = [9u8; 32];
        let payload = json!({ "action": "command", "command": "effect" });
        let sent_at = unix_time_millis();
        let first_state = state_with_secret(client_id, secret).await;
        let first = verify_e2ee_message(
            signed_direct(
                &secret,
                "command-restart",
                "nonce-direct",
                payload.clone(),
                sent_at,
            ),
            client_id,
            &first_state,
            &database,
        )
        .await
        .unwrap();
        assert!(matches!(
            first.disposition,
            RemoteCommandDisposition::Accepted(_)
        ));

        drop(database);
        let reopened_pool =
            vortideck_core::db::db::establish_pool_at(&directory.path().join("remote-commands.db"))
                .expect("reopened database pool");
        let reopened_database = Database::new(&reopened_pool);
        let reconstructed_state = state_with_secret(client_id, secret).await;
        let retried = verify_relay_message(
            &signed_relay(
                client_id,
                &secret,
                "command-restart",
                "nonce-relay",
                payload,
                sent_at,
            ),
            &reconstructed_state,
            &reopened_database,
        )
        .await
        .unwrap();
        let RemoteCommandDisposition::Duplicate(receipt) = retried.disposition else {
            panic!("cross-route retry must return the stored receipt");
        };
        assert_eq!(receipt["status"], "accepted");
        assert_eq!(receipt["outcome"], "unresolved");
    }

    #[tokio::test]
    async fn expired_bad_signature_and_storage_failure_fail_closed() {
        let (_directory, database) = test_database(true);
        let client_id = "mobile:failure";
        let secret = [11u8; 32];
        let state = state_with_secret(client_id, secret).await;
        let payload = json!({ "action": "command", "command": "effect" });

        let expired = signed_direct(
            &secret,
            "expired",
            "expired-nonce",
            payload.clone(),
            unix_time_millis().saturating_sub(31_000),
        );
        assert!(verify_e2ee_message(expired, client_id, &state, &database)
            .await
            .unwrap_err()
            .contains("Expired"));

        let mut tampered = signed_direct(
            &secret,
            "tampered",
            "tampered-nonce",
            payload.clone(),
            unix_time_millis(),
        );
        tampered["command"] = json!("different-effect");
        assert!(verify_e2ee_message(tampered, client_id, &state, &database)
            .await
            .unwrap_err()
            .contains("Invalid HMAC"));

        let (_broken_directory, broken_database) = test_database(false);
        let storage_failure = signed_direct(
            &secret,
            "storage-failure",
            "storage-failure-nonce",
            payload,
            unix_time_millis(),
        );
        assert!(
            verify_e2ee_message(storage_failure, client_id, &state, &broken_database)
                .await
                .unwrap_err()
                .contains("persist remote command acceptance")
        );
    }

    #[tokio::test]
    async fn key_rotation_invalidates_old_signatures_and_starts_a_new_generation() {
        let (_directory, database) = test_database(true);
        let client_id = "mobile:rotation";
        let old_secret = [13u8; 32];
        let new_secret = [17u8; 32];
        let state = state_with_secret(client_id, old_secret).await;
        let payload = json!({ "action": "workflow_approval_decision", "wakeId": "wake" });
        let old_envelope = signed_direct(
            &old_secret,
            "rotation-command",
            "rotation-old",
            payload.clone(),
            unix_time_millis(),
        );
        assert!(matches!(
            verify_e2ee_message(old_envelope.clone(), client_id, &state, &database)
                .await
                .unwrap()
                .disposition,
            RemoteCommandDisposition::Accepted(_)
        ));

        state
            .store_shared_secret(client_id.to_string(), new_secret)
            .await;
        assert!(
            verify_e2ee_message(old_envelope, client_id, &state, &database)
                .await
                .unwrap_err()
                .contains("Invalid HMAC")
        );
        let new_envelope = signed_direct(
            &new_secret,
            "rotation-command",
            "rotation-new",
            payload,
            unix_time_millis(),
        );
        assert!(matches!(
            verify_e2ee_message(new_envelope, client_id, &state, &database)
                .await
                .unwrap()
                .disposition,
            RemoteCommandDisposition::Accepted(_)
        ));
    }

    #[tokio::test]
    async fn read_only_messages_use_atomic_transient_nonce_claims() {
        let (_directory, database) = test_database(true);
        let client_id = "mobile:read-only";
        let secret = [19u8; 32];
        let state = state_with_secret(client_id, secret).await;
        let envelope = signed_direct(
            &secret,
            "read-only",
            "read-only-nonce",
            json!({ "action": "get_current_deck" }),
            unix_time_millis(),
        );
        assert_eq!(
            verify_e2ee_message(envelope.clone(), client_id, &state, &database)
                .await
                .unwrap()
                .disposition,
            RemoteCommandDisposition::Transient
        );
        assert!(verify_e2ee_message(envelope, client_id, &state, &database)
            .await
            .unwrap_err()
            .contains("transient nonce"));
    }

    #[tokio::test]
    async fn legacy_direct_envelopes_are_read_only_only() {
        let (_directory, database) = test_database(true);
        let client_id = "mobile:legacy";
        let secret = [23u8; 32];
        let state = state_with_secret(client_id, secret).await;

        let query = signed_legacy_direct(
            &secret,
            "legacy-query-nonce",
            json!({ "action": "get_current_deck" }),
        );
        assert_eq!(
            verify_e2ee_message(query, client_id, &state, &database)
                .await
                .unwrap()
                .disposition,
            RemoteCommandDisposition::Transient
        );

        let command = signed_legacy_direct(
            &secret,
            "legacy-command-nonce",
            json!({ "action": "command", "command": "effect" }),
        );
        assert!(verify_e2ee_message(command, client_id, &state, &database)
            .await
            .unwrap_err()
            .contains("require a versioned"));
    }

    #[tokio::test]
    async fn conflicting_message_or_nonce_reuse_fails_closed() {
        let (_directory, database) = test_database(true);
        let client_id = "mobile:conflict";
        let secret = [29u8; 32];
        let state = state_with_secret(client_id, secret).await;
        let sent_at = unix_time_millis();

        let original = signed_direct(
            &secret,
            "conflict-message",
            "conflict-nonce",
            json!({ "action": "command", "command": "first" }),
            sent_at,
        );
        assert!(matches!(
            verify_e2ee_message(original, client_id, &state, &database)
                .await
                .unwrap()
                .disposition,
            RemoteCommandDisposition::Accepted(_)
        ));

        let changed_payload = signed_direct(
            &secret,
            "conflict-message",
            "fresh-nonce",
            json!({ "action": "command", "command": "second" }),
            sent_at,
        );
        assert!(
            verify_e2ee_message(changed_payload, client_id, &state, &database)
                .await
                .unwrap_err()
                .contains("reused with different signed content")
        );

        let reused_nonce = signed_direct(
            &secret,
            "fresh-message",
            "conflict-nonce",
            json!({ "action": "command", "command": "first" }),
            sent_at,
        );
        assert!(
            verify_e2ee_message(reused_nonce, client_id, &state, &database)
                .await
                .unwrap_err()
                .contains("reused with different signed content")
        );
    }
}
