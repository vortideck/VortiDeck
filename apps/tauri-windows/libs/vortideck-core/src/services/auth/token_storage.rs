//! Token Storage Service - Keyring Operations
//!
//! This module handles secure token storage using the system keyring.
//! It provides pure functions without Tauri dependencies.
//!
//! IMPORTANT: All keyring operations are serialized using a global mutex
//! to prevent Windows Credential Manager thread safety issues where
//! simultaneous reads/writes can return incorrect values.

use keyring::Entry;
use lazy_static::lazy_static;
use std::error::Error;
use std::sync::Mutex;

const SERVICE_NAME: &str = "vortideck";

// Global mutex to serialize all keyring operations (required for Windows thread safety)
// See: https://docs.rs/keyring/latest/keyring/ - "Always access any single credential from just one thread at a time"
lazy_static! {
    static ref KEYRING_LOCK: Mutex<()> = Mutex::new(());
}

/// Token storage errors
#[derive(Debug, thiserror::Error)]
pub enum TokenStorageError {
    #[error("Failed to access keyring: {0}")]
    KeyringError(String),
    #[error("Failed to decode token: {0}")]
    DecodeError(String),
    #[error("Token not found")]
    TokenNotFound,
}

/// Token storage service
pub struct TokenStorage;

impl TokenStorage {
    /// Immutable action credential generations, serialized with token access.
    /// Callers persist only the returned opaque identity, never this value.
    pub fn store_action_secret(value: &str) -> Result<String, TokenStorageError> {
        let identity = uuid::Uuid::new_v4().to_string();
        let _lock = KEYRING_LOCK
            .lock()
            .map_err(|_| TokenStorageError::KeyringError("Credential lock unavailable".into()))?;
        let entry =
            Entry::new_with_target(&format!("action-secret:{}", identity), SERVICE_NAME, "user")
                .map_err(|_| {
                    TokenStorageError::KeyringError("Cannot open action credential".into())
                })?;
        entry.set_secret(value.as_bytes()).map_err(|_| {
            TokenStorageError::KeyringError("Cannot store action credential".into())
        })?;
        Ok(identity)
    }

    pub fn load_action_secret(identity: &str) -> Result<String, TokenStorageError> {
        uuid::Uuid::parse_str(identity)
            .map_err(|_| TokenStorageError::DecodeError("Invalid credential reference".into()))?;
        let _lock = KEYRING_LOCK
            .lock()
            .map_err(|_| TokenStorageError::KeyringError("Credential lock unavailable".into()))?;
        let entry =
            Entry::new_with_target(&format!("action-secret:{}", identity), SERVICE_NAME, "user")
                .map_err(|_| {
                    TokenStorageError::KeyringError("Cannot open action credential".into())
                })?;
        let secret = entry
            .get_secret()
            .map_err(|_| TokenStorageError::TokenNotFound)?;
        String::from_utf8(secret)
            .map_err(|_| TokenStorageError::DecodeError("Invalid stored action credential".into()))
    }

    pub fn remove_action_secret(identity: &str) -> Result<(), TokenStorageError> {
        uuid::Uuid::parse_str(identity)
            .map_err(|_| TokenStorageError::DecodeError("Invalid credential reference".into()))?;
        let _lock = KEYRING_LOCK
            .lock()
            .map_err(|_| TokenStorageError::KeyringError("Credential lock unavailable".into()))?;
        let entry =
            Entry::new_with_target(&format!("action-secret:{}", identity), SERVICE_NAME, "user")
                .map_err(|_| {
                    TokenStorageError::KeyringError("Cannot open action credential".into())
                })?;
        entry
            .delete_credential()
            .map_err(|_| TokenStorageError::KeyringError("Cannot remove action credential".into()))
    }

    /// Get refresh token from secure storage
    pub fn get_refresh_token() -> Result<String, TokenStorageError> {
        let _lock = KEYRING_LOCK.lock().unwrap();
        let entry = Entry::new_with_target("refresh_token", SERVICE_NAME, "user")
            .map_err(|e| TokenStorageError::KeyringError(e.to_string()))?;

        match entry.get_secret() {
            Ok(secret) => {
                String::from_utf8(secret).map_err(|e| TokenStorageError::DecodeError(e.to_string()))
            }
            Err(_) => Err(TokenStorageError::TokenNotFound),
        }
    }

    /// Get access token from secure storage
    pub fn get_access_token() -> Result<String, TokenStorageError> {
        let _lock = KEYRING_LOCK.lock().unwrap();
        let entry = Entry::new_with_target("access_token", SERVICE_NAME, "user")
            .map_err(|e| TokenStorageError::KeyringError(e.to_string()))?;

        match entry.get_secret() {
            Ok(secret) => {
                String::from_utf8(secret).map_err(|e| TokenStorageError::DecodeError(e.to_string()))
            }
            Err(_) => Err(TokenStorageError::TokenNotFound),
        }
    }

    /// Save refresh token to secure storage
    pub fn set_refresh_token(refresh_token: &str) -> Result<(), TokenStorageError> {
        let _lock = KEYRING_LOCK.lock().unwrap();
        let entry = Entry::new_with_target("refresh_token", SERVICE_NAME, "user")
            .map_err(|e| TokenStorageError::KeyringError(e.to_string()))?;

        entry
            .set_secret(refresh_token.as_bytes())
            .map_err(|e| TokenStorageError::KeyringError(e.to_string()))?;

        Ok(())
    }

    /// Save access token to secure storage
    pub fn set_access_token(access_token: &str) -> Result<(), TokenStorageError> {
        let _lock = KEYRING_LOCK.lock().unwrap();
        let entry = Entry::new_with_target("access_token", SERVICE_NAME, "user")
            .map_err(|e| TokenStorageError::KeyringError(e.to_string()))?;

        entry
            .set_secret(access_token.as_bytes())
            .map_err(|e| TokenStorageError::KeyringError(e.to_string()))?;

        Ok(())
    }

    /// Delete refresh token from secure storage
    pub fn delete_refresh_token() -> Result<(), TokenStorageError> {
        if let Ok(entry) = Entry::new_with_target("refresh_token", SERVICE_NAME, "user") {
            entry
                .delete_credential()
                .map_err(|e| TokenStorageError::KeyringError(e.to_string()))?;
        }
        Ok(())
    }

    /// Delete access token from secure storage
    pub fn delete_access_token() -> Result<(), TokenStorageError> {
        if let Ok(entry) = Entry::new_with_target("access_token", SERVICE_NAME, "user") {
            entry
                .delete_credential()
                .map_err(|e| TokenStorageError::KeyringError(e.to_string()))?;
        }
        Ok(())
    }

    /// Clear all tokens
    pub fn clear_all_tokens() -> Result<(), TokenStorageError> {
        let _ = Self::delete_refresh_token();
        let _ = Self::delete_access_token();
        Ok(())
    }
}
