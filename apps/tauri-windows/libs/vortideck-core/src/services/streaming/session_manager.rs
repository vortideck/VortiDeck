use super::command_policy::{normalize_commands, validate_commands};
use super::error::{StreamingError, StreamingResult};
use super::models::{PlatformStream, SessionSettings, SessionStatus, StreamRun, StreamSession};
use crate::db::models::{NewStreamSession, StreamSessionDb};
use crate::db::schema::stream_sessions;
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use serde_json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

type DbPool = r2d2::Pool<ConnectionManager<SqliteConnection>>;

/// Session Manager - Manages reusable streaming session templates
///
/// # Architecture
///
/// Sessions are **reusable templates** stored in SQLite:
/// - Create once (e.g., "Monday Gaming Stream")
/// - Use many times (each stream creates a new "run")
/// - Full history of all past runs
/// - Auto-save to database on all modifications
///
/// # Performance Notes
///
/// **Known Optimization Opportunities** (deferred for v1):
/// 1. Sessions are cloned on every read (get_session, get_all_sessions)
///    - Current: O(n) memory allocations per query
///    - Future: Store Arc<StreamSession> internally for O(1) clones
/// 2. Manual JSON serialization for database conversion
///    - Current: 14 manual field mappings per save/load
///    - Future: Use Diesel derives or From traits
/// 3. Database write on every update
///    - Current: Full session write even for status changes
///    - Future: Use Diesel's AsChangeset for partial updates
///
/// These optimizations are not critical for typical workloads (<100 sessions)
/// but should be addressed if performance becomes an issue.
///
#[derive(Clone)]
pub struct SessionManager {
    sessions: Arc<RwLock<Vec<Arc<StreamSession>>>>,
    db_pool: Option<Arc<DbPool>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(Vec::new())),
            db_pool: None,
        }
    }

    pub fn with_db(db_pool: Arc<DbPool>) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(Vec::new())),
            db_pool: Some(db_pool),
        }
    }

    /// Load all sessions from database
    pub async fn load_from_db(&self) -> StreamingResult<usize> {
        let db_pool = self
            .db_pool
            .as_ref()
            .ok_or_else(|| StreamingError::Internal("Database pool not configured".to_string()))?;

        let mut conn = db_pool
            .get()
            .map_err(|e| StreamingError::Internal(format!("Database connection error: {}", e)))?;

        let db_sessions: Vec<StreamSessionDb> = tokio::task::spawn_blocking(move || {
            stream_sessions::table.load::<StreamSessionDb>(&mut *conn)
        })
        .await
        .map_err(|e| StreamingError::Internal(format!("Task join error: {}", e)))?
        .map_err(|e| StreamingError::Internal(format!("Database query error: {}", e)))?;

        let mut sessions = Vec::with_capacity(db_sessions.len());
        for record in db_sessions {
            let Ok(mut session) = StreamSession::try_from(record) else {
                continue;
            };
            let old_commands = serde_json::to_value(&session.commands)
                .map_err(|e| StreamingError::Internal(e.to_string()))?;
            normalize_commands(&session.id, &mut session.commands);
            if let Err(error) = validate_commands(&session.commands) {
                // Retain malformed legacy definitions for editing, but fail closed.
                log::warn!(
                    "Session '{}' has invalid legacy commands: {}",
                    session.id,
                    error
                );
                for command in &mut session.commands {
                    command.enabled = false;
                }
            }
            if serde_json::to_value(&session.commands)
                .map_err(|e| StreamingError::Internal(e.to_string()))?
                != old_commands
            {
                self.save_to_db(&session).await?;
            }
            sessions.push(Arc::new(session));
        }

        let count = sessions.len();
        *self.sessions.write().await = sessions;

        log::info!("📊 Loaded {} sessions from database", count);
        Ok(count)
    }

    /// Save a session to database
    async fn save_to_db(&self, session: &StreamSession) -> StreamingResult<()> {
        let db_pool = match &self.db_pool {
            Some(pool) => pool.clone(),
            None => {
                log::warn!("⚠️ Database pool not configured, skipping persistence");
                return Ok(());
            }
        };

        let db_session: StreamSessionDb = session.try_into()?;
        let mut conn = db_pool
            .get()
            .map_err(|e| StreamingError::Internal(format!("Database connection error: {}", e)))?;

        tokio::task::spawn_blocking(move || {
            diesel::replace_into(stream_sessions::table)
                .values(db_session)
                .execute(&mut *conn)
        })
        .await
        .map_err(|e| StreamingError::Internal(format!("Task join error: {}", e)))?
        .map_err(|e| StreamingError::Internal(format!("Database insert error: {}", e)))?;

        log::debug!("💾 Saved session to database: {}", session.id);
        Ok(())
    }

    /// Delete a session from database
    async fn delete_from_db(&self, session_id: &str) -> StreamingResult<()> {
        let db_pool = match &self.db_pool {
            Some(pool) => pool.clone(),
            None => return Ok(()),
        };

        let session_id_str = session_id.to_string();
        let mut conn = db_pool
            .get()
            .map_err(|e| StreamingError::Internal(format!("Database connection error: {}", e)))?;

        tokio::task::spawn_blocking(move || {
            diesel::delete(stream_sessions::table.filter(stream_sessions::id.eq(&session_id_str)))
                .execute(&mut *conn)
        })
        .await
        .map_err(|e| StreamingError::Internal(format!("Task join error: {}", e)))?
        .map_err(|e| StreamingError::Internal(format!("Database delete error: {}", e)))?;

        log::debug!("🗑️ Deleted session from database: {}", session_id);
        Ok(())
    }

    // ============================================================================
    // DATABASE CONVERSION - Using From/TryFrom traits for cleaner code
    // ============================================================================
}

