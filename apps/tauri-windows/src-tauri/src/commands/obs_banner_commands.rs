use chrono::Timelike;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{path::BaseDirectory, AppHandle, Manager, State};
use tokio::sync::RwLock;
use uuid::Uuid;
use vortideck_core::core::StateManager;

/// 🎯 Generic OBS banner command sender - replaces 7 identical functions
async fn send_obs_banner_command(
    state_manager: &StateManager,
    action_id: &str,
    parameters: serde_json::Value,
    success_message: &str,
) -> Result<serde_json::Value, String> {
    println!("🎯 Sending OBS banner command: {}", action_id);

    // Get OBS integration GUID from StateManager
    let integration_guid = state_manager
        .get_obs_integration_guid()
        .unwrap_or_else(|| "obs-integration-fallback".to_string());

    if let Some(obs_plugin_sender) = state_manager.get_obs_plugin_sender().await {
        // Build the WebSocket message in the correct format
        let banner_message = serde_json::json!({
            "path": "/api/v1/integration/sdk/action/invoke",
            "verb": "BROADCAST",
            "payload": {
                "actionId": action_id,
                "integrationGuid": integration_guid,
                "parameters": parameters
            }
        });

        let message_str = banner_message.to_string();
        println!("🎬 Sending banner message: {}", message_str);

        // Send directly to OBS plugin
        if let Err(e) = obs_plugin_sender.send(tokio_tungstenite::tungstenite::Message::Text(
            message_str.into(),
        )) {
            println!("❌ Failed to send banner message to OBS: {}", e);
            Err(format!("Failed to send banner message: {}", e))
        } else {
            println!("✅ VortiDeck Banner API message sent successfully");
            Ok(serde_json::json!({"success": true, "message": success_message}))
        }
    } else {
        Err("OBS plugin not connected".to_string())
    }
}

// 🎯 Banner-related structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BannerData {
    pub content_data: String,
    pub content_type: String,
    pub duration_seconds: u32,
    pub premium_status: String,
    pub interrupt_current: Option<bool>,
    pub css: Option<String>,
    pub css_locked: Option<bool>,
    pub queue_mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BannerQueueData {
    pub banners: Vec<BannerContent>,
    pub premium_status: String,
    pub auto_rotation: bool,
    pub batch_id: Option<String>,
}

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
pub struct AnalyticsParams {
    pub tracking_level: String,
    pub report_interval_seconds: u64,
    pub include_viewer_metrics: bool,
    pub include_revenue_attribution: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsReportParams {
    pub report_type: String,
    pub time_range: String,
    pub include_revenue: bool,
}

// 🔄 Context-Aware Ad Refresh System
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamContext {
    // Local context detected by desktop app
    pub current_game: Option<String>,
    pub viewer_count: u64,
    pub stream_platform: String,
    pub stream_category: String,
    pub stream_title: Option<String>,
    pub is_sponsored: bool,
    pub has_premium_viewers: bool,
    pub chat_activity: String, // "low", "medium", "high"
    pub timezone: String,
    pub obs_streaming: bool,
    pub stream_quality: Option<String>, // "1080p60", "720p30"
                                        // Backend determines user from auth headers - no streamer_id needed
}

#[derive(Debug, Clone, PartialEq)]
pub enum RefreshTrigger {
    GameChange,
    ViewerCountSpike,
    CategoryChange,
    TimeSlot,
    QueueEmpty,
    HighValueViewer,
    SponsoredContent,
    TournamentMode,
    RaidReceived,
    SubscriberMilestone,
}

#[derive(Debug, Clone)]
pub struct RefreshStrategy {
    pub trigger: RefreshTrigger,
    pub interval: Duration,
    pub priority: u8, // 1-10, 10 = highest
}

// Simple context cache - stores last context for comparison
static LAST_CONTEXT: std::sync::OnceLock<RwLock<(StreamContext, Instant)>> =
    std::sync::OnceLock::new();

fn get_last_context() -> &'static RwLock<(StreamContext, Instant)> {
    LAST_CONTEXT.get_or_init(|| {
        RwLock::new((
            StreamContext {
                current_game: None,
                viewer_count: 0,
                stream_platform: "unknown".to_string(),
                stream_category: "gaming".to_string(),
                stream_title: None,
                is_sponsored: false,
                has_premium_viewers: false,
                chat_activity: "low".to_string(),
                timezone: "UTC".to_string(),
                obs_streaming: false,
                stream_quality: None,
            },
            Instant::now(),
        ))
    })
}

