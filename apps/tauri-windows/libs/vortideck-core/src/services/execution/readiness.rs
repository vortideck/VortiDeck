//! Readiness barrier for the canonical action runtime.
//!
//! Tauri publishes service handles synchronously during setup, while provider
//! discovery and recovery require asynchronous work. This barrier lets every
//! action surface share that async lifecycle without blocking a Tokio worker or
//! observing a partially published integration catalog.

use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::sync::watch;

const DEFAULT_READINESS_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionRuntimeStatus {
    Initializing,
    Ready,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ActionRuntimeReadinessSnapshot {
    pub status: ActionRuntimeStatus,
    pub phase: String,
    pub warnings: Vec<String>,
    pub error: Option<String>,
}

impl Default for ActionRuntimeReadinessSnapshot {
    fn default() -> Self {
        Self {
            status: ActionRuntimeStatus::Initializing,
            phase: "created".to_string(),
            warnings: Vec::new(),
            error: None,
        }
    }
}

pub struct ActionRuntimeReadiness {
    state: watch::Sender<ActionRuntimeReadinessSnapshot>,
    wait_timeout: Duration,
}

impl Default for ActionRuntimeReadiness {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionRuntimeReadiness {
    pub fn new() -> Self {
        Self::with_timeout(DEFAULT_READINESS_TIMEOUT)
    }

    pub fn with_timeout(wait_timeout: Duration) -> Self {
        let (state, _) = watch::channel(ActionRuntimeReadinessSnapshot::default());
        Self {
            state,
            wait_timeout,
        }
    }

    pub fn snapshot(&self) -> ActionRuntimeReadinessSnapshot {
        self.state.borrow().clone()
    }

    pub fn begin_phase(&self, phase: impl Into<String>) -> ActionRuntimeReadinessSnapshot {
        let mut snapshot = self.snapshot();
        if snapshot.status == ActionRuntimeStatus::Initializing {
            snapshot.phase = phase.into();
            self.state.send_replace(snapshot.clone());
        }
        snapshot
    }

    pub fn mark_ready(&self, warnings: Vec<String>) -> ActionRuntimeReadinessSnapshot {
        let snapshot = ActionRuntimeReadinessSnapshot {
            status: ActionRuntimeStatus::Ready,
            phase: "ready".to_string(),
            warnings,
            error: None,
        };
        self.state.send_replace(snapshot.clone());
        snapshot
    }

    pub fn mark_failed(&self, error: impl Into<String>) -> ActionRuntimeReadinessSnapshot {
        let snapshot = ActionRuntimeReadinessSnapshot {
            status: ActionRuntimeStatus::Failed,
            phase: "failed".to_string(),
            warnings: Vec::new(),
            error: Some(error.into()),
        };
        self.state.send_replace(snapshot.clone());
        snapshot
    }

    /// Wait for the provider catalog to be fully published. A bounded wait
    /// prevents a crashed bootstrap task from hanging an action forever.
    pub async fn wait_until_ready(&self) -> Result<(), String> {
        let mut receiver = self.state.subscribe();
        let wait = async {
            loop {
                let snapshot = receiver.borrow().clone();
                match snapshot.status {
                    ActionRuntimeStatus::Ready => return Ok(()),
                    ActionRuntimeStatus::Failed => {
                        return Err(snapshot.error.unwrap_or_else(|| {
                            "Action runtime initialization failed".to_string()
                        }));
                    }
                    ActionRuntimeStatus::Initializing => {}
                }
                receiver.changed().await.map_err(|_| {
                    "Action runtime readiness channel closed during initialization".to_string()
                })?;
            }
        };

        tokio::time::timeout(self.wait_timeout, wait)
            .await
            .map_err(|_| {
                format!(
                    "Action runtime did not become ready within {} seconds",
                    self.wait_timeout.as_secs()
                )
            })?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn concurrent_waiters_are_released_by_one_ready_transition() {
        let readiness = Arc::new(ActionRuntimeReadiness::new());
        let first = {
            let readiness = readiness.clone();
            tokio::spawn(async move { readiness.wait_until_ready().await })
        };
        let second = {
            let readiness = readiness.clone();
            tokio::spawn(async move { readiness.wait_until_ready().await })
        };

        readiness.begin_phase("publishing_catalog");
        readiness.mark_ready(vec!["optional provider unavailable".to_string()]);

        assert!(first.await.unwrap().is_ok());
        assert!(second.await.unwrap().is_ok());
        assert_eq!(readiness.snapshot().warnings.len(), 1);
    }

    #[tokio::test]
    async fn failed_transition_returns_the_bootstrap_error() {
        let readiness = ActionRuntimeReadiness::new();
        readiness.mark_failed("catalog publication failed");

        assert_eq!(
            readiness.wait_until_ready().await.unwrap_err(),
            "catalog publication failed"
        );
    }

    #[tokio::test]
    async fn a_missing_transition_times_out_instead_of_hanging_forever() {
        let readiness = ActionRuntimeReadiness::with_timeout(Duration::from_millis(5));

        assert!(readiness
            .wait_until_ready()
            .await
            .unwrap_err()
            .contains("did not become ready"));
    }
}
