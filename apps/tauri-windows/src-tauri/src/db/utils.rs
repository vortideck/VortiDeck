use crate::db::schema::active_deck;
use crate::db::schema::app_config::dsl::*;
use crate::db::schema::peers;
use crate::db::schema::peers::dsl::*;
// Icons are now handled via the adapter layer
// use crate::icons::remove_icon;
use crate::adapters::icons_commands::IconServiceState;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::result::Error;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::db::models::*;
use crate::db::schema::{
    actions, app_config, buttons, deck_settings, decks, macros, programs, saved_actions, themes,
};
use crate::db::MacroType;
use vortideck_core::core::SerializablePeerInfo;
use vortideck_core::services::execution::ActionReference;
// use tauri::{AppHandle, Manager};
// use tauri::Emitter;
#[cfg(target_os = "windows")]
use vortideck_core::services::system::SystemControl;
// use crate::adapters::icons_commands::IconServiceState;
// Temporary type stubs during migration
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct MacroCommand {
    pub name: String,
    pub macro_chain: Vec<SerializedMacroStep>,
    pub macro_type: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ProgramData {
    pub name: String,
    pub executable: String,
    pub args: Vec<String>,
    pub working_dir: Option<String>,
    pub env_vars: Option<std::collections::HashMap<String, String>>,
    pub timeout_ms: Option<u64>,
    pub show_terminal: bool,
    pub program_type: String,
    pub description: Option<String>,
    pub custom_icon: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SerializedMacroStep {
    pub action: ActionExecutable,
    pub delay: u64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub enum ActionExecutable {
    KeyDown(String),
    KeyUp(String),
    MouseMove { x: i32, y: i32 },
    MouseClick(String),
    ObsAction(String),
    SystemControl(String),
}

// Helper function to convert String to MacroType
pub fn string_to_macro_type(s: &str) -> MacroType {
    match s {
        "RunOnce" => MacroType::RunOnce,
        "RepeatWhilePressing" => MacroType::RepeatWhilePressing,
        "Switch" => MacroType::Switch,
        "Queue" => MacroType::Queue,
        _ => MacroType::RunOnce, // Default fallback
    }
}

/// Helper function to load icon as base64 for relay/WebSocket transmission
/// Returns None if icon doesn't exist or on error
/// Note: icons_base_path should already point to the icons directory (e.g., ".../VortiDeck/icons")
pub fn load_icon_as_base64(icons_base_path: &std::path::Path, button_id: i32) -> Option<String> {
    use base64::{engine::general_purpose, Engine as _};
    use std::fs;

    // icons_base_path already includes "icons", so just join button_id/icon.webp
    let icon_path = icons_base_path
        .join(button_id.to_string())
        .join("icon.webp");

    if !icon_path.exists() {
        return None;
    }

    match fs::read(&icon_path) {
        Ok(file_bytes) => {
            let base64_string = general_purpose::STANDARD.encode(&file_bytes);
            Some(format!("data:image/webp;base64,{}", base64_string))
        }
        Err(e) => {
            eprintln!("Error reading icon file {:?}: {}", icon_path, e);
            None
        }
    }
}

#[derive(Clone)]
pub struct Database {
    pool: Pool<ConnectionManager<SqliteConnection>>,
}

impl Database {
    pub fn new(pool: &Pool<ConnectionManager<SqliteConnection>>) -> Self {
        Database {
            pool: pool.clone(), // Clone the pool here
        }
    }

    /// Get the underlying pool to create a vortideck_core::db::Database
    pub fn get_pool(&self) -> &Pool<ConnectionManager<SqliteConnection>> {
        &self.pool
    }

    /// Create a vortideck_core::db::Database from this wrapper
    pub fn to_core_database(&self) -> vortideck_core::db::Database {
        vortideck_core::db::Database::new(&self.pool)
    }

    pub async fn get_deck_widget_document(
        &self,
        deck_id: i32,
    ) -> Result<vortideck_core::db::DeckWidgetDocument, String> {
        self.to_core_database()
            .get_deck_widget_document(deck_id)
            .await
    }

    // ============================================================================
    // DELEGATED METHODS: These delegate to vortideck_core::db::Database
    // ============================================================================

    pub fn get_peer_count(&self) -> i64 {
        vortideck_core::db::Database::new(&self.pool).get_peer_count()
    }

    pub fn save_shared_secret(
        &self,
        peer_client_id: &str,
        secret_bytes: &[u8; 32],
    ) -> Result<(), diesel::result::Error> {
        vortideck_core::db::Database::new(&self.pool)
            .save_shared_secret(peer_client_id, secret_bytes)
    }

    pub fn load_shared_secret(&self, peer_client_id: &str) -> Option<[u8; 32]> {
        vortideck_core::db::Database::new(&self.pool).load_shared_secret(peer_client_id)
    }

    pub async fn get_button_by_id(
        &self,
        button_id: i32,
    ) -> Result<Option<vortideck_core::db::models::Button>, diesel::result::Error> {
        vortideck_core::db::Database::new(&self.pool)
            .get_button_by_id(button_id)
            .await
    }

    pub async fn get_http_request_by_id(
        &self,
        request_id: i32,
    ) -> Result<vortideck_core::db::models::HttpRequest, diesel::result::Error> {
        vortideck_core::db::Database::new(&self.pool)
            .get_http_request_by_id(request_id)
            .await
            .map_err(|e| diesel::result::Error::NotFound)
    }

    pub async fn get_focus_window_by_id(
        &self,
        window_id: i32,
    ) -> Result<vortideck_core::db::models::FocusWindow, diesel::result::Error> {
        vortideck_core::db::Database::new(&self.pool)
            .get_focus_window_by_id(window_id)
            .await
            .map_err(|e| diesel::result::Error::NotFound)
    }

    pub async fn get_program_by_id(
        &self,
        program_id: i32,
    ) -> Result<Option<vortideck_core::db::models::Program>, String> {
        vortideck_core::db::Database::new(&self.pool)
            .get_program_by_id(program_id)
            .await
    }

    pub fn save_flow(
        &self,
        flow_definition: &vortideck_core::services::flow::executor::FlowDefinition,
    ) -> Result<(), String> {
        vortideck_core::db::Database::new(&self.pool).save_flow(flow_definition)
    }

    pub fn load_flow(
        &self,
        flow_id: &str,
    ) -> Result<vortideck_core::services::flow::executor::FlowDefinition, String> {
        vortideck_core::db::Database::new(&self.pool).load_flow(flow_id)
    }

    // ============================================================================
    // END DELEGATED METHODS
    // ============================================================================

    // New method to add a peer to the database
    pub fn add_peer<'a>(&self, new_peer: NewPeer<'a>) -> Result<(), diesel::result::Error> {
        let mut conn = self.pool.get().expect("Failed to get DB connection");

        // Insert the new peer into the database
        diesel::insert_into(peers::table)
            .values(&new_peer)
            .execute(&mut conn)?;

        Ok(())
    }

    pub async fn get_all_peers(&self) -> Vec<SerializablePeerInfo> {
        let mut conn = self.pool.get().expect("Failed to get DB connection");

        let result = peers.load::<Peer>(&mut conn).expect("Error loading peers");

        result
            .into_iter()
            .map(|peer| SerializablePeerInfo {
                client_id: peer.client_id,
                device_type: peer.device_type,
                device: peer.device,
            })
            .collect()
    }

    pub fn get_peer_by_token(&self, token: &str) -> Option<Peer> {
        let mut conn = self.pool.get().expect("Failed to get DB connection");

        let result = peers
            .filter(device_token.eq(token)) // Filter by the device_token column
            .first::<Peer>(&mut conn)
            .optional() // Return Option<Peer>
            .expect("Error querying peer by token");

        result
    }

    pub fn peer_exists(&self, given_client_id: &str) -> bool {
        let mut conn = self.pool.get().expect("Failed to get DB connection");

        peers
            .filter(client_id.eq(given_client_id)) // Use the distinct parameter name
            .first::<Peer>(&mut conn)
            .is_ok() // Return true if a matching record is found, false otherwise
    }

    pub fn update_peer_token(
        &self,
        other_client_id: &str,
        token: &str,
    ) -> Result<(), diesel::result::Error> {
        let mut conn = self.pool.get().expect("Failed to get DB connection");

        diesel::update(peers::table.filter(client_id.eq(client_id)))
            .set(device_token.eq(token))
            .execute(&mut conn)?;

        Ok(())
    }

    /// Update E2EE keys for existing peer (both mobile_public_key and shared_secret)
    pub fn update_peer_e2ee_keys(
        &self,
        peer_client_id: &str,
        mobile_pub_key_b64: Option<&str>,
        secret_b64: Option<&str>,
    ) -> Result<(), diesel::result::Error> {
        use crate::db::schema::peers::dsl::*;

        let mut conn = self.pool.get().expect("Failed to get DB connection");

        // Build update query dynamically based on what's provided
        if let Some(pub_key) = mobile_pub_key_b64 {
            if let Some(secret) = secret_b64 {
                // Update both
                diesel::update(peers.filter(client_id.eq(peer_client_id)))
                    .set((mobile_public_key.eq(pub_key), shared_secret.eq(secret)))
                    .execute(&mut conn)?;
                println!(
                    "✅ Updated E2EE keys (public key + shared secret) for client: {}",
                    peer_client_id
                );
            } else {
                // Update only public key
                diesel::update(peers.filter(client_id.eq(peer_client_id)))
                    .set(mobile_public_key.eq(pub_key))
                    .execute(&mut conn)?;
                println!(
                    "✅ Updated mobile public key for client: {}",
                    peer_client_id
                );
            }
        } else if let Some(secret) = secret_b64 {
            // Update only shared secret
            diesel::update(peers.filter(client_id.eq(peer_client_id)))
                .set(shared_secret.eq(secret))
                .execute(&mut conn)?;
            println!("✅ Updated shared secret for client: {}", peer_client_id);
        }

        Ok(())
    }

    // ============================================================================
    // E2EE Shared Secret Management (Phase 2: Database Persistence)
    // ============================================================================

    /// Load all shared secrets from database (for restore on startup)
    pub fn load_all_shared_secrets(&self) -> Vec<(String, [u8; 32])> {
        use crate::db::schema::peers::dsl::*;
        use base64::engine::general_purpose::STANDARD as Base64;
        use base64::Engine;

        let mut conn = self.pool.get().expect("Failed to get DB connection");

        let results = peers
            .filter(shared_secret.is_not_null())
            .select((client_id, shared_secret))
            .load::<(String, Option<String>)>(&mut conn)
            .expect("Error loading shared secrets");

        results
            .into_iter()
            .filter_map(|(peer_id, secret_opt)| {
                secret_opt.and_then(|secret_b64| {
                    Base64.decode(&secret_b64).ok().and_then(|bytes| {
                        if bytes.len() == 32 {
                            let mut arr = [0u8; 32];
                            arr.copy_from_slice(&bytes);
                            Some((peer_id, arr))
                        } else {
                            None
                        }
                    })
                })
            })
            .collect()
    }

    /// Delete shared secret from database (when unpaired)
    pub fn delete_shared_secret(&self, peer_client_id: &str) -> Result<(), diesel::result::Error> {
        use crate::db::schema::peers::dsl::*;

        let mut conn = self.pool.get().expect("Failed to get DB connection");

        diesel::update(peers.filter(client_id.eq(peer_client_id)))
            .set(shared_secret.eq(Option::<String>::None))
            .execute(&mut conn)?;

        println!(
            "🗑️ Deleted shared secret from database for client: {}",
            peer_client_id
        );
        Ok(())
    }

    // ============================================================================

    // Get the TLS status from the app_config table
    // Fetch the tls_enabled value for a specific config key
    pub fn get_tls_enabled_for_config_key(&self, config_key: &str) -> Option<bool> {
        let mut conn = self.pool.get().expect("Failed to get DB connection");

        // Query the app_config table for the tls_enabled value where the key matches the provided config_key
        let result = app_config
            .filter(key.eq(config_key)) // Filter by the key (e.g., "general_settings")
            .select(tls_enabled) // Select the tls_enabled field
            .first::<bool>(&mut conn) // Get the first matching record (assuming there is only one)
            .optional() // Return an Option<bool>
            .expect("Error querying TLS status");

        result
    }

    // ============================================================================
    // Desktop ID Management (Relay E2EE Persistence)
    // ============================================================================

    /// Save desktop_id to app_config table (persistent relay identifier)
    /// 🔐 CRITICAL FIX: Desktop ID must persist across restarts for relay E2EE
    pub fn save_desktop_id(&self, desktop_id_value: &str) -> Result<(), diesel::result::Error> {
        use crate::db::schema::app_config::dsl::*;

        let mut conn = self.pool.get().expect("Failed to get DB connection");

        // Update desktop_id for the existing 'general_settings' row
        diesel::update(app_config.filter(key.eq("general_settings")))
            .set(desktop_id.eq(desktop_id_value))
            .execute(&mut conn)?;

        println!("💾 Saved desktop_id to database: {}", desktop_id_value);
        Ok(())
    }

    /// Load desktop_id from app_config table
    pub fn load_desktop_id(&self) -> Option<String> {
        use crate::db::schema::app_config::dsl::*;

        let mut conn = self.pool.get().expect("Failed to get DB connection");

        let result = app_config
            .filter(key.eq("general_settings"))
            .select(desktop_id)
            .first::<Option<String>>(&mut conn)
            .optional()
            .expect("Error querying desktop_id");

        result.flatten()
    }

    // ============================================================================

    ///! Decks and buttons ///

    /// Fetch all decks
    pub async fn get_all_decks(&self) -> Vec<Deck> {
        let mut conn = self.pool.get().expect("Failed to get DB connection");

        decks::dsl::decks
            .load::<Deck>(&mut conn)
            .expect("Error loading decks")
    }
    pub async fn get_all_decks_with_buttons(
        &self,
        include_icons: bool,
        icons_base_path: Option<&std::path::Path>,
    ) -> Result<Vec<DeckWithNestedButtons>, diesel::result::Error> {
        let mut conn = self.pool.get().expect("Failed to get DB connection");

        // Fetch all decks
        let all_decks = decks::dsl::decks.load::<Deck>(&mut conn)?;

        let mut decks_with_buttons = Vec::new();

        for deck in all_decks {
            // Fetch buttons for this deck
            let mut deck_buttons = buttons::dsl::buttons
                .filter(buttons::dsl::deck_id.eq(deck.id))
                .load::<Button>(&mut conn)?;

            // Do not truncate here; UI shows all, server blocks execution beyond limits.

            // Optimized volume processing - get volume once for all buttons

            #[cfg(target_os = "windows")]
            let current_volume = SystemControl::get_volume().ok();
            #[cfg(not(target_os = "windows"))]
            let current_volume: Option<f32> = None;

            if let Some(vol) = current_volume {
                // println!("Current system volume: {}", vol);

                // Use iterator methods instead of a for loop for better performance
                deck_buttons
                    .iter_mut()
                    .filter(|button| {
                        // Only process buttons with the specific volume action
                        button
                            .action
                            .as_ref()
                            .map(|action| {
                                action.contains("system.volume.set")
                                    && !action.contains("system.volume.set:")
                            })
                            .unwrap_or(false)
                    })
                    .for_each(|button| {
                        // println!("Button {} has a volume action: {}", button.id, button.action.as_ref().unwrap());
                        button.background = Some(
                            serde_json::json!({
                                "needs_volume_data": true,
                                "current_action": button.action.as_ref().unwrap(),
                                "value": vol * 100.0
                            })
                            .to_string(),
                        );
                    });
            } else {
                println!("Failed to get system volume");
            }

            // Load icon data as base64 if requested (for relay mode)
            if include_icons {
                if let Some(icons_path) = icons_base_path {
                    for button in deck_buttons.iter_mut() {
                        // Try to load icon for this button ID (regardless of icon field value)
                        if let Some(base64_data) = load_icon_as_base64(icons_path, button.id) {
                            button.icon = Some(base64_data);
                        }
                    }
                }
            }

            let deck_settings = deck_settings::dsl::deck_settings
                .filter(deck_settings::dsl::deck_id.eq(deck.id))
                .first::<DeckSettings>(&mut conn)
                .expect("Error loading deck settings");
            let deck_theme = match deck_settings.active_theme {
                Some(theme_id) => themes::dsl::themes
                    .filter(themes::dsl::id.eq(theme_id))
                    .first::<DeckTheme>(&mut conn)
                    .ok(),
                None => None,
            };
            decks_with_buttons.push(DeckWithNestedButtons {
                id: deck.id,
                name: deck.name,
                buttons: deck_buttons,
                settings: Some(deck_settings),
                theme: deck_theme,
            });
        }

        Ok(decks_with_buttons)
    }

    /// Fetch a single deck by ID with its associated buttons
    pub async fn get_deck_with_buttons(
        &self,
        deck_id: i32,
        include_icons: bool,
        icons_base_path: Option<&std::path::Path>,
    ) -> Option<DeckWithNestedButtons> {
        let mut conn = self.pool.get().expect("Failed to get DB connection");

        // Fetch the deck by its ID
        let deck = decks::dsl::decks
            .filter(decks::dsl::id.eq(deck_id))
            .first::<Deck>(&mut conn)
            .optional()
            .expect("Error querying deck");

        // If the deck exists, fetch its associated buttons
        match deck {
            Some(deck) => {
                let mut buttons = buttons::dsl::buttons
                    .filter(buttons::dsl::deck_id.eq(deck_id))
                    .load::<Button>(&mut conn)
                    .expect("Error loading buttons");

                // Process buttons with volume actions more efficiently
                // Get the current volume once instead of for each button
                #[cfg(target_os = "windows")]
                let current_volume = SystemControl::get_volume().ok();
                #[cfg(not(target_os = "windows"))]
                let current_volume: Option<f32> = None;

                if let Some(vol) = current_volume {
                    // println!("Current system volume: {}", vol);

                    // Use iterator methods instead of a for loop for better performance
                    buttons
                        .iter_mut()
                        .filter(|button| {
                            // Only process buttons with the specific volume action
                            button
                                .action
                                .as_ref()
                                .map(|action| {
                                    action.contains("system.volume.set")
                                        && !action.contains("system.volume.set:")
                                })
                                .unwrap_or(false)
                        })
                        .for_each(|button| {
                            // println!("Button {} has a volume action: {}", button.id, button.action.as_ref().unwrap());
                            button.background = Some(
                                serde_json::json!({
                                    "needs_volume_data": true,
                                    "current_action": button.action.as_ref().unwrap(),
                                    "value": vol * 100.0
                                })
                                .to_string(),
                            );
                        });
                } else {
                    println!("Failed to get system volume");
                }

                // Load icon data as base64 if requested (for relay mode)
                if include_icons {
                    if let Some(icons_path) = icons_base_path {
                        for button in buttons.iter_mut() {
                            // Try to load icon for this button ID (regardless of icon field value)
                            if let Some(base64_data) = load_icon_as_base64(icons_path, button.id) {
                                button.icon = Some(base64_data);
                            }
                        }
                    }
                }

                let deck_settings = deck_settings::dsl::deck_settings
                    .filter(deck_settings::dsl::deck_id.eq(deck_id))
                    .first::<DeckSettings>(&mut conn)
                    .expect("Error loading deck settings");

                let deck_theme = match deck_settings.active_theme {
                    Some(theme_id) => themes::dsl::themes
                        .filter(themes::dsl::id.eq(theme_id))
                        .first::<DeckTheme>(&mut conn)
                        .ok(),
                    None => None,
                };

                Some(DeckWithNestedButtons {
                    id: deck.id,
                    name: deck.name,
                    buttons,
                    settings: Some(deck_settings),
                    theme: deck_theme,
                })
            }
            None => None, // Return None if no deck was found
        }
    }

    pub async fn create_deck_with_buttons(
        &self,
        deck_name: String,
    ) -> Result<DeckWithNestedButtons, diesel::result::Error> {
        let mut conn = self.pool.get().expect("Failed to get DB connection");

        // Create a new deck
        let new_deck = NewDeck { name: &deck_name };

        // Insert the new deck and get its ID
        let deck_id: i32 = diesel::insert_into(decks::dsl::decks)
            .values(&new_deck)
            .returning(decks::dsl::id)
            .get_result(&mut conn)?;

        let new_deck_settings = self.create_default_deck_settings(deck_id).unwrap();

        println!("Inserted Example Deck Settings with ID: {}", deck_id);

        // Parse the grid string to extract rows and columns, get from settings
        let grid_parts: Vec<&str> = new_deck_settings.grid.split(',').collect();
        if grid_parts.len() == 2 {
            let grid_rows: usize = grid_parts[0].parse().expect("Invalid number of rows");
            let grid_columns: usize = grid_parts[1].parse().expect("Invalid number of columns");

            let total_buttons = 9 * 11;

            // Generate buttons based on the grid
            let buttons_to_insert: Vec<NewButton> = (1..=total_buttons)
                .map(|i| NewButton {
                    deck_id,
                    label: format!("Button {}", i),
                    icon: None,
                    action: None,
                    background: Some(format!("button{}_bg.png", i)),
                    position: i as i32,
                    mode: None,
                    execution_mode: Some("run_once".to_string()),
                    execution_config: None,
                })
                .collect();

            // Insert the buttons into the database
            diesel::insert_into(buttons::dsl::buttons)
                .values(&buttons_to_insert)
                .execute(&mut conn)?;
        } else {
            return Err(diesel::result::Error::NotFound);
        }

        // Retrieve the created deck with buttons
        let deck = decks::dsl::decks
            .filter(decks::dsl::id.eq(deck_id))
            .first::<Deck>(&mut conn)?;

        let buttons = buttons::dsl::buttons
            .filter(buttons::dsl::deck_id.eq(deck_id))
            .load::<Button>(&mut conn)?;

        let deck_settings = deck_settings::dsl::deck_settings
            .filter(deck_settings::dsl::deck_id.eq(deck_id))
            .first::<DeckSettings>(&mut conn)
            .expect("Error loading deck settings");

        let deck_theme = match deck_settings.active_theme {
            Some(theme_id) => themes::dsl::themes
                .filter(themes::dsl::id.eq(theme_id))
                .first::<DeckTheme>(&mut conn)
                .ok(),
            None => None,
        };

        Ok(DeckWithNestedButtons {
            id: deck.id,
            name: deck.name,
            buttons,
            settings: Some(deck_settings),
            theme: deck_theme,
        })
    }

    /// Update an existing deck
    pub async fn update_deck<'a>(
        &self,
        deck_id: i32,
        updated_deck: UpdateDeck<'a>,
    ) -> Result<Deck, Error> {
        let mut conn = self.pool.get().expect("Failed to get DB connection");

        diesel::update(decks::dsl::decks)
            .filter(decks::dsl::id.eq(deck_id))
            .set(&updated_deck)
            .get_result::<Deck>(&mut conn)
    }

    /// Delete a deck by ID and its buttons
    /// Delete a deck by ID and its buttons
    pub async fn delete_deck<'a>(&self, deck_id: i32) -> usize {
        let mut conn = self.pool.get().expect("Failed to get DB connection");

        // Start by deleting all associated buttons
        let deleted_buttons = diesel::delete(crate::db::schema::buttons::dsl::buttons)
            .filter(crate::db::schema::buttons::dsl::deck_id.eq(deck_id))
            .execute(&mut conn)
            .expect("Error deleting buttons");

        // Now delete the deck itself
        let deleted_deck = diesel::delete(crate::db::schema::decks::dsl::decks)
            .filter(crate::db::schema::decks::dsl::id.eq(deck_id))
            .execute(&mut conn)
            .expect("Error deleting deck");

        let deleted_deck_settings =
            diesel::delete(crate::db::schema::deck_settings::dsl::deck_settings)
                .filter(crate::db::schema::deck_settings::dsl::deck_id.eq(deck_id))
                .execute(&mut conn)
                .expect("Error deleting deck settings");

        // Return the number of deleted rows (deck and buttons)
        deleted_buttons + deleted_deck
    }

    /// Add a new button
    pub async fn add_button(&self, new_button: NewButton) -> Result<(), diesel::result::Error> {
        let mut conn = self.pool.get().expect("Failed to get DB connection");

        diesel::insert_into(buttons::table)
            .values(&new_button)
            .execute(&mut conn)?;

        Ok(())
    }

    /// Update an existing button
    pub async fn update_button<'a>(
        &self,
        button_id: i32,
        updated_button: UpdateButton<'a>,
    ) -> Result<Button, Error> {
        let mut conn = self.pool.get().expect("Failed to get DB connection");

        diesel::update(buttons::dsl::buttons.filter(buttons::dsl::id.eq(button_id)))
            .set(&updated_button)
            .get_result::<Button>(&mut conn)
    }

    /// Delete a button by ID
    pub async fn delete_button(&self, button_id: i32) -> usize {
        let mut conn = self.pool.get().expect("Failed to get DB connection");

        diesel::delete(buttons::dsl::buttons.filter(buttons::dsl::id.eq(button_id)))
            .execute(&mut conn)
            .expect("Error deleting button")
    }

    /// Swap positions of two buttons in the database
    pub async fn swap_buttons(
        &self,
        swapping_button_id: i32,
        swapped_button_id: i32,
        from: i32,
        to: i32,
    ) -> Result<String, String> {
        let mut conn = self.pool.get().expect("Failed to get DB connection");

        // Proceed with the swap inside a transaction to ensure atomicity
        conn.transaction::<_, diesel::result::Error, _>(|conn| {
            // Step 1: Move the swapping button to the target position
            diesel::update(buttons::dsl::buttons.filter(buttons::dsl::id.eq(swapping_button_id)))
                .set(buttons::dsl::position.eq(to))
                .execute(conn)?;

            // Step 2: Move the swapped button to the original position
            diesel::update(buttons::dsl::buttons.filter(buttons::dsl::id.eq(swapped_button_id)))
                .set(buttons::dsl::position.eq(from))
                .execute(conn)?;

            Ok(())
        })
        .map(|_| "Successfully swapped button positions".to_string())
        .map_err(|err| format!("Failed to swap button positions: {}", err))
    }

    pub async fn reset_button(&self, button_id: i32) -> Result<String, String> {
        let mut conn = self.pool.get().expect("Failed to get DB connection");

        //TODO
        // // Step 1: Check if the button has an icon, and remove it if it exists
        // let icon_path = ''
        // let app_data_dir = app_handle.path().data_dir().expect("Unable to find data dir");
        // let save_dir = app_data_dir.join("VortiDeck").join("icons").join(button_id);

        // // If the icon exists, delete it
        // if let Some(icon_path) = icon_path {
        //     if Path::new(&icon_path).exists() {
        //         fs::remove_file(icon_path).map_err(|e| format!("Failed to remove icon file: {}", e))?;
        //     }
        // }

        // Start a database transaction for atomicity
        conn.transaction::<_, diesel::result::Error, _>(|conn| {
            // Perform the update operation to reset the button without affecting position
            diesel::update(buttons::dsl::buttons.filter(buttons::dsl::id.eq(button_id)))
                .set((
                    buttons::dsl::label.eq("".to_string()), // Reset label to "Default"
                    buttons::dsl::icon.eq(None::<String>),  // Reset icon to None
                    buttons::dsl::background.eq(None::<String>), // Reset background to None
                    buttons::dsl::action.eq(None::<String>), // Reset action to None
                    buttons::dsl::icon_fit.eq("cover"),
                ))
                .execute(conn)?;

            Ok(())
        })
        .map_err(|e| format!("Failed to reset button: {}", e))?;

        Ok("Button reset successfully.".to_string())
    }

    pub async fn get_active_deck(&self) -> Option<ActiveDeck> {
        let mut conn = self.pool.get().expect("Failed to get DB connection");

        // Fetch the first active deck (there should only be one)
        let active_deck = active_deck::dsl::active_deck
            .first::<ActiveDeck>(&mut conn)
            .optional() // Return an Option<ActiveDeck> (None if not found)
            .expect("Error fetching active deck");

        active_deck
    }

    pub async fn get_active_deck_with_buttons(
        &self,
        include_icons: bool,
        icons_base_path: Option<&std::path::Path>,
    ) -> Option<DeckWithNestedButtons> {
        // Reuse get_active_deck to fetch the active deck
        let active_deck = self.get_active_deck().await;

        match active_deck {
            Some(active_deck) => {
                // Reuse get_deck_with_buttons to fetch the deck and its buttons using the deck_id from active_deck
                log::trace!("Loading buttons for active deck {}", active_deck.deck_id);
                self.get_deck_with_buttons(active_deck.deck_id, include_icons, icons_base_path)
                    .await
            }
            None => None, // Return None if no active deck is found
        }
    }

    /// Update the active deck in the database
    pub async fn update_active_deck(&self, deck_id: u16) -> Result<(), Error> {
        let mut conn = self.pool.get().expect("Failed to get DB connection");

        // Convert deck_id to i32 since Diesel expects it for SQL INTEGER
        let deck_id = deck_id as i32;

        // Get the first active deck record to update
        if let Some(active_deck) = active_deck::dsl::active_deck
            .first::<ActiveDeck>(&mut conn)
            .optional()?
        {
            // Update the existing active deck record
            diesel::update(
                active_deck::dsl::active_deck.filter(active_deck::dsl::id.eq(active_deck.id)),
            )
            .set(active_deck::dsl::deck_id.eq(deck_id))
            .execute(&mut conn)?;
        } else {
            // If no active deck exists, create one
            diesel::insert_into(active_deck::dsl::active_deck)
                .values(active_deck::dsl::deck_id.eq(deck_id))
                .execute(&mut conn)?;
        }

        Ok(())
    }

    pub fn add_macro<'a>(
        &self,
        macro_name: &'a str,
        serialized_chain: &'a str, // Ensure this is the already serialized string
        macro_type: &'a MacroType, // Same for macro_type, it's a string reference
    ) -> Result<(), diesel::result::Error> {
        let mut conn = self.pool.get().expect("Failed to get DB connection");

        // Ensure `serialized_chain` is already a valid JSON string
        // If you need to serialize it inside this function, remove the parameter and serialize it
        // but in this case, it's passed as already serialized string

        // Insert the new macro into the database with serialized chain

        println!("name: {}", macro_name);
        diesel::insert_into(macros::table)
            .values(&NewMacro {
                name: macro_name,                         // Field `name` matches with table's column
                macro_chain: serialized_chain, // Serialized macro chain (already JSON string)
                macro_type: &format!("{:?}", macro_type), // Serialized string representation of macro type
            })
            .execute(&mut conn)?;

        Ok(())
    }

    // Get all macros from the database
    pub async fn get_all_macros(&self) -> Result<Vec<Macro>, diesel::result::Error> {
        let mut conn = self.pool.get().expect("Failed to get DB connection");

        macros::dsl::macros
            .load::<Macro>(&mut conn) // Get all macros from the macros table
            .map_err(|e| {
                // Handle any errors that occur during the load operation
                e
            })
    }

    // Count total number of macros stored in the database
    pub async fn count_stored_macros(&self) -> Result<i64, diesel::result::Error> {
        let mut conn = self.pool.get().expect("Failed to get DB connection");

        macros::dsl::macros
            .count()
            .get_result(&mut conn)
            .map_err(|e| {
                eprintln!("Error counting stored macros: {}", e);
                e
            })
    }

    // Get a specific macro by ID
    pub async fn get_macro_with_chain(&self, macro_id: i32) -> Option<Macro> {
        let mut conn = self.pool.get().expect("Failed to get DB connection");

        // Get the macro from the database
        let macro_data = macros::table
            .filter(macros::dsl::id.eq(macro_id))
            .first::<Macro>(&mut conn)
            .optional()
            .expect("Error querying macro");

        macro_data
    }

    /// Update an existing button
    pub fn update_macro_chain(
        &self,
        macro_id: i32,
        updated_chain: Vec<SerializedMacroStep>,
    ) -> Result<(), diesel::result::Error> {
        let mut conn = self.pool.get().expect("Failed to get DB connection");

        // Serialize the updated chain to JSON
        let serialized_chain =
            serde_json::to_string(&updated_chain).expect("Error serializing updated chain");

        // Update the macro with the new chain
        diesel::update(macros::table.filter(macros::dsl::id.eq(macro_id)))
            .set(macros::dsl::macro_chain.eq(serialized_chain))
            .execute(&mut conn)?;

        Ok(())
    }

    pub fn update_macro<'a>(
        &self,
        macro_id: i32,
        macro_name: &'a str,
        macro_chain: &'a str,
        macro_type: &'a MacroType,
    ) -> Result<(), diesel::result::Error> {
        let mut conn = self.pool.get().expect("Failed to get DB connection");
        let macro_type_str = format!("{:?}", macro_type);
        let update_macro = UpdateMacro {
            macro_chain: Some(macro_chain),
            name: Some(macro_name),
            macro_type: Some(&macro_type_str),
        };

        diesel::update(macros::table.filter(macros::dsl::id.eq(macro_id)))
            .set(&update_macro)
            .execute(&mut conn)?;

        Ok(())
    }

    // Delete a macro by ID
    pub async fn remove_macro(&self, macro_id: i32) -> Result<usize, diesel::result::Error> {
        let mut conn = self.pool.get().expect("Failed to get DB connection");

        // Perform the delete operation and return the result
        diesel::delete(macros::dsl::macros.filter(macros::dsl::id.eq(macro_id)))
            .execute(&mut conn)
            .map_err(|err| {
                eprintln!("Error during macro deletion: {}", err);
                err
            })

        //TODO: remove actions from associated buttons with macros
    }
    pub async fn update_deck_settings(
        &self,
        deck_id: i32,
        updated_settings: DeckSettings,
    ) -> Result<Option<DeckWithNestedButtons>, diesel::result::Error> {
        let mut conn = self.pool.get().expect("Failed to get DB connection");

        // Get current deck settings
        let current_settings = deck_settings::dsl::deck_settings
            .filter(deck_settings::dsl::deck_id.eq(deck_id))
            .first::<DeckSettings>(&mut conn)?;

        // Count how many buttons the deck already has
        let button_count = buttons::dsl::buttons
            .filter(buttons::dsl::deck_id.eq(deck_id))
            .count()
            .get_result::<i64>(&mut conn)?;

        // Calculate buttons needed for new grid
        let grid_parts: Vec<&str> = updated_settings.grid.split(',').collect();
        let grid_rows: usize = grid_parts[0].parse().expect("Invalid number of rows");
        let grid_columns: usize = grid_parts[1].parse().expect("Invalid number of columns");
        let total_buttons_needed = grid_rows * grid_columns;

        // Update the deck settings
        diesel::update(
            deck_settings::dsl::deck_settings.filter(deck_settings::dsl::deck_id.eq(deck_id)),
        )
        .set(&updated_settings)
        .execute(&mut conn)?;

        // Only update grid and buttons if grid changed
        if current_settings.grid != updated_settings.grid {
            self.update_grid_and_generate_buttons(deck_id, &updated_settings.grid, &mut conn)?;
        }

        // Always return the updated deck
        let deck = self.get_deck_with_buttons(deck_id, false, None).await;
        Ok(deck)
    }

    // Separate function for updating just the grid
    pub async fn update_deck_grid(
        &self,
        deck_id: i32,
        new_grid: Grid,
    ) -> Result<(), diesel::result::Error> {
        let mut conn = self.pool.get().expect("Failed to get DB connection");

        // Start transaction
        conn.transaction(|conn| {
            // Update only the grid setting
            diesel::update(
                deck_settings::dsl::deck_settings.filter(deck_settings::dsl::deck_id.eq(deck_id)),
            )
            .set(deck_settings::dsl::grid.eq(new_grid.to_string()))
            .execute(conn)?;

            // Handle button generation based on new grid
            self.update_grid_and_generate_buttons(deck_id, &new_grid.to_string(), conn)
        })
    }

    fn update_grid_and_generate_buttons(
        &self,
        deck_id: i32,
        grid: &str,
        conn: &mut diesel::SqliteConnection,
    ) -> Result<(), diesel::result::Error> {
        let grid_parts: Vec<&str> = grid.split(',').collect();
        let grid_rows: usize = grid_parts[0].parse().expect("Invalid number of rows");
        let grid_columns: usize = grid_parts[1].parse().expect("Invalid number of columns");
        let total_buttons_needed = grid_rows * grid_columns;

        // Get current number of buttons
        let current_button_count: i64 = buttons::dsl::buttons
            .filter(buttons::dsl::deck_id.eq(deck_id))
            .count()
            .get_result(conn)?;

        // Only add buttons if we need more
        if current_button_count < total_buttons_needed as i64 {
            let buttons_to_add = total_buttons_needed as i64 - current_button_count;

            println!("generating buttons, count: {}", buttons_to_add);

            // Generate only the additional buttons needed
            let buttons_to_insert: Vec<NewButton> = ((current_button_count + 1)
                ..=(current_button_count + buttons_to_add))
                .map(|i| NewButton {
                    deck_id,
                    label: format!("Button {}", i),
                    icon: None,
                    action: None,
                    background: Some(format!("button{}_bg.png", i)),
                    position: i as i32,
                    mode: None,
                    execution_mode: Some("run_once".to_string()),
                    execution_config: None,
                })
                .collect();

            // Insert only the new buttons
            diesel::insert_into(buttons::table)
                .values(&buttons_to_insert)
                .execute(conn)?;

            println!("inserted buttons");
        }

        Ok(())
    }

    pub fn create_default_deck_settings(
        &self,
        deck_id: i32,
    ) -> Result<NewDeckSettings, diesel::result::Error> {
        let mut conn = self.pool.get().expect("Failed to get DB connection");

        let new_deck_settings = NewDeckSettings {
            button_spacing: 10.0,
            grid_margin: 10.0,
            group_buttons: false,
            maximize_button_space: false,
            grid: "3,5".to_string(),
            deck_id,
            buttons_padding: 4.0,
            sides_enabled: false,
            orientation_sync_enabled: true,
            active_theme: None,
        };

        diesel::insert_into(deck_settings::dsl::deck_settings)
            .values(&new_deck_settings)
            .execute(&mut conn)
            .expect("Error inserting example deck settings");

        Ok(new_deck_settings)
    }

    pub async fn get_deck_settings(
        &self,
        deck_id: i32,
    ) -> Result<DeckSettings, diesel::result::Error> {
        let mut conn = self.pool.get().expect("Failed to get DB connection");

        deck_settings::dsl::deck_settings
            .filter(deck_settings::dsl::deck_id.eq(deck_id))
            .first::<DeckSettings>(&mut conn)
    }

    // // Set the TLS status in the app_config table
    // pub fn set_tls_enabled(&self, tls_enabled: bool) -> Result<(), diesel::result::Error> {
    //     let mut conn = self.pool.get().expect("Failed to get DB connection");

    //     // Check if the key already exists (assuming key is "tls_status")
    //     let existing_config: Option<AppConfig> = app_config
    //         .filter(key.eq("tls_status"))
    //         .first::<AppConfig>(&mut conn)
    //         .optional()
    //         .expect("Error querying existing config");

    //     // If the configuration exists, update it; otherwise, insert a new one
    //     match existing_config {
    //         Some(_) => {
    //             // Update the tls_enabled field for the existing key
    //             diesel::update(app_config.filter(key.eq("tls_status")))
    //                 .set(tls_enabled.eq(tls_enabled))
    //                 .execute(&mut conn)?;
    //         }
    //         None => {
    //             // Insert a new entry if the key doesn't exist
    //             diesel::insert_into(app_config::table)  // Corrected usage
    //                 .values(NewAppConfig {
    //                     key: "tls_status",
    //                     tls_enabled,  // `tls_enabled` should be a `bool` here
    //                     api_url: Some("default_api_url"),  // Add missing fields
    //                     language: Some("en"),              // Add missing fields
    //                     log_level: Some("info"),           // Add missing fields
    //                     // Fill in any other required fields
    //                 })
    //                 .execute(&mut conn)?;
    //         }
    //     }

    //     Ok(())
    // }

    pub fn list_flows(
        &self,
    ) -> Result<Vec<vortideck_core::services::flow::executor::FlowSummary>, String> {
        use crate::db::models::Flow;
        use crate::db::schema::flows::dsl::*;

        let mut conn = self.pool.get().map_err(|e| e.to_string())?;

        let db_flows = flows
            .order(updated_at.desc())
            .load::<Flow>(&mut conn)
            .map_err(|e| format!("Failed to load flows: {}", e))?;

        let flow_summaries = db_flows
            .into_iter()
            .map(|flow| {
                let flow_tags = flow
                    .tags
                    .as_ref()
                    .and_then(|t| serde_json::from_str::<Vec<String>>(t).ok())
                    .unwrap_or_default();

                vortideck_core::services::flow::executor::FlowSummary {
                    id: flow.id,
                    name: flow.name,
                    description: flow.description,
                    created_at: chrono::DateTime::parse_from_rfc3339(&flow.created_at)
                        .map_err(|_| "Invalid date format".to_string())
                        .unwrap_or_else(|_| chrono::Utc::now().into())
                        .with_timezone(&chrono::Utc),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&flow.updated_at)
                        .map_err(|_| "Invalid date format".to_string())
                        .unwrap_or_else(|_| chrono::Utc::now().into())
                        .with_timezone(&chrono::Utc),
                    status: None,
                    tags: flow_tags,
                }
            })
            .collect();

        Ok(flow_summaries)
    }

    pub fn delete_flow(&self, flow_id: &str) -> Result<(), String> {
        use crate::db::schema::flows::dsl::*;

        let mut conn = self.pool.get().map_err(|e| e.to_string())?;

        diesel::delete(flows.filter(id.eq(flow_id)))
            .execute(&mut conn)
            .map_err(|e| format!("Failed to delete flow: {}", e))?;

        Ok(())
    }

    pub fn assign_flow_to_button(&self, button_id: i32, flow_id: &str) -> Result<(), String> {
        use crate::db::models::Action;
        use crate::db::schema::buttons::dsl::*;

        let mut conn = self.pool.get().map_err(|e| e.to_string())?;

        // Create flow action
        let flow_action = Action::FlowId(flow_id.to_string());
        let action_json = serde_json::to_string(&flow_action)
            .map_err(|e| format!("Failed to serialize flow action: {}", e))?;

        // Update button with flow action
        diesel::update(buttons.filter(id.eq(button_id)))
            .set(action.eq(action_json))
            .execute(&mut conn)
            .map_err(|e| format!("Failed to assign flow to button: {}", e))?;

        Ok(())
    }

    pub fn get_flow_by_button(
        &self,
        button_id: i32,
    ) -> Result<Option<vortideck_core::services::flow::executor::FlowDefinition>, String> {
        use crate::db::models::{Action, Button};
        use crate::db::schema::buttons::dsl::*;

        let mut conn = self.pool.get().map_err(|e| e.to_string())?;

        // Get button
        let button = buttons
            .filter(id.eq(button_id))
            .first::<Button>(&mut conn)
            .map_err(|e| format!("Button not found: {}", e))?;

        // Check if button has a flow action
        if let Some(action_str) = button.action {
            let button_action: Action = serde_json::from_str(&action_str)
                .map_err(|e| format!("Failed to deserialize action: {}", e))?;

            match button_action {
                Action::FlowId(flow_id) => {
                    // Load the flow
                    match self.load_flow(&flow_id) {
                        Ok(flow) => Ok(Some(flow)),
                        Err(_) => Ok(None), // Flow not found
                    }
                }
                _ => Ok(None), // Not a flow action
            }
        } else {
            Ok(None) // No action assigned
        }
    }
}
// use tauri::State;

