//! Command adapters for VortiDeck Core
//!
//! These adapters bridge between Tauri commands and the headless VortiDeck Core.

use tauri::{command, AppHandle, Runtime, State};
use vortideck_core::VortiDeckCore;
// Temporary type stub during migration
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct MacroStep;

/// Record a macro using the headless core
#[command]
pub async fn record_macro_new(core: State<'_, VortiDeckCore>) -> Result<Vec<MacroStep>, String> {
    // For now, return empty vector as macro engine is not yet implemented in new architecture
    // TODO: Implement macro recording in the new StateManager
    println!("🎬 Macro recording requested via new architecture");
    Ok(Vec::new())
}

/// Exchange auth code using the headless core
#[command]
pub async fn exchange_auth_code_new(
    core: State<'_, VortiDeckCore>,
    code: String,
    login_id: String,
) -> Result<(), String> {
    // For now, return success as auth engine is not yet implemented in new architecture
    // TODO: Implement auth exchange in the new StateManager
    println!(
        "🔐 Auth code exchange requested via new architecture: code={}, login_id={}",
        code, login_id
    );
    Ok(())
}

/// Get flow registry using the headless core
#[command]
pub async fn get_flow_registry_new(core: State<'_, VortiDeckCore>) -> Result<String, String> {
    // For now, return empty registry as flow system is not yet implemented in new architecture
    // TODO: Implement flow registry in the new StateManager
    println!("🌊 Flow registry requested via new architecture");
    Ok("{}".to_string())
}

pub fn register_commands<R: Runtime>(
    app_handle: &AppHandle<R>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 Registering VortiDeck Core commands");
    // Commands will be registered when we implement the full adapter system
    Ok(())
}

async fn execute_flow<R: Runtime>(
    flow_id: String,
    core: tauri::State<'_, VortiDeckCore>,
) -> Result<String, String> {
    println!("🌊 Executing flow {} via new architecture", flow_id);
    Ok("Flow executed".to_string())
}
