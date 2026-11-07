use crate::db::Database;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};
use uuid::Uuid;
use vortideck_core::db::db::DbPool;
use vortideck_core::db::models::SavedActionBinding;
use vortideck_core::db::schema::saved_actions;
use vortideck_core::integrations::contracts::ActionRepairKind;
use vortideck_core::integrations::{
    ActionPreflight, ActionPreflightIssue, ActionPreflightStatus, UniversalIntegrationManager,
};
use vortideck_core::services::execution::{
    ActionInvocationService, ActionReference, ActionReferenceKind, UniversalAction,
};
use vortideck_core::services::flow::executor::FlowConnection;
use vortideck_core::services::flow::commands::{FlowExecutorState, FlowRegistryState};
use vortideck_core::services::flow::flow_control::ToolNodeInfo;
use vortideck_core::services::mcp::commands::ChatState;

// Response types for Tauri commands
#[derive(Serialize)]
pub struct ExecuteNodeResponse {
    pub success: bool,
    pub result: Option<Value>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowActionHealth {
    pub node_id: String,
    pub node_name: String,
    pub stable_action_id: String,
    pub preflight: ActionPreflight,
}

fn unresolved_action_health(
    node: &vortideck_core::services::flow::executor::FlowNodeDefinition,
    stable_action_id: String,
    code: &str,
    message: String,
) -> FlowActionHealth {
    FlowActionHealth {
        node_id: node.id.clone(),
        node_name: node.name.clone(),
        stable_action_id: stable_action_id.clone(),
        preflight: ActionPreflight {
            integration_id: String::new(),
            action_id: stable_action_id,
            status: ActionPreflightStatus::MissingTarget,
            capability: None,
            issues: vec![ActionPreflightIssue {
                code: code.to_string(),
                message,
                recoverable: true,
                repairs: vec![ActionRepairKind::RebindAction],
            }],
        },
    }
}

#[cfg(test)]
mod preflight_tests {
    use super::*;