/// Convert database model to service model using TryFrom trait
impl TryFrom<StreamSessionDb> for StreamSession {
    type Error = StreamingError;

    fn try_from(db: StreamSessionDb) -> Result<Self, Self::Error> {
        let platforms = serde_json::from_str(&db.platforms)
            .map_err(|e| StreamingError::DeserializationFailed(format!("platforms: {}", e)))?;

        let commands = serde_json::from_str(&db.commands)
            .map_err(|e| StreamingError::DeserializationFailed(format!("commands: {}", e)))?;

        let settings = serde_json::from_str(&db.settings)
            .map_err(|e| StreamingError::DeserializationFailed(format!("settings: {}", e)))?;

        let status = match db.status.as_str() {
            "draft" => SessionStatus::Draft,
            "waiting_for_live" => SessionStatus::WaitingForLive,
            "active" => SessionStatus::Active,
            "paused" => SessionStatus::Paused,
            "inactive" => SessionStatus::Inactive,
            _ => SessionStatus::Draft,
        };

        let runs = if let Some(runs_str) = db.runs {
            serde_json::from_str(&runs_str)
                .map_err(|e| StreamingError::DeserializationFailed(format!("runs: {}", e)))?
        } else {
            vec![]
        };

        Ok(StreamSession {
            id: db.id,
            name: db.name,
            description: db.description,
            platforms,
            commands,
            settings,
            status,
            created_at: db.created_at,
            updated_at: db.updated_at,
            current_run_started_at: db.current_run_started_at,
            paused_at: db.paused_at,
            duration: db.duration.map(|d| d as u64),
            message_count: db.message_count.map(|c| c as u64),
            command_executions: db.command_executions.map(|c| c as u64),
            runs,
            total_runs: db.total_runs.unwrap_or(0) as u32,
            last_used_at: db.last_used_at,
            total_duration: db.total_duration.map(|d| d as u64),
            total_messages: db.total_messages.map(|m| m as u64),
            total_commands: db.total_commands.map(|c| c as u64),
        })
    }
}

/// Convert service model to database model using TryFrom trait
impl TryFrom<&StreamSession> for StreamSessionDb {
    type Error = StreamingError;

