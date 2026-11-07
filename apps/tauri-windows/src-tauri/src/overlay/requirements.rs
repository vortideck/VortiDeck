//! Overlay feature requirements for VortiDeck
//!
//! This module provides clean, dedicated feature requirement structs for overlay functionality.

use vortideck_core::features::{guest_limits, FeatureRequirement, PremiumFeature, ResetPeriod};

/// Overlay elements usage requirement (3 free, unlimited premium)
pub struct OverlayElementsRequirement;

impl FeatureRequirement for OverlayElementsRequirement {
    fn get_requirements(&self) -> Vec<PremiumFeature> {
        let guest_limit = guest_limits::get_guest_feature_limit("overlay_elements");
        vec![PremiumFeature::StorageLimit {
            feature_id: "overlay_elements".to_string(),
            free_limit: guest_limit,
            premium_limit: None,
        }]
    }

    fn feature_context(&self) -> String {
        "overlay elements".to_string()
    }
}

/// Advanced custom overlay features (custom CSS, templating, JavaScript).
/// Free for all users; kept as a requirement type so callers keep one validation path.
pub struct CustomOverlayRequirement;

impl FeatureRequirement for CustomOverlayRequirement {
    fn get_requirements(&self) -> Vec<PremiumFeature> {
        Vec::new()
    }

    fn feature_context(&self) -> String {
        "custom overlay features".to_string()
    }
}

/// Saved overlay designs storage requirement (1 free, unlimited premium)
pub struct SavedOverlaysRequirement;

impl FeatureRequirement for SavedOverlaysRequirement {
    fn get_requirements(&self) -> Vec<PremiumFeature> {
        let guest_limit = guest_limits::get_guest_feature_limit("saved_overlays");
        vec![PremiumFeature::StorageLimit {
            feature_id: "saved_overlays".to_string(),
            free_limit: guest_limit,
            premium_limit: None,
        }]
    }

    fn feature_context(&self) -> String {
        "saved overlay designs".to_string()
    }
}