    #[test]
    fn unresolved_binding_is_reported_as_recoverable_instead_of_aborting_preflight() {
        let node = serde_json::from_value(serde_json::json!({
            "id": "action-1",
            "nodeType": "invoke-action",
            "name": "Switch to break scene",
            "inputs": {},
            "outputs": [],
            "aiBehavior": null,
            "aiModel": null,
            "promptTemplate": null,
            "toolName": null,
            "position": [0.0, 0.0],
            "uiData": {
                "color": null,
                "icon": null,
                "collapsed": false,
                "selected": false,
                "zIndex": 0,
                "customProperties": {}
            },
            "executionConfig": {
                "timeoutMs": null,
                "retryCount": 0,
                "retryDelayMs": 0,
                "skipOnError": false,
                "parallelExecution": false
            }
        }))
        .expect("valid flow node fixture");

        let health = unresolved_action_health(
            &node,
            "recipe-binding:action-1".to_string(),
            "missing_saved_action",
            "Choose a local action".to_string(),
        );

        assert_eq!(
            health.preflight.status,
            ActionPreflightStatus::MissingTarget
        );
        assert_eq!(
            health.preflight.issues[0].repairs,
            vec![ActionRepairKind::RebindAction]
        );
        assert!(health.preflight.issues[0].recoverable);
    }
}

async fn inspect_flow_actions(
    app_handle: &AppHandle,
    db_pool: &DbPool,
    flow: &vortideck_core::services::flow::executor::FlowDefinition,
) -> Result<Vec<FlowActionHealth>, String> {
    let manager = app_handle
        .try_state::<Arc<UniversalIntegrationManager>>()
        .map(|state| state.inner().clone())
        .ok_or_else(|| "Integration contract registry is unavailable".to_string())?;
    let invocation_service = app_handle
        .try_state::<Arc<ActionInvocationService>>()
        .map(|state| state.inner().clone())
        .ok_or_else(|| "Action invocation service is unavailable".to_string())?;
    let mut results = Vec::new();

    for node in flow
        .nodes
        .iter()
        .filter(|node| node.node_type == "invoke-action")
    {
        let Some(reference_value) = node
            .inputs
            .get("actionReference")
            .or_else(|| node.ui_data.custom_properties.get("actionReference"))
        else {
            results.push(unresolved_action_health(
                node,
                String::new(),
                "missing_action_reference",
                format!("Action node '{}' needs an action binding", node.name),
            ));
            continue;
        };
        let reference: ActionReference = match serde_json::from_value(reference_value.clone()) {
            Ok(reference) => reference,
            Err(error) => {
                results.push(unresolved_action_health(
                    node,
                    String::new(),
                    "invalid_action_reference",
                    format!(
                        "Action node '{}' has an invalid reference: {}",
                        node.name, error
                    ),
                ));
                continue;
            }
        };
        // Connected values are resolved at execution. Validate static inputs now;
        // never pretend the editor already knows a value supplied by another node.
        let connected_inputs = flow.connections.iter().any(|edge| matches!(edge,
            FlowConnection::Data { to_node, to_input, .. } if to_node == &node.id && to_input == "inputs"));
        let inputs = node.inputs.get("inputs").cloned().unwrap_or_else(|| serde_json::json!({}));
        let resolution = if connected_inputs {
            invocation_service.resolve_reference(&reference, None, None).await
        } else {
            invocation_service.prepare_reference(&reference, None, None, &inputs).await
        };
        let resolved = match resolution {
            Ok(action) => action,
            Err(error) => {
                results.push(unresolved_action_health(node, reference.id, "invalid_action_inputs",
                    format!("Action node '{}' cannot be prepared: {}", node.name, error)));
                continue;
            }
        };
        if reference.kind != ActionReferenceKind::SavedAction {
            let kind = format!("{:?}", reference.kind).to_ascii_lowercase();
            results.push(FlowActionHealth {
                    node_id: node.id.clone(),
                    node_name: node.name.clone(),
                    stable_action_id: reference.id.clone(),
                    preflight: ActionPreflight {
                        integration_id: kind,
                        action_id: reference.id,
                        status: ActionPreflightStatus::Ready,
                        capability: None,
                        issues: Vec::new(),
                    },
            });
            continue;
        }
        let stable_id = reference.id;
        let mut connection = db_pool.get().map_err(|error| error.to_string())?;
        let binding = saved_actions::table
            .filter(saved_actions::stable_id.eq(&stable_id))
            .select(SavedActionBinding::as_select())
            .first::<SavedActionBinding>(&mut connection)
            .optional()
            .map_err(|error| error.to_string())?;
        let Some(binding) = binding else {
            results.push(unresolved_action_health(
                node,
                stable_id.clone(),
                "missing_saved_action",
                format!(
                    "Action node '{}' references a binding that is not installed on this device",
                    node.name
                ),
            ));
            continue;
        };
        let UniversalAction::Integration { parameters, .. } = resolved else {
            return Err("Saved integration binding resolved to a different action kind".into());
        };
        let preflight = manager
            .preflight_action_versioned(
                &binding.integration_id,
                &binding.action,
                &parameters,
                Some(binding.capability_version.max(0) as u32),
            )
            .await;
        results.push(FlowActionHealth {
            node_id: node.id.clone(),
            node_name: node.name.clone(),
            stable_action_id: stable_id,
            preflight,
        });
    }
    Ok(results)
}

#[tauri::command]
pub async fn preflight_flow_actions(
    app_handle: AppHandle,
    db_pool: State<'_, DbPool>,
    flow: vortideck_core::services::flow::executor::FlowDefinition,
) -> Result<Vec<FlowActionHealth>, String> {
    inspect_flow_actions(&app_handle, &db_pool, &flow).await
}

// ✅ PROPER ARCHITECTURE: Window management in Tauri adapter
#[tauri::command]
pub async fn open_flow_editor(
    app_handle: AppHandle,
    flow_id: Option<String>,
) -> Result<String, String> {
    if let Some(flow_id) = flow_id.as_deref()
        && (flow_id.is_empty()
            || !flow_id.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '-' | '_')
            }))
    {
        return Err("Invalid flow ID".to_string());
    }

    open_flow_editor_window(app_handle, flow_id, None).await
}

