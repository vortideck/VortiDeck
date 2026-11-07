use std::sync::Arc;
/// Workspace Commands
///
/// Tauri commands for managing the VortiDeck workspace in Documents/VortiDeck/workspace/
use tauri::command;
use tokio::sync::RwLock;
use vortideck_core::services::WorkspaceConfig;

/// Global workspace state
pub struct WorkspaceState {
    pub config: Arc<RwLock<WorkspaceConfig>>,
}

impl WorkspaceState {
    pub fn new(config: WorkspaceConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
        }
    }
}

/// Get the workspace root directory path
#[command]
pub async fn get_workspace_root(
    workspace: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    let config = workspace.config.read().await;
    Ok(config.get_root().to_string_lossy().to_string())
}

/// Get the output directory path
#[command]
pub async fn get_workspace_output_dir(
    workspace: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    let config = workspace.config.read().await;
    Ok(config.get_output_dir().to_string_lossy().to_string())
}

/// Get the input directory path
#[command]
pub async fn get_workspace_input_dir(
    workspace: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    let config = workspace.config.read().await;
    Ok(config.get_input_dir().to_string_lossy().to_string())
}

/// Get the temp directory path
#[command]
pub async fn get_workspace_temp_dir(
    workspace: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    let config = workspace.config.read().await;
    Ok(config.get_temp_dir().to_string_lossy().to_string())
}

/// Open the workspace directory in the file explorer
#[command]
pub async fn open_workspace_directory(
    workspace: tauri::State<'_, WorkspaceState>,
) -> Result<(), String> {
    let config = workspace.config.read().await;
    let workspace_root = config.get_root();

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(workspace_root)
            .spawn()
            .map_err(|e| format!("Failed to open workspace directory: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(workspace_root)
            .spawn()
            .map_err(|e| format!("Failed to open workspace directory: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(workspace_root)
            .spawn()
            .map_err(|e| format!("Failed to open workspace directory: {}", e))?;
    }

    Ok(())
}
