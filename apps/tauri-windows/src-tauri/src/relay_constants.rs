/// Relay WebSocket Message Type Constants
/// Centralized constants for relay message types to prevent typos and improve maintainability

/// Message types sent TO the relay server (desktop → relay)
pub mod outgoing {
    pub const REGISTER_DESKTOP: &str = "register_desktop";
    pub const RELAY_TO_MOBILE: &str = "relay_message";
    pub const DESKTOP_STATUS: &str = "desktop_status";
}

/// Message types received FROM the relay server (relay → desktop)
pub mod incoming {
    pub const RELAY_REGISTERED: &str = "relay_registered";
    pub const RELAY_FROM_MOBILE: &str = "relay_from_mobile";
    pub const MOBILE_CONNECTED: &str = "mobile_connected";
    pub const MOBILE_DISCONNECTED: &str = "mobile_disconnected";
    pub const ERROR: &str = "error";
}

/// Connection metadata keys
pub mod metadata {
    pub const OS: &str = "os";
    pub const HOSTNAME: &str = "hostname";
    pub const VERSION: &str = "version";
    pub const IP: &str = "ip";
    pub const DESKTOP_ID: &str = "desktop_id";
    pub const DESKTOP_NAME: &str = "desktop_name";
    pub const DESKTOP_METADATA: &str = "desktop_metadata";
}