    fn try_from(session: &StreamSession) -> Result<Self, Self::Error> {
        let platforms = serde_json::to_string(&session.platforms)
            .map_err(|e| StreamingError::SerializationFailed(format!("platforms: {}", e)))?;

        let commands = serde_json::to_string(&session.commands)
            .map_err(|e| StreamingError::SerializationFailed(format!("commands: {}", e)))?;

        let settings = serde_json::to_string(&session.settings)
            .map_err(|e| StreamingError::SerializationFailed(format!("settings: {}", e)))?;

        let status = match session.status {
            SessionStatus::Draft => "draft".to_string(),
            SessionStatus::WaitingForLive => "waiting_for_live".to_string(),
            SessionStatus::Active => "active".to_string(),
            SessionStatus::Paused => "paused".to_string(),
            SessionStatus::Inactive => "inactive".to_string(),
        };

        let runs = if session.runs.is_empty() {
            None
        } else {
            Some(
                serde_json::to_string(&session.runs)
                    .map_err(|e| StreamingError::SerializationFailed(format!("runs: {}", e)))?,
            )
        };

        Ok(StreamSessionDb {
            id: session.id.clone(),
            name: session.name.clone(),
            description: session.description.clone(),
            platforms,
            commands,
            settings,
            status,
            created_at: session.created_at.clone(),
            updated_at: session.updated_at.clone(),
            current_run_started_at: session.current_run_started_at.clone(),
            paused_at: session.paused_at.clone(),
            duration: session.duration.map(|d| d as i32),
            message_count: session.message_count.map(|c| c as i32),
            command_executions: session.command_executions.map(|c| c as i32),
            runs,
            total_runs: Some(session.total_runs as i32),
            last_used_at: session.last_used_at.clone(),
            total_duration: session.total_duration.map(|d| d as i32),
            total_messages: session.total_messages.map(|m| m as i32),
            total_commands: session.total_commands.map(|c| c as i32),
        })
    }
}

impl SessionManager {
    // ============================================================================
    // QUERY METHODS
    // ============================================================================
    // NOTE: These methods currently clone sessions for safety. This is inefficient
    // for large session counts or frequent reads. Future optimization: use Arc<StreamSession>
    // internally to return cheap Arc clones instead of deep copies.
    // See: https://github.com/rust-lang/rust/issues/...
    // ============================================================================

    /// Get all sessions (returns Arc clones - cheap operation)
    pub async fn get_all_sessions(&self) -> StreamingResult<Vec<Arc<StreamSession>>> {
        let sessions = self.sessions.read().await;
        Ok(sessions.clone()) // Clones Arc pointers, not session data
    }

    /// Get session by ID (returns Arc clone - cheap operation)
    pub async fn get_session(&self, session_id: &str) -> StreamingResult<Arc<StreamSession>> {
        let sessions = self.sessions.read().await;
        sessions
            .iter()
            .find(|s| s.id == session_id)
            .cloned() // Clones Arc pointer, not session data
            .ok_or_else(|| StreamingError::SessionNotFound(session_id.to_string()))
    }

    /// Create a new session
    pub async fn create_session(
        &self,
        mut session: StreamSession,
    ) -> StreamingResult<Arc<StreamSession>> {
        // Ensure proper defaults
        if session.id.is_empty() {
            session.id = format!(
                "session-{}-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(|e| StreamingError::Internal(format!("System time error: {}", e)))?
                    .as_millis(),
                uuid::Uuid::new_v4()
                    .to_string()
                    .split('-')
                    .next()
                    .ok_or_else(|| StreamingError::Internal(
                        "Failed to generate UUID".to_string()
                    ))?
            );
        }

        session.status = SessionStatus::Draft;
        normalize_commands(&session.id, &mut session.commands);
        validate_commands(&session.commands).map_err(StreamingError::InvalidConfig)?;
        session.created_at = chrono::Utc::now().to_rfc3339();
        session.updated_at = chrono::Utc::now().to_rfc3339();

        // Persist before publishing a session to command dispatch.
        self.save_to_db(&session).await?;
        // Wrap in Arc and add to sessions
        let session_arc = Arc::new(session);
        let mut sessions = self.sessions.write().await;
        sessions.insert(0, session_arc.clone());
        drop(sessions); // Release lock before DB call

        // Persist to database

        log::info!("✅ Created session: {}", session_arc.id);
        Ok(session_arc)
    }

