//! Feature registry module for VortiDeck
//!
//! This module handles feature registration.

use crate::features::types::PremiumFeature;
use crate::features::{Feature, FeatureId};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

#[derive(Debug, Error)]
pub enum FeatureError {
    #[error("Feature already registered: {0}")]
    AlreadyRegistered(String),
    #[error("Feature not found: {0}")]
    NotFound(String),
}

pub type FeatureResult<T> = Result<T, FeatureError>;

/// Feature registry for VortiDeck
pub struct FeatureRegistry {
    features: Arc<RwLock<HashMap<String, Feature>>>,
}

impl FeatureRegistry {
    /// Create a new registry instance
    pub fn new() -> Self {
        Self {
            features: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new feature
    pub async fn register_feature(&self, feature: Feature) -> FeatureResult<()> {
        let mut features = self.features.write().await;
        let id = feature.id.0.clone();
        if features.contains_key(&id) {
            return Err(FeatureError::AlreadyRegistered(id));
        }
        features.insert(id, feature);
        Ok(())
    }

    pub async fn get_feature(&self, id: &str) -> Option<Feature> {
        let features = self.features.read().await;
        features.get(id).cloned()
    }

    pub async fn remove_feature(&self, id: &str) -> FeatureResult<()> {
        let mut features = self.features.write().await;
        if features.remove(id).is_none() {
            return Err(FeatureError::NotFound(id.to_string()));
        }
        Ok(())
    }

    pub async fn list_features(&self) -> Vec<String> {
        let features = self.features.read().await;
        features.keys().cloned().collect()
    }

    pub async fn check_requirements(&self, id: &str) -> FeatureResult<Vec<PremiumFeature>> {
        let features = self.features.read().await;
        if let Some(feature) = features.get(id) {
            Ok(feature.get_requirements())
        } else {
            Err(FeatureError::NotFound(id.to_string()))
        }
    }
}
