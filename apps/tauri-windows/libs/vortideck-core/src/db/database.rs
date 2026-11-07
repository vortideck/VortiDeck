/// Database access layer - Platform-agnostic database operations
///
/// This module provides the core database functionality that can be used
/// across different platforms (Tauri, CLI, Server, etc.)
///
/// Migration Status: IN PROGRESS
/// - This is a new module being created to centralize database logic
/// - Functions are being moved from src-tauri/db/utils.rs
/// - Goal: Make vortideck-core fully self-contained
use super::models::*;
use super::schema::*;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use std::sync::Arc;

pub type DbPool = Pool<ConnectionManager<SqliteConnection>>;

/// Validate a creator template edit against its current contract while the caller
/// holds the same SQLite write transaction. Execution never repairs invalid edits.
pub fn validate_saved_input_contract(
    connection: &mut SqliteConnection,
    reference: &crate::services::execution::ActionReference,
    template: &crate::services::execution::UniversalAction,
) -> Result<(), String> {
    let raw = action_input_contracts::table
        .find(reference.storage_key()?)
        .select(action_input_contracts::contract)
        .first::<String>(connection)
        .optional()
        .map_err(|_| "Could not read the saved action input contract")?;
    if let Some(raw) = raw {
        let contract: Option<crate::services::execution::input_contract::ActionInputContract> =
            serde_json::from_str(&raw).map_err(|_| "Invalid saved action input contract")?;
        if let Some(contract) = contract {
            contract.validate_for_template(template)?;
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteCommandClaim {
    Accepted(RemoteCommandReceiptRecord),
    Duplicate(RemoteCommandReceiptRecord),
}

/// Database access struct - provides all database operations
///
/// This struct wraps a connection pool and provides methods for all database operations.
/// It can be cloned cheaply (Arc-wrapped pool) and shared across threads.
#[derive(Debug)]
pub struct Database {
    pool: DbPool,
}

impl Database {
    pub async fn get_action_input_contract(
        &self,
        key: &str,
    ) -> Result<Option<(i64, String)>, String> {
        let pool = self.pool.clone();
        let key = key.to_string();
        tokio::task::spawn_blocking(move || {
            let mut connection = pool
                .get()
                .map_err(|_| "Action input storage unavailable".to_string())?;
            action_input_contracts::table
                .find(key)
                .select((
                    action_input_contracts::revision,
                    action_input_contracts::contract,
                ))
                .first::<(i64, String)>(&mut connection)
                .optional()
                .map_err(|_| "Action input contract could not be loaded".to_string())
        })
        .await
        .map_err(|_| "Action input loading interrupted".to_string())?
    }

    pub async fn save_action_input_contract(
        &self,
        key: &str,
        expected_revision: i64,
        contract: String,
    ) -> Result<i64, String> {
        let pool = self.pool.clone();
        let key = key.to_string();
        let revision = expected_revision
            .checked_add(1)
            .filter(|revision| *revision > 0)
            .ok_or("Invalid action input revision")?;
        tokio::task::spawn_blocking(move || {
            let mut connection = pool.get().map_err(|_| "Action input storage unavailable".to_string())?;
            connection.immediate_transaction::<i64, anyhow::Error, _>(|connection| {
            let reference: crate::services::execution::ActionReference = serde_json::from_str(&key)?;
            let declaration: Option<crate::services::execution::input_contract::ActionInputContract> = serde_json::from_str(&contract)?;
            if let Some(declaration) = &declaration {
                let template = match reference.kind {
                    crate::services::execution::ActionReferenceKind::HttpRequest => {
                        let record = http_requests::table.find(reference.id.parse::<i32>()?)
                            .select(HttpRequest::as_select()).first::<HttpRequest>(connection)?;
                        Some(crate::services::execution::action_converter::saved_http_request_template(&record).map_err(anyhow::Error::msg)?)
                    }
                    crate::services::execution::ActionReferenceKind::SavedAction => {
                        let record = saved_actions::table.filter(saved_actions::stable_id.eq(&reference.id))
                            .select(SavedActionBinding::as_select()).first::<SavedActionBinding>(connection)?;
                        Some(crate::services::execution::UniversalAction::Integration { integration_id: record.integration_id,
                            action_id: record.action, parameters: serde_json::from_str(&record.params)? })
                    }
                    _ => None,
                };
                if let Some(template) = template { declaration.validate_for_template(&template).map_err(anyhow::Error::msg)?; }
            }
            let updated_at = chrono::Utc::now().to_rfc3339();
            let count = if expected_revision == 0 {
                diesel::insert_into(action_input_contracts::table).values((
                    action_input_contracts::reference_key.eq(&key), action_input_contracts::revision.eq(revision),
                    action_input_contracts::contract.eq(&contract), action_input_contracts::updated_at.eq(&updated_at),
                )).on_conflict(action_input_contracts::reference_key).do_nothing().execute(connection)
            } else {
                diesel::update(action_input_contracts::table.find(&key)
                    .filter(action_input_contracts::revision.eq(expected_revision))).set((
                    action_input_contracts::revision.eq(revision), action_input_contracts::contract.eq(&contract),
                    action_input_contracts::updated_at.eq(&updated_at),
                )).execute(connection)
            }.map_err(|_| anyhow::anyhow!("Action input contract could not be saved"))?;
            anyhow::ensure!(count == 1, "Stale action input revision; reload before saving");
            Ok(revision)
            }).map_err(|error| error.to_string())
        }).await.map_err(|_| "Action input save interrupted".to_string())?
    }

    /// Create a new Database instance from a connection pool
    pub fn new(pool: &DbPool) -> Self {
        Database { pool: pool.clone() }
    }

    /// Atomically accept one consequential remote command identity. The
    /// caller must verify the command signature before invoking this method.
    /// A matching duplicate returns the original receipt; conflicting reuse
    /// of either the message ID or nonce fails closed.
    pub async fn claim_remote_command(
        &self,
        candidate: NewRemoteCommandReceipt<'_>,
    ) -> Result<RemoteCommandClaim, String> {
        use crate::db::schema::remote_command_receipts::dsl::*;

        let candidate_acceptance_id = candidate.acceptance_id.to_string();
        let candidate_peer = candidate.peer_client_id.to_string();
        let candidate_key = candidate.key_fingerprint.to_string();
        let candidate_message = candidate.message_id.to_string();
        let candidate_nonce = candidate.nonce.to_string();
        let candidate_digest = candidate.payload_digest.to_string();
        let mut conn = self
            .pool
            .get()
            .map_err(|error| format!("Failed to get DB connection: {error}"))?;

        conn.transaction::<RemoteCommandClaim, diesel::result::Error, _>(|conn| {
            let inserted = diesel::insert_or_ignore_into(remote_command_receipts)
                .values(candidate)
                .execute(conn)?;

            if inserted == 1 {
                let record = remote_command_receipts
                    .filter(acceptance_id.eq(&candidate_acceptance_id))
                    .select(RemoteCommandReceiptRecord::as_select())
                    .first(conn)?;
                return Ok(RemoteCommandClaim::Accepted(record));
            }

            let existing_message = remote_command_receipts
                .filter(peer_client_id.eq(&candidate_peer))
                .filter(key_fingerprint.eq(&candidate_key))
                .filter(message_id.eq(&candidate_message))
                .select(RemoteCommandReceiptRecord::as_select())
                .first(conn)
                .optional()?;
            if let Some(record) = existing_message {
                if record.payload_digest == candidate_digest {
                    return Ok(RemoteCommandClaim::Duplicate(record));
                }
                return Err(diesel::result::Error::RollbackTransaction);
            }

            let nonce_reused = remote_command_receipts
                .filter(peer_client_id.eq(&candidate_peer))
                .filter(key_fingerprint.eq(&candidate_key))
                .filter(nonce.eq(&candidate_nonce))
                .select(acceptance_id)
                .first::<String>(conn)
                .optional()?
                .is_some();
            if nonce_reused {
                return Err(diesel::result::Error::RollbackTransaction);
            }

            Err(diesel::result::Error::NotFound)
        })
        .map_err(|error| match error {
            diesel::result::Error::RollbackTransaction => {
                "Remote command message ID or nonce was reused with different signed content"
                    .to_string()
            }
            diesel::result::Error::NotFound => {
                "Remote command acceptance conflict could not be resolved safely".to_string()
            }
            other => format!("Failed to persist remote command acceptance: {other}"),
        })
    }

    // ========================================================================
    // CANONICAL ACTION INVOCATION HISTORY
    // ========================================================================

    // ========================================================================
    // DURABLE OVERLAY ALERT / MEDIA QUEUE
    // ========================================================================

    pub async fn enqueue_overlay_media(
        &self,
        record: NewOverlayMediaQueueRecord<'_>,
    ) -> Result<(), String> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get DB connection: {}", e))?;
        diesel::insert_into(overlay_media_queue::table)
            .values(record)
            .execute(&mut conn)
            .map_err(|e| format!("Failed to persist overlay queue item: {}", e))?;
        Ok(())
    }

    pub async fn interrupt_dispatched_overlay_media(&self) -> Result<usize, String> {
        use crate::db::schema::overlay_media_queue::dsl::*;
        let now = chrono::Utc::now().to_rfc3339();
        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get DB connection: {}", e))?;
        diesel::update(overlay_media_queue.filter(status.eq("dispatching")))
            .set((
                status.eq("interrupted"),
                error.eq(Some(
                    "Application restarted while playback outcome was unknown".to_string(),
                )),
                updated_at.eq(&now),
                completed_at.eq(Some(&now)),
            ))
            .execute(&mut conn)
            .map_err(|e| format!("Failed to recover overlay queue: {}", e))
    }

    pub async fn claim_next_overlay_media(
        &self,
        kind: &str,
    ) -> Result<Option<OverlayMediaQueueRecord>, String> {
        use crate::db::schema::overlay_media_queue::dsl::*;
        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get DB connection: {}", e))?;
        conn.transaction::<_, diesel::result::Error, _>(|conn| {
            let candidate = overlay_media_queue
                .filter(queue_kind.eq(kind))
                .filter(status.eq("queued"))
                .order(created_at.asc())
                .select(OverlayMediaQueueRecord::as_select())
                .first::<OverlayMediaQueueRecord>(conn)
                .optional()?;
            let Some(candidate) = candidate else {
                return Ok(None);
            };
            let now = chrono::Utc::now().to_rfc3339();
            let claimed = diesel::update(
                overlay_media_queue
                    .filter(id.eq(&candidate.id))
                    .filter(status.eq("queued")),
            )
            .set((
                status.eq("dispatching"),
                started_at.eq(Some(&now)),
                updated_at.eq(&now),
            ))
            .execute(conn)?;
            if claimed != 1 {
                return Ok(None);
            }
            Ok(Some(OverlayMediaQueueRecord {
                status: "dispatching".to_string(),
                started_at: Some(now.clone()),
                updated_at: now,
                ..candidate
            }))
        })
        .map_err(|e| format!("Failed to claim overlay queue item: {}", e))
    }

    pub async fn complete_overlay_media(
        &self,
        item_id: &str,
        final_status: &str,
        final_receipt_phase: Option<&str>,
        receipt_id: Option<&str>,
        final_error: Option<&str>,
    ) -> Result<(), String> {
        use crate::db::schema::overlay_media_queue::dsl::*;
        let now = chrono::Utc::now().to_rfc3339();
        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get DB connection: {}", e))?;
        let updated = diesel::update(overlay_media_queue.filter(id.eq(item_id)))
            .set((
                status.eq(final_status),
                receipt_phase.eq(final_receipt_phase),
                provider_receipt_id.eq(receipt_id),
                error.eq(final_error),
                updated_at.eq(&now),
                completed_at.eq(Some(&now)),
            ))
            .execute(&mut conn)
            .map_err(|e| format!("Failed to complete overlay queue item: {}", e))?;
        if updated != 1 {
            return Err(format!("Overlay queue item '{}' was not found", item_id));
        }
        Ok(())
    }

    pub async fn requeue_overlay_media(&self, item_id: &str) -> Result<(), String> {
        use crate::db::schema::overlay_media_queue::dsl::*;
        let now = chrono::Utc::now().to_rfc3339();
        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get DB connection: {}", e))?;
        diesel::update(
            overlay_media_queue
                .filter(id.eq(item_id))
                .filter(status.eq("dispatching")),
        )
        .set((
            status.eq("queued"),
            started_at.eq(None::<String>),
            updated_at.eq(&now),
        ))
        .execute(&mut conn)
        .map_err(|e| format!("Failed to return overlay item to its queue: {}", e))?;
        Ok(())
    }

    pub async fn cancel_queued_overlay_media(&self, kind: &str) -> Result<usize, String> {
        use crate::db::schema::overlay_media_queue::dsl::*;
        let now = chrono::Utc::now().to_rfc3339();
        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get DB connection: {}", e))?;
        diesel::update(
            overlay_media_queue
                .filter(queue_kind.eq(kind))
                .filter(status.eq("queued")),
        )
        .set((
            status.eq("cancelled"),
            error.eq(Some("Queue cleared".to_string())),
            updated_at.eq(&now),
            completed_at.eq(Some(&now)),
        ))
        .execute(&mut conn)
        .map_err(|e| format!("Failed to clear overlay queue: {}", e))
    }

    pub async fn create_action_invocation(
        &self,
        invocation: NewActionInvocation<'_>,
    ) -> Result<(), String> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get DB connection: {}", e))?;
        diesel::insert_into(action_invocations::table)
            .values(invocation)
            .execute(&mut conn)
            .map_err(|e| format!("Failed to persist invocation before dispatch: {}", e))?;
        Ok(())
    }

    /// Atomically admit one intent for a stable invocation ID. Provider
    /// idempotency keys may legitimately repeat across explicitly approved tries.
    pub async fn claim_action_invocation(
        &self,
        invocation: NewActionInvocation<'_>,
    ) -> Result<bool, String> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get DB connection: {}", e))?;
        diesel::insert_into(action_invocations::table)
            .values(invocation)
            .on_conflict(action_invocations::invocation_id)
            .do_nothing()
            .execute(&mut conn)
            .map(|inserted| inserted == 1)
            .map_err(|e| format!("Failed to persist invocation before dispatch: {}", e))
    }

    /// Record a non-terminal dispatch phase without manufacturing a provider
    /// receipt or completion timestamp. The terminal writer remains
    /// `complete_action_invocation`.
    pub async fn update_action_invocation_dispatch_status(
        &self,
        invocation_id_value: &str,
        dispatch_status_value: &str,
    ) -> Result<(), String> {
        use crate::db::schema::action_invocations::dsl::*;
        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get DB connection: {}", e))?;
        let now = chrono::Utc::now().to_rfc3339();
        let updated =
            diesel::update(action_invocations.filter(invocation_id.eq(invocation_id_value)))
                .set((
                    dispatch_status.eq(dispatch_status_value),
                    updated_at.eq(&now),
                ))
                .execute(&mut conn)
                .map_err(|e| format!("Failed to persist invocation dispatch phase: {}", e))?;
        if updated != 1 {
            return Err(format!(
                "Invocation '{}' disappeared before dispatch phase '{}'",
                invocation_id_value, dispatch_status_value
            ));
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn complete_action_invocation(
        &self,
        invocation_id_value: &str,
        dispatch_status_value: &str,
        request_id_value: Option<&str>,
        provider_receipt_id_value: Option<&str>,
        provider_outputs_value: Option<&str>,
        normalized_status_value: &str,
        error_value: Option<&str>,
        recovery_decision_value: Option<&str>,
        completed_at_value: &str,
        execution_time_ms_value: i64,
    ) -> Result<(), String> {
        use crate::db::schema::action_invocations::dsl::*;
        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get DB connection: {}", e))?;
        let updated =
            diesel::update(action_invocations.filter(invocation_id.eq(invocation_id_value)))
                .set((
                    dispatch_status.eq(dispatch_status_value),
                    request_id.eq(request_id_value),
                    provider_receipt_id.eq(provider_receipt_id_value),
                    provider_outputs.eq(provider_outputs_value),
                    normalized_status.eq(normalized_status_value),
                    error.eq(error_value),
                    recovery_decision.eq(recovery_decision_value),
                    completed_at.eq(completed_at_value),
                    execution_time_ms.eq(execution_time_ms_value),
                    updated_at.eq(completed_at_value),
                ))
                .execute(&mut conn)
                .map_err(|e| format!("Failed to persist invocation outcome: {}", e))?;
        if updated != 1 {
            return Err(format!(
                "Invocation '{}' disappeared before completion",
                invocation_id_value
            ));
        }
        Ok(())
    }

    pub async fn get_action_invocation(
        &self,
        invocation_id_value: &str,
    ) -> Result<Option<ActionInvocationRecord>, String> {
        use crate::db::schema::action_invocations::dsl::*;
        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get DB connection: {}", e))?;
        action_invocations
            .filter(invocation_id.eq(invocation_id_value))
            .select(ActionInvocationRecord::as_select())
            .first(&mut conn)
            .optional()
            .map_err(|e| format!("Failed to load invocation: {}", e))
    }

    pub async fn list_action_invocations(
        &self,
        limit_value: i64,
        source_filter: Option<&str>,
        status_filter: Option<&str>,
        workflow_run_filter: Option<&str>,
        node_filter: Option<&str>,
    ) -> Result<Vec<ActionInvocationRecord>, String> {
        use crate::db::schema::action_invocations::dsl::*;
        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get DB connection: {}", e))?;
        let mut query = action_invocations.into_boxed();
        if let Some(value) = source_filter {
            query = query.filter(source.eq(value));
        }
        if let Some(value) = status_filter {
            query = query.filter(normalized_status.eq(value));
        }
        if let Some(value) = workflow_run_filter {
            query = query.filter(workflow_run_id.eq(value));
        }
        if let Some(value) = node_filter {
            query = query.filter(node_id.eq(value));
        }
        query
            .order(started_at.desc())
            .limit(limit_value.clamp(1, 500))
            .select(ActionInvocationRecord::as_select())
            .load(&mut conn)
            .map_err(|e| format!("Failed to list invocations: {}", e))
    }

    pub async fn save_deck_item_persistent_state(
        &self,
        item_id: &str,
        state: &serde_json::Value,
    ) -> Result<(), String> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get DB connection: {}", e))?;
        let state_json = state.to_string();
        let updated_at = chrono::Utc::now().to_rfc3339();
        diesel::replace_into(deck_item_persistent_state::dsl::deck_item_persistent_state)
            .values(NewDeckItemPersistentState {
                item_id,
                state_json: &state_json,
                updated_at: &updated_at,
            })
            .execute(&mut conn)
            .map_err(|e| format!("Failed to save persistent widget state: {}", e))?;
        Ok(())
    }

    pub async fn get_deck_item_persistent_state(
        &self,
        item_id: &str,
    ) -> Result<Option<serde_json::Value>, String> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get DB connection: {}", e))?;
        let row = deck_item_persistent_state::dsl::deck_item_persistent_state
            .filter(deck_item_persistent_state::dsl::item_id.eq(item_id))
            .select(DeckItemPersistentState::as_select())
            .first::<DeckItemPersistentState>(&mut conn)
            .optional()
            .map_err(|e| format!("Failed to load persistent widget state: {}", e))?;
        row.map(|value| {
            serde_json::from_str(&value.state_json)
                .map_err(|e| format!("Invalid persistent widget state: {}", e))
        })
        .transpose()
    }

    // ============================================================================
    // PACKAGE PERMISSION GRANT OPERATIONS
    // ============================================================================

    /// Atomically replace the durable consent record for a package.
    pub async fn save_package_permission_grant(
        &self,
        pkg: NewPackagePermissionGrant<'_>,
    ) -> Result<(), String> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get DB connection: {}", e))?;
        conn.transaction::<_, diesel::result::Error, _>(|conn| {
            diesel::delete(
                package_permission_grants::dsl::package_permission_grants
                    .filter(package_permission_grants::dsl::package_id.eq(pkg.package_id)),
            )
            .execute(conn)?;
            diesel::insert_into(package_permission_grants::dsl::package_permission_grants)
                .values(&pkg)
                .execute(conn)?;
            Ok(())
        })
        .map_err(|e| format!("Failed to save package permission grant: {}", e))?;
        Ok(())
    }

    pub async fn get_package_permission_grant(
        &self,
        package_id: &str,
    ) -> Result<Option<PackagePermissionGrant>, String> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get DB connection: {}", e))?;
        package_permission_grants::dsl::package_permission_grants
            .filter(package_permission_grants::dsl::package_id.eq(package_id))
            .select(PackagePermissionGrant::as_select())
            .first::<PackagePermissionGrant>(&mut conn)
            .optional()
            .map_err(|e| format!("Failed to load package permission grant: {}", e))
    }

    pub async fn remove_package_permission_grant(&self, package_id: &str) -> Result<(), String> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get DB connection: {}", e))?;
        diesel::delete(
            package_permission_grants::dsl::package_permission_grants
                .filter(package_permission_grants::dsl::package_id.eq(package_id)),
        )
        .execute(&mut conn)
        .map_err(|e| format!("Failed to remove package permission grant: {}", e))?;
        Ok(())
    }

    /// Remove a package's canonical/legacy grant aliases as one database
    /// transaction so uninstall can compensate without observing half-state.
    pub async fn remove_package_permission_grants(
        &self,
        package_ids: &[String],
    ) -> Result<(), String> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get DB connection: {}", e))?;
        conn.transaction::<_, diesel::result::Error, _>(|conn| {
            for package_id in package_ids {
                diesel::delete(
                    package_permission_grants::dsl::package_permission_grants
                        .filter(package_permission_grants::dsl::package_id.eq(package_id)),
                )
                .execute(conn)?;
            }
            Ok(())
        })
        .map_err(|e| format!("Failed to remove package permission grants: {}", e))
    }

    /// Restore consent rows during an uninstall rollback as one transaction.
    pub async fn restore_package_permission_grants(
        &self,
        grants: &[PackagePermissionGrant],
    ) -> Result<(), String> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get DB connection: {}", e))?;
        conn.transaction::<_, diesel::result::Error, _>(|conn| {
            for grant in grants {
                diesel::delete(
                    package_permission_grants::dsl::package_permission_grants
                        .filter(package_permission_grants::dsl::package_id.eq(&grant.package_id)),
                )
                .execute(conn)?;
                diesel::insert_into(package_permission_grants::dsl::package_permission_grants)
                    .values(NewPackagePermissionGrant {
                        package_id: &grant.package_id,
                        manifest_json: &grant.manifest_json,
                        permissions_json: &grant.permissions_json,
                        content_hash: &grant.content_hash,
                        trust: &grant.trust,
                        granted_at: &grant.granted_at,
                    })
                    .execute(conn)?;
            }
            Ok(())
        })
        .map_err(|e| format!("Failed to restore package permission grants: {}", e))
    }

    pub async fn get_all_package_permission_grants(
        &self,
    ) -> Result<Vec<PackagePermissionGrant>, String> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get DB connection: {}", e))?;
        package_permission_grants::dsl::package_permission_grants
            .select(PackagePermissionGrant::as_select())
            .load::<PackagePermissionGrant>(&mut conn)
            .map_err(|e| format!("Failed to load package permission grants: {}", e))
    }

    // ============================================================================
    // INSTALLED PACKAGE RELEASE / PROVENANCE OPERATIONS
    // ============================================================================

    pub async fn save_installed_package_release(
        &self,
        release: NewInstalledPackageRelease<'_>,
    ) -> Result<(), String> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get DB connection: {}", e))?;
        diesel::replace_into(installed_package_releases::dsl::installed_package_releases)
            .values(&release)
            .execute(&mut conn)
            .map_err(|e| format!("Failed to save installed package release: {}", e))?;
        Ok(())
    }

    pub async fn get_installed_package_release(
        &self,
        package_id: &str,
    ) -> Result<Option<InstalledPackageRelease>, String> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get DB connection: {}", e))?;
        installed_package_releases::dsl::installed_package_releases
            .filter(installed_package_releases::dsl::package_id.eq(package_id))
            .select(InstalledPackageRelease::as_select())
            .first::<InstalledPackageRelease>(&mut conn)
            .optional()
            .map_err(|e| format!("Failed to load installed package release: {}", e))
    }

    pub async fn get_all_installed_package_releases(
        &self,
    ) -> Result<Vec<InstalledPackageRelease>, String> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get DB connection: {}", e))?;
        installed_package_releases::dsl::installed_package_releases
            .select(InstalledPackageRelease::as_select())
            .load::<InstalledPackageRelease>(&mut conn)
            .map_err(|e| format!("Failed to list installed package releases: {}", e))
    }

    pub async fn remove_installed_package_release(&self, package_id: &str) -> Result<(), String> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get DB connection: {}", e))?;
        diesel::delete(
            installed_package_releases::dsl::installed_package_releases
                .filter(installed_package_releases::dsl::package_id.eq(package_id)),
        )
        .execute(&mut conn)
        .map_err(|e| format!("Failed to remove installed package release: {}", e))?;
        Ok(())
    }

    // ============================================================================
    // BUTTON OPERATIONS
    // ============================================================================

    /// Get a button by ID
    pub async fn get_button_by_id(
        &self,
        button_id: i32,
    ) -> Result<Option<Button>, diesel::result::Error> {
        let mut conn = self.pool.get().expect("Failed to get DB connection");

        buttons::dsl::buttons
            .filter(buttons::dsl::id.eq(button_id))
            .select(Button::as_select())
            .first::<Button>(&mut conn)
            .optional()
    }

    /// Resolve a saved integration action by its stable identity.
    pub async fn get_saved_action_by_stable_id(
        &self,
        saved_action_id: &str,
    ) -> Result<SavedActionBinding, String> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get DB connection: {}", e))?;

        saved_actions::dsl::saved_actions
            .filter(saved_actions::dsl::stable_id.eq(saved_action_id))
            .select(SavedActionBinding::as_select())
            .first::<SavedActionBinding>(&mut conn)
            .map_err(|e| format!("Failed to load saved action '{}': {}", saved_action_id, e))
    }

    // ============================================================================
    // HTTP REQUEST OPERATIONS
    // ============================================================================

    /// Get an HTTP request by ID
    pub async fn get_http_request_by_id(&self, request_id: i32) -> Result<HttpRequest, String> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            use crate::services::api::credentials::{
                is_credential_reference, protect_headers, requires_protection,
            };
            let mut connection = pool
                .get()
                .map_err(|_| "HTTP storage unavailable".to_string())?;
            let mut request = http_requests::table
                .find(request_id)
                .select(HttpRequest::as_select())
                .first::<HttpRequest>(&mut connection)
                .map_err(|_| "HTTP request not found".to_string())?;
            let Some(raw_headers) = request
                .headers
                .as_deref()
                .filter(|raw| !raw.trim().is_empty())
            else {
                return Ok(request);
            };
            let headers: std::collections::HashMap<String, String> =
                serde_json::from_str(raw_headers).map_err(|_| {
                    "Stored HTTP headers must be an object with string values".to_string()
                })?;
            if !headers
                .iter()
                .any(|(name, value)| requires_protection(name) && !is_credential_reference(value))
            {
                return Ok(request);
            }
            // Migrate legacy plaintext on read, before any action snapshot or editor response.
            // Compare-and-save protects a concurrent edit; compensate newly created keys on failure.
            let protected = protect_headers(headers, &[])?;
            let migration = (|| {
                let headers_json = serde_json::to_string(&protected.headers)
                    .map_err(|_| "Invalid HTTP headers".to_string())?;
                let count = diesel::update(
                    http_requests::table
                        .filter(http_requests::id.eq(request_id))
                        .filter(http_requests::headers.eq(&request.headers)),
                )
                .set(http_requests::headers.eq(Some(&headers_json)))
                .execute(&mut connection)
                .map_err(|_| "HTTP credential migration could not be saved".to_string())?;
                if count != 1 {
                    return Err(
                        "HTTP request changed during credential migration; reload it".into(),
                    );
                }
                Ok::<_, String>(headers_json)
            })();
            match migration {
                Ok(headers_json) => {
                    request.headers = Some(headers_json);
                    Ok(request)
                }
                Err(error) => {
                    protected.rollback();
                    Err(error)
                }
            }
        })
        .await
        .map_err(|_| "HTTP request loading interrupted".to_string())?
    }

    pub async fn list_http_requests(&self) -> Result<Vec<HttpRequest>, String> {
        let pool = self.pool.clone();
        let ids = tokio::task::spawn_blocking(move || {
            let mut connection = pool
                .get()
                .map_err(|_| "HTTP storage unavailable".to_string())?;
            http_requests::table
                .select(http_requests::id)
                .load::<i32>(&mut connection)
                .map_err(|_| "HTTP requests could not be loaded".to_string())
        })
        .await
        .map_err(|_| "HTTP request loading interrupted".to_string())??;
        let mut requests = Vec::with_capacity(ids.len());
        for id in ids {
            requests.push(self.get_http_request_by_id(id).await?);
        }
        Ok(requests)
    }

    // ============================================================================
    // FOCUS WINDOW OPERATIONS
    // ============================================================================

    /// Get a focus window configuration by ID
    pub async fn get_focus_window_by_id(&self, window_id: i32) -> Result<FocusWindow, String> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get DB connection: {}", e))?;

        focus_windows::dsl::focus_windows
            .filter(focus_windows::dsl::id.eq(window_id))
            .select(FocusWindow::as_select())
            .first::<FocusWindow>(&mut conn)
            .map_err(|e| format!("Failed to load focus window: {}", e))
    }

    // ============================================================================
    // PROGRAM OPERATIONS
    // ============================================================================

    /// Get a program by ID
    pub async fn get_program_by_id(&self, program_id: i32) -> Result<Option<Program>, String> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get DB connection: {}", e))?;

        programs::dsl::programs
            .filter(programs::dsl::id.eq(program_id))
            .select(Program::as_select())
            .first::<Program>(&mut conn)
            .optional()
            .map_err(|e| format!("Failed to load program: {}", e))
    }

    // ============================================================================
    // FLOW OPERATIONS
    // ============================================================================

    /// Load a flow definition from the database
    pub fn load_flow(
        &self,
        flow_id: &str,
    ) -> Result<crate::services::flow::executor::FlowDefinition, String> {
        use crate::db::schema::flows::dsl::*;

        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get DB connection: {}", e))?;

        let flow = flows
            .filter(id.eq(flow_id))
            .select(Flow::as_select())
            .first::<Flow>(&mut conn)
            .map_err(|e| format!("Flow not found: {}", e))?;

        let requires_edge_migration = serde_json::from_str::<serde_json::Value>(&flow.data)
            .ok()
            .and_then(|value| {
                value
                    .get("connections")
                    .and_then(|edges| edges.as_array())
                    .cloned()
            })
            .is_some_and(|edges| {
                edges
                    .iter()
                    .any(|edge| edge.get("kind").and_then(|kind| kind.as_str()).is_none())
            });

        let flow_definition: crate::services::flow::executor::FlowDefinition =
            serde_json::from_str(&flow.data)
                .map_err(|e| format!("Failed to deserialize flow: {}", e))?;
        let protected = crate::services::flow::http_credentials::protect_flow(&flow_definition)?;
        let credentials_changed = crate::services::flow::executor::flow_revision(&flow_definition)?
            != crate::services::flow::executor::flow_revision(&protected.flow)?;
        if requires_edge_migration || credentials_changed {
            let canonical_data = serde_json::to_string(&protected.flow)
                .map_err(|e| format!("Failed to serialize migrated flow: {}", e))?;
            let affected = diesel::update(flows.filter(id.eq(flow_id)).filter(data.eq(&flow.data)))
                .set(data.eq(canonical_data))
                .execute(&mut conn)
                .map_err(|e| format!("Failed to persist migrated flow edges: {}", e))?;
            if affected != 1 {
                return Err("Flow changed during credential migration; reload it".into());
            }
        }

        Ok(protected.commit())
    }

    /// Save a flow definition to the database
    pub fn save_flow(
        &self,
        flow_definition: &crate::services::flow::executor::FlowDefinition,
    ) -> Result<(), String> {
        use crate::db::models::{Flow, NewFlow, UpdateFlow};
        use crate::db::schema::flows::dsl::*;

        let mut conn = self.pool.get().map_err(|e| e.to_string())?;

        // Serialize flow definition to JSON
        let protected = crate::services::flow::http_credentials::protect_flow(flow_definition)?;
        let flow_definition = &protected.flow;
        let flow_json = serde_json::to_string(flow_definition)
            .map_err(|e| format!("Failed to serialize flow: {}", e))?;

        // Serialize tags if they exist
        let tags_json = if !flow_definition.tags.is_empty() {
            Some(
                serde_json::to_string(&flow_definition.tags)
                    .map_err(|e| format!("Failed to serialize tags: {}", e))?,
            )
        } else {
            None
        };

        // Check if flow already exists
        let existing_flow = flows
            .filter(id.eq(&flow_definition.id))
            .first::<Flow>(&mut conn)
            .optional()
            .map_err(|e| format!("Database error: {}", e))?;

        if existing_flow.is_some() {
            // Update existing flow
            let update_flow = UpdateFlow {
                name: Some(&flow_definition.name),
                description: flow_definition.description.as_deref(),
                data: Some(&flow_json),
                updated_at: &flow_definition.updated_at.to_rfc3339(),
                tags: tags_json.as_deref(),
                version: Some(&flow_definition.version),
            };

            diesel::update(flows.filter(id.eq(&flow_definition.id)))
                .set(&update_flow)
                .execute(&mut conn)
                .map_err(|e| format!("Failed to update flow: {}", e))?;
        } else {
            // Insert new flow
            let new_flow = NewFlow {
                id: &flow_definition.id,
                name: &flow_definition.name,
                description: flow_definition.description.as_deref(),
                data: &flow_json,
                created_at: &flow_definition.created_at.to_rfc3339(),
                updated_at: &flow_definition.updated_at.to_rfc3339(),
                created_by: flow_definition.created_by.as_deref(),
                tags: tags_json.as_deref(),
                version: &flow_definition.version,
            };

            diesel::insert_into(flows)
                .values(&new_flow)
                .execute(&mut conn)
                .map_err(|e| format!("Failed to insert flow: {}", e))?;
        }

        protected.commit();
        Ok(())
    }

    /// Atomically replace an existing flow when its persisted semantic
    /// revision still matches the caller's observation.
    pub fn replace_flow_at_revision(
        &self,
        expected_revision: &str,
        flow_definition: &crate::services::flow::executor::FlowDefinition,
    ) -> Result<crate::services::flow::executor::FlowDefinition, String> {
        use crate::db::models::{Flow, UpdateFlow};
        use crate::db::schema::flows::dsl::*;
        use crate::services::flow::executor::flow_revision;

        let mut conn = self.pool.get().map_err(|error| error.to_string())?;
        let existing = flows
            .filter(id.eq(&flow_definition.id))
            .select(Flow::as_select())
            .first::<Flow>(&mut conn)
            .map_err(|error| format!("Flow not found: {}", error))?;
        let current: crate::services::flow::executor::FlowDefinition =
            serde_json::from_str(&existing.data)
                .map_err(|error| format!("Failed to deserialize flow: {}", error))?;
        let actual_revision = flow_revision(&current)?;
        if actual_revision != expected_revision {
            return Err(format!(
                "Stale flow revision: expected '{}', current is '{}'",
                expected_revision, actual_revision
            ));
        }

        let protected = crate::services::flow::http_credentials::protect_flow(flow_definition)?;
        let flow_definition = &protected.flow;
        let flow_json = serde_json::to_string(flow_definition)
            .map_err(|error| format!("Failed to serialize flow: {}", error))?;
        let tags_json = (!flow_definition.tags.is_empty())
            .then(|| serde_json::to_string(&flow_definition.tags))
            .transpose()
            .map_err(|error| format!("Failed to serialize tags: {}", error))?;
        let updated_at_value = flow_definition.updated_at.to_rfc3339();
        let update_flow = UpdateFlow {
            name: Some(&flow_definition.name),
            description: flow_definition.description.as_deref(),
            data: Some(&flow_json),
            updated_at: &updated_at_value,
            tags: tags_json.as_deref(),
            version: Some(&flow_definition.version),
        };

        let affected = diesel::update(
            flows
                .filter(id.eq(&flow_definition.id))
                .filter(data.eq(&existing.data)),
        )
        .set(&update_flow)
        .execute(&mut conn)
        .map_err(|error| format!("Failed to update flow: {}", error))?;
        if affected != 1 {
            return Err(
                "Flow changed while the edit was being saved; reload and retry".to_string(),
            );
        }

        // The replacement now owns these generations, even if readback fails.
        let protected_flow_id = flow_definition.id.clone();
        protected.commit();

        let saved = flows
            .filter(id.eq(&protected_flow_id))
            .select(Flow::as_select())
            .first::<Flow>(&mut conn)
            .map_err(|error| format!("Failed to read back saved flow: {}", error))?;
        serde_json::from_str(&saved.data)
            .map_err(|error| format!("Failed to deserialize saved flow: {}", error))
    }

    /// List all flows (summary only)
    pub fn list_flows(&self) -> Result<Vec<crate::services::flow::executor::FlowSummary>, String> {
        use crate::db::schema::flows::dsl::*;

        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get DB connection: {}", e))?;

        let flow_records = flows
            .select(Flow::as_select())
            .load::<Flow>(&mut conn)
            .map_err(|e| format!("Failed to list flows: {}", e))?;

        let summaries = flow_records
            .into_iter()
            .map(|f| crate::services::flow::executor::FlowSummary {
                id: f.id,
                name: f.name,
                description: f.description,
                created_at: chrono::DateTime::parse_from_rfc3339(&f.created_at)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                updated_at: chrono::DateTime::parse_from_rfc3339(&f.updated_at)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                status: None,
                tags: f
                    .tags
                    .and_then(|t| serde_json::from_str(&t).ok())
                    .unwrap_or_default(),
            })
            .collect();

        Ok(summaries)
    }

    /// Delete a flow by ID
    pub fn delete_flow(&self, flow_id: &str) -> Result<(), String> {
        use crate::db::schema::flows::dsl::*;

        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get DB connection: {}", e))?;

        diesel::delete(flows.filter(id.eq(flow_id)))
            .execute(&mut conn)
            .map_err(|e| format!("Failed to delete flow: {}", e))?;

        Ok(())
    }

    // ============================================================================
    // MACRO OPERATIONS
    // ============================================================================

    /// Get a macro with its full chain (for execution)
    pub async fn get_macro_with_chain(&self, macro_id: i32) -> Option<Macro> {
        let mut conn = self.pool.get().expect("Failed to get DB connection");

        // Get the macro from the database
        let macro_data = macros::table
            .filter(macros::dsl::id.eq(macro_id))
            .first::<Macro>(&mut conn)
            .optional()
            .expect("Error querying macro");

        macro_data
    }

    // ============================================================================
    // PEER OPERATIONS (for WebSocket/E2EE)
    // ============================================================================

    /// Get the count of all peers
    pub fn get_peer_count(&self) -> i64 {
        use crate::db::schema::peers::dsl::*;

        let mut conn = self.pool.get().expect("Failed to get DB connection");

        peers.count().get_result::<i64>(&mut conn).unwrap_or(0)
    }

    /// Save a shared secret for E2EE (stored in peers table)
    pub fn save_shared_secret(
        &self,
        peer_client_id: &str,
        secret_bytes: &[u8; 32],
    ) -> Result<(), diesel::result::Error> {
        use crate::db::schema::peers::dsl::*;
        use base64::engine::general_purpose::STANDARD as Base64;
        use base64::Engine;

        let mut conn = self.pool.get().expect("Failed to get DB connection");

        // Encode secret as base64 for storage
        let secret_b64 = Base64.encode(secret_bytes);

        // Update the peer's shared_secret field
        diesel::update(peers.filter(client_id.eq(peer_client_id)))
            .set(shared_secret.eq(secret_b64))
            .execute(&mut conn)?;

        println!(
            "💾 Saved shared secret to database for client: {}",
            peer_client_id
        );
        Ok(())
    }

    /// Load a shared secret for E2EE (from peers table)
    pub fn load_shared_secret(&self, peer_client_id: &str) -> Option<[u8; 32]> {
        use crate::db::schema::peers::dsl::*;
        use base64::engine::general_purpose::STANDARD as Base64;
        use base64::Engine;

        let mut conn = self.pool.get().expect("Failed to get DB connection");

        let result = peers
            .filter(client_id.eq(peer_client_id))
            .select(shared_secret)
            .first::<Option<String>>(&mut conn)
            .optional()
            .expect("Error querying shared secret");

        // Decode base64 and convert to [u8; 32]
        result.flatten().and_then(|secret_b64| {
            Base64.decode(&secret_b64).ok().and_then(|bytes| {
                if bytes.len() == 32 {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&bytes);
                    Some(arr)
                } else {
                    println!(
                        "⚠️ Invalid shared secret length in database for client: {}",
                        peer_client_id
                    );
                    None
                }
            })
        })
    }
    // ============================================================================
    // CHAT MESSAGE OPERATIONS (Streaming)
    // ============================================================================

    /// Save a chat message to the database
    pub async fn save_chat_message(
        &self,
        msg: NewChatMessage<'_>,
    ) -> Result<(), diesel::result::Error> {
        let mut conn = self
            .pool
            .get()
            .map_err(|error| diesel::result::Error::QueryBuilderError(error.into()))?;

        diesel::insert_into(chat_messages::table)
            .values(&msg)
            .execute(&mut conn)?;

        Ok(())
    }

    /// Load messages for a specific session run
    pub async fn load_messages_for_session_run(
        &self,
        session_id: &str,
        run_index: i32,
        limit: Option<i32>,
    ) -> Result<Vec<ChatMessageDb>, diesel::result::Error> {
        let mut conn = self.pool.get().expect("Failed to get DB connection");

        let mut query = chat_messages::table
            .filter(chat_messages::session_id.eq(session_id))
            .filter(chat_messages::session_run_index.eq(run_index))
            .order(chat_messages::timestamp.desc())
            .into_boxed();

        if let Some(limit_val) = limit {
            query = query.limit(limit_val.into());
        }

        let mut messages = query.load::<ChatMessageDb>(&mut conn)?;

        // Reverse so oldest messages appear first in chat (top)
        messages.reverse();

        Ok(messages)
    }

    /// Load messages for multiple account IDs (for multi-platform sessions)
    pub async fn load_messages_for_accounts(
        &self,
        account_ids: &[u64],
        limit: Option<i32>,
    ) -> Result<Vec<ChatMessageDb>, diesel::result::Error> {
        let mut conn = self.pool.get().expect("Failed to get DB connection");

        // Convert u64 to i32 for SQLite
        let account_ids_i32: Vec<i32> = account_ids.iter().map(|&id| id as i32).collect();

        let mut query = chat_messages::table
            .filter(chat_messages::account_id.eq_any(account_ids_i32))
            .order(chat_messages::timestamp.desc())
            .into_boxed();

        if let Some(limit_val) = limit {
            query = query.limit(limit_val.into());
        }

        let mut messages = query.load::<ChatMessageDb>(&mut conn)?;

        // Reverse so oldest messages appear first in chat (top)
        messages.reverse();

        Ok(messages)
    }

    /// Delete messages for old session runs (keep only last 2 runs)
    pub async fn cleanup_old_session_runs(
        &self,
        session_id: &str,
        keep_from_run_index: i32,
    ) -> Result<usize, diesel::result::Error> {
        let mut conn = self.pool.get().expect("Failed to get DB connection");

        diesel::delete(chat_messages::table)
            .filter(chat_messages::session_id.eq(session_id))
            .filter(chat_messages::session_run_index.lt(keep_from_run_index))
            .execute(&mut conn)
    }

    /// Load a complete, revisioned deck widget document. A deck that has not
    /// been upgraded yet returns revision 0 with no persisted items; callers
    /// can then project its legacy button rows without mutating the database.
    pub async fn get_deck_widget_document(
        &self,
        requested_deck_id: i32,
    ) -> Result<DeckWidgetDocument, String> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get DB connection: {}", e))?;

        let record = deck_widget_documents::table
            .filter(deck_widget_documents::deck_id.eq(requested_deck_id))
            .select(DeckWidgetDocumentRecord::as_select())
            .first::<DeckWidgetDocumentRecord>(&mut conn)
            .optional()
            .map_err(|e| format!("Failed to load deck widget revision: {}", e))?;
        let items = deck_items::table
            .filter(deck_items::deck_id.eq(requested_deck_id))
            .order(deck_items::id.asc())
            .select(DeckItem::as_select())
            .load::<DeckItem>(&mut conn)
            .map_err(|e| format!("Failed to load deck items: {}", e))?;

        let mut documents = Vec::with_capacity(items.len());
        for item in items {
            let layouts = deck_item_layouts::table
                .filter(deck_item_layouts::item_id.eq(&item.id))
                .order(deck_item_layouts::layout_key.asc())
                .select(DeckItemLayout::as_select())
                .load::<DeckItemLayout>(&mut conn)
                .map_err(|e| format!("Failed to load layouts for '{}': {}", item.id, e))?;
            let bindings = deck_item_bindings::table
                .filter(deck_item_bindings::item_id.eq(&item.id))
                .order(deck_item_bindings::id.asc())
                .select(DeckItemBinding::as_select())
                .load::<DeckItemBinding>(&mut conn)
                .map_err(|e| format!("Failed to load bindings for '{}': {}", item.id, e))?;
            documents.push(DeckItemDocument {
                item,
                layouts,
                bindings,
            });
        }

        Ok(DeckWidgetDocument {
            deck_id: requested_deck_id,
            revision: record.as_ref().map_or(0, |value| value.revision),
            updated_at: record.map_or_else(String::new, |value| value.updated_at),
            items: documents,
        })
    }

    /// Atomically replace the widget document if the caller still owns the
    /// expected revision. This prevents an autosave or mobile client from
    /// silently overwriting a newer editor revision.
    pub async fn save_deck_widget_document(
        &self,
        requested_deck_id: i32,
        expected_revision: i32,
        documents: &[DeckItemDocument],
    ) -> Result<i32, String> {
        use std::collections::HashSet;

        let mut item_ids = HashSet::new();
        let mut binding_ids = HashSet::new();
        for document in documents {
            if document.item.deck_id != requested_deck_id {
                return Err(format!(
                    "Deck item '{}' belongs to deck {}, expected {}",
                    document.item.id, document.item.deck_id, requested_deck_id
                ));
            }
            if !item_ids.insert(document.item.id.clone()) {
                return Err(format!("Duplicate deck item id '{}'", document.item.id));
            }
            if document.item.config_version < 1 {
                return Err(format!(
                    "Deck item '{}' has an invalid config version",
                    document.item.id
                ));
            }
            serde_json::from_str::<serde_json::Value>(&document.item.config_json)
                .map_err(|e| format!("Invalid config JSON for '{}': {}", document.item.id, e))?;
            serde_json::from_str::<serde_json::Value>(&document.item.appearance_json).map_err(
                |e| format!("Invalid appearance JSON for '{}': {}", document.item.id, e),
            )?;

            if document.layouts.is_empty() {
                return Err(format!("Deck item '{}' has no layouts", document.item.id));
            }
            for layout in &document.layouts {
                if layout.item_id != document.item.id
                    || layout.x < 0
                    || layout.y < 0
                    || layout.col_span < 1
                    || layout.row_span < 1
                {
                    return Err(format!(
                        "Invalid layout for deck item '{}'",
                        document.item.id
                    ));
                }
            }
            for binding in &document.bindings {
                if binding.item_id != document.item.id || !binding_ids.insert(binding.id.clone()) {
                    return Err(format!("Invalid or duplicate binding '{}'", binding.id));
                }
                serde_json::from_str::<serde_json::Value>(&binding.action_json)
                    .map_err(|e| format!("Invalid action JSON for '{}': {}", binding.id, e))?;
                serde_json::from_str::<serde_json::Value>(&binding.policy_json)
                    .map_err(|e| format!("Invalid policy JSON for '{}': {}", binding.id, e))?;
                if let Some(transform) = &binding.transform_json {
                    serde_json::from_str::<serde_json::Value>(transform).map_err(|e| {
                        format!("Invalid transform JSON for '{}': {}", binding.id, e)
                    })?;
                }
            }
        }

        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get DB connection: {}", e))?;
        let next_revision = expected_revision
            .checked_add(1)
            .ok_or_else(|| "Deck widget revision overflow".to_string())?;
        let updated_at = chrono::Utc::now().to_rfc3339();
        let mut revision_conflict = None;

        let result = conn.transaction::<_, diesel::result::Error, _>(|conn| {
            let current_revision = deck_widget_documents::table
                .filter(deck_widget_documents::deck_id.eq(requested_deck_id))
                .select(deck_widget_documents::revision)
                .first::<i32>(conn)
                .optional()?
                .unwrap_or(0);
            if current_revision != expected_revision {
                revision_conflict = Some(current_revision);
                return Err(diesel::result::Error::RollbackTransaction);
            }

            let previous_ids = deck_items::table
                .filter(deck_items::deck_id.eq(requested_deck_id))
                .select(deck_items::id)
                .load::<String>(conn)?;
            if !previous_ids.is_empty() {
                diesel::delete(
                    deck_item_bindings::table
                        .filter(deck_item_bindings::item_id.eq_any(&previous_ids)),
                )
                .execute(conn)?;
                diesel::delete(
                    deck_item_layouts::table
                        .filter(deck_item_layouts::item_id.eq_any(&previous_ids)),
                )
                .execute(conn)?;
            }
            diesel::delete(deck_items::table.filter(deck_items::deck_id.eq(requested_deck_id)))
                .execute(conn)?;

            for document in documents {
                diesel::insert_into(deck_items::table)
                    .values(&document.item)
                    .execute(conn)?;
                diesel::insert_into(deck_item_layouts::table)
                    .values(&document.layouts)
                    .execute(conn)?;
                if !document.bindings.is_empty() {
                    diesel::insert_into(deck_item_bindings::table)
                        .values(&document.bindings)
                        .execute(conn)?;
                }
            }

            diesel::delete(
                deck_widget_documents::table
                    .filter(deck_widget_documents::deck_id.eq(requested_deck_id)),
            )
            .execute(conn)?;
            diesel::insert_into(deck_widget_documents::table)
                .values(DeckWidgetDocumentRecord {
                    deck_id: requested_deck_id,
                    revision: next_revision,
                    updated_at: updated_at.clone(),
                })
                .execute(conn)?;
            Ok(())
        });

        if let Some(current_revision) = revision_conflict {
            return Err(format!(
                "Deck widget revision conflict: expected {}, current {}",
                expected_revision, current_revision
            ));
        }
        result.map_err(|e| format!("Failed to save deck widget document: {}", e))?;
        Ok(next_revision)
    }

    /// Remove only the first-class widget projection. Legacy buttons are never
    /// deleted, so this is a safe rollback to the V1 document source.
    pub async fn revert_deck_widget_document(&self, requested_deck_id: i32) -> Result<(), String> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get DB connection: {}", e))?;
        conn.transaction::<_, diesel::result::Error, _>(|conn| {
            let item_ids = deck_items::table
                .filter(deck_items::deck_id.eq(requested_deck_id))
                .select(deck_items::id)
                .load::<String>(conn)?;
            if !item_ids.is_empty() {
                diesel::delete(
                    deck_item_bindings::table.filter(deck_item_bindings::item_id.eq_any(&item_ids)),
                )
                .execute(conn)?;
                diesel::delete(
                    deck_item_layouts::table.filter(deck_item_layouts::item_id.eq_any(&item_ids)),
                )
                .execute(conn)?;
            }
            diesel::delete(deck_items::table.filter(deck_items::deck_id.eq(requested_deck_id)))
                .execute(conn)?;
            diesel::delete(
                deck_widget_documents::table
                    .filter(deck_widget_documents::deck_id.eq(requested_deck_id)),
            )
            .execute(conn)?;
            Ok(())
        })
        .map_err(|e| format!("Failed to revert deck widget document: {}", e))
    }
}

