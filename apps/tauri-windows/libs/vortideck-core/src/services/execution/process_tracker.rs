use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::process::Child;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{timeout, Duration};
use uuid::Uuid;

// ============================================================================
// PROCESS TRACKING TYPES
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedProcess {
    pub id: Uuid,
    pub command: String,
    pub args: Vec<String>,
    pub started_at: DateTime<Utc>,
    pub timeout: Option<Duration>,
    pub status: ProcessStatus,
    pub resource_usage: ResourceUsage,
    pub working_dir: Option<String>,
    pub button_id: Option<String>,
    pub deck_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessStatus {
    Running,
    Completed {
        exit_code: i32,
        finished_at: DateTime<Utc>,
    },
    TimedOut {
        finished_at: DateTime<Utc>,
    },
    Killed {
        finished_at: DateTime<Utc>,
    },
    Failed {
        error: String,
        finished_at: DateTime<Utc>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourceUsage {
    pub cpu_percent: f32,
    pub memory_bytes: u64,
    pub peak_memory_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub execution_time_ms: u64,
}

#[derive(Debug, Clone)]
pub enum ProcessError {
    ProcessNotFound,
    KillFailed(String),
    Timeout,
    ExecutionFailed(String),
    InvalidParameters(String),
}

impl std::fmt::Display for ProcessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProcessError::ProcessNotFound => write!(f, "Process not found"),
            ProcessError::KillFailed(msg) => write!(f, "Failed to kill process: {}", msg),
            ProcessError::Timeout => write!(f, "Process execution timed out"),
            ProcessError::ExecutionFailed(msg) => write!(f, "Process execution failed: {}", msg),
            ProcessError::InvalidParameters(msg) => write!(f, "Invalid parameters: {}", msg),
        }
    }
}

impl std::error::Error for ProcessError {}

// ============================================================================
// PROCESS TRACKER IMPLEMENTATION
// ============================================================================

struct ManagedProcess {
    child: Arc<Mutex<Option<Child>>>,
    info: TrackedProcess,
}

pub struct ProcessTracker {
    processes: Arc<RwLock<HashMap<Uuid, ManagedProcess>>>,
    resource_monitor: Arc<ResourceMonitor>,
    cleanup_interval: Duration,
}

impl ProcessTracker {
    pub fn new() -> Self {
        let tracker = Self {
            processes: Arc::new(RwLock::new(HashMap::new())),
            resource_monitor: Arc::new(ResourceMonitor::new()),
            cleanup_interval: Duration::from_secs(30), // Cleanup every 30 seconds
        };

        // Start background monitoring and cleanup tasks
        tracker.start_monitoring();
        tracker.start_cleanup();

        tracker
    }

    /// Register a new process for tracking
    pub async fn register(
        &self,
        command: String,
        args: Vec<String>,
        mut child: Child,
        timeout_ms: Option<u64>,
        working_dir: Option<String>,
        button_id: Option<String>,
        deck_id: Option<String>,
    ) -> Uuid {
        let process_id = Uuid::new_v4();
        let now = Utc::now();

        println!(
            "📝 Registering process {}: {} {:?}",
            process_id, command, args
        );

        let tracked_process = TrackedProcess {
            id: process_id,
            command: command.clone(),
            args: args.clone(),
            started_at: now,
            timeout: timeout_ms.map(Duration::from_millis),
            status: ProcessStatus::Running,
            resource_usage: ResourceUsage::default(),
            working_dir,
            button_id,
            deck_id,
        };

        let managed_process = ManagedProcess {
            child: Arc::new(Mutex::new(Some(child))),
            info: tracked_process,
        };

        self.processes
            .write()
            .await
            .insert(process_id, managed_process);

        // Set up timeout if specified
        if let Some(timeout_duration) = timeout_ms.map(Duration::from_millis) {
            let processes = self.processes.clone();
            tokio::spawn(async move {
                tokio::time::sleep(timeout_duration).await;

                // Check if process is still running and kill it if timeout reached
                if let Some(managed_process) = processes.write().await.get_mut(&process_id) {
                    if matches!(managed_process.info.status, ProcessStatus::Running) {
                        println!(
                            "⏰ Process {} timed out after {:?}",
                            process_id, timeout_duration
                        );

                        if let Some(mut child) = managed_process.child.lock().await.take() {
                            let _ = child.kill().await;
                            managed_process.info.status = ProcessStatus::TimedOut {
                                finished_at: Utc::now(),
                            };
                        }
                    }
                }
            });
        }

        process_id
    }

