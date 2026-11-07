//! Migration commands for frontend control

use crate::migration::{get_migration_manager, MigrationConfig, ValidationResult};
use std::collections::HashMap;
use tauri::State;

/// Get current migration configuration
#[tauri::command]
pub async fn get_migration_config() -> Result<MigrationConfig, String> {
    Ok(get_migration_manager().get_config())
}

/// Update migration configuration
#[tauri::command]
pub async fn update_migration_config(config: MigrationConfig) -> Result<(), String> {
    get_migration_manager().update_config(config);
    Ok(())
}

/// Enable a specific feature for migration
#[tauri::command]
pub async fn enable_migration_feature(feature: String) -> Result<(), String> {
    get_migration_manager().enable_feature(&feature);
    Ok(())
}

/// Disable a specific feature (rollback)
#[tauri::command]
pub async fn disable_migration_feature(feature: String) -> Result<(), String> {
    get_migration_manager().disable_feature(&feature);
    Ok(())
}

/// Get validation results for monitoring
#[tauri::command]
pub async fn get_validation_results() -> Result<HashMap<String, ValidationResult>, String> {
    Ok(get_migration_manager().get_validation_results())
}

/// Check if a feature is using new core
#[tauri::command]
pub async fn is_feature_migrated(feature: String) -> Result<bool, String> {
    Ok(get_migration_manager().should_use_new_core(&feature))
}

/// Get migration status overview
#[tauri::command]
pub async fn get_migration_status() -> Result<MigrationStatus, String> {
    let config = get_migration_manager().get_config();
    let validation_results = get_migration_manager().get_validation_results();
    
    let total_features = config.features.len();
    let migrated_features = config.features.values().filter(|&&v| v).count();
    let failed_validations = validation_results
        .values()
        .filter(|result| !result.matches)
        .count();
    
    Ok(MigrationStatus {
        total_features,
        migrated_features,
        migration_percentage: (migrated_features as f64 / total_features as f64 * 100.0),
        failed_validations,
        rollback_enabled: config.rollback_enabled,
        validation_enabled: config.validation_enabled,
        features: config.features,
    })
}

#[derive(serde::Serialize)]
pub struct MigrationStatus {
    pub total_features: usize,
    pub migrated_features: usize,
    pub migration_percentage: f64,
    pub failed_validations: usize,
    pub rollback_enabled: bool,
    pub validation_enabled: bool,
    pub features: HashMap<String, bool>,
} 