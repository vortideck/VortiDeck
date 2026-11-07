//! Macro Commands Adapter
//!
//! Bridges between Tauri commands and the new VortiDeck Core macro system

use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, WindowEvent};
use vortideck_core::core::state_manager::StateManager;
use vortideck_core::features::FeatureRequirement;
use vortideck_core::services::macros::{MacroEngine, MacroStep};

/// Convert from vortideck_core ActionExecutable to db utils ActionExecutable
fn convert_action_executable(
    action: &vortideck_core::services::macros::ActionExecutable,
) -> crate::db::ActionExecutable {
    match action {
        vortideck_core::services::macros::ActionExecutable::KeyDown(key) => {
            crate::db::ActionExecutable::KeyDown(key.clone())
        }
        vortideck_core::services::macros::ActionExecutable::KeyUp(key) => {
            crate::db::ActionExecutable::KeyUp(key.clone())
        }
        vortideck_core::services::macros::ActionExecutable::MouseMove { x, y } => {
            crate::db::ActionExecutable::MouseMove { x: *x, y: *y }
        }
        vortideck_core::services::macros::ActionExecutable::MouseClick(button) => {
            crate::db::ActionExecutable::MouseClick(button.clone())
        }
        vortideck_core::services::macros::ActionExecutable::ObsAction(action) => {
            crate::db::ActionExecutable::ObsAction(action.clone())
        }
        vortideck_core::services::macros::ActionExecutable::SystemControl(control) => {
            crate::db::ActionExecutable::SystemControl(control.clone())
        }
    }
}

/// Initialize macro recording system with Tauri focus integration
pub fn init_macro_recording(
    app_handle: &AppHandle,
    state_manager: Arc<StateManager>,
) -> Result<Arc<MacroEngine>, String> {
    println!("🎬 Initializing macro recording system with Tauri focus integration");

    let macro_engine = Arc::new(MacroEngine::new(state_manager));
    let macro_engine_clone = macro_engine.clone();

    // Get the main window
    let main_window = app_handle
        .get_webview_window("main")
        .ok_or("Failed to get main window")?;

    // Set up direct Tauri focus event handling
    main_window.on_window_event(move |event| {
        if let WindowEvent::Focused(is_focused) = event {
            macro_engine_clone.set_window_focused(*is_focused);
        }
    });

    println!("✅ Macro recording system initialized with Tauri focus integration");
    Ok(macro_engine)
}

/// Start real-time macro recording with event streaming
#[tauri::command]
pub async fn start_real_time_macro_recording(
    app_handle: AppHandle,
    state_manager: tauri::State<'_, Arc<StateManager>>,
    macro_engine: tauri::State<'_, Arc<MacroEngine>>,
) -> Result<String, String> {
    println!("🎬 Starting real-time macro recording (FREE & UNLIMITED)");

    // ✅ NO VALIDATION: Macro recording is FREE and UNLIMITED for all users

    // Create event callback that emits real-time events to frontend
    let app_handle_clone = app_handle.clone();
    let event_callback = move |step: MacroStep| {
        let emit_result = app_handle_clone.emit("macro-step-recorded", &step);
        if let Err(e) = emit_result {
            println!("❌ Failed to emit macro step: {}", e);
        }
    };

    // Start recording with real-time event streaming
    match macro_engine.start_real_time_recording(event_callback).await {
        Ok(steps) => {
            println!(
                "✅ Real-time macro recording started. Initial steps: {}",
                steps.len()
            );
            Ok("Recording started".to_string())
        }
        Err(e) => {
            println!("❌ Failed to start real-time recording: {}", e);
            Err(e)
        }
    }
}

/// Stop macro recording
#[tauri::command]
pub async fn stop_real_time_macro_recording(
    macro_engine: tauri::State<'_, Arc<MacroEngine>>,
) -> Result<Vec<MacroStep>, String> {
    println!("🛑 Stopping real-time macro recording");

    let recorded_steps = macro_engine.get_recorded_steps();

    match macro_engine.stop_real_time_recording().await {
        Ok(_) => {
            println!(
                "✅ Real-time macro recording stopped. Steps recorded: {}",
                recorded_steps.len()
            );
            Ok(recorded_steps)
        }
        Err(e) => {
            println!("❌ Failed to stop real-time recording: {}", e);
            Err(e)
        }
    }
}

/// Reset macro recording
#[tauri::command]
pub async fn reset_macro_recording(
    macro_engine: tauri::State<'_, Arc<MacroEngine>>,
) -> Result<(), String> {
    println!("🔄 Resetting macro recording");
    macro_engine.reset_recording().await
}

/// Get current recording state
#[tauri::command]
pub fn get_macro_recording_state(
    macro_engine: tauri::State<'_, Arc<MacroEngine>>,
) -> Result<serde_json::Value, String> {
    let state = serde_json::json!({
        "is_recording": macro_engine.is_recording(),
        "is_paused": macro_engine.is_paused(),
        "recorded_steps": macro_engine.get_recorded_steps()
    });

    Ok(state)
}

