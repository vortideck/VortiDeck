// Legacy overlay server - DEPRECATED
// The overlay is now part of the unified Warp server in main.rs
// This file is kept for the get_server_url function only

use vortideck_core::core::{SerializableServerInfo, StateManager};

fn build_server_url(server_info: &SerializableServerInfo) -> String {
    format!(
        "http://{}:{}/overlay.html?overlayToken={}",
        server_info.ip, server_info.port, server_info.overlay_access_token
    )
}

/// Return the process-scoped overlay capability URL when the unified server is ready.
pub fn try_get_server_url(state_manager: &StateManager) -> Option<String> {
    state_manager
        .get_server_info()
        .as_ref()
        .map(build_server_url)
}

/// Get the overlay capability URL in code paths where server readiness is guaranteed.
pub fn get_server_url(state_manager: &StateManager) -> String {
    try_get_server_url(state_manager).expect("Server info must be available in StateManager")
}

#[cfg(test)]
mod tests {
    use super::build_server_url;
    use std::net::{IpAddr, Ipv4Addr};
    use vortideck_core::core::SerializableServerInfo;

    #[test]
    fn overlay_url_is_a_tokenized_capability() {
        let info = SerializableServerInfo {
            ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: 9001,
            icons_server_port: 9002,
            overlay_access_token: "test-secret".to_string(),
            device_type: "desktop".to_string(),
            server_name: "test".to_string(),
            connection_candidates: Vec::new(),
        };

        assert_eq!(
            build_server_url(&info),
            "http://127.0.0.1:9001/overlay.html?overlayToken=test-secret"
        );
    }
}
