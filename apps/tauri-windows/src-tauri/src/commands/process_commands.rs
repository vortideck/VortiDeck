use std::collections::HashMap;
use std::sync::Arc;
use tauri::State;
use uuid::Uuid;
use vortideck_core::core::state_manager::StateManager;
use vortideck_core::services::execution::{
    ActionInvocationService, ErrorType, ExecutionError, ProcessStatus, ResourceUsage,
    TrackedProcess, UniversalExecutionEngine,
};

// ============================================================================
// PROCESS MANAGEMENT COMMANDS
// ============================================================================

/// Get all tracked processes
#[tauri::command]
pub async fn get_all_processes(
    execution_engine: State<'_, Arc<UniversalExecutionEngine>>,
) -> Result<HashMap<String, TrackedProcess>, String> {
    let process_tracker = execution_engine.get_process_tracker();

    let processes = process_tracker.get_all_processes().await;

    // Convert UUID keys to strings for JSON serialization
    let result = processes
        .into_iter()
        .map(|(id, process)| (id.to_string(), process))
        .collect();

    Ok(result)
}

/// Get only running processes
#[tauri::command]
pub async fn get_running_processes(
    execution_engine: State<'_, Arc<UniversalExecutionEngine>>,
) -> Result<HashMap<String, TrackedProcess>, String> {
    let process_tracker = execution_engine.get_process_tracker();

    let processes = process_tracker.get_running_processes().await;

    // Convert UUID keys to strings for JSON serialization
    let result = processes
        .into_iter()
        .map(|(id, process)| (id.to_string(), process))
        .collect();

    Ok(result)
}

/// Get a specific process by ID
#[tauri::command]
pub async fn get_process(
    process_id: String,
    execution_engine: State<'_, Arc<UniversalExecutionEngine>>,
) -> Result<Option<TrackedProcess>, String> {
    let id = Uuid::parse_str(&process_id).map_err(|e| format!("Invalid process ID: {}", e))?;

    let process_tracker = execution_engine.get_process_tracker();

    Ok(process_tracker.get_process(id).await)
}

/// Kill a specific process
#[tauri::command]
pub async fn kill_process(
    process_id: String,
    execution_engine: State<'_, Arc<UniversalExecutionEngine>>,
) -> Result<(), String> {
    let id = Uuid::parse_str(&process_id).map_err(|e| format!("Invalid process ID: {}", e))?;

    let process_tracker = execution_engine.get_process_tracker();

    process_tracker
        .kill_process(id)
        .await
        .map_err(|e| e.to_string())
}

/// Kill all running processes
#[tauri::command]
pub async fn kill_all_processes(
    execution_engine: State<'_, Arc<UniversalExecutionEngine>>,
) -> Result<Vec<(String, bool)>, String> {
    let process_tracker = execution_engine.get_process_tracker();

    let results = process_tracker.kill_all().await;

    // Convert results to frontend-friendly format
    let formatted_results = results
        .into_iter()
        .map(|(id, result)| (id.to_string(), result.is_ok()))
        .collect();

    Ok(formatted_results)
}

/// Get process count (total, running)
#[tauri::command]
pub async fn get_process_count(
    execution_engine: State<'_, Arc<UniversalExecutionEngine>>,
) -> Result<(usize, usize), String> {
    let process_tracker = execution_engine.get_process_tracker();

    Ok(process_tracker.get_process_count().await)
}

/// Get processes by button ID
#[tauri::command]
pub async fn get_processes_by_button(
    button_id: String,
    execution_engine: State<'_, Arc<UniversalExecutionEngine>>,
) -> Result<HashMap<String, TrackedProcess>, String> {
    let process_tracker = execution_engine.get_process_tracker();

    let all_processes = process_tracker.get_all_processes().await;

    // Filter by button ID
    let filtered_processes = all_processes
        .into_iter()
        .filter(|(_, process)| process.button_id.as_ref() == Some(&button_id))
        .map(|(id, process)| (id.to_string(), process))
        .collect();

    Ok(filtered_processes)
}

/// Get processes by deck ID
#[tauri::command]
pub async fn get_processes_by_deck(
    deck_id: String,
    execution_engine: State<'_, Arc<UniversalExecutionEngine>>,
) -> Result<HashMap<String, TrackedProcess>, String> {
    let process_tracker = execution_engine.get_process_tracker();

    let all_processes = process_tracker.get_all_processes().await;

    // Filter by deck ID
    let filtered_processes = all_processes
        .into_iter()
        .filter(|(_, process)| process.deck_id.as_ref() == Some(&deck_id))
        .map(|(id, process)| (id.to_string(), process))
        .collect();

    Ok(filtered_processes)
}