use crate::db::db::DbPool;

#[tauri::command]
pub async fn get_all_decks(state: State<'_, DbPool>) -> Result<Vec<DeckWithNestedButtons>, String> {
    let db = Database::new(&state);

    // Spawn a new thread to handle the database operation
    // Don't include icons for desktop frontend commands (icons accessed via file paths)
    match db.get_all_decks_with_buttons(false, None).await {
        Ok(decks_with_buttons) => Ok(decks_with_buttons),
        Err(err) => Err(format!("Failed to retrieve decks: {}", err)),
    }
}

#[tauri::command]
pub async fn get_feature_limit(feature_id: String) -> Result<serde_json::Value, String> {
    // Use the existing JWT features implementation
    let provider = vortideck_core::features::jwt_features::get_jwt_provider();
    let limit = provider.get_effective_feature_limit(&feature_id);
    let is_logged_in = provider.claims.is_some(); // Check if JWT claims exist

    Ok(serde_json::json!({
        "feature": feature_id,
        "limit": limit,
        "is_logged_in": is_logged_in
    }))
}

// Get all feature limits at once (used for mobile server_info)
pub fn get_all_feature_limits() -> serde_json::Value {
    let provider = vortideck_core::features::jwt_features::get_jwt_provider();

    // Define all feature IDs we need limits for
    let feature_ids = vec![
        "ui_buttons",
        "saved_decks",
        "plugin_count",
        "macro_recording",
        "premium_themes",
        "custom_overlay",
        "overlay_elements",
        "saved_overlays",
    ];

    let mut limits = serde_json::Map::new();
    for feature_id in feature_ids {
        let limit = provider.get_effective_feature_limit(feature_id);
        limits.insert(feature_id.to_string(), serde_json::json!(limit));
    }

    serde_json::Value::Object(limits)
}