impl RefreshStrategy {
    /// Calculate optimal refresh interval based on stream context
    pub fn calculate_refresh_interval(
        context: &StreamContext,
        prev_context: Option<&StreamContext>,
    ) -> Self {
        // Detect context changes
        if let Some(prev) = prev_context {
            // Game changed - immediate refresh for new targeting
            if context.current_game != prev.current_game {
                return RefreshStrategy {
                    trigger: RefreshTrigger::GameChange,
                    interval: Duration::from_secs(0),
                    priority: 10,
                };
            }

            // Viewer surge (>100% increase)
            if context.viewer_count > prev.viewer_count * 2 && context.viewer_count > 100 {
                return RefreshStrategy {
                    trigger: RefreshTrigger::ViewerCountSpike,
                    interval: Duration::from_secs(5),
                    priority: 9,
                };
            }

            // Category change (Gaming -> Just Chatting)
            if context.stream_category != prev.stream_category {
                return RefreshStrategy {
                    trigger: RefreshTrigger::CategoryChange,
                    interval: Duration::from_secs(0),
                    priority: 9,
                };
            }

            // Sponsored content started
            if context.is_sponsored && !prev.is_sponsored {
                return RefreshStrategy {
                    trigger: RefreshTrigger::SponsoredContent,
                    interval: Duration::from_secs(0),
                    priority: 8,
                };
            }
        }

        // High-value viewers detected
        if context.has_premium_viewers {
            return RefreshStrategy {
                trigger: RefreshTrigger::HighValueViewer,
                interval: Duration::from_secs(30),
                priority: 7,
            };
        }

        // Peak hours detection (7-11 PM local time)
        let hour = chrono::Local::now().hour();
        if (19..=23).contains(&hour) {
            return RefreshStrategy {
                trigger: RefreshTrigger::TimeSlot,
                interval: Duration::from_secs(60),
                priority: 6,
            };
        }

        // High chat activity = engaged audience
        if context.chat_activity == "high" {
            return RefreshStrategy {
                trigger: RefreshTrigger::HighValueViewer,
                interval: Duration::from_secs(90),
                priority: 5,
            };
        }

        // Default stable streaming
        RefreshStrategy {
            trigger: RefreshTrigger::TimeSlot,
            interval: Duration::from_secs(300), // 5 minutes
            priority: 3,
        }
    }

    /// Check if immediate refresh is needed
    pub fn needs_immediate_refresh(&self) -> bool {
        matches!(
            self.trigger,
            RefreshTrigger::GameChange
                | RefreshTrigger::CategoryChange
                | RefreshTrigger::QueueEmpty
                | RefreshTrigger::SponsoredContent
        ) && self.interval.as_secs() <= 5
    }
}

