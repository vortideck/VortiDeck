/// Automatic Banner Rotation System
/// Handles automatic banner display and rotation when OBS connects
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tauri::{path::BaseDirectory, AppHandle, Manager};
use tokio_tungstenite::tungstenite::Message;
use vortideck_core::core::StateManager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BannerContent {
    pub id: String,
    pub content_data: String,
    pub content_type: String,
    pub duration_seconds: u32,
    pub campaign_id: Option<String>,
    pub priority: Option<String>,
    pub css: Option<String>,
    pub css_locked: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BannerQueue {
    pub banners: Vec<BannerContent>,
    pub premium_status: String,
    pub auto_rotation: bool,
    pub batch_id: Option<String>,
    pub rotation_interval_minutes: Option<u64>,
}

/// Start automatic banner rotation system when OBS connects
pub async fn start_automatic_banner_rotation(
    state_manager: Arc<StateManager>,
    app_handle: Option<AppHandle>,
) -> Result<(), String> {
    println!("🎯 Starting automatic banner rotation system");

    // Check if OBS is actually connected
    if state_manager.get_obs_plugin_sender().await.is_none() {
        return Err("OBS plugin not connected".to_string());
    }

    // Load fallback banners
    let fallback_banners = match app_handle {
        Some(app) => load_fallback_banners_with_paths(&app).await?,
        None => load_fallback_banners_basic().await,
    };

    if fallback_banners.is_empty() {
        return Err("No fallback banners available".to_string());
    }

    // Create banner queue with auto-rotation
    let banner_queue = BannerQueue {
        banners: fallback_banners,
        premium_status: "false".to_string(),
        auto_rotation: true,
        batch_id: Some(format!("auto_rotation_{}", chrono::Utc::now().timestamp())),
        rotation_interval_minutes: Some(3), // Rotate every 3 minutes
    };

    // Send banner queue to OBS plugin
    send_banner_queue_to_obs(&state_manager, banner_queue).await?;

    println!("✅ Automatic banner rotation started successfully");
    Ok(())
}

/// Send banner queue to OBS plugin via WebSocket
async fn send_banner_queue_to_obs(
    state_manager: &StateManager,
    banner_queue: BannerQueue,
) -> Result<(), String> {
    let obs_sender = state_manager
        .get_obs_plugin_sender()
        .await
        .ok_or("No OBS plugin sender available")?;

    let integration_guid = state_manager
        .get_obs_integration_guid()
        .unwrap_or_else(|| "obs-fallback".to_string());

    let queue_message = json!({
        "path": "/api/v1/integration/sdk/action/invoke",
        "verb": "BROADCAST",
        "payload": {
            "actionId": "obs_banner_set_queue",
            "integrationGuid": integration_guid,
            "parameters": {
                "banners": banner_queue.banners,
                "premium_status": banner_queue.premium_status,
                "auto_rotation": banner_queue.auto_rotation,
                "batch_id": banner_queue.batch_id,
                "rotation_interval_minutes": banner_queue.rotation_interval_minutes.unwrap_or(3)
            }
        }
    });

    obs_sender
        .send(Message::Text(queue_message.to_string().into()))
        .map_err(|e| format!("Failed to send banner queue: {}", e))?;

    println!(
        "✅ Banner queue sent to OBS with {} banners",
        banner_queue.banners.len()
    );
    Ok(())
}

/// Load fallback banners with resolved file paths (when AppHandle available)
async fn load_fallback_banners_with_paths(
    app_handle: &AppHandle,
) -> Result<Vec<BannerContent>, String> {
    // Resolve actual file paths
    let promo_1_path = app_handle
        .path()
        .resolve(
            "resources/ads/fallback/vortideck-promo-1.png",
            BaseDirectory::Resource,
        )
        .map_err(|e| format!("Failed to resolve promo-1 asset: {}", e))?;

    let promo_2_path = app_handle
        .path()
        .resolve(
            "resources/ads/fallback/vortideck-promo-2.png",
            BaseDirectory::Resource,
        )
        .map_err(|e| format!("Failed to resolve promo-2 asset: {}", e))?;

    let logo_path = app_handle
        .path()
        .resolve(
            "resources/ads/fallback/vortideck-logo.png",
            BaseDirectory::Resource,
        )
        .map_err(|e| format!("Failed to resolve logo asset: {}", e))?;

    Ok(vec![
        BannerContent {
            id: "vortideck_promo_1".to_string(),
            content_data: promo_1_path.to_string_lossy().to_string(),
            content_type: "image".to_string(),
            duration_seconds: 8,
            campaign_id: Some("vortideck_promo".to_string()),
            priority: Some("medium".to_string()),
            css: None,
            css_locked: Some(false),
        },
        BannerContent {
            id: "vortideck_promo_2".to_string(),
            content_data: promo_2_path.to_string_lossy().to_string(),
            content_type: "image".to_string(),
            duration_seconds: 10,
            campaign_id: Some("vortideck_promo".to_string()),
            priority: Some("medium".to_string()),
            css: None,
            css_locked: Some(false),
        },
        BannerContent {
            id: "vortideck_logo".to_string(),
            content_data: logo_path.to_string_lossy().to_string(),
            content_type: "image".to_string(),
            duration_seconds: 6,
            campaign_id: Some("vortideck_branding".to_string()),
            priority: Some("low".to_string()),
            css: None,
            css_locked: Some(false),
        },
    ])
}

/// Load fallback banners with relative paths (fallback when no AppHandle)
async fn load_fallback_banners_basic() -> Vec<BannerContent> {
    vec![
        BannerContent {
            id: "vortideck_promo_1".to_string(),
            content_data: "resources/ads/fallback/vortideck-promo-1.png".to_string(),
            content_type: "image".to_string(),
            duration_seconds: 8,
            campaign_id: Some("vortideck_promo".to_string()),
            priority: Some("medium".to_string()),
            css: None,
            css_locked: Some(false),
        },
        BannerContent {
            id: "vortideck_promo_2".to_string(),
            content_data: "resources/ads/fallback/vortideck-promo-2.png".to_string(),
            content_type: "image".to_string(),
            duration_seconds: 10,
            campaign_id: Some("vortideck_promo".to_string()),
            priority: Some("medium".to_string()),
            css: None,
            css_locked: Some(false),
        },
        BannerContent {
            id: "vortideck_logo".to_string(),
            content_data: "resources/ads/fallback/vortideck-logo.png".to_string(),
            content_type: "image".to_string(),
            duration_seconds: 6,
            campaign_id: Some("vortideck_branding".to_string()),
            priority: Some("low".to_string()),
            css: None,
            css_locked: Some(false),
        },
    ]
}

/// Show a single fallback banner (for testing or one-time display)
pub async fn show_single_fallback_banner(
    state_manager: Arc<StateManager>,
    app_handle: Option<AppHandle>,
    banner_index: Option<usize>,
) -> Result<(), String> {
    println!("🎯 Showing single fallback banner");

    // Load banners
    let banners = match app_handle {
        Some(app) => load_fallback_banners_with_paths(&app).await?,
        None => load_fallback_banners_basic().await,
    };

    if banners.is_empty() {
        return Err("No fallback banners available".to_string());
    }

    // Select banner (random if index not specified)
    let selected_index = banner_index.unwrap_or_else(|| {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        chrono::Utc::now().timestamp().hash(&mut hasher);
        (hasher.finish() as usize) % banners.len()
    });

    let selected_banner = &banners[selected_index.min(banners.len() - 1)];

    // Send single banner to OBS
    send_single_banner_to_obs(&state_manager, selected_banner.clone()).await?;

    println!("✅ Single banner displayed: {}", selected_banner.id);
    Ok(())
}

/// Send single banner to OBS plugin
async fn send_single_banner_to_obs(
    state_manager: &StateManager,
    banner: BannerContent,
) -> Result<(), String> {
    let obs_sender = state_manager
        .get_obs_plugin_sender()
        .await
        .ok_or("No OBS plugin sender available")?;

    let integration_guid = state_manager
        .get_obs_integration_guid()
        .unwrap_or_else(|| "obs-fallback".to_string());

    let banner_message = json!({
        "path": "/api/v1/integration/sdk/action/invoke",
        "verb": "BROADCAST",
        "payload": {
            "actionId": "obs_banner_set_data",
            "integrationGuid": integration_guid,
            "parameters": {
                "content_data": banner.content_data,
                "content_type": banner.content_type,
                "duration_seconds": banner.duration_seconds,
                "premium_status": "false",
                "interrupt_current": false,
                "css": banner.css,
                "css_locked": banner.css_locked.unwrap_or(false)
            }
        }
    });

    obs_sender
        .send(Message::Text(banner_message.to_string().into()))
        .map_err(|e| format!("Failed to send banner: {}", e))?;

    Ok(())
}

/// Stop automatic banner rotation system when OBS disconnects
pub async fn stop_automatic_banner_rotation(
    state_manager: Arc<StateManager>,
) -> Result<(), String> {
    println!("🛑 Stopping automatic banner rotation system");

    // Check if OBS is still connected (it might already be disconnected)
    if let Some(obs_sender) = state_manager.get_obs_plugin_sender().await {
        let integration_guid = state_manager
            .get_obs_integration_guid()
            .unwrap_or_else(|| "obs-fallback".to_string());

        // Send hide banner command to clear any current banners
        let hide_message = json!({
            "path": "/api/v1/integration/sdk/action/invoke",
            "verb": "BROADCAST",
            "payload": {
                "actionId": "obs_banner_hide",
                "integrationGuid": integration_guid,
                "parameters": {
                    "banner_id": null,
                    "fade_out": true,
                    "fade_duration_ms": 300
                }
            }
        });

        if let Err(e) = obs_sender.send(Message::Text(hide_message.to_string().into())) {
            println!(
                "⚠️ Failed to send banner hide command (OBS might be disconnected): {}",
                e
            );
        } else {
            println!("✅ Sent banner hide command to clear banners");
        }
    }

    // Clear any banner-related state from StateManager if needed
    // (This could be extended to clear banner queue state, etc.)

    println!("✅ Banner rotation system stopped");
    Ok(())
}