#[tauri::command]
pub async fn remove_deck(state: State<'_, DbPool>, deck_id: i32) -> Result<String, String> {
    let db = Database::new(&state);

    // Delete the deck using the delete_deck method and capture the number of affected rows
    let deleted_rows = db.delete_deck(deck_id).await;

    // Check if any rows were deleted
    if deleted_rows > 0 {
        Ok(format!("Successfully removed deck with ID: {}", deck_id))
    } else {
        Err(format!("No deck found with ID: {}", deck_id))
    }
}

#[tauri::command]
pub async fn get_active_deck_id(state: State<'_, DbPool>) -> Result<Option<ActiveDeck>, String> {
    let db = Database::new(&state);

    // Fetch the active deck using the get_active_deck method
    match db.get_active_deck().await {
        Some(active_deck) => Ok(Some(active_deck)),
        None => Err("No active deck found".to_string()),
    }
}

#[tauri::command]
pub async fn update_active_deck(
    state: tauri::State<'_, DbPool>,
    deck_id: u16, // Accept deck_id directly from the frontend
) -> Result<String, String> {
    let db = Database::new(&state);

    println!("Updating active deck to: {}", deck_id);

    match db.update_active_deck(deck_id).await {
        Ok(_) => Ok(format!(
            "Successfully updated the active deck to ID {}",
            deck_id
        )),
        Err(err) => Err(format!("Failed to update active deck: {}", err)),
    }
}

