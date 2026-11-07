CREATE TABLE remote_command_receipts (
    acceptance_id TEXT PRIMARY KEY NOT NULL,
    peer_client_id TEXT NOT NULL,
    key_fingerprint TEXT NOT NULL,
    message_id TEXT NOT NULL,
    nonce TEXT NOT NULL,
    payload_digest TEXT NOT NULL,
    transport TEXT NOT NULL,
    status TEXT NOT NULL,
    sent_at BIGINT NOT NULL,
    expires_at BIGINT NOT NULL,
    receipt_json TEXT NOT NULL,
    accepted_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX remote_command_receipts_message_identity_idx
    ON remote_command_receipts(peer_client_id, key_fingerprint, message_id);
CREATE UNIQUE INDEX remote_command_receipts_nonce_identity_idx
    ON remote_command_receipts(peer_client_id, key_fingerprint, nonce);
CREATE INDEX remote_command_receipts_expiry_idx
    ON remote_command_receipts(expires_at);
