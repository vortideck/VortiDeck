pub mod access;
pub mod companion_auth;
pub mod html;
pub mod requirements;
pub mod server;
pub mod settings;
pub mod commands;
pub mod state;

// Re-export commonly used types
pub use access::{
    bearer_authorized, overlay_query_authorized, redact_capability_url, ServerAccessTokens,
};
pub use html::generate_overlay_html;
pub use requirements::{
    CustomOverlayRequirement, OverlayElementsRequirement, SavedOverlaysRequirement,
};
pub use server::{get_server_url, try_get_server_url};
pub use state::{OverlayDesign, OverlayElement, OverlayState, Position, Size};
