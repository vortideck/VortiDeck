//! Shared ownership and lifecycle for the executable flow runtime.

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use tokio::sync::{oneshot, Mutex};
use tokio::task::JoinHandle;

use crate::core::{PlatformContext, PlatformError};
use crate::db::database::DbPool;
use crate::services::mcp::commands::ChatState;

use super::commands::{FlowExecutorState, FlowRegistryState};
use super::event_router::{DurableStreamEventIngress, DurableStreamEventIngressState, EventRouter};
use super::executor::{FlowExecutor, FlowStorage};

const OWNED_TASK_SHUTDOWN_DEADLINE: std::time::Duration = std::time::Duration::from_secs(10);

const STOPPED: u8 = 0;
const STARTING: u8 = 1;
const RUNNING: u8 = 2;
const STOPPING: u8 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowRuntimeStatus {
    Stopped,
    Starting,
    Running,
    Stopping,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowRuntimeStartReport {
    pub recovered_runs: usize,
    pub active_trigger_routes: usize,
}

struct RuntimeTask {
    shutdown: oneshot::Sender<()>,
    join: JoinHandle<()>,
}

/// The single owner of flow recovery, durable stream ingress and the live event
/// router. Hosts keep their UI/transport adapters outside this type and share
/// these exact state handles with commands rather than constructing another
/// executor.
#[derive(Clone)]
pub struct FlowRuntime {
    platform_context: Arc<PlatformContext>,
    flow_registry: FlowRegistryState,
    flow_executor: FlowExecutorState,
    chat_state: ChatState,
    durable_stream_ingress: DurableStreamEventIngressState,
    status: Arc<AtomicU8>,
    task: Arc<Mutex<Option<RuntimeTask>>>,
    lifecycle: Arc<Mutex<()>>,
}

impl FlowRuntime {
    pub fn from_initialized(
        platform_context: Arc<PlatformContext>,
        flow_registry: FlowRegistryState,
        flow_executor: FlowExecutorState,
        chat_state: ChatState,
        durable_stream_ingress: DurableStreamEventIngressState,
    ) -> Self {
        if let Ok(executor) = flow_executor.executor.try_lock() {
            if let Some(executor) = executor.as_ref() {
                executor.mark_runtime_owned();
            }
        }
        Self {
            platform_context,
            flow_registry,
            flow_executor,
            chat_state,
            durable_stream_ingress,
            status: Arc::new(AtomicU8::new(STOPPED)),
            task: Arc::new(Mutex::new(None)),
            lifecycle: Arc::new(Mutex::new(())),
        }
    }

    /// Construct the same runtime state used by the desktop host. Providers may
    /// be registered on `flow_registry()` before `start()`.
    pub fn new_universal(
        platform_context: Arc<PlatformContext>,
        db_pool: DbPool,
    ) -> Result<Self, PlatformError> {
        let flow_registry = FlowRegistryState::new();
        let flow_executor = FlowExecutorState::new();
        let chat_state = ChatState::new_with_flow_executor(Some(flow_executor.executor.clone()));
        let executor = FlowExecutor::new_universal(
            Arc::new(flow_registry.clone()),
            Arc::new(chat_state.clone()),
            platform_context.clone(),
            db_pool,
        )?;
        executor.mark_runtime_owned();
        *flow_executor
            .executor
            .try_lock()
            .expect("a newly-created executor slot cannot be contended") = Some(executor);
        Ok(Self::from_initialized(
            platform_context,
            flow_registry,
            flow_executor,
            chat_state,
            DurableStreamEventIngressState::default(),
        ))
    }

    /// Construct a runtime with explicitly owned storage. This is the entry
    /// point for an isolated local host and deterministic lifecycle tests.
    pub fn new_with_storage(
        platform_context: Arc<PlatformContext>,
        storage: Arc<dyn FlowStorage>,
    ) -> Self {
        let flow_registry = FlowRegistryState::new();
        let flow_executor = FlowExecutorState::new();
        let chat_state = ChatState::new_with_flow_executor(Some(flow_executor.executor.clone()));
        let executor = FlowExecutor::new_with_storage(
            Arc::new(flow_registry.clone()),
            Arc::new(chat_state.clone()),
            platform_context.clone(),
            storage,
        );
        executor.mark_runtime_owned();
        *flow_executor
            .executor
            .try_lock()
            .expect("a newly-created executor slot cannot be contended") = Some(executor);
        Self::from_initialized(
            platform_context,
            flow_registry,
            flow_executor,
            chat_state,
            DurableStreamEventIngressState::default(),
        )
    }

    pub fn flow_registry(&self) -> FlowRegistryState {
        self.flow_registry.clone()
    }

    pub fn flow_executor(&self) -> FlowExecutorState {
        self.flow_executor.clone()
    }

    pub fn chat_state(&self) -> ChatState {
        self.chat_state.clone()
    }

    pub fn durable_stream_ingress(&self) -> DurableStreamEventIngressState {
        self.durable_stream_ingress.clone()
    }

    pub fn status(&self) -> FlowRuntimeStatus {
        match self.status.load(Ordering::Acquire) {
            STARTING => FlowRuntimeStatus::Starting,
            RUNNING => FlowRuntimeStatus::Running,
            STOPPING => FlowRuntimeStatus::Stopping,
            _ => FlowRuntimeStatus::Stopped,
        }
    }

    pub async fn executor(&self) -> Result<FlowExecutor, String> {
        self.flow_executor
            .executor
            .lock()
            .await
            .clone()
            .ok_or("FlowExecutor is unavailable".to_string())
    }

    pub async fn start(&self) -> Result<FlowRuntimeStartReport, String> {
        let _lifecycle = self.lifecycle.lock().await;
        self.status
            .compare_exchange(STOPPED, STARTING, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| format!("Flow runtime cannot start while {:?}", self.status()))?;

        let executor = match self.executor().await {
            Ok(executor) => executor,
            Err(error) => {
                self.status.store(STOPPED, Ordering::Release);
                return Err(error);
            }
        };
        executor.begin_runtime_startup().await;

        match self.start_inner(&executor).await {
            Ok(report) => {
                executor.open_runtime_admission().await;
                self.status.store(RUNNING, Ordering::Release);
                Ok(report)
            }
            Err(start_error) => {
                self.durable_stream_ingress.clear().await;
                let router_error = self.stop_router_task().await.err();
                let checkpoint_error = executor.prepare_for_runtime_shutdown().await.err();
                let task_error = executor
                    .finish_runtime_shutdown(OWNED_TASK_SHUTDOWN_DEADLINE)
                    .await
                    .err();
                self.status.store(STOPPED, Ordering::Release);
                let cleanup_errors = [router_error, checkpoint_error, task_error]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>();
                if cleanup_errors.is_empty() {
                    Err(start_error)
                } else {
                    Err(format!(
                        "{start_error}; runtime startup cleanup failed: {}",
                        cleanup_errors.join("; ")
                    ))
                }
            }
        }
    }

    async fn start_inner(&self, executor: &FlowExecutor) -> Result<FlowRuntimeStartReport, String> {
        let recovered_runs = executor.recover_interrupted_runs().await?;
        let active_trigger_routes = executor.rebuild_active_trigger_registry().await?;

        let ingress = DurableStreamEventIngress::new(executor.clone());
        ingress.refresh_registry().await?;
        self.durable_stream_ingress.install(ingress.clone()).await;

        let (shutdown, shutdown_receiver) = oneshot::channel();
        let router = EventRouter::with_ingress(self.platform_context.event_bus.clone(), ingress);
        let join = tokio::spawn(router.run_until(async move {
            let _ = shutdown_receiver.await;
        }));
        *self.task.lock().await = Some(RuntimeTask { shutdown, join });

        Ok(FlowRuntimeStartReport {
            recovered_runs,
            active_trigger_routes,
        })
    }

    pub async fn stop(&self) -> Result<(), String> {
        let _lifecycle = self.lifecycle.lock().await;
        match self
            .status
            .compare_exchange(RUNNING, STOPPING, Ordering::AcqRel, Ordering::Acquire)
        {
            Ok(_) => {}
            Err(STOPPED) => return Ok(()),
            Err(STOPPING) => {}
            Err(_) => {
                return Err(format!(
                    "Flow runtime cannot stop while {:?}",
                    self.status()
                ))
            }
        }

        let executor = self.executor().await;
        let admission_result = match &executor {
            Ok(executor) => {
                executor
                    .close_runtime_admission(OWNED_TASK_SHUTDOWN_DEADLINE)
                    .await
            }
            Err(error) => Err(error.clone()),
        };
        self.durable_stream_ingress.clear().await;
        let router_result = self.stop_router_task().await;

        let checkpoint_result = match &executor {
            Ok(executor) => executor.prepare_for_runtime_shutdown().await.map(|count| {
                if count > 0 {
                    log::info!("Checkpointed {count} active workflow run(s) for shutdown");
                }
            }),
            Err(error) => Err(error.clone()),
        };

        let owned_task_result = match &executor {
            Ok(executor) => {
                executor
                    .finish_runtime_shutdown(OWNED_TASK_SHUTDOWN_DEADLINE)
                    .await
            }
            Err(error) => Err(error.clone()),
        };

        let errors = [
            admission_result,
            router_result,
            checkpoint_result,
            owned_task_result,
        ]
        .into_iter()
        .filter_map(Result::err)
        .collect::<Vec<_>>();
        if errors.is_empty() {
            self.status.store(STOPPED, Ordering::Release);
            Ok(())
        } else {
            Err(errors.join("; "))
        }
    }

    async fn stop_router_task(&self) -> Result<(), String> {
        match self.task.lock().await.take() {
            Some(task) => {
                let _ = task.shutdown.send(());
                task.join
                    .await
                    .map_err(|error| format!("EventRouter task failed during shutdown: {error}"))
            }
            _ => Ok(()),
        }
    }
}