#[tauri::command]
pub async fn open_workflow_approval(
    app_handle: AppHandle,
    wake_id: String,
) -> Result<String, String> {
    if wake_id.is_empty()
        || !wake_id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err("Invalid workflow approval ID".to_string());
    }

    // Approval review is tied to the immutable run snapshot. It must focus the
    // editor without loading a saved flow over an unsaved draft.
    open_flow_editor_window(app_handle, None, Some(wake_id)).await
}

async fn open_flow_editor_window(
    app_handle: AppHandle,
    initial_flow_id: Option<String>,
    initial_approval_wake_id: Option<String>,
) -> Result<String, String> {
    // Check if the flow editor window already exists
    if let Some(flow_editor_window) = app_handle.get_webview_window("flow_editor") {
        eprintln!("Flow Editor Window already exists");
        if let Some(flow_id) = initial_flow_id {
            flow_editor_window
                .emit("load_flow_request", serde_json::json!({ "id": flow_id }))
                .map_err(|error| error.to_string())?;
        }
        let _ = crate::utils::focus_and_reveal_window(&flow_editor_window);
        if let Some(wake_id) = initial_approval_wake_id {
            flow_editor_window
                .emit(
                    "review-workflow-approval",
                    serde_json::json!({ "wakeId": wake_id }),
                )
                .map_err(|error| error.to_string())?;
        }
        return Ok("Flow editor focused".to_string());
    }

    // Get main window for positioning reference
    let main_window = app_handle
        .get_webview_window("main")
        .ok_or("Main window not found")?;

    // Define flow editor dimensions
    let base_width = 1200.0;
    let base_height = 800.0;

    // Get all available screens
    let screens = app_handle.available_monitors().map_err(|e| e.to_string())?;

    // Find the screen that doesn't contain the main window
    let main_position = main_window.outer_position().map_err(|e| e.to_string())?;
    let target_screen = screens
        .iter()
        .find(|screen| {
            let screen_position = screen.position();
            let screen_size = screen.size();
            !(main_position.x >= screen_position.x
                && main_position.x < screen_position.x + screen_size.width as i32
                && main_position.y >= screen_position.y
                && main_position.y < screen_position.y + screen_size.height as i32)
        })
        .or_else(|| screens.first());

    let (x_pos, y_pos) = if let Some(screen) = target_screen {
        // Center on the target screen
        let screen_position = screen.position();
        let screen_size = screen.size();
        (
            screen_position.x as f64 + (screen_size.width as f64 - base_width) / 2.0,
            screen_position.y as f64 + (screen_size.height as f64 - base_height) / 2.0,
        )
    } else {
        // Fallback to centering relative to main window
        let main_size = main_window.inner_size().map_err(|e| e.to_string())?;
        (
            (main_size.width as f64 - base_width) / 2.0,
            (main_size.height as f64 - base_height) / 2.0,
        )
    };

    let flow_editor_url = if let Some(wake_id) = initial_approval_wake_id {
        format!("flow?approvalWakeId={wake_id}")
    } else if let Some(flow_id) = initial_flow_id {
        format!("flow?flowId={flow_id}")
    } else {
        "flow".to_string()
    };
    let mut builder = WebviewWindowBuilder::new(
        &app_handle,
        "flow_editor",
        WebviewUrl::App(flow_editor_url.into()),
    )
    .title("VortiDeck - Flow Editor")
    .position(x_pos, y_pos)
    .resizable(true)
    .decorations(false)
    .inner_size(base_width, base_height)
    .visible(false)
    .maximizable(true)
    .minimizable(true)
    .disable_drag_drop_handler();

    // Platform-specific window settings
    #[cfg(target_os = "windows")]
    {
        builder = builder.transparent(true);
    }

    // Create the window
    let _window = builder.build().map_err(|e| e.to_string())?;

    // Emit event for the frontend
    let _ = app_handle.emit(
        "flow-editor-opened",
        serde_json::json!({
            "window_id": "flow_editor"
        }),
    );

    println!("✅ Flow Editor Window Created");
    Ok("Flow editor opened successfully".to_string())
}