// ============================================================================
// FUTURE: More functions will be migrated here
// ============================================================================
//
// This is Phase 1 of the migration - moving the top 10 most critical functions.
// Future migrations will move:
// - Deck operations
// - Theme operations
// - Settings operations
// - All other database operations from src-tauri/db/utils.rs
//
// Goal: Eventually src-tauri/db/utils.rs will only contain Tauri command wrappers

#[cfg(test)]
mod deck_widget_document_tests {
    use super::*;
    use crate::services::flow::executor::{
        FlowConnection, FlowDefinition, FlowMetadata, FlowNodeDefinition, NodeExecutionConfig,
        NodeUIData,
    };
    use chrono::Utc;
    use diesel::r2d2::ConnectionManager;
    use diesel_migrations::MigrationHarness;
    use std::collections::HashMap;

    fn test_database() -> (tempfile::TempDir, Database) {
        let directory = tempfile::tempdir().expect("temporary database directory");
        let path = directory.path().join("widgets.db");
        let manager = ConnectionManager::<SqliteConnection>::new(path.to_string_lossy());
        let pool = Pool::builder()
            .max_size(1)
            .build(manager)
            .expect("database pool");
        {
            let mut connection = pool.get().expect("database connection");
            connection
                .run_pending_migrations(crate::db::db::MIGRATIONS)
                .expect("widget migrations");
            diesel::insert_into(decks::table)
                .values(decks::name.eq("Widget test"))
                .execute(&mut connection)
                .expect("test deck");
        }
        (directory, Database::new(&pool))
    }

