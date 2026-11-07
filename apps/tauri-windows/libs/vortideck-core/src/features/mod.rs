//! Features module for VortiDeck
//!
//! This module implements the Universal Feature Architecture for VortiDeck.

pub mod auth_provider;
pub mod guest_limits;
pub mod jwt_features;
pub mod limits;
pub mod registry;
pub mod requirements;
pub mod types;
pub mod validator;

// Export specific items from limits
pub use limits::*;

// Export specific items from requirements
pub use requirements::{AsyncFeatureRequirement, BasicFeatureRequirement, FeatureRequirement};

// Export specific items from registry
pub use registry::FeatureRegistry;

// Export specific items from validator (ENHANCED)
pub use validator::{FeatureValidation, UniversalValidator, UsageCounter};

// Export specific items from types
pub use types::{PremiumFeature, ResetPeriod};

// Re-export main types for convenience
pub use auth_provider::*;
pub use jwt_features::*;
pub use requirements::*;
pub use types::*;
pub use validator::*;

use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

/// Feature identifier type
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FeatureId(pub String);

impl fmt::Display for FeatureId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for FeatureId {
    fn from(s: String) -> Self {
        FeatureId(s)
    }
}

impl From<&str> for FeatureId {
    fn from(s: &str) -> Self {
        FeatureId(s.to_string())
    }
}

/// Feature validation error
#[derive(Debug, Error)]
pub enum FeatureError {
    #[error("Feature not found: {0}")]
    NotFound(FeatureId),

    #[error("Feature validation failed: {0}")]
    ValidationFailed(String),

    #[error("Feature limit exceeded: {0}")]
    LimitExceeded(FeatureId),

    #[error("Premium subscription required for feature: {0}")]
    PremiumRequired(FeatureId),

    #[error("Usage limit exceeded for feature: {0}")]
    UsageLimitExceeded(FeatureId),
}

/// Result type for feature operations
pub type FeatureResult<T> = Result<T, FeatureError>;

/// Universal feature builder for easy feature creation
pub struct FeatureBuilder {
    id: FeatureId,
    requirements: Vec<PremiumFeature>,
    context: String,
}

impl FeatureBuilder {
    /// Create a new feature builder
    pub fn new<T: Into<FeatureId>>(id: T) -> Self {
        Self {
            id: id.into(),
            requirements: Vec::new(),
            context: "unknown".to_string(),
        }
    }

    /// Add a usage limit requirement
    pub fn with_usage_limit(
        mut self,
        free_limit: u32,
        premium_limit: Option<u32>,
        reset_period: ResetPeriod,
    ) -> Self {
        self.requirements.push(PremiumFeature::UsageLimit {
            feature_id: self.id.0.clone(),
            free_limit,
            premium_limit,
            reset_period,
        });
        self
    }

    /// Add a storage limit requirement (for features that limit stored items)
    pub fn with_storage_limit(mut self, free_limit: u32, premium_limit: Option<u32>) -> Self {
        self.requirements.push(PremiumFeature::StorageLimit {
            feature_id: self.id.0.clone(),
            free_limit,
            premium_limit,
        });
        self
    }

    /// Add a premium-only feature flag
    pub fn premium_only(mut self) -> Self {
        self.requirements.push(PremiumFeature::FeatureFlag {
            feature_id: self.id.0.clone(),
            enabled: true,
        });
        self
    }

    /// Add a time limit requirement
    pub fn with_time_limit(
        mut self,
        free_limit_seconds: u64,
        premium_limit_seconds: Option<u64>,
    ) -> Self {
        self.requirements.push(PremiumFeature::TimeLimit {
            feature_id: self.id.0.clone(),
            free_limit_seconds,
            premium_limit_seconds,
        });
        self
    }

    /// Set the context for this feature
    pub fn with_context<T: Into<String>>(mut self, context: T) -> Self {
        self.context = context.into();
        self
    }

    /// Build the feature
    pub fn build(self) -> Feature {
        Feature {
            id: self.id,
            requirement: BasicFeatureRequirement::new(self.requirements, self.context),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Feature {
    pub id: FeatureId,
    pub requirement: BasicFeatureRequirement,
}

impl Feature {
    /// Create a new feature with builder pattern
    pub fn builder<T: Into<FeatureId>>(id: T) -> FeatureBuilder {
        FeatureBuilder::new(id)
    }

    pub fn new(id: FeatureId, requirements: Vec<PremiumFeature>, context: String) -> Self {
        Self {
            id,
            requirement: BasicFeatureRequirement::new(requirements, context),
        }
    }

    pub fn get_requirements(&self) -> Vec<PremiumFeature> {
        self.requirement.get_requirements()
    }

    pub fn feature_context(&self) -> String {
        self.requirement.feature_context()
    }
}

impl FeatureRequirement for Feature {
    fn get_requirements(&self) -> Vec<PremiumFeature> {
        self.requirement.get_requirements()
    }

    fn feature_context(&self) -> String {
        self.requirement.feature_context()
    }
}

/// Pre-defined common features for VortiDeck
pub mod common_features {
    use super::*;

    /// UI Button usage limit (6 free, unlimited premium)
    pub fn ui_buttons() -> Feature {
        Feature::builder("ui_buttons")
            .with_usage_limit(6, None, ResetPeriod::Never)
            .with_context("UI customization")
            .build()
    }

    /// Macro recording (free recording, limited storage - 10 free, unlimited premium)
    pub fn macro_recording() -> Feature {
        Feature::builder("macro_recording")
            .with_storage_limit(10, None)
            .with_context("macro system")
            .build()
    }

    /// Advanced integrations (premium only)
    pub fn advanced_integrations() -> Feature {
        Feature::builder("advanced_integrations")
            .premium_only()
            .with_context("integration manager")
            .build()
    }

    /// Plugin uploads (5 free per month, 50 premium)
    pub fn plugin_uploads() -> Feature {
        Feature::builder("plugin_uploads")
            .with_usage_limit(5, Some(50), ResetPeriod::Monthly)
            .with_context("plugin marketplace")
            .build()
    }

    /// Cloud sync (premium only)
    pub fn cloud_sync() -> Feature {
        Feature::builder("cloud_sync")
            .premium_only()
            .with_context("settings synchronization")
            .build()
    }

    /// Screen recording time (30 mins free per day, unlimited premium)
    pub fn screen_recording() -> Feature {
        Feature::builder("screen_recording")
            .with_time_limit(30 * 60, None) // 30 minutes in seconds
            .with_context("recording system")
            .build()
    }
}