#[tauri::command]
pub async fn change_button_position(
    state: tauri::State<'_, DbPool>,
    app_handle: AppHandle,
    swapping_button_id: i32,
    swapped_button_id: i32,
    from: i32,
    to: i32,
) -> Result<String, String> {
    let db = Database::new(&state);

    // Log the inputs to confirm the function is being invoked
    println!(
        "Swapping button positions: swapping_button_id={}, swapped_button_id={}, from={}, to={}",
        swapping_button_id, swapped_button_id, from, to
    );

    // First, get the deck_id from one of the buttons
    let deck_id = match db.get_button_by_id(swapping_button_id).await {
        Ok(Some(button)) => button.deck_id,
        Ok(None) => {
            return Err("Could not find button to get deck_id".to_string());
        }
        Err(e) => {
            return Err(format!("Database error getting button: {}", e));
        }
    };

    // Attempt to swap the button positions
    match db
        .swap_buttons(swapping_button_id, swapped_button_id, from, to)
        .await
    {
        Ok(success_message) => {
            println!("Swap Successful: {}", success_message); // Log success message

            // Get the updated deck to broadcast position changes (with icons for relay mode)
            let icons_base_path = app_handle
                .try_state::<IconServiceState>()
                .map(|icon_state| icon_state.service.get_icons_base_dir());

            match db
                .get_deck_with_buttons(deck_id, true, icons_base_path.as_deref())
                .await
            {
                Some(deck) => {
                    // Broadcast deck update via StateManager
                    use tauri::Manager;
                    if let Some(state_manager) =
                        app_handle.try_state::<std::sync::Arc<vortideck_core::core::StateManager>>()
                    {
                        let update_message = serde_json::json!({
                            "action": "deck_updated",
                            "deck_id": deck_id,
                            "deck": deck
                        });
                        crate::broadcast_helper::broadcast_to_all_clients(
                            &app_handle,
                            &state_manager,
                            update_message,
                        )
                        .await;
                        println!("📡 Button positions updated and deck broadcasted to all clients");
                    } else {
                        println!(
                            "📡 Button positions updated (StateManager not available for broadcast)"
                        );
                    }
                }
                None => {
                    eprintln!(
                        "Could not find deck with id {} after position swap",
                        deck_id
                    );
                }
            }

            Ok(success_message)
        }
        Err(err) => {
            eprintln!("Error: {}", err); // Log the error
            Err(format!("Failed to swap button positions: {}", err))
        }
    }
}