// ✅ PROPER ARCHITECTURE: Tauri adapter calls core business logic
#[tauri::command]
pub async fn retrieve_system_tools(
    app_handle: AppHandle,
    flow_state: State<'_, FlowRegistryState>,
) -> Result<Vec<ToolNodeInfo>, String> {
    // Call core business logic
    let registry = flow_state.registry.lock().map_err(|e| e.to_string())?;
    let all_nodes = registry.get_all_node_info();

    // Filter to include only system nodes
    let system_nodes = all_nodes
        .into_iter()
        .filter(|node| node.type_name.to_lowercase() == "system")
        .collect();

    Ok(system_nodes)
}

#[tauri::command]
pub async fn retrieve_all_flow_nodes(
    app_handle: AppHandle,
    flow_state: State<'_, FlowRegistryState>,
) -> Result<Vec<ToolNodeInfo>, String> {
    // Call core business logic
    let registry = flow_state.registry.lock().map_err(|e| e.to_string())?;
    Ok(registry.get_all_node_info())
}

#[tauri::command]
pub async fn execute_flow_node(
    app_handle: AppHandle,
    flow_state: State<'_, FlowRegistryState>,
    node_id: String,
    parameters: Value,
) -> Result<ExecuteNodeResponse, String> {
    // Get the node and clone it, but drop the registry_guard before await
    let node = {
        let registry_guard = flow_state.registry.lock().map_err(|e| e.to_string())?;
        registry_guard
            .get_node(&node_id)
            .ok_or_else(|| format!("Node with ID '{}' not found", node_id))?
    };

    // Now we can safely await with no MutexGuard in scope
    match node.execute(parameters).await {
        Ok(result) => {
            // Emit Tauri-specific event
            let _ = app_handle.emit(
                "flow-node-executed",
                serde_json::json!({
                    "node_id": node_id,
                    "success": true,
                    "result": result
                }),
            );

            Ok(ExecuteNodeResponse {
                success: true,
                result: Some(result),
                error: None,
            })
        }
        Err(e) => {
            // Emit Tauri-specific error event
            let _ = app_handle.emit(
                "flow-node-executed",
                serde_json::json!({
                    "node_id": node_id,
                    "success": false,
                    "error": e
                }),
            );

            Ok(ExecuteNodeResponse {
                success: false,
                result: None,
                error: Some(e),
            })
        }
    }
}

#[tauri::command]
pub async fn retrieve_all_tools(
    app_handle: AppHandle,
    flow_state: State<'_, FlowRegistryState>,
) -> Result<Vec<ToolNodeInfo>, String> {
    // Call core business logic
    let registry = flow_state.registry.lock().map_err(|e| e.to_string())?;
    Ok(registry.get_all_node_info())
}