/// Save macro to database with proper storage limit validation
#[tauri::command]
pub async fn save_macro_to_db(
    app_handle: tauri::AppHandle,
    state_manager: tauri::State<'_, Arc<StateManager>>,
    macro_engine: tauri::State<'_, Arc<MacroEngine>>,
    db_state: tauri::State<'_, vortideck_core::db::db::DbPool>,
    macro_name: String,
    macro_type: Option<String>,
    macro_chain: Option<String>,
) -> Result<String, String> {
    println!("💾 Saving macro to database: {}", macro_name);

    // ✅ STORAGE VALIDATION: Use proper features service validation
    use crate::db::Database;
    let db = Database::new(&db_state);

    // Get current count of stored macros
    let current_count = match db.count_stored_macros().await {
        Ok(count) => count as u32,
        Err(e) => {
            println!("❌ Failed to count stored macros: {}", e);
            return Err("Failed to check storage limits".to_string());
        }
    };

    // Use the features service for storage validation
    let storage_validation = state_manager
        .validate_storage_feature("macro_recording", current_count, "macro save")
        .await;
    if !storage_validation.available {
        return Err(storage_validation
            .message
            .unwrap_or_else(|| "Macro storage limit reached!".to_string()));
    }

    // Get recorded steps from parameter or macro engine
    let steps: Vec<vortideck_core::services::macros::MacroStep> =
        if let Some(chain_str) = macro_chain {
            serde_json::from_str(&chain_str)
                .map_err(|e| format!("Failed to parse macro chain: {}", e))?
        } else {
            macro_engine.get_recorded_steps()
        };

    if steps.is_empty() {
        return Err("No macro steps recorded".to_string());
    }

    // Convert to legacy format for database compatibility
    let macro_chain: Vec<crate::db::SerializedMacroStep> = steps
        .iter()
        .map(|step| crate::db::SerializedMacroStep {
            action: convert_action_executable(&step.action),
            delay: step.delay,
        })
        .collect();

    let macro_command = crate::db::MacroCommand {
        name: macro_name.clone(),
        macro_chain,
        macro_type: macro_type.unwrap_or_else(|| "RunOnce".to_string()),
    };

    // Save using legacy database method
    match crate::db::save_macro_to_db(db_state, macro_command).await {
        Ok(_) => {
            println!(
                "✅ Macro '{}' saved successfully (storage used: {})",
                macro_name,
                current_count + 1
            );

            // Emit event to frontend that macro was created
            let _ = app_handle.emit("macro_created", ());

            // 📊 Track macro creation
            crate::analytics::capture(
                &app_handle,
                crate::analytics::AnalyticsEvent::new("macro_recorded")
                    .prop("macro_name", macro_name.clone()),
            );

            Ok(format!("Macro '{}' saved successfully", macro_name))
        }
        Err(e) => {
            println!("❌ Failed to save macro: {}", e);
            Err(e)
        }
    }
}

/// Execute macro by ID
#[tauri::command]
pub async fn execute_macro(
    app_handle: tauri::AppHandle,
    invocation_service: tauri::State<
        '_,
        Arc<vortideck_core::services::execution::ActionInvocationService>,
    >,
    macro_id: String,
) -> Result<(), String> {
    println!("▶️ Executing macro: {} (FREE & UNLIMITED)", macro_id);

    // ✅ NO VALIDATION: Macro execution is FREE and UNLIMITED for all users

    use vortideck_core::services::execution::{
        ActionReference, ActionReferenceKind, ButtonExecutionMode, ExecutionContext, RetryConfig,
        TriggerSource,
    };

    let reference = ActionReference {
        kind: ActionReferenceKind::Macro,
        id: macro_id.clone(),
    };
    let context = ExecutionContext {
        trigger_source: TriggerSource::Macro {
            macro_id: macro_id.clone(),
        },
        user_id: None,
        session_id: "macro-command".to_string(),
        variables: std::collections::HashMap::new(),
        timeout_ms: Some(30_000),
        retry_config: RetryConfig {
            max_retries: 0,
            retry_delay_ms: 0,
            exponential_backoff: false,
        },
        execution_mode: ButtonExecutionMode::RunOnce,
        execution_config: None,
    };

    match invocation_service
        .invoke_reference(&reference, None, None, &context)
        .await
    {
        Ok(result) if result.succeeded() => {
            println!("✅ Macro executed successfully: {}", macro_id);

            // 📊 Track macro execution
            crate::analytics::capture(
                &app_handle,
                crate::analytics::AnalyticsEvent::new("macro_executed")
                    .prop("macro_id", macro_id.clone()),
            );

            Ok(())
        }
        Ok(result) => {
            let message = result
                .error
                .map(|error| error.message)
                .unwrap_or_else(|| format!("Macro invocation ended as {:?}", result.status));
            println!("❌ Failed to execute macro: {}", message);
            Err(message)
        }
        Err(e) => {
            println!("❌ Failed to execute macro: {}", e);
            Err(e)
        }
    }
}

/// Get user's macros
#[tauri::command]
pub async fn get_user_macros(
    macro_engine: tauri::State<'_, Arc<MacroEngine>>,
    user_id: String,
) -> Result<Vec<vortideck_core::services::macros::RecordedMacro>, String> {
    let macros = macro_engine.list_user_macros(&user_id).await;

    match macros {
        Ok(macros) => Ok(macros),
        Err(e) => {
            println!("❌ Failed to get user macros: {}", e);
            Err(e)
        }
    }
}

/// Delete macro by ID
#[tauri::command]
pub async fn delete_macro(
    app_handle: AppHandle,
    macro_engine: tauri::State<'_, Arc<MacroEngine>>,
    macro_id: String,
) -> Result<(), String> {
    log::info!("Deleting macro: {}", macro_id);

    match macro_engine.delete_macro(&macro_id).await {
        Ok(_) => {
            log::info!("Macro deleted successfully: {}", macro_id);
            // Clean up context menu items referencing this macro
            if crate::commands::context_menu_commands::clear_orphaned_items("MacroId", &macro_id) {
                let _ = app_handle.emit("context-menu-updated", ());
            }
            Ok(())
        }
        Err(e) => {
            log::error!("Failed to delete macro: {}", e);
            Err(e)
        }
    }
}
