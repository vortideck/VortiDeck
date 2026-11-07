use crate::features::validator::UniversalValidator;
use crate::features::{FeatureRequirement, PremiumFeature, ResetPeriod};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Configuration for a button (matches database schema)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonConfig {
    pub id: String,
    pub label: String,
    pub icon: Option<String>,
    pub action: String,
    pub deck_id: Option<u32>,
    pub position: Option<u32>,
}

#[derive(Debug, thiserror::Error)]
pub enum ButtonError {
    #[error("Premium upgrade required: {0}")]
    UpgradeRequired(String),
    #[error("Button validation failed: {0}")]
    ValidationFailed(String),
    #[error("Database operation failed: {0}")]
    DatabaseError(String),
}

/// Premium validation layer for button operations
///
/// This manager validates premium features for button creation/usage.
/// It works alongside the existing database button system in db/utils.rs
///
/// Flow:
/// 1. Frontend calls ButtonManager to validate premium access
/// 2. If allowed, frontend calls database functions (db/utils.rs)
/// 3. ButtonManager tracks usage for premium limits
pub struct ButtonManager {
    max_free_buttons: u32,
    validator: Arc<UniversalValidator>,
}

impl ButtonManager {
    pub fn new(max_free_buttons: u32, validator: Arc<UniversalValidator>) -> Self {
        Self {
            max_free_buttons,
            validator,
        }
    }

    /// Check if user can add a new button (premium validation only)
    ///
    /// This should be called BEFORE creating a button in the database.
    /// If this returns true, proceed with database operations in db/utils.rs
    pub async fn can_add_button(&self) -> Result<bool, ButtonError> {
        let validation = self.validator.validate_feature(self).await;
        Ok(validation.available)
    }

    /// Validate and track button creation (call this AFTER database creation)
    ///
    /// This increments the usage counter for premium tracking.
    /// Call this after successfully creating a button in the database.
    pub async fn track_button_creation(&self) -> Result<(), ButtonError> {
        let validation = self.validator.try_use_feature(self).await;

        if !validation.available {
            return Err(ButtonError::UpgradeRequired(
                validation.message.unwrap_or_else(|| {
                    format!(
                        "Get unlimited buttons with Premium! Free users are limited to {} buttons.",
                        self.max_free_buttons
                    )
                }),
            ));
        }

        Ok(())
    }

    /// Get current button usage statistics
    pub async fn get_usage_stats(&self) -> Option<crate::features::validator::UsageCounter> {
        self.validator.get_usage_stats("ui_buttons").await
    }

    /// Check if user can access all their buttons or if some should be hidden
    ///
    /// This can be used to filter buttons in the UI based on premium status.
    /// For example, free users might only see their first 6 buttons.
    pub async fn get_accessible_button_count(&self) -> u32 {
        let validation = self.validator.validate_feature(self).await;

        if validation.available {
            // Premium user or within limits - can access all buttons
            u32::MAX
        } else {
            // Free user - limited access
            self.max_free_buttons
        }
    }

    /// Get upgrade message for UI display
    pub async fn get_upgrade_message(&self) -> Option<String> {
        let validation = self.validator.validate_feature(self).await;

        if !validation.available {
            validation.message.or_else(|| Some(
                format!("Upgrade to Premium for unlimited buttons! Free users are limited to {} buttons.", self.max_free_buttons)
            ))
        } else {
            None
        }
    }
}

impl FeatureRequirement for ButtonManager {
    fn get_requirements(&self) -> Vec<PremiumFeature> {
        vec![PremiumFeature::UsageLimit {
            feature_id: "ui_buttons".to_string(),
            free_limit: self.max_free_buttons,
            premium_limit: None, // Unlimited for premium
            reset_period: ResetPeriod::Never,
        }]
    }

    fn feature_context(&self) -> String {
        "UI".to_string()
    }
}

// Example integration with existing database system:
//
// ```rust
// // In your Tauri command or frontend handler:
//
// #[tauri::command]
// async fn create_button(button_config: ButtonConfig, button_manager: State<ButtonManager>, db_pool: State<DbPool>) -> Result<(), String> {
//     // 1. Check premium validation
//     if !button_manager.can_add_button().await.map_err(|e| e.to_string())? {
//         return Err("Premium upgrade required".to_string());
//     }
//
//     // 2. Create button in database (existing system)
//     let db = Database::new(&db_pool);
//     db.create_button(button_config).await.map_err(|e| e.to_string())?;
//
//     // 3. Track usage for premium limits
//     button_manager.track_button_creation().await.map_err(|e| e.to_string())?;
//
//     Ok(())
// }
//
// #[tauri::command]
// async fn get_user_buttons(button_manager: State<ButtonManager>, db_pool: State<DbPool>) -> Result<Vec<Button>, String> {
//     // 1. Get all buttons from database (existing system)
//     let db = Database::new(&db_pool);
//     let all_buttons = db.get_user_buttons().await.map_err(|e| e.to_string())?;
//
//     // 2. Filter based on premium limits
//     let accessible_count = button_manager.get_accessible_button_count().await;
//     let filtered_buttons = all_buttons.into_iter()
//         .take(accessible_count as usize)
//         .collect();
//
//     Ok(filtered_buttons)
// }
// ```
