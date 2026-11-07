//! SQLite-backed persistence for workflow runtime state.
//!
//! Flow definitions stay in the main Diesel database; execution state,
//! trigger routes, deduplication claims, and accepted event envelopes use the
//! runtime SQLite database.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::sync::Arc;
use tokio::sync::Semaphore;

use super::executor::{
    ActiveTriggerDefinition, ApprovalResolution, ApprovalResolveOutcome, DatabaseStats,
    ExecutionHistoryEntry, FlowDefinition, FlowExecutionState, FlowStatus, FlowStorage,
    FlowSummary, WorkflowResourceLease, WorkflowWake, WorkflowWakeKind, WorkflowWakeStatus,
};
use crate::db::database::DbPool;

#[derive(Clone)]
pub struct HybridFlowStorage {
    backend: Arc<HybridFlowStorageBackend>,
    work_permits: Arc<Semaphore>,
    admitted_work_permits: Arc<Semaphore>,
}

#[derive(Clone)]
struct HybridFlowStorageBackend {
    db_pool: DbPool,
    execution_db_path: String,
}

impl HybridFlowStorage {
    const DEFAULT_MAX_BLOCKING_OPERATIONS: usize = 4;
    const DEFAULT_MAX_ADMITTED_OPERATIONS: usize = 64;

    pub fn new(db_pool: DbPool, execution_db_path: &str) -> Result<Self, String> {
        Self::new_with_work_limits(
            db_pool,
            execution_db_path,
            Self::DEFAULT_MAX_BLOCKING_OPERATIONS,
            Self::DEFAULT_MAX_ADMITTED_OPERATIONS,
        )
    }

    fn new_with_work_limits(
        db_pool: DbPool,
        execution_db_path: &str,
        max_blocking_operations: usize,
        max_admitted_operations: usize,
    ) -> Result<Self, String> {
        let backend = HybridFlowStorageBackend {
            db_pool,
            execution_db_path: execution_db_path.to_string(),
        };
        backend.initialize_schema()?;
        Ok(Self {
            backend: Arc::new(backend),
            work_permits: Arc::new(Semaphore::new(max_blocking_operations.max(1))),
            admitted_work_permits: Arc::new(Semaphore::new(max_admitted_operations.max(1))),
        })
    }

    async fn run_database_work<T, Operation, OperationFuture>(
        &self,
        operation_name: &'static str,
        operation: Operation,
    ) -> Result<T, String>
    where
        T: Send + 'static,
        Operation: FnOnce(Arc<HybridFlowStorageBackend>) -> OperationFuture + Send + 'static,
        OperationFuture: Future<Output = Result<T, String>> + Send + 'static,
    {
        let admission = self
            .admitted_work_permits
            .clone()
            .try_acquire_owned()
            .map_err(|_| {
                format!("Database work queue is full; rejected {operation_name} before starting")
            })?;
        let worker_permit = self
            .work_permits
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| format!("Database work queue closed before {operation_name}"))?;
        let backend = self.backend.clone();
        let runtime = tokio::runtime::Handle::current();

        tokio::task::spawn_blocking(move || {
            // The permit deliberately lives in the worker. Dropping/cancelling
            // the async caller detaches the JoinHandle but cannot release
            // capacity while the synchronous transaction is still running.
            let _admission = admission;
            let _worker_permit = worker_permit;
            runtime.block_on(operation(backend))
        })
        .await
        .map_err(|error| format!("Database worker failed during {operation_name}: {error}"))?
    }

    fn schedule_history_maintenance(&self) {
        static EXECUTION_COUNTER: std::sync::atomic::AtomicUsize =
            std::sync::atomic::AtomicUsize::new(0);
        let count = EXECUTION_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if count % 10 != 0 {
            return;
        }
        let storage = self.clone();
        tokio::spawn(async move {
            if let Err(error) = storage.cleanup_execution_history(30, 50).await {
                log::error!("Automatic workflow-history cleanup failed: {}", error);
            }
            if count % 100 == 0
                && let Err(error) = storage.vacuum_database().await
            {
                log::error!("Automatic workflow database vacuum failed: {}", error);
            }
        });
    }
}

impl HybridFlowStorageBackend {
    /// Create the runtime schema once at storage construction. Runtime methods
    /// can then focus on their query/transaction instead of repeating lazy DDL.
    fn initialize_schema(&self) -> Result<(), String> {
        let mut conn = self.connection()?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA foreign_keys = ON;
            CREATE TABLE IF NOT EXISTS archived_flows (
                id TEXT PRIMARY KEY,
                original_flow_id TEXT NOT NULL,
                name TEXT NOT NULL,
                description TEXT,
                data TEXT NOT NULL,
                archived_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                original_created_at DATETIME,
                original_updated_at DATETIME
            );
            CREATE TABLE IF NOT EXISTS execution_states (
                run_id TEXT PRIMARY KEY,
                state TEXT NOT NULL,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );
            CREATE TABLE IF NOT EXISTS execution_history (
                id TEXT PRIMARY KEY,
                run_id TEXT,
                flow_id TEXT NOT NULL,
                flow_name TEXT NOT NULL,
                status TEXT NOT NULL,
                started_at DATETIME NOT NULL,
                completed_at DATETIME,
                duration_ms INTEGER,
                reachable_node_count INTEGER NOT NULL,
                completed_nodes INTEGER NOT NULL,
                failed_nodes INTEGER NOT NULL,
                error_message TEXT,
                final_results TEXT,
                node_execution_details TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );
            CREATE TABLE IF NOT EXISTS trigger_states (
                flow_id TEXT NOT NULL,
                node_id TEXT NOT NULL,
                key TEXT NOT NULL,
                value TEXT NOT NULL,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY (flow_id, node_id, key)
            );
            CREATE TABLE IF NOT EXISTS trigger_deduplication (
                dedup_key TEXT PRIMARY KEY,
                event_id TEXT,
                occurred_at TEXT NOT NULL,
                claimed_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );
            CREATE TABLE IF NOT EXISTS active_trigger_registry (
                flow_id TEXT NOT NULL,
                node_id TEXT NOT NULL,
                definition TEXT NOT NULL,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY (flow_id, node_id)
            );
            CREATE TABLE IF NOT EXISTS workflow_event_inbox (
                event_id TEXT PRIMARY KEY,
                event_type TEXT NOT NULL,
                source TEXT NOT NULL,
                envelope TEXT NOT NULL,
                received_at TEXT NOT NULL,
                recorded_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );
            CREATE INDEX IF NOT EXISTS idx_workflow_event_inbox_received_at
                ON workflow_event_inbox(received_at);
            CREATE TABLE IF NOT EXISTS observed_stream_values (
                platform TEXT NOT NULL, account_id TEXT NOT NULL, metric TEXT NOT NULL,
                total INTEGER NOT NULL CHECK(total >= 0),
                first_observed_at TEXT NOT NULL, last_observed_at TEXT NOT NULL,
                PRIMARY KEY(platform, account_id, metric)
            );
            CREATE TABLE IF NOT EXISTS overlay_command_deliveries (
                command_id TEXT PRIMARY KEY,
                status TEXT NOT NULL,
                expires_at TEXT,
                delivery TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_overlay_command_deliveries_pending
                ON overlay_command_deliveries(status, expires_at);
            CREATE TABLE IF NOT EXISTS workflow_side_effect_outbox (
                idempotency_key TEXT PRIMARY KEY,
                status TEXT NOT NULL,
                entry TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS workflow_dead_letters (
                id TEXT PRIMARY KEY,
                event_id TEXT NOT NULL,
                resolved_at TEXT,
                record TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_workflow_dead_letters_unresolved
                ON workflow_dead_letters(resolved_at, updated_at);
            CREATE INDEX IF NOT EXISTS idx_trigger_deduplication_claimed_at
                ON trigger_deduplication(claimed_at);
            CREATE TABLE IF NOT EXISTS workflow_wakes (
                id TEXT PRIMARY KEY,
                run_id TEXT NOT NULL,
                node_id TEXT NOT NULL,
                kind TEXT NOT NULL,
                status TEXT NOT NULL,
                due_at TEXT,
                wake TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_workflow_wakes_open
                ON workflow_wakes(status, due_at, run_id);
            CREATE TABLE IF NOT EXISTS workflow_resource_leases (
                resource_key TEXT PRIMARY KEY,
                owner_run_id TEXT NOT NULL,
                owner_node_id TEXT NOT NULL,
                lease_token TEXT NOT NULL UNIQUE,
                expires_at TEXT NOT NULL,
                lease TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_workflow_resource_leases_expiry
                ON workflow_resource_leases(expires_at);
            CREATE TABLE IF NOT EXISTS workflow_runtime_quarantine (
                id TEXT PRIMARY KEY,
                record_type TEXT NOT NULL,
                record_key TEXT NOT NULL,
                reason TEXT NOT NULL,
                payload TEXT NOT NULL,
                quarantined_at TEXT NOT NULL
            );",
        )
        .map_err(|e| format!("Failed to initialize workflow runtime schema: {}", e))?;
        Self::ensure_execution_history_run_id(&conn)?;
        Self::ensure_execution_state_run_id(&conn)?;
        Self::migrate_singular_recovery_checkpoints(&mut conn)?;
        let reconciled = Self::migrate_terminal_execution_states(&mut conn)?;
        if reconciled > 0 {
            log::info!(
                "Removed {} stale workflow snapshot(s) already represented by terminal history",
                reconciled
            );
        }
        Ok(())
    }

    fn ensure_execution_history_run_id(conn: &Connection) -> Result<(), String> {
        let has_run_id = {
            let mut statement = conn
                .prepare("PRAGMA table_info(execution_history)")
                .map_err(|error| {
                    format!("Failed to inspect execution history schema: {}", error)
                })?;
            let columns = statement
                .query_map([], |row| row.get::<_, String>(1))
                .map_err(|error| format!("Failed to query execution history schema: {}", error))?;
            columns
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| format!("Failed to read execution history schema: {}", error))?
                .iter()
                .any(|column| column == "run_id")
        };
        if !has_run_id {
            conn.execute("ALTER TABLE execution_history ADD COLUMN run_id TEXT", [])
                .map_err(|error| format!("Failed to add execution history run ID: {}", error))?;
        }
        conn.execute(
            "UPDATE execution_history SET run_id = flow_id WHERE run_id IS NULL OR run_id = ''",
            [],
        )
        .map_err(|error| format!("Failed to backfill execution history run IDs: {}", error))?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_execution_history_flow_started
             ON execution_history(flow_id, started_at)",
            [],
        )
        .map_err(|error| format!("Failed to index execution history identity: {}", error))?;
        Ok(())
    }

    fn ensure_execution_state_run_id(conn: &Connection) -> Result<(), String> {
        let columns = {
            let mut statement = conn
                .prepare("PRAGMA table_info(execution_states)")
                .map_err(|error| format!("Failed to inspect execution state schema: {}", error))?;
            let columns = statement
                .query_map([], |row| row.get::<_, String>(1))
                .map_err(|error| format!("Failed to query execution state schema: {}", error))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| format!("Failed to read execution state schema: {}", error))?;
            columns
        };
        if columns.iter().any(|column| column == "flow_id")
            && !columns.iter().any(|column| column == "run_id")
        {
            conn.execute(
                "ALTER TABLE execution_states RENAME COLUMN flow_id TO run_id",
                [],
            )
            .map_err(|error| format!("Failed to migrate execution state run IDs: {}", error))?;
        }
        Ok(())
    }

    /// Remove only snapshots with an exact terminal-history match. This is a
    /// conservative, idempotent migration for versions that wrote a terminal
    /// history row and then recreated the execution snapshot afterwards.
    fn migrate_terminal_execution_states(conn: &mut Connection) -> Result<usize, String> {
        let terminal_runs = {
            let mut statement = conn
                .prepare(
                    "SELECT run_id, flow_id, started_at FROM execution_history
                     WHERE status IN ('completed', 'completed_with_errors', 'failed', 'cancelled')",
                )
                .map_err(|error| {
                    format!("Failed to prepare terminal history migration: {}", error)
                })?;
            let rows = statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?.unwrap_or_default(),
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })
                .map_err(|error| {
                    format!("Failed to query terminal history migration: {}", error)
                })?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|error| format!("Failed to read terminal history migration: {}", error))?
        };
        let snapshots = {
            let mut statement = conn
                .prepare("SELECT run_id, state FROM execution_states")
                .map_err(|error| {
                    format!("Failed to prepare execution snapshot migration: {}", error)
                })?;
            let rows = statement
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(|error| {
                    format!("Failed to query execution snapshot migration: {}", error)
                })?;
            rows.collect::<Result<Vec<_>, _>>().map_err(|error| {
                format!("Failed to read execution snapshot migration: {}", error)
            })?
        };

        let mut stale_keys = Vec::new();
        for (storage_key, serialized) in snapshots {
            let state: FlowExecutionState = match serde_json::from_str(&serialized) {
                Ok(state) => state,
                Err(error) => {
                    // This cleanup is intentionally proof-based. Older snapshot
                    // shapes may still need a later field-level migration, and a
                    // corrupt record must remain available for operator recovery.
                    // Neither case is evidence that the run is terminal.
                    log::warn!(
                        "Skipping terminal snapshot reconciliation for {}: {}",
                        storage_key,
                        error
                    );
                    continue;
                }
            };
            let run_id = if state.run_id.is_empty() {
                state.flow_id.as_str()
            } else {
                state.run_id.as_str()
            };
            let has_terminal_match = terminal_runs.iter().any(
                |(history_run_id, history_flow_id, history_started_at)| {
                    if history_flow_id != &state.flow_id {
                        return false;
                    }
                    let identity_matches = history_run_id == run_id
                        || (history_run_id == history_flow_id && run_id == state.flow_id);
                    identity_matches
                        && DateTime::parse_from_rfc3339(history_started_at)
                            .map(|timestamp| timestamp.with_timezone(&Utc) == state.started_at)
                            .unwrap_or(false)
                },
            );
            if has_terminal_match {
                stale_keys.push(storage_key);
            }
        }

        let transaction = conn
            .transaction()
            .map_err(|error| format!("Failed to start terminal snapshot migration: {}", error))?;
        let mut removed = 0;
        for storage_key in stale_keys {
            removed += transaction
                .execute(
                    "DELETE FROM execution_states WHERE run_id = ?1",
                    rusqlite::params![storage_key],
                )
                .map_err(|error| {
                    format!("Failed to remove terminal execution snapshot: {}", error)
                })?;
        }
        transaction
            .commit()
            .map_err(|error| format!("Failed to commit terminal snapshot migration: {}", error))?;
        Ok(removed)
    }

    /// One-time data migration from the removed singular recoveryCheckpoint
    /// field. Timers become per-node durable wakes; overlay/outbox operations
    /// already have authoritative ledger rows. The legacy JSON member is then
    /// removed so future state reads cannot accidentally depend on it.
    fn migrate_singular_recovery_checkpoints(conn: &mut Connection) -> Result<(), String> {
        let states = {
            let mut statement = conn
                .prepare("SELECT run_id, state FROM execution_states")
                .map_err(|e| format!("Failed to prepare recovery-state migration: {}", e))?;
            let rows = statement
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(|e| format!("Failed to query recovery-state migration: {}", e))?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("Failed to read recovery-state migration: {}", e))?
        };
        let transaction = conn
            .transaction()
            .map_err(|e| format!("Failed to start recovery-state migration: {}", e))?;
        for (flow_id, serialized) in states {
            let mut state: Value = match serde_json::from_str(&serialized) {
                Ok(state) => state,
                Err(error) => {
                    transaction
                        .execute(
                            "INSERT OR REPLACE INTO workflow_runtime_quarantine
                             (id, record_type, record_key, reason, payload, quarantined_at)
                             VALUES (?1, 'execution_state', ?2, ?3, ?4, ?5)",
                            rusqlite::params![
                                format!("execution_state:{flow_id}"),
                                &flow_id,
                                format!("Invalid legacy execution JSON: {error}"),
                                serialized,
                                Utc::now().to_rfc3339(),
                            ],
                        )
                        .map_err(|e| format!("Failed to preserve invalid state {flow_id}: {e}"))?;
                    transaction
                        .execute(
                            "DELETE FROM execution_states WHERE run_id = ?1",
                            rusqlite::params![flow_id],
                        )
                        .map_err(|e| format!("Failed to quarantine invalid state: {e}"))?;
                    continue;
                }
            };
            let Some(checkpoint) = state
                .as_object_mut()
                .and_then(|object| object.remove("recoveryCheckpoint"))
            else {
                continue;
            };
            if checkpoint.get("kind").and_then(Value::as_str) == Some("durable-delay") {
                let timer_fields =
                    (|| -> Result<(String, String, String, DateTime<Utc>), String> {
                        let run_id = state
                            .get("runId")
                            .and_then(Value::as_str)
                            .filter(|value| !value.is_empty())
                            .unwrap_or(&flow_id)
                            .to_string();
                        let node_id = checkpoint
                            .get("nodeId")
                            .and_then(Value::as_str)
                            .ok_or_else(|| format!("Legacy timer {} has no nodeId", flow_id))?
                            .to_string();
                        let legacy_id = checkpoint
                            .get("correlationId")
                            .and_then(Value::as_str)
                            .unwrap_or(&node_id)
                            .to_string();
                        let deadline = checkpoint
                            .get("deadline")
                            .and_then(Value::as_str)
                            .ok_or_else(|| format!("Legacy timer {} has no deadline", flow_id))?;
                        let due_at = DateTime::parse_from_rfc3339(deadline)
                            .map_err(|e| {
                                format!("Legacy timer {} has invalid deadline: {}", flow_id, e)
                            })?
                            .with_timezone(&Utc);
                        Ok((run_id, node_id, legacy_id, due_at))
                    })();
                let (run_id, node_id, legacy_id, due_at) = match timer_fields {
                    Ok(fields) => fields,
                    Err(reason) => {
                        transaction
                            .execute(
                                "INSERT OR REPLACE INTO workflow_runtime_quarantine
                                 (id, record_type, record_key, reason, payload, quarantined_at)
                                 VALUES (?1, 'execution_state', ?2, ?3, ?4, ?5)",
                                rusqlite::params![
                                    format!("execution_state:{flow_id}"),
                                    &flow_id,
                                    reason,
                                    serialized,
                                    Utc::now().to_rfc3339(),
                                ],
                            )
                            .map_err(|e| {
                                format!("Failed to preserve incompatible timer {flow_id}: {e}")
                            })?;
                        transaction
                            .execute(
                                "DELETE FROM execution_states WHERE run_id = ?1",
                                rusqlite::params![flow_id],
                            )
                            .map_err(|e| format!("Failed to quarantine invalid timer: {e}"))?;
                        continue;
                    }
                };
                let now = Utc::now();
                let wake = WorkflowWake {
                    id: format!("timer:{}:{}:{}", run_id, node_id, legacy_id),
                    run_id,
                    node_id,
                    kind: WorkflowWakeKind::Timer,
                    status: WorkflowWakeStatus::Waiting,
                    due_at: Some(due_at),
                    attempt: 0,
                    payload: serde_json::json!({ "migratedFrom": "recoveryCheckpoint" }),
                    resume_policy: Default::default(),
                    created_at: now,
                    updated_at: now,
                };
                transaction.execute(
                    "INSERT OR IGNORE INTO workflow_wakes (id, run_id, node_id, kind, status, due_at, wake, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    rusqlite::params![
                        wake.id,
                        wake.run_id,
                        wake.node_id,
                        serde_json::to_string(&wake.kind).unwrap_or_default(),
                        serde_json::to_string(&wake.status).unwrap_or_default(),
                        wake.due_at.map(|value| value.to_rfc3339()),
                        serde_json::to_string(&wake).map_err(|e| e.to_string())?,
                        wake.updated_at.to_rfc3339(),
                    ],
                ).map_err(|e| format!("Failed to migrate durable timer {}: {}", flow_id, e))?;
            }
            transaction
                .execute(
                    "UPDATE execution_states SET state = ?2, updated_at = CURRENT_TIMESTAMP WHERE run_id = ?1",
                    rusqlite::params![flow_id, serde_json::to_string(&state).map_err(|e| e.to_string())?],
                )
                .map_err(|e| format!("Failed to remove legacy checkpoint: {}", e))?;
        }
        transaction
            .commit()
            .map_err(|e| format!("Failed to commit recovery-state migration: {}", e))
    }

    fn connection(&self) -> Result<Connection, String> {
        let connection = Connection::open(&self.execution_db_path)
            .map_err(|e| format!("Failed to open execution database: {}", e))?;
        connection
            .busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|e| format!("Failed to configure execution database timeout: {}", e))?;
        connection
            .execute_batch("PRAGMA foreign_keys = ON;")
            .map_err(|e| format!("Failed to configure execution database connection: {}", e))?;
        Ok(connection)
    }

    fn stored_status(status: &FlowStatus) -> &'static str {
        match status {
            FlowStatus::Pending => "pending",
            FlowStatus::Running => "running",
            FlowStatus::RecoveryRequired => "recovery_required",
            FlowStatus::Completed => "completed",
            FlowStatus::CompletedWithErrors => "completed_with_errors",
            FlowStatus::Failed => "failed",
            FlowStatus::Cancelled => "cancelled",
        }
    }

    fn parse_stored_status(status: &str) -> Result<FlowStatus, String> {
        match status {
            "pending" => Ok(FlowStatus::Pending),
            "running" => Ok(FlowStatus::Running),
            "recovery_required" => Ok(FlowStatus::RecoveryRequired),
            "completed" => Ok(FlowStatus::Completed),
            "completed_with_errors" => Ok(FlowStatus::CompletedWithErrors),
            "failed" => Ok(FlowStatus::Failed),
            "cancelled" => Ok(FlowStatus::Cancelled),
            value => Err(format!("Unknown stored workflow status: {}", value)),
        }
    }

    fn insert_execution_history(
        conn: &Connection,
        history_entry: &ExecutionHistoryEntry,
    ) -> Result<(), String> {
        let final_results_json = history_entry
            .final_results
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|error| format!("Failed to serialize final results: {}", error))?;
        let node_details_json = serde_json::to_string(&history_entry.node_execution_details)
            .map_err(|error| format!("Failed to serialize node details: {}", error))?;

        conn.execute(
            "INSERT INTO execution_history (
                id, run_id, flow_id, flow_name, status, started_at, completed_at,
                duration_ms, reachable_node_count, completed_nodes, failed_nodes,
                error_message, final_results, node_execution_details
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            rusqlite::params![
                &history_entry.id,
                &history_entry.run_id,
                &history_entry.flow_id,
                &history_entry.flow_name,
                Self::stored_status(&history_entry.status),
                history_entry.started_at.to_rfc3339(),
                history_entry.completed_at.map(|time| time.to_rfc3339()),
                history_entry.duration_ms.map(|duration| duration as i64),
                history_entry.reachable_node_count as i64,
                history_entry.completed_nodes as i64,
                history_entry.failed_nodes as i64,
                &history_entry.error_message,
                final_results_json,
                node_details_json,
            ],
        )
        .map_err(|error| format!("Failed to save execution history: {}", error))?;
        Ok(())
    }
}