    /// Wait for a process to complete and get its result
    pub async fn wait_for_completion(&self, id: Uuid) -> Result<ProcessResult, ProcessError> {
        let (child_arc, start_time) = {
            let processes = self.processes.read().await;
            let managed_process = processes.get(&id).ok_or(ProcessError::ProcessNotFound)?;
            (
                managed_process.child.clone(),
                managed_process.info.started_at,
            )
        };

        // Wait for the process to complete
        let output = {
            let mut child_guard = child_arc.lock().await;
            match child_guard.take() {
                Some(mut child) => child
                    .wait_with_output()
                    .await
                    .map_err(|e| ProcessError::ExecutionFailed(e.to_string()))?,
                _ => {
                    return Err(ProcessError::ProcessNotFound);
                }
            }
        };

        let execution_time_ms = (Utc::now() - start_time).num_milliseconds() as u64;
        let exit_code = output.status.code().unwrap_or(-1);

        // Update process status
        {
            let mut processes = self.processes.write().await;
            if let Some(managed_process) = processes.get_mut(&id) {
                managed_process.info.status = ProcessStatus::Completed {
                    exit_code,
                    finished_at: Utc::now(),
                };
            }
        }

        Ok(ProcessResult {
            exit_code,
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            execution_time_ms,
        })
    }

    /// Kill a specific process
    pub async fn kill_process(&self, id: Uuid) -> Result<(), ProcessError> {
        let mut processes = self.processes.write().await;

        if let Some(managed_process) = processes.get_mut(&id) {
            let mut child_guard = managed_process.child.lock().await;
            match child_guard.take() {
                Some(mut child) => {
                    child
                        .kill()
                        .await
                        .map_err(|e| ProcessError::KillFailed(e.to_string()))?;
                    managed_process.info.status = ProcessStatus::Killed {
                        finished_at: Utc::now(),
                    };
                    println!("🔪 Killed process {}", id);
                    Ok(())
                }
                _ => Err(ProcessError::ProcessNotFound),
            }
        } else {
            Err(ProcessError::ProcessNotFound)
        }
    }

    /// Kill all running processes
    pub async fn kill_all(&self) -> Vec<(Uuid, Result<(), ProcessError>)> {
        let process_ids: Vec<Uuid> = {
            self.processes
                .read()
                .await
                .iter()
                .filter(|(_, process)| matches!(process.info.status, ProcessStatus::Running))
                .map(|(id, _)| *id)
                .collect()
        };

        println!("🔪 Killing {} running processes", process_ids.len());

        let mut results = Vec::new();
        for id in process_ids {
            let result = self.kill_process(id).await;
            results.push((id, result));
        }
        results
    }

    /// Get status of all processes
    pub async fn get_all_processes(&self) -> HashMap<Uuid, TrackedProcess> {
        self.processes
            .read()
            .await
            .iter()
            .map(|(id, managed_process)| (*id, managed_process.info.clone()))
            .collect()
    }

    /// Get running processes only
    pub async fn get_running_processes(&self) -> HashMap<Uuid, TrackedProcess> {
        self.processes
            .read()
            .await
            .iter()
            .filter(|(_, managed_process)| {
                matches!(managed_process.info.status, ProcessStatus::Running)
            })
            .map(|(id, managed_process)| (*id, managed_process.info.clone()))
            .collect()
    }

    /// Get process by ID
    pub async fn get_process(&self, id: Uuid) -> Option<TrackedProcess> {
        self.processes
            .read()
            .await
            .get(&id)
            .map(|managed_process| managed_process.info.clone())
    }