    fn document(deck_id: i32) -> DeckItemDocument {
        DeckItemDocument {
            item: DeckItem {
                id: "widget:test".to_string(),
                deck_id,
                definition_id: "vortideck.telemetry".to_string(),
                package_id: None,
                contribution_id: None,
                config_json: r#"{"label":"CPU"}"#.to_string(),
                config_version: 1,
                appearance_json: "{}".to_string(),
                legacy_button_id: None,
                enabled: true,
            },
            layouts: vec![DeckItemLayout {
                item_id: "widget:test".to_string(),
                layout_key: "portrait".to_string(),
                x: 0,
                y: 0,
                col_span: 2,
                row_span: 1,
            }],
            bindings: vec![DeckItemBinding {
                id: "binding:test".to_string(),
                item_id: "widget:test".to_string(),
                port_id: "primary".to_string(),
                action_json: r#"{"SystemControl":"system.test"}"#.to_string(),
                policy_json: r#"{"type":"run_once"}"#.to_string(),
                transform_json: None,
            }],
        }
    }

    #[test]
    fn loading_a_legacy_flow_rewrites_its_edges_once() {
        let (_directory, database) = test_database();
        let node = |id: &str| FlowNodeDefinition {
            id: id.to_string(),
            node_type: "test".to_string(),
            name: id.to_string(),
            inputs: Default::default(),
            outputs: Vec::new(),
            ai_behavior: None,
            ai_model: None,
            prompt_template: None,
            tool_name: None,
            position: (0.0, 0.0),
            ui_data: NodeUIData::default(),
            execution_config: NodeExecutionConfig::default(),
        };
        let flow = FlowDefinition {
            id: "legacy-flow".to_string(),
            name: "Legacy flow".to_string(),
            version: "1".to_string(),
            description: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            created_by: None,
            tags: Vec::new(),
            nodes: vec![node("one"), node("two")],
            connections: vec![FlowConnection::control("one", "two")],
            start_node_id: "one".to_string(),
            metadata: FlowMetadata::default(),
            is_active: false,
        };
        let mut legacy_value = serde_json::to_value(&flow).expect("flow JSON");
        legacy_value["connections"] = serde_json::json!([{
            "fromNode": "one",
            "fromPort": "flow-output",
            "toNode": "two",
            "toPort": "flow-input"
        }]);
        let legacy_data = legacy_value.to_string();
        let now = Utc::now().to_rfc3339();
        {
            let mut connection = database.pool.get().expect("database connection");
            diesel::insert_into(flows::table)
                .values(NewFlow {
                    id: "legacy-flow",
                    name: "Legacy flow",
                    description: None,
                    data: &legacy_data,
                    created_at: &now,
                    updated_at: &now,
                    created_by: None,
                    tags: None,
                    version: "1",
                })
                .execute(&mut connection)
                .expect("legacy flow fixture");
        }

        let loaded = database.load_flow("legacy-flow").expect("migrated flow");
        assert_eq!(
            loaded.connections,
            vec![FlowConnection::control("one", "two")]
        );

        let mut connection = database.pool.get().expect("database connection");
        let stored = flows::table
            .filter(flows::id.eq("legacy-flow"))
            .select(flows::data)
            .first::<String>(&mut connection)
            .expect("rewritten flow JSON");
        let stored: serde_json::Value = serde_json::from_str(&stored).expect("valid stored JSON");
        assert_eq!(stored["connections"][0]["kind"], "control");
        assert!(stored["connections"][0].get("fromPort").is_none());
    }