#[async_trait]
impl FlowStorage for HybridFlowStorageBackend {
    async fn finalize_execution(
        &self,
        history_entry: &ExecutionHistoryEntry,
    ) -> Result<(), String> {
        let mut conn = self.connection()?;
        let transaction = conn
            .transaction()
            .map_err(|error| format!("Failed to start workflow finalization: {}", error))?;
        Self::insert_execution_history(&transaction, history_entry)?;
        transaction
            .execute(
                "DELETE FROM execution_states WHERE run_id = ?1",
                rusqlite::params![&history_entry.run_id],
            )
            .map_err(|error| format!("Failed to remove finalized workflow snapshot: {}", error))?;
        transaction
            .commit()
            .map_err(|error| format!("Failed to commit workflow finalization: {}", error))?;
        Ok(())
    }

    async fn list_execution_history(
        &self,
        flow_id: Option<&str>,
        limit: u32,
    ) -> Result<Vec<ExecutionHistoryEntry>, String> {
        let conn = self.connection()?;
        let query = if flow_id.is_some() {
            "SELECT id, run_id, flow_id, flow_name, status, started_at, completed_at,
                    duration_ms, reachable_node_count, completed_nodes, failed_nodes,
                    error_message, final_results, node_execution_details
             FROM execution_history WHERE flow_id = ?1
             ORDER BY started_at DESC LIMIT ?2"
        } else {
            "SELECT id, run_id, flow_id, flow_name, status, started_at, completed_at,
                    duration_ms, reachable_node_count, completed_nodes, failed_nodes,
                    error_message, final_results, node_execution_details
             FROM execution_history
             ORDER BY started_at DESC LIMIT ?2"
        };
        let mut statement = conn
            .prepare(query)
            .map_err(|error| format!("Failed to prepare execution history query: {}", error))?;
        let read_row = |row: &rusqlite::Row<'_>| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<i64>>(7)?,
                row.get::<_, i64>(8)?,
                row.get::<_, i64>(9)?,
                row.get::<_, i64>(10)?,
                row.get::<_, Option<String>>(11)?,
                row.get::<_, Option<String>>(12)?,
                row.get::<_, String>(13)?,
            ))
        };
        let serialized_rows = if let Some(flow_id) = flow_id {
            statement
                .query_map(rusqlite::params![flow_id, limit.max(1)], read_row)
                .map_err(|error| format!("Failed to query execution history: {}", error))?
                .collect::<Result<Vec<_>, _>>()
        } else {
            statement
                .query_map(
                    rusqlite::params![rusqlite::types::Null, limit.max(1)],
                    read_row,
                )
                .map_err(|error| format!("Failed to query execution history: {}", error))?
                .collect::<Result<Vec<_>, _>>()
        }
        .map_err(|error| format!("Failed to read execution history: {}", error))?;

        serialized_rows
            .into_iter()
            .map(
                |(
                    id,
                    run_id,
                    flow_id,
                    flow_name,
                    status,
                    started_at,
                    completed_at,
                    duration_ms,
                    reachable_node_count,
                    completed_nodes,
                    failed_nodes,
                    error_message,
                    final_results,
                    node_execution_details,
                )| {
                    Ok(ExecutionHistoryEntry {
                        id,
                        run_id: if run_id.is_empty() {
                            flow_id.clone()
                        } else {
                            run_id
                        },
                        flow_id,
                        flow_name,
                        status: Self::parse_stored_status(&status)?,
                        started_at: DateTime::parse_from_rfc3339(&started_at)
                            .map_err(|error| format!("Invalid history start timestamp: {}", error))?
                            .with_timezone(&Utc),
                        completed_at: completed_at
                            .map(|value| {
                                DateTime::parse_from_rfc3339(&value)
                                    .map(|time| time.with_timezone(&Utc))
                            })
                            .transpose()
                            .map_err(|error| {
                                format!("Invalid history completion timestamp: {}", error)
                            })?,
                        duration_ms: duration_ms.map(|value| value.max(0) as u64),
                        reachable_node_count: reachable_node_count.max(0) as u32,
                        completed_nodes: completed_nodes.max(0) as u32,
                        failed_nodes: failed_nodes.max(0) as u32,
                        error_message,
                        final_results: final_results
                            .map(|value| serde_json::from_str::<HashMap<String, Value>>(&value))
                            .transpose()
                            .map_err(|error| format!("Invalid final history results: {}", error))?,
                        node_execution_details: serde_json::from_str(&node_execution_details)
                            .map_err(|error| format!("Invalid node history details: {}", error))?,
                    })
                },
            )
            .collect()
    }

    /// 🆕 MANUAL DATABASE MAINTENANCE
    // Flow definitions use main Diesel database
    async fn save_flow(&self, flow: &FlowDefinition) -> Result<(), String> {
        let db = crate::db::Database::new(&self.db_pool);
        db.save_flow(flow)
    }

    async fn replace_flow_at_revision(
        &self,
        expected_revision: &str,
        flow: &FlowDefinition,
    ) -> Result<FlowDefinition, String> {
        let db = crate::db::Database::new(&self.db_pool);
        db.replace_flow_at_revision(expected_revision, flow)
    }

    async fn load_flow(&self, flow_id: &str) -> Result<FlowDefinition, String> {
        let db = crate::db::Database::new(&self.db_pool);
        db.load_flow(flow_id)
    }

    async fn list_flows(&self) -> Result<Vec<FlowSummary>, String> {
        let db = crate::db::Database::new(&self.db_pool);
        db.list_flows()
    }

    async fn delete_flow(&self, flow_id: &str) -> Result<(), String> {
        let db = crate::db::Database::new(&self.db_pool);
        db.delete_flow(flow_id)
    }

    async fn archive_flow(&self, flow_id: &str) -> Result<(), String> {
        // Archive to flows.db execution history database

        let conn = self.connection()?;

        // Create archive table if not exists

        // Load flow from main DB and save to archive
        let flow = self.load_flow(flow_id).await?;
        let flow_json =
            serde_json::to_string(&flow).map_err(|e| format!("Failed to serialize flow: {}", e))?;

        conn.execute(
            "INSERT INTO archived_flows (id, original_flow_id, name, description, data, original_created_at, original_updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                format!("{}_{}", flow_id, chrono::Utc::now().timestamp()),
                flow_id,
                flow.name,
                flow.description.as_deref().unwrap_or(""),
                flow_json,
                flow.created_at.to_rfc3339(),
                flow.updated_at.to_rfc3339()
            ],
        ).map_err(|e| format!("Failed to archive flow: {}", e))?;

        // Delete from main DB
        self.delete_flow(flow_id).await?;
        Ok(())
    }

    // Execution state uses separate SQLite database (flows.db)
    async fn save_execution_state(&self, state: &FlowExecutionState) -> Result<(), String> {
        let json =
            serde_json::to_string(state).map_err(|e| format!("Serialization error: {}", e))?;

        let conn = self.connection()?;

        conn.execute(
            "INSERT INTO execution_states (run_id, state, updated_at)
             VALUES (?1, ?2, CURRENT_TIMESTAMP)
             ON CONFLICT(run_id) DO UPDATE SET
                 state = excluded.state,
                 updated_at = CURRENT_TIMESTAMP",
            rusqlite::params![&state.run_id, &json],
        )
        .map_err(|e| format!("Failed to save execution state: {}", e))?;

        Ok(())
    }

    async fn load_execution_state(&self, flow_id: &str) -> Result<FlowExecutionState, String> {
        let conn = self.connection()?;

        let mut stmt = conn
            .prepare("SELECT state FROM execution_states WHERE run_id = ?1")
            .map_err(|e| format!("Failed to prepare statement: {}", e))?;

        let state_data: Option<String> = stmt
            .query_row(rusqlite::params![flow_id], |row| Ok(row.get(0)?))
            .optional()
            .map_err(|e| format!("Failed to query execution state: {}", e))?;

        if let Some(data) = state_data {
            let state: FlowExecutionState = serde_json::from_str(&data)
                .map_err(|e| format!("Failed to deserialize execution state: {}", e))?;
            Ok(state)
        } else {
            Err(format!("Execution state not found for flow: {}", flow_id))
        }
    }

    async fn list_recoverable_execution_states(&self) -> Result<Vec<FlowExecutionState>, String> {
        let mut conn = self.connection()?;
        let serialized_states = {
            let mut statement = conn
                .prepare("SELECT run_id, state FROM execution_states")
                .map_err(|e| format!("Failed to prepare execution-state query: {}", e))?;
            let rows = statement
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(|e| format!("Failed to query execution states: {}", e))?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("Failed to read execution states: {}", e))?
        };
        let mut interrupted = Vec::new();
        let mut incompatible = Vec::new();
        for (run_id, serialized) in serialized_states {
            let state: FlowExecutionState = match serde_json::from_str(&serialized) {
                Ok(state) => state,
                Err(error) => {
                    incompatible.push((run_id, serialized, error.to_string()));
                    continue;
                }
            };
            if matches!(
                state.status,
                FlowStatus::Pending | FlowStatus::Running | FlowStatus::RecoveryRequired
            ) {
                interrupted.push(state);
            }
        }

        if !incompatible.is_empty() {
            let transaction = conn
                .transaction()
                .map_err(|error| format!("Failed to start runtime quarantine: {error}"))?;
            for (run_id, payload, reason) in incompatible {
                transaction
                    .execute(
                        "INSERT OR REPLACE INTO workflow_runtime_quarantine
                         (id, record_type, record_key, reason, payload, quarantined_at)
                         VALUES (?1, 'execution_state', ?2, ?3, ?4, ?5)",
                        rusqlite::params![
                            format!("execution_state:{run_id}"),
                            &run_id,
                            reason,
                            payload,
                            Utc::now().to_rfc3339(),
                        ],
                    )
                    .map_err(|error| format!("Failed to preserve incompatible state: {error}"))?;
                transaction
                    .execute(
                        "DELETE FROM execution_states WHERE run_id = ?1",
                        rusqlite::params![run_id],
                    )
                    .map_err(|error| format!("Failed to quarantine execution state: {error}"))?;
            }
            transaction
                .commit()
                .map_err(|error| format!("Failed to commit runtime quarantine: {error}"))?;
        }

        Ok(interrupted)
    }

    async fn cleanup_execution_history(
        &self,
        retention_days: u32,
        max_size_mb: u32,
    ) -> Result<usize, String> {
        let conn = self.connection()?;

        // Get current database size
        let current_size_bytes = std::fs::metadata(&self.execution_db_path)
            .map(|m| m.len())
            .unwrap_or(0);
        let current_size_mb = current_size_bytes as f64 / 1024.0 / 1024.0;

        log::info!(
            "📊 EXECUTION DB SIZE CHECK: {:.2}MB (limit: {}MB)",
            current_size_mb,
            max_size_mb
        );

        let mut total_deleted = 0;

        // Execution states are resumable work, not history. They are removed
        // only by terminal finalization, exact terminal-history reconciliation,
        // or explicit orphan cleanup—never merely because they are old.
        total_deleted += conn
            .execute(
                "DELETE FROM execution_history
                 WHERE created_at < datetime('now', ?1)
                   AND created_at < datetime('now', '-1 day')
                   AND id NOT IN (
                       SELECT id FROM execution_history ORDER BY created_at DESC LIMIT 50
                   )",
                rusqlite::params![format!("-{} days", retention_days)],
            )
            .map_err(|e| format!("Failed to delete expired execution history: {}", e))?;

        if current_size_mb > max_size_mb as f64 {
            total_deleted += conn
                .execute(
                    "DELETE FROM execution_history
                     WHERE created_at < datetime('now', '-1 day')
                       AND id NOT IN (
                           SELECT id FROM execution_history ORDER BY created_at DESC LIMIT 50
                       )",
                    [],
                )
                .map_err(|e| format!("Failed to trim execution history by size: {}", e))?;
        }

        // Runtime delivery/audit rows are bounded without ever deleting work
        // that may still require reconciliation.
        total_deleted += conn
            .execute(
                "DELETE FROM overlay_command_deliveries
             WHERE status <> '\"pending\"' AND datetime(updated_at) < datetime('now', ?1)",
                rusqlite::params![format!("-{} days", retention_days)],
            )
            .map_err(|e| format!("Failed to delete old overlay deliveries: {}", e))?;
        total_deleted += conn
            .execute(
                "DELETE FROM workflow_side_effect_outbox
             WHERE status NOT IN ('\"dispatching\"', '\"uncertain\"')
               AND datetime(updated_at) < datetime('now', ?1)",
                rusqlite::params![format!("-{} days", retention_days)],
            )
            .map_err(|e| format!("Failed to delete old outbox entries: {}", e))?;
        total_deleted += conn
            .execute(
                "DELETE FROM workflow_dead_letters
             WHERE resolved_at IS NOT NULL AND datetime(updated_at) < datetime('now', ?1)",
                rusqlite::params![format!("-{} days", retention_days)],
            )
            .map_err(|e| format!("Failed to delete resolved dead letters: {}", e))?;
        total_deleted += conn
            .execute(
                "DELETE FROM workflow_event_inbox
             WHERE datetime(received_at) < datetime('now', ?1)
               AND event_id NOT IN (
                    SELECT event_id FROM workflow_dead_letters WHERE resolved_at IS NULL
               )
               AND event_id NOT IN (
                    SELECT json_extract(node.value, '$.inputs.trigger_event_id')
                    FROM execution_states AS execution,
                         json_each(execution.state, '$.flowDefinition.nodes') AS node
                    WHERE json_type(node.value, '$.inputs.trigger_event_id') = 'text'
               )",
                rusqlite::params![format!("-{} days", retention_days)],
            )
            .map_err(|e| format!("Failed to delete old canonical events: {}", e))?;
        total_deleted += conn
            .execute(
                "DELETE FROM workflow_wakes
                 WHERE status NOT IN ('\"waiting\"', '\"ready\"')
                   AND datetime(updated_at) < datetime('now', ?1)",
                rusqlite::params![format!("-{} days", retention_days)],
            )
            .map_err(|e| format!("Failed to delete terminal workflow wakes: {}", e))?;
        total_deleted += conn
            .execute(
                "DELETE FROM workflow_resource_leases WHERE expires_at <= ?1",
                rusqlite::params![Utc::now().to_rfc3339()],
            )
            .map_err(|e| format!("Failed to delete expired workflow leases: {}", e))?;
        total_deleted += conn.execute(
            "DELETE FROM trigger_deduplication WHERE datetime(claimed_at) < datetime('now', ?1)",
            rusqlite::params![format!("-{} days", retention_days)],
        ).map_err(|e| format!("Failed to delete old trigger deduplication claims: {}", e))?;

        // Vacuum if we deleted anything
        if total_deleted > 0 {
            let _ = conn.execute("VACUUM", []);
        }

        Ok(total_deleted)
    }

    async fn get_database_stats(&self) -> Result<DatabaseStats, String> {
        let conn = self.connection()?;

        let total_size_bytes = std::fs::metadata(&self.execution_db_path)
            .map(|m| m.len())
            .unwrap_or(0);

        // Count from main DB
        let db = crate::db::Database::new(&self.db_pool);
        let flows = db.list_flows().unwrap_or_default();
        let flow_count = flows.len() as u32;

        let execution_states_count = conn
            .query_row("SELECT COUNT(*) FROM execution_states", [], |row| {
                row.get::<_, i64>(0)
            })
            .map_err(|error| format!("Failed to count execution states: {}", error))?
            .max(0) as u32;
        let execution_history_count = conn
            .query_row("SELECT COUNT(*) FROM execution_history", [], |row| {
                row.get::<_, i64>(0)
            })
            .map_err(|error| format!("Failed to count execution history: {}", error))?
            .max(0) as u32;
        let (oldest, newest): (Option<String>, Option<String>) = conn
            .query_row(
                "SELECT MIN(started_at), MAX(started_at) FROM execution_history",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|error| format!("Failed to inspect execution history range: {}", error))?;
        let parse_timestamp = |value: Option<String>| -> Result<Option<DateTime<Utc>>, String> {
            value
                .map(|timestamp| {
                    DateTime::parse_from_rfc3339(&timestamp)
                        .map(|parsed| parsed.with_timezone(&Utc))
                        .map_err(|error| format!("Invalid execution history timestamp: {}", error))
                })
                .transpose()
        };

        Ok(DatabaseStats {
            total_size_bytes,
            flow_count,
            execution_history_count,
            execution_states_count,
            oldest_execution: parse_timestamp(oldest)?,
            newest_execution: parse_timestamp(newest)?,
        })
    }

    async fn vacuum_database(&self) -> Result<(), String> {
        let conn = self.connection()?;

        conn.execute("VACUUM", [])
            .map_err(|e| format!("Failed to vacuum database: {}", e))?;

        Ok(())
    }

    async fn save_trigger_state(
        &self,
        flow_id: &str,
        node_id: &str,
        key: &str,
        value: Value,
    ) -> Result<(), String> {
        let conn = self.connection()?;

        // Create trigger states table if not exists

        let value_json = serde_json::to_string(&value)
            .map_err(|e| format!("Failed to serialize trigger state: {}", e))?;

        conn.execute(
            "INSERT OR REPLACE INTO trigger_states (flow_id, node_id, key, value, updated_at)
             VALUES (?1, ?2, ?3, ?4, CURRENT_TIMESTAMP)",
            rusqlite::params![flow_id, node_id, key, value_json],
        )
        .map_err(|e| format!("Failed to save trigger state: {}", e))?;

        Ok(())
    }

    async fn load_trigger_state(
        &self,
        flow_id: &str,
        node_id: &str,
        key: &str,
    ) -> Result<Option<Value>, String> {
        let conn = self.connection()?;

        let value_json: Option<String> = conn
            .query_row(
                "SELECT value FROM trigger_states WHERE flow_id = ?1 AND node_id = ?2 AND key = ?3",
                rusqlite::params![flow_id, node_id, key],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| format!("Failed to load trigger state: {}", e))?;

        if let Some(json) = value_json {
            let value: Value = serde_json::from_str(&json)
                .map_err(|e| format!("Failed to deserialize trigger state: {}", e))?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    async fn delete_trigger_state(&self, flow_id: &str, node_id: &str) -> Result<(), String> {
        let conn = self.connection()?;

        conn.execute(
            "DELETE FROM trigger_states WHERE flow_id = ?1 AND node_id = ?2",
            rusqlite::params![flow_id, node_id],
        )
        .map_err(|e| format!("Failed to delete trigger state: {}", e))?;

        Ok(())
    }

    async fn claim_trigger_dedup(
        &self,
        dedup_key: &str,
        event_id: Option<&str>,
        occurred_at: DateTime<Utc>,
    ) -> Result<bool, String> {
        let conn = self.connection()?;

        // Retain keys long enough to cover connector retries and restarts while
        // keeping the desktop database bounded. This is deliberately longer
        // than the old ten-second in-memory window.
        conn.execute(
            "DELETE FROM trigger_deduplication WHERE claimed_at < datetime('now', '-7 days')",
            [],
        )
        .map_err(|e| format!("Failed to clean trigger deduplication table: {}", e))?;

        let inserted = conn
            .execute(
                "INSERT OR IGNORE INTO trigger_deduplication (dedup_key, event_id, occurred_at)
             VALUES (?1, ?2, ?3)",
                rusqlite::params![dedup_key, event_id, occurred_at.to_rfc3339()],
            )
            .map_err(|e| format!("Failed to claim trigger deduplication key: {}", e))?;

        Ok(inserted == 1)
    }

    async fn replace_active_triggers(
        &self,
        flow_id: &str,
        triggers: &[ActiveTriggerDefinition],
    ) -> Result<(), String> {
        let mut conn = self.connection()?;

        let transaction = conn
            .transaction()
            .map_err(|e| format!("Failed to start trigger registry transaction: {}", e))?;
        transaction
            .execute(
                "DELETE FROM active_trigger_registry WHERE flow_id = ?1",
                rusqlite::params![flow_id],
            )
            .map_err(|e| format!("Failed to replace trigger registry: {}", e))?;

        for trigger in triggers {
            let definition = serde_json::to_string(trigger)
                .map_err(|e| format!("Failed to serialize trigger definition: {}", e))?;
            transaction
                .execute(
                    "INSERT INTO active_trigger_registry (flow_id, node_id, definition)
                     VALUES (?1, ?2, ?3)",
                    rusqlite::params![flow_id, &trigger.node_id, definition],
                )
                .map_err(|e| format!("Failed to insert active trigger: {}", e))?;
        }

        transaction
            .commit()
            .map_err(|e| format!("Failed to commit trigger registry: {}", e))
    }

    async fn remove_active_triggers(&self, flow_id: &str) -> Result<(), String> {
        let conn = self.connection()?;
        conn.execute(
            "DELETE FROM active_trigger_registry WHERE flow_id = ?1",
            rusqlite::params![flow_id],
        )
        .map_err(|e| format!("Failed to remove active triggers: {}", e))?;
        Ok(())
    }

    async fn clear_active_triggers(&self) -> Result<(), String> {
        let conn = self.connection()?;
        conn.execute("DELETE FROM active_trigger_registry", [])
            .map_err(|e| format!("Failed to clear active trigger registry: {}", e))?;
        Ok(())
    }

    async fn list_active_triggers(&self) -> Result<Vec<ActiveTriggerDefinition>, String> {
        let conn = self.connection()?;

        let mut statement = conn
            .prepare("SELECT definition FROM active_trigger_registry ORDER BY flow_id, node_id")
            .map_err(|e| format!("Failed to query active trigger registry: {}", e))?;
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| format!("Failed to read active trigger registry: {}", e))?;
        let mut triggers = Vec::new();
        for row in rows {
            let definition = row.map_err(|e| format!("Failed to read active trigger: {}", e))?;
            triggers.push(
                serde_json::from_str(&definition)
                    .map_err(|e| format!("Failed to deserialize active trigger: {}", e))?,
            );
        }
        Ok(triggers)
    }

    async fn store_event_envelope(
        &self,
        envelope: &crate::events::EventEnvelope,
    ) -> Result<bool, String> {
        let serialized = serde_json::to_string(envelope)
            .map_err(|e| format!("Failed to serialize event envelope: {}", e))?;
        let mut conn = self.connection()?;
        let transaction = conn
            .transaction()
            .map_err(|error| format!("Failed to admit stream event: {error}"))?;

        let inserted = transaction
            .execute(
                "INSERT OR IGNORE INTO workflow_event_inbox
             (event_id, event_type, source, envelope, received_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![
                    &envelope.event_id,
                    &envelope.event_type,
                    &envelope.source,
                    serialized,
                    envelope.received_at.to_rfc3339(),
                ],
            )
            .map_err(|e| format!("Failed to store workflow event: {}", e))?;
        if inserted == 1
            && let Some((metric, increment)) =
                crate::services::streaming::stream_values::observed_increment(envelope)
        {
            let account = envelope
                .account_id
                .ok_or("Observed metric account missing")?
                .to_string();
            let increment =
                i64::try_from(increment).map_err(|_| "Observed increment is too large")?;
            let changed = transaction.execute(
                    "INSERT INTO observed_stream_values(platform,account_id,metric,total,first_observed_at,last_observed_at)
                     VALUES(?1,?2,?3,?4,?5,?5)
                     ON CONFLICT(platform,account_id,metric) DO UPDATE SET
                       total=total+excluded.total,last_observed_at=excluded.last_observed_at
                     WHERE total<=?6",
                    rusqlite::params![envelope.source,account,metric,increment,envelope.received_at.to_rfc3339(),i64::MAX-increment]
                ).map_err(|error| format!("Failed to project accepted stream value: {error}"))?;
            if changed != 1 {
                return Err("Observed stream counter exhausted".into());
            }
        }
        transaction
            .commit()
            .map_err(|error| format!("Failed to commit accepted stream event: {error}"))?;
        Ok(inserted == 1)
    }

    async fn observed_stream_values(
        &self,
    ) -> Result<Vec<crate::services::streaming::stream_values::ObservedStreamValue>, String> {
        let conn = self.connection()?;
        let mut statement = conn.prepare("SELECT platform,account_id,metric,total,first_observed_at,last_observed_at FROM observed_stream_values ORDER BY platform,account_id,metric")
            .map_err(|error| format!("Failed to read observed stream values: {error}"))?;
        let rows = statement
            .query_map([], |row| {
                let account: String = row.get(1)?;
                let account_id = account.parse::<u64>().map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        1,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?;
                Ok(
                    crate::services::streaming::stream_values::ObservedStreamValue {
                        platform: row.get(0)?,
                        account_id,
                        metric: row.get(2)?,
                        total: row.get(3)?,
                        first_observed_at: row.get(4)?,
                        last_observed_at: row.get(5)?,
                    },
                )
            })
            .map_err(|error| format!("Failed to query observed stream values: {error}"))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("Invalid observed stream value: {error}"))
    }

    async fn load_event_envelope(
        &self,
        event_id: &str,
    ) -> Result<Option<crate::events::EventEnvelope>, String> {
        let conn = self.connection()?;

        let serialized = conn
            .query_row(
                "SELECT envelope FROM workflow_event_inbox WHERE event_id = ?1",
                rusqlite::params![event_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|e| format!("Failed to load workflow event: {}", e))?;

        serialized
            .map(|value| {
                serde_json::from_str(&value)
                    .map_err(|e| format!("Failed to deserialize workflow event: {}", e))
            })
            .transpose()
    }

    async fn trace_workflow_event(
        &self,
        event_id: &str,
    ) -> Result<Option<crate::events::WorkflowRuntimeTrace>, String> {
        fn parse_time(value: &str, field: &str) -> Result<DateTime<Utc>, String> {
            DateTime::parse_from_rfc3339(value)
                .map(|value| value.with_timezone(&Utc))
                .map_err(|error| format!("Invalid {field} timestamp: {error}"))
        }

        let Some(envelope) = self.load_event_envelope(event_id).await? else {
            return Ok(None);
        };
        let conn = self.connection()?;
        let mut runs = Vec::new();

        {
            let mut statement = conn
                .prepare("SELECT state FROM execution_states")
                .map_err(|error| format!("Failed to prepare active event trace: {error}"))?;
            let rows = statement
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(|error| format!("Failed to query active event trace: {error}"))?;
            for row in rows {
                let serialized = row.map_err(|error| error.to_string())?;
                let Ok(state) = serde_json::from_str::<FlowExecutionState>(&serialized) else {
                    continue;
                };
                if state.flow_definition.nodes.iter().any(|node| {
                    node.inputs.get("trigger_event_id").and_then(Value::as_str) == Some(event_id)
                }) {
                    runs.push(crate::events::WorkflowRunTrace {
                        run_id: if state.run_id.is_empty() {
                            state.flow_id.clone()
                        } else {
                            state.run_id
                        },
                        flow_id: state.flow_id,
                        status: format!("{:?}", state.status),
                        started_at: state.started_at,
                        completed_at: state.completed_at,
                    });
                }
            }
        }

        {
            let mut statement = conn
                .prepare(
                    "SELECT run_id, flow_id, status, started_at, completed_at
                     FROM execution_history
                     WHERE EXISTS (
                         SELECT 1 FROM json_each(execution_history.node_execution_details) AS detail
                         WHERE json_extract(detail.value, '$.inputs.trigger_event_id') = ?1
                     )",
                )
                .map_err(|error| format!("Failed to prepare terminal event trace: {error}"))?;
            let rows = statement
                .query_map(rusqlite::params![event_id], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, Option<String>>(4)?,
                    ))
                })
                .map_err(|error| format!("Failed to query terminal event trace: {error}"))?;
            for row in rows {
                let (run_id, flow_id, status, started_at, completed_at) =
                    row.map_err(|error| error.to_string())?;
                runs.push(crate::events::WorkflowRunTrace {
                    run_id,
                    flow_id,
                    status,
                    started_at: parse_time(&started_at, "run start")?,
                    completed_at: completed_at
                        .as_deref()
                        .map(|value| parse_time(value, "run completion"))
                        .transpose()?,
                });
            }
        }
        runs.sort_by_key(|run| run.started_at);
        runs.dedup_by(|left, right| left.run_id == right.run_id);
        let run_ids = runs
            .iter()
            .map(|run| run.run_id.as_str())
            .collect::<HashSet<_>>();

        let mut side_effect_statement = conn
            .prepare("SELECT entry FROM workflow_side_effect_outbox")
            .map_err(|error| format!("Failed to prepare side-effect event trace: {error}"))?;
        let side_effect_rows = side_effect_statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|error| format!("Failed to query side-effect event trace: {error}"))?;
        let mut side_effect_entries = Vec::new();
        for row in side_effect_rows {
            side_effect_entries.push(
                serde_json::from_str::<crate::events::SideEffectOutboxEntry>(
                    &row.map_err(|error| error.to_string())?,
                )
                .map_err(|error| format!("Failed to deserialize traced side effect: {error}"))?,
            );
        }
        drop(side_effect_statement);
        let side_effects = side_effect_entries
            .into_iter()
            .filter(|entry| run_ids.contains(entry.run_id.as_str()))
            .map(|entry| crate::events::WorkflowSideEffectTrace {
                idempotency_key: entry.idempotency_key,
                invocation_id: entry.invocation_id,
                run_id: entry.run_id,
                node_id: entry.node_id,
                status: entry.status,
                has_error: entry.error.is_some(),
                updated_at: entry.updated_at,
            })
            .collect();

        let mut dead_letters = Vec::new();
        let mut statement = conn
            .prepare("SELECT record FROM workflow_dead_letters WHERE event_id = ?1")
            .map_err(|error| format!("Failed to prepare dead-letter event trace: {error}"))?;
        let rows = statement
            .query_map(rusqlite::params![event_id], |row| row.get::<_, String>(0))
            .map_err(|error| format!("Failed to query dead-letter event trace: {error}"))?;
        for row in rows {
            let record: crate::events::WorkflowDeadLetter = serde_json::from_str(
                &row.map_err(|error| error.to_string())?,
            )
            .map_err(|error| format!("Failed to deserialize traced dead letter: {error}"))?;
            dead_letters.push(crate::events::WorkflowDeadLetterTrace {
                id: record.id,
                flow_id: record.flow_id,
                node_id: record.node_id,
                attempts: record.attempts,
                resolved_at: record.resolved_at,
                has_reason: !record.reason.is_empty(),
                updated_at: record.updated_at,
            });
        }

        Ok(Some(crate::events::WorkflowRuntimeTrace {
            event: crate::events::WorkflowEventTrace {
                event_id: envelope.event_id,
                event_type: envelope.event_type,
                source: envelope.source,
                schema_version: envelope.schema_version,
                occurred_at: envelope.occurred_at,
                received_at: envelope.received_at,
                correlation_id: envelope.correlation_id,
            },
            runs,
            side_effects,
            invocations: Vec::new(),
            dead_letters,
        }))
    }

    async fn list_event_envelopes(
        &self,
        event_type: Option<&str>,
        since: Option<DateTime<Utc>>,
        limit: u32,
    ) -> Result<Vec<crate::events::EventEnvelope>, String> {
        let conn = self.connection()?;
        let since = since.map(|value| value.to_rfc3339());
        let mut statement = conn
            .prepare(
                "SELECT envelope FROM workflow_event_inbox
                 WHERE (?1 IS NULL OR event_type = ?1)
                   AND (?2 IS NULL OR received_at >= ?2)
                 ORDER BY received_at DESC
                 LIMIT ?3",
            )
            .map_err(|error| format!("Failed to prepare workflow inbox query: {}", error))?;
        let rows = statement
            .query_map(
                rusqlite::params![event_type, since.as_deref(), i64::from(limit)],
                |row| row.get::<_, String>(0),
            )
            .map_err(|error| format!("Failed to query workflow inbox: {}", error))?;
        let mut envelopes = Vec::new();
        for row in rows {
            let serialized =
                row.map_err(|error| format!("Failed to read workflow inbox row: {}", error))?;
            envelopes.push(
                serde_json::from_str(&serialized)
                    .map_err(|error| format!("Failed to deserialize workflow event: {}", error))?,
            );
        }
        Ok(envelopes)
    }

    async fn save_overlay_command_delivery(
        &self,
        delivery: &crate::events::OverlayCommandDelivery,
    ) -> Result<(), String> {
        let serialized = serde_json::to_string(delivery)
            .map_err(|error| format!("Failed to serialize overlay delivery: {}", error))?;
        let expires_at = delivery.command.expires_at.map(|value| value.to_rfc3339());
        let conn = self.connection()?;
        conn.execute(
            "INSERT OR REPLACE INTO overlay_command_deliveries (command_id, status, expires_at, delivery, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                &delivery.command.command_id,
                serde_json::to_string(&delivery.status).unwrap_or_default(),
                expires_at,
                serialized,
                delivery.updated_at.to_rfc3339(),
            ],
        ).map_err(|error| format!("Failed to save overlay delivery: {}", error))?;
        Ok(())
    }

    async fn resolve_overlay_command_delivery(
        &self,
        acknowledgement: &crate::events::OverlayCommandAcknowledgement,
    ) -> Result<bool, String> {
        let conn = self.connection()?;
        let serialized: Option<String> = conn.query_row(
            "SELECT delivery FROM overlay_command_deliveries WHERE command_id = ?1 AND status = '\"pending\"'",
            rusqlite::params![&acknowledgement.command_id], |row| row.get(0),
        ).optional().map_err(|error| format!("Failed to load overlay delivery: {}", error))?;
        let Some(serialized) = serialized else {
            return Ok(false);
        };
        let mut delivery: crate::events::OverlayCommandDelivery = serde_json::from_str(&serialized)
            .map_err(|error| format!("Failed to deserialize overlay delivery: {}", error))?;
        delivery.status = match acknowledgement.status {
            crate::events::OverlayCommandAckStatus::Accepted => {
                crate::events::OverlayCommandDeliveryStatus::Acknowledged
            }
            crate::events::OverlayCommandAckStatus::Rejected => {
                crate::events::OverlayCommandDeliveryStatus::Rejected
            }
        };
        delivery.acknowledgement = Some(acknowledgement.clone());
        delivery.updated_at = Utc::now();
        let updated = serde_json::to_string(&delivery)
            .map_err(|error| format!("Failed to serialize resolved overlay delivery: {}", error))?;
        let changed = conn
            .execute(
                "UPDATE overlay_command_deliveries SET status = ?2, delivery = ?3, updated_at = ?4
             WHERE command_id = ?1 AND status = '\"pending\"'",
                rusqlite::params![
                    &acknowledgement.command_id,
                    serde_json::to_string(&delivery.status).unwrap_or_default(),
                    updated,
                    delivery.updated_at.to_rfc3339(),
                ],
            )
            .map_err(|error| format!("Failed to resolve overlay delivery: {}", error))?;
        Ok(changed == 1)
    }

    async fn expire_overlay_command_delivery(&self, command_id: &str) -> Result<bool, String> {
        let conn = self.connection()?;
        let serialized: Option<String> = conn.query_row(
            "SELECT delivery FROM overlay_command_deliveries WHERE command_id = ?1 AND status = '\"pending\"'",
            rusqlite::params![command_id],
            |row| row.get(0),
        ).optional().map_err(|error| format!("Failed to load overlay delivery: {}", error))?;
        let Some(serialized) = serialized else {
            return Ok(false);
        };
        let mut delivery: crate::events::OverlayCommandDelivery = serde_json::from_str(&serialized)
            .map_err(|error| format!("Failed to deserialize overlay delivery: {}", error))?;
        delivery.status = crate::events::OverlayCommandDeliveryStatus::Expired;
        delivery.updated_at = Utc::now();
        let updated = serde_json::to_string(&delivery)
            .map_err(|error| format!("Failed to serialize expired overlay delivery: {}", error))?;
        let changed = conn.execute(
            "UPDATE overlay_command_deliveries SET status = '\"expired\"', delivery = ?2, updated_at = ?3
             WHERE command_id = ?1 AND status = '\"pending\"'",
            rusqlite::params![command_id, updated, delivery.updated_at.to_rfc3339()],
        ).map_err(|error| format!("Failed to expire overlay delivery: {}", error))?;
        Ok(changed == 1)
    }

    async fn expire_pending_overlay_command_deliveries_for_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<String>, String> {
        let deliveries = self.list_overlay_command_deliveries(run_id).await?;
        let mut expired = Vec::new();
        for delivery in deliveries {
            if delivery.status == crate::events::OverlayCommandDeliveryStatus::Pending
                && self
                    .expire_overlay_command_delivery(&delivery.command.command_id)
                    .await?
            {
                expired.push(delivery.command.command_id);
            }
        }
        Ok(expired)
    }

    async fn list_pending_overlay_command_deliveries(
        &self,
    ) -> Result<Vec<crate::events::OverlayCommandDelivery>, String> {
        let conn = self.connection()?;
        conn.execute(
            "UPDATE overlay_command_deliveries SET status = '\"expired\"', updated_at = ?1
             WHERE status = '\"pending\"' AND expires_at IS NOT NULL AND expires_at <= ?1",
            rusqlite::params![Utc::now().to_rfc3339()],
        )
        .map_err(|error| format!("Failed to expire overlay deliveries: {}", error))?;
        let mut statement = conn.prepare(
            "SELECT delivery FROM overlay_command_deliveries WHERE status = '\"pending\"' ORDER BY updated_at",
        ).map_err(|error| format!("Failed to prepare pending delivery query: {}", error))?;
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|error| format!("Failed to query pending deliveries: {}", error))?;
        rows.map(|row| {
            serde_json::from_str(
                &row.map_err(|error| format!("Failed to read pending delivery: {}", error))?,
            )
            .map_err(|error| format!("Failed to deserialize pending delivery: {}", error))
        })
        .collect()
    }

    async fn load_overlay_command_delivery(
        &self,
        command_id: &str,
    ) -> Result<Option<crate::events::OverlayCommandDelivery>, String> {
        let conn = self.connection()?;
        let stored = conn
            .query_row(
                "SELECT delivery, status FROM overlay_command_deliveries WHERE command_id = ?1",
                rusqlite::params![command_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(|e| format!("Failed to load overlay delivery: {}", e))?;
        stored
            .map(|(value, status)| {
                let mut delivery: crate::events::OverlayCommandDelivery =
                    serde_json::from_str(&value)
                        .map_err(|e| format!("Failed to deserialize overlay delivery: {}", e))?;
                delivery.status = serde_json::from_str(&status)
                    .map_err(|e| format!("Failed to deserialize overlay delivery status: {}", e))?;
                Ok(delivery)
            })
            .transpose()
    }

    async fn list_overlay_command_deliveries(
        &self,
        run_id: &str,
    ) -> Result<Vec<crate::events::OverlayCommandDelivery>, String> {
        let conn = self.connection()?;
        let mut statement = conn
            .prepare("SELECT delivery, status FROM overlay_command_deliveries ORDER BY updated_at")
            .map_err(|e| format!("Failed to prepare overlay delivery query: {}", e))?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| format!("Failed to query overlay deliveries: {}", e))?;
        let mut deliveries = Vec::new();
        for row in rows {
            let (value, status) =
                row.map_err(|e| format!("Failed to read overlay delivery: {}", e))?;
            let mut delivery: crate::events::OverlayCommandDelivery = serde_json::from_str(&value)
                .map_err(|e| format!("Failed to deserialize overlay delivery: {}", e))?;
            delivery.status = serde_json::from_str(&status)
                .map_err(|e| format!("Failed to deserialize overlay delivery status: {}", e))?;
            if delivery.run_id == run_id {
                deliveries.push(delivery);
            }
        }
        Ok(deliveries)
    }

    async fn claim_side_effect_outbox(
        &self,
        entry: &crate::events::SideEffectOutboxEntry,
    ) -> Result<bool, String> {
        let conn = self.connection()?;
        let inserted = conn.execute(
            "INSERT OR IGNORE INTO workflow_side_effect_outbox (idempotency_key, status, entry, updated_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![&entry.idempotency_key, serde_json::to_string(&entry.status).unwrap_or_default(), serde_json::to_string(entry).map_err(|e| e.to_string())?, entry.updated_at.to_rfc3339()],
        ).map_err(|e| format!("Failed to claim side-effect outbox entry: {}", e))?;
        Ok(inserted == 1)
    }

    async fn replace_side_effect_outbox_attempt(
        &self,
        entry: &crate::events::SideEffectOutboxEntry,
    ) -> Result<(), String> {
        let conn = self.connection()?;
        let changed = conn
            .execute(
                "UPDATE workflow_side_effect_outbox SET status = ?2, entry = ?3, updated_at = ?4 WHERE idempotency_key = ?1",
                rusqlite::params![
                    &entry.idempotency_key,
                    serde_json::to_string(&entry.status).unwrap_or_default(),
                    serde_json::to_string(entry).map_err(|error| error.to_string())?,
                    entry.updated_at.to_rfc3339(),
                ],
            )
            .map_err(|error| format!("Failed to persist approved side-effect attempt: {}", error))?;
        if changed != 1 {
            return Err(format!(
                "Side-effect outbox record '{}' disappeared before approved retry",
                entry.idempotency_key
            ));
        }
        Ok(())
    }

    async fn complete_side_effect_outbox(
        &self,
        key: &str,
        status: crate::events::SideEffectOutboxStatus,
        error: Option<String>,
    ) -> Result<(), String> {
        let conn = self.connection()?;
        let raw: String = conn
            .query_row(
                "SELECT entry FROM workflow_side_effect_outbox WHERE idempotency_key = ?1",
                rusqlite::params![key],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to load side-effect outbox entry: {}", e))?;
        let mut entry: crate::events::SideEffectOutboxEntry = serde_json::from_str(&raw)
            .map_err(|e| format!("Failed to deserialize side-effect outbox entry: {}", e))?;
        entry.status = status;
        entry.error = error;
        entry.updated_at = Utc::now();
        conn.execute("UPDATE workflow_side_effect_outbox SET status = ?2, entry = ?3, updated_at = ?4 WHERE idempotency_key = ?1", rusqlite::params![key, serde_json::to_string(&entry.status).unwrap_or_default(), serde_json::to_string(&entry).map_err(|e| e.to_string())?, entry.updated_at.to_rfc3339()])
            .map_err(|e| format!("Failed to complete side-effect outbox entry: {}", e))?;
        Ok(())
    }

    async fn list_side_effect_outbox(
        &self,
    ) -> Result<Vec<crate::events::SideEffectOutboxEntry>, String> {
        let conn = self.connection()?;
        let mut statement = conn
            .prepare(
                "SELECT entry FROM workflow_side_effect_outbox ORDER BY updated_at DESC LIMIT 500",
            )
            .map_err(|e| format!("Failed to prepare side-effect outbox query: {}", e))?;
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| format!("Failed to query side-effect outbox: {}", e))?;
        rows.map(|row| {
            serde_json::from_str(
                &row.map_err(|e| format!("Failed to read side-effect outbox entry: {}", e))?,
            )
            .map_err(|e| format!("Failed to deserialize side-effect outbox entry: {}", e))
        })
        .collect()
    }

    async fn load_side_effect_outbox(
        &self,
        key: &str,
    ) -> Result<Option<crate::events::SideEffectOutboxEntry>, String> {
        let conn = self.connection()?;
        let serialized = conn
            .query_row(
                "SELECT entry FROM workflow_side_effect_outbox WHERE idempotency_key = ?1",
                rusqlite::params![key],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|e| format!("Failed to load side-effect outbox entry: {}", e))?;
        serialized
            .map(|value| {
                serde_json::from_str(&value)
                    .map_err(|e| format!("Failed to deserialize side-effect outbox entry: {}", e))
            })
            .transpose()
    }

    async fn workflow_runtime_metrics(
        &self,
    ) -> Result<crate::events::WorkflowRuntimeMetrics, String> {
        fn count(conn: &Connection, table: &str, status: Option<&str>) -> Result<u64, String> {
            let query = match status {
                Some(_) => format!("SELECT COUNT(*) FROM {} WHERE status = ?1", table),
                None => format!("SELECT COUNT(*) FROM {}", table),
            };
            let value: i64 = match status {
                Some(status) => conn.query_row(&query, rusqlite::params![status], |row| row.get(0)),
                None => conn.query_row(&query, [], |row| row.get(0)),
            }
            .map_err(|error| format!("Failed to count {} runtime records: {}", table, error))?;
            Ok(value.max(0) as u64)
        }

        let conn = self.connection()?;
        conn.execute(
            "DELETE FROM workflow_resource_leases WHERE expires_at <= ?1",
            rusqlite::params![Utc::now().to_rfc3339()],
        )
        .map_err(|error| format!("Failed to expire resource leases for metrics: {}", error))?;
        let oldest_dispatching = conn.query_row(
            "SELECT MIN(updated_at) FROM workflow_side_effect_outbox WHERE status = '\"dispatching\"'",
            [],
            |row| row.get::<_, Option<String>>(0),
        ).map_err(|error| format!("Failed to inspect oldest side effect: {}", error))?
            .map(|value| DateTime::parse_from_rfc3339(&value)
                .map(|parsed| parsed.with_timezone(&Utc))
                .map_err(|error| format!("Invalid side-effect timestamp: {}", error)))
            .transpose()?;

        Ok(crate::events::WorkflowRuntimeMetrics {
            canonical_events: count(&conn, "workflow_event_inbox", None)?,
            pending_overlay_commands: count(
                &conn,
                "overlay_command_deliveries",
                Some("\"pending\""),
            )?,
            acknowledged_overlay_commands: count(
                &conn,
                "overlay_command_deliveries",
                Some("\"acknowledged\""),
            )?,
            rejected_overlay_commands: count(
                &conn,
                "overlay_command_deliveries",
                Some("\"rejected\""),
            )?,
            expired_overlay_commands: count(
                &conn,
                "overlay_command_deliveries",
                Some("\"expired\""),
            )?,
            dispatching_side_effects: count(
                &conn,
                "workflow_side_effect_outbox",
                Some("\"dispatching\""),
            )?,
            succeeded_side_effects: count(
                &conn,
                "workflow_side_effect_outbox",
                Some("\"succeeded\""),
            )?,
            failed_side_effects: count(&conn, "workflow_side_effect_outbox", Some("\"failed\""))?,
            uncertain_side_effects: count(
                &conn,
                "workflow_side_effect_outbox",
                Some("\"uncertain\""),
            )?,
            unresolved_dead_letters: conn
                .query_row(
                    "SELECT COUNT(*) FROM workflow_dead_letters WHERE resolved_at IS NULL",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(|error| format!("Failed to count dead letters: {}", error))?
                .max(0) as u64,
            open_workflow_wakes: conn
                .query_row(
                    "SELECT COUNT(*) FROM workflow_wakes WHERE status IN ('\"waiting\"', '\"ready\"')",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(|error| format!("Failed to count workflow wakes: {}", error))?
                .max(0) as u64,
            waiting_approvals: conn
                .query_row(
                    "SELECT COUNT(*) FROM workflow_wakes WHERE kind = '\"approval\"' AND status = '\"waiting\"'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(|error| format!("Failed to count approval wakes: {}", error))?
                .max(0) as u64,
            active_resource_leases: conn
                .query_row("SELECT COUNT(*) FROM workflow_resource_leases", [], |row| {
                    row.get::<_, i64>(0)
                })
                .map_err(|error| format!("Failed to count resource leases: {}", error))?
                .max(0) as u64,
            incompatible_runtime_records: count(&conn, "workflow_runtime_quarantine", None)?,
            oldest_dispatching_side_effect_at: oldest_dispatching,
        })
    }

    async fn save_workflow_dead_letter(
        &self,
        record: &crate::events::WorkflowDeadLetter,
    ) -> Result<(), String> {
        let conn = self.connection()?;
        conn.execute(
            "INSERT OR REPLACE INTO workflow_dead_letters (id, event_id, resolved_at, record, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                &record.id,
                &record.event_id,
                record.resolved_at.map(|value| value.to_rfc3339()),
                serde_json::to_string(record).map_err(|error| error.to_string())?,
                record.updated_at.to_rfc3339(),
            ],
        ).map_err(|error| format!("Failed to save workflow dead letter: {}", error))?;
        Ok(())
    }

    async fn load_workflow_dead_letter(
        &self,
        id: &str,
    ) -> Result<Option<crate::events::WorkflowDeadLetter>, String> {
        let conn = self.connection()?;
        let value = conn
            .query_row(
                "SELECT record FROM workflow_dead_letters WHERE id = ?1",
                rusqlite::params![id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| format!("Failed to load workflow dead letter: {}", error))?;
        value
            .map(|serialized| {
                serde_json::from_str(&serialized).map_err(|error| {
                    format!("Failed to deserialize workflow dead letter: {}", error)
                })
            })
            .transpose()
    }

    async fn list_workflow_dead_letters(
        &self,
    ) -> Result<Vec<crate::events::WorkflowDeadLetter>, String> {
        let conn = self.connection()?;
        let mut statement = conn.prepare(
            "SELECT record FROM workflow_dead_letters WHERE resolved_at IS NULL ORDER BY updated_at DESC LIMIT 500",
        ).map_err(|error| format!("Failed to prepare workflow dead-letter query: {}", error))?;
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|error| format!("Failed to query workflow dead letters: {}", error))?;
        rows.map(|row| {
            serde_json::from_str(&row.map_err(|error| error.to_string())?)
                .map_err(|error| format!("Failed to deserialize workflow dead letter: {}", error))
        })
        .collect()
    }

    async fn save_workflow_wake(&self, wake: &WorkflowWake) -> Result<(), String> {
        let conn = self.connection()?;
        conn.execute(
            "INSERT OR REPLACE INTO workflow_wakes
                (id, run_id, node_id, kind, status, due_at, wake, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                &wake.id,
                &wake.run_id,
                &wake.node_id,
                serde_json::to_string(&wake.kind).map_err(|error| error.to_string())?,
                serde_json::to_string(&wake.status).map_err(|error| error.to_string())?,
                wake.due_at.map(|value| value.to_rfc3339()),
                serde_json::to_string(wake).map_err(|error| error.to_string())?,
                wake.updated_at.to_rfc3339(),
            ],
        )
        .map_err(|error| format!("Failed to save workflow wake: {}", error))?;
        Ok(())
    }

    async fn load_workflow_wake(&self, wake_id: &str) -> Result<Option<WorkflowWake>, String> {
        let conn = self.connection()?;
        let serialized = conn
            .query_row(
                "SELECT wake FROM workflow_wakes WHERE id = ?1",
                rusqlite::params![wake_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| format!("Failed to load workflow wake: {}", error))?;
        serialized
            .map(|value| {
                serde_json::from_str(&value)
                    .map_err(|error| format!("Failed to deserialize workflow wake: {}", error))
            })
            .transpose()
    }

    async fn list_open_workflow_wakes(
        &self,
        run_id: Option<&str>,
    ) -> Result<Vec<WorkflowWake>, String> {
        let conn = self.connection()?;
        let waiting = serde_json::to_string(&WorkflowWakeStatus::Waiting)
            .map_err(|error| error.to_string())?;
        let ready =
            serde_json::to_string(&WorkflowWakeStatus::Ready).map_err(|error| error.to_string())?;
        let query = if run_id.is_some() {
            "SELECT wake FROM workflow_wakes
             WHERE run_id = ?1 AND status IN (?2, ?3)
             ORDER BY COALESCE(due_at, '9999-12-31T23:59:59Z'), updated_at"
        } else {
            "SELECT wake FROM workflow_wakes
             WHERE status IN (?2, ?3)
             ORDER BY COALESCE(due_at, '9999-12-31T23:59:59Z'), updated_at"
        };
        let mut statement = conn
            .prepare(query)
            .map_err(|error| format!("Failed to prepare workflow wake query: {}", error))?;
        let deserialize = |row: &rusqlite::Row<'_>| row.get::<_, String>(0);
        let serialized = if let Some(run_id) = run_id {
            statement
                .query_map(rusqlite::params![run_id, waiting, ready], deserialize)
                .map_err(|error| format!("Failed to query workflow wakes: {}", error))?
                .collect::<Result<Vec<_>, _>>()
        } else {
            statement
                .query_map(
                    rusqlite::params![rusqlite::types::Null, waiting, ready],
                    deserialize,
                )
                .map_err(|error| format!("Failed to query workflow wakes: {}", error))?
                .collect::<Result<Vec<_>, _>>()
        }
        .map_err(|error| format!("Failed to read workflow wake: {}", error))?;
        serialized
            .into_iter()
            .map(|value| {
                serde_json::from_str(&value)
                    .map_err(|error| format!("Failed to deserialize workflow wake: {}", error))
            })
            .collect()
    }

    async fn set_workflow_wake_status(
        &self,
        wake_id: &str,
        status: WorkflowWakeStatus,
    ) -> Result<bool, String> {
        let mut conn = self.connection()?;
        let transaction = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|error| format!("Failed to start wake transition: {error}"))?;
        let existing = transaction
            .query_row(
                "SELECT wake FROM workflow_wakes WHERE id = ?1",
                rusqlite::params![wake_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| format!("Failed to load workflow wake: {}", error))?;
        let Some(serialized) = existing else {
            return Ok(false);
        };
        let mut wake: WorkflowWake = serde_json::from_str(&serialized)
            .map_err(|error| format!("Failed to deserialize workflow wake: {}", error))?;
        if wake.status == status {
            return Ok(true);
        }
        let transition_allowed = matches!(
            (&wake.status, &status),
            (WorkflowWakeStatus::Waiting, WorkflowWakeStatus::Ready)
                | (WorkflowWakeStatus::Waiting, WorkflowWakeStatus::Consumed)
                | (WorkflowWakeStatus::Waiting, WorkflowWakeStatus::Cancelled)
                | (WorkflowWakeStatus::Ready, WorkflowWakeStatus::Consumed)
                | (WorkflowWakeStatus::Ready, WorkflowWakeStatus::Cancelled)
        );
        if !transition_allowed {
            return Ok(false);
        }
        let previous = serde_json::to_string(&wake.status).map_err(|error| error.to_string())?;
        wake.status = status;
        wake.updated_at = Utc::now();
        let next = serde_json::to_string(&wake.status).map_err(|error| error.to_string())?;
        let changed = transaction
            .execute(
                "UPDATE workflow_wakes
                 SET status = ?1, wake = ?2, updated_at = ?3
                 WHERE id = ?4 AND status = ?5",
                rusqlite::params![
                    next,
                    serde_json::to_string(&wake).map_err(|error| error.to_string())?,
                    wake.updated_at.to_rfc3339(),
                    wake_id,
                    previous,
                ],
            )
            .map_err(|error| format!("Failed to transition workflow wake: {error}"))?;
        transaction
            .commit()
            .map_err(|error| format!("Failed to commit workflow wake transition: {error}"))?;
        Ok(changed == 1)
    }

    async fn resolve_workflow_approval(
        &self,
        wake_id: &str,
        resolution: &ApprovalResolution,
    ) -> Result<ApprovalResolveOutcome, String> {
        let mut conn = self.connection()?;
        let transaction = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|error| format!("Failed to start approval transaction: {}", error))?;
        let serialized = transaction
            .query_row(
                "SELECT wake FROM workflow_wakes WHERE id = ?1",
                rusqlite::params![wake_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| format!("Failed to load approval wake: {}", error))?
            .ok_or_else(|| format!("Approval wake {} does not exist", wake_id))?;
        let mut wake: WorkflowWake = serde_json::from_str(&serialized)
            .map_err(|error| format!("Failed to deserialize approval wake: {}", error))?;
        if wake.kind != WorkflowWakeKind::Approval {
            return Err(format!("Wake {} is not an approval", wake_id));
        }

        if wake.payload.get("resolution").is_some() {
            return Ok(ApprovalResolveOutcome::AlreadyResolved { wake });
        }
        if wake.status != WorkflowWakeStatus::Waiting {
            return Err(format!("Approval wake {} is not waiting", wake_id));
        }

        wake.status = WorkflowWakeStatus::Ready;
        wake.updated_at = resolution.decided_at;
        wake.payload["resolution"] = serde_json::to_value(resolution)
            .map_err(|error| format!("Failed to serialize approval resolution: {}", error))?;
        let status = serde_json::to_string(&wake.status).map_err(|error| error.to_string())?;
        let waiting = serde_json::to_string(&WorkflowWakeStatus::Waiting)
            .map_err(|error| error.to_string())?;
        let changed = transaction
            .execute(
                "UPDATE workflow_wakes
                 SET status = ?1, wake = ?2, updated_at = ?3
                 WHERE id = ?4 AND status = ?5",
                rusqlite::params![
                    status,
                    serde_json::to_string(&wake).map_err(|error| error.to_string())?,
                    wake.updated_at.to_rfc3339(),
                    wake_id,
                    waiting,
                ],
            )
            .map_err(|error| format!("Failed to resolve approval wake: {}", error))?;
        if changed != 1 {
            return Err(format!("Approval wake {} changed concurrently", wake_id));
        }
        transaction
            .commit()
            .map_err(|error| format!("Failed to commit approval decision: {}", error))?;
        Ok(ApprovalResolveOutcome::Applied { wake })
    }

    async fn acquire_resource_lease(
        &self,
        lease: &WorkflowResourceLease,
        now: DateTime<Utc>,
    ) -> Result<bool, String> {
        if lease.expires_at <= now {
            return Ok(false);
        }
        let mut conn = self.connection()?;
        let transaction = conn
            .transaction()
            .map_err(|error| format!("Failed to start lease transaction: {}", error))?;
        transaction
            .execute(
                "DELETE FROM workflow_resource_leases WHERE expires_at <= ?1",
                rusqlite::params![now.to_rfc3339()],
            )
            .map_err(|error| format!("Failed to expire resource leases: {}", error))?;
        let inserted = transaction
            .execute(
                "INSERT OR IGNORE INTO workflow_resource_leases
                    (resource_key, owner_run_id, owner_node_id, lease_token, expires_at, lease, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    &lease.resource_key,
                    &lease.owner_run_id,
                    &lease.owner_node_id,
                    &lease.lease_token,
                    lease.expires_at.to_rfc3339(),
                    serde_json::to_string(lease).map_err(|error| error.to_string())?,
                    lease.updated_at.to_rfc3339(),
                ],
            )
            .map_err(|error| format!("Failed to acquire resource lease: {}", error))?;
        transaction
            .commit()
            .map_err(|error| format!("Failed to commit resource lease: {}", error))?;
        Ok(inserted == 1)
    }

    async fn renew_resource_lease(
        &self,
        resource_key: &str,
        lease_token: &str,
        expires_at: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Result<bool, String> {
        let conn = self.connection()?;
        let updated = conn
            .execute(
                "UPDATE workflow_resource_leases
                 SET expires_at = ?3, updated_at = ?4
                 WHERE resource_key = ?1 AND lease_token = ?2 AND expires_at > ?4",
                rusqlite::params![
                    resource_key,
                    lease_token,
                    expires_at.to_rfc3339(),
                    now.to_rfc3339(),
                ],
            )
            .map_err(|error| format!("Failed to renew resource lease: {}", error))?;
        Ok(updated == 1)
    }

    async fn release_resource_lease(
        &self,
        resource_key: &str,
        lease_token: &str,
    ) -> Result<bool, String> {
        let conn = self.connection()?;
        let deleted = conn
            .execute(
                "DELETE FROM workflow_resource_leases
                 WHERE resource_key = ?1 AND lease_token = ?2",
                rusqlite::params![resource_key, lease_token],
            )
            .map_err(|error| format!("Failed to release resource lease: {}", error))?;
        Ok(deleted == 1)
    }
}

/// Async boundary for the synchronous Diesel/rusqlite backend.
///
/// Every trait call owns bounded admission and worker permits for the complete
/// database operation. The backend future is driven on Tokio's blocking pool,
/// so SQLite busy waits, serialization and whole transactions never occupy an
/// async worker thread.
#[async_trait]
impl FlowStorage for HybridFlowStorage {
    async fn observed_stream_values(
        &self,
    ) -> Result<Vec<crate::services::streaming::stream_values::ObservedStreamValue>, String> {
        self.run_database_work("read observed stream values", |backend| async move {
            backend.observed_stream_values().await
        })
        .await
    }
    async fn save_flow(&self, flow: &FlowDefinition) -> Result<(), String> {
        let flow = flow.clone();
        self.run_database_work("save flow", move |backend| async move {
            backend.save_flow(&flow).await
        })
        .await
    }

    async fn replace_flow_at_revision(
        &self,
        expected_revision: &str,
        flow: &FlowDefinition,
    ) -> Result<FlowDefinition, String> {
        let expected_revision = expected_revision.to_string();
        let flow = flow.clone();
        self.run_database_work("replace flow", move |backend| async move {
            backend
                .replace_flow_at_revision(&expected_revision, &flow)
                .await
        })
        .await
    }

    async fn load_flow(&self, flow_id: &str) -> Result<FlowDefinition, String> {
        let flow_id = flow_id.to_string();
        self.run_database_work("load flow", move |backend| async move {
            backend.load_flow(&flow_id).await
        })
        .await
    }

    async fn save_execution_state(&self, state: &FlowExecutionState) -> Result<(), String> {
        let state = state.clone();
        self.run_database_work("save execution state", move |backend| async move {
            backend.save_execution_state(&state).await
        })
        .await
    }

    async fn load_execution_state(&self, flow_id: &str) -> Result<FlowExecutionState, String> {
        let flow_id = flow_id.to_string();
        self.run_database_work("load execution state", move |backend| async move {
            backend.load_execution_state(&flow_id).await
        })
        .await
    }

    async fn list_recoverable_execution_states(&self) -> Result<Vec<FlowExecutionState>, String> {
        self.run_database_work("list recoverable executions", move |backend| async move {
            backend.list_recoverable_execution_states().await
        })
        .await
    }

    async fn finalize_execution(
        &self,
        history_entry: &ExecutionHistoryEntry,
    ) -> Result<(), String> {
        let history_entry = history_entry.clone();
        let result = self
            .run_database_work("finalize execution", move |backend| async move {
                backend.finalize_execution(&history_entry).await
            })
            .await;
        if result.is_ok() {
            self.schedule_history_maintenance();
        }
        result
    }

    async fn list_execution_history(
        &self,
        flow_id: Option<&str>,
        limit: u32,
    ) -> Result<Vec<ExecutionHistoryEntry>, String> {
        let flow_id = flow_id.map(str::to_string);
        self.run_database_work("list execution history", move |backend| async move {
            backend
                .list_execution_history(flow_id.as_deref(), limit)
                .await
        })
        .await
    }

    async fn list_flows(&self) -> Result<Vec<FlowSummary>, String> {
        self.run_database_work("list flows", move |backend| async move {
            backend.list_flows().await
        })
        .await
    }

    async fn delete_flow(&self, flow_id: &str) -> Result<(), String> {
        let flow_id = flow_id.to_string();
        self.run_database_work("delete flow", move |backend| async move {
            backend.delete_flow(&flow_id).await
        })
        .await
    }

    async fn archive_flow(&self, flow_id: &str) -> Result<(), String> {
        let flow_id = flow_id.to_string();
        self.run_database_work("archive flow", move |backend| async move {
            backend.archive_flow(&flow_id).await
        })
        .await
    }

    async fn cleanup_execution_history(
        &self,
        retention_days: u32,
        max_size_mb: u32,
    ) -> Result<usize, String> {
        self.run_database_work("clean execution history", move |backend| async move {
            backend
                .cleanup_execution_history(retention_days, max_size_mb)
                .await
        })
        .await
    }

    async fn get_database_stats(&self) -> Result<DatabaseStats, String> {
        self.run_database_work("read database statistics", move |backend| async move {
            backend.get_database_stats().await
        })
        .await
    }

    async fn vacuum_database(&self) -> Result<(), String> {
        self.run_database_work("vacuum workflow database", move |backend| async move {
            backend.vacuum_database().await
        })
        .await
    }

    async fn save_trigger_state(
        &self,
        flow_id: &str,
        node_id: &str,
        key: &str,
        value: Value,
    ) -> Result<(), String> {
        let (flow_id, node_id, key) = (flow_id.to_string(), node_id.to_string(), key.to_string());
        self.run_database_work("save trigger state", move |backend| async move {
            backend
                .save_trigger_state(&flow_id, &node_id, &key, value)
                .await
        })
        .await
    }

    async fn load_trigger_state(
        &self,
        flow_id: &str,
        node_id: &str,
        key: &str,
    ) -> Result<Option<Value>, String> {
        let (flow_id, node_id, key) = (flow_id.to_string(), node_id.to_string(), key.to_string());
        self.run_database_work("load trigger state", move |backend| async move {
            backend.load_trigger_state(&flow_id, &node_id, &key).await
        })
        .await
    }

    async fn delete_trigger_state(&self, flow_id: &str, node_id: &str) -> Result<(), String> {
        let (flow_id, node_id) = (flow_id.to_string(), node_id.to_string());
        self.run_database_work("delete trigger state", move |backend| async move {
            backend.delete_trigger_state(&flow_id, &node_id).await
        })
        .await
    }

    async fn claim_trigger_dedup(
        &self,
        dedup_key: &str,
        event_id: Option<&str>,
        occurred_at: DateTime<Utc>,
    ) -> Result<bool, String> {
        let dedup_key = dedup_key.to_string();
        let event_id = event_id.map(str::to_string);
        self.run_database_work("claim trigger deduplication", move |backend| async move {
            backend
                .claim_trigger_dedup(&dedup_key, event_id.as_deref(), occurred_at)
                .await
        })
        .await
    }

    async fn replace_active_triggers(
        &self,
        flow_id: &str,
        triggers: &[ActiveTriggerDefinition],
    ) -> Result<(), String> {
        let flow_id = flow_id.to_string();
        let triggers = triggers.to_vec();
        self.run_database_work("replace active triggers", move |backend| async move {
            backend.replace_active_triggers(&flow_id, &triggers).await
        })
        .await
    }

    async fn remove_active_triggers(&self, flow_id: &str) -> Result<(), String> {
        let flow_id = flow_id.to_string();
        self.run_database_work("remove active triggers", move |backend| async move {
            backend.remove_active_triggers(&flow_id).await
        })
        .await
    }

    async fn clear_active_triggers(&self) -> Result<(), String> {
        self.run_database_work("clear active triggers", move |backend| async move {
            backend.clear_active_triggers().await
        })
        .await
    }

    async fn list_active_triggers(&self) -> Result<Vec<ActiveTriggerDefinition>, String> {
        self.run_database_work("list active triggers", move |backend| async move {
            backend.list_active_triggers().await
        })
        .await
    }

    async fn store_event_envelope(
        &self,
        envelope: &crate::events::EventEnvelope,
    ) -> Result<bool, String> {
        let envelope = envelope.clone();
        self.run_database_work("store event envelope", move |backend| async move {
            backend.store_event_envelope(&envelope).await
        })
        .await
    }

    async fn load_event_envelope(
        &self,
        event_id: &str,
    ) -> Result<Option<crate::events::EventEnvelope>, String> {
        let event_id = event_id.to_string();
        self.run_database_work("load event envelope", move |backend| async move {
            backend.load_event_envelope(&event_id).await
        })
        .await
    }

    async fn trace_workflow_event(
        &self,
        event_id: &str,
    ) -> Result<Option<crate::events::WorkflowRuntimeTrace>, String> {
        let event_id = event_id.to_string();
        self.run_database_work("trace workflow event", move |backend| async move {
            backend.trace_workflow_event(&event_id).await
        })
        .await
    }

    async fn list_event_envelopes(
        &self,
        event_type: Option<&str>,
        since: Option<DateTime<Utc>>,
        limit: u32,
    ) -> Result<Vec<crate::events::EventEnvelope>, String> {
        let event_type = event_type.map(str::to_string);
        self.run_database_work("list event envelopes", move |backend| async move {
            backend
                .list_event_envelopes(event_type.as_deref(), since, limit)
                .await
        })
        .await
    }

    async fn save_overlay_command_delivery(
        &self,
        delivery: &crate::events::OverlayCommandDelivery,
    ) -> Result<(), String> {
        let delivery = delivery.clone();
        self.run_database_work("save overlay delivery", move |backend| async move {
            backend.save_overlay_command_delivery(&delivery).await
        })
        .await
    }

    async fn resolve_overlay_command_delivery(
        &self,
        acknowledgement: &crate::events::OverlayCommandAcknowledgement,
    ) -> Result<bool, String> {
        let acknowledgement = acknowledgement.clone();
        self.run_database_work("resolve overlay delivery", move |backend| async move {
            backend
                .resolve_overlay_command_delivery(&acknowledgement)
                .await
        })
        .await
    }

    async fn expire_overlay_command_delivery(&self, command_id: &str) -> Result<bool, String> {
        let command_id = command_id.to_string();
        self.run_database_work("expire overlay delivery", move |backend| async move {
            backend.expire_overlay_command_delivery(&command_id).await
        })
        .await
    }

    async fn expire_pending_overlay_command_deliveries_for_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<String>, String> {
        let run_id = run_id.to_string();
        self.run_database_work("expire run overlay deliveries", move |backend| async move {
            backend
                .expire_pending_overlay_command_deliveries_for_run(&run_id)
                .await
        })
        .await
    }

    async fn list_pending_overlay_command_deliveries(
        &self,
    ) -> Result<Vec<crate::events::OverlayCommandDelivery>, String> {
        self.run_database_work(
            "list pending overlay deliveries",
            move |backend| async move { backend.list_pending_overlay_command_deliveries().await },
        )
        .await
    }

    async fn list_overlay_command_deliveries(
        &self,
        run_id: &str,
    ) -> Result<Vec<crate::events::OverlayCommandDelivery>, String> {
        let run_id = run_id.to_string();
        self.run_database_work("list overlay deliveries", move |backend| async move {
            backend.list_overlay_command_deliveries(&run_id).await
        })
        .await
    }

    async fn load_overlay_command_delivery(
        &self,
        command_id: &str,
    ) -> Result<Option<crate::events::OverlayCommandDelivery>, String> {
        let command_id = command_id.to_string();
        self.run_database_work("load overlay delivery", move |backend| async move {
            backend.load_overlay_command_delivery(&command_id).await
        })
        .await
    }

    async fn claim_side_effect_outbox(
        &self,
        entry: &crate::events::SideEffectOutboxEntry,
    ) -> Result<bool, String> {
        let entry = entry.clone();
        self.run_database_work("claim side-effect outbox", move |backend| async move {
            backend.claim_side_effect_outbox(&entry).await
        })
        .await
    }

    async fn replace_side_effect_outbox_attempt(
        &self,
        entry: &crate::events::SideEffectOutboxEntry,
    ) -> Result<(), String> {
        let entry = entry.clone();
        self.run_database_work("replace side-effect attempt", move |backend| async move {
            backend.replace_side_effect_outbox_attempt(&entry).await
        })
        .await
    }

    async fn complete_side_effect_outbox(
        &self,
        key: &str,
        status: crate::events::SideEffectOutboxStatus,
        error: Option<String>,
    ) -> Result<(), String> {
        let key = key.to_string();
        self.run_database_work("complete side-effect outbox", move |backend| async move {
            backend
                .complete_side_effect_outbox(&key, status, error)
                .await
        })
        .await
    }

    async fn list_side_effect_outbox(
        &self,
    ) -> Result<Vec<crate::events::SideEffectOutboxEntry>, String> {
        self.run_database_work("list side-effect outbox", move |backend| async move {
            backend.list_side_effect_outbox().await
        })
        .await
    }

    async fn load_side_effect_outbox(
        &self,
        key: &str,
    ) -> Result<Option<crate::events::SideEffectOutboxEntry>, String> {
        let key = key.to_string();
        self.run_database_work("load side-effect outbox", move |backend| async move {
            backend.load_side_effect_outbox(&key).await
        })
        .await
    }

    async fn workflow_runtime_metrics(
        &self,
    ) -> Result<crate::events::WorkflowRuntimeMetrics, String> {
        self.run_database_work("read workflow runtime metrics", move |backend| async move {
            backend.workflow_runtime_metrics().await
        })
        .await
    }

    async fn save_workflow_dead_letter(
        &self,
        record: &crate::events::WorkflowDeadLetter,
    ) -> Result<(), String> {
        let record = record.clone();
        self.run_database_work("save workflow dead letter", move |backend| async move {
            backend.save_workflow_dead_letter(&record).await
        })
        .await
    }

    async fn load_workflow_dead_letter(
        &self,
        id: &str,
    ) -> Result<Option<crate::events::WorkflowDeadLetter>, String> {
        let id = id.to_string();
        self.run_database_work("load workflow dead letter", move |backend| async move {
            backend.load_workflow_dead_letter(&id).await
        })
        .await
    }

    async fn list_workflow_dead_letters(
        &self,
    ) -> Result<Vec<crate::events::WorkflowDeadLetter>, String> {
        self.run_database_work("list workflow dead letters", move |backend| async move {
            backend.list_workflow_dead_letters().await
        })
        .await
    }

    async fn save_workflow_wake(&self, wake: &WorkflowWake) -> Result<(), String> {
        let wake = wake.clone();
        self.run_database_work("save workflow wake", move |backend| async move {
            backend.save_workflow_wake(&wake).await
        })
        .await
    }

    async fn load_workflow_wake(&self, wake_id: &str) -> Result<Option<WorkflowWake>, String> {
        let wake_id = wake_id.to_string();
        self.run_database_work("load workflow wake", move |backend| async move {
            backend.load_workflow_wake(&wake_id).await
        })
        .await
    }

    async fn list_open_workflow_wakes(
        &self,
        run_id: Option<&str>,
    ) -> Result<Vec<WorkflowWake>, String> {
        let run_id = run_id.map(str::to_string);
        self.run_database_work("list workflow wakes", move |backend| async move {
            backend.list_open_workflow_wakes(run_id.as_deref()).await
        })
        .await
    }

    async fn set_workflow_wake_status(
        &self,
        wake_id: &str,
        status: WorkflowWakeStatus,
    ) -> Result<bool, String> {
        let wake_id = wake_id.to_string();
        self.run_database_work("transition workflow wake", move |backend| async move {
            backend.set_workflow_wake_status(&wake_id, status).await
        })
        .await
    }

    async fn resolve_workflow_approval(
        &self,
        wake_id: &str,
        resolution: &ApprovalResolution,
    ) -> Result<ApprovalResolveOutcome, String> {
        let wake_id = wake_id.to_string();
        let resolution = resolution.clone();
        self.run_database_work("resolve workflow approval", move |backend| async move {
            backend
                .resolve_workflow_approval(&wake_id, &resolution)
                .await
        })
        .await
    }

    async fn acquire_resource_lease(
        &self,
        lease: &WorkflowResourceLease,
        now: DateTime<Utc>,
    ) -> Result<bool, String> {
        let lease = lease.clone();
        self.run_database_work("acquire workflow lease", move |backend| async move {
            backend.acquire_resource_lease(&lease, now).await
        })
        .await
    }

    async fn renew_resource_lease(
        &self,
        resource_key: &str,
        lease_token: &str,
        expires_at: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Result<bool, String> {
        let (resource_key, lease_token) = (resource_key.to_string(), lease_token.to_string());
        self.run_database_work("renew workflow lease", move |backend| async move {
            backend
                .renew_resource_lease(&resource_key, &lease_token, expires_at, now)
                .await
        })
        .await
    }

    async fn release_resource_lease(
        &self,
        resource_key: &str,
        lease_token: &str,
    ) -> Result<bool, String> {
        let (resource_key, lease_token) = (resource_key.to_string(), lease_token.to_string());
        self.run_database_work("release workflow lease", move |backend| async move {
            backend
                .release_resource_lease(&resource_key, &lease_token)
                .await
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{EventEnvelope, SideEffectOutboxEntry, SideEffectOutboxStatus};
    use crate::services::flow::executor::{FlowMetadata, RuntimeVariables};
    use diesel::r2d2::{ConnectionManager, Pool};
    use diesel::SqliteConnection;
    use std::collections::{HashMap, HashSet};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};
    use tempfile::TempDir;

    fn test_storage_with_limit(limit: usize) -> (TempDir, HybridFlowStorage, String) {
        test_storage_with_limits(limit, limit.saturating_mul(16).max(1))
    }

    fn test_storage_with_limits(
        worker_limit: usize,
        admitted_limit: usize,
    ) -> (TempDir, HybridFlowStorage, String) {
        let directory = tempfile::tempdir().expect("temporary workflow directory");
        let runtime_path = directory.path().join("runtime.db");
        let manager = ConnectionManager::<SqliteConnection>::new(":memory:");
        let pool = Pool::builder()
            .max_size(1)
            .build(manager)
            .expect("flow definition pool");
        let runtime_path = runtime_path.to_string_lossy().to_string();
        let storage = HybridFlowStorage::new_with_work_limits(
            pool,
            &runtime_path,
            worker_limit,
            admitted_limit,
        )
        .expect("workflow runtime storage");
        (directory, storage, runtime_path)
    }

    fn event_envelope(event_id: &str) -> EventEnvelope {
        let now = Utc::now();
        EventEnvelope {
            event_id: event_id.to_string(),
            event_type: "test.database".to_string(),
            source: "f06".to_string(),
            schema_version: 1,
            occurred_at: now,
            received_at: now,
            account_id: None,
            session_id: None,
            stream_id: None,
            correlation_id: None,
            causation_id: None,
            idempotency_key: Some(event_id.to_string()),
            payload: serde_json::json!({ "eventId": event_id }),
        }
    }

    fn hold_write_lock(path: String, duration: Duration) -> std::thread::JoinHandle<()> {
        let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel(1);
        let writer = std::thread::spawn(move || {
            let conn = Connection::open(path).expect("contention connection");
            conn.busy_timeout(Duration::from_secs(1))
                .expect("contention busy timeout");
            conn.execute_batch("BEGIN IMMEDIATE")
                .expect("acquire SQLite write lock");
            ready_tx.send(()).expect("announce SQLite write lock");
            std::thread::sleep(duration);
            conn.execute_batch("COMMIT")
                .expect("release SQLite write lock");
        });
        ready_rx.recv().expect("wait for SQLite write lock");
        writer
    }

    #[tokio::test]
    async fn observed_values_commit_with_acceptance_and_survive_reopen_without_double_counts() {
        let (_directory, storage, path) = test_storage_with_limit(2);
        let mut first = event_envelope("follow-a");
        first.event_type = "stream.follow".into();
        first.source = "twitch".into();
        first.account_id = Some(7);
        first.payload["occurredAt"] = serde_json::json!(first.occurred_at.to_rfc3339());
        assert!(storage.store_event_envelope(&first).await.unwrap());
        assert!(!storage.store_event_envelope(&first).await.unwrap());
        let mut bits = event_envelope("bits-a");
        bits.event_type = "stream.bits".into();
        bits.source = "twitch".into();
        bits.account_id = Some(7);
        bits.payload = serde_json::json!({"amount":500,"occurredAt":bits.occurred_at.to_rfc3339()});
        assert!(storage.store_event_envelope(&bits).await.unwrap());
        let mut expired = first.clone();
        expired.event_id = "expired-follow".into();
        expired.occurred_at -= chrono::Duration::minutes(6);
        assert!(storage.store_event_envelope(&expired).await.unwrap());
        let conn = Connection::open(&path).unwrap();
        conn.execute("DELETE FROM workflow_event_inbox", [])
            .unwrap();
        let reopened = HybridFlowStorage::new(storage.backend.db_pool.clone(), &path).unwrap();
        let values = reopened.observed_stream_values().await.unwrap();
        assert_eq!(values.len(), 2);
        assert_eq!(
            values
                .iter()
                .find(|value| value.metric == "followers")
                .unwrap()
                .total,
            1
        );
        assert_eq!(
            values
                .iter()
                .find(|value| value.metric == "bits")
                .unwrap()
                .total,
            500
        );
        // A projection failure rolls back admission too: a caller can repair and
        // retry without leaving a permanently accepted-but-uncounted envelope.
        conn.execute_batch("CREATE TRIGGER fail_projection BEFORE UPDATE ON observed_stream_values BEGIN SELECT RAISE(ABORT,'projection unavailable'); END;").unwrap();
        let mut next = first.clone();
        next.event_id = "follow-b".into();
        next.idempotency_key = Some("follow-b".into());
        assert!(reopened.store_event_envelope(&next).await.is_err());
        assert!(reopened
            .load_event_envelope("follow-b")
            .await
            .unwrap()
            .is_none());
        conn.execute_batch("DROP TRIGGER fail_projection").unwrap();
        assert!(reopened.store_event_envelope(&next).await.unwrap());
        assert_eq!(
            reopened
                .observed_stream_values()
                .await
                .unwrap()
                .iter()
                .find(|value| value.metric == "followers")
                .unwrap()
                .total,
            2
        );
    }

    fn create_runtime_tables(conn: &Connection) {
        conn.execute_batch(
            "CREATE TABLE execution_states (
                run_id TEXT PRIMARY KEY,
                state TEXT NOT NULL,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
             );
             CREATE TABLE execution_history (
                id TEXT PRIMARY KEY,
                run_id TEXT,
                flow_id TEXT NOT NULL,
                flow_name TEXT NOT NULL,
                status TEXT NOT NULL,
                started_at DATETIME NOT NULL,
                completed_at DATETIME,
                duration_ms INTEGER,
                reachable_node_count INTEGER NOT NULL,
                completed_nodes INTEGER NOT NULL,
                failed_nodes INTEGER NOT NULL,
                error_message TEXT,
                final_results TEXT,
                node_execution_details TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
             );",
        )
        .unwrap();
    }

    fn running_state(run_id: &str, flow_id: &str, started_at: DateTime<Utc>) -> FlowExecutionState {
        FlowExecutionState {
            run_id: run_id.to_string(),
            flow_id: flow_id.to_string(),
            flow_definition: FlowDefinition {
                id: flow_id.to_string(),
                name: flow_id.to_string(),
                version: "1".to_string(),
                description: None,
                created_at: started_at,
                updated_at: started_at,
                created_by: None,
                tags: Vec::new(),
                nodes: Vec::new(),
                connections: Vec::new(),
                start_node_id: String::new(),
                metadata: FlowMetadata::default(),
                is_active: false,
            },
            status: FlowStatus::Running,
            current_node: None,
            completed_nodes: HashSet::new(),
            failed_nodes: HashSet::new(),
            node_results: HashMap::new(),
            execution_log: Vec::new(),
            started_at,
            completed_at: None,
            recovery_detected_at: None,
            pending_nodes: Vec::new(),
            sorted_node_results: Vec::new(),
            runtime_variables: RuntimeVariables::default(),
            approved_effect_bindings: HashMap::new(),
            lineage_tracker: None,
            lineage: None,
        }
    }

    #[test]
    fn legacy_execution_state_key_is_migrated_from_flow_id_to_run_id() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE execution_states (
                flow_id TEXT PRIMARY KEY,
                state TEXT NOT NULL,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
             );
             INSERT INTO execution_states (flow_id, state) VALUES ('run-1', '{}');",
        )
        .unwrap();

        HybridFlowStorageBackend::ensure_execution_state_run_id(&conn).unwrap();

        let columns = conn
            .prepare("PRAGMA table_info(execution_states)")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert!(columns.iter().any(|column| column == "run_id"));
        assert!(!columns.iter().any(|column| column == "flow_id"));
        let run_id: String = conn
            .query_row("SELECT run_id FROM execution_states", [], |row| row.get(0))
            .unwrap();
        assert_eq!(run_id, "run-1");
    }

    #[test]
    fn startup_reconciliation_removes_only_snapshots_with_exact_terminal_history() {
        let mut conn = Connection::open_in_memory().unwrap();
        create_runtime_tables(&conn);
        let completed_start = DateTime::parse_from_rfc3339("2026-03-17T19:00:00.2104274Z")
            .unwrap()
            .with_timezone(&Utc);
        let interrupted_start = DateTime::parse_from_rfc3339("2026-08-29T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let completed = running_state("completed-run", "display", completed_start);
        let interrupted = running_state("interrupted-run", "display", interrupted_start);
        for state in [&completed, &interrupted] {
            conn.execute(
                "INSERT INTO execution_states (run_id, state) VALUES (?1, ?2)",
                rusqlite::params![state.run_id, serde_json::to_string(state).unwrap()],
            )
            .unwrap();
        }
        conn.execute(
            "INSERT INTO execution_history
                (id, run_id, flow_id, flow_name, status, started_at,
                 reachable_node_count, completed_nodes, failed_nodes, node_execution_details)
             VALUES ('history-completed-run', 'completed-run', 'display', 'display', 'completed', ?1, 7, 7, 0, '[]')",
            rusqlite::params![completed_start.to_rfc3339()],
        )
        .unwrap();

        let removed =
            HybridFlowStorageBackend::migrate_terminal_execution_states(&mut conn).unwrap();

        assert_eq!(removed, 1);
        let remaining: String = conn
            .query_row("SELECT run_id FROM execution_states", [], |row| row.get(0))
            .unwrap();
        assert_eq!(remaining, "interrupted-run");
        assert_eq!(
            HybridFlowStorageBackend::migrate_terminal_execution_states(&mut conn).unwrap(),
            0,
            "the migration must be idempotent"
        );
    }

    #[test]
    fn terminal_finalization_archives_and_removes_snapshot_in_one_transaction() {
        let mut conn = Connection::open_in_memory().unwrap();
        create_runtime_tables(&conn);
        let started_at = Utc::now();
        let state = running_state("run-1", "flow-1", started_at);
        conn.execute(
            "INSERT INTO execution_states (run_id, state) VALUES (?1, ?2)",
            rusqlite::params![state.run_id, serde_json::to_string(&state).unwrap()],
        )
        .unwrap();
        let history = ExecutionHistoryEntry {
            id: "history-run-1".to_string(),
            run_id: "run-1".to_string(),
            flow_id: "flow-1".to_string(),
            flow_name: "flow-1".to_string(),
            status: FlowStatus::Completed,
            started_at,
            completed_at: Some(started_at),
            duration_ms: Some(0),
            reachable_node_count: 0,
            completed_nodes: 0,
            failed_nodes: 0,
            error_message: None,
            final_results: None,
            node_execution_details: Vec::new(),
        };

        let transaction = conn.transaction().unwrap();
        HybridFlowStorageBackend::insert_execution_history(&transaction, &history).unwrap();
        transaction
            .execute(
                "DELETE FROM execution_states WHERE run_id = ?1",
                rusqlite::params![history.run_id],
            )
            .unwrap();
        transaction.commit().unwrap();

        let states: i64 = conn
            .query_row("SELECT COUNT(*) FROM execution_states", [], |row| {
                row.get(0)
            })
            .unwrap();
        let histories: i64 = conn
            .query_row("SELECT COUNT(*) FROM execution_history", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!((states, histories), (0, 1));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn bounded_database_work_keeps_the_async_worker_responsive_under_lock_contention() {
        let (_directory, storage, runtime_path) = test_storage_with_limit(2);

        let baseline_writer = hold_write_lock(runtime_path.clone(), Duration::from_millis(260));
        let baseline_started = Instant::now();
        let baseline_heartbeat = tokio::spawn(async {
            tokio::time::sleep(Duration::from_millis(25)).await;
            Instant::now()
        });
        storage
            .backend
            .store_event_envelope(&event_envelope("baseline-blocking"))
            .await
            .expect("raw synchronous baseline eventually stores event");
        let baseline_delay = baseline_heartbeat
            .await
            .expect("baseline heartbeat")
            .duration_since(baseline_started);
        baseline_writer.join().expect("baseline writer");

        let bounded_writer = hold_write_lock(runtime_path, Duration::from_millis(260));
        let bounded_started = Instant::now();
        let bounded_heartbeat = tokio::spawn(async {
            tokio::time::sleep(Duration::from_millis(25)).await;
            Instant::now()
        });
        storage
            .store_event_envelope(&event_envelope("bounded-worker"))
            .await
            .expect("bounded worker eventually stores event");
        let bounded_delay = bounded_heartbeat
            .await
            .expect("bounded heartbeat")
            .duration_since(bounded_started);
        bounded_writer.join().expect("bounded writer");

        println!(
            "F06 heartbeat under a 260ms write lock: direct={}ms, bounded={}ms",
            baseline_delay.as_millis(),
            bounded_delay.as_millis()
        );
        assert!(
            baseline_delay >= Duration::from_millis(180),
            "the direct baseline should reproduce async-worker starvation: {baseline_delay:?}"
        );
        assert!(
            bounded_delay < Duration::from_millis(120),
            "the bounded adapter should keep timers responsive: {bounded_delay:?}"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn database_work_is_bounded_and_cancellation_does_not_release_capacity_early() {
        let (_directory, storage, runtime_path) = test_storage_with_limit(1);
        let writer = hold_write_lock(runtime_path.clone(), Duration::from_millis(420));

        let first_storage = storage.clone();
        let first = tokio::spawn(async move {
            first_storage
                .store_event_envelope(&event_envelope("cancelled-caller"))
                .await
        });
        tokio::time::timeout(Duration::from_secs(1), async {
            while storage.work_permits.available_permits() != 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("first database worker acquired the only permit");
        first.abort();

        let second_storage = storage.clone();
        let mut second = tokio::spawn(async move {
            second_storage
                .store_event_envelope(&event_envelope("queued-after-cancel"))
                .await
        });
        assert!(
            tokio::time::timeout(Duration::from_millis(120), &mut second)
                .await
                .is_err(),
            "aborting the caller must not free capacity while its SQLite worker still runs"
        );
        second
            .await
            .expect("queued database task")
            .expect("queued database operation");
        writer.join().expect("contention writer");

        let conn = Connection::open(runtime_path).expect("verification connection");
        let stored: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM workflow_event_inbox WHERE event_id IN (?1, ?2)",
                rusqlite::params!["cancelled-caller", "queued-after-cancel"],
                |row| row.get(0),
            )
            .expect("count persisted events");
        assert_eq!(stored, 2, "the detached worker must finish truthfully");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn database_work_queue_limits_concurrent_blocking_workers() {
        let (_directory, storage, _runtime_path) = test_storage_with_limit(2);
        let active = Arc::new(AtomicUsize::new(0));
        let maximum = Arc::new(AtomicUsize::new(0));
        let mut jobs = Vec::new();

        for _ in 0..8 {
            let storage = storage.clone();
            let active = active.clone();
            let maximum = maximum.clone();
            jobs.push(tokio::spawn(async move {
                storage
                    .run_database_work("F06 bounded worker probe", move |_backend| async move {
                        let now_active = active.fetch_add(1, Ordering::SeqCst) + 1;
                        maximum.fetch_max(now_active, Ordering::SeqCst);
                        std::thread::sleep(Duration::from_millis(40));
                        active.fetch_sub(1, Ordering::SeqCst);
                        Ok(())
                    })
                    .await
            }));
        }
        for job in jobs {
            job.await
                .expect("bounded worker task")
                .expect("worker probe");
        }
        assert_eq!(maximum.load(Ordering::SeqCst), 2);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn full_database_queue_rejects_new_work_before_spawning_another_worker() {
        let (_directory, storage, _runtime_path) = test_storage_with_limits(1, 2);
        let active = Arc::new(AtomicUsize::new(0));

        let first_storage = storage.clone();
        let first_active = active.clone();
        let first = tokio::spawn(async move {
            first_storage
                .run_database_work("first queue probe", move |_backend| async move {
                    first_active.fetch_add(1, Ordering::SeqCst);
                    std::thread::sleep(Duration::from_millis(180));
                    first_active.fetch_sub(1, Ordering::SeqCst);
                    Ok(())
                })
                .await
        });
        tokio::time::timeout(Duration::from_secs(1), async {
            while active.load(Ordering::SeqCst) == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("first queue probe started");

        let second_storage = storage.clone();
        let second = tokio::spawn(async move {
            second_storage
                .run_database_work("second queue probe", move |_backend| async move { Ok(()) })
                .await
        });
        tokio::time::timeout(Duration::from_secs(1), async {
            while storage.admitted_work_permits.available_permits() != 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("second queue probe admitted");

        let error = storage
            .run_database_work(
                "overflow queue probe",
                move |_backend| async move { Ok(()) },
            )
            .await
            .expect_err("the finite database queue must reject overflow");
        assert!(error.contains("queue is full"));

        first.await.expect("first queue task").expect("first probe");
        second
            .await
            .expect("second queue task")
            .expect("second probe");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn ingress_outbox_and_history_share_the_bounded_database_boundary() {
        let (_directory, storage, _runtime_path) = test_storage_with_limit(3);
        let now = Utc::now();
        let event = event_envelope("representative-ingress");
        let outbox = SideEffectOutboxEntry {
            idempotency_key: "representative-action".to_string(),
            invocation_id: None,
            run_id: "run-f06".to_string(),
            node_id: "node-f06".to_string(),
            action: serde_json::json!({ "type": "test" }),
            status: SideEffectOutboxStatus::Dispatching,
            created_at: now,
            updated_at: now,
            error: None,
        };
        let started = Instant::now();
        let (ingress, action, history) = tokio::join!(
            storage.store_event_envelope(&event),
            storage.claim_side_effect_outbox(&outbox),
            storage.list_execution_history(None, 50),
        );
        let elapsed = started.elapsed();
        assert!(ingress.expect("ingress write"));
        assert!(action.expect("outbox claim"));
        assert!(history.expect("history query").is_empty());
        println!(
            "F06 concurrent ingress/outbox/history completed in {}ms",
            elapsed.as_millis()
        );
    }
}