    /// Get process count
    pub async fn get_process_count(&self) -> (usize, usize) {
        let processes = self.processes.read().await;
        let total = processes.len();
        let running = processes
            .values()
            .filter(|p| matches!(p.info.status, ProcessStatus::Running))
            .count();
        (total, running)
    }

    /// Start background resource monitoring
    fn start_monitoring(&self) {
        let processes = self.processes.clone();
        let monitor = self.resource_monitor.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));

            loop {
                interval.tick().await;

                // Update resource usage for all running processes
                let mut procs = processes.write().await;
                for (id, managed_process) in procs.iter_mut() {
                    if matches!(managed_process.info.status, ProcessStatus::Running) {
                        // Get process ID from child if available
                        if let Ok(child_guard) = managed_process.child.try_lock() {
                            if let Some(child) = child_guard.as_ref() {
                                if let Some(pid) = child.id() {
                                    managed_process.info.resource_usage =
                                        monitor.get_usage(pid).await;
                                }
                            }
                        }
                    }
                }
            }
        });
    }

    /// Start background cleanup of completed processes
    fn start_cleanup(&self) {
        let processes = self.processes.clone();
        let cleanup_interval = self.cleanup_interval;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(cleanup_interval);

            loop {
                interval.tick().await;

                let mut procs = processes.write().await;
                let initial_count = procs.len();

                // Remove completed processes older than 5 minutes
                let cutoff_time = Utc::now() - chrono::Duration::minutes(5);
                procs.retain(|id, managed_process| {
                    match &managed_process.info.status {
                        ProcessStatus::Running => true, // Keep running processes
                        ProcessStatus::Completed { finished_at, .. }
                        | ProcessStatus::TimedOut { finished_at }
                        | ProcessStatus::Killed { finished_at }
                        | ProcessStatus::Failed { finished_at, .. } => {
                            if *finished_at < cutoff_time {
                                println!("🧹 Cleaning up old process {}", id);
                                false // Remove old completed processes
                            } else {
                                true // Keep recent completed processes
                            }
                        }
                    }
                });

                let final_count = procs.len();
                if initial_count > final_count {
                    println!(
                        "🧹 Cleaned up {} old processes ({} → {})",
                        initial_count - final_count,
                        initial_count,
                        final_count
                    );
                }
            }
        });
    }
}

// ============================================================================
// RESOURCE MONITORING
// ============================================================================

/// Platform-specific resource monitoring
pub struct ResourceMonitor;

impl ResourceMonitor {
    pub fn new() -> Self {
        Self
    }

    pub async fn get_usage(&self, pid: u32) -> ResourceUsage {
        // Platform-specific implementation
        #[cfg(target_os = "windows")]
        {
            self.get_windows_usage(pid).await
        }

        #[cfg(target_os = "linux")]
        {
            self.get_linux_usage(pid).await
        }

        #[cfg(target_os = "macos")]
        {
            self.get_macos_usage(pid).await
        }

        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        {
            ResourceUsage::default()
        }
    }

    #[cfg(target_os = "windows")]
    async fn get_windows_usage(&self, pid: u32) -> ResourceUsage {
        // TODO: Implement Windows-specific resource monitoring
        // Could use WMI or Windows API calls
        ResourceUsage::default()
    }

    #[cfg(target_os = "linux")]
    async fn get_linux_usage(&self, pid: u32) -> ResourceUsage {
        // TODO: Implement Linux-specific resource monitoring
        // Could read from /proc/{pid}/stat and /proc/{pid}/status
        ResourceUsage::default()
    }

    #[cfg(target_os = "macos")]
    async fn get_macos_usage(&self, pid: u32) -> ResourceUsage {
        // TODO: Implement macOS-specific resource monitoring
        // Could use libproc or system calls
        ResourceUsage::default()
    }
}

impl Default for ProcessTracker {
    fn default() -> Self {
        Self::new()
    }
}