    /// Update existing session (uses clone-on-write for Arc)
    pub async fn update_session(
        &self,
        session_id: &str,
        updates: HashMap<String, serde_json::Value>,
    ) -> StreamingResult<Arc<StreamSession>> {
        let mut sessions = self.sessions.write().await;

        // Find the session index
        let index = sessions
            .iter()
            .position(|s| s.id == session_id)
            .ok_or_else(|| StreamingError::SessionNotFound(session_id.to_string()))?;

        // Clone the Arc'd session to get mutable access (clone-on-write)
        let mut session = (*sessions[index]).clone();

        // Apply updates
        if let Some(name) = updates.get("name") {
            if let Some(name_str) = name.as_str() {
                session.name = name_str.to_string();
            }
        }

        if let Some(description) = updates.get("description") {
            session.description = description.as_str().map(|s| s.to_string());
        }

        if let Some(platforms) = updates.get("platforms") {
            if let Ok(platforms_vec) = serde_json::from_value(platforms.clone()) {
                session.platforms = platforms_vec;
            }
        }

        if let Some(commands) = updates.get("commands") {
            session.commands = serde_json::from_value(commands.clone())
                .map_err(|_| StreamingError::InvalidConfig("Invalid command definitions".into()))?;
            normalize_commands(&session.id, &mut session.commands);
            validate_commands(&session.commands).map_err(StreamingError::InvalidConfig)?;
        }

        if let Some(settings) = updates.get("settings") {
            if let Ok(settings_obj) = serde_json::from_value(settings.clone()) {
                session.settings = settings_obj;
            }
        }

        if let Some(status) = updates.get("status") {
            if let Some(status_str) = status.as_str() {
                session.status = match status_str {
                    "draft" => SessionStatus::Draft,
                    "waiting_for_live" => SessionStatus::WaitingForLive,
                    "active" => SessionStatus::Active,
                    "paused" => SessionStatus::Paused,
                    "inactive" => SessionStatus::Inactive,
                    // Legacy support
                    "ended_by_user" | "ended_auto" => SessionStatus::Inactive,
                    _ => {
                        return Err(StreamingError::InvalidConfig(format!(
                            "Invalid status: {}",
                            status_str
                        )))
                    }
                };
            }
        }

        if let Some(started_at) = updates.get("current_run_started_at") {
            session.current_run_started_at = started_at.as_str().map(|s| s.to_string());
        }

        // Legacy support for old field names
        if let Some(started_at) = updates.get("started_at") {
            session.current_run_started_at = started_at.as_str().map(|s| s.to_string());
        }

        if let Some(paused_at) = updates.get("paused_at") {
            session.paused_at = paused_at.as_str().map(|s| s.to_string());
        }

        if let Some(duration) = updates.get("duration") {
            session.duration = duration.as_u64();
        }

        session.updated_at = chrono::Utc::now().to_rfc3339();

        // Wrap updated session in new Arc and replace
        let updated_arc = Arc::new(session);
        // Serialize updates and publish only after successful persistence.
        self.save_to_db(&updated_arc).await?;
        sessions[index] = updated_arc.clone();
        drop(sessions); // Release lock before DB call

        // Persist to database

        log::info!("✅ Updated session: {}", session_id);
        Ok(updated_arc)
    }

    /// Delete session
    pub async fn delete_session(&self, session_id: &str) -> StreamingResult<()> {
        let mut sessions = self.sessions.write().await;
        let initial_len = sessions.len();

        sessions.retain(|s| s.id != session_id);

        if sessions.len() == initial_len {
            return Err(StreamingError::SessionNotFound(session_id.to_string()));
        }
        drop(sessions); // Release lock before DB call

        // Delete from database
        self.delete_from_db(session_id).await?;

        log::info!("✅ Deleted session: {}", session_id);
        Ok(())
    }

