//! Guest Mode Limits Configuration
//!
//! This module manages guest mode limits that can be configured from the server
//! while providing secure fallbacks for offline scenarios.

use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;

/// Guest mode limits configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuestLimitsConfig {
    /// Feature limits for guest users (not logged in)
    pub feature_limits: HashMap<String, u32>,
    /// Last updated timestamp
    pub last_updated: u64,
    /// Version for cache invalidation
    pub version: String,
}

impl Default for GuestLimitsConfig {
    fn default() -> Self {
        let mut feature_limits = HashMap::new();

        // 🔒 SECURE FALLBACK LIMITS (if server config unavailable)
        feature_limits.insert("plugin_count".to_string(), 3);
        feature_limits.insert("macro_recording".to_string(), 10); // ✅ 10 stored macros limit
        feature_limits.insert("saved_decks".to_string(), 3); // ✅ 3 saved decks limit for free accounts
        feature_limits.insert("ui_buttons".to_string(), 8); // to test
        feature_limits.insert("plugin_downloads".to_string(), 0); //to remove
        feature_limits.insert("premium_themes".to_string(), 0); // to test
        feature_limits.insert("api_calls".to_string(), 5); //to remove
        feature_limits.insert("export_formats".to_string(), 1); // to test

        // 🎨 OVERLAY SYSTEM LIMITS
        feature_limits.insert("custom_overlay".to_string(), 0); // Premium-only for guests
        feature_limits.insert("overlay_elements".to_string(), 3); // 3 elements max per overlay
        feature_limits.insert("saved_overlays".to_string(), 1); // 1 saved overlay design

        Self {
            feature_limits,
            last_updated: 0,
            version: "fallback".to_string(),
        }
    }
}

/// Guest limits manager with server sync capabilities
pub struct GuestLimitsManager {
    config: RwLock<GuestLimitsConfig>,
}

impl GuestLimitsManager {
    pub fn new() -> Self {
        Self {
            config: RwLock::new(GuestLimitsConfig::default()),
        }
    }

    /// Update guest limits from server configuration
    pub async fn update_from_server(&self, server_config: GuestLimitsConfig) -> Result<(), String> {
        let mut config = self.config.write().unwrap();
        *config = server_config;
        log::info!(
            "✅ Updated guest limits from server (version: {})",
            config.version
        );
        Ok(())
    }

    /// Get limit for a specific feature (guest mode)
    pub fn get_guest_limit(&self, feature_id: &str) -> u32 {
        let config = self.config.read().unwrap();
        config.feature_limits.get(feature_id).copied().unwrap_or(1) // Very restrictive default
    }

    /// Get all guest limits (for debugging)
    pub fn get_all_guest_limits(&self) -> HashMap<String, u32> {
        let config = self.config.read().unwrap();
        config.feature_limits.clone()
    }

    /// Check if configuration is from server or fallback
    pub fn is_server_configured(&self) -> bool {
        let config = self.config.read().unwrap();
        config.version != "fallback"
    }
}

lazy_static! {
    static ref GUEST_LIMITS_MANAGER: GuestLimitsManager = GuestLimitsManager::new();
}

/// Get the global guest limits manager
pub fn get_guest_limits_manager() -> &'static GuestLimitsManager {
    &GUEST_LIMITS_MANAGER
}

/// Update guest limits from server (called by your backend sync)
pub async fn update_guest_limits_from_server(config: GuestLimitsConfig) -> Result<(), String> {
    get_guest_limits_manager().update_from_server(config).await
}

/// Get guest limit for a feature
pub fn get_guest_feature_limit(feature_id: &str) -> u32 {
    get_guest_limits_manager().get_guest_limit(feature_id)
}