#[tauri::command]
pub async fn save_flow(
    app_handle: AppHandle,
    db_pool: State<'_, DbPool>,
    flow_executor_state: State<'_, FlowExecutorState>,
    flow: vortideck_core::services::flow::executor::FlowDefinition,
    expected_revision: Option<String>,
) -> Result<String, String> {
    if flow.is_active {
        flow.validate_graph()?;
        let health = inspect_flow_actions(&app_handle, &db_pool, &flow).await?;
        let blockers: Vec<_> = health
            .iter()
            .filter(|item| !item.preflight.is_ready())
            .collect();
        if !blockers.is_empty() {
            return Err(format!(
                "Flow activation blocked by {} unhealthy action node(s): {}",
                blockers.len(),
                blockers
                    .iter()
                    .map(|item| format!("{} ({:?})", item.node_name, item.preflight.status))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }
    let executor_guard = flow_executor_state.executor.lock().await;
    let executor = executor_guard
        .as_ref()
        .ok_or_else(|| "Flow executor not initialized".to_string())?;
    let save_result = match expected_revision.as_deref() {
        Some(revision) => executor
            .save_flow_at_revision(revision, &flow)
            .await
            .map(|_| ()),
        None => executor.save_flow(&flow).await,
    };

    match save_result {
        Ok(_) => {
            // Emit Tauri-specific event
            let _ = app_handle.emit(
                "flow-saved",
                serde_json::json!({
                    "flow_id": flow.id,
                    "name": flow.name
                }),
            );

            // 📊 Track flow save
            crate::analytics::capture(
                &app_handle,
                crate::analytics::AnalyticsEvent::new("flow_saved")
                    .prop("flow_name", flow.name.clone()),
            );

            Ok(format!("Flow '{}' saved successfully", flow.name))
        }
        Err(e) => {
            // Emit Tauri-specific error event
            let _ = app_handle.emit(
                "flow-save-error",
                serde_json::json!({
                    "flow_id": flow.id,
                    "error": e
                }),
            );

            Err(e)
        }
    }
}

#[tauri::command]
pub async fn load_flow(
    app_handle: AppHandle,
    db_pool: State<'_, DbPool>,
    flow_id: String,
) -> Result<vortideck_core::services::flow::executor::FlowDefinition, String> {
    let db = Database::new(&db_pool);

    match db.load_flow(&flow_id) {
        Ok(flow) => {
            // Emit Tauri-specific event
            let _ = app_handle.emit(
                "flow-loaded",
                serde_json::json!({
                    "flow_id": flow.id,
                    "name": flow.name
                }),
            );

            Ok(flow)
        }
        Err(e) => {
            // Emit Tauri-specific error event
            let _ = app_handle.emit(
                "flow-load-error",
                serde_json::json!({
                    "flow_id": flow_id,
                    "error": e
                }),
            );

            Err(e)
        }
    }
}

#[tauri::command]
pub async fn list_flows(
    app_handle: AppHandle,
    db_pool: State<'_, DbPool>,
) -> Result<Vec<vortideck_core::services::flow::executor::FlowSummary>, String> {
    let db = Database::new(&db_pool);

    match db.list_flows() {
        Ok(flows) => {
            // Emit Tauri-specific event
            let _ = app_handle.emit(
                "flows-listed",
                serde_json::json!({
                    "count": flows.len()
                }),
            );

            Ok(flows)
        }
        Err(e) => {
            // Emit Tauri-specific error event
            let _ = app_handle.emit(
                "flows-list-error",
                serde_json::json!({
                    "error": e
                }),
            );

            Err(e)
        }
    }
}

#[tauri::command]
pub async fn delete_flow(
    app_handle: AppHandle,
    db_pool: State<'_, DbPool>,
    flow_id: String,
) -> Result<String, String> {
    let db = Database::new(&db_pool);

    match db.delete_flow(&flow_id) {
        Ok(_) => {
            // Emit Tauri-specific event
            let _ = app_handle.emit(
                "flow-deleted",
                serde_json::json!({
                    "flow_id": flow_id
                }),
            );

            // Clean up context menu items referencing this flow
            if crate::commands::context_menu_commands::clear_orphaned_items("FlowId", &flow_id) {
                let _ = app_handle.emit("context-menu-updated", ());
            }

            Ok(format!("Flow '{}' deleted successfully", flow_id))
        }
        Err(e) => {
            // Emit Tauri-specific error event
            let _ = app_handle.emit(
                "flow-delete-error",
                serde_json::json!({
                    "flow_id": flow_id,
                    "error": e
                }),
            );

            Err(e)
        }
    }
}

// ✅ FLOW EXECUTION COMMANDS: Based on legacy implementation
#[tauri::command]
pub async fn init_flow_executor(
    app_handle: AppHandle,
    db_pool: State<'_, DbPool>,
    flow_registry_state: State<'_, FlowRegistryState>,
    chat_state: State<'_, ChatState>,
    executor_state: State<'_, FlowExecutorState>,
    platform_context: State<'_, std::sync::Arc<vortideck_core::core::PlatformContext>>,
    invocation_service: State<
        '_,
        Arc<vortideck_core::services::execution::ActionInvocationService>,
    >,
) -> Result<String, String> {
    // Check if executor is already initialized
    {
        let mut executor_guard = executor_state.executor.lock().await;
        if let Some(executor) = executor_guard.as_mut() {
            executor.set_action_invocation_service(invocation_service.inner().clone());
            // Emit initialization event for frontend consistency
            let _ = app_handle.emit(
                "flow-executor-initialized",
                serde_json::json!({
                    "status": "ready"
                }),
            );
            return Ok("Flow executor already initialized".to_string());
        }
    }

    // Use the existing initialized chat state
    let chat_state_arc = Arc::new(chat_state.inner().clone());

    // Use the existing registry that already has standard nodes registered
    let flow_registry_arc = Arc::new(flow_registry_state.inner().clone());

    // Use the platform context from state
    let platform_context_arc = platform_context.inner().clone();

    let mut executor = vortideck_core::services::flow::executor::FlowExecutor::new_universal(
        flow_registry_arc,
        chat_state_arc,
        platform_context_arc,
        db_pool.inner().clone(),
    )
    .map_err(|e| format!("Failed to create flow executor: {:?}", e))?;

    executor.set_action_invocation_service(invocation_service.inner().clone());

    let mut executor_guard = executor_state.executor.lock().await;
    *executor_guard = Some(executor);

    // Emit initialization event
    let _ = app_handle.emit(
        "flow-executor-initialized",
        serde_json::json!({
            "status": "ready"
        }),
    );

    Ok("Flow executor initialized".to_string())
}

#[tauri::command]
pub async fn start_flow_execution(
    app_handle: AppHandle,
    executor_state: State<'_, FlowExecutorState>,
    flow_definition: vortideck_core::services::flow::executor::FlowDefinition,
    inputs: Option<HashMap<String, Value>>,
) -> Result<String, String> {
    let executor_guard = executor_state.executor.lock().await;
    if let Some(executor) = executor_guard.as_ref() {
        // 📊 Track flow execution
        crate::analytics::capture(
            &app_handle,
            crate::analytics::AnalyticsEvent::new("flow_executed")
                .prop("flow_name", flow_definition.name.clone()),
        );

        match executor.start_flow_with_inputs(flow_definition, inputs.unwrap_or_default()).await {
            Ok(flow_id) => {
                // Emit execution start event
                let _ = app_handle.emit(
                    "flow-execution-started",
                    serde_json::json!({
                        "execution_id": flow_id,
                        "status": "running"
                    }),
                );

                Ok(flow_id)
            }
            Err(e) => Err(e),
        }
    } else {
        Err("Flow executor not initialized".to_string())
    }
}

#[tauri::command]
pub async fn get_flow_execution_status(
    app_handle: AppHandle,
    executor_state: State<'_, FlowExecutorState>,
    flow_id: String,
) -> Result<vortideck_core::services::flow::executor::FlowExecutionState, String> {
    let executor_guard = executor_state.executor.lock().await;
    if let Some(executor) = executor_guard.as_ref() {
        executor.get_flow_status(&flow_id).await
    } else {
        Err("Flow executor not initialized".to_string())
    }
}

#[tauri::command]
pub async fn cancel_flow_execution(
    app_handle: AppHandle,
    executor_state: State<'_, FlowExecutorState>,
    flow_id: String,
) -> Result<String, String> {
    let executor_guard = executor_state.executor.lock().await;
    if let Some(executor) = executor_guard.as_ref() {
        match executor.cancel_flow(&flow_id).await {
            Ok(_) => {
                // Emit Tauri-specific event
                let _ = app_handle.emit(
                    "flow-execution-cancelled",
                    serde_json::json!({
                        "flow_id": flow_id
                    }),
                );

                Ok(format!("Flow execution cancelled: {}", flow_id))
            }
            Err(e) => Err(e),
        }
    } else {
        Err("Flow executor not initialized".to_string())
    }
}

/// Lists runs stopped safely at a durable recovery checkpoint.
#[tauri::command]
pub async fn list_flow_recovery_runs(
    executor_state: State<'_, FlowExecutorState>,
) -> Result<Vec<vortideck_core::services::flow::executor::FlowExecutionState>, String> {
    let executor_guard = executor_state.executor.lock().await;
    let executor = executor_guard
        .as_ref()
        .ok_or("Flow executor not initialized")?;
    Ok(executor.list_recovery_required_runs().await)
}

/// Lists archived terminal runs. Recovery-required snapshots are intentionally
/// returned by `list_flow_recovery_runs` instead of being mixed into history.
#[tauri::command]
pub async fn list_flow_execution_history(
    executor_state: State<'_, FlowExecutorState>,
    flow_id: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<vortideck_core::services::flow::executor::ExecutionHistoryEntry>, String> {
    let executor_guard = executor_state.executor.lock().await;
    let executor = executor_guard
        .as_ref()
        .ok_or("Flow executor not initialized")?;
    executor
        .list_execution_history(flow_id.as_deref(), limit.unwrap_or(100))
        .await
}

#[tauri::command]
pub async fn list_workflow_wakes(
    executor_state: State<'_, FlowExecutorState>,
) -> Result<Vec<vortideck_core::services::flow::executor::WorkflowWake>, String> {
    let executor_guard = executor_state.executor.lock().await;
    let executor = executor_guard
        .as_ref()
        .ok_or("Flow executor not initialized")?;
    executor.list_open_workflow_wakes().await
}

#[tauri::command]
pub async fn resolve_workflow_approval(
    app_handle: AppHandle,
    executor_state: State<'_, FlowExecutorState>,
    wake_id: String,
    decision: vortideck_core::services::flow::executor::ApprovalDecision,
    comment: Option<String>,
) -> Result<vortideck_core::services::flow::executor::ApprovalResolveOutcome, String> {
    let executor_guard = executor_state.executor.lock().await;
    let executor = executor_guard
        .as_ref()
        .ok_or("Flow executor not initialized")?;
    let outcome = executor
        .resolve_approval_wake(
            &wake_id,
            vortideck_core::services::flow::executor::ApprovalDecisionRequest {
                decision: decision.clone(),
                actor: vortideck_core::services::flow::executor::ApprovalActor {
                    id: "local-desktop-operator".to_string(),
                    display_name: Some("Desktop operator".to_string()),
                    channel: "desktop".to_string(),
                },
                comment,
            },
        )
        .await?;
    let _ = app_handle.emit(
        "workflow-approval-resolved",
        serde_json::json!({ "wakeId": wake_id, "decision": decision }),
    );
    Ok(outcome)
}

/// Explicit fail-closed resolution for a recovered run. Resume/retry is not
/// exposed until node-specific compensation and idempotency policies exist.
#[tauri::command]
pub async fn fail_flow_recovery_run(
    app_handle: AppHandle,
    executor_state: State<'_, FlowExecutorState>,
    run_id: String,
    reason: Option<String>,
) -> Result<String, String> {
    let executor_guard = executor_state.executor.lock().await;
    let executor = executor_guard
        .as_ref()
        .ok_or("Flow executor not initialized")?;
    executor
        .fail_recovery_run(
            &run_id,
            reason.unwrap_or_else(|| "Recovery rejected by operator".to_string()),
        )
        .await?;
    let _ = app_handle.emit(
        "flow-recovery-resolved",
        serde_json::json!({ "runId": run_id, "resolution": "failed" }),
    );
    Ok("Recovery run marked failed".to_string())
}

#[tauri::command]
pub async fn resume_flow_recovery_run(
    app_handle: AppHandle,
    executor_state: State<'_, FlowExecutorState>,
    run_id: String,
) -> Result<String, String> {
    let executor_guard = executor_state.executor.lock().await;
    let executor = executor_guard
        .as_ref()
        .ok_or("Flow executor not initialized")?;
    executor.resume_recovery_run(&run_id).await?;
    let _ = app_handle.emit(
        "flow-recovery-resolved",
        serde_json::json!({ "runId": run_id, "resolution": "resumed" }),
    );
    Ok("Recovery run resumed from acknowledged overlay checkpoint".to_string())
}

#[tauri::command]
pub async fn inspect_workflow_runtime(
    executor_state: State<'_, FlowExecutorState>,
) -> Result<vortideck_core::events::WorkflowRuntimeInspection, String> {
    let executor_guard = executor_state.executor.lock().await;
    let executor = executor_guard
        .as_ref()
        .ok_or("Flow executor not initialized")?;
    executor.inspect_workflow_runtime().await
}

#[tauri::command]
pub async fn export_workflow_metrics(
    executor_state: State<'_, FlowExecutorState>,
) -> Result<String, String> {
    let executor_guard = executor_state.executor.lock().await;
    let executor = executor_guard
        .as_ref()
        .ok_or("Flow executor not initialized")?;
    Ok(executor
        .inspect_workflow_runtime()
        .await?
        .metrics
        .to_prometheus_text())
}

/// Redacted correlation view for operator diagnostics. Unlike the raw event
/// storage API, this never exports payloads, resolved actions or provider output.
#[tauri::command]
pub async fn trace_workflow_event(
    executor_state: State<'_, FlowExecutorState>,
    event_id: String,
) -> Result<Option<vortideck_core::events::WorkflowRuntimeTrace>, String> {
    let executor_guard = executor_state.executor.lock().await;
    let executor = executor_guard
        .as_ref()
        .ok_or("Flow executor not initialized")?;
    executor.trace_workflow_event(&event_id).await
}

/// Operator-only replay entry point. Replaying can repeat external actions, so
/// the caller must present an explicit confirmation rather than relying on a
/// defaulted argument or a passive inspection screen.
#[tauri::command]
pub async fn replay_workflow_dead_letter(
    executor_state: State<'_, FlowExecutorState>,
    dead_letter_id: String,
    confirm_side_effects: bool,
) -> Result<String, String> {
    if !confirm_side_effects {
        return Err(
            "Replay may execute external side effects; explicit confirmation is required"
                .to_string(),
        );
    }
    let executor_guard = executor_state.executor.lock().await;
    let executor = executor_guard
        .as_ref()
        .ok_or("Flow executor not initialized")?;
    executor.replay_workflow_dead_letter(&dead_letter_id).await
}

// Flow button assignment commands
#[tauri::command]
pub async fn assign_flow_to_button(
    app_handle: AppHandle,
    db_pool: State<'_, DbPool>,
    button_id: i32,
    flow_id: String,
) -> Result<String, String> {
    let db = Database::new(&db_pool);

    match db.assign_flow_to_button(button_id, &flow_id) {
        Ok(_) => {
            // Emit Tauri-specific event
            let _ = app_handle.emit(
                "flow-assigned-to-button",
                serde_json::json!({
                    "button_id": button_id,
                    "flow_id": flow_id
                }),
            );

            Ok(format!(
                "Flow '{}' assigned to button {}",
                flow_id, button_id
            ))
        }
        Err(e) => {
            // Emit Tauri-specific error event
            let _ = app_handle.emit(
                "flow-assignment-error",
                serde_json::json!({
                    "button_id": button_id,
                    "flow_id": flow_id,
                    "error": e
                }),
            );

            Err(e)
        }
    }
}

// 🚫 UNUSED COMMANDS REMOVED - Frontend doesn't use these
// convert_frontend_flow: Frontend creates FlowDefinition directly
// run_test_flow: No frontend usage found
// database maintenance commands: No frontend usage found