    /// Activate session (move to waiting_for_live state)
    pub async fn activate_session(&self, session_id: &str) -> StreamingResult<Arc<StreamSession>> {
        // Get session to check conflicts
        let session = self.get_session(session_id).await?;

        // Check for conflicts (one account = one active session)
        let conflicts = self.check_session_conflicts(&session.platforms).await?;
        if !conflicts.is_empty() {
            return Err(StreamingError::SessionConflict);
        }

        // Update status
        let mut updates = HashMap::new();
        updates.insert("status".to_string(), serde_json::json!("waiting_for_live"));
        updates.insert(
            "started_at".to_string(),
            serde_json::json!(chrono::Utc::now().to_rfc3339()),
        );

        let updated = self.update_session(session_id, updates).await?;
        log::info!("🎬 Session activated: {}", session_id);
        Ok(updated)
    }

    /// Start session (move to active state when stream goes live)
    pub async fn start_session(&self, session_id: &str) -> StreamingResult<Arc<StreamSession>> {
        let mut updates = HashMap::new();
        updates.insert("status".to_string(), serde_json::json!("active"));
        updates.insert(
            "started_at".to_string(),
            serde_json::json!(chrono::Utc::now().to_rfc3339()),
        );

        let updated = self.update_session(session_id, updates).await?;
        log::info!("🔴 Session started (LIVE): {}", session_id);
        Ok(updated)
    }

    /// Pause session
    pub async fn pause_session(&self, session_id: &str) -> StreamingResult<Arc<StreamSession>> {
        let mut updates = HashMap::new();
        updates.insert("status".to_string(), serde_json::json!("paused"));
        updates.insert(
            "paused_at".to_string(),
            serde_json::json!(chrono::Utc::now().to_rfc3339()),
        );

        let updated = self.update_session(session_id, updates).await?;
        log::info!("⏸️ Session paused: {}", session_id);
        Ok(updated)
    }

    /// Resume session from pause
    pub async fn resume_session(&self, session_id: &str) -> StreamingResult<Arc<StreamSession>> {
        let mut updates = HashMap::new();
        updates.insert("status".to_string(), serde_json::json!("active"));
        updates.insert("paused_at".to_string(), serde_json::json!(null));

        let updated = self.update_session(session_id, updates).await?;
        log::info!("▶️ Session resumed: {}", session_id);
        Ok(updated)
    }

    /// End session
    pub async fn end_session(
        &self,
        session_id: &str,
        auto: bool,
    ) -> StreamingResult<Arc<StreamSession>> {
        let session = self.get_session(session_id).await?;

        let ended_at = chrono::Utc::now();
        let duration = if let Some(started_at) = &session.current_run_started_at {
            let started = chrono::DateTime::parse_from_rfc3339(started_at).map_err(|e| {
                StreamingError::Internal(format!("Invalid started_at format: {}", e))
            })?;
            (ended_at.timestamp_millis() - started.timestamp_millis()) as u64
        } else {
            0
        };

        let mut updates = HashMap::new();
        updates.insert("status".to_string(), serde_json::json!("inactive"));
        updates.insert("duration".to_string(), serde_json::json!(duration));

        let updated = self.update_session(session_id, updates).await?;
        log::info!(
            "⏹️ Session ended: {} {}",
            session_id,
            if auto { "(auto)" } else { "(manual)" }
        );
        Ok(updated)
    }

