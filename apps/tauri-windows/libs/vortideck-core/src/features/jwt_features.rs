//! JWT-based feature system for VortiDeck
//!
//! This module handles extracting feature limits and premium status from JWT tokens.

use crate::features::types::{PremiumFeature, ResetPeriod};
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtUser {
    pub id: u32,
    pub email: String,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    #[serde(rename = "type", default)]
    pub user_type: Option<String>,
    pub company_name: Option<String>,
    pub date_joined: String,
    pub avatar: Option<String>,
    pub premium: Option<JwtPremium>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtPremium {
    pub status: String,
    pub id: String,
    pub current_period_end: String,
    pub plan: String,
    pub plan_name: String,
    pub cancel_at_period_end: bool,
    pub is_lifetime: bool,
    pub features: HashMap<String, serde_json::Value>,
    pub price_amount: f64,
    pub billing_cycle: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    pub token_type: String,
    pub exp: u64,
    pub iat: u64,
    pub jti: String,
    pub user_id: u32,
    pub user: JwtUser,
}

/// Features whose limit comes from a store purchase. Everything else is unlimited.
const STORE_ENTITLEMENT_FEATURES: &[&str] = &["premium_themes"];

/// JWT-based feature provider that extracts limits from token
#[derive(Clone)]
pub struct JwtFeatureProvider {
    pub claims: Option<JwtClaims>,
}

impl JwtFeatureProvider {
    pub fn new() -> Self {
        Self { claims: None }
    }

    /// Update the JWT claims (called when user logs in or token refreshes)
    pub fn update_claims(&mut self, token: &str) -> Result<(), String> {
        if let Some(event) = self.apply_claims(token)? {
            emit_jwt_change_event(event);
        }
        Ok(())
    }

    fn apply_claims(&mut self, token: &str) -> Result<Option<JwtChangeEvent>, String> {
        // Store old state for comparison
        let old_plan = self.get_user_plan();
        let old_limits = self.get_all_feature_limits();
        let was_authenticated = self.claims.is_some();

        // For now, we'll decode the JWT manually (you might want to use a proper JWT library)
        // This is a simplified version that assumes the token is already validated
        let claims = self.decode_jwt_claims(token)?;
        self.claims = Some(claims);

        // Get new state after update
        let new_plan = self.get_user_plan();
        let new_limits = self.get_all_feature_limits();

        // Determine event type and emit if changed
        let event_type = if !was_authenticated {
            JwtChangeType::Login
        } else if old_plan != new_plan {
            JwtChangeType::PlanChange
        } else {
            JwtChangeType::TokenRefresh
        };

        // Emit change event if limits actually changed
        if old_limits != new_limits || !was_authenticated {
            let event = JwtChangeEvent {
                event_type,
                old_plan,
                new_plan: new_plan.clone(),
                old_limits,
                new_limits,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            };

            return Ok(Some(event));
        }

        Ok(None)
    }

    /// Get all current feature limits as a map
    fn get_all_feature_limits(&self) -> HashMap<String, u32> {
        let mut limits = HashMap::new();

        let features = [
            "plugin_count",
            "macro_recording",
            "ui_buttons",
            "saved_decks",
            "plugin_downloads",
            "premium_themes",
        ];

        for feature in &features {
            limits.insert(
                feature.to_string(),
                self.get_feature_limit(feature).unwrap_or(0),
            );
        }

        limits
    }

    /// Get the user's plan information
    pub fn get_user_plan(&self) -> Option<String> {
        if let Some(claims) = &self.claims {
            // User is logged in - get premium plan name or default to Free Plan
            claims
                .user
                .premium
                .as_ref()
                .map(|p| p.plan_name.clone())
                .or_else(|| Some("Free Plan".to_string()))
        } else {
            // 🎭 GUEST MODE: No JWT = Guest user, not Free Plan
            Some("Guest Mode".to_string())
        }
    }

    /// Check if user has premium status
    pub fn is_premium(&self) -> bool {
        let result = self
            .claims
            .as_ref()
            .and_then(|c| c.user.premium.as_ref())
            .map(|p| p.status == "active")
            .unwrap_or(false);

        println!(
            "🔍 JWT Premium Check: has_claims={}, is_premium={}",
            self.claims.is_some(),
            result
        );
        result
    }

    /// Get feature limit from JWT token (authenticated users only)
    pub fn get_feature_limit(&self, feature_id: &str) -> Option<u32> {
        self.claims
            .as_ref()
            .and_then(|c| c.user.premium.as_ref())
            .and_then(|p| p.features.get(feature_id))
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
    }

    /// Get effective feature limit (JWT for authenticated, guest_limits for guests AND free users)
    ///
    /// Local features are free and unlimited for everyone. Only content sold through the
    /// VortiDeck store keeps an entitlement limit; hosted services enforce their own access.
    pub fn get_effective_feature_limit(&self, feature_id: &str) -> u32 {
        if !STORE_ENTITLEMENT_FEATURES.contains(&feature_id) {
            return u32::MAX;
        }
        if let Some(_claims) = &self.claims {
            // 🔒 AUTHENTICATED USER: Try to get limit from JWT features
            // If not specified in JWT, fallback to guest_limits (free users get guest limits)
            if let Some(limit) = self.get_feature_limit(feature_id) {
                limit
            } else {
                // Free plan users (logged in but no premium features) → use guest_limits
                use crate::features::guest_limits::get_guest_feature_limit;
                get_guest_feature_limit(feature_id)
            }
        } else {
            // 🎭 GUEST MODE (not logged in): Use guest_limits.rs system
            use crate::features::guest_limits::get_guest_feature_limit;
            get_guest_feature_limit(feature_id)
        }
    }

    /// Get the current user's plugin limit (simplified - single limit)
    pub fn get_plugin_limit(&self) -> u32 {
        self.get_effective_feature_limit("plugin_count")
    }

    /// Generate PremiumFeature based on JWT token data (simplified)
    pub fn get_premium_feature(&self, feature_id: &str, default_free_limit: u32) -> PremiumFeature {
        // Get the current user's limit from JWT
        let user_limit = self.get_feature_limit(feature_id).unwrap_or_else(|| {
            if self.is_premium() {
                u32::MAX // Unlimited for premium
            } else {
                default_free_limit // Default free limit
            }
        });

        PremiumFeature::UsageLimit {
            feature_id: feature_id.to_string(),
            free_limit: default_free_limit, // Fallback only
            premium_limit: if user_limit == u32::MAX {
                None
            } else {
                Some(user_limit)
            },
            reset_period: ResetPeriod::Never,
        }
    }

    /// Print user plan information (for debugging/logging)
    pub fn print_user_plan(&self) {
        if let Some(claims) = &self.claims {
            if let Some(premium) = &claims.user.premium {
                println!("🎯 User Plan: {} ({})", premium.plan_name, premium.status);
                println!("💰 Price: ${}/month", premium.price_amount);
                println!("📅 Period End: {}", premium.current_period_end);
                println!("🔄 Auto-renew: {}", !premium.cancel_at_period_end);

                if !premium.features.is_empty() {
                    println!("✨ Features:");
                    for (key, value) in &premium.features {
                        println!("   - {}: {}", key, value);
                    }
                }
            } else {
                println!("🆓 User Plan: Free Plan");
                println!("💡 Upgrade to unlock premium features!");
            }
        } else {
            println!("❓ User Plan: Unknown (no token loaded)");
            println!("⚠️  Call update_jwt_token() first to load user data!");
        }
    }

    /// Simplified JWT decoder (you should use a proper JWT library in production)
    fn decode_jwt_claims(&self, token: &str) -> Result<JwtClaims, String> {
        println!("🔓 Decoding JWT token...");

        // Split the JWT token
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err("Invalid JWT format".to_string());
        }

        // Decode the payload (second part)
        let payload = parts[1];

        // Add padding if needed for base64 decoding
        let padded_payload = match payload.len() % 4 {
            0 => payload.to_string(),
            n => format!("{}{}", payload, "=".repeat(4 - n)),
        };

        // Decode base64 using the new API
        let decoded = general_purpose::STANDARD
            .decode(&padded_payload)
            .map_err(|e| format!("Base64 decode error: {}", e))?;

        // Parse JSON
        let claims: JwtClaims =
            serde_json::from_slice(&decoded).map_err(|e| format!("JSON parse error: {}", e))?;

        println!(
            "✅ JWT decoded successfully for user: {}",
            claims.user.email
        );
        Ok(claims)
    }
}

