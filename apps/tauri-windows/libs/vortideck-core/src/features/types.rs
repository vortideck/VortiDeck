use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResetPeriod {
    Daily,
    Weekly,
    Monthly,
    Yearly,
    Never,
}

/// Trait for services that can provide storage-based validation
pub trait StorageValidator: Send + Sync {
    /// Get current storage count for the feature
    fn get_current_storage_count(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<u32, String>> + Send + '_>>;

    /// Get the feature ID this validator handles
    fn feature_id(&self) -> &str;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PremiumFeature {
    UsageLimit {
        feature_id: String,
        free_limit: u32,
        premium_limit: Option<u32>,
        reset_period: ResetPeriod,
    },
    StorageLimit {
        feature_id: String,
        free_limit: u32,
        premium_limit: Option<u32>,
    },
    FeatureFlag {
        feature_id: String,
        enabled: bool,
    },
    TimeLimit {
        feature_id: String,
        free_limit_seconds: u64,
        premium_limit_seconds: Option<u64>,
    },
}
