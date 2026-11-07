//! Feature limits module for VortiDeck
//!
//! This module handles feature usage limits.

use super::{FeatureError, FeatureId, FeatureResult};

/// Feature limit type
#[derive(Debug, Clone, Copy)]
pub enum LimitType {
    /// No limit
    Unlimited,
    /// Count-based limit
    Count(u32),
    /// Time-based limit (in seconds)
    Time(u64),
}

/// Feature limit
#[derive(Debug, Clone)]
pub struct FeatureLimit {
    feature_id: FeatureId,
    limit_type: LimitType,
    current_usage: u64,
}

impl FeatureLimit {
    /// Create a new feature limit
    pub fn new(feature_id: FeatureId, limit_type: LimitType) -> Self {
        Self {
            feature_id,
            limit_type,
            current_usage: 0,
        }
    }

    /// Check if the limit is exceeded
    pub fn is_exceeded(&self) -> bool {
        match self.limit_type {
            LimitType::Unlimited => false,
            LimitType::Count(limit) => self.current_usage >= limit as u64,
            LimitType::Time(limit) => self.current_usage >= limit,
        }
    }

    /// Increment usage
    pub fn increment(&mut self) -> FeatureResult<()> {
        if self.is_exceeded() {
            return Err(FeatureError::LimitExceeded(self.feature_id.clone()));
        }
        self.current_usage += 1;
        Ok(())
    }
}