    #[test]
    fn flow_round_trip_preserves_ai_fields_and_revision_compare_and_save() {
        use crate::services::flow::executor::flow_revision;

        let (_directory, database) = test_database();
        let mut node = FlowNodeDefinition {
            id: "start".to_string(),
            node_type: "ai".to_string(),
            name: "AI start".to_string(),
            inputs: HashMap::from([("topic".to_string(), serde_json::json!("OBS"))]),
            outputs: Vec::new(),
            ai_behavior: None,
            ai_model: Some("deterministic-test-model".to_string()),
            prompt_template: Some("Explain {{topic}}".to_string()),
            tool_name: None,
            position: (12.0, 24.0),
            ui_data: NodeUIData::default(),
            execution_config: NodeExecutionConfig::default(),
        };
        node.ui_data.custom_properties.insert(
            "retainedUnknown".to_string(),
            serde_json::json!({"value": 7}),
        );
        let now = Utc::now();
        let flow = FlowDefinition {
            id: "revision-flow".to_string(),
            name: "Revision flow".to_string(),
            version: "1.0.0".to_string(),
            description: Some("round trip".to_string()),
            created_at: now,
            updated_at: now,
            created_by: Some("test".to_string()),
            tags: vec!["ai".to_string()],
            nodes: vec![node],
            connections: Vec::new(),
            start_node_id: "start".to_string(),
            metadata: FlowMetadata::default(),
            is_active: false,
        };
        database.save_flow(&flow).expect("initial flow save");

        let loaded = database.load_flow(&flow.id).expect("flow readback");
        assert_eq!(
            loaded.nodes[0].prompt_template.as_deref(),
            Some("Explain {{topic}}")
        );
        assert_eq!(
            loaded.nodes[0].ai_model.as_deref(),
            Some("deterministic-test-model")
        );
        assert_eq!(
            loaded.nodes[0].ui_data.custom_properties["retainedUnknown"],
            serde_json::json!({"value": 7})
        );

        let revision = flow_revision(&loaded).expect("flow revision");
        let mut proposed = loaded.clone();
        proposed.name = "Revision flow updated".to_string();
        proposed.updated_at = Utc::now();
        let saved = database
            .replace_flow_at_revision(&revision, &proposed)
            .expect("compare-and-save");
        assert_eq!(saved.name, "Revision flow updated");
        assert!(database
            .replace_flow_at_revision(&revision, &loaded)
            .unwrap_err()
            .contains("Stale flow revision"));
    }

