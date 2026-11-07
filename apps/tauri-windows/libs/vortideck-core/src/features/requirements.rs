//! Feature requirements module for VortiDeck
//!
//! This module handles feature requirements.

use crate::features::types::PremiumFeature;
use async_trait::async_trait;

/// Synchronous feature requirement trait (for simple, config-based requirements)
pub trait FeatureRequirement: Send + Sync {
    fn get_requirements(&self) -> Vec<PremiumFeature>;
    fn feature_context(&self) -> String;
}

/// Asynchronous feature requirement trait (for database-backed, dynamic requirements)
#[async_trait]
pub trait AsyncFeatureRequirement: Send + Sync {
    async fn get_requirements(&self) -> Vec<PremiumFeature>;
    async fn feature_context(&self) -> String;
}

#[derive(Debug, Clone)]
pub struct BasicFeatureRequirement {
    requirements: Vec<PremiumFeature>,
    context: String,
}

impl BasicFeatureRequirement {
    pub fn new(requirements: Vec<PremiumFeature>, context: String) -> Self {
        Self {
            requirements,
            context,
        }
    }

    pub fn with_requirement(mut self, requirement: PremiumFeature) -> Self {
        self.requirements.push(requirement);
        self
    }

    pub fn with_context(mut self, context: String) -> Self {
        self.context = context;
        self
    }
}

impl FeatureRequirement for BasicFeatureRequirement {
    fn get_requirements(&self) -> Vec<PremiumFeature> {
        self.requirements.clone()
    }

    fn feature_context(&self) -> String {
        self.context.clone()
    }
}