/// 🎯 Set single banner data
#[tauri::command]
pub async fn obs_banner_set_data(
    state_manager: State<'_, Arc<StateManager>>,
    banner_data: BannerData,
) -> Result<serde_json::Value, String> {
    println!("🎯 Setting OBS banner data: {:?}", banner_data);

    // Create message for OBS integration
    let ad_id = format!("banner_{}", &Uuid::new_v4().to_string()[..8]);
    let campaign_id = format!("campaign_{}", &Uuid::new_v4().to_string()[..8]);

    // Default CSS if none provided
    let default_css = format!(
        r#"
          body {{
            margin: 0;
            padding: 0;
            background: transparent;
            width: 100vw;
            height: 100vh;
            overflow: hidden;
            position: relative;
          }}

          img {{
            position: fixed;
            left: 50%;
            bottom: 0;
            transform: translateX(-50%) translateY(100%);
            width: 70vw;
            max-width: 70vw;
            height: 150px;
            object-fit: cover;
            border-radius: 24px 24px 0 0;
            box-shadow: 0 0 32px 0 rgba(0,0,0,0.25);
            background: rgba(30, 30, 30, 0.95);
            border: none;
            z-index: 9999;
            animation: banner-slide-in 0.7s cubic-bezier(0.4,0,0.2,1) forwards, banner-slide-out 0.7s cubic-bezier(0.4,0,0.2,1) forwards;
            animation-delay: 0s, calc({}s - 0.7s);
          }}

          @keyframes banner-slide-in {{
            from {{ transform: translateX(-50%) translateY(100%); opacity: 0; }}
            to   {{ transform: translateX(-50%) translateY(0); opacity: 1; }}
          }}
          @keyframes banner-slide-out {{
            from {{ transform: translateX(-50%) translateY(0); opacity: 1; }}
            to   {{ transform: translateX(-50%) translateY(100%); opacity: 0; }}
          }}
        "#,
        banner_data.duration_seconds
    );

    let css_to_use = banner_data.css.as_deref().unwrap_or(&default_css);

    let parameters = serde_json::json!({
        "ad_id": ad_id,
        "campaign_id": campaign_id,
        "content_data": banner_data.content_data,
        "content_type": banner_data.content_type,
        "css": css_to_use,
        "duration_seconds": banner_data.duration_seconds.to_string(),
        "height": 150,
        "width": 900,
        "premium_status": banner_data.premium_status,
        "auto_hide": "true",
        "queue_mode": banner_data.queue_mode
    });

    send_obs_banner_command(
        &state_manager,
        "obs_banner_set_data",
        parameters,
        "Banner data set successfully",
    )
    .await
}

/// 🎯 Set banner queue
#[tauri::command]
pub async fn obs_banner_set_queue(
    state_manager: State<'_, Arc<StateManager>>,
    queue_data: BannerQueueData,
) -> Result<serde_json::Value, String> {
    println!(
        "🎯 Setting OBS banner queue: {} banners",
        queue_data.banners.len()
    );

    let parameters = serde_json::json!({
        "banners": queue_data.banners,
        "premium_status": queue_data.premium_status,
        "auto_rotation": queue_data.auto_rotation,
        "batch_id": queue_data.batch_id.unwrap_or_else(|| Uuid::new_v4().to_string()),
        "queue_mode": "true"
    });

    send_obs_banner_command(
        &state_manager,
        "obs_banner_set_queue",
        parameters,
        "Banner queue set successfully",
    )
    .await
}

/// 🎯 Add to existing banner queue
#[tauri::command]
pub async fn obs_banner_add_to_queue(
    state_manager: State<'_, Arc<StateManager>>,
    banners: Vec<BannerContent>,
    premium_status: String,
) -> Result<serde_json::Value, String> {
    println!("🎯 Adding {} banners to OBS queue", banners.len());

    let parameters = serde_json::json!({
        "banners": banners,
        "premium_status": premium_status,
        "append_to_existing": true,
        "queue_mode": "true"
    });

    send_obs_banner_command(
        &state_manager,
        "obs_banner_add_to_queue",
        parameters,
        "Banners added to queue successfully",
    )
    .await
}

/// 🎯 Hide banner
#[tauri::command]
pub async fn obs_banner_hide(
    state_manager: State<'_, Arc<StateManager>>,
    banner_id: Option<String>,
) -> Result<serde_json::Value, String> {
    println!("🎯 Hiding OBS banner: {:?}", banner_id);

    let parameters = serde_json::json!({
        "banner_id": banner_id
    });

    send_obs_banner_command(
        &state_manager,
        "obs_banner_hide",
        parameters,
        "Banner hidden successfully",
    )
    .await
}

/// 🎯 Show banner
#[tauri::command]
pub async fn obs_banner_show(
    state_manager: State<'_, Arc<StateManager>>,
    banner_id: Option<String>,
) -> Result<serde_json::Value, String> {
    println!("🎯 Showing OBS banner: {:?}", banner_id);

    let parameters = serde_json::json!({
        "banner_id": banner_id
    });

    send_obs_banner_command(
        &state_manager,
        "obs_banner_show",
        parameters,
        "Banner shown successfully",
    )
    .await
}