#[tauri::command]
pub async fn get_active_deck(
    state: tauri::State<'_, DbPool>,
) -> Result<DeckWithNestedButtons, String> {
    let db = Database::new(&state);

    // Attempt to fetch the active deck with its associated buttons
    // Don't include icons for desktop frontend commands (icons accessed via file paths)
    match db.get_active_deck_with_buttons(false, None).await {
        Some(deck_with_buttons) => Ok(deck_with_buttons), // Return the deck with buttons if found
        None => Err("No active deck found".to_string()), // Return an error if no active deck is found
    }
}

#[tauri::command]
pub async fn create_new_deck(
    state: tauri::State<'_, DbPool>,
    app_handle: AppHandle,
    deck_name: String, // game: String
) -> Result<DeckWithNestedButtons, String> {
    let db = Database::new(&state);

    // 🔒 STORAGE VALIDATION: Check saved_decks limit before creating
    use std::sync::Arc;
    use vortideck_core::core::StateManager;

    let state_manager = app_handle.state::<Arc<StateManager>>();

    // Count current decks
    let current_count = db.get_all_decks().await.len() as u32;

    // Validate storage limit
    let storage_validation = state_manager
        .validate_storage_feature("saved_decks", current_count, "deck creation")
        .await;

    if !storage_validation.available {
        return Err(storage_validation
            .message
            .unwrap_or_else(|| "Deck storage limit reached!".to_string()));
    }

    // Attempt to create the new deck and return the created deck
    match db.create_deck_with_buttons(deck_name).await {
        Ok(deck) => {
            log::info!("New deck created successfully: {:?}", deck);

            // 📊 Track deck creation
            crate::analytics::capture(
                &app_handle,
                crate::analytics::AnalyticsEvent::new("deck_created")
                    .prop("deck_name", deck.name.clone()),
            );

            Ok(deck) // Return the new deck
        }
        Err(err) => {
            log::error!("Error: {}", err); // Log the error
            Err(format!("Failed to create new deck: {}", err))
        }
    }
}

