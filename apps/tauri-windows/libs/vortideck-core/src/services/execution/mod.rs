// Universal Execution Engine for VortiDeck
// Handles all types of actions: buttons, macros, flows, scripts, integrations

use crate::core::state_manager::StateManager;
use crate::features::{FeatureRequirement, PremiumFeature, ResetPeriod};
use crate::integrations::UniversalIntegrationManager;
use crate::services::{FlowState, IconService, MacroEngine};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tokio::sync::{oneshot, Mutex};

// Import process tracking
pub mod process_tracker;
pub use process_tracker::{
    ProcessError, ProcessResult, ProcessStatus, ProcessTracker, ResourceUsage, TrackedProcess,
};

// Import action converter
pub mod action_converter;
pub use action_converter::{
    convert_action_reference_to_universal_action, convert_action_to_universal_action,
    ActionReference, ActionReferenceKind,
};

pub mod input_contract;
pub mod invocation;
pub use invocation::{
    ActionInvocationService, ExternalInvocationCompletion, ExternalInvocationStart,
    InvocationResult, InvocationStatus,
};

pub mod readiness;
pub use readiness::{ActionRuntimeReadiness, ActionRuntimeReadinessSnapshot, ActionRuntimeStatus};

// Import keyboard simulation
pub mod keyboard;
pub use keyboard::{simulate_key_down, simulate_key_up};

// ============================================================================
// PROGRAM TYPE ENUM
// ============================================================================

/// Type-safe program type enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProgramType {
    /// Shell commands (curl, npm, git, etc.) - parsed with shlex, no shell
    Command,
    /// Direct executable files (.exe, .app, etc.)
    Executable,
    /// Script files (.bat, .cmd, .ps1, .sh) - require shell execution
    Script,
}

impl ProgramType {
    /// Check if this program type needs shell execution
    pub fn needs_shell(&self) -> bool {
        match self {
            ProgramType::Command => true, // Commands need cmd /C on Windows (e.g. "uv run script.py")
            ProgramType::Script => true,  // Scripts need shell
            ProgramType::Executable => false, // Direct execution
        }
    }

    /// Check if file extension indicates need for shell execution
    pub fn needs_shell_by_extension(executable: &str) -> bool {
        let trimmed = executable.trim().to_lowercase();
        trimmed.ends_with(".bat")
            || trimmed.ends_with(".cmd")
            || trimmed.ends_with(".ps1")
            || trimmed.ends_with(".sh")
    }
}

impl std::fmt::Display for ProgramType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProgramType::Command => write!(f, "command"),
            ProgramType::Executable => write!(f, "executable"),
            ProgramType::Script => write!(f, "script"),
        }
    }
}

impl std::str::FromStr for ProgramType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "command" => Ok(ProgramType::Command),
            "executable" => Ok(ProgramType::Executable),
            "script" => Ok(ProgramType::Script),
            _ => Err(format!(
                "Invalid program type: {}. Valid values: command, executable, script",
                s
            )),
        }
    }
}

// Helper function to determine if a program needs shell execution
// DEPRECATED: Use ProgramType::needs_shell() instead
fn needs_shell_execution(program_type: &str, executable: &str) -> bool {
    // Temporary compatibility layer during migration
    match program_type {
        "command" => true, // Commands need cmd /C on Windows
        "script" => true,
        "executable" => ProgramType::needs_shell_by_extension(executable),
        _ => false,
    }
}

// TODO: Implement these modules as part of the universal execution engine
// pub mod engine;
// pub mod actions;
// pub mod context;
// pub mod validators;

// ============================================================================
// UNIVERSAL ACTION SYSTEM
// ============================================================================

