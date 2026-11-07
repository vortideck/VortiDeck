//! # VortiDeck Analytics Events
//!
//! All PostHog events tracked across the app. Keep this list updated when adding new events.
//!
//! ## Rust Backend Events (via `crate::analytics::capture()`)
//!
//! | Event                    | Properties                     | Location                          |
//! |--------------------------|--------------------------------|-----------------------------------|
//! | `app_opened`             | —                              | main.rs (setup)                   |
//! | `button_pressed`         | action_type, button_id         | command_handler.rs                |
//! | `context_menu_triggered` | action_id, file_count          | main.rs (deep link handler)       |
//! | `device_paired`          | device_type                    | main.rs (pairing handler, 2 paths)|
//! | `deck_created`           | deck_name                      | db/utils.rs                       |
//! | `flow_executed`          | flow_name                      | adapters/flow_commands.rs         |
//! | `flow_saved`             | flow_name                      | adapters/flow_commands.rs         |
//! | `user_logged_in`         | method: "google"               | commands/auth_commands.rs         |
//! | `macro_recorded`         | macro_name                     | adapters/macro_commands.rs        |
//! | `macro_executed`         | macro_id                       | adapters/macro_commands.rs        |
//! | `streaming_connected`    | url                            | commands/streaming_commands.rs    |
//! | `assistant_message_sent` | provider, model                | commands/assistant_commands.rs    |
//!
//! ## Frontend Events (via `$posthog.capture()`)
//!
//! | Event                    | Properties                     | Location                          |
//! |--------------------------|--------------------------------|-----------------------------------|
//! | `$pageview`              | path                           | posthog.client.ts (auto, on route)|
//! | `user_logged_in`         | method: "email"                | Modal/Login.vue                   |
//! | `user_signed_up`         | —                              | Modal/Login.vue                   |
//! | `user_logged_out`        | —                              | pages/account/index.vue           |
//! | `premium_page_viewed`    | —                              | pages/premium.vue                 |
//! | `store_page_viewed`      | section: "plugins"             | pages/deck/store/plugins.vue      |
//! | `plugin_installed`       | plugin_name                    | pages/deck/store/plugins.vue      |

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri_plugin_better_posthog::{PostHogEvent, PostHogExt};

/// Global analytics enabled flag, synced from app_config on startup and on settings change.
/// Using AtomicBool so Rust-side event capture can check without DB queries.
pub static ANALYTICS_ENABLED: AtomicBool = AtomicBool::new(false);

/// Capture an analytics event only if analytics is enabled.
/// Use this instead of `app_handle.capture_event()` directly.
pub fn capture(app: &tauri::AppHandle, event: AnalyticsEvent) {
    if ANALYTICS_ENABLED.load(Ordering::Relaxed) {
        app.capture_event(event);
    }
}

/// Generic analytics event for PostHog tracking via the Tauri plugin.
/// Uses `PostHogExt::capture_event` to inherit the plugin's persisted identity.
pub struct AnalyticsEvent {
    name: String,
    properties: HashMap<String, serde_json::Value>,
}

impl AnalyticsEvent {
    pub fn new(name: impl Into<String>) -> Self {
        let mut properties = HashMap::new();

        if cfg!(debug_assertions) {
            properties.insert("environment".into(), "development".into());
        } else {
            properties.insert("environment".into(), "production".into());
        }

        Self {
            name: name.into(),
            properties,
        }
    }

    pub fn prop(mut self, key: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        self.properties.insert(key.into(), value.into());
        self
    }
}

impl PostHogEvent for AnalyticsEvent {
    fn name(&self) -> &str {
        &self.name
    }

    fn properties(&self) -> HashMap<String, serde_json::Value> {
        self.properties.clone()
    }
}