impl Default for JwtFeatureProvider {
    fn default() -> Self {
        Self::new()
    }
}

/// Authentication state is synchronized; readers receive an owned snapshot so
/// they cannot retain a global lock across an await or a callback.
static JWT_PROVIDER: LazyLock<RwLock<JwtFeatureProvider>> =
    LazyLock::new(|| RwLock::new(JwtFeatureProvider::new()));

pub fn get_jwt_provider() -> JwtFeatureProvider {
    JWT_PROVIDER
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

/// Update JWT token globally (call this on login/token refresh).
pub fn update_jwt_token(token: &str) -> Result<(), String> {
    let event = {
        let mut provider = JWT_PROVIDER.write().unwrap_or_else(|e| e.into_inner());
        let event = provider.apply_claims(token)?;
        provider.print_user_plan();
        event
    };
    // Subscribers may inspect authentication state or register other callbacks.
    // Never invoke application code while holding either registry/state lock.
    if let Some(event) = event {
        emit_jwt_change_event(event);
    }
    Ok(())
}

/// Clear JWT token (call this on logout).
pub fn clear_jwt_token() -> Result<(), String> {
    let event = {
        let mut provider = JWT_PROVIDER.write().unwrap_or_else(|e| e.into_inner());
        let old_plan = provider.get_user_plan();
        let old_limits = provider.get_all_feature_limits();
        let was_authenticated = provider.claims.take().is_some();
        was_authenticated.then(|| JwtChangeEvent {
            event_type: JwtChangeType::Logout,
            old_plan,
            new_plan: Some("Guest Mode".to_string()),
            old_limits,
            new_limits: provider.get_all_feature_limits(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        })
    };
    if let Some(mut event) = event {
        event.new_limits =
            crate::features::guest_limits::get_guest_limits_manager().get_all_guest_limits();
        emit_jwt_change_event(event);
    }
    Ok(())
}

/// Register a callback for JWT change events
pub fn register_jwt_change_callback<F>(callback: F)
where
    F: Fn(JwtChangeEvent) + Send + Sync + 'static,
{
    JWT_CHANGE_CALLBACKS
        .write()
        .unwrap_or_else(|e| e.into_inner())
        .push(Arc::new(callback));
}

/// Emit JWT change event to all registered callbacks
fn emit_jwt_change_event(event: JwtChangeEvent) {
    println!(
        "🔄 JWT Change Event: {:?} - {} -> {}",
        event.event_type,
        event.old_plan.as_deref().unwrap_or("None"),
        event.new_plan.as_deref().unwrap_or("None")
    );

    let callbacks = JWT_CHANGE_CALLBACKS
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    for callback in callbacks {
        callback(event.clone());
    }
}

/// Check if user is premium (global function)
pub fn is_user_premium() -> bool {
    get_jwt_provider().is_premium()
}

/// Get user's plan name (global function)
pub fn get_user_plan() -> String {
    get_jwt_provider()
        .get_user_plan()
        .unwrap_or_else(|| "Unknown".to_string())
}

/// 🐛 DEBUG: Get all feature limits from JWT for debugging
pub fn debug_get_all_feature_limits() -> Result<serde_json::Value, String> {
    let provider = get_jwt_provider();

    if let Some(claims) = &provider.claims {
        let mut debug_info = serde_json::json!({
            "user_id": claims.user_id,
            "email": claims.user.email,
            "username": claims.user.username,
            "user_type": claims.user.user_type,
            "is_premium": provider.is_premium(),
            "plan": provider.get_user_plan(),
            "jwt_present": true,
            "token_exp": claims.exp,
            "features": {}
        });

        // Add premium info if available
        if let Some(premium) = &claims.user.premium {
            debug_info["premium"] = serde_json::json!({
                "status": premium.status,
                "plan": premium.plan,
                "plan_name": premium.plan_name,
                "price_amount": premium.price_amount,
                "billing_cycle": premium.billing_cycle,
                "is_lifetime": premium.is_lifetime,
                "cancel_at_period_end": premium.cancel_at_period_end,
                "current_period_end": premium.current_period_end
            });

            // Extract all feature limits
            debug_info["features"] = serde_json::json!(premium.features);
        }

        // Add computed limits for known features
        debug_info["computed_limits"] = serde_json::json!({
            "plugin_count": provider.get_effective_feature_limit("plugin_count"),
            "macro_recording": provider.get_effective_feature_limit("macro_recording"),
            "saved_decks": provider.get_effective_feature_limit("saved_decks"),
            "ui_buttons": provider.get_effective_feature_limit("ui_buttons"),
            "plugin_downloads": provider.get_effective_feature_limit("plugin_downloads"),
            "premium_themes": provider.get_effective_feature_limit("premium_themes")
        });

        Ok(debug_info)
    } else {
        use crate::features::guest_limits::get_guest_limits_manager;
        let guest_limits = get_guest_limits_manager().get_all_guest_limits();

        // No JWT - return guest info
        Ok(serde_json::json!({
            "jwt_present": false,
            "is_premium": false,
            "plan": "Guest/Not Logged In",
            "error": "No JWT token loaded",
            "guest_limits": guest_limits
        }))
    }
}

/// 🐛 DEBUG: Check if JWT is expired
pub fn debug_is_jwt_expired() -> bool {
    let provider = get_jwt_provider();
    if let Some(claims) = &provider.claims {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now >= claims.exp
    } else {
        true // No JWT = considered expired
    }
}

/// 🐛 DEBUG: Validate JWT signature (placeholder - implement with proper JWT library)
pub fn debug_validate_jwt_signature(token: &str) -> Result<bool, String> {
    // TODO: Implement proper JWT signature validation
    // For now, just check format
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err("Invalid JWT format".to_string());
    }

    // ⚠️ WARNING: This is NOT secure - implement proper signature validation!
    Ok(true)
}

/// JWT change event for notifying when authentication status changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtChangeEvent {
    pub event_type: JwtChangeType,
    pub old_plan: Option<String>,
    pub new_plan: Option<String>,
    pub old_limits: HashMap<String, u32>,
    pub new_limits: HashMap<String, u32>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JwtChangeType {
    Login,        // User logged in (guest -> authenticated)
    Logout,       // User logged out (authenticated -> guest)
    PlanChange,   // User plan changed (free -> premium, etc.)
    TokenRefresh, // Token refreshed with same plan
}

/// JWT change event callback type
pub type JwtChangeCallback = Arc<dyn Fn(JwtChangeEvent) + Send + Sync>;

static JWT_CHANGE_CALLBACKS: RwLock<Vec<JwtChangeCallback>> = RwLock::new(Vec::new());

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn global_updates_publish_snapshots_and_invoke_subscribers_without_locks() {
        clear_jwt_token().unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let received = calls.clone();
        register_jwt_change_callback(move |_| {
            assert!(JWT_PROVIDER.try_write().is_ok());
            assert!(JWT_CHANGE_CALLBACKS.try_write().is_ok());
            let _snapshot = get_jwt_provider();
            register_jwt_change_callback(|_| {});
            received.fetch_add(1, Ordering::SeqCst);
        });
        let claims = serde_json::json!({
            "token_type": "access", "exp": 2000000000u64, "iat": 1,
            "jti": "edition-test", "user_id": 1,
            "user": {
                "id": 1, "email": "fixture@example.invalid", "username": "fixture",
                "first_name": "", "last_name": "", "date_joined": "",
                "company_name": null, "avatar": null, "premium": null
            }
        });
        let token = format!(
            "header.{}.signature",
            general_purpose::STANDARD.encode(claims.to_string())
        );
        update_jwt_token(&token).unwrap();
        let snapshot = get_jwt_provider();
        assert_eq!(snapshot.claims.as_ref().unwrap().user_id, 1);
        assert!(update_jwt_token("invalid").is_err());
        assert!(get_jwt_provider().claims.is_some());
        clear_jwt_token().unwrap();
        assert!(get_jwt_provider().claims.is_none());
        assert!(
            snapshot.claims.is_some(),
            "owned reads remain stable across logout"
        );
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }
}
