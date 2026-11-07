//! Durable workflow scheduling records.
//!
//! A flow run may have several concurrent nodes waiting at once. These records
//! are therefore keyed per wait/node. Overlay delivery and side-effect outbox
//! tables provide the equivalent per-operation authority for external work.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

pub const CURRENT_RESUME_POLICY_VERSION: u16 = 1;

/// Durable waits always store an absolute UTC instant. Local timezone/DST is a
/// schedule-authoring concern and is resolved before a wake is persisted.
pub trait WorkflowClock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

#[derive(Default)]
pub struct SystemWorkflowClock;

impl WorkflowClock for SystemWorkflowClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// Re-check wall time periodically rather than trusting one long monotonic
/// sleep. This makes forward clock changes and machine sleep observe an
/// already-passed deadline promptly. A backward change delays until the same
/// absolute UTC instant; it never causes duplicate firing.
pub async fn wait_until_wall_clock_deadline(
    clock: Arc<dyn WorkflowClock>,
    deadline: DateTime<Utc>,
) {
    wait_until_wall_clock_deadline_with_poll(clock, deadline, std::time::Duration::from_secs(1))
        .await;
}

async fn wait_until_wall_clock_deadline_with_poll(
    clock: Arc<dyn WorkflowClock>,
    deadline: DateTime<Utc>,
    max_poll: std::time::Duration,
) {
    loop {
        let remaining = deadline.signed_duration_since(clock.now());
        let Ok(remaining) = remaining.to_std() else {
            return;
        };
        if remaining.is_zero() {
            return;
        }
        tokio::time::sleep(remaining.min(max_poll)).await;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResumeGraphPolicy {
    PinnedRunSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResumeAuthorityPolicy {
    RevalidateBeforeEffect,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MissedDeadlinePolicy {
    ResumeImmediately,
}

/// Policy identity stored with every wait. The graph itself lives in the run
/// snapshot; this hash prevents a wake from being attached to a different or
/// incompatible snapshot. Missing legacy policy is fail-closed at recovery.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowResumePolicy {
    pub version: u16,
    pub flow_revision: String,
    pub graph: ResumeGraphPolicy,
    pub authority: ResumeAuthorityPolicy,
    pub schedule_timezone: String,
    pub missed_deadline: MissedDeadlinePolicy,
}

impl WorkflowResumePolicy {
    pub fn pinned(flow_revision: String) -> Self {
        Self {
            version: CURRENT_RESUME_POLICY_VERSION,
            flow_revision,
            graph: ResumeGraphPolicy::PinnedRunSnapshot,
            authority: ResumeAuthorityPolicy::RevalidateBeforeEffect,
            schedule_timezone: "UTC".to_string(),
            missed_deadline: MissedDeadlinePolicy::ResumeImmediately,
        }
    }

    pub fn is_current(&self) -> bool {
        self.version == CURRENT_RESUME_POLICY_VERSION && !self.flow_revision.is_empty()
    }
}

impl Default for WorkflowResumePolicy {
    fn default() -> Self {
        // Legacy rows have no evidence that they were created under the current
        // contract. They remain inspectable but cannot auto-resume.
        Self {
            version: 0,
            flow_revision: String::new(),
            graph: ResumeGraphPolicy::PinnedRunSnapshot,
            authority: ResumeAuthorityPolicy::RevalidateBeforeEffect,
            schedule_timezone: "UTC".to_string(),
            missed_deadline: MissedDeadlinePolicy::ResumeImmediately,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowWakeKind {
    Retry,
    Approval,
    Timer,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowWakeStatus {
    Waiting,
    Ready,
    Consumed,
    Cancelled,
}

/// A terminal answer to a human approval request. Approval is deliberately
/// richer than a boolean so timeout and cancellation can never be mistaken for
/// an explicit denial in audit logs or workflow routing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecision {
    Approved,
    Denied,
    TimedOut,
    Cancelled,
}

impl ApprovalDecision {
    pub fn route(&self) -> &'static str {
        match self {
            Self::Approved => "approved",
            Self::Denied => "denied",
            Self::TimedOut => "timed_out",
            Self::Cancelled => "cancelled",
        }
    }
}

/// Authenticated identity and transport that supplied a decision. The core
/// does not trust a transport name as authorization; adapters must authenticate
/// the actor before calling the approval service.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalActor {
    pub id: String,
    pub display_name: Option<String>,
    pub channel: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalResolution {
    pub decision: ApprovalDecision,
    pub decided_at: DateTime<Utc>,
    pub decided_by: ApprovalActor,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalDecisionRequest {
    pub decision: ApprovalDecision,
    pub actor: ApprovalActor,
    pub comment: Option<String>,
}

fn default_approval_decision_channels() -> Vec<String> {
    vec![
        "desktop".to_string(),
        "mobile_direct".to_string(),
        "mobile_relay".to_string(),
    ]
}

/// Authorization boundary for an approval. Notification delivery and decision
/// authority are intentionally separate: an email or webhook may announce a
/// request, but only an authenticated adapter named here may resolve it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalDecisionPolicy {
    #[serde(default = "default_approval_decision_channels")]
    pub allowed_channels: Vec<String>,
}

impl Default for ApprovalDecisionPolicy {
    fn default() -> Self {
        Self {
            allowed_channels: default_approval_decision_channels(),
        }
    }
}

impl ApprovalDecisionPolicy {
    pub fn permits(&self, actor: &ApprovalActor) -> bool {
        self.allowed_channels
            .iter()
            .any(|channel| channel == &actor.channel)
    }
}

/// Stable, transport-neutral request body persisted inside an approval wake.
/// New notification transports can consume this object without changing the
/// workflow executor or the decision protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalRequestDetails {
    pub title: String,
    pub message: String,
    pub severity: String,
    #[serde(default)]
    pub requested_channels: Vec<String>,
    #[serde(default)]
    pub request_snapshot: Value,
    /// Resolved effects reachable through the approved control branch. The
    /// executor compares these hashes again before accepting the decision and
    /// immediately before dispatch, so an edit cannot inherit old consent.
    #[serde(default)]
    pub approved_effects: Vec<ApprovedEffectBinding>,
    #[serde(default)]
    pub decision_policy: ApprovalDecisionPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApprovedEffectBinding {
    pub node_id: String,
    pub action_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "outcome")]
pub enum ApprovalResolveOutcome {
    Applied { wake: WorkflowWake },
    AlreadyResolved { wake: WorkflowWake },
}

impl ApprovalResolveOutcome {
    pub fn wake(&self) -> &WorkflowWake {
        match self {
            Self::Applied { wake } | Self::AlreadyResolved { wake } => wake,
        }
    }

    pub fn was_applied(&self) -> bool {
        matches!(self, Self::Applied { .. })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowWake {
    pub id: String,
    pub run_id: String,
    pub node_id: String,
    pub kind: WorkflowWakeKind,
    pub status: WorkflowWakeStatus,
    /// Timed wakes become ready at this absolute instant. Approval wakes have
    /// no deadline until an explicit decision makes them ready.
    pub due_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub attempt: u32,
    #[serde(default)]
    pub payload: Value,
    #[serde(default)]
    pub resume_policy: WorkflowResumePolicy,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowResourceLease {
    pub resource_key: String,
    pub owner_run_id: String,
    pub owner_node_id: String,
    pub lease_token: String,
    pub expires_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::sync::RwLock;

    struct ManualClock(RwLock<DateTime<Utc>>);

    impl ManualClock {
        fn new(now: DateTime<Utc>) -> Self {
            Self(RwLock::new(now))
        }

        fn set(&self, now: DateTime<Utc>) {
            *self.0.write().expect("manual clock write") = now;
        }
    }

    impl WorkflowClock for ManualClock {
        fn now(&self) -> DateTime<Utc> {
            *self.0.read().expect("manual clock read")
        }
    }

    #[test]
    fn approval_policy_uses_authenticated_transport_identity() {
        let policy = ApprovalDecisionPolicy {
            allowed_channels: vec!["mobile_relay".to_string()],
        };
        let relay_actor = ApprovalActor {
            id: "paired-phone".to_string(),
            display_name: None,
            channel: "mobile_relay".to_string(),
        };
        let direct_actor = ApprovalActor {
            channel: "mobile_direct".to_string(),
            ..relay_actor.clone()
        };

        assert!(policy.permits(&relay_actor));
        assert!(!policy.permits(&direct_actor));
    }

    #[tokio::test]
    async fn wall_clock_wait_observes_sleep_or_forward_clock_jump() {
        let start = Utc.with_ymd_and_hms(2026, 3, 1, 10, 0, 0).unwrap();
        let clock = Arc::new(ManualClock::new(start));
        let deadline = start + chrono::Duration::hours(2);
        let waiter = tokio::spawn(wait_until_wall_clock_deadline_with_poll(
            clock.clone(),
            deadline,
            std::time::Duration::from_millis(5),
        ));

        tokio::task::yield_now().await;
        clock.set(deadline + chrono::Duration::seconds(1));
        tokio::time::timeout(std::time::Duration::from_millis(100), waiter)
            .await
            .expect("forward wall-clock change must release the wait")
            .expect("wait task must not panic");
    }

    #[tokio::test]
    async fn backward_clock_change_does_not_fire_a_deadline_early() {
        let start = Utc.with_ymd_and_hms(2026, 3, 1, 10, 0, 0).unwrap();
        let clock = Arc::new(ManualClock::new(start));
        let deadline = start + chrono::Duration::minutes(1);
        let waiter = tokio::spawn(wait_until_wall_clock_deadline_with_poll(
            clock.clone(),
            deadline,
            std::time::Duration::from_millis(5),
        ));

        clock.set(start - chrono::Duration::hours(1));
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        assert!(!waiter.is_finished());
        clock.set(deadline);
        tokio::time::timeout(std::time::Duration::from_millis(100), waiter)
            .await
            .expect("absolute UTC deadline must eventually release")
            .expect("wait task must not panic");
    }

    #[test]
    fn dst_fallback_instants_remain_distinct_after_utc_persistence() {
        let summer_offset = chrono::FixedOffset::east_opt(2 * 3600).unwrap();
        let winter_offset = chrono::FixedOffset::east_opt(3600).unwrap();
        let first = summer_offset
            .with_ymd_and_hms(2026, 10, 25, 2, 30, 0)
            .unwrap()
            .with_timezone(&Utc);
        let second = winter_offset
            .with_ymd_and_hms(2026, 10, 25, 2, 30, 0)
            .unwrap()
            .with_timezone(&Utc);

        assert_ne!(first, second);
        assert_eq!(second - first, chrono::Duration::hours(1));
    }
}