/// 📊 Enable analytics tracking
#[tauri::command]
pub async fn obs_analytics_enable(
    state_manager: State<'_, Arc<StateManager>>,
    params: AnalyticsParams,
) -> Result<serde_json::Value, String> {
    println!("📊 Enabling OBS analytics: {:?}", params);

    let parameters = serde_json::json!({
        "tracking_level": params.tracking_level,
        "report_interval_seconds": params.report_interval_seconds,
        "include_viewer_metrics": params.include_viewer_metrics,
        "include_revenue_attribution": params.include_revenue_attribution
    });

    send_obs_banner_command(
        &state_manager,
        "obs_analytics_enable",
        parameters,
        "Analytics enabled successfully",
    )
    .await
}

/// 📊 Get analytics report
#[tauri::command]
pub async fn obs_analytics_get_report(
    state_manager: State<'_, Arc<StateManager>>,
    params: AnalyticsReportParams,
) -> Result<serde_json::Value, String> {
    println!("📊 Getting OBS analytics report: {:?}", params);

    let parameters = serde_json::json!({
        "report_type": params.report_type,
        "time_range": params.time_range,
        "include_revenue": params.include_revenue
    });

    send_obs_banner_command(
        &state_manager,
        "obs_analytics_get_report",
        parameters,
        "Analytics report requested successfully",
    )
    .await
}

/// 🎯 Get current banner queue status
#[tauri::command]
pub async fn obs_banner_get_queue_status(
    state_manager: State<'_, Arc<StateManager>>,
) -> Result<serde_json::Value, String> {
    println!("🎯 Getting OBS banner queue status");

    // Get queue state from StateManager
    if let Some(queue_state) = state_manager.get_banner_queue_state().await {
        Ok(serde_json::json!({
            "success": true,
            "queue_state": queue_state
        }))
    } else {
        Ok(serde_json::json!({
            "success": true,
            "queue_state": null,
            "message": "No queue state available"
        }))
    }
}

/// 🎯 Test banner functionality (for debugging)
#[tauri::command]
pub async fn obs_banner_test(
    state_manager: State<'_, Arc<StateManager>>,
) -> Result<serde_json::Value, String> {
    println!("🎯 Testing OBS banner functionality");

    // Create test banner data
    let test_banner = BannerData {
        content_data: "https://example.com/test-banner.png".to_string(),
        content_type: "image".to_string(),
        duration_seconds: 30,
        premium_status: "false".to_string(),
        interrupt_current: Some(false),
        css: None,
        css_locked: Some(false),
        queue_mode: Some("false".to_string()),
    };

    // Test setting banner data
    match obs_banner_set_data(state_manager, test_banner).await {
        Ok(result) => Ok(serde_json::json!({
            "success": true,
            "message": "Banner test completed successfully",
            "result": result
        })),
        Err(e) => Err(format!("Banner test failed: {}", e)),
    }
}

// 🔄 Context-Aware Ad Refresh Commands

/// 🔄 Smart ad fetch with context awareness
#[tauri::command]
pub async fn smart_fetch_ads_with_context(
    app_handle: AppHandle,
    state_manager: State<'_, Arc<StateManager>>,
    context: StreamContext,
) -> Result<serde_json::Value, String> {
    println!("🔄 Smart fetch ads with local context");

    // Get previous context for refresh strategy calculation
    let mut last_context = get_last_context().write().await;
    let prev_context = Some(last_context.0.clone());

    // Calculate refresh strategy
    let strategy = RefreshStrategy::calculate_refresh_interval(&context, prev_context.as_ref());

    println!(
        "🎯 Refresh strategy: {:?} (priority: {})",
        strategy.trigger, strategy.priority
    );

    // Fetch ads from backend with enhanced context
    let api_url =
        std::env::var("API_URL").unwrap_or_else(|_| "https://api.vortideck.com".to_string());
    let url = format!("{}/api/v1/ads/smart", api_url);

    let request_payload = serde_json::json!({
        // Local context from desktop app
        "local_context": {
            "current_game": context.current_game,
            "viewer_count": context.viewer_count,
            "stream_platform": context.stream_platform,
            "stream_category": context.stream_category,
            "stream_title": context.stream_title,
            "is_sponsored": context.is_sponsored,
            "has_premium_viewers": context.has_premium_viewers,
            "chat_activity": context.chat_activity,
            "timezone": context.timezone,
            "obs_streaming": context.obs_streaming,
            "stream_quality": context.stream_quality
        },
        // Refresh strategy info (optional for backend analytics)
        "refresh_info": {
            "trigger": format!("{:?}", strategy.trigger),
            "priority": strategy.priority,
            "timestamp": chrono::Utc::now().timestamp()
        }
    });

    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(150)) // Fast timeout for real-time
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    match client.post(&url).json(&request_payload).send().await {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<serde_json::Value>().await {
                    Ok(ads_response) => {
                        println!(
                            "✅ Smart fetch successful with strategy: {:?}",
                            strategy.trigger
                        );

                        // Update last context for next comparison
                        last_context.0 = context.clone();
                        last_context.1 = Instant::now();

                        Ok(serde_json::json!({
                            "success": true,
                            "ads": ads_response.get("ads").unwrap_or(&serde_json::Value::Array(vec![]))
                        }))
                    }
                    Err(e) => {
                        println!("❌ Failed to parse smart ads response: {}", e);
                        // Return cached ads if available
                        get_cached_ads_fallback(&app_handle).await
                    }
                }
            } else {
                println!("❌ Backend returned error: {}", response.status());
                get_cached_ads_fallback(&app_handle).await
            }
        }
        Err(e) => {
            println!("❌ Failed to connect to backend: {}", e);
            get_cached_ads_fallback(&app_handle).await
        }
    }
}

