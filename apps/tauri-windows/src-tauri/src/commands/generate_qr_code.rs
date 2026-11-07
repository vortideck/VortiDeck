use crate::crypto::E2EEKeys;
use base64::engine::general_purpose::STANDARD as Base64Standard;
use base64::Engine;
use image::{DynamicImage, ImageFormat, Luma};
use qrcode::QrCode;
use std::collections::HashMap;
use std::env;
use std::io::Cursor;
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use tokio::sync::RwLock;
use uuid::Uuid;
use vortideck_core::core::StateManager; // Works in both lib and bin (bin re-exports from app_lib::crypto)

// Type alias for PendingPairingsMap (matches main.rs)
type PendingPairingsMap = Arc<RwLock<HashMap<String, E2EEKeys>>>;

/// Compare a client-supplied pairing secret with the expected one without
/// short-circuiting on the first differing byte.
pub fn pairing_secret_matches(provided: &str, expected: &str) -> bool {
    let (provided, expected) = (provided.as_bytes(), expected.as_bytes());
    let mut diff = (provided.len() != expected.len()) as u8;
    for (index, byte) in expected.iter().enumerate() {
        diff |= byte ^ provided.get(index).copied().unwrap_or(0);
    }
    std::hint::black_box(diff) == 0
}

fn get_hostname() -> String {
    env::var("COMPUTERNAME") // Windows
        .or_else(|_| env::var("HOSTNAME")) // Unix-like systems
        .unwrap_or_else(|_| "unknown-host".to_string())
}

#[tauri::command]
pub async fn generate_qr_code(
    app_handle: AppHandle,
    data: Option<String>,
) -> Result<String, String> {
    let secret_key = Uuid::new_v4().to_string();
    let pairing_token = Uuid::new_v4().to_string();

    // Generate E2EE keys for pairing
    let e2ee_keys = E2EEKeys::generate();
    let desktop_public_key = Base64Standard.encode(e2ee_keys.desktop_public.as_bytes());

    // Use StateManager directly
    let state_manager = match app_handle.try_state::<Arc<StateManager>>() {
        Some(state) => state,
        None => {
            return Err("StateManager not available".to_string());
        }
    };
    state_manager.set_secret_key(secret_key.clone()).await;

    // 🔐 E2EE: Store full E2EEKeys object in PendingPairingsMap
    // This includes the desktop private key needed to complete handshake
    let pending_pairings = match app_handle.try_state::<PendingPairingsMap>() {
        Some(state) => state,
        None => {
            return Err("PendingPairingsMap not available".to_string());
        }
    };

    // Store E2EEKeys with pairing_token as key
    // Mobile will send this token in handshake message
    pending_pairings
        .write()
        .await
        .insert(pairing_token.clone(), e2ee_keys);

    println!("🔑 Generated new secret key and E2EE keys");

    // UNIVERSAL QR CODE: Include BOTH local and relay info
    // Mobile will auto-detect which connection method to use based on network conditions

    // Get local connection info
    let server_info = state_manager
        .get_server_info()
        .ok_or_else(|| "VortiDeck connection server is not ready".to_string())?;
    let ip_address = server_info.ip.to_string();
    let hostname = get_hostname();
    let websocket_url = server_info
        .connection_candidates
        .first()
        .map(|candidate| candidate.websocket_url())
        .unwrap_or_else(|| format!("ws://{}:{}/ws", ip_address, server_info.port));
    let custom_data = data.unwrap_or_else(|| "default-message".to_string());

    // Get relay info if relay is enabled
    let relay_enabled = state_manager.is_relay_enabled().await;
    let desktop_id = state_manager.get_desktop_id().await.unwrap_or_default();
    let mut qr_data = serde_json::json!({
        "schema_version": 2,
        "ip": ip_address,
        "key": secret_key,
        "ws": websocket_url,
        "host": hostname,
        "data": custom_data,
        "desktop_public_key": desktop_public_key,
        "pairing_token": pairing_token,
        "connection_candidates": server_info.connection_candidates,
        "relay_enabled": relay_enabled,
    });
    if !desktop_id.is_empty() {
        qr_data["desktop_id"] = serde_json::Value::String(desktop_id.clone());
    }

    if relay_enabled {
        let relay_url = state_manager
            .get_relay_url()
            .await
            .unwrap_or_else(|| "wss://vortideck.com/ws/relay/".to_string());

        println!(
            "📡 Relay enabled - desktop_id: {}, relay_url: {}",
            desktop_id, relay_url
        );

        qr_data["desktop_id"] = serde_json::Value::String(desktop_id);
        qr_data["relay_url"] = serde_json::Value::String(relay_url);
    } else {
        println!("🏠 Local-only mode - relay disabled");
    }

    let qr_data = serde_json::to_string(&qr_data).map_err(|error| error.to_string())?;

    let code = QrCode::new(&qr_data).map_err(|e| e.to_string())?;
    let image = code.render::<Luma<u8>>().build();
    let dynamic_image = DynamicImage::ImageLuma8(image);

    let mut buf = Cursor::new(Vec::new());
    dynamic_image
        .write_to(&mut buf, ImageFormat::Png)
        .map_err(|e| e.to_string())?;

    let base64_string = Base64Standard.encode(buf.get_ref());

    println!("✅ Generated universal QR code successfully");
    Ok(base64_string)
}

#[cfg(test)]
mod tests {
    use super::pairing_secret_matches;

    #[test]
    fn pairing_secret_comparison_requires_exact_match() {
        assert!(pairing_secret_matches("abc-123", "abc-123"));
        assert!(!pairing_secret_matches("abc-124", "abc-123"));
        assert!(!pairing_secret_matches("abc-12", "abc-123"));
        assert!(!pairing_secret_matches("abc-1234", "abc-123"));
        assert!(!pairing_secret_matches("", "abc-123"));
    }
}