    #[tokio::test]
    async fn widget_document_round_trip_conflict_and_legacy_rollback() {
        let (_directory, database) = test_database();
        let deck_id = 1;

        {
            let mut connection = database.pool.get().expect("database connection");
            diesel::insert_into(buttons::table)
                .values((
                    buttons::deck_id.eq(deck_id),
                    buttons::label.eq("Legacy action"),
                    buttons::position.eq(1),
                    buttons::action.eq(Some(r#"{"SystemControl":"system.test"}"#)),
                    buttons::mode.eq(Some("click/press")),
                ))
                .execute(&mut connection)
                .expect("legacy button fixture");
        }

        let mut migrated = document(deck_id);
        migrated.item.legacy_button_id = Some(1);

        assert_eq!(
            database
                .save_deck_widget_document(deck_id, 0, &[migrated.clone()])
                .await
                .expect("first save"),
            1
        );
        let saved = database
            .get_deck_widget_document(deck_id)
            .await
            .expect("saved document");
        assert_eq!(saved.revision, 1);
        assert_eq!(saved.items.len(), 1);
        assert_eq!(saved.items[0].layouts[0].col_span, 2);

        let conflict = database
            .save_deck_widget_document(deck_id, 0, &[migrated])
            .await
            .expect_err("stale save must conflict");
        assert!(conflict.contains("expected 0, current 1"));

        database
            .revert_deck_widget_document(deck_id)
            .await
            .expect("legacy rollback");
        let reverted = database
            .get_deck_widget_document(deck_id)
            .await
            .expect("reverted document");
        assert_eq!(reverted.revision, 0);
        assert!(reverted.items.is_empty());

        let mut connection = database.pool.get().expect("database connection");
        let legacy_button = buttons::table
            .filter(buttons::id.eq(1))
            .select((buttons::label, buttons::action, buttons::position))
            .first::<(String, Option<String>, i32)>(&mut connection)
            .expect("legacy rollback source remains intact");
        assert_eq!(legacy_button.0, "Legacy action");
        assert_eq!(legacy_button.2, 1);
        assert!(legacy_button.1.unwrap().contains("system.test"));
    }

    #[tokio::test]
    async fn invocation_lifecycle_is_durable_and_keeps_retry_attempts() {
        let (_directory, database) = test_database();
        let now = chrono::Utc::now().to_rfc3339();
        let new_invocation = || NewActionInvocation {
            invocation_id: "invocation:test",
            parent_invocation_id: None,
            source: "flow",
            source_reference: Some("run:node"),
            workflow_run_id: Some("run"),
            node_id: Some("node"),
            resolved_action: r#"{"type":"integration"}"#,
            integration_id: Some("test"),
            action_id: Some("apply"),
            capability_version: Some(1),
            idempotency_key: Some("flow-action:run:node"),
            resource_key: Some("test:target"),
            dispatch_status: "dispatching",
            receipt_semantics: Some("applied"),
            request_id: Some("invocation:test"),
            provider_receipt_id: None,
            provider_outputs: None,
            normalized_status: None,
            error: None,
            recovery_decision: Some("manual_review"),
            started_at: &now,
            completed_at: None,
            execution_time_ms: 0,
            created_at: &now,
            updated_at: &now,
        };
        database
            .create_action_invocation(new_invocation())
            .await
            .expect("claim invocation before dispatch");

        let duplicate = NewActionInvocation {
            invocation_id: "invocation:duplicate",
            ..new_invocation()
        };
        database
            .create_action_invocation(duplicate)
            .await
            .expect("approved attempts may retain the same provider idempotency key");

        database
            .complete_action_invocation(
                "invocation:test",
                "succeeded",
                Some("request:test"),
                Some("receipt:test"),
                Some(r#"{"ok":true}"#),
                "applied",
                None,
                Some("none"),
                &now,
                12,
            )
            .await
            .expect("persist outcome");
        let record = database
            .get_action_invocation("invocation:test")
            .await
            .expect("load invocation")
            .expect("record exists");
        assert_eq!(record.normalized_status.as_deref(), Some("applied"));
        assert_eq!(record.provider_receipt_id.as_deref(), Some("receipt:test"));
        assert_eq!(record.execution_time_ms, 12);
        assert_eq!(
            database
                .list_action_invocations(
                    10,
                    Some("flow"),
                    Some("applied"),
                    Some("run"),
                    Some("node")
                )
                .await
                .expect("filtered invocation list")
                .len(),
            1
        );
    }
}