/// Kill all processes for a specific button
#[tauri::command]
pub async fn kill_processes_by_button(
    button_id: String,
    execution_engine: State<'_, Arc<UniversalExecutionEngine>>,
) -> Result<Vec<(String, bool)>, String> {
    let process_tracker = execution_engine.get_process_tracker();

    let all_processes = process_tracker.get_running_processes().await;

    // Filter and kill processes by button ID
    let mut results = Vec::new();
    for (id, process) in all_processes {
        if process.button_id.as_ref() == Some(&button_id) {
            let kill_result = process_tracker.kill_process(id).await;
            results.push((id.to_string(), kill_result.is_ok()));
        }
    }

    Ok(results)
}

/// Kill all processes for a specific deck
#[tauri::command]
pub async fn kill_processes_by_deck(
    deck_id: String,
    execution_engine: State<'_, Arc<UniversalExecutionEngine>>,
) -> Result<Vec<(String, bool)>, String> {
    let process_tracker = execution_engine.get_process_tracker();

    let all_processes = process_tracker.get_running_processes().await;

    // Filter and kill processes by deck ID
    let mut results = Vec::new();
    for (id, process) in all_processes {
        if process.deck_id.as_ref() == Some(&deck_id) {
            let kill_result = process_tracker.kill_process(id).await;
            results.push((id.to_string(), kill_result.is_ok()));
        }
    }

    Ok(results)
}

// ============================================================================
// PROGRAM EXECUTION COMMANDS
// ============================================================================

/// Execute a program with optional terminal visibility
#[tauri::command]
pub async fn execute_program(
    executable: String,
    args: Vec<String>,
    working_dir: Option<String>,
    env_vars: Option<HashMap<String, String>>,
    timeout_ms: Option<u64>,
    show_terminal: Option<bool>,
    invocation_service: State<'_, Arc<ActionInvocationService>>,
) -> Result<serde_json::Value, String> {
    use vortideck_core::services::execution::{
        ExecutionContext, RetryConfig, TriggerSource, UniversalAction,
    };

    println!("🚀 Executing program: {}", executable);
    println!("📦 Args: {:?}", args);
    println!("📂 Working dir: {:?}", working_dir);
    println!("🖥️ Show terminal: {:?}", show_terminal);

    // Terminal and background launches both use the same execution boundary.
    let final_working_dir = working_dir;

    println!(
        "🔧 Final working dir for background mode: {:?}",
        final_working_dir
    );

    let universal_action = UniversalAction::Program {
        executable,
        args,
        working_dir: final_working_dir,
        env_vars,
        timeout_ms: timeout_ms.or(Some(30000)), // Default 30 second timeout
        program_type: vortideck_core::services::execution::ProgramType::Executable, // Default for manual execution
        show_terminal: show_terminal.unwrap_or(false), // Default to background mode
    };

    // Create execution context
    let context = ExecutionContext {
        trigger_source: TriggerSource::Manual {
            user_id: "test_user".to_string(),
        },
        user_id: None,
        session_id: "execution_session".to_string(),
        variables: HashMap::new(),
        timeout_ms: timeout_ms.or(Some(30000)),
        retry_config: RetryConfig {
            max_retries: 0,
            retry_delay_ms: 0,
            exponential_backoff: false,
        },
        execution_mode: vortideck_core::services::execution::ButtonExecutionMode::RunOnce,
        execution_config: None,
    };

    let result = invocation_service
        .invoke_resolved_action(&universal_action, &context)
        .await;

    // Transform ExecutionResult to frontend-expected format
    let exit_code = if result.succeeded() { 0 } else { 1 };
    let stdout = result
        .outputs
        .get("stdout")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let stderr = result
        .outputs
        .get("stderr")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    println!("🔍 Debug - ExecutionResult outputs: {:?}", result.outputs);
    println!(
        "🔍 Debug - Extracted stdout: '{}' (length: {})",
        stdout,
        stdout.len()
    );
    println!(
        "🔍 Debug - Extracted stderr: '{}' (length: {})",
        stderr,
        stderr.len()
    );

    println!("✅ Program execution successful");
    Ok(serde_json::json!({
        "exit_code": exit_code,
        "stdout": stdout,
        "stderr": stderr,
        "execution_time_ms": result.execution_time_ms,
        "success": result.succeeded(),
        "status": result.status,
        "invocationId": result.invocation_id
    }))
}