/// Universal action that can be executed by any trigger (button, macro, flow, websocket, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UniversalAction {
    // Basic Actions
    SystemControl {
        command: String,
        parameters: Option<HashMap<String, Value>>,
    },
    // Script {
    //     language: ScriptLanguage,
    //     code: String,
    //     parameters: Option<HashMap<String, Value>>,
    // },
    Program {
        executable: String,
        args: Vec<String>,
        working_dir: Option<String>,
        env_vars: Option<HashMap<String, String>>,
        timeout_ms: Option<u64>,
        program_type: ProgramType, // ✅ Now type-safe!
        show_terminal: bool,
    },
    Integration {
        integration_id: String,
        action_id: String,
        parameters: HashMap<String, Value>,
    },
    HttpRequest {
        name: String,
        method: String,
        url: String,
        headers: HashMap<String, String>,
        body: Option<Value>,
        timeout_ms: u64,
        #[serde(default)]
        accepted_status_codes: Option<Vec<u16>>,
    },
    FocusWindow {
        name: String,
        window_title: String,
        process_name: String,
        match_mode: String,
    },

    // Input Simulation
    KeySequence {
        keys: Vec<KeyAction>,
        timing: TimingConfig,
    },
    MouseAction {
        action_type: MouseActionType,
        coordinates: Option<(i32, i32)>,
        button: Option<MouseButton>,
    },

    // Composite Actions
    Macro {
        macro_id: String,
        parameters: Option<HashMap<String, Value>>,
    },
    Flow {
        flow_id: String,
        parameters: Option<HashMap<String, Value>>,
    },
    Sequence {
        actions: Vec<UniversalAction>,
        execution_mode: SequenceMode,
    },

    // Conditional Actions
    Conditional {
        condition: ExecutionCondition,
        if_true: Box<UniversalAction>,
        if_false: Option<Box<UniversalAction>>,
    },

    // Application Control
    AppControl {
        app_identifier: String,
        action: AppAction,
        parameters: Option<HashMap<String, Value>>,
    },

    // Custom Plugin Actions
    Plugin {
        plugin_id: String,
        action_id: String,
        parameters: HashMap<String, Value>,
    },
}

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub enum ScriptLanguage {
//     JavaScript,
//     Python,
//     PowerShell,
//     Bash,
//     Lua,
// }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyAction {
    Press(String),
    Release(String),
    Type(String),
    Combination(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingConfig {
    pub delay_before_ms: u64,
    pub delay_after_ms: u64,
    pub key_press_duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MouseActionType {
    Click,
    DoubleClick,
    RightClick,
    Move,
    Drag,
    Scroll,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    X1,
    X2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SequenceMode {
    Sequential,       // Execute one after another
    Parallel,         // Execute all at once
    ConditionalChain, // Execute next only if previous succeeds
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionCondition {
    Always,
    Never,
    SystemState(String, Value), // Check system state
    UserInput(String),          // Prompt user
    TimeRange(String, String),  // Execute only in time range
    Custom(String),             // Custom condition script
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppAction {
    Launch,
    Close,
    Focus,
    Minimize,
    Maximize,
    SendKeys(String),
    Custom(String),
}

// ============================================================================
// EXECUTION CONTEXT
// ============================================================================

/// Button execution modes (how buttons should execute when pressed)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ButtonExecutionMode {
    /// Execute once per button press
    RunOnce,
    /// Execute repeatedly while button is held (not yet implemented)
    RepeatWhilePressing,
    /// Toggle on/off state (not yet implemented)
    Switch,
    /// Queue execution per button and await this invocation's terminal result
    Queue,
}

impl Default for ButtonExecutionMode {
    fn default() -> Self {
        ButtonExecutionMode::RunOnce
    }
}

impl std::str::FromStr for ButtonExecutionMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "run_once" => Ok(ButtonExecutionMode::RunOnce),
            "repeat_while_pressing" => Ok(ButtonExecutionMode::RepeatWhilePressing),
            "switch" => Ok(ButtonExecutionMode::Switch),
            "queue" => Ok(ButtonExecutionMode::Queue),
            _ => Err(format!("Unknown execution mode: {}", s)),
        }
    }
}

/// Context passed to all action executions
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub trigger_source: TriggerSource,
    pub user_id: Option<String>,
    pub session_id: String,
    pub variables: HashMap<String, Value>,
    pub timeout_ms: Option<u64>,
    pub retry_config: RetryConfig,
    pub execution_mode: ButtonExecutionMode,
    pub execution_config: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TriggerSource {
    Button { button_id: String, deck_id: String },
    Macro { macro_id: String },
    Flow { flow_id: String, node_id: String },
    WebSocket { client_id: String },
    Schedule { schedule_id: String },
    Plugin { plugin_id: String },
    Manual { user_id: String },
    ContextMenu { item_id: String },
}

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub retry_delay_ms: u64,
    pub exponential_backoff: bool,
}

// ============================================================================
// EXECUTION RESULTS
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub success: bool,
    pub outputs: HashMap<String, Value>,
    pub error: Option<ExecutionError>,
    pub execution_time_ms: u64,
    pub retry_count: u32,
    pub logs: Vec<ExecutionLogEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionError {
    pub error_type: ErrorType,
    pub message: String,
    pub details: Option<Value>,
    pub recoverable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorType {
    PermissionDenied,
    Timeout,
    InvalidParameters,
    SystemError,
    IntegrationError,
    ScriptError,
    NetworkError,
    ConcurrencyLimitReached,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionLogEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub level: LogLevel,
    pub message: String,
    pub details: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

// ============================================================================
// EXECUTION ENGINE TRAIT
// ============================================================================

#[async_trait]
pub trait ExecutionEngine: Send + Sync {
    /// Execute a single universal action
    async fn execute_action(
        &self,
        action: &UniversalAction,
        context: &ExecutionContext,
    ) -> ExecutionResult;

    /// Execute multiple actions with specified mode
    async fn execute_sequence(
        &self,
        actions: &[UniversalAction],
        mode: SequenceMode,
        context: &ExecutionContext,
    ) -> Vec<ExecutionResult>;

    /// Validate action before execution
    async fn validate_action(
        &self,
        action: &UniversalAction,
        context: &ExecutionContext,
    ) -> Result<(), ExecutionError>;

    /// Get available actions for a context
    async fn get_available_actions(&self, context: &ExecutionContext) -> Vec<ActionDescriptor>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionDescriptor {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub parameters: Vec<ParameterDescriptor>,
    pub permissions_required: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterDescriptor {
    pub name: String,
    pub parameter_type: String,
    pub required: bool,
    pub default_value: Option<Value>,
    pub description: String,
}

/// RAII guard for tracking active permits
/// Automatically decrements the active permit counter when dropped
struct ActivePermitGuard {
    active_permits: Arc<AtomicUsize>,
    _permit: Option<tokio::sync::OwnedSemaphorePermit>,
}

impl ActivePermitGuard {
    fn new(
        active_permits: Arc<AtomicUsize>,
        permit: Option<tokio::sync::OwnedSemaphorePermit>,
    ) -> Self {
        Self {
            active_permits,
            _permit: permit,
        }
    }
}

impl Drop for ActivePermitGuard {
    fn drop(&mut self) {
        if self._permit.is_some() {
            self.active_permits.fetch_sub(1, Ordering::AcqRel);
        }
    }
}

struct QueuedExecution {
    action: UniversalAction,
    context: ExecutionContext,
    admission: oneshot::Receiver<Result<(), ExecutionResult>>,
    completion: oneshot::Sender<ExecutionResult>,
}

#[derive(Default)]
struct ExecutionQueueState {
    worker_running: bool,
    pending: VecDeque<QueuedExecution>,
}

/// Default implementation of the Universal Execution Engine
#[derive(Clone)]
pub struct UniversalExecutionEngine {
    state_manager: Arc<StateManager>,
    process_tracker: Arc<ProcessTracker>,
    /// Semaphore for concurrency control (None = unlimited)
    /// Wrapped in RwLock to allow runtime updates
    semaphore: Arc<tokio::sync::RwLock<Option<Arc<tokio::sync::Semaphore>>>>,
    max_concurrent: Arc<tokio::sync::RwLock<usize>>,
    /// Active permit counter - tracks how many actions are currently running
    /// Used to gracefully wait for in-flight actions before updating semaphore
    active_permits: Arc<AtomicUsize>,
    /// Switch state tracking (button_id -> is_active)
    switch_states: Arc<Mutex<HashMap<String, bool>>>,
    /// Execution queue tracking (button_id -> queue of pending actions)
    execution_queues: Arc<Mutex<HashMap<String, ExecutionQueueState>>>,
    /// Database access for loading macro/flow data
    database: Option<Arc<crate::db::Database>>,
    /// Flow executor for executing flows
    flow_executor: Option<Arc<Mutex<Option<crate::services::flow::executor::FlowExecutor>>>>,
    /// Enigo instance for keyboard/mouse simulation
    enigo: Option<Arc<Mutex<enigo::Enigo>>>,
    /// Existing package permission authority. It is injected by the desktop
    /// composition root; native connectors never maintain a second grant set.
    permission_checker: Option<Arc<crate::plugins::permissions::PermissionChecker>>,
    /// Connector-owned action contracts and dispatch providers.
    integration_manager: Option<Arc<UniversalIntegrationManager>>,
}

impl UniversalExecutionEngine {
    pub fn new(state_manager: Arc<StateManager>) -> Self {
        Self {
            state_manager,
            process_tracker: Arc::new(ProcessTracker::new()),
            semaphore: Arc::new(tokio::sync::RwLock::new(None)), // No limit by default
            max_concurrent: Arc::new(tokio::sync::RwLock::new(0)),
            active_permits: Arc::new(AtomicUsize::new(0)),
            switch_states: Arc::new(Mutex::new(HashMap::new())),
            execution_queues: Arc::new(Mutex::new(HashMap::new())),
            database: None,
            flow_executor: None,
            enigo: None,
            permission_checker: None,
            integration_manager: None,
        }
    }

    /// Create new engine with concurrency limit
    pub fn with_concurrency_limit(state_manager: Arc<StateManager>, max_concurrent: usize) -> Self {
        let semaphore = if max_concurrent > 0 {
            Some(Arc::new(tokio::sync::Semaphore::new(max_concurrent)))
        } else {
            None // 0 = unlimited
        };

        Self {
            state_manager,
            process_tracker: Arc::new(ProcessTracker::new()),
            semaphore: Arc::new(tokio::sync::RwLock::new(semaphore)),
            max_concurrent: Arc::new(tokio::sync::RwLock::new(max_concurrent)),
            active_permits: Arc::new(AtomicUsize::new(0)),
            switch_states: Arc::new(Mutex::new(HashMap::new())),
            execution_queues: Arc::new(Mutex::new(HashMap::new())),
            database: None,
            flow_executor: None,
            enigo: None,
            permission_checker: None,
            integration_manager: None,
        }
    }

    /// Update concurrency limit at runtime (gracefully waits for running actions)
    /// This allows changing max_concurrent_buttons without restarting the app
    ///
    /// IMPORTANT: This method waits for all in-flight actions to complete before
    /// updating the semaphore to avoid orphaning permits and double-counting.
    pub async fn update_concurrency_limit(&self, max_concurrent: usize) {
        let active_count = self.active_permits.load(Ordering::Acquire);

        if active_count > 0 {
            log::info!(
                "⏳ Waiting for {} in-flight actions to complete before updating concurrency limit...",
                active_count
            );

            // Poll until all active permits are released (with timeout)
            let max_wait = tokio::time::Duration::from_secs(30);
            let start = tokio::time::Instant::now();

            while self.active_permits.load(Ordering::Acquire) > 0 {
                if start.elapsed() > max_wait {
                    log::warn!(
                        "⚠️ Timeout waiting for actions to complete. Forcing semaphore update."
                    );
                    break;
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }

            log::info!("✅ All in-flight actions completed. Updating concurrency limit now.");
        }

        // Update the limit and recreate semaphore
        *self.max_concurrent.write().await = max_concurrent;
        let new_semaphore = if max_concurrent > 0 {
            Some(Arc::new(tokio::sync::Semaphore::new(max_concurrent)))
        } else {
            None // 0 = unlimited
        };
        *self.semaphore.write().await = new_semaphore;

        log::info!(
            "🔄 Updated ExecutionEngine concurrency limit to: {}",
            if max_concurrent > 0 {
                max_concurrent.to_string()
            } else {
                "UNLIMITED".to_string()
            }
        );
    }

    /// Set database for macro/flow loading
    pub fn with_database(mut self, database: Arc<crate::db::Database>) -> Self {
        self.database = Some(database);
        self
    }

    /// Set flow executor for flow execution
    pub fn with_flow_executor(
        mut self,
        flow_executor: Arc<Mutex<Option<crate::services::flow::executor::FlowExecutor>>>,
    ) -> Self {
        self.flow_executor = Some(flow_executor);
        self
    }

    /// Set enigo for keyboard/mouse simulation
    pub fn with_enigo(mut self, enigo: Arc<Mutex<enigo::Enigo>>) -> Self {
        self.enigo = Some(enigo);
        self
    }

    pub fn with_permission_checker(
        mut self,
        permission_checker: Arc<crate::plugins::permissions::PermissionChecker>,
    ) -> Self {
        self.permission_checker = Some(permission_checker);
        self
    }

    pub fn with_integration_manager(
        mut self,
        integration_manager: Arc<UniversalIntegrationManager>,
    ) -> Self {
        self.integration_manager = Some(integration_manager);
        self
    }

    /// Get the process tracker for external access
    pub fn get_process_tracker(&self) -> Arc<ProcessTracker> {
        self.process_tracker.clone()
    }

    pub async fn resolve_integration_capability(
        &self,
        action: &UniversalAction,
    ) -> Result<Option<crate::integrations::CapabilityDefinition>, ExecutionError> {
        let UniversalAction::Integration {
            integration_id,
            action_id,
            parameters,
        } = action
        else {
            return Ok(None);
        };
        let manager = self
            .integration_manager
            .as_ref()
            .ok_or_else(|| ExecutionError {
                error_type: ErrorType::IntegrationError,
                message: "Integration capability registry is unavailable".to_string(),
                details: None,
                recoverable: false,
            })?;
        manager
            .resolve_capability(integration_id, action_id, parameters)
            .await
            .map(Some)
            .map_err(|error| ExecutionError {
                error_type: ErrorType::InvalidParameters,
                message: error.to_string(),
                details: None,
                recoverable: false,
            })
    }

    fn is_composite_action(action: &UniversalAction) -> bool {
        matches!(
            action,
            UniversalAction::Macro { .. }
                | UniversalAction::Flow { .. }
                | UniversalAction::Sequence { .. }
                | UniversalAction::Conditional { .. }
        )
    }

    async fn acquire_leaf_permit(&self) -> Result<ActivePermitGuard, Box<ExecutionResult>> {
        let semaphore_guard = self.semaphore.read().await;
        let permit = if let Some(semaphore) = semaphore_guard.as_ref() {
            let max_concurrent = *self.max_concurrent.read().await;
            match semaphore.clone().acquire_owned().await {
                Ok(permit) => {
                    self.active_permits.fetch_add(1, Ordering::AcqRel);
                    log::debug!(
                        "Acquired leaf execution permit ({}/{} used)",
                        max_concurrent - semaphore.available_permits(),
                        max_concurrent
                    );
                    Some(permit)
                }
                Err(error) => {
                    return Err(Box::new(ExecutionResult {
                        success: false,
                        outputs: HashMap::new(),
                        error: Some(ExecutionError {
                            error_type: ErrorType::ConcurrencyLimitReached,
                            message: "Failed to acquire execution permit".to_string(),
                            details: Some(serde_json::json!({ "error": error.to_string() })),
                            recoverable: true,
                        }),
                        execution_time_ms: 0,
                        retry_count: 0,
                        logs: vec![],
                    }));
                }
            }
        } else {
            None
        };
        drop(semaphore_guard);
        Ok(ActivePermitGuard::new(self.active_permits.clone(), permit))
    }

    fn queue_error(
        error_type: ErrorType,
        message: impl Into<String>,
        details: Option<Value>,
        recoverable: bool,
    ) -> ExecutionResult {
        ExecutionResult {
            success: false,
            outputs: HashMap::new(),
            error: Some(ExecutionError {
                error_type,
                message: message.into(),
                details,
                recoverable,
            }),
            execution_time_ms: 0,
            retry_count: 0,
            logs: vec![],
        }
    }

    fn invocation_id(context: &ExecutionContext) -> Option<&str> {
        context
            .execution_config
            .as_ref()
            .and_then(|config| config.get("invocationId"))
            .and_then(Value::as_str)
    }

    async fn enqueue_and_await(
        &self,
        button_id: String,
        action: UniversalAction,
        mut context: ExecutionContext,
        max_depth: usize,
    ) -> ExecutionResult {
        let invocation_id = Self::invocation_id(&context).map(ToOwned::to_owned);
        context.execution_mode = ButtonExecutionMode::RunOnce;

        let (admission_tx, admission_rx) = oneshot::channel();
        let (completion_tx, completion_rx) = oneshot::channel();
        let (queue_position, start_worker) = {
            let mut queues = self.execution_queues.lock().await;
            let queue = queues.entry(button_id.clone()).or_default();
            let current_depth = queue.pending.len() + usize::from(queue.worker_running);
            if current_depth >= max_depth {
                return Self::queue_error(
                    ErrorType::ConcurrencyLimitReached,
                    format!(
                        "Queue depth limit ({}) reached for {}",
                        max_depth, button_id
                    ),
                    Some(serde_json::json!({
                        "maxDepth": max_depth,
                        "currentDepth": current_depth,
                    })),
                    true,
                );
            }

            let start_worker = !queue.worker_running;
            queue.worker_running = true;
            queue.pending.push_back(QueuedExecution {
                action,
                context,
                admission: admission_rx,
                completion: completion_tx,
            });
            (queue.pending.len(), start_worker)
        };

        // Start the owner before the first fallible/awaiting admission step.
        // If this caller is cancelled, dropping `admission_tx` makes the worker
        // reject this item and continue draining instead of orphaning the queue.
        if start_worker {
            let engine = self.clone();
            let worker_button_id = button_id.clone();
            tokio::spawn(async move {
                engine.drain_execution_queue(worker_button_id).await;
            });
        }

        let admission = if let (Some(database), Some(invocation_id)) =
            (self.database.as_ref(), invocation_id.as_deref())
        {
            database
                .update_action_invocation_dispatch_status(invocation_id, "queued")
                .await
                .map_err(|error| {
                    Self::queue_error(
                        ErrorType::SystemError,
                        format!("Queued action was not dispatched: {}", error),
                        Some(serde_json::json!({
                            "invocationId": invocation_id,
                            "queuePersistenceFailure": true,
                        })),
                        false,
                    )
                })
        } else {
            Ok(())
        };
        let _ = admission_tx.send(admission);

        log::info!(
            "Queue mode: awaiting terminal result for position {}",
            queue_position
        );
        completion_rx.await.unwrap_or_else(|_| {
            Self::queue_error(
                ErrorType::SystemError,
                "Queued action worker stopped before reporting a terminal result",
                Some(serde_json::json!({ "queueWorkerLost": true })),
                false,
            )
        })
    }

    async fn drain_execution_queue(self, button_id: String) {
        loop {
            let next = {
                let mut queues = self.execution_queues.lock().await;
                let Some(queue) = queues.get_mut(&button_id) else {
                    return;
                };
                match queue.pending.pop_front() {
                    Some(item) => Some(item),
                    None => {
                        queues.remove(&button_id);
                        None
                    }
                }
            };
            let Some(item) = next else {
                log::info!("Queue mode: queue empty for button {}", button_id);
                return;
            };

            let result = match item.admission.await {
                Ok(Ok(())) => self.execute_action(&item.action, &item.context).await,
                Ok(Err(error)) => error,
                Err(_) => Self::queue_error(
                    ErrorType::SystemError,
                    "Queued action admission was abandoned before dispatch",
                    Some(serde_json::json!({ "queueAdmissionLost": true })),
                    false,
                ),
            };
            let _ = item.completion.send(result);
        }
    }
}

impl Default for UniversalExecutionEngine {
    fn default() -> Self {
        Self {
            state_manager: Arc::new(StateManager::new()),
            process_tracker: Arc::new(ProcessTracker::new()),
            semaphore: Arc::new(tokio::sync::RwLock::new(None)),
            max_concurrent: Arc::new(tokio::sync::RwLock::new(0)),
            active_permits: Arc::new(AtomicUsize::new(0)),
            switch_states: Arc::new(Mutex::new(HashMap::new())),
            execution_queues: Arc::new(Mutex::new(HashMap::new())),
            database: None,
            flow_executor: None,
            enigo: None,
            permission_checker: None,
            integration_manager: None,
        }
    }
}

#[async_trait]
impl ExecutionEngine for UniversalExecutionEngine {
    async fn execute_action(
        &self,
        action: &UniversalAction,
        context: &ExecutionContext,
    ) -> ExecutionResult {
        let start_time = std::time::Instant::now();

        // 🎯 EXECUTION MODE: Check execution mode first
        match context.execution_mode {
            ButtonExecutionMode::RunOnce => {
                // Normal execution - continue below
            }
            ButtonExecutionMode::RepeatWhilePressing => {
                // RepeatWhilePressing mode: Execute action once, caller should handle repeat logic
                // This mode requires the caller to detect button press/release events
                // and call execute_action multiple times while the button is held
                //
                // Implementation note:
                // - Mobile/Desktop apps should send repeated commands while button is held
                // - The executor executes each request once
                // - No infinite loops in executor to prevent blocking
                log::info!(
                    "🔁 RepeatWhilePressing mode: Executing action once (caller handles repeat)"
                );
                // Continue to normal execution
            }
            ButtonExecutionMode::Switch => {
                // Switch mode: Toggle on/off state
                // Get button_id from trigger source
                let button_id = match &context.trigger_source {
                    TriggerSource::Button { button_id, .. } => button_id.clone(),
                    _ => {
                        return ExecutionResult {
                            success: false,
                            outputs: HashMap::new(),
                            error: Some(ExecutionError {
                                error_type: ErrorType::SystemError,
                                message: "Switch mode only works with button triggers".to_string(),
                                details: None,
                                recoverable: false,
                            }),
                            execution_time_ms: start_time.elapsed().as_millis() as u64,
                            retry_count: 0,
                            logs: vec![],
                        };
                    }
                };

                // Get current state and toggle it
                let mut switch_states = self.switch_states.lock().await;
                let current_state = switch_states.get(&button_id).copied().unwrap_or(false);
                let new_state = !current_state;
                switch_states.insert(button_id.clone(), new_state);
                drop(switch_states); // Release lock before execution

                // Check execution_config for switch-specific actions
                if let Some(config) = &context.execution_config {
                    if let Some(on_action) = config.get("on_action") {
                        if let Some(off_action) = config.get("off_action") {
                            // If we have separate on/off actions, choose based on new state
                            log::info!(
                                "🔄 Switch mode: {} -> {} (using separate actions)",
                                current_state,
                                new_state
                            );

                            // For now, just execute the main action
                            // In a full implementation, you'd parse on_action/off_action as UniversalAction
                            // and execute the appropriate one based on new_state
                            // Continue to normal execution below
                        }
                    }
                }

                if !new_state {
                    // Switched OFF - don't execute action
                    log::info!(
                        "🔄 Switch mode: Button {} switched OFF - skipping execution",
                        button_id
                    );
                    return ExecutionResult {
                        success: true,
                        outputs: [
                            ("switched".to_string(), serde_json::json!(true)),
                            ("state".to_string(), serde_json::json!(false)),
                        ]
                        .iter()
                        .cloned()
                        .collect(),
                        error: None,
                        execution_time_ms: start_time.elapsed().as_millis() as u64,
                        retry_count: 0,
                        logs: vec![ExecutionLogEntry {
                            timestamp: chrono::Utc::now(),
                            level: LogLevel::Info,
                            message: format!("Button {} switched OFF", button_id),
                            details: None,
                        }],
                    };
                }

                log::info!(
                    "🔄 Switch mode: Button {} switched ON - executing action",
                    button_id
                );
                // Continue to normal execution (state is ON)
            }
            ButtonExecutionMode::Queue => {
                let button_id = match &context.trigger_source {
                    TriggerSource::Button { button_id, .. } => button_id.clone(),
                    _ => {
                        return ExecutionResult {
                            success: false,
                            outputs: HashMap::new(),
                            error: Some(ExecutionError {
                                error_type: ErrorType::SystemError,
                                message: "Queue mode only works with button triggers".to_string(),
                                details: None,
                                recoverable: false,
                            }),
                            execution_time_ms: start_time.elapsed().as_millis() as u64,
                            retry_count: 0,
                            logs: vec![],
                        };
                    }
                };
                let max_depth = context
                    .execution_config
                    .as_ref()
                    .and_then(|config| config.get("maxDepth"))
                    .and_then(Value::as_u64)
                    .unwrap_or(100)
                    .clamp(1, 10_000) as usize;
                return self
                    .enqueue_and_await(button_id, action.clone(), context.clone(), max_depth)
                    .await;
            }
        }

        // Composite containers do not retain capacity while awaiting children.
        // Each actual leaf action participates in the global bound.
        let _guard = if Self::is_composite_action(action) {
            None
        } else {
            match self.acquire_leaf_permit().await {
                Ok(guard) => Some(guard),
                Err(mut error) => {
                    error.execution_time_ms = start_time.elapsed().as_millis() as u64;
                    return *error;
                }
            }
        };

        // Validate action first
        if let Err(error) = self.validate_action(action, context).await {
            return ExecutionResult {
                success: false,
                outputs: HashMap::new(),
                error: Some(error),
                execution_time_ms: start_time.elapsed().as_millis() as u64,
                retry_count: 0,
                logs: vec![ExecutionLogEntry {
                    timestamp: chrono::Utc::now(),
                    level: LogLevel::Error,
                    message: "Action validation failed".to_string(),
                    details: None,
                }],
            };
        }

        let result = match action {
            UniversalAction::Integration {
                integration_id,
                action_id,
                parameters,
            } => {
                let integration_action = crate::integrations::interface::IntegrationAction {
                    name: action_id.clone(),
                    parameters: parameters.clone(),
                    execution: Some(crate::integrations::interface::IntegrationActionExecution {
                        request_id: context
                            .execution_config
                            .as_ref()
                            .and_then(|value| value.get("requestId"))
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned)
                            .or_else(|| Some(uuid::Uuid::new_v4().to_string())),
                        idempotency_key: context
                            .execution_config
                            .as_ref()
                            .and_then(|value| value.get("idempotencyKey"))
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned),
                    }),
                };
                let manager = self.integration_manager.as_ref().expect(
                    "integration manager presence is validated before integration dispatch",
                );

                match manager
                    .execute_integration_action(integration_id, integration_action)
                    .await
                {
                    Ok(provider_result) => {
                        let mut outputs = provider_result.outputs;
                        outputs.insert(
                            "invocation".to_string(),
                            serde_json::json!({
                                "integrationId": integration_id,
                                "actionId": action_id,
                                "status": provider_result.status,
                                "requestId": provider_result.request_id,
                                "providerReceiptId": provider_result.provider_receipt_id,
                            }),
                        );
                        ExecutionResult {
                        success: true,
                        outputs,
                        error: None,
                        execution_time_ms: start_time.elapsed().as_millis() as u64,
                        retry_count: 0,
                        logs: vec![ExecutionLogEntry {
                            timestamp: chrono::Utc::now(),
                            level: LogLevel::Info,
                            message: format!(
                                "Integration action '{}:{}' executed successfully",
                                integration_id, action_id
                            ),
                            details: Some(serde_json::json!({ "parameters": parameters })),
                        }],
                    }
                    },
                    Err(error) => ExecutionResult {
                        success: false,
                        outputs: HashMap::new(),
                        error: Some(ExecutionError {
                            error_type: ErrorType::IntegrationError,
                            message: format!(
                                "Integration action '{}:{}' failed: {}",
                                integration_id, action_id, error
                            ),
                            details: Some(serde_json::json!({ "parameters": parameters })),
                            recoverable: matches!(
                                error,
                                crate::integrations::traits::IntegrationError::NotConnected(_)
                                    | crate::integrations::traits::IntegrationError::ConnectionError(_)
                            ),
                        }),
                        execution_time_ms: start_time.elapsed().as_millis() as u64,
                        retry_count: 0,
                        logs: vec![ExecutionLogEntry {
                            timestamp: chrono::Utc::now(),
                            level: LogLevel::Error,
                            message: format!(
                                "Integration action '{}:{}' failed",
                                integration_id, action_id
                            ),
                            details: Some(serde_json::json!({ "error": error.to_string() })),
                        }],
                    },
                }
            }
            UniversalAction::Macro {
                macro_id,
                parameters,
            } => {
                // Execute macro by loading from database and executing each step
                let start_time = std::time::Instant::now();
                let mut logs = vec![];
                let mut had_critical_error = false;
                let mut critical_errors: Vec<String> = vec![];
                let enigo_available = self.enigo.is_some();
                log::info!(
                    "🎬 Macro execution started: macro_id={}, trigger_source={:?}, enigo_available={}",
                    macro_id,
                    context.trigger_source,
                    enigo_available
                );

                // Check if database is available
                let db = match &self.database {
                    Some(db) => db.clone(),
                    None => {
                        return ExecutionResult {
                            success: false,
                            outputs: HashMap::new(),
                            error: Some(ExecutionError {
                                error_type: ErrorType::SystemError,
                                message: "Database not available for macro execution".to_string(),
                                details: None,
                                recoverable: false,
                            }),
                            execution_time_ms: start_time.elapsed().as_millis() as u64,
                            retry_count: 0,
                            logs,
                        };
                    }
                };

                logs.push(ExecutionLogEntry {
                    timestamp: chrono::Utc::now(),
                    level: LogLevel::Info,
                    message: format!("Loading macro {}", macro_id),
                    details: None,
                });

                // Parse macro_id from string to i32
                let macro_id_int: i32 = match macro_id.parse() {
                    Ok(id) => id,
                    Err(_) => {
                        return ExecutionResult {
                            success: false,
                            outputs: HashMap::new(),
                            error: Some(ExecutionError {
                                error_type: ErrorType::InvalidParameters,
                                message: format!("Invalid macro ID: {}", macro_id),
                                details: None,
                                recoverable: false,
                            }),
                            execution_time_ms: start_time.elapsed().as_millis() as u64,
                            retry_count: 0,
                            logs,
                        };
                    }
                };

                // Load macro from database (returns Option<Macro>)
                let macro_data = match db.get_macro_with_chain(macro_id_int).await {
                    Some(data) => data,
                    None => {
                        return ExecutionResult {
                            success: false,
                            outputs: HashMap::new(),
                            error: Some(ExecutionError {
                                error_type: ErrorType::SystemError,
                                message: format!("Macro {} not found", macro_id),
                                details: None,
                                recoverable: false,
                            }),
                            execution_time_ms: start_time.elapsed().as_millis() as u64,
                            retry_count: 0,
                            logs,
                        };
                    }
                };

                // Deserialize macro chain
                #[derive(serde::Deserialize)]
                struct SerializedMacroStep {
                    action: ActionExecutable,
                    delay: u64,
                }

                #[derive(Debug, serde::Deserialize)]
                enum ActionExecutable {
                    KeyDown(String),
                    KeyUp(String),
                    MouseMove { x: i32, y: i32 },
                    MouseClick(String),
                    ObsAction(String),
                    SystemControl(String),
                }

                let macro_chain: Vec<SerializedMacroStep> =
                    match serde_json::from_str(&macro_data.macro_chain) {
                        Ok(chain) => chain,
                        Err(e) => {
                            return ExecutionResult {
                                success: false,
                                outputs: HashMap::new(),
                                error: Some(ExecutionError {
                                    error_type: ErrorType::SystemError,
                                    message: format!("Failed to deserialize macro chain: {}", e),
                                    details: None,
                                    recoverable: false,
                                }),
                                execution_time_ms: start_time.elapsed().as_millis() as u64,
                                retry_count: 0,
                                logs,
                            };
                        }
                    };

                logs.push(ExecutionLogEntry {
                    timestamp: chrono::Utc::now(),
                    level: LogLevel::Info,
                    message: format!("Macro loaded with {} steps", macro_chain.len()),
                    details: None,
                });

                // Execute each step in the macro
                let mut step_count = 0;
                for step in macro_chain {
                    step_count += 1;

                    // Handle the action based on type
                    match step.action {
                        ActionExecutable::KeyDown(key) => {
                            let _step_guard = match self.acquire_leaf_permit().await {
                                Ok(guard) => guard,
                                Err(mut error) => {
                                    error.execution_time_ms =
                                        start_time.elapsed().as_millis() as u64;
                                    return *error;
                                }
                            };
                            if let Some(enigo) = &self.enigo {
                                let mut enigo_guard = enigo.lock().await;
                                match simulate_key_down(&mut enigo_guard, &key) {
                                    Ok(_) => {
                                        logs.push(ExecutionLogEntry {
                                            timestamp: chrono::Utc::now(),
                                            level: LogLevel::Info,
                                            message: format!(
                                                "Step {}: Key down: {}",
                                                step_count, key
                                            ),
                                            details: None,
                                        });
                                    }
                                    Err(e) => {
                                        had_critical_error = true;
                                        critical_errors.push(format!(
                                            "Step {} key down '{}' failed: {}",
                                            step_count, key, e
                                        ));
                                        logs.push(ExecutionLogEntry {
                                            timestamp: chrono::Utc::now(),
                                            level: LogLevel::Error,
                                            message: format!(
                                                "Step {}: Failed to press key: {}",
                                                step_count, e
                                            ),
                                            details: None,
                                        });
                                    }
                                }
                            } else {
                                had_critical_error = true;
                                critical_errors.push(format!(
                                    "Step {} key down '{}' could not run: Enigo not available",
                                    step_count, key
                                ));
                                logs.push(ExecutionLogEntry {
                                    timestamp: chrono::Utc::now(),
                                    level: LogLevel::Warning,
                                    message: format!(
                                        "Step {}: Enigo not available for key down",
                                        step_count
                                    ),
                                    details: Some(serde_json::Value::String(key)),
                                });
                            }
                        }
                        ActionExecutable::KeyUp(key) => {
                            let _step_guard = match self.acquire_leaf_permit().await {
                                Ok(guard) => guard,
                                Err(mut error) => {
                                    error.execution_time_ms =
                                        start_time.elapsed().as_millis() as u64;
                                    return *error;
                                }
                            };
                            if let Some(enigo) = &self.enigo {
                                let mut enigo_guard = enigo.lock().await;
                                match simulate_key_up(&mut enigo_guard, &key) {
                                    Ok(_) => {
                                        logs.push(ExecutionLogEntry {
                                            timestamp: chrono::Utc::now(),
                                            level: LogLevel::Info,
                                            message: format!(
                                                "Step {}: Key up: {}",
                                                step_count, key
                                            ),
                                            details: None,
                                        });
                                    }
                                    Err(e) => {
                                        had_critical_error = true;
                                        critical_errors.push(format!(
                                            "Step {} key up '{}' failed: {}",
                                            step_count, key, e
                                        ));
                                        logs.push(ExecutionLogEntry {
                                            timestamp: chrono::Utc::now(),
                                            level: LogLevel::Error,
                                            message: format!(
                                                "Step {}: Failed to release key: {}",
                                                step_count, e
                                            ),
                                            details: None,
                                        });
                                    }
                                }
                            } else {
                                had_critical_error = true;
                                critical_errors.push(format!(
                                    "Step {} key up '{}' could not run: Enigo not available",
                                    step_count, key
                                ));
                                logs.push(ExecutionLogEntry {
                                    timestamp: chrono::Utc::now(),
                                    level: LogLevel::Warning,
                                    message: format!(
                                        "Step {}: Enigo not available for key up",
                                        step_count
                                    ),
                                    details: Some(serde_json::Value::String(key)),
                                });
                            }
                        }
                        ActionExecutable::MouseMove { .. } | ActionExecutable::MouseClick(_) => {
                            // Mouse actions not yet implemented
                            logs.push(ExecutionLogEntry {
                                timestamp: chrono::Utc::now(),
                                level: LogLevel::Warning,
                                message: format!(
                                    "Step {}: Mouse actions not yet implemented",
                                    step_count
                                ),
                                details: Some(serde_json::Value::String(format!(
                                    "{:?}",
                                    step.action
                                ))),
                            });
                        }
                        ActionExecutable::ObsAction(action_id) => {
                            logs.push(ExecutionLogEntry {
                                timestamp: chrono::Utc::now(),
                                level: LogLevel::Info,
                                message: format!(
                                    "Step {}: Executing OBS action: {}",
                                    step_count, action_id
                                ),
                                details: None,
                            });

                            // Create UniversalAction for OBS
                            let obs_action = UniversalAction::Integration {
                                integration_id: "obs".to_string(),
                                action_id,
                                parameters: HashMap::new(),
                            };

                            // Execute recursively through same engine
                            let result = self.execute_action(&obs_action, context).await;
                            if !result.success {
                                had_critical_error = true;
                                critical_errors.push(format!(
                                    "Step {} OBS action failed: {}",
                                    step_count,
                                    result
                                        .error
                                        .as_ref()
                                        .map(|e| e.message.clone())
                                        .unwrap_or_else(|| "Unknown OBS error".to_string())
                                ));
                                logs.push(ExecutionLogEntry {
                                    timestamp: chrono::Utc::now(),
                                    level: LogLevel::Error,
                                    message: format!("Step {}: OBS action failed", step_count),
                                    details: result
                                        .error
                                        .map(|e| serde_json::Value::String(e.message)),
                                });
                            }
                        }
                        ActionExecutable::SystemControl(command) => {
                            logs.push(ExecutionLogEntry {
                                timestamp: chrono::Utc::now(),
                                level: LogLevel::Info,
                                message: format!(
                                    "Step {}: Executing system control: {}",
                                    step_count, command
                                ),
                                details: None,
                            });

                            // Create UniversalAction for SystemControl
                            let system_action = UniversalAction::SystemControl {
                                command,
                                parameters: parameters.clone(),
                            };

                            // Execute recursively through same engine
                            let result = self.execute_action(&system_action, context).await;
                            if !result.success {
                                had_critical_error = true;
                                critical_errors.push(format!(
                                    "Step {} system control failed: {}",
                                    step_count,
                                    result
                                        .error
                                        .as_ref()
                                        .map(|e| e.message.clone())
                                        .unwrap_or_else(
                                            || "Unknown system control error".to_string()
                                        )
                                ));
                                logs.push(ExecutionLogEntry {
                                    timestamp: chrono::Utc::now(),
                                    level: LogLevel::Error,
                                    message: format!("Step {}: System control failed", step_count),
                                    details: result
                                        .error
                                        .map(|e| serde_json::Value::String(e.message)),
                                });
                            }
                        }
                    }

                    // Handle delay between steps
                    if step.delay > 0 {
                        tokio::time::sleep(tokio::time::Duration::from_millis(step.delay)).await;
                    }
                }

                logs.push(ExecutionLogEntry {
                    timestamp: chrono::Utc::now(),
                    level: LogLevel::Info,
                    message: format!("Macro {} completed with {} steps", macro_id, step_count),
                    details: None,
                });

                if had_critical_error {
                    return ExecutionResult {
                        success: false,
                        outputs: [
                            (
                                "steps_executed".to_string(),
                                serde_json::Value::Number(step_count.into()),
                            ),
                            (
                                "critical_errors".to_string(),
                                serde_json::json!(critical_errors),
                            ),
                        ]
                        .into(),
                        error: Some(ExecutionError {
                            error_type: ErrorType::SystemError,
                            message: format!(
                                "Macro {} completed with critical execution errors ({} issue(s))",
                                macro_id,
                                critical_errors.len()
                            ),
                            details: Some(serde_json::json!(critical_errors)),
                            recoverable: true,
                        }),
                        execution_time_ms: start_time.elapsed().as_millis() as u64,
                        retry_count: 0,
                        logs,
                    };
                }

                ExecutionResult {
                    success: true,
                    outputs: [(
                        "steps_executed".to_string(),
                        serde_json::Value::Number(step_count.into()),
                    )]
                    .into(),
                    error: None,
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                    retry_count: 0,
                    logs,
                }
            }
            UniversalAction::Flow {
                flow_id,
                parameters,
            } => {
                // Execute flow using FlowExecutor
                let start_time = std::time::Instant::now();
                let mut logs = vec![];

                logs.push(ExecutionLogEntry {
                    timestamp: chrono::Utc::now(),
                    level: LogLevel::Info,
                    message: format!("Starting flow execution: {}", flow_id),
                    details: None,
                });

                // Check if database is available
                let db = match &self.database {
                    Some(db) => db.clone(),
                    None => {
                        return ExecutionResult {
                            success: false,
                            outputs: HashMap::new(),
                            error: Some(ExecutionError {
                                error_type: ErrorType::SystemError,
                                message: "Database not available for flow execution".to_string(),
                                details: None,
                                recoverable: false,
                            }),
                            execution_time_ms: start_time.elapsed().as_millis() as u64,
                            retry_count: 0,
                            logs,
                        };
                    }
                };

                // Check if flow executor is available
                let flow_executor = match &self.flow_executor {
                    Some(executor) => executor.clone(),
                    None => {
                        return ExecutionResult {
                            success: false,
                            outputs: HashMap::new(),
                            error: Some(ExecutionError {
                                error_type: ErrorType::SystemError,
                                message: "Flow executor not available".to_string(),
                                details: None,
                                recoverable: false,
                            }),
                            execution_time_ms: start_time.elapsed().as_millis() as u64,
                            retry_count: 0,
                            logs,
                        };
                    }
                };

                // Load flow from database
                let flow_definition = match db.load_flow(flow_id) {
                    Ok(flow) => {
                        logs.push(ExecutionLogEntry {
                            timestamp: chrono::Utc::now(),
                            level: LogLevel::Info,
                            message: format!("Flow loaded: {}", flow.name),
                            details: None,
                        });
                        flow
                    }
                    Err(e) => {
                        return ExecutionResult {
                            success: false,
                            outputs: HashMap::new(),
                            error: Some(ExecutionError {
                                error_type: ErrorType::SystemError,
                                message: format!("Failed to load flow: {}", e),
                                details: None,
                                recoverable: false,
                            }),
                            execution_time_ms: start_time.elapsed().as_millis() as u64,
                            retry_count: 0,
                            logs,
                        };
                    }
                };

                // Execute the flow
                let executor_guard = flow_executor.lock().await;
                let execution_result = if let Some(executor) = executor_guard.as_ref() {
                    match executor
                        .start_flow_with_inputs(
                            flow_definition,
                            parameters.clone().unwrap_or_default(),
                        )
                        .await
                    {
                        Ok(execution_id) => {
                            logs.push(ExecutionLogEntry {
                                timestamp: chrono::Utc::now(),
                                level: LogLevel::Info,
                                message: format!(
                                    "Flow started successfully with execution ID: {}",
                                    execution_id
                                ),
                                details: None,
                            });

                            ExecutionResult {
                                success: true,
                                outputs: [
                                    (
                                        "execution_id".to_string(),
                                        serde_json::Value::String(execution_id.clone()),
                                    ),
                                    (
                                        "flow_id".to_string(),
                                        serde_json::Value::String(flow_id.clone()),
                                    ),
                                ]
                                .into(),
                                error: None,
                                execution_time_ms: start_time.elapsed().as_millis() as u64,
                                retry_count: 0,
                                logs,
                            }
                        }
                        Err(e) => {
                            logs.push(ExecutionLogEntry {
                                timestamp: chrono::Utc::now(),
                                level: LogLevel::Error,
                                message: format!("Flow execution failed: {}", e),
                                details: None,
                            });

                            ExecutionResult {
                                success: false,
                                outputs: HashMap::new(),
                                error: Some(ExecutionError {
                                    error_type: ErrorType::SystemError,
                                    message: format!("Flow execution failed: {}", e),
                                    details: None,
                                    recoverable: true,
                                }),
                                execution_time_ms: start_time.elapsed().as_millis() as u64,
                                retry_count: 0,
                                logs,
                            }
                        }
                    }
                } else {
                    logs.push(ExecutionLogEntry {
                        timestamp: chrono::Utc::now(),
                        level: LogLevel::Error,
                        message: "Flow executor not initialized".to_string(),
                        details: None,
                    });

                    ExecutionResult {
                        success: false,
                        outputs: HashMap::new(),
                        error: Some(ExecutionError {
                            error_type: ErrorType::SystemError,
                            message: "Flow executor not initialized".to_string(),
                            details: None,
                            recoverable: false,
                        }),
                        execution_time_ms: start_time.elapsed().as_millis() as u64,
                        retry_count: 0,
                        logs,
                    }
                };

                execution_result
            }
            UniversalAction::Program {
                executable,
                args,
                working_dir,
                env_vars,
                timeout_ms,
                program_type,
                show_terminal,
            } => {
                // Substitute {variable} placeholders from execution context (e.g. {context_menu_path}).
                let substitute = |s: &str| -> String {
                    let mut result = s.to_owned();
                    for (key, val) in &context.variables {
                        let placeholder = format!("{{{}}}", key);
                        let replacement = val
                            .as_str()
                            .map(|v| v.to_owned())
                            .unwrap_or_else(|| val.to_string());
                        result = result.replace(&placeholder, &replacement);
                    }
                    result
                };
                let executable = substitute(executable);
                let args: Vec<String> = args.iter().map(|a| substitute(a)).collect();
                let working_dir: Option<String> = working_dir.as_deref().map(|w| substitute(w));

                println!("🚀 Executing program: {} with args: {:?}", executable, args);
                println!(
                    "🔍 Program type: {}, show terminal: {}",
                    program_type, show_terminal
                );

                // Use stored program type to determine execution strategy
                let use_shell = program_type.needs_shell()
                    || ProgramType::needs_shell_by_extension(&executable);
                println!(
                    "🔍 Use shell execution: {} (type: {:?})",
                    use_shell, program_type
                );

                // Create command based on program type
                let mut cmd = if use_shell {
                    // For command-type programs, parse the command string instead of using shell
                    // This avoids shell injection and properly handles quotes
                    if *program_type == ProgramType::Command {
                        // Command type: pass each part as a separate arg to cmd /C.
                        // This lets Rust quote individual args properly (e.g. paths
                        // with spaces) without double-escaping the full string.

                        println!("🔧 Command mode (cmd /C): {} {:?}", executable, args);

                        #[cfg(target_os = "windows")]
                        {
                            let mut cmd = tokio::process::Command::new("cmd");
                            cmd.arg("/C");
                            // Split executable string into parts (e.g. "uv run script.py" → ["uv","run","script.py"])
                            for part in executable.split_whitespace() {
                                cmd.arg(part);
                            }
                            // Append any extra args (e.g. auto-appended file paths)
                            // Rust will quote args that contain spaces automatically
                            for a in args.iter() {
                                cmd.arg(a);
                            }
                            cmd
                        }
                        #[cfg(not(target_os = "windows"))]
                        {
                            // On Unix, shlex safely handles quoting and splitting
                            let full_command = if args.is_empty() {
                                executable.clone()
                            } else {
                                format!(
                                    "{} {}",
                                    executable,
                                    args.iter()
                                        .map(|a| shlex::try_quote(a)
                                            .unwrap_or_else(|_| a.clone().into())
                                            .to_string())
                                        .collect::<Vec<_>>()
                                        .join(" ")
                                )
                            };
                            let parts = match shlex::split(&full_command) {
                                Some(parts) if !parts.is_empty() => parts,
                                _ => {
                                    println!("❌ Failed to parse command: {}", full_command);
                                    return ExecutionResult {
                                        success: false,
                                        outputs: HashMap::new(),
                                        error: Some(ExecutionError {
                                            error_type: ErrorType::InvalidParameters,
                                            message: format!(
                                                "Invalid command format: {}",
                                                full_command
                                            ),
                                            details: Some(
                                                serde_json::json!({"command": full_command}),
                                            ),
                                            recoverable: false,
                                        }),
                                        execution_time_ms: start_time.elapsed().as_millis() as u64,
                                        retry_count: 0,
                                        logs: vec![],
                                    };
                                }
                            };
                            let mut cmd = tokio::process::Command::new(&parts[0]);
                            if parts.len() > 1 {
                                cmd.args(&parts[1..]);
                            }
                            cmd
                        }
                    } else {
                        // For scripts (.bat, .cmd, .ps1, .sh), they need shell execution
                        #[cfg(target_os = "windows")]
                        {
                            let mut cmd = tokio::process::Command::new("cmd");
                            cmd.args(&["/C", &executable]);
                            if !args.is_empty() {
                                cmd.args(&args);
                            }
                            cmd
                        }
                        #[cfg(not(target_os = "windows"))]
                        {
                            let mut cmd = tokio::process::Command::new("sh");
                            let full_command = if args.is_empty() {
                                executable.clone()
                            } else {
                                format!("{} {}", executable, args.join(" "))
                            };
                            cmd.args(&["-c", &full_command]);
                            cmd
                        }
                    }
                } else {
                    // Direct execution for .exe files
                    let mut cmd = tokio::process::Command::new(&executable);
                    cmd.args(&args);
                    cmd
                };

                // Resolve working directory: use provided dir, or default to user home
                let work_dir = working_dir.clone().unwrap_or_else(|| {
                    std::env::var("USERPROFILE")
                        .or_else(|_| std::env::var("HOME"))
                        .unwrap_or_else(|_| ".".to_string())
                });
                println!("📁 Working directory: {}", work_dir);

                // ── show_terminal=true → fire-and-forget with visible console ──
                #[cfg(target_os = "windows")]
                if *show_terminal {
                    use std::os::windows::process::CommandExt;
                    const CREATE_NO_WINDOW: u32 = 0x08000000;

                    // Build the full command string for cmd /K (keeps window open).
                    // For the terminal path we build a string because `start` needs
                    // the whole command as one piece. We quote args that contain spaces.
                    let full_cmd = {
                        let mut parts: Vec<String> = executable
                            .split_whitespace()
                            .map(|s| s.to_string())
                            .collect();
                        for a in args.iter() {
                            if a.contains(' ') {
                                parts.push(format!("\"{}\"", a));
                            } else {
                                parts.push(a.clone());
                            }
                        }
                        parts.join(" ")
                    };

                    println!("🖥️ Terminal mode (via start): {}", full_cmd);

                    // Use `start` to open an independent terminal window.
                    // Each piece passed as a separate arg so Rust handles quoting
                    // correctly — embedding quotes in a single string breaks cmd.exe.
                    let mut std_cmd = std::process::Command::new("cmd");
                    std_cmd.args(["/C", "start", "", "cmd", "/K", &full_cmd]);
                    std_cmd.current_dir(&work_dir);

                    // Set env vars (inherited by the started terminal)
                    if let Some(vars) = env_vars {
                        for (key, value) in vars.iter() {
                            std_cmd.env(key, value);
                        }
                    }

                    return match std_cmd.spawn() {
                        Ok(_child) => {
                            println!("✅ Terminal launched for: {}", executable);
                            ExecutionResult {
                                success: true,
                                outputs: [
                                    (
                                        "stdout".to_string(),
                                        serde_json::Value::String(
                                            "Launched in terminal".to_string(),
                                        ),
                                    ),
                                    (
                                        "message".to_string(),
                                        serde_json::Value::String(
                                            "Launched in terminal".to_string(),
                                        ),
                                    ),
                                ]
                                .into(),
                                error: None,
                                execution_time_ms: start_time.elapsed().as_millis() as u64,
                                retry_count: 0,
                                logs: vec![ExecutionLogEntry {
                                    timestamp: chrono::Utc::now(),
                                    level: LogLevel::Info,
                                    message: format!(
                                        "Program '{}' launched in terminal",
                                        executable
                                    ),
                                    details: Some(serde_json::json!({"terminal_mode": true})),
                                }],
                            }
                        }
                        Err(e) => {
                            println!("❌ Failed to launch terminal: {:?}", e);
                            ExecutionResult {
                                success: false,
                                outputs: HashMap::new(),
                                error: Some(ExecutionError {
                                    error_type: ErrorType::SystemError,
                                    message: format!(
                                        "Failed to launch terminal for '{}': {}",
                                        executable, e
                                    ),
                                    details: Some(serde_json::json!({
                                        "executable": executable,
                                        "error": e.to_string()
                                    })),
                                    recoverable: true,
                                }),
                                execution_time_ms: start_time.elapsed().as_millis() as u64,
                                retry_count: 0,
                                logs: vec![],
                            }
                        }
                    };
                }

                // ── show_terminal=false → silent execution with captured output ──
                cmd.stdout(std::process::Stdio::piped());
                cmd.stderr(std::process::Stdio::piped());
                #[cfg(target_os = "windows")]
                {
                    use std::os::windows::process::CommandExt;
                    const CREATE_NO_WINDOW: u32 = 0x08000000;
                    cmd.creation_flags(CREATE_NO_WINDOW);
                }

                cmd.current_dir(&work_dir);

                // SECURITY: Add user environment variables with warnings for risky ones
                if let Some(vars) = env_vars {
                    const RISKY_ENV_VARS: &[&str] = &[
                        "LD_PRELOAD",
                        "LD_LIBRARY_PATH",
                        "DYLD_INSERT_LIBRARIES",
                        "DYLD_LIBRARY_PATH",
                    ];

                    for (key, value) in vars.iter() {
                        let key_upper = key.to_uppercase();
                        if RISKY_ENV_VARS.contains(&key_upper.as_str()) {
                            println!(
                                "⚠️ Warning: Setting potentially risky env var: {}={}",
                                key, value
                            );
                            log::warn!("⚠️ User set risky env var: {}={}", key, value);
                        }
                        cmd.env(key, value);
                        println!("✅ Set environment variable: {}={}", key, value);
                    }
                }

                // Spawn the process
                let child = match cmd.spawn() {
                    Ok(child) => child,
                    Err(e) => {
                        println!("❌ Failed to spawn process: {:?}", e);
                        return ExecutionResult {
                            success: false,
                            outputs: HashMap::new(),
                            error: Some(ExecutionError {
                                error_type: ErrorType::SystemError,
                                message: format!("Failed to spawn process '{}': {}", executable, e),
                                details: Some(serde_json::json!({
                                    "executable": executable,
                                    "args": args,
                                    "error": e.to_string()
                                })),
                                recoverable: true,
                            }),
                            execution_time_ms: start_time.elapsed().as_millis() as u64,
                            retry_count: 0,
                            logs: vec![ExecutionLogEntry {
                                timestamp: chrono::Utc::now(),
                                level: LogLevel::Error,
                                message: format!("Failed to spawn process '{}'", executable),
                                details: Some(serde_json::Value::String(e.to_string())),
                            }],
                        };
                    }
                };

                // Extract button and deck ID from context
                let (button_id, deck_id) = match &context.trigger_source {
                    TriggerSource::Button { button_id, deck_id } => {
                        (Some(button_id.clone()), Some(deck_id.clone()))
                    }
                    _ => (None, None),
                };

                // Register the process with the tracker
                let process_id = self
                    .process_tracker
                    .register(
                        executable.clone(),
                        args.clone(),
                        child,
                        timeout_ms.or(context.timeout_ms),
                        working_dir.clone(),
                        button_id,
                        deck_id,
                    )
                    .await;

                println!("📝 Process registered with ID: {}", process_id);

                // Wait for completion through the process tracker
                match self.process_tracker.wait_for_completion(process_id).await {
                    Ok(process_result) => {
                        let success = process_result.exit_code == 0;

                        println!(
                            "📤 Program execution result - Success: {}, Exit code: {}",
                            success, process_result.exit_code
                        );

                        ExecutionResult {
                            success,
                            outputs: [
                                (
                                    "stdout".to_string(),
                                    serde_json::Value::String(process_result.stdout.clone()),
                                ),
                                (
                                    "stderr".to_string(),
                                    serde_json::Value::String(process_result.stderr.clone()),
                                ),
                                (
                                    "exit_code".to_string(),
                                    serde_json::Value::Number(serde_json::Number::from(
                                        process_result.exit_code,
                                    )),
                                ),
                                (
                                    "process_id".to_string(),
                                    serde_json::Value::String(process_id.to_string()),
                                ),
                            ]
                            .into(),
                            error: if !success {
                                Some(ExecutionError {
                                    error_type: ErrorType::SystemError,
                                    message: format!(
                                        "Program exited with code {}: {}",
                                        process_result.exit_code, process_result.stderr
                                    ),
                                    details: Some(serde_json::json!({
                                        "executable": executable,
                                        "args": args,
                                        "exit_code": process_result.exit_code,
                                        "stderr": process_result.stderr,
                                        "process_id": process_id.to_string()
                                    })),
                                    recoverable: true,
                                })
                            } else {
                                None
                            },
                            execution_time_ms: process_result.execution_time_ms,
                            retry_count: 0,
                            logs: vec![ExecutionLogEntry {
                                timestamp: chrono::Utc::now(),
                                level: if success {
                                    LogLevel::Info
                                } else {
                                    LogLevel::Error
                                },
                                message: format!(
                                    "Program '{}' executed with exit code {} (Process ID: {}){}",
                                    executable,
                                    process_result.exit_code,
                                    process_id,
                                    if *show_terminal {
                                        " - Terminal mode"
                                    } else {
                                        ""
                                    }
                                ),
                                details: Some(serde_json::json!({
                                    "stdout_length": process_result.stdout.len(),
                                    "stderr_length": process_result.stderr.len(),
                                    "process_id": process_id.to_string(),
                                    "terminal_mode": show_terminal
                                })),
                            }],
                        }
                    }
                    Err(e) => {
                        println!("❌ Program execution failed: {:?}", e);
                        ExecutionResult {
                            success: false,
                            outputs: [(
                                "process_id".to_string(),
                                serde_json::Value::String(process_id.to_string()),
                            )]
                            .into(),
                            error: Some(ExecutionError {
                                error_type: match e {
                                    ProcessError::Timeout => ErrorType::Timeout,
                                    _ => ErrorType::SystemError,
                                },
                                message: format!("Program execution failed: {}", e),
                                details: Some(serde_json::json!({
                                    "executable": executable,
                                    "args": args,
                                    "error": e.to_string(),
                                    "process_id": process_id.to_string()
                                })),
                                recoverable: true,
                            }),
                            execution_time_ms: start_time.elapsed().as_millis() as u64,
                            retry_count: 0,
                            logs: vec![ExecutionLogEntry {
                                timestamp: chrono::Utc::now(),
                                level: LogLevel::Error,
                                message: format!(
                                    "Program '{}' execution failed (Process ID: {})",
                                    executable, process_id
                                ),
                                details: Some(serde_json::Value::String(e.to_string())),
                            }],
                        }
                    }
                }
            }
            UniversalAction::HttpRequest {
                name,
                method,
                url,
                headers,
                body,
                timeout_ms,
                accepted_status_codes,
            } => {
                println!(
                    "[UniversalExecutionEngine] Executing HTTP request '{}'",
                    name
                );

                let mut request_headers = headers.clone();
                let method_supports_idempotency = matches!(
                    method.to_ascii_uppercase().as_str(),
                    "POST" | "PUT" | "PATCH" | "DELETE"
                );
                if method_supports_idempotency
                    && !request_headers
                        .keys()
                        .any(|key| key.eq_ignore_ascii_case("Idempotency-Key"))
                {
                    if let Some(key) = context
                        .execution_config
                        .as_ref()
                        .and_then(|value| value.get("idempotencyKey"))
                        .and_then(Value::as_str)
                    {
                        request_headers.insert("Idempotency-Key".to_string(), key.to_string());
                    }
                }

                // Build the HTTP request config from the action data (NO DATABASE ACCESS!)
                let config = crate::services::api::HttpRequestConfig {
                    method: method.clone(),
                    url: url.clone(),
                    authentication: crate::services::api::AuthType::None,
                    headers: request_headers,
                    body: body.clone(),
                    timeout_ms: *timeout_ms,
                    response_format:
                        crate::services::api::http_request::ResponseFormat::FullResponse,
                    retry: None,
                    pagination: None,
                };

                // Execute the HTTP request
                match crate::services::api::execute_http_request(config).await {
                    Ok(response) => {
                        let status =
                            response.get("status").and_then(Value::as_u64).unwrap_or(0) as u16;
                        let success = crate::services::api::saved_request::accepts_status(
                            accepted_status_codes.as_deref(),
                            status,
                        );
                        let response_body = response.get("body").cloned().unwrap_or(Value::Null);
                        ExecutionResult {
                            success,
                            outputs: HashMap::from([
                                ("response".into(), response_body),
                                ("status".into(), serde_json::json!(status)),
                                (
                                    "headers".into(),
                                    response.get("headers").cloned().unwrap_or(Value::Null),
                                ),
                            ]),
                            error: (!success).then(|| ExecutionError {
                                error_type: ErrorType::NetworkError,
                                message: format!(
                                    "HTTP request returned unaccepted status {}",
                                    status
                                ),
                                details: Some(serde_json::json!({ "status": status })),
                                recoverable: false,
                            }),
                            execution_time_ms: start_time.elapsed().as_millis() as u64,
                            retry_count: 0,
                            logs: vec![ExecutionLogEntry {
                                timestamp: chrono::Utc::now(),
                                level: if success {
                                    LogLevel::Info
                                } else {
                                    LogLevel::Error
                                },
                                message: format!(
                                    "HTTP request '{}' returned status {}",
                                    name, status
                                ),
                                details: None,
                            }],
                        }
                    }
                    Err(e) => {
                        println!(
                            "[UniversalExecutionEngine] HTTP request '{}' failed: {:?}",
                            name, e
                        );
                        ExecutionResult {
                            success: false,
                            outputs: HashMap::new(),
                            error: Some(ExecutionError {
                                error_type: ErrorType::NetworkError,
                                message: format!("HTTP request '{}' failed: {}", name, e),
                                details: Some(serde_json::json!({
                                    "request_name": name,
                                    "method": method,
                                })),
                                recoverable: matches!(
                                    method.to_ascii_uppercase().as_str(),
                                    "GET" | "HEAD" | "OPTIONS"
                                ),
                            }),
                            execution_time_ms: start_time.elapsed().as_millis() as u64,
                            retry_count: 0,
                            logs: vec![ExecutionLogEntry {
                                timestamp: chrono::Utc::now(),
                                level: LogLevel::Error,
                                message: format!("HTTP request '{}' failed", name),
                                details: Some(serde_json::Value::String(e.to_string())),
                            }],
                        }
                    }
                }
            }
            UniversalAction::FocusWindow {
                name,
                window_title,
                process_name,
                match_mode,
            } => {
                println!(
                    "[UniversalExecutionEngine] Executing Focus Window '{}'",
                    name
                );

                // Use the window management service to focus the window
                match crate::services::system::window_management::focus_window(
                    window_title,
                    process_name,
                    match_mode,
                ) {
                    Ok(()) => {
                        println!(
                            "[UniversalExecutionEngine] Window '{}' focused successfully",
                            name
                        );
                        ExecutionResult {
                            success: true,
                            outputs: [(
                                "result".to_string(),
                                serde_json::json!({
                                    "window_title": window_title,
                                    "process_name": process_name,
                                    "match_mode": match_mode,
                                    "status": "focused"
                                }),
                            )]
                            .into(),
                            error: None,
                            execution_time_ms: start_time.elapsed().as_millis() as u64,
                            retry_count: 0,
                            logs: vec![ExecutionLogEntry {
                                timestamp: chrono::Utc::now(),
                                level: LogLevel::Info,
                                message: format!(
                                    "Window '{}' [{}] focused successfully",
                                    window_title, process_name
                                ),
                                details: None,
                            }],
                        }
                    }
                    Err(e) => {
                        println!(
                            "[UniversalExecutionEngine] Failed to focus window '{}': {}",
                            name, e
                        );
                        ExecutionResult {
                            success: false,
                            outputs: HashMap::new(),
                            error: Some(ExecutionError {
                                error_type: ErrorType::SystemError,
                                message: format!("Failed to focus window '{}': {}", name, e),
                                details: Some(serde_json::json!({
                                    "window_title": window_title,
                                    "process_name": process_name,
                                    "match_mode": match_mode,
                                    "error": e.to_string(),
                                })),
                                recoverable: true,
                            }),
                            execution_time_ms: start_time.elapsed().as_millis() as u64,
                            retry_count: 0,
                            logs: vec![ExecutionLogEntry {
                                timestamp: chrono::Utc::now(),
                                level: LogLevel::Error,
                                message: format!(
                                    "Failed to focus window '{}' [{}]",
                                    window_title, process_name
                                ),
                                details: Some(serde_json::Value::String(e.to_string())),
                            }],
                        }
                    }
                }
            }
            UniversalAction::SystemControl {
                command,
                parameters,
            } => {
                println!(
                    "[UniversalExecutionEngine] Executing system control command: {}",
                    command
                );

                // Use the existing SystemControl service (same as legacy code in main.rs)
                #[cfg(target_os = "windows")]
                {
                    use crate::services::system::SystemControl;

                    let system_control = SystemControl::new();
                    let params_value = if let Some(params) = parameters {
                        serde_json::to_value(params).unwrap_or(serde_json::json!({}))
                    } else {
                        serde_json::json!({})
                    };

                    match system_control.execute_action(command, params_value).await {
                        Ok(result) => ExecutionResult {
                            success: true,
                            outputs: [("result".to_string(), result)].into(),
                            error: None,
                            execution_time_ms: start_time.elapsed().as_millis() as u64,
                            retry_count: 0,
                            logs: vec![ExecutionLogEntry {
                                timestamp: chrono::Utc::now(),
                                level: LogLevel::Info,
                                message: format!(
                                    "System control command '{}' executed successfully",
                                    command
                                ),
                                details: None,
                            }],
                        },
                        Err(e) => ExecutionResult {
                            success: false,
                            outputs: HashMap::new(),
                            error: Some(ExecutionError {
                                error_type: ErrorType::SystemError,
                                message: format!(
                                    "System control command '{}' failed: {}",
                                    command, e
                                ),
                                details: Some(serde_json::json!({"command": command})),
                                recoverable: false,
                            }),
                            execution_time_ms: start_time.elapsed().as_millis() as u64,
                            retry_count: 0,
                            logs: vec![],
                        },
                    }
                }
                #[cfg(not(target_os = "windows"))]
                {
                    ExecutionResult {
                        success: false,
                        outputs: HashMap::new(),
                        error: Some(ExecutionError {
                            error_type: ErrorType::SystemError,
                            message: "System control not supported on this platform".to_string(),
                            details: None,
                            recoverable: false,
                        }),
                        execution_time_ms: start_time.elapsed().as_millis() as u64,
                        retry_count: 0,
                        logs: vec![],
                    }
                }
            }
            UniversalAction::KeySequence { keys, timing } => {
                // TODO: Migrate legacy keyboard simulation (KeyDown/KeyUp from old Action enum)
                ExecutionResult {
                    success: false,
                    outputs: HashMap::new(),
                    error: Some(ExecutionError {
                        error_type: ErrorType::SystemError,
                        message: "KeySequence action not yet implemented - needs migration from legacy code".to_string(),
                        details: Some(serde_json::json!({"key_count": keys.len()})),
                        recoverable: false,
                    }),
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                    retry_count: 0,
                    logs: vec![],
                }
            }
            UniversalAction::MouseAction {
                action_type,
                coordinates,
                button,
            } => {
                // TODO: Migrate legacy mouse simulation (MouseMove/MouseClick from old Action enum)
                ExecutionResult {
                    success: false,
                    outputs: HashMap::new(),
                    error: Some(ExecutionError {
                        error_type: ErrorType::SystemError,
                        message:
                            "MouseAction not yet implemented - needs migration from legacy code"
                                .to_string(),
                        details: Some(
                            serde_json::json!({"action_type": format!("{:?}", action_type)}),
                        ),
                        recoverable: false,
                    }),
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                    retry_count: 0,
                    logs: vec![],
                }
            }
            UniversalAction::Sequence {
                actions,
                execution_mode,
            } => {
                // Sequence execution is actually working - it recursively calls execute_action
                let results = self
                    .execute_sequence(actions, execution_mode.clone(), context)
                    .await;
                let all_success = results.iter().all(|r| r.success);
                let total_time: u64 = results.iter().map(|r| r.execution_time_ms).sum();

                ExecutionResult {
                    success: all_success,
                    outputs: [("results".to_string(), serde_json::json!(results))].into(),
                    error: if !all_success {
                        Some(ExecutionError {
                            error_type: ErrorType::SystemError,
                            message: "One or more actions in sequence failed".to_string(),
                            details: Some(serde_json::json!({
                                "total_actions": actions.len(),
                                "failed_count": results.iter().filter(|r| !r.success).count()
                            })),
                            recoverable: false,
                        })
                    } else {
                        None
                    },
                    execution_time_ms: total_time,
                    retry_count: 0,
                    logs: vec![ExecutionLogEntry {
                        timestamp: chrono::Utc::now(),
                        level: if all_success {
                            LogLevel::Info
                        } else {
                            LogLevel::Error
                        },
                        message: format!("Sequence of {} actions completed", actions.len()),
                        details: Some(serde_json::json!({
                            "mode": format!("{:?}", execution_mode),
                            "total_actions": actions.len(),
                            "success_count": results.iter().filter(|r| r.success).count()
                        })),
                    }],
                }
            }
            UniversalAction::Conditional {
                condition,
                if_true,
                if_false,
            } => {
                // TODO: Implement conditional logic (no legacy equivalent to migrate from)
                ExecutionResult {
                    success: false,
                    outputs: HashMap::new(),
                    error: Some(ExecutionError {
                        error_type: ErrorType::SystemError,
                        message: "Conditional actions not yet implemented".to_string(),
                        details: Some(serde_json::json!({"condition": format!("{:?}", condition)})),
                        recoverable: false,
                    }),
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                    retry_count: 0,
                    logs: vec![],
                }
            }
            UniversalAction::AppControl {
                app_identifier,
                action,
                parameters,
            } => {
                // TODO: Migrate legacy App action handling from old Action enum
                ExecutionResult {
                    success: false,
                    outputs: HashMap::new(),
                    error: Some(ExecutionError {
                        error_type: ErrorType::SystemError,
                        message: format!(
                            "AppControl action {:?} not yet implemented - needs migration from legacy code",
                            action
                        ),
                        details: Some(
                            serde_json::json!({"app": app_identifier, "action": format!("{:?}", action)}),
                        ),
                        recoverable: false,
                    }),
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                    retry_count: 0,
                    logs: vec![],
                }
            }
            UniversalAction::Plugin {
                plugin_id,
                action_id,
                parameters,
            } => {
                println!(
                    "[UniversalExecutionEngine] Executing plugin action: {} / {}",
                    plugin_id, action_id
                );

                ExecutionResult {
                    success: false,
                    outputs: HashMap::new(),
                    error: Some(ExecutionError {
                        error_type: ErrorType::SystemError,
                        message: format!(
                            "Plugin action '{}:{}' is unsupported in schema 1.x; use a registered plugin FlowNode or a host integration capability",
                            plugin_id, action_id
                        ),
                        details: Some(serde_json::json!({
                            "plugin_id": plugin_id,
                            "action_id": action_id,
                            "parameters": parameters
                        })),
                        recoverable: false,
                    }),
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                    retry_count: 0,
                    logs: vec![ExecutionLogEntry {
                        timestamp: chrono::Utc::now(),
                        level: LogLevel::Warning,
                        message: "Unsupported legacy plugin action was rejected before dispatch"
                            .to_string(),
                        details: Some(
                            serde_json::json!({"plugin_id": plugin_id, "action_id": action_id}),
                        ),
                    }],
                }
            }
        };

        // Add execution time
        let mut result = result;
        result.execution_time_ms = start_time.elapsed().as_millis() as u64;

        // 🔓 ActivePermitGuard will automatically decrement the counter when it goes out of scope

        result
    }

    async fn execute_sequence(
        &self,
        actions: &[UniversalAction],
        mode: SequenceMode,
        context: &ExecutionContext,
    ) -> Vec<ExecutionResult> {
        match mode {
            SequenceMode::Sequential => {
                let mut results = Vec::new();
                for action in actions {
                    let result = self.execute_action(action, context).await;
                    results.push(result);
                }
                results
            }
            SequenceMode::Parallel => {
                let futures: Vec<_> = actions
                    .iter()
                    .map(|action| self.execute_action(action, context))
                    .collect();
                futures::future::join_all(futures).await
            }
            SequenceMode::ConditionalChain => {
                let mut results = Vec::new();
                for action in actions {
                    let result = self.execute_action(action, context).await;
                    let success = result.success;
                    results.push(result);
                    if !success {
                        break; // Stop on first failure
                    }
                }
                results
            }
        }
    }

    async fn validate_action(
        &self,
        action: &UniversalAction,
        context: &ExecutionContext,
    ) -> Result<(), ExecutionError> {
        if let UniversalAction::Integration { .. } = action {
            let capability = self
                .resolve_integration_capability(action)
                .await?
                .expect("integration actions always resolve a capability");

            // Local buttons/macros/flows are owner-authored capabilities. A
            // package is an untrusted principal and must have the exact durable
            // manifest grant. Remote transports authenticate/authorize before
            // constructing their local Button/Flow execution context.
            if let TriggerSource::Plugin { plugin_id } = &context.trigger_source {
                let checker = self
                    .permission_checker
                    .as_ref()
                    .ok_or_else(|| ExecutionError {
                        error_type: ErrorType::PermissionDenied,
                        message: "Package permission authority is unavailable".to_string(),
                        details: None,
                        recoverable: false,
                    })?;
                checker
                    .enforce_permission(plugin_id, &capability.contract.required_permission)
                    .map_err(|message| ExecutionError {
                        error_type: ErrorType::PermissionDenied,
                        message,
                        details: Some(serde_json::json!({
                            "pluginId": plugin_id,
                            "requiredPermission": capability.contract.required_permission,
                            "contractVersion": capability.version,
                        })),
                        recoverable: false,
                    })?;
            }
        }
        Ok(())
    }

    async fn get_available_actions(&self, context: &ExecutionContext) -> Vec<ActionDescriptor> {
        let mut actions = Vec::new();

        // Integration actions are resolved by registered providers at runtime.
        actions.push(ActionDescriptor {
            id: "integration".to_string(),
            name: "Integration Action".to_string(),
            description: "Execute an action published by a registered integration".to_string(),
            category: "Integration".to_string(),
            parameters: vec![
                ParameterDescriptor {
                    name: "integration_id".to_string(),
                    parameter_type: "string".to_string(),
                    required: true,
                    default_value: None,
                    description: "Registered integration ID".to_string(),
                },
                ParameterDescriptor {
                    name: "action_id".to_string(),
                    parameter_type: "string".to_string(),
                    required: true,
                    default_value: None,
                    description: "Action ID to execute".to_string(),
                },
            ],
            permissions_required: vec!["integration:<integration-id>:<action-id>".to_string()],
        });

        // Add macro actions
        actions.push(ActionDescriptor {
            id: "macro".to_string(),
            name: "Execute Macro".to_string(),
            description: "Execute a recorded macro sequence".to_string(),
            category: "Automation".to_string(),
            parameters: vec![ParameterDescriptor {
                name: "macro_id".to_string(),
                parameter_type: "string".to_string(),
                required: true,
                default_value: None,
                description: "ID of the macro to execute".to_string(),
            }],
            permissions_required: vec!["macro_execution".to_string()],
        });

        // Add flow actions
        actions.push(ActionDescriptor {
            id: "flow".to_string(),
            name: "Execute Flow".to_string(),
            description: "Execute a flow with nodes and connections".to_string(),
            category: "Automation".to_string(),
            parameters: vec![ParameterDescriptor {
                name: "flow_id".to_string(),
                parameter_type: "string".to_string(),
                required: true,
                default_value: None,
                description: "ID of the flow to execute".to_string(),
            }],
            permissions_required: vec!["flow_execution".to_string()],
        });

        actions
    }
}

#[cfg(test)]
mod concurrency_lifecycle_tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn context(mode: ButtonExecutionMode, button_id: &str) -> ExecutionContext {
        ExecutionContext {
            trigger_source: TriggerSource::Button {
                button_id: button_id.into(),
                deck_id: "deck-1".into(),
            },
            user_id: None,
            session_id: "concurrency-test".into(),
            variables: HashMap::new(),
            timeout_ms: None,
            retry_config: RetryConfig {
                max_retries: 0,
                retry_delay_ms: 0,
                exponential_backoff: false,
            },
            execution_mode: mode,
            execution_config: Some(serde_json::json!({ "maxDepth": 16 })),
        }
    }

    fn failing_leaf(id: &str) -> UniversalAction {
        UniversalAction::Plugin {
            plugin_id: "test.plugin".into(),
            action_id: id.into(),
            parameters: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn limit_one_nested_and_deep_sequences_terminate_without_self_deadlock() {
        let engine =
            UniversalExecutionEngine::with_concurrency_limit(Arc::new(StateManager::new()), 1);
        let deep = UniversalAction::Sequence {
            execution_mode: SequenceMode::Sequential,
            actions: vec![
                UniversalAction::Sequence {
                    execution_mode: SequenceMode::Sequential,
                    actions: vec![UniversalAction::Sequence {
                        execution_mode: SequenceMode::Parallel,
                        actions: vec![failing_leaf("deep-a"), failing_leaf("deep-b")],
                    }],
                },
                failing_leaf("sibling"),
            ],
        };
        let result = tokio::time::timeout(
            Duration::from_secs(1),
            engine.execute_action(&deep, &context(ButtonExecutionMode::RunOnce, "direct")),
        )
        .await
        .expect("nested sequence must not wait on its own permit");
        assert!(!result.success);
    }

    #[tokio::test]
    async fn mixed_queued_and_direct_nested_work_both_make_progress_at_limit_one() {
        let engine =
            UniversalExecutionEngine::with_concurrency_limit(Arc::new(StateManager::new()), 1);
        let nested = UniversalAction::Sequence {
            execution_mode: SequenceMode::Sequential,
            actions: vec![failing_leaf("queued-a"), failing_leaf("queued-b")],
        };
        let queued_engine = engine.clone();
        let queued = tokio::spawn(async move {
            queued_engine
                .execute_action(&nested, &context(ButtonExecutionMode::Queue, "queued"))
                .await
        });
        let direct_engine = engine.clone();
        let direct = tokio::spawn(async move {
            direct_engine
                .execute_action(
                    &failing_leaf("direct"),
                    &context(ButtonExecutionMode::RunOnce, "direct"),
                )
                .await
        });
        let (queued_result, direct_result) = tokio::time::timeout(Duration::from_secs(1), async {
            tokio::join!(queued, direct)
        })
        .await
        .expect("unrelated queue and direct work must both progress");
        assert!(!queued_result.unwrap().success);
        assert!(!direct_result.unwrap().success);
    }

    #[tokio::test]
    async fn global_leaf_capacity_is_bounded_and_cancelled_waiter_does_not_leak() {
        let engine =
            UniversalExecutionEngine::with_concurrency_limit(Arc::new(StateManager::new()), 2);
        let current = Arc::new(AtomicUsize::new(0));
        let maximum = Arc::new(AtomicUsize::new(0));
        let started = Instant::now();
        let mut workers = Vec::new();
        for _ in 0..12 {
            let engine = engine.clone();
            let current = current.clone();
            let maximum = maximum.clone();
            workers.push(tokio::spawn(async move {
                let _guard = engine.acquire_leaf_permit().await.unwrap();
                let active = current.fetch_add(1, Ordering::SeqCst) + 1;
                maximum.fetch_max(active, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(10)).await;
                current.fetch_sub(1, Ordering::SeqCst);
            }));
        }
        for worker in workers {
            worker.await.unwrap();
        }
        let elapsed = started.elapsed();
        assert_eq!(maximum.load(Ordering::SeqCst), 2);
        println!(
            "R03 workload: 12 leaves, limit=2, observed_max={}, elapsed_ms={}",
            maximum.load(Ordering::SeqCst),
            elapsed.as_millis()
        );

        let single =
            UniversalExecutionEngine::with_concurrency_limit(Arc::new(StateManager::new()), 1);
        let held = single.acquire_leaf_permit().await.unwrap();
        let waiting_engine = single.clone();
        let waiter = tokio::spawn(async move { waiting_engine.acquire_leaf_permit().await });
        tokio::time::sleep(Duration::from_millis(10)).await;
        waiter.abort();
        let cancellation = match waiter.await {
            Err(error) => error,
            Ok(_) => panic!("aborted permit waiter unexpectedly completed"),
        };
        assert!(cancellation.is_cancelled());
        drop(held);
        tokio::time::timeout(Duration::from_secs(1), single.acquire_leaf_permit())
            .await
            .expect("cancelled waiter must not consume future capacity")
            .unwrap();
    }
}

#[cfg(test)]
mod queue_lifecycle_tests {
    use super::*;
    use std::time::Duration;

    fn context(invocation_id: &str) -> ExecutionContext {
        ExecutionContext {
            trigger_source: TriggerSource::Button {
                button_id: "button-1".into(),
                deck_id: "deck-1".into(),
            },
            user_id: None,
            session_id: "queue-test".into(),
            variables: HashMap::new(),
            timeout_ms: None,
            retry_config: RetryConfig {
                max_retries: 0,
                retry_delay_ms: 0,
                exponential_backoff: false,
            },
            execution_mode: ButtonExecutionMode::Queue,
            execution_config: Some(serde_json::json!({
                "invocationId": invocation_id,
                "maxDepth": 8,
            })),
        }
    }

    fn successful_action() -> UniversalAction {
        UniversalAction::Sequence {
            actions: vec![],
            execution_mode: SequenceMode::Sequential,
        }
    }

    fn failing_action() -> UniversalAction {
        UniversalAction::Plugin {
            plugin_id: "test.plugin".into(),
            action_id: "fail".into(),
            parameters: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn queued_call_waits_for_its_own_terminal_failure_and_next_call_succeeds() {
        let engine = UniversalExecutionEngine::new(Arc::new(StateManager::new()));
        let (first_admission_tx, first_admission_rx) = oneshot::channel();
        let (first_completion_tx, first_completion_rx) = oneshot::channel();
        {
            let mut queues = engine.execution_queues.lock().await;
            queues.insert(
                "button-1".into(),
                ExecutionQueueState {
                    worker_running: true,
                    pending: VecDeque::from([QueuedExecution {
                        action: successful_action(),
                        context: {
                            let mut value = context("invocation-first");
                            value.execution_mode = ButtonExecutionMode::RunOnce;
                            value
                        },
                        admission: first_admission_rx,
                        completion: first_completion_tx,
                    }]),
                },
            );
        }
        let worker = engine.clone();
        tokio::spawn(async move {
            worker.drain_execution_queue("button-1".into()).await;
        });

        let second_engine = engine.clone();
        let mut second = tokio::spawn(async move {
            second_engine
                .execute_action(&failing_action(), &context("invocation-second"))
                .await
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(
            !second.is_finished(),
            "queue admission must not be returned as terminal success"
        );

        first_admission_tx.send(Ok(())).unwrap();
        assert!(first_completion_rx.await.unwrap().success);
        let second_result = second.await.unwrap();
        assert!(!second_result.success);
        assert!(second_result
            .error
            .as_ref()
            .unwrap()
            .message
            .contains("unsupported in schema 1.x"));

        let third = engine
            .execute_action(&successful_action(), &context("invocation-third"))
            .await;
        assert!(
            third.success,
            "a later queue item must retain its own result"
        );
    }

    #[tokio::test]
    async fn queue_depth_rejection_is_terminal_failure_not_acceptance() {
        let engine = UniversalExecutionEngine::new(Arc::new(StateManager::new()));
        let (admission_tx, admission_rx) = oneshot::channel();
        let (completion_tx, _completion_rx) = oneshot::channel();
        {
            let mut queues = engine.execution_queues.lock().await;
            queues.insert(
                "button-1".into(),
                ExecutionQueueState {
                    worker_running: true,
                    pending: VecDeque::from([QueuedExecution {
                        action: successful_action(),
                        context: context("held"),
                        admission: admission_rx,
                        completion: completion_tx,
                    }]),
                },
            );
        }
        let mut limited = context("rejected");
        limited.execution_config = Some(serde_json::json!({
            "invocationId": "rejected",
            "maxDepth": 2,
        }));
        let result = engine.execute_action(&successful_action(), &limited).await;
        assert!(!result.success);
        assert!(matches!(
            result.error.unwrap().error_type,
            ErrorType::ConcurrencyLimitReached
        ));
        drop(admission_tx);
    }
}

#[cfg(all(test, feature = "obs-integration"))]
mod action_contract_tests {
    use super::*;
    use crate::integrations::builtin::obs::ObsIntegration;
    use crate::plugins::bundle::PluginPermission;

    async fn obs_state_with_action() -> Arc<StateManager> {
        let state = Arc::new(StateManager::new());
        state
            .handle_obs_action_register(&[serde_json::json!({
                "actionId": "set_scene",
                "name": "Set scene",
                "parameters": [{
                    "assignmentMessage": "Scene",
                    "errorMessage": "Invalid scene",
                    "listSelection": [{ "displayName": "Main", "value": "Main" }],
                    "parameterId": "sceneName",
                    "parameterType": "LIST"
                }]
            })])
            .await
            .unwrap();
        state
    }

    async fn obs_engine() -> UniversalExecutionEngine {
        let state = obs_state_with_action().await;
        let manager = Arc::new(UniversalIntegrationManager::new(state.clone()));
        manager
            .register_integration(Arc::new(ObsIntegration::new(state.clone())))
            .await
            .unwrap();
        UniversalExecutionEngine::new(state).with_integration_manager(manager)
    }

    fn plugin_context(plugin_id: &str) -> ExecutionContext {
        ExecutionContext {
            trigger_source: TriggerSource::Plugin {
                plugin_id: plugin_id.to_string(),
            },
            user_id: None,
            session_id: "permission-test".into(),
            variables: HashMap::new(),
            timeout_ms: None,
            retry_config: RetryConfig {
                max_retries: 0,
                retry_delay_ms: 0,
                exponential_backoff: false,
            },
            execution_mode: ButtonExecutionMode::RunOnce,
            execution_config: None,
        }
    }

    #[tokio::test]
    async fn package_obs_action_requires_existing_manifest_permission() {
        let checker = Arc::new(crate::plugins::permissions::PermissionChecker::new());
        let engine = obs_engine().await.with_permission_checker(checker.clone());
        let action = UniversalAction::Integration {
            integration_id: "obs".into(),
            action_id: "set_scene".into(),
            parameters: HashMap::from([("sceneName".into(), serde_json::json!("Main"))]),
        };
        let context = plugin_context("plugin.test");

        let denied = engine.validate_action(&action, &context).await.unwrap_err();
        assert!(matches!(denied.error_type, ErrorType::PermissionDenied));

        checker.grant_permissions(
            "plugin.test",
            vec![PluginPermission::TriggerAction(
                "integration:obs:set_scene".into(),
            )],
        );
        engine.validate_action(&action, &context).await.unwrap();
    }

    #[tokio::test]
    async fn owner_flow_still_validates_contract_without_package_grant() {
        let engine = obs_engine().await;
        let action = UniversalAction::Integration {
            integration_id: "obs".into(),
            action_id: "set_scene".into(),
            parameters: HashMap::from([("sceneName".into(), serde_json::json!("Main"))]),
        };
        let mut context = plugin_context("unused");
        context.trigger_source = TriggerSource::Flow {
            flow_id: "flow-1".into(),
            node_id: "node-1".into(),
        };
        engine.validate_action(&action, &context).await.unwrap();
    }
}