/// 🔄 Update stream context (triggers refresh if needed)
#[tauri::command]
pub async fn update_stream_context(context: StreamContext) -> Result<serde_json::Value, String> {
    println!("🔄 Updating local stream context");

    let mut last_context = get_last_context().write().await;
    let prev_context = Some(last_context.0.clone());

    let strategy = RefreshStrategy::calculate_refresh_interval(&context, prev_context.as_ref());
    let should_refresh = strategy.needs_immediate_refresh();

    // Update last context
    last_context.0 = context.clone();
    last_context.1 = Instant::now();

    Ok(serde_json::json!({
        "success": true,
        "should_refresh": should_refresh,
        "refresh_strategy": {
            "trigger": format!("{:?}", strategy.trigger),
            "interval_seconds": strategy.interval.as_secs(),
            "priority": strategy.priority
        },
        "context_updated": true
    }))
}

/// 🔄 Get current refresh recommendations  
#[tauri::command]
pub async fn get_refresh_recommendations() -> Result<serde_json::Value, String> {
    let last_context = get_last_context().read().await;

    let strategy = RefreshStrategy::calculate_refresh_interval(&last_context.0, None);
    let time_since_update = last_context.1.elapsed();
    let should_refresh =
        time_since_update >= strategy.interval || strategy.needs_immediate_refresh();

    Ok(serde_json::json!({
        "success": true,
        "should_refresh": should_refresh,
        "time_since_update_seconds": time_since_update.as_secs(),
        "recommended_interval_seconds": strategy.interval.as_secs(),
        "trigger": format!("{:?}", strategy.trigger),
        "priority": strategy.priority
    }))
}

/// 🔄 Clear context cache (for testing/debugging)
#[tauri::command]
pub async fn clear_context_cache() -> Result<serde_json::Value, String> {
    let mut last_context = get_last_context().write().await;

    // Reset to default context
    last_context.0 = StreamContext {
        current_game: None,
        viewer_count: 0,
        stream_platform: "unknown".to_string(),
        stream_category: "gaming".to_string(),
        stream_title: None,
        is_sponsored: false,
        has_premium_viewers: false,
        chat_activity: "low".to_string(),
        timezone: "UTC".to_string(),
        obs_streaming: false,
        stream_quality: None,
    };
    last_context.1 = Instant::now();

    println!("🧹 Cleared context cache");

    Ok(serde_json::json!({
        "success": true,
        "message": "Context cache cleared"
    }))
}

// 🔧 Helper functions

async fn get_cached_ads_fallback(app_handle: &AppHandle) -> Result<serde_json::Value, String> {
    // Load local VortiDeck promotional ads using Tauri resource system
    println!("🎯 Loading fallback VortiDeck promotional ads from local resources");

    // Resolve paths to local ad assets
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

    let fallback_ads = vec![
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
    ];

    println!(
        "📦 Loaded {} fallback ads from local resources",
        fallback_ads.len()
    );

    Ok(serde_json::json!({
        "success": true,
        "ads": fallback_ads
    }))
}
