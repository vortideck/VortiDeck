pub mod auto_rotation;
/// Banners module for VortiDeck
/// Handles banner display functionality for OBS integration
pub mod html;

// Re-export the main functions
pub use auto_rotation::{
    show_single_fallback_banner, start_automatic_banner_rotation, stop_automatic_banner_rotation,
    BannerContent, BannerQueue,
};
pub use html::generate_banner_html;

// Banner-related types could go here in the future
// e.g., BannerState, BannerConfig, etc.