    /// Check for session conflicts (one account = one active session)
    pub async fn check_session_conflicts(
        &self,
        platforms_to_check: &[PlatformStream],
    ) -> StreamingResult<Vec<Arc<StreamSession>>> {
        let sessions = self.sessions.read().await;

        let conflicts: Vec<Arc<StreamSession>> = sessions
            .iter()
            .filter(|s| {
                matches!(
                    s.status,
                    SessionStatus::Active | SessionStatus::WaitingForLive | SessionStatus::Paused
                )
            })
            .filter(|session| {
                // Check if any platform+account pair conflicts
                session.platforms.iter().any(|session_platform| {
                    platforms_to_check.iter().any(|check_platform| {
                        session_platform.platform == check_platform.platform
                            && session_platform.account_id == check_platform.account_id
                    })
                })
            })
            .cloned()
            .collect();

        Ok(conflicts)
    }

    /// Auto-resume session logic (customizable grace period per session)
    pub async fn check_auto_resume(
        &self,
        platform: &str,
        account_id: &str,
    ) -> StreamingResult<Option<Arc<StreamSession>>> {
        let sessions = self.sessions.read().await;
        let now = chrono::Utc::now();

        // Find recently ended sessions with auto-resume enabled
        let mut candidates: Vec<&Arc<StreamSession>> = sessions
            .iter()
            .filter(|s| {
                // Must have auto-resume enabled
                if !s.settings.auto_resume {
                    return false;
                }

                // Must be in inactive or paused state
                if !matches!(s.status, SessionStatus::Inactive | SessionStatus::Paused) {
                    return false;
                }

                // Check if this session has the platform+account pair
                let has_platform_account = s
                    .platforms
                    .iter()
                    .any(|p| p.platform.to_string() == platform && p.account_id == *account_id);
                if !has_platform_account {
                    return false;
                }

                // Check grace period for paused sessions
                if s.status == SessionStatus::Paused {
                    if let Some(paused_at) = &s.paused_at {
                        if let Ok(paused_time) = chrono::DateTime::parse_from_rfc3339(paused_at) {
                            let time_since_pause = now.signed_duration_since(paused_time);
                            let grace_period_minutes =
                                s.settings.grace_period_minutes.unwrap_or(30);
                            let grace_period =
                                chrono::Duration::minutes(grace_period_minutes as i64);

                            return time_since_pause <= grace_period;
                        }
                    }
                }

                // For inactive sessions, allow resume anytime (reusable)
                true
            })
            .collect();

        if candidates.is_empty() {
            return Ok(None);
        }

        // Sort by priority (higher first), then by most recently ended
        candidates.sort_by(|a, b| {
            let priority_a = a.settings.priority.unwrap_or(0);
            let priority_b = b.settings.priority.unwrap_or(0);

            // Higher priority wins
            if priority_b != priority_a {
                return priority_b.cmp(&priority_a);
            }

            // Same priority → most recently used wins
            let last_used_a = a
                .last_used_at
                .as_ref()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok());
            let last_used_b = b
                .last_used_at
                .as_ref()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok());

            match (last_used_a, last_used_b) {
                (Some(a), Some(b)) => b.cmp(&a),
                _ => std::cmp::Ordering::Equal,
            }
        });

        let session_to_resume = (*candidates[0]).clone();
        log::info!(
            "🔄 Auto-resume candidate found: {} (priority: {}, grace: {}min)",
            session_to_resume.id,
            session_to_resume.settings.priority.unwrap_or(0),
            session_to_resume
                .settings
                .grace_period_minutes
                .unwrap_or(30)
        );

        Ok(Some(session_to_resume))
    }

    /// Find session by account_id (platform + account pair)
    pub async fn find_session_by_account_id(
        &self,
        platform: &str,
        account_id: &str,
    ) -> StreamingResult<Option<Arc<StreamSession>>> {
        let sessions = self.sessions.read().await;

        log::debug!(
            "🔍 [FIND SESSION] Looking for platform='{}', account_id='{}'",
            platform,
            account_id
        );
        log::debug!("🔍 [FIND SESSION] Total sessions: {}", sessions.len());

        for (i, s) in sessions.iter().enumerate() {
            log::debug!(
                "🔍 [FIND SESSION] Session {}: id={}, status={:?}, platforms={:?}",
                i,
                s.id,
                s.status,
                s.platforms
                    .iter()
                    .map(|p| format!("{}:{}", p.platform, p.account_id))
                    .collect::<Vec<_>>()
            );
        }

        let session = sessions.iter()
            .find(|s| {
                // Only search in active/waiting/paused sessions
                if !matches!(s.status, SessionStatus::Active | SessionStatus::WaitingForLive | SessionStatus::Paused) {
                    log::debug!("🔍 [FIND SESSION] Session {} skipped (status={:?})", s.id, s.status);
                    return false;
                }

                let found = s.platforms.iter().any(|p| {
                    let platform_match = p.platform.to_string() == platform;
                    let account_match = p.account_id == account_id;
                    log::debug!("🔍 [FIND SESSION] Checking session {} platform: {}=={} ({}), account: '{}'=='{}' ({})",
                        s.id, p.platform, platform, platform_match, p.account_id, account_id, account_match);
                    platform_match && account_match
                });

                if found {
                    log::info!("✅ [FIND SESSION] Found matching session: {}", s.id);
                }

                found
            })
            .cloned();

        if session.is_none() {
            log::warn!(
                "⚠️ [FIND SESSION] No session found for platform='{}', account_id='{}'",
                platform,
                account_id
            );
        }

        Ok(session)
    }

    /// Check if a session has any live streams (stream_id present)
    pub async fn is_session_live(&self, session_id: &str) -> StreamingResult<bool> {
        let session = self.get_session(session_id).await?;
        Ok(session.platforms.iter().any(|p| p.stream_id.is_some()))
    }

    /// Get all account IDs from a session
    pub async fn get_session_account_ids(&self, session_id: &str) -> StreamingResult<Vec<String>> {
        let session = self.get_session(session_id).await?;
        let account_ids: Vec<String> = session
            .platforms
            .iter()
            .map(|p| p.account_id.clone())
            .collect();
        Ok(account_ids)
    }

    /// Get active sessions
    pub async fn get_active_sessions(&self) -> StreamingResult<Vec<Arc<StreamSession>>> {
        let sessions = self.sessions.read().await;
        Ok(sessions
            .iter()
            .filter(|s| {
                matches!(
                    s.status,
                    SessionStatus::Active | SessionStatus::WaitingForLive | SessionStatus::Paused
                )
            })
            .cloned()
            .collect())
    }

    /// Get draft sessions
    pub async fn get_draft_sessions(&self) -> StreamingResult<Vec<Arc<StreamSession>>> {
        let sessions = self.sessions.read().await;
        Ok(sessions
            .iter()
            .filter(|s| matches!(s.status, SessionStatus::Draft))
            .cloned()
            .collect())
    }

    /// Get recent inactive sessions
    pub async fn get_recent_sessions(
        &self,
        limit: usize,
    ) -> StreamingResult<Vec<Arc<StreamSession>>> {
        let sessions = self.sessions.read().await;
        let mut recent: Vec<Arc<StreamSession>> = sessions
            .iter()
            .filter(|s| matches!(s.status, SessionStatus::Inactive))
            .cloned()
            .collect();

        recent.sort_by(|a, b| {
            let last_used_a = a
                .last_used_at
                .as_ref()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok());
            let last_used_b = b
                .last_used_at
                .as_ref()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok());

            match (last_used_a, last_used_b) {
                (Some(a), Some(b)) => b.cmp(&a),
                _ => std::cmp::Ordering::Equal,
            }
        });

        Ok(recent.into_iter().take(limit).collect())
    }

    /// Increment the command execution counter for a session
    pub async fn increment_command_executions(
        &self,
        session_id: &str,
    ) -> StreamingResult<Arc<StreamSession>> {
        let session = self.get_session(session_id).await?;
        self.increment_command_executions_for_run(
            session_id,
            session.current_run_started_at.as_deref(),
        )
        .await
    }

    pub async fn increment_command_executions_for_run(
        &self,
        session_id: &str,
        expected_run: Option<&str>,
    ) -> StreamingResult<Arc<StreamSession>> {
        let mut sessions = self.sessions.write().await;
        let index = sessions
            .iter()
            .position(|session| session.id == session_id)
            .ok_or_else(|| StreamingError::SessionNotFound(session_id.into()))?;
        if sessions[index].current_run_started_at.as_deref() != expected_run {
            return Ok(sessions[index].clone());
        }
        let mut session = (*sessions[index]).clone();
        session.command_executions =
            Some(session.command_executions.unwrap_or(0).saturating_add(1));
        session.updated_at = chrono::Utc::now().to_rfc3339();
        self.save_to_db(&session).await?;
        let session = Arc::new(session);
        sessions[index] = session.clone();
        Ok(session)
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::streaming::models::{Platform, SessionCommand};

    fn create_test_session() -> StreamSession {
        StreamSession {
            id: "test-session-1".to_string(),
            name: "Test Session".to_string(),
            description: Some("Test description".to_string()),
            platforms: vec![PlatformStream {
                platform: Platform::Twitch,
                account_id: "12345".to_string(),
                stream_id: None,
                title: None,
                started_at: None,
                viewer_count: None,
            }],
            commands: vec![],
            settings: SessionSettings {
                auto_start: true,
                notifications: true,
                auto_resume: true,
                grace_period_minutes: Some(30),
                priority: Some(0),
            },
            status: SessionStatus::Draft,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            current_run_started_at: None,
            paused_at: None,
            duration: None,
            message_count: None,
            command_executions: None,
            runs: vec![],
            total_runs: 0,
            last_used_at: None,
            total_duration: None,
            total_messages: None,
            total_commands: None,
        }
    }

    #[tokio::test]
    async fn test_create_and_get_session() {
        let manager = SessionManager::new();
        let session = create_test_session();

        let created = manager.create_session(session.clone()).await.unwrap();
        assert_eq!(created.name, "Test Session");
        assert_eq!(created.status, SessionStatus::Draft);

        let retrieved = manager.get_session(&created.id).await.unwrap();
        assert_eq!(retrieved.id, created.id);
    }

    #[tokio::test]
    async fn test_activate_session() {
        let manager = SessionManager::new();
        let session = create_test_session();
        let created = manager.create_session(session).await.unwrap();

        let activated = manager.activate_session(&created.id).await.unwrap();
        assert_eq!(activated.status, SessionStatus::WaitingForLive);
        assert!(activated.current_run_started_at.is_some());
    }

    #[tokio::test]
    async fn test_session_conflicts() {
        let manager = SessionManager::new();

        // Create and activate first session
        let session1 = create_test_session();
        let created1 = manager.create_session(session1).await.unwrap();
        manager.activate_session(&created1.id).await.unwrap();

        // Try to activate second session with same platform+account
        let session2 = create_test_session();
        let created2 = manager.create_session(session2).await.unwrap();
        let result = manager.activate_session(&created2.id).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            StreamingError::SessionConflict
        ));
    }

    #[tokio::test]
    async fn test_session_lifecycle() {
        let manager = SessionManager::new();
        let session = create_test_session();
        let created = manager.create_session(session).await.unwrap();

        // Activate
        let activated = manager.activate_session(&created.id).await.unwrap();
        assert_eq!(activated.status, SessionStatus::WaitingForLive);

        // Start
        let started = manager.start_session(&created.id).await.unwrap();
        assert_eq!(started.status, SessionStatus::Active);

        // Pause
        let paused = manager.pause_session(&created.id).await.unwrap();
        assert_eq!(paused.status, SessionStatus::Paused);

        // Resume
        let resumed = manager.resume_session(&created.id).await.unwrap();
        assert_eq!(resumed.status, SessionStatus::Active);

        // End
        let ended = manager.end_session(&created.id, false).await.unwrap();
        assert_eq!(ended.status, SessionStatus::Inactive);
        assert!(ended.duration.is_some());
    }
}
