//! Auth Module - Pure Business Logic
//!
//! This module contains pure authentication and authorization logic without state management.
//! Services use this module for JWT validation, permission checking, and user data extraction.

pub mod oauth;
pub mod token_storage;

// Re-export common types and functions
pub use oauth::{OAuthError, OAuthResult, OAuthService};
pub use token_storage::{TokenStorage, TokenStorageError};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// User information extracted from JWT claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub user_type: Option<String>,
    pub company_name: Option<String>,
    pub date_joined: String,
    pub avatar: Option<String>,
    pub permissions: Vec<Permission>,
    pub plan: UserPlan,
}

/// User plan information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserPlan {
    Guest,
    Free,
    Premium {
        plan_name: String,
        status: String,
        current_period_end: String,
        cancel_at_period_end: bool,
        is_lifetime: bool,
        features: HashMap<String, serde_json::Value>,
        price_amount: f64,
        billing_cycle: String,
    },
}

/// User permission
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Permission {
    pub resource: String,
    pub action: String,
    pub scope: Option<String>,
}

/// JWT claims structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    pub token_type: String,
    pub exp: u64,
    pub iat: u64,
    pub jti: String,
    pub user_id: u32,
    pub user: JwtUser,
}

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

/// Authentication errors
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Invalid JWT format")]
    InvalidJwtFormat,
    #[error("JWT token expired")]
    TokenExpired,
    #[error("Invalid JWT signature")]
    InvalidSignature,
    #[error("Base64 decode error: {0}")]
    Base64DecodeError(String),
    #[error("JSON parse error: {0}")]
    JsonParseError(String),
    #[error("Missing required field: {0}")]
    MissingField(String),
    #[error("Permission denied")]
    PermissionDenied,
}

/// Auth Module - Pure business logic functions
pub struct AuthModule;

impl AuthModule {
    /// Validate JWT token and extract claims (pure function)
    pub fn validate_jwt_token(token: &str) -> Result<JwtClaims, AuthError> {
        // Split the JWT token
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err(AuthError::InvalidJwtFormat);
        }

        // Decode the payload (second part)
        let payload = parts[1];

        // Add padding if needed for base64 decoding
        let padded_payload = match payload.len() % 4 {
            0 => payload.to_string(),
            n => format!("{}{}", payload, "=".repeat(4 - n)),
        };

        // Decode base64
        use base64::{engine::general_purpose, Engine as _};
        let decoded = general_purpose::STANDARD
            .decode(&padded_payload)
            .map_err(|e| AuthError::Base64DecodeError(e.to_string()))?;

        // Parse JSON
        let claims: JwtClaims = serde_json::from_slice(&decoded)
            .map_err(|e| AuthError::JsonParseError(e.to_string()))?;

        // Check expiration
        let now = Utc::now().timestamp() as u64;
        if claims.exp < now {
            return Err(AuthError::TokenExpired);
        }

        Ok(claims)
    }

    /// Extract user information from JWT claims (pure function)
    pub fn extract_user_from_claims(claims: &JwtClaims) -> User {
        let permissions = Self::extract_user_permissions(claims);
        let plan = Self::extract_user_plan(claims);

        User {
            id: claims.user.id.to_string(),
            email: claims.user.email.clone(),
            username: claims.user.username.clone(),
            first_name: claims.user.first_name.clone(),
            last_name: claims.user.last_name.clone(),
            user_type: claims.user.user_type.clone(),
            company_name: claims.user.company_name.clone(),
            date_joined: claims.user.date_joined.clone(),
            avatar: claims.user.avatar.clone(),
            permissions,
            plan,
        }
    }

    /// Extract user permissions from JWT claims (pure function)
    pub fn extract_user_permissions(claims: &JwtClaims) -> Vec<Permission> {
        let mut permissions = Vec::new();

        // Basic permissions for all authenticated users
        permissions.push(Permission {
            resource: "plugins".to_string(),
            action: "install".to_string(),
            scope: None,
        });
        permissions.push(Permission {
            resource: "macros".to_string(),
            action: "record".to_string(),
            scope: None,
        });

        // Premium permissions
        if let Some(premium) = &claims.user.premium {
            if premium.status == "active" {
                permissions.push(Permission {
                    resource: "themes".to_string(),
                    action: "premium_access".to_string(),
                    scope: None,
                });
                permissions.push(Permission {
                    resource: "plugins".to_string(),
                    action: "unlimited".to_string(),
                    scope: None,
                });
                permissions.push(Permission {
                    resource: "decks".to_string(),
                    action: "unlimited_save".to_string(),
                    scope: None,
                });
            }
        }

        permissions
    }

    /// Extract user plan from JWT claims (pure function)
    pub fn extract_user_plan(claims: &JwtClaims) -> UserPlan {
        if let Some(premium) = &claims.user.premium {
            UserPlan::Premium {
                plan_name: premium.plan_name.clone(),
                status: premium.status.clone(),
                current_period_end: premium.current_period_end.clone(),
                cancel_at_period_end: premium.cancel_at_period_end,
                is_lifetime: premium.is_lifetime,
                features: premium.features.clone(),
                price_amount: premium.price_amount,
                billing_cycle: premium.billing_cycle.clone(),
            }
        } else {
            UserPlan::Free
        }
    }

    /// Check if user has specific permission (pure function)
    pub fn has_permission(permissions: &[Permission], required: &Permission) -> bool {
        permissions.iter().any(|p| {
            p.resource == required.resource
                && p.action == required.action
                && (p.scope.is_none() || p.scope == required.scope)
        })
    }

    /// Check if user is premium (pure function)
    pub fn is_premium_user(plan: &UserPlan) -> bool {
        matches!(plan, UserPlan::Premium { status, .. } if status == "active")
    }

    /// Get feature limit from user plan (pure function)
    /// Note: This function is deprecated. Use jwt_features::get_effective_feature_limit instead.
    /// Free users now fallback to guest_limits.rs for consistency.
    pub fn get_feature_limit(plan: &UserPlan, feature_id: &str) -> Option<u32> {
        match plan {
            UserPlan::Premium { features, .. } => {
                // Premium users: get limits from JWT token features
                features
                    .get(feature_id)
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32)
            }
            UserPlan::Free => {
                // Free users: fallback to None, will use guest_limits.rs via jwt_features
                None
            }
            UserPlan::Guest => None, // Guest limits handled by guest_limits.rs
        }
    }

    /// Validate token expiration (pure function)
    pub fn is_token_expired(claims: &JwtClaims) -> bool {
        let now = Utc::now().timestamp() as u64;
        claims.exp < now
    }

    /// Get token expiration time (pure function)
    pub fn get_token_expiration(claims: &JwtClaims) -> DateTime<Utc> {
        DateTime::from_timestamp(claims.exp as i64, 0).unwrap_or_else(Utc::now)
    }

    /// Create guest user (pure function)
    pub fn create_guest_user() -> User {
        User {
            id: "guest".to_string(),
            email: "guest@local".to_string(),
            username: "Guest User".to_string(),
            first_name: "Guest".to_string(),
            last_name: "User".to_string(),
            user_type: Some("guest".to_string()),
            company_name: None,
            date_joined: Utc::now().to_rfc3339(),
            avatar: None,
            permissions: vec![], // Guest permissions handled by guest_limits.rs
            plan: UserPlan::Guest,
        }
    }
}
