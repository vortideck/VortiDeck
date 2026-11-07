//! Feature validator module for VortiDeck
//!
//! This module handles feature validation with usage tracking and token-based premium validation.

use crate::features::requirements::FeatureRequirement;
use crate::features::types::StorageValidator;
use crate::features::types::{PremiumFeature, ResetPeriod};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureValidation {
    pub available: bool,
    pub message: Option<String>,
    pub reason: Option<String>,
    pub current_usage: Option<u32>,
    pub limit: Option<u32>,
    pub reset_time: Option<DateTime<Utc>>,
}

impl FeatureValidation {
    pub fn available() -> Self {
        Self {
            available: true,
            message: None,
            reason: None,
            current_usage: None,
            limit: None,
            reset_time: None,
        }
    }

    pub fn unavailable(message: String, reason: String) -> Self {
        Self {
            available: false,
            message: Some(message),
            reason: Some(reason),
            current_usage: None,
            limit: None,
            reset_time: None,
        }
    }

    pub fn with_usage_info(mut self, current: u32, limit: u32, reset_time: DateTime<Utc>) -> Self {
        self.current_usage = Some(current);
        self.limit = Some(limit);
        self.reset_time = Some(reset_time);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageCounter {
    pub feature_id: String,
    pub current_usage: u32,
    pub limit: u32,
    pub reset_period: ResetPeriod,
    pub last_reset: DateTime<Utc>,
    pub next_reset: DateTime<Utc>,
}

impl UsageCounter {
    pub fn new(feature_id: String, limit: u32, reset_period: ResetPeriod) -> Self {
        let now = Utc::now();
        let next_reset = Self::calculate_next_reset(now, &reset_period);

        Self {
            feature_id,
            current_usage: 0,
            limit,
            reset_period,
            last_reset: now,
            next_reset,
        }
    }

    pub fn increment(&mut self) -> bool {
        if self.should_reset() {
            self.reset();
        }

        if self.current_usage < self.limit {
            self.current_usage += 1;
            true
        } else {
            false
        }
    }

    pub fn can_use(&self) -> bool {
        if self.should_reset() {
            true
        } else {
            self.current_usage < self.limit
        }
    }

    pub fn should_reset(&self) -> bool {
        Utc::now() >= self.next_reset
    }

    pub fn reset(&mut self) {
        self.current_usage = 0;
        self.last_reset = Utc::now();
        self.next_reset = Self::calculate_next_reset(self.last_reset, &self.reset_period);
    }

    fn calculate_next_reset(from: DateTime<Utc>, period: &ResetPeriod) -> DateTime<Utc> {
        match period {
            ResetPeriod::Daily => from + Duration::days(1),
            ResetPeriod::Weekly => from + Duration::weeks(1),
            ResetPeriod::Monthly => from + Duration::weeks(4), // Approximate
            ResetPeriod::Yearly => from + Duration::days(365),
            ResetPeriod::Never => DateTime::<Utc>::MAX_UTC,
        }
    }
}

/// Cached premium validation result
#[derive(Debug, Clone)]
pub struct PremiumCacheEntry {
    pub is_premium: bool,
    pub cached_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl PremiumCacheEntry {
    pub fn new(is_premium: bool, cache_duration_seconds: u64) -> Self {
        let now = Utc::now();
        Self {
            is_premium,
            cached_at: now,
            expires_at: now + Duration::seconds(cache_duration_seconds as i64),
        }
    }

    pub fn is_valid(&self) -> bool {
        Utc::now() < self.expires_at
    }
}

/// Auth provider trait for premium validation
#[async_trait::async_trait]
pub trait AuthProvider: Send + Sync {
    /// Validate premium status using access token
    async fn validate_premium_status(&self, access_token: &str) -> Result<bool, String>;

    /// Refresh access token using refresh token
    async fn refresh_access_token(&self, refresh_token: &str) -> Result<String, String>;

    /// Get current access token
    async fn get_access_token(&self) -> Option<String>;

    /// Get current refresh token  
    async fn get_refresh_token(&self) -> Option<String>;

    /// Clear tokens (on logout/expiry)
    async fn clear_tokens(&self);
}

/// Universal Feature Validator with token-based premium validation
pub struct UniversalValidator {
    /// Usage counters for each feature
    usage_counters: Arc<RwLock<HashMap<String, UsageCounter>>>,
    /// Feature configurations
    feature_configs: Arc<RwLock<HashMap<String, PremiumFeature>>>,
    /// Auth provider for premium validation
    auth_provider: Arc<dyn AuthProvider>,
    /// Premium status cache (short-lived for performance)
    premium_cache: Arc<RwLock<Option<PremiumCacheEntry>>>,
    /// Cache duration in seconds (default: 60 seconds)
    cache_duration: u64,
}

impl UniversalValidator {
    /// Create a new universal validator with auth provider
    pub fn new(auth_provider: Arc<dyn AuthProvider>) -> Self {
        Self {
            usage_counters: Arc::new(RwLock::new(HashMap::new())),
            feature_configs: Arc::new(RwLock::new(HashMap::new())),
            auth_provider,
            premium_cache: Arc::new(RwLock::new(None)),
            cache_duration: 60, // 60 seconds cache
        }
    }

    /// Create validator with custom cache duration
    pub fn with_cache_duration(
        auth_provider: Arc<dyn AuthProvider>,
        cache_duration_seconds: u64,
    ) -> Self {
        Self {
            usage_counters: Arc::new(RwLock::new(HashMap::new())),
            feature_configs: Arc::new(RwLock::new(HashMap::new())),
            auth_provider,
            premium_cache: Arc::new(RwLock::new(None)),
            cache_duration: cache_duration_seconds,
        }
    }

    /// Get premium status directly from JWT token (no caching needed)
    pub async fn is_premium(&self) -> Result<bool, String> {
        // Get premium status directly from JWT token
        use crate::features::jwt_features::is_user_premium;
        Ok(is_user_premium())
    }

    /// Validate premium status with automatic token refresh
    async fn validate_premium_with_refresh(&self) -> Result<bool, String> {
        // Try with current access token
        if let Some(access_token) = self.auth_provider.get_access_token().await {
            match self
                .auth_provider
                .validate_premium_status(&access_token)
                .await
            {
                Ok(is_premium) => return Ok(is_premium),
                Err(_) => {
                    // Token might be expired, try to refresh
                    if let Some(refresh_token) = self.auth_provider.get_refresh_token().await {
                        match self
                            .auth_provider
                            .refresh_access_token(&refresh_token)
                            .await
                        {
                            Ok(new_access_token) => {
                                // Retry with new token
                                return self
                                    .auth_provider
                                    .validate_premium_status(&new_access_token)
                                    .await;
                            }
                            Err(_) => {
                                // Refresh failed, clear tokens
                                self.auth_provider.clear_tokens().await;
                                return Ok(false); // Not premium if auth failed
                            }
                        }
                    }
                }
            }
        }

        // No valid tokens
        Ok(false)
    }

    /// Clear premium cache (call when user logs out or subscription changes)
    pub async fn invalidate_premium_cache(&self) {
        let mut cache = self.premium_cache.write().await;
        *cache = None;
    }

    /// Register a feature configuration
    pub async fn register_feature(&self, feature: PremiumFeature) {
        let feature_id = match &feature {
            PremiumFeature::UsageLimit { feature_id, .. } => feature_id.clone(),
            PremiumFeature::FeatureFlag { feature_id, .. } => feature_id.clone(),
            PremiumFeature::TimeLimit { feature_id, .. } => feature_id.clone(),
            PremiumFeature::StorageLimit { feature_id, .. } => feature_id.clone(),
        };

        let mut configs = self.feature_configs.write().await;
        configs.insert(feature_id, feature);
    }

    /// Validate a feature requirement
    pub async fn validate_feature<T: FeatureRequirement>(&self, feature: &T) -> FeatureValidation {
        let requirements = feature.get_requirements();
        let context = feature.feature_context();

        for requirement in requirements {
            let validation = self.validate_requirement(&requirement, &context).await;
            if !validation.available {
                return validation;
            }
        }

        FeatureValidation::available()
    }

    /// Validate a single premium feature requirement
    pub async fn validate_requirement(
        &self,
        requirement: &PremiumFeature,
        context: &str,
    ) -> FeatureValidation {
        match requirement {
            PremiumFeature::UsageLimit {
                feature_id,
                free_limit,
                premium_limit,
                reset_period,
            } => {
                self.validate_usage_limit(
                    feature_id,
                    *free_limit,
                    *premium_limit,
                    reset_period,
                    context,
                )
                .await
            }
            PremiumFeature::FeatureFlag {
                feature_id,
                enabled,
            } => {
                self.validate_feature_flag(feature_id, *enabled, context)
                    .await
            }
            PremiumFeature::TimeLimit {
                feature_id,
                free_limit_seconds,
                premium_limit_seconds,
            } => {
                self.validate_time_limit(
                    feature_id,
                    *free_limit_seconds,
                    *premium_limit_seconds,
                    context,
                )
                .await
            }
            PremiumFeature::StorageLimit {
                feature_id,
                free_limit,
                premium_limit,
            } => {
                self.validate_storage_limit(feature_id, *free_limit, *premium_limit, context)
                    .await
            }
        }
    }

    /// Validate usage limit requirement with JWT-sourced limits and secure fallbacks
    async fn validate_usage_limit(
        &self,
        feature_id: &str,
        _free_limit: u32,            // ❌ Ignore hardcoded limit
        _premium_limit: Option<u32>, // ❌ Ignore hardcoded limit
        reset_period: &ResetPeriod,
        context: &str,
    ) -> FeatureValidation {
        use crate::features::jwt_features::{get_jwt_provider, is_user_premium};

        // ✅ Get premium status from JWT token
        let is_premium = is_user_premium();
        let jwt_provider = get_jwt_provider();

        // 🔒 SECURITY: Check if we have valid JWT claims
        let has_valid_jwt = jwt_provider.claims.is_some();

        // ✅ Get REAL limits from JWT token with SECURE fallbacks
        // Use get_effective_feature_limit which handles fallback to guest_limits for free users
        let effective_limit = jwt_provider.get_effective_feature_limit(feature_id);

        // Get or create usage counter with JWT-sourced limit
        let mut counters = self.usage_counters.write().await;
        let counter = counters.entry(feature_id.to_string()).or_insert_with(|| {
            UsageCounter::new(
                feature_id.to_string(),
                effective_limit,
                reset_period.clone(),
            )
        });

        // ✅ Always update limit from JWT (in case user upgraded)
        counter.limit = effective_limit;

        // Check if usage is allowed
        if counter.can_use() {
            FeatureValidation::available().with_usage_info(
                counter.current_usage,
                counter.limit,
                counter.next_reset,
            )
        } else {
            // ✅ Contextual upgrade messages based on feature and context
            let message = self.generate_contextual_upgrade_message(
                feature_id,
                context,
                is_premium,
                has_valid_jwt,
                counter.current_usage,
                effective_limit,
            );

            FeatureValidation::unavailable(message, "usage_limit_exceeded".to_string())
                .with_usage_info(counter.current_usage, counter.limit, counter.next_reset)
        }
    }

    /// Validate storage limit requirement
    async fn validate_storage_limit(
        &self,
        feature_id: &str,
        free_limit: u32,
        premium_limit: Option<u32>,
        context: &str,
    ) -> FeatureValidation {
        let is_premium = match self.is_premium().await {
            Ok(premium) => premium,
            Err(err) => {
                return FeatureValidation::unavailable(
                    format!(
                        "Authentication error for {} in {}: {}",
                        feature_id, context, err
                    ),
                    "auth_error".to_string(),
                );
            }
        };

        let effective_limit = if is_premium {
            premium_limit.unwrap_or(u32::MAX)
        } else {
            free_limit
        };

        // For storage limits, we'd need to track actual storage usage
        // This is a simplified implementation
        if effective_limit > 0 {
            FeatureValidation::available()
        } else {
            let message = if is_premium {
                format!("Storage limit exceeded for {} in {}.", feature_id, context)
            } else {
                format!("Free storage limit exceeded for {} in {}. Upgrade to premium for more storage.", feature_id, context)
            };

            FeatureValidation::unavailable(message, "storage_limit_exceeded".to_string())
        }
    }

    /// Validate storage-based feature with actual current count
    /// This should be called from service commands that have access to storage data
    pub async fn validate_storage_feature(
        &self,
        feature_id: &str,
        current_count: u32,
        context: &str,
    ) -> FeatureValidation {
        use crate::features::guest_limits::get_guest_feature_limit;
        use crate::features::jwt_features::{get_jwt_provider, is_user_premium};

        let is_premium = is_user_premium();
        let has_jwt = get_jwt_provider().claims.is_some();

        // Get storage limit from JWT/guest limits
        // Use get_effective_feature_limit which handles fallback to guest_limits for free users
        let jwt_provider = get_jwt_provider();
        let storage_limit = jwt_provider.get_effective_feature_limit(feature_id);

        println!(
            "📊 Storage validation for {}: {}/{} stored (Premium: {}, JWT: {})",
            feature_id, current_count, storage_limit, is_premium, has_jwt
        );

        // Check if storage limit is reached
        if current_count >= storage_limit {
            let message = if !has_jwt {
                format!(
                    "Storage limit reached ({}/{}). You can only store {} items!",
                    current_count, storage_limit, storage_limit
                )
            } else if is_premium {
                format!(
                    "Storage limit reached ({}/{}). Contact support to increase your limit.",
                    current_count, storage_limit
                )
            } else {
                format!(
                    "Storage limit reached ({}/{}). Upgrade to Premium for unlimited storage!",
                    current_count, storage_limit
                )
            };

            FeatureValidation::unavailable(message, "storage_limit_exceeded".to_string())
                .with_usage_info(
                    current_count,
                    storage_limit,
                    Utc::now() + chrono::Duration::days(365),
                )
        } else {
            FeatureValidation::available().with_usage_info(
                current_count,
                storage_limit,
                Utc::now() + chrono::Duration::days(365),
            )
        }
    }

    /// Generate contextual upgrade messages based on feature type, context, and auth status
    fn generate_contextual_upgrade_message(
        &self,
        feature_id: &str,
        context: &str,
        is_premium: bool,
        has_jwt: bool,
        current: u32,
        limit: u32,
    ) -> String {
        if !has_jwt {
            // 🔒 Not logged in - but some features are free for guests
            match feature_id {
                "macro_recording" => format!("Macro storage limit reached ({}/{}). Recording is free, but you can only store {} macros!", current, limit, limit),
                "plugin_count" => "Please log in to install plugins. Create a free account to get started!".to_string(),
                "saved_decks" => "Please log in to save decks. Create a free account to get started!".to_string(),
                "plugin_downloads" => "Please log in to download plugins. Create a free account to get started!".to_string(),
                "premium_themes" => "Please log in to access themes. Create a free account to get started!".to_string(),
                _ => format!("Please log in to use {}. Create a free account to get started!", feature_id.replace("_", " ")),
            }
        } else if is_premium {
            // Premium user hitting limits
            match feature_id {
                "plugin_count" => format!(
                    "Plugin limit reached ({}/{}). Contact support to increase your limit.",
                    current, limit
                ),
                _ => format!(
                    "Premium limit reached for {} ({}/{}). Contact support if you need more.",
                    feature_id.replace("_", " "),
                    current,
                    limit
                ),
            }
        } else {
            // Free user hitting limits
            match (feature_id, context) {
                ("plugin_count", _) => format!("Plugin limit reached ({}/{}). Upgrade to Premium for unlimited plugins!", current, limit),
                ("macro_recording", _) => format!("Macro storage limit reached ({}/{}). Recording is free, but storage is limited. Upgrade to Premium for unlimited macro storage!", current, limit),
                ("saved_decks", "Storage") => format!("Deck storage full ({}/{}). Upgrade to Premium to save unlimited decks!", current, limit),
                ("ui_buttons", "UI") => format!("Button limit reached ({}/{}). Upgrade to Premium for unlimited UI buttons!", current, limit),
                ("plugin_downloads", "Marketplace") => format!("Download limit reached ({}/{} this month). Upgrade to Premium for more downloads!", current, limit),
                ("premium_themes", "Themes") => format!("Theme limit reached ({}/{}). Upgrade to Premium for unlimited themes!", current, limit),
                _ => format!("Limit reached for {} ({}/{}). Upgrade to Premium for unlimited access!", feature_id.replace("_", " "), current, limit),
            }
        }
    }

    /// Validate feature flag requirement
    async fn validate_feature_flag(
        &self,
        feature_id: &str,
        enabled: bool,
        context: &str,
    ) -> FeatureValidation {
        if enabled {
            match self.is_premium().await {
                Ok(is_premium) => {
                    if is_premium {
                        FeatureValidation::available()
                    } else {
                        FeatureValidation::unavailable(
                            format!("Premium feature '{}' is not available in {}. Upgrade to premium to access this feature.", feature_id, context),
                            "premium_required".to_string()
                        )
                    }
                }
                Err(err) => FeatureValidation::unavailable(
                    format!(
                        "Authentication error for {} in {}: {}",
                        feature_id, context, err
                    ),
                    "auth_error".to_string(),
                ),
            }
        } else {
            FeatureValidation::available()
        }
    }

    /// Validate time limit requirement
    async fn validate_time_limit(
        &self,
        feature_id: &str,
        free_limit_seconds: u64,
        premium_limit_seconds: Option<u64>,
        context: &str,
    ) -> FeatureValidation {
        let is_premium = match self.is_premium().await {
            Ok(premium) => premium,
            Err(err) => {
                return FeatureValidation::unavailable(
                    format!(
                        "Authentication error for {} in {}: {}",
                        feature_id, context, err
                    ),
                    "auth_error".to_string(),
                );
            }
        };

        let effective_limit = if is_premium {
            premium_limit_seconds.unwrap_or(u64::MAX)
        } else {
            free_limit_seconds
        };

        // For time limits, we'd need to track usage over time
        // This is a simplified implementation
        if effective_limit > 0 {
            FeatureValidation::available()
        } else {
            let message = if is_premium {
                format!("Time limit exceeded for {} in {}.", feature_id, context)
            } else {
                format!(
                    "Free time limit exceeded for {} in {}. Upgrade to premium for more time.",
                    feature_id, context
                )
            };

            FeatureValidation::unavailable(message, "time_limit_exceeded".to_string())
        }
    }

    /// Try to use a feature (increments usage counter)
    pub async fn try_use_feature<T: FeatureRequirement>(&self, feature: &T) -> FeatureValidation {
        let validation = self.validate_feature(feature).await;

        if validation.available {
            // Increment usage counters for all requirements
            let requirements = feature.get_requirements();
            for requirement in requirements {
                self.increment_usage(&requirement).await;
            }
        }

        validation
    }

    /// Increment usage counter for a requirement
    async fn increment_usage(&self, requirement: &PremiumFeature) {
        if let PremiumFeature::UsageLimit { feature_id, .. } = requirement {
            let mut counters = self.usage_counters.write().await;
            if let Some(counter) = counters.get_mut(feature_id) {
                counter.increment();
            }
        }
    }

    /// Get usage statistics for a feature
    pub async fn get_usage_stats(&self, feature_id: &str) -> Option<UsageCounter> {
        let counters = self.usage_counters.read().await;
        counters.get(feature_id).cloned()
    }

    /// Reset usage counters (for testing or manual reset)
    pub async fn reset_usage_counter(&self, feature_id: &str) {
        let mut counters = self.usage_counters.write().await;
        if let Some(counter) = counters.get_mut(feature_id) {
            counter.reset();
        }
    }

    /// Get all usage statistics
    pub async fn get_all_usage_stats(&self) -> HashMap<String, UsageCounter> {
        self.usage_counters.read().await.clone()
    }

    /// 🐛 DEBUG: Get comprehensive validation status for all features
    pub async fn debug_get_validation_status(&self) -> serde_json::Value {
        use crate::features::jwt_features::{debug_get_all_feature_limits, debug_is_jwt_expired};

        // Get JWT info
        let jwt_info =
            debug_get_all_feature_limits().unwrap_or_else(|e| serde_json::json!({"error": e}));

        // Get all usage counters
        let usage_stats = self.get_all_usage_stats().await;
        let mut usage_json = serde_json::Map::new();
        for (feature_id, counter) in usage_stats {
            usage_json.insert(
                feature_id,
                serde_json::json!({
                    "current_usage": counter.current_usage,
                    "limit": counter.limit,
                    "reset_period": format!("{:?}", counter.reset_period),
                    "last_reset": counter.last_reset,
                    "next_reset": counter.next_reset,
                    "can_use": counter.can_use(),
                    "should_reset": counter.should_reset()
                }),
            );
        }

        // Combine all debug info
        serde_json::json!({
            "timestamp": chrono::Utc::now(),
            "jwt_expired": debug_is_jwt_expired(),
            "jwt_info": jwt_info,
            "usage_counters": usage_json,
            "validator_cache_duration": self.cache_duration,
            "security_status": {
                "has_jwt": jwt_info.get("jwt_present").unwrap_or(&serde_json::Value::Bool(false)),
                "is_premium": jwt_info.get("is_premium").unwrap_or(&serde_json::Value::Bool(false)),
                "guest_mode": !jwt_info.get("jwt_present").unwrap_or(&serde_json::Value::Bool(false)).as_bool().unwrap_or(false)
            }
        })
    }

    /// 🐛 DEBUG: Test feature validation for a specific feature ID
    pub async fn debug_test_feature_validation(
        &self,
        feature_id: &str,
        context: &str,
    ) -> serde_json::Value {
        use crate::features::types::{PremiumFeature, ResetPeriod};

        // Create a test feature requirement
        let test_feature = PremiumFeature::UsageLimit {
            feature_id: feature_id.to_string(),
            free_limit: 0,       // Ignored
            premium_limit: None, // Ignored
            reset_period: ResetPeriod::Never,
        };

        // Test validation
        let validation = self.validate_requirement(&test_feature, context).await;

        serde_json::json!({
            "feature_id": feature_id,
            "context": context,
            "validation": {
                "available": validation.available,
                "message": validation.message,
                "reason": validation.reason,
                "current_usage": validation.current_usage,
                "limit": validation.limit,
                "reset_time": validation.reset_time
            }
        })
    }
}

// Legacy validator removed - use UniversalValidator instead
