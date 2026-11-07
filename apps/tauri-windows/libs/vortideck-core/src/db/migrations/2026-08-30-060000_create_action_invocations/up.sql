CREATE TABLE action_invocations (
    invocation_id TEXT PRIMARY KEY NOT NULL,
    parent_invocation_id TEXT,
    source TEXT NOT NULL,
    source_reference TEXT,
    workflow_run_id TEXT,
    node_id TEXT,
    resolved_action TEXT NOT NULL,
    integration_id TEXT,
    action_id TEXT,
    capability_version INTEGER,
    idempotency_key TEXT,
    resource_key TEXT,
    dispatch_status TEXT NOT NULL,
    receipt_semantics TEXT,
    request_id TEXT,
    provider_receipt_id TEXT,
    provider_outputs TEXT,
    normalized_status TEXT,
    error TEXT,
    recovery_decision TEXT,
    started_at TEXT NOT NULL,
    completed_at TEXT,
    execution_time_ms BIGINT NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX action_invocations_started_at_idx
    ON action_invocations(started_at DESC);
CREATE INDEX action_invocations_source_idx
    ON action_invocations(source, started_at DESC);
CREATE INDEX action_invocations_status_idx
    ON action_invocations(normalized_status, started_at DESC);
CREATE INDEX action_invocations_workflow_idx
    ON action_invocations(workflow_run_id, node_id, started_at DESC);
-- Multiple explicitly approved attempts may share a provider idempotency key.
-- Deduplication policy belongs to the invoking outbox/recovery coordinator;
-- this repository preserves every attempt for audit and reconciliation.
CREATE INDEX action_invocations_idempotency_idx
    ON action_invocations(source, idempotency_key);