// ✅ MIGRATED TO NEW ARCHITECTURE: Command moved to adapters/macro_commands.rs
// #[tauri::command]
pub async fn save_macro_to_db(
    state: State<'_, DbPool>,
    macro_command: MacroCommand,
) -> Result<(), String> {
    // Access the macro command fields
    let name = &macro_command.name; // Reference to a String
    let macro_chain = &macro_command.macro_chain; // A Vec<MacroStep>

    // Directly use the macro_type from the command (no deserialization needed)
    let macro_type = &macro_command.macro_type;

    // Log the macro type
    match macro_type.as_str() {
        "RunOnce" => println!("Saving a 'Run Once' macro"),
        "RepeatWhilePressing" => println!("Saving a 'Repeat While Pressing' macro"),
        "Switch" => println!("Saving a 'Switch' macro"),
        "Queue" => println!("Saving a 'Queue' macro"),
        _ => println!("Saving a macro with unknown type: {}", macro_type),
    }

    // Serialize macro_chain to JSON string
    let serialized_chain = serde_json::to_string(macro_chain)
        .map_err(|e| format!("Error serializing macro chain: {}", e))?;

    // println!("chain: {}", serialized_chain);

    // Connect to the database and save the macro
    let db = Database::new(&state); // Assuming it's synchronous, no await needed
    let macro_type_enum = string_to_macro_type(&macro_type);
    db.add_macro(&name, &serialized_chain, &macro_type_enum)
        .map_err(|e| format!("Database error: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn update_macro(
    app_handle: tauri::AppHandle,
    state: State<'_, DbPool>,
    macro_id: i32,
    macro_name: String,
    macro_type: String,
    macro_chain: String,
) -> Result<serde_json::Value, serde_json::Value> {
    // // Access the macro command fields
    // let name = &macro_command.name;
    // let macro_chain = &macro_command.macro_chain;
    // let macro_type = &macro_command.macro_type;

    // Log the macro type
    match macro_type.as_str() {
        "RunOnce" => println!("Updating to 'Run Once' macro"),
        "RepeatWhilePressing" => println!("Updating to 'Repeat While Pressing' macro"),
        "Switch" => println!("Updating to 'Switch' macro"),
        "Queue" => println!("Updating to 'Queue' macro"),
        _ => println!("Updating to macro with unknown type: {}", macro_type),
    }

    // macro_chain is already a JSON string from frontend, no need to serialize again
    let serialized_chain = macro_chain;

    // Connect to the database and update the macro
    let db = Database::new(&state);
    let macro_type_enum = string_to_macro_type(&macro_type);
    match db.update_macro(macro_id, &macro_name, &serialized_chain, &macro_type_enum) {
        Ok(_) => {
            // Emit event to frontend that macro was updated
            let _ = app_handle.emit("macro_updated", ());

            Ok(serde_json::json!({
                "status": "success",
                "message": "Successfully updated macro"
            }))
        }
        Err(err) => Err(serde_json::json!({
            "status": "error",
            "message": format!("Failed to update macro: {}", err)
        })),
    }
}

#[tauri::command]
pub async fn get_all_macros(state: State<'_, DbPool>) -> Result<Vec<Macro>, String> {
    let db = Database::new(&state);

    // Spawn a new thread to handle the database operation
    match db.get_all_macros().await {
        Ok(macros) => Ok(macros),
        Err(err) => Err(format!("Failed to retrieve macros: {}", err)),
    }
}

#[tauri::command]
pub async fn remove_macro(
    app_handle: AppHandle,
    state: State<'_, DbPool>,
    macro_id: i32, // We assume you're passing an ID for the macro to remove
) -> Result<String, String> {
    let db = Database::new(&state);

    // Call the `remove_macro` method and capture the result (number of affected rows)
    match db.remove_macro(macro_id).await {
        Ok(affected_rows) if affected_rows > 0 => {
            if crate::commands::context_menu_commands::clear_orphaned_items(
                "MacroId",
                &macro_id.to_string(),
            ) {
                let _ = app_handle.emit("context-menu-updated", ());
            }
            Ok(format!("Successfully removed {} macro(s)", affected_rows))
        }
        Ok(_) => Err("No macros were removed.".to_string()),
        Err(err) => Err(format!("Error removing macro: {}", err)),
    }
}

#[tauri::command]
pub async fn set_button_action(
    app_handle: AppHandle,           // Add app_handle for StateManager access
    state: tauri::State<'_, DbPool>, // Access the database pool
    button_id: i32,                  // Button ID to update
    action_reference: ActionReference,
    label: Option<String>,
) -> Result<String, String> {
    let action = action_reference.to_database_action()?;
    let mut conn = state.get().expect("Failed to get DB connection");

    // Serialize the action (this is done by `set_action` method)
    let mut button = buttons::dsl::buttons
        .filter(buttons::dsl::id.eq(button_id))
        .first::<Button>(&mut conn)
        .map_err(|e| format!("Error finding button: {}", e))?;

    let deck_id = button.deck_id; // Store deck_id for broadcast

    // Set the action (serializing Action)
    button
        .set_action(action)
        .map_err(|e| format!("Error setting button action: {}", e))?;

    // Use UpdateButton to perform the update
    let update_button = UpdateButton {
        label: label
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .or(Some(&button.label)),
        icon: button.icon.as_deref(),
        background: button.background.as_deref(),
        position: Some(button.position),
        action: button.action.as_ref().map(|x| x.as_str()), // Convert Option<&String> to Option<&str>
        mode: None,
        execution_mode: button.execution_mode.as_deref(),
        execution_config: button.execution_config.as_deref(),
        icon_fit: None,
    };

    // Update the button in the database with the new action
    diesel::update(buttons::dsl::buttons.filter(buttons::dsl::id.eq(button_id)))
        .set(update_button) // Use `UpdateButton` for the update
        .execute(&mut conn)
        .map_err(|e| format!("Error updating button: {}", e))?;

    // Broadcast updated deck to all connected mobile clients (with icons for relay mode)
    let db = Database::new(&state);
    let icons_base_path = app_handle
        .try_state::<IconServiceState>()
        .map(|icon_state| icon_state.service.get_icons_base_dir());

    match db
        .get_deck_with_buttons(deck_id, true, icons_base_path.as_deref())
        .await
    {
        Some(deck) => {
            use std::sync::Arc;
            use tauri::Manager;
            use vortideck_core::core::StateManager;
            if let Some(state_manager) = app_handle.try_state::<Arc<StateManager>>() {
                let update_message = serde_json::json!({
                    "action": "deck_updated",
                    "deck_id": deck_id,
                    "deck": deck
                });
                crate::broadcast_helper::broadcast_to_all_clients(
                    &app_handle,
                    &state_manager,
                    update_message,
                )
                .await;
                println!("📡 Button action updated and deck broadcasted to all clients");
            }
        }
        None => {
            eprintln!(
                "Could not find deck with id {} after button action update",
                deck_id
            );
        }
    }

    let _ = crate::commands::widget_commands::sync_button_to_widget_document(
        &app_handle,
        &db.to_core_database(),
        button_id,
    )
    .await;

    Ok("Button action updated.".to_string())
}

#[tauri::command]
pub async fn set_button_mode(
    app_handle: AppHandle,           // Add app_handle for StateManager access
    state: tauri::State<'_, DbPool>, // Access the database pool
    button_id: i32,                  // Button ID to update
    mode: String,                    // Action enum (could be MacroId, System, etc.)
) -> Result<String, String> {
    let mut conn = state.get().expect("Failed to get DB connection");

    // Serialize the action (this is done by `set_action` method)
    let mut button = buttons::dsl::buttons
        .filter(buttons::dsl::id.eq(button_id))
        .first::<Button>(&mut conn)
        .map_err(|e| format!("Error finding button: {}", e))?;

    // Use UpdateButton to perform the update
    let update_button = UpdateButton {
        label: Some(&button.label), // Update the fields you want
        icon: button.icon.as_deref(),
        background: button.background.as_deref(),
        position: Some(button.position),
        action: button.action.as_ref().map(|x| x.as_str()), // Convert Option<&String> to Option<&str>
        mode: Some(&mode),
        execution_mode: button.execution_mode.as_deref(),
        execution_config: button.execution_config.as_deref(),
        icon_fit: None,
    };

    // Update the button in the database with the new action
    diesel::update(buttons::dsl::buttons.filter(buttons::dsl::id.eq(button_id)))
        .set(update_button) // Use `UpdateButton` for the update
        .execute(&mut conn)
        .map_err(|e| format!("Error updating button: {}", e))?;

    // Get the updated deck to broadcast (with icons for relay mode)
    let db = Database::new(&state);
    let icons_base_path = app_handle
        .try_state::<IconServiceState>()
        .map(|icon_state| icon_state.service.get_icons_base_dir());

    match db
        .get_deck_with_buttons(button.deck_id, true, icons_base_path.as_deref())
        .await
    {
        Some(deck) => {
            // Broadcast deck update via StateManager
            use tauri::Manager;
            if let Some(state_manager) = app_handle.try_state::<Arc<StateManager>>() {
                let update_message = serde_json::json!({
                    "action": "deck_updated",
                    "deck_id": button.deck_id,
                    "deck": deck
                });
                crate::broadcast_helper::broadcast_to_all_clients(
                    &app_handle,
                    &state_manager,
                    update_message,
                )
                .await;
                println!("📡 Button mode updated and deck broadcasted to all clients");
            } else {
                println!("📡 Button mode updated (StateManager not available for broadcast)");
            }
        }
        None => {
            eprintln!("Could not find deck with id {}", button.deck_id);
        }
    }

    let _ = crate::commands::widget_commands::sync_button_to_widget_document(
        &app_handle,
        &db.to_core_database(),
        button_id,
    )
    .await;

    Ok("Button action updated.".to_string())
}

#[tauri::command]
pub async fn get_macro_by_id(
    state: State<'_, DbPool>,
    macro_id: i32,
) -> Result<serde_json::Value, serde_json::Value> {
    let db = Database::new(&state);
    match db.get_macro_with_chain(macro_id).await {
        Some(macro_data) => Ok(serde_json::json!({
            "data": macro_data,
            "status": 200,
            "message": "Successfully retrieved macro"
        })),
        None => Err(serde_json::json!({
            "status": 404,
            "message": "Macro not found"
        })),
    }
}

#[tauri::command]
pub async fn reset_button(
    app_handle: AppHandle,
    button_id: i32,           // ID of the button to reset
    state: State<'_, DbPool>, // Access to the database pool
) -> Result<String, String> {
    // Instantiate the Database object with the DB pool
    let db = Database::new(&state);

    // Get button's deck_id before resetting
    let deck_id = match db.get_button_by_id(button_id).await {
        Ok(Some(button)) => button.deck_id,
        Ok(None) => return Err("Button not found".to_string()),
        Err(e) => return Err(format!("Database error: {}", e)),
    };

    // Call the reset_button method and handle the result
    match db.reset_button(button_id).await {
        Ok(message) => {
            // Remove image from button using new adapter architecture
            if let Some(icons_state) = app_handle.try_state::<IconServiceState>() {
                let _ = icons_state
                    .service
                    .remove_icon(&button_id.to_string())
                    .await;
            }

            // Broadcast updated deck to all connected mobile clients (with icons for relay mode)
            let icons_base_path = app_handle
                .try_state::<IconServiceState>()
                .map(|icon_state| icon_state.service.get_icons_base_dir());

            match db
                .get_deck_with_buttons(deck_id, true, icons_base_path.as_deref())
                .await
            {
                Some(deck) => {
                    use std::sync::Arc;
                    use tauri::Manager;
                    use vortideck_core::core::StateManager;
                    if let Some(state_manager) = app_handle.try_state::<Arc<StateManager>>() {
                        let update_message = serde_json::json!({
                            "action": "deck_updated",
                            "deck_id": deck_id,
                            "deck": deck
                        });
                        crate::broadcast_helper::broadcast_to_all_clients(
                            &app_handle,
                            &state_manager,
                            update_message,
                        )
                        .await;
                        println!("📡 Button reset and deck broadcasted to all clients");
                    }
                }
                None => {
                    eprintln!("Could not find deck with id {} after button reset", deck_id);
                }
            }

            let _ = crate::commands::widget_commands::sync_button_to_widget_document(
                &app_handle,
                &db.to_core_database(),
                button_id,
            )
            .await;

            Ok(message) // Return success message
        }
        Err(err) => Err(err), // Return error if any
    }
}

#[tauri::command]
pub async fn set_button_icon_fit(
    app_handle: AppHandle,
    state: tauri::State<'_, DbPool>,
    button_id: i32,
    icon_fit: String,
) -> Result<Button, String> {
    let normalized_fit = match icon_fit.as_str() {
        "cover" => "cover",
        "contain" => "contain",
        _ => return Err("icon_fit must be either 'cover' or 'contain'".to_string()),
    };
    let db = Database::new(&state);
    let current = db
        .get_button_by_id(button_id)
        .await
        .map_err(|error| format!("Database error: {error}"))?
        .ok_or_else(|| "Button not found".to_string())?;
    let deck_id = current.deck_id;
    let updated = db
        .update_button(
            button_id,
            UpdateButton {
                label: None,
                icon: None,
                background: None,
                position: None,
                action: None,
                mode: None,
                execution_mode: None,
                execution_config: None,
                icon_fit: Some(normalized_fit),
            },
        )
        .await
        .map_err(|error| format!("Could not update button image fit: {error}"))?;

    let icons_base_path = app_handle
        .try_state::<IconServiceState>()
        .map(|icon_state| icon_state.service.get_icons_base_dir());
    if let Some(deck) = db
        .get_deck_with_buttons(deck_id, true, icons_base_path.as_deref())
        .await
    {
        use std::sync::Arc;
        use tauri::Manager;
        use vortideck_core::core::StateManager;
        if let Some(state_manager) = app_handle.try_state::<Arc<StateManager>>() {
            crate::broadcast_helper::broadcast_to_all_clients(
                &app_handle,
                &state_manager,
                serde_json::json!({
                    "action": "deck_updated",
                    "deck_id": deck_id,
                    "deck": deck,
                }),
            )
            .await;
        }
    }

    let _ = crate::commands::widget_commands::sync_button_to_widget_document(
        &app_handle,
        &db.to_core_database(),
        button_id,
    )
    .await;

    Ok(updated)
}

/// Set button execution mode
#[tauri::command]
pub async fn set_button_execution_mode(
    app_handle: AppHandle,
    state: tauri::State<'_, DbPool>,
    button_id: i32,
    execution_mode: String,
    execution_config: Option<serde_json::Value>,
) -> Result<String, String> {
    let db = Database::new(&state);

    // Get button's deck_id before updating
    let deck_id = match db.get_button_by_id(button_id).await {
        Ok(Some(button)) => button.deck_id,
        Ok(None) => return Err("Button not found".to_string()),
        Err(e) => return Err(format!("Database error: {}", e)),
    };

    // Convert the JSON config to a string for storage
    let config_string = execution_config
        .map(|config| serde_json::to_string(&config).unwrap_or_else(|_| "{}".to_string()));

    let updated_button = UpdateButton {
        label: None,
        icon: None,
        background: None,
        position: None,
        action: None,
        mode: None,
        execution_mode: Some(&execution_mode),
        execution_config: config_string.as_deref(),
        icon_fit: None,
    };

    match db.update_button(button_id, updated_button).await {
        Ok(_) => {
            // Broadcast updated deck to all connected mobile clients (with icons for relay mode)
            let icons_base_path = app_handle
                .try_state::<IconServiceState>()
                .map(|icon_state| icon_state.service.get_icons_base_dir());

            match db
                .get_deck_with_buttons(deck_id, true, icons_base_path.as_deref())
                .await
            {
                Some(deck) => {
                    use std::sync::Arc;
                    use tauri::Manager;
                    use vortideck_core::core::StateManager;
                    if let Some(state_manager) = app_handle.try_state::<Arc<StateManager>>() {
                        let update_message = serde_json::json!({
                            "action": "deck_updated",
                            "deck_id": deck_id,
                            "deck": deck
                        });
                        crate::broadcast_helper::broadcast_to_all_clients(
                            &app_handle,
                            &state_manager,
                            update_message,
                        )
                        .await;
                        println!(
                            "📡 Button execution mode updated and deck broadcasted to all clients"
                        );
                    }
                }
                None => {
                    eprintln!(
                        "Could not find deck with id {} after execution mode update",
                        deck_id
                    );
                }
            }

            let _ = crate::commands::widget_commands::sync_button_to_widget_document(
                &app_handle,
                &db.to_core_database(),
                button_id,
            )
            .await;

            Ok(format!(
                "Button {} execution mode set to: {}",
                button_id, execution_mode
            ))
        }
        Err(e) => Err(format!("Error updating button execution mode: {}", e)),
    }
}

use std::sync::Arc;
use vortideck_core::core::StateManager;

#[tauri::command]
pub async fn update_deck_settings(
    app_handle: AppHandle,
    state: State<'_, DbPool>,
    deck_id: i32,
    deck_settings: DeckSettings,
) -> Result<(String, i32, Option<DeckWithNestedButtons>), (String, i32)> {
    let db = Database::new(&state);

    match db.update_deck_settings(deck_id, deck_settings).await {
        Ok(updated_deck) => {
            let icons_base_path = app_handle
                .try_state::<IconServiceState>()
                .map(|icon_state| icon_state.service.get_icons_base_dir());

            if let Some(deck) = db
                .get_deck_with_buttons(deck_id, true, icons_base_path.as_deref())
                .await
            {
                if let Some(state_manager) = app_handle.try_state::<Arc<StateManager>>() {
                    crate::broadcast_helper::broadcast_to_all_clients(
                        &app_handle,
                        &state_manager,
                        serde_json::json!({
                            "action": "deck_updated",
                            "deck_id": deck_id,
                            "deck": deck,
                        }),
                    )
                    .await;
                    println!("📡 Deck settings updated and deck broadcasted to all clients");
                } else {
                    println!("📡 Deck settings updated (StateManager not available for broadcast)");
                }
            }

            let message = if updated_deck.is_some() {
                "Deck settings and grid updated."
            } else {
                "Deck settings updated."
            };
            Ok((message.to_string(), 200, updated_deck))
        }
        Err(err) => Err((format!("Failed to update deck settings: {}", err), 500)),
    }
}
#[tauri::command]
pub async fn update_deck(
    state: State<'_, DbPool>,
    // peer_state: State<'_, PeerState>,
    deck_id: i32,
    deck_name: Option<String>,
) -> Result<(String, i32), (String, i32)> {
    let db = Database::new(&state);

    // Use UpdateButton to perform the update
    let _updated_deck = UpdateDeck {
        name: deck_name.as_deref(),
    };

    match db.update_deck(deck_id, _updated_deck).await {
        Ok(_) => {
            // Send updated deck to all connected peers
            // peer_state.broadcast_message(serde_json::json!({
            //     "action": "deck_updated",
            //     "deck": deck
            // })).await;
            Ok(("Deck updated successfully".to_string(), 200))
        }
        Err(err) => Err((format!("Failed to update deck: {}", err), 500)),
    }
}

#[tauri::command]
pub async fn update_deck_grid(
    state: State<'_, DbPool>,
    deck_id: i32,
    new_grid: Grid,
) -> Result<(String, i32), (String, i32)> {
    let db = Database::new(&state);
    db.update_deck_grid(deck_id, new_grid).await;

    match db.update_deck_grid(deck_id, new_grid).await {
        Ok(_) => Ok(("Deck grid updated.".to_string(), 200)),
        Err(err) => Err((format!("Failed to update deck grid: {}", err), 500)),
    }
}

#[tauri::command]
pub async fn download_theme(
    state: State<'_, DbPool>,
    deck_theme: ThemeInput, // Use ThemeInput for frontend input
) -> Result<serde_json::Value, serde_json::Value> {
    log::info!("Downloading theme: {:?}", deck_theme);

    let conn = &mut state.get().map_err(|e| {
        log::error!("Failed to get database connection: {}", e);
        serde_json::json!({
            "status": 500,
            "message": format!("Failed to get database connection: {}", e)
        })
    })?;

    // Convert ThemeInput to DeckTheme
    let deck_theme = DeckTheme::from(deck_theme);
    log::info!("Converted theme input to DeckTheme");

    let inserted_theme = diesel::insert_into(themes::table)
        .values(&deck_theme) // DeckTheme is now Insertable
        .get_result::<DeckTheme>(conn)
        .map_err(|e| {
            log::error!("Failed to insert theme into database: {}", e);
            serde_json::json!({
                "status": 500,
                "message": format!("Failed to add deck theme: {}", e)
            })
        })?;

    log::info!("Successfully inserted theme into database");

    Ok(serde_json::json!({
        "data": inserted_theme.get(), // Use get() to return nested JSON
        "status": 200,
        "message": "Successfully added deck theme"
    }))
}

#[tauri::command]
pub async fn uninstall_theme(
    state: State<'_, DbPool>,
    theme_id: i32,
) -> Result<serde_json::Value, serde_json::Value> {
    let conn = &mut state.get().map_err(|e| {
        serde_json::json!({
            "status": 500,
            "message": format!("Failed to get database connection: {}", e)
        })
    })?;

    let deleted_count = diesel::delete(themes::table.filter(themes::id.eq(theme_id)))
        .execute(conn)
        .map_err(|e| {
            serde_json::json!({
                "status": 500,
                "message": format!("Failed to uninstall theme: {}", e)
            })
        })?;

    if deleted_count == 0 {
        return Err(serde_json::json!({
            "status": 404,
            "message": "Theme not found"
        }));
    }

    Ok(serde_json::json!({
        "status": 200,
        "message": "Theme uninstalled successfully"
    }))
}

#[tauri::command]
pub async fn get_themes(state: State<'_, DbPool>) -> Result<serde_json::Value, serde_json::Value> {
    let conn = &mut state.get().map_err(|e| {
        serde_json::json!({
            "status": 500,
            "message": format!("Failed to get database connection: {}", e)
        })
    })?;

    let themes = themes::table.load::<DeckTheme>(conn).map_err(|e| {
        serde_json::json!({
            "status": 500,
            "message": format!("Failed to get themes: {}", e)
        })
    })?;

    Ok(serde_json::json!({
        "data": themes.iter().map(|t| t.get()).collect::<Vec<_>>(),
        "status": 200,
        "message": "Successfully retrieved themes"
    }))
}
#[tauri::command]
pub async fn get_app_config(
    state: State<'_, DbPool>,
) -> Result<serde_json::Value, serde_json::Value> {
    let conn = &mut state.get().map_err(|e| {
        serde_json::json!({
            "status": 500,
            "message": format!("Failed to get database connection: {}", e)
        })
    })?;

    let config = app_config.first::<AppConfig>(conn).map_err(|e| {
        serde_json::json!({
            "status": 500,
            "message": format!("Failed to get app config: {}", e)
        })
    })?;

    Ok(serde_json::json!({
        "data": config,
        "status": 200,
        "message": "Successfully retrieved app config"
    }))
}
#[tauri::command]
pub async fn update_app_config(
    state: State<'_, DbPool>,
    app_handle: AppHandle,
    new_tls_enabled: Option<bool>,
    new_language: Option<String>,
    new_theme: Option<String>,
    new_api_url: Option<String>,
    new_log_level: Option<String>,
    new_timezone: Option<String>,
    new_show_instruction: Option<bool>,
    new_mcp_enabled: Option<bool>,
    new_max_concurrent_actions: Option<i32>,
    new_max_action_timeout_ms: Option<i32>,
    new_relay_enabled: Option<bool>,
    new_streaming_enabled: Option<bool>,
    new_analytics_enabled: Option<bool>,
    new_analytics_consent_given: Option<bool>,
) -> Result<serde_json::Value, serde_json::Value> {
    let conn = &mut state.get().map_err(|e| {
        serde_json::json!({
            "status": 500,
            "message": format!("Failed to get database connection: {}", e)
        })
    })?;

    let current_config = app_config.first::<AppConfig>(conn).map_err(|e| {
        serde_json::json!({
            "status": 500,
            "message": format!("Failed to get app config: {}", e)
        })
    })?;

    // if let Some(lang) = &new_language {
    //     println!("incoming language: {}", lang);
    //     println!("Updating language to: {}", lang);
    // }

    // Create longer-lived values to fix the temporary value dropped while borrowed errors
    // For language, we need to ensure we're using the new value if provided

    // Only update fields that are explicitly provided, otherwise keep current values
    let updated_config = UpdateConfig {
        key: &current_config.key,
        tls_enabled: new_tls_enabled.unwrap_or(current_config.tls_enabled),
        language: new_language.as_deref(),
        theme: new_theme.as_deref(),
        api_url: new_api_url.as_deref(),
        log_level: new_log_level.as_deref(),
        timezone: new_timezone.as_deref(),
        show_instruction: new_show_instruction.unwrap_or(current_config.show_instruction),
        mcp_enabled: new_mcp_enabled.unwrap_or(current_config.mcp_enabled),
        plugin_hot_reload_enabled: current_config.plugin_hot_reload_enabled,
        max_concurrent_actions: new_max_concurrent_actions
            .unwrap_or(current_config.max_concurrent_actions),
        max_action_timeout_ms: new_max_action_timeout_ms
            .unwrap_or(current_config.max_action_timeout_ms),
        relay_enabled: new_relay_enabled.unwrap_or(current_config.relay_enabled),
        streaming_enabled: new_streaming_enabled.unwrap_or(current_config.streaming_enabled),
        analytics_enabled: new_analytics_enabled.unwrap_or(current_config.analytics_enabled),
        analytics_consent_given: new_analytics_consent_given
            .unwrap_or(current_config.analytics_consent_given),
    };

    diesel::update(app_config.find(&current_config.key))
        .set(&updated_config)
        .execute(conn)
        .map_err(|e| {
            serde_json::json!({
                "status": 500,
                "message": format!("Failed to update app config: {}", e)
            })
        })?;

    // Fetch the updated config to ensure we return the latest values
    let updated_config = app_config.first::<AppConfig>(conn).map_err(|e| {
        serde_json::json!({
            "status": 500,
            "message": format!("Failed to get updated app config: {}", e)
        })
    })?;

    // Sync analytics flag to in-memory atomic for Rust-side gating
    crate::analytics::ANALYTICS_ENABLED.store(
        updated_config.analytics_enabled,
        std::sync::atomic::Ordering::Relaxed,
    );

    let emit_message = serde_json::json!({
        "status": 200,
        "message": "Successfully updated app config",
        "data": updated_config
    });

    app_handle
        .emit("app_config_updated", &emit_message)
        .map_err(|e| {
            serde_json::json!({
                "status": 500,
                "message": format!("Failed to emit app_config_updated event: {}", e)
            })
        })?;

    Ok(emit_message)
}

// ============================================================================
// PROGRAM DATABASE FUNCTIONS
// ============================================================================

#[tauri::command]
pub async fn save_program_to_db(
    app_handle: tauri::AppHandle,
    state: State<'_, DbPool>,
    program_data: ProgramData,
) -> Result<(String, i32), String> {
    use crate::db::schema::programs;

    let conn = &mut state
        .get()
        .map_err(|e| format!("Database connection error: {}", e))?;

    // Serialize arrays and objects as JSON strings
    let args_json = if program_data.args.is_empty() {
        None
    } else {
        Some(
            serde_json::to_string(&program_data.args)
                .map_err(|e| format!("Failed to serialize args: {}", e))?,
        )
    };

    let env_vars_json = if let Some(env_vars_map) = &program_data.env_vars {
        if env_vars_map.is_empty() {
            None
        } else {
            Some(
                serde_json::to_string(env_vars_map)
                    .map_err(|e| format!("Failed to serialize env_vars: {}", e))?,
            )
        }
    } else {
        None
    };

    let new_program = NewProgram {
        name: &program_data.name,
        executable: &program_data.executable,
        args: args_json.as_deref(),
        working_dir: program_data.working_dir.as_deref(),
        env_vars: env_vars_json.as_deref(),
        timeout_ms: program_data.timeout_ms.map(|t| t as i32),
        show_terminal: program_data.show_terminal,
        program_type: &program_data.program_type,
        description: program_data.description.as_deref(),
        custom_icon: program_data.custom_icon.as_deref(),
    };

    let result: Program = diesel::insert_into(programs::table)
        .values(&new_program)
        .returning(programs::all_columns)
        .get_result(conn)
        .map_err(|e| format!("Failed to save program: {}", e))?;

    // Emit event to frontend that program was created
    let _ = app_handle.emit("program_created", ());

    Ok((
        format!("Program '{}' saved successfully", program_data.name),
        result.id,
    ))
}

#[tauri::command]
pub async fn get_saved_programs(state: State<'_, DbPool>) -> Result<Vec<Program>, String> {
    use crate::db::schema::programs::dsl::*;

    let conn = &mut state
        .get()
        .map_err(|e| format!("Database connection error: {}", e))?;

    let programs_list = programs
        .select(Program::as_select())
        .load(conn)
        .map_err(|e| format!("Failed to get programs: {}", e))?;

    Ok(programs_list)
}

#[tauri::command]
pub async fn get_programs_with_meta(
    state: State<'_, DbPool>,
    include_icons: Option<bool>,
) -> Result<Vec<ProgramWithMeta>, String> {
    use crate::db::schema::programs::dsl::*;

    let conn = &mut state
        .get()
        .map_err(|e| format!("Database connection error: {}", e))?;

    let programs_list = programs
        .select(Program::as_select())
        .load(conn)
        .map_err(|e| format!("Failed to get programs: {}", e))?;

    // Convert to enhanced type with metadata
    let mut programs_with_meta: Vec<ProgramWithMeta> = programs_list
        .into_iter()
        .map(ProgramWithMeta::from)
        .collect();

    // Optionally load icons if requested
    if include_icons.unwrap_or(false) {
        for program in &mut programs_with_meta {
            program.icon_data =
                extract_program_icon(&program.executable, &program.program_type).await;
        }
    }

    Ok(programs_with_meta)
}

#[cfg(target_os = "windows")]
async fn extract_program_icon(executable_path: &str, program_type: &str) -> Option<String> {
    match program_type {
        "executable" => {
            // Try to extract icon from executable file
            extract_exe_icon(executable_path).await
        }
        "command" => {
            // Try to resolve command to executable path and extract icon
            if let Some(resolved_path) = resolve_command_path(executable_path).await {
                extract_exe_icon(&resolved_path).await
            } else {
                None
            }
        }
        _ => None, // Scripts and shell commands use generic icons
    }
}

#[cfg(not(target_os = "windows"))]
async fn extract_program_icon(_executable_path: &str, _program_type: &str) -> Option<String> {
    None // Icon extraction not implemented for non-Windows platforms
}

#[cfg(target_os = "windows")]
async fn extract_exe_icon(exe_path: &str) -> Option<String> {
    use std::path::Path;

    // Check if file exists first
    if !Path::new(exe_path).exists() {
        return None;
    }

    // Use spawn_blocking to avoid blocking the async runtime
    let exe_path = exe_path.to_string();
    tokio::task::spawn_blocking(move || extract_icon_sync(&exe_path))
        .await
        .unwrap_or(None)
}

#[cfg(target_os = "windows")]
fn extract_icon_sync(exe_path: &str) -> Option<String> {
    use windows::{
        core::HSTRING,
        Win32::Foundation::HINSTANCE,
        Win32::UI::Shell::ExtractIconW,
        Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, ICONINFO},
    };

    unsafe {
        // Convert path to HSTRING
        let path_hstring = HSTRING::from(exe_path);

        // Extract the first icon (index 0) from the executable
        let h_icon = ExtractIconW(None, &path_hstring, 0);

        // Check if extraction was successful
        if h_icon.is_invalid() {
            return None;
        }

        // Convert icon to base64
        let result = convert_icon_to_base64(h_icon);

        // Clean up the icon handle
        let _ = DestroyIcon(h_icon);

        result
    }
}

#[cfg(target_os = "windows")]
fn convert_icon_to_base64(
    h_icon: windows::Win32::UI::WindowsAndMessaging::HICON,
) -> Option<String> {
    use base64::Engine;
    use windows::Win32::{
        Graphics::Gdi::{
            CreateCompatibleDC, DeleteDC, GetDC, GetDIBits, GetObjectW, ReleaseDC, BITMAP,
            BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS,
        },
        UI::WindowsAndMessaging::{GetIconInfo, ICONINFO},
    };

    unsafe {
        // Get icon info
        let mut icon_info: ICONINFO = std::mem::zeroed();
        if GetIconInfo(h_icon, &mut icon_info).is_err() {
            return None;
        }

        // Get the color bitmap info
        let mut bitmap: BITMAP = std::mem::zeroed();
        if GetObjectW(
            icon_info.hbmColor.into(),
            std::mem::size_of::<BITMAP>() as i32,
            Some(&mut bitmap as *mut _ as *mut _),
        ) == 0
        {
            return None;
        }

        let width = bitmap.bmWidth;
        let height = bitmap.bmHeight;

        // Get device context
        let h_dc = GetDC(None);
        if h_dc.is_invalid() {
            return None;
        }

        // Create compatible DC
        let mem_dc = CreateCompatibleDC(Some(h_dc));

        // Setup bitmap info for 32-bit RGBA
        let mut bmi: BITMAPINFO = std::mem::zeroed();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = width;
        bmi.bmiHeader.biHeight = -height; // Top-down bitmap
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = 0; // BI_RGB value

        // Allocate buffer for bitmap data
        let buffer_size = (width * height * 4) as usize;
        let mut buffer: Vec<u8> = vec![0; buffer_size];

        // Get bitmap bits
        let lines_copied = GetDIBits(
            h_dc,
            icon_info.hbmColor,
            0,
            height as u32,
            Some(buffer.as_mut_ptr() as *mut _),
            &mut bmi,
            DIB_RGB_COLORS,
        );

        // Clean up
        let _ = ReleaseDC(None, h_dc);
        let _ = DeleteDC(mem_dc);

        if lines_copied == 0 {
            return None;
        }

        // Convert BGRA to RGBA and create PNG
        for chunk in buffer.chunks_mut(4) {
            chunk.swap(0, 2); // Swap B and R channels
        }

        // Create image from raw RGBA data
        if let Some(img) = image::RgbaImage::from_raw(width as u32, height as u32, buffer) {
            let mut png_data = Vec::new();
            if let Ok(_) = image::DynamicImage::ImageRgba8(img).write_to(
                &mut std::io::Cursor::new(&mut png_data),
                image::ImageFormat::Png,
            ) {
                // Encode as base64
                let base64_data = base64::engine::general_purpose::STANDARD.encode(&png_data);
                return Some(format!("data:image/png;base64,{}", base64_data));
            }
        }

        None
    }
}

async fn resolve_command_path(command: &str) -> Option<String> {
    // Simple PATH resolution without external crates for now
    // TODO: Use which crate for better PATH resolution
    let path_env = std::env::var("PATH").ok()?;

    #[cfg(target_os = "windows")]
    let separator = ";";
    #[cfg(not(target_os = "windows"))]
    let separator = ":";

    #[cfg(target_os = "windows")]
    let exe_extensions = vec!["", ".exe", ".cmd", ".bat"];
    #[cfg(not(target_os = "windows"))]
    let exe_extensions = vec![""];

    for path_dir in path_env.split(separator) {
        for ext in &exe_extensions {
            let full_path = std::path::Path::new(path_dir).join(format!("{}{}", command, ext));
            if full_path.exists() {
                return full_path.to_str().map(String::from);
            }
        }
    }

    None
}

#[tauri::command]
pub async fn get_program_by_id(
    state: State<'_, DbPool>,
    program_id: i32,
) -> Result<Option<Program>, String> {
    use crate::db::schema::programs::dsl::*;

    let conn = &mut state
        .get()
        .map_err(|e| format!("Database connection error: {}", e))?;

    let program = programs
        .filter(id.eq(program_id))
        .select(Program::as_select())
        .first(conn)
        .optional()
        .map_err(|e| format!("Failed to get program: {}", e))?;

    Ok(program)
}

#[tauri::command]
pub async fn update_program_in_db(
    app_handle: tauri::AppHandle,
    state: State<'_, DbPool>,
    program_id: i32,
    program_data: ProgramData,
) -> Result<(String, i32), String> {
    use crate::db::schema::programs::dsl::*;

    let conn = &mut state
        .get()
        .map_err(|e| format!("Database connection error: {}", e))?;

    // Serialize arrays and objects as JSON strings
    let args_json = if program_data.args.is_empty() {
        None
    } else {
        Some(
            serde_json::to_string(&program_data.args)
                .map_err(|e| format!("Failed to serialize args: {}", e))?,
        )
    };

    let env_vars_json = if let Some(env_vars_map) = &program_data.env_vars {
        if env_vars_map.is_empty() {
            None
        } else {
            Some(
                serde_json::to_string(env_vars_map)
                    .map_err(|e| format!("Failed to serialize env_vars: {}", e))?,
            )
        }
    } else {
        None
    };

    let update_program = UpdateProgram {
        name: Some(&program_data.name),
        executable: Some(&program_data.executable),
        args: Some(args_json.as_deref()),
        working_dir: Some(program_data.working_dir.as_deref()),
        env_vars: Some(env_vars_json.as_deref()),
        timeout_ms: Some(program_data.timeout_ms.map(|t| t as i32)),
        show_terminal: Some(program_data.show_terminal),
        program_type: Some(&program_data.program_type),
        description: Some(program_data.description.as_deref()),
        custom_icon: Some(program_data.custom_icon.as_deref()),
    };

    let updated_count = diesel::update(programs.filter(id.eq(program_id)))
        .set(&update_program)
        .execute(conn)
        .map_err(|e| format!("Failed to update program: {}", e))?;

    if updated_count == 0 {
        return Err("Program not found".to_string());
    }

    // Emit event to frontend that program was updated
    let _ = app_handle.emit("program_updated", ());

    Ok((
        format!("Program '{}' updated successfully", program_data.name),
        program_id,
    ))
}

#[tauri::command]
pub async fn remove_program_from_db(
    app_handle: AppHandle,
    state: State<'_, DbPool>,
    program_id: i32,
) -> Result<(String, i32), String> {
    use crate::db::schema::programs::dsl::*;

    let conn = &mut state
        .get()
        .map_err(|e| format!("Database connection error: {}", e))?;

    let deleted_count = diesel::delete(programs.filter(id.eq(program_id)))
        .execute(conn)
        .map_err(|e| format!("Failed to delete program: {}", e))?;

    if deleted_count == 0 {
        return Err("Program not found".to_string());
    }

    // Clean up context menu items referencing this program
    if crate::commands::context_menu_commands::clear_orphaned_items(
        "Program",
        &program_id.to_string(),
    ) {
        let _ = app_handle.emit("context-menu-updated", ());
    }

    Ok(("Program deleted successfully".to_string(), program_id))
}

// ============================================================================
// EXECUTION SETTINGS COMMANDS
// ============================================================================
// NOTE: Execution settings are now part of app_config.
// Use get_app_config() and update_app_config() instead of separate commands.
// This provides a single source of truth for all configuration settings.
