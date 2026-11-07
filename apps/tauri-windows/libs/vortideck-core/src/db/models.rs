use crate::db::schema::app_config;
use crate::db::schema::macros;
use crate::db::schema::peers;
use crate::db::schema::{buttons, decks, flows, http_requests, themes, webhook_events, webhooks};
use diesel::backend::Backend;
use diesel::deserialize::{self, FromSql};
use diesel::prelude::*;
use diesel::serialize::{self, Output, ToSql};
use diesel::sql_types::Text;
use diesel::sqlite::Sqlite;
use diesel::AsExpression;
use diesel::FromSqlRow;
use serde::Deserialize;
use serde::Serialize;
use std::borrow::Cow;
use std::io::Write;
//  diesel migration generate --diff-schema

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::db::schema::peers)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Peer {
    pub id: Option<i32>, // Nullable column
    pub client_id: String,
    pub ip: String,
    pub device_type: String,
    pub device: String,
    pub device_token: Option<String>, // New field for the device token
    pub mobile_public_key: Option<String>, // E2EE: Mobile's X25519 public key (base64)
    pub shared_secret: Option<String>, // E2EE: ECDH shared secret (base64)
}

#[derive(Insertable, Clone, Serialize)]
#[diesel(table_name = crate::db::schema::peers)]
pub struct NewPeer<'a> {
    pub client_id: &'a str,
    pub ip: &'a str,
    pub device_type: &'a str,
    pub device: &'a str,
    pub device_token: Option<&'a str>, // Token to insert (optional)
    pub mobile_public_key: Option<&'a str>, // E2EE: Mobile's public key
    pub shared_secret: Option<&'a str>, // E2EE: Shared secret
}

#[derive(Queryable, Selectable, Serialize)]
#[diesel(table_name = crate::db::schema::app_config)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct AppConfig {
    pub key: String,
    pub tls_enabled: bool,
    pub api_url: Option<String>,   // Optional API URL
    pub log_level: Option<String>, // Optional log level
    pub language: Option<String>,  // Optional language setting
    pub timezone: Option<String>,  // Optional timezone
    pub theme: Option<String>,     // Optional theme setting
    pub show_instruction: bool,    // show instruction settings on app run
    pub mcp_enabled: bool,
    pub plugin_hot_reload_enabled: bool,
    pub desktop_id: Option<String>, // Persistent relay desktop identifier
    pub max_concurrent_actions: i32, // Maximum concurrent action executions (buttons, macros, flows, webhooks, etc.)
    pub max_action_timeout_ms: i32,  // Action execution timeout in milliseconds
    pub relay_enabled: bool,         // Enable/disable relay connection
    pub streaming_enabled: bool,     // Enable/disable streaming connection
    pub analytics_enabled: bool,     // Enable/disable anonymous usage analytics
    pub analytics_consent_given: bool, // Whether user has made an explicit analytics choice
}

#[derive(Insertable)]
#[diesel(table_name = crate::db::schema::app_config)]
pub struct NewAppConfig<'a> {
    pub key: &'a str,
    pub tls_enabled: bool,
    pub api_url: Option<&'a str>,   // Optional API URL
    pub log_level: Option<&'a str>, // Optional log level
    pub language: Option<&'a str>,  // Optional language setting
    pub timezone: Option<&'a str>,  // Optional timezone
    pub theme: Option<&'a str>,     // Optional theme setting
    pub show_instruction: bool,     // show instruction settings on app run
    pub mcp_enabled: bool,
    pub plugin_hot_reload_enabled: bool,
    pub desktop_id: Option<&'a str>, // Persistent relay desktop identifier
    pub max_concurrent_actions: i32, // Maximum concurrent action executions (buttons, macros, flows, webhooks, etc.)
    pub max_action_timeout_ms: i32,  // Action execution timeout in milliseconds
    pub relay_enabled: bool,         // Enable/disable relay connection
    pub streaming_enabled: bool,     // Enable/disable streaming connection
    pub analytics_enabled: bool,     // Enable/disable anonymous usage analytics
    pub analytics_consent_given: bool, // Whether user has made an explicit analytics choice
}

#[derive(AsChangeset)]
#[diesel(table_name = crate::db::schema::app_config)]
pub struct UpdateConfig<'a> {
    pub key: &'a str,
    pub tls_enabled: bool,
    pub api_url: Option<&'a str>,   // Optional API URL
    pub log_level: Option<&'a str>, // Optional log level
    pub language: Option<&'a str>,  // Optional language setting
    pub timezone: Option<&'a str>,  // Optional timezone
    pub theme: Option<&'a str>,     // Optional theme setting
    pub show_instruction: bool,     // show instruction settings on app run
    pub mcp_enabled: bool,
    pub plugin_hot_reload_enabled: bool,
    pub max_concurrent_actions: i32, // Maximum concurrent action executions (buttons, macros, flows, webhooks, etc.)
    pub max_action_timeout_ms: i32,  // Action execution timeout in milliseconds
    pub relay_enabled: bool,         // Enable/disable relay connection
    pub streaming_enabled: bool,     // Enable/disable streaming connection
    pub analytics_enabled: bool,     // Enable/disable anonymous usage analytics
    pub analytics_consent_given: bool, // Whether user has made an explicit analytics choice
}

// Define Deck model

#[derive(Debug, Queryable, Selectable, Serialize)]
#[diesel(table_name = crate::db::schema::decks)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Deck {
    pub id: i32,
    pub name: String,
}

// Ensure `NewDeck` is correctly set up
#[derive(Insertable)]
#[diesel(table_name = crate::db::schema::decks)]
pub struct NewDeck<'a> {
    pub name: &'a str,
}

#[derive(AsChangeset)]
#[diesel(table_name = crate::db::schema::decks)]
pub struct UpdateDeck<'a> {
    pub name: Option<&'a str>,
}

#[derive(Debug, Clone, Queryable, Selectable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = crate::db::schema::deck_items)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct DeckItem {
    pub id: String,
    pub deck_id: i32,
    pub definition_id: String,
    pub package_id: Option<String>,
    pub contribution_id: Option<String>,
    pub config_json: String,
    pub config_version: i32,
    pub appearance_json: String,
    pub legacy_button_id: Option<i32>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Queryable, Selectable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = crate::db::schema::deck_item_layouts)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct DeckItemLayout {
    pub item_id: String,
    pub layout_key: String,
    pub x: i32,
    pub y: i32,
    pub col_span: i32,
    pub row_span: i32,
}

#[derive(Debug, Clone, Queryable, Selectable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = crate::db::schema::deck_item_bindings)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct DeckItemBinding {
    pub id: String,
    pub item_id: String,
    pub port_id: String,
    pub action_json: String,
    pub policy_json: String,
    pub transform_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckItemDocument {
    pub item: DeckItem,
    pub layouts: Vec<DeckItemLayout>,
    pub bindings: Vec<DeckItemBinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckWidgetDocument {
    pub deck_id: i32,
    pub revision: i32,
    pub updated_at: String,
    pub items: Vec<DeckItemDocument>,
}

#[derive(Debug, Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::db::schema::deck_widget_documents)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct DeckWidgetDocumentRecord {
    pub deck_id: i32,
    pub revision: i32,
    pub updated_at: String,
}

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = crate::db::schema::deck_item_persistent_state)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct DeckItemPersistentState {
    pub item_id: String,
    pub state_json: String,
    pub updated_at: String,
}

#[derive(Insertable)]
#[diesel(table_name = crate::db::schema::deck_item_persistent_state)]
pub struct NewDeckItemPersistentState<'a> {
    pub item_id: &'a str,
    pub state_json: &'a str,
    pub updated_at: &'a str,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Action {
    MacroId(i32),          // Store a reference to the macro by its ID
    SystemControl(String), // Custom action, like running a script
    Program(i32),          // Store a reference to the program by its ID
    SavedActionId(String), // Stable reference to a saved integration action
    App(String),
    FlowId(String),     // Store a reference to the flow by its ID
    HttpRequestId(i32), // HTTP request template ID
    FocusWindowId(i32), // Store a reference to the saved focus window by its ID
}

// Define Button model
#[derive(Debug, Queryable, Selectable, Serialize, Associations)]
#[diesel(belongs_to(Deck, foreign_key = deck_id))]
#[diesel(table_name = crate::db::schema::buttons)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Button {
    pub id: i32,
    pub deck_id: i32,
    pub label: String,
    pub icon: Option<String>,
    pub background: Option<String>,
    pub position: i32,
    pub action: Option<String>, // Change action_id to action (Option<String>) to hold serialized action
    pub mode: Option<String>,
    pub execution_mode: Option<String>,
    pub execution_config: Option<String>,
    pub icon_updated_at: Option<i32>, // Unix timestamp when icon was last updated
    pub icon_fit: String,
}

impl Button {
    // Get the action by deserializing the `action` field (which is a JSON string)
    pub fn get_action(&self) -> Result<Action, String> {
        match &self.action {
            Some(action_str) => serde_json::from_str(action_str)
                .map_err(|e| format!("Error deserializing action: {}", e)),
            None => Err("Action is missing".to_string()),
        }
    }

    // Set the action by serializing the Action enum into a JSON string
    pub fn set_action(&mut self, action: Action) -> Result<(), serde_json::Error> {
        self.action = Some(serde_json::to_string(&action)?);
        Ok(())
    }

    // Get the execution mode, defaulting to "run_once" if not set
    pub fn get_execution_mode(&self) -> String {
        self.execution_mode
            .clone()
            .unwrap_or_else(|| "run_once".to_string())
    }

    // Set the execution mode
    pub fn set_execution_mode(&mut self, mode: &str) {
        self.execution_mode = Some(mode.to_string());
    }

    // Get the execution config as a JSON value
    pub fn get_execution_config(&self) -> Result<Option<serde_json::Value>, String> {
        match &self.execution_config {
            Some(config_str) => match serde_json::from_str(config_str) {
                Ok(config) => Ok(Some(config)),
                Err(e) => Err(format!("Error deserializing execution config: {}", e)),
            },
            None => Ok(None),
        }
    }

    // Set the execution config by serializing a JSON value
    pub fn set_execution_config(
        &mut self,
        config: Option<serde_json::Value>,
    ) -> Result<(), serde_json::Error> {
        self.execution_config = match config {
            Some(config_val) => Some(serde_json::to_string(&config_val)?),
            None => None,
        };
        Ok(())
    }
}

// Implement `From<Action>` to serialize it as a String
impl From<Action> for String {
    fn from(action: Action) -> Self {
        serde_json::to_string(&action).unwrap() // Safely serialize the Action enum to a String
    }
}

// Implement `From<String>` to deserialize it into an Action enum
impl From<String> for Action {
    fn from(action_str: String) -> Self {
        // You should return the Action directly, not a Result
        serde_json::from_str(&action_str).unwrap() // This should be safe if the string is valid JSON
    }
}

#[derive(Insertable)]
#[diesel(table_name = crate::db::schema::buttons)]
pub struct NewButton {
    pub deck_id: i32,
    pub label: String,
    pub icon: Option<String>,
    pub background: Option<String>,
    pub position: i32,
    pub action: Option<String>,
    pub mode: Option<String>,
    pub execution_mode: Option<String>,
    pub execution_config: Option<String>,
}

#[derive(AsChangeset)]
#[diesel(table_name = crate::db::schema::buttons)]
pub struct UpdateButton<'a> {
    pub label: Option<&'a str>,
    pub icon: Option<&'a str>,
    pub background: Option<&'a str>,
    pub position: Option<i32>,
    pub action: Option<&'a str>,
    pub mode: Option<&'a str>,
    pub execution_mode: Option<&'a str>,
    pub execution_config: Option<&'a str>,
    pub icon_fit: Option<&'a str>,
}

// Grid type for storing deck grid dimensions (e.g. "3,5")
#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
pub struct Grid {
    pub rows: usize,
    pub columns: usize,
}

impl Grid {
    pub fn new(rows: usize, columns: usize) -> Self {
        Self { rows, columns }
    }

    pub fn from_string(grid_str: &str) -> Option<Self> {
        let parts: Vec<&str> = grid_str.split(',').collect();
        if parts.len() == 2 {
            let rows = parts[0].parse().ok()?;
            let columns = parts[1].parse().ok()?;
            Some(Self::new(rows, columns))
        } else {
            None
        }
    }

    pub fn to_string(&self) -> String {
        format!("{},{}", self.rows, self.columns)
    }
}

// create settings struct
#[derive(Debug, Queryable, Selectable, Serialize, Associations, AsChangeset, Deserialize)]
#[diesel(belongs_to(Deck, foreign_key = deck_id))]
#[diesel(table_name = crate::db::schema::deck_settings)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct DeckSettings {
    pub id: i32,
    pub button_spacing: f32,
    pub grid_margin: f32,
    pub group_buttons: bool,
    pub maximize_button_space: bool,
    pub grid: String,
    pub deck_id: i32,
    pub buttons_padding: f32,
    pub sides_enabled: bool,
    pub orientation_sync_enabled: bool,
    pub active_theme: Option<i32>,
}

// Add NewDeckSettings for insertions
#[derive(Insertable)]
#[diesel(table_name = crate::db::schema::deck_settings)]
pub struct NewDeckSettings {
    pub button_spacing: f32,
    pub grid_margin: f32,
    pub group_buttons: bool,
    pub maximize_button_space: bool,
    pub grid: String,
    pub deck_id: i32,
    pub buttons_padding: f32,
    pub sides_enabled: bool,
    pub orientation_sync_enabled: bool,
    pub active_theme: Option<i32>,
}

// Add UpdateDeckSettings for updates
#[derive(AsChangeset)]
#[diesel(table_name = crate::db::schema::deck_settings)]
pub struct UpdateDeckSettings {
    pub button_spacing: Option<f32>,
    pub grid_margin: Option<f32>,
    pub group_buttons: Option<bool>,
    pub maximize_button_space: Option<bool>,
    pub grid: Option<String>,
    pub buttons_padding: Option<f32>,
    pub sides_enabled: Option<bool>,
    pub orientation_sync_enabled: Option<bool>,
    pub active_theme: Option<i32>,
}

// Enable association
#[derive(Debug, Serialize)]
pub struct DeckWithButtons {
    pub deck: Deck,
    pub buttons: Vec<Button>, // Buttons will be inside the deck
    pub settings: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DeckWithNestedButtons {
    pub id: i32,
    pub name: String,
    pub buttons: Vec<Button>,
    pub settings: Option<DeckSettings>,
    #[serde(serialize_with = "serialize_theme")] // Custom serialization
    pub theme: Option<DeckTheme>,
}

#[derive(Debug, Queryable, Serialize)]
#[diesel(table_name = crate::db::schema::active_deck)]
pub struct ActiveDeck {
    pub id: i32,
    pub deck_id: i32, // Foreign key to the `decks` table
}

#[derive(Insertable)]
#[diesel(table_name = crate::db::schema::active_deck)]
pub struct NewActiveDeck<'a> {
    pub deck_id: &'a i32, // Reference to the `deck_id` that should be inserted
}

#[derive(AsChangeset, Deserialize)]
#[diesel(table_name = crate::db::schema::active_deck)]
pub struct UpdateActiveDeck {
    pub deck_id: Option<i32>, // Optional field for updating the active deck
}

#[derive(Debug, Serialize, Deserialize, Clone, diesel_derive_enum::DbEnum)]
pub enum MacroType {
    RunOnce,
    RepeatWhilePressing,
    Switch,
    Queue,
}

#[derive(Queryable, Selectable, Debug, Serialize, Deserialize)]
#[diesel(table_name = crate::db::schema::macros)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Macro {
    pub id: i32,
    pub macro_chain: String, // Store the serialized macro steps as JSON
    pub name: String,
    pub macro_type: String,
}

#[derive(Insertable)]
#[diesel(table_name = crate::db::schema::macros)]
pub struct NewMacro<'a> {
    pub macro_chain: &'a str, // The macro steps as a JSON string
    pub name: &'a str,
    pub macro_type: &'a str,
}

#[derive(AsChangeset)]
#[diesel(table_name = crate::db::schema::macros)]
pub struct UpdateMacro<'a> {
    pub macro_chain: Option<&'a str>, // Optional update to the macro_chain
    pub name: Option<&'a str>,
    pub macro_type: Option<&'a str>,
}

// HTTP Request model
#[derive(Debug, Queryable, Selectable, Serialize)]
#[diesel(table_name = crate::db::schema::http_requests)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct HttpRequest {
    pub id: i32,
    pub name: String,
    pub method: String,
    pub url: String,
    pub headers: Option<String>,
    pub body: Option<String>,
    pub timeout_ms: Option<i32>,
    pub created_at: String,
    pub accepted_status_codes: Option<String>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = crate::db::schema::http_requests)]
pub struct NewHttpRequest<'a> {
    pub name: &'a str,
    pub method: &'a str,
    pub url: &'a str,
    pub headers: Option<&'a str>,
    pub body: Option<&'a str>,
    pub timeout_ms: Option<i32>,
    pub created_at: &'a str,
    pub accepted_status_codes: Option<&'a str>,
}

#[derive(Queryable, Selectable, Debug, Serialize, Deserialize, Clone)]
#[diesel(table_name = crate::db::schema::saved_actions)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct SavedActionBinding {
    pub id: i32,
    pub stable_id: String,
    pub integration_id: String,
    pub name: String,
    pub action: String,
    pub params: String,
    pub capability_version: i32,
    pub connection_selector: Option<String>,
    pub presentation: String,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = crate::db::schema::saved_actions)]
pub struct NewSavedActionBinding<'a> {
    pub stable_id: &'a str,
    pub integration_id: &'a str,
    pub name: &'a str,
    pub action: &'a str,
    pub params: &'a str,
    pub capability_version: i32,
    pub connection_selector: Option<&'a str>,
    pub presentation: &'a str,
}

#[derive(AsChangeset)]
#[diesel(table_name = crate::db::schema::saved_actions)]
pub struct UpdateSavedActionBinding<'a> {
    pub integration_id: Option<&'a str>,
    pub name: Option<&'a str>,
    pub action: Option<&'a str>,
    pub params: Option<&'a str>,
    pub capability_version: Option<i32>,
    pub connection_selector: Option<&'a str>,
    pub presentation: Option<&'a str>,
}

#[derive(Queryable, Selectable, Debug, Serialize, Deserialize, Clone)]
#[diesel(table_name = crate::db::schema::action_invocations)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[serde(rename_all = "camelCase")]
pub struct ActionInvocationRecord {
    pub invocation_id: String,
    pub parent_invocation_id: Option<String>,
    pub source: String,
    pub source_reference: Option<String>,
    pub workflow_run_id: Option<String>,
    pub node_id: Option<String>,
    pub resolved_action: String,
    pub integration_id: Option<String>,
    pub action_id: Option<String>,
    pub capability_version: Option<i32>,
    pub idempotency_key: Option<String>,
    pub resource_key: Option<String>,
    pub dispatch_status: String,
    pub receipt_semantics: Option<String>,
    pub request_id: Option<String>,
    pub provider_receipt_id: Option<String>,
    pub provider_outputs: Option<String>,
    pub normalized_status: Option<String>,
    pub error: Option<String>,
    pub recovery_decision: Option<String>,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub execution_time_ms: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = crate::db::schema::action_invocations)]
pub struct NewActionInvocation<'a> {
    pub invocation_id: &'a str,
    pub parent_invocation_id: Option<&'a str>,
    pub source: &'a str,
    pub source_reference: Option<&'a str>,
    pub workflow_run_id: Option<&'a str>,
    pub node_id: Option<&'a str>,
    pub resolved_action: &'a str,
    pub integration_id: Option<&'a str>,
    pub action_id: Option<&'a str>,
    pub capability_version: Option<i32>,
    pub idempotency_key: Option<&'a str>,
    pub resource_key: Option<&'a str>,
    pub dispatch_status: &'a str,
    pub receipt_semantics: Option<&'a str>,
    pub request_id: Option<&'a str>,
    pub provider_receipt_id: Option<&'a str>,
    pub provider_outputs: Option<&'a str>,
    pub normalized_status: Option<&'a str>,
    pub error: Option<&'a str>,
    pub recovery_decision: Option<&'a str>,
    pub started_at: &'a str,
    pub completed_at: Option<&'a str>,
    pub execution_time_ms: i64,
    pub created_at: &'a str,
    pub updated_at: &'a str,
}

#[derive(Queryable, Selectable, Debug, Serialize, Deserialize, Clone)]
#[diesel(table_name = crate::db::schema::overlay_media_queue)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[serde(rename_all = "camelCase")]
pub struct OverlayMediaQueueRecord {
    pub id: String,
    pub invocation_id: String,
    pub queue_kind: String,
    pub command: String,
    pub payload: String,
    pub status: String,
    pub receipt_phase: Option<String>,
    pub provider_receipt_id: Option<String>,
    pub error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = crate::db::schema::overlay_media_queue)]
pub struct NewOverlayMediaQueueRecord<'a> {
    pub id: &'a str,
    pub invocation_id: &'a str,
    pub queue_kind: &'a str,
    pub command: &'a str,
    pub payload: &'a str,
    pub status: &'a str,
    pub receipt_phase: Option<&'a str>,
    pub provider_receipt_id: Option<&'a str>,
    pub error: Option<&'a str>,
    pub created_at: &'a str,
    pub updated_at: &'a str,
    pub started_at: Option<&'a str>,
    pub completed_at: Option<&'a str>,
}

/// Durable acceptance receipt for a consequential command received from a
/// paired remote peer. This is an ingress record, not a claim that the
/// downstream provider applied the requested effect.
#[derive(Queryable, Selectable, Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[diesel(table_name = crate::db::schema::remote_command_receipts)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[serde(rename_all = "camelCase")]
pub struct RemoteCommandReceiptRecord {
    pub acceptance_id: String,
    pub peer_client_id: String,
    pub key_fingerprint: String,
    pub message_id: String,
    pub nonce: String,
    pub payload_digest: String,
    pub transport: String,
    pub status: String,
    pub sent_at: i64,
    pub expires_at: i64,
    pub receipt_json: String,
    pub accepted_at: String,
    pub updated_at: String,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = crate::db::schema::remote_command_receipts)]
pub struct NewRemoteCommandReceipt<'a> {
    pub acceptance_id: &'a str,
    pub peer_client_id: &'a str,
    pub key_fingerprint: &'a str,
    pub message_id: &'a str,
    pub nonce: &'a str,
    pub payload_digest: &'a str,
    pub transport: &'a str,
    pub status: &'a str,
    pub sent_at: i64,
    pub expires_at: i64,
    pub receipt_json: &'a str,
    pub accepted_at: &'a str,
    pub updated_at: &'a str,
}

// ============================================================================
// PACKAGE PERMISSION GRANTS — durable package-level consent.
// See docs/WIDGET_CAPABILITY_ARCHITECTURE.md §6.5 (threat model).
// ============================================================================

/// One durable consent record per package, covering runtime permissions and all
/// widget contributions in the package.
#[derive(Queryable, Selectable, Debug, Serialize, Deserialize, Clone)]
#[diesel(table_name = crate::db::schema::package_permission_grants)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct PackagePermissionGrant {
    /// Stable package id (primary key).
    pub package_id: String,
    /// Full manifest JSON (for re-dialog diffs on update).
    pub manifest_json: String,
    /// Granted Permission set, JSON-serialized (compiled from capabilities).
    pub permissions_json: String,
    /// sha256 of widget content — for verify-on-load (tamper detection).
    pub content_hash: String,
    /// "builtin" | "verified" | "community".
    pub trust: String,
    /// RFC3339 timestamp of when the grant was made.
    pub granted_at: String,
}

/// Insertable package consent record.
#[derive(Insertable, Debug)]
#[diesel(table_name = crate::db::schema::package_permission_grants)]
pub struct NewPackagePermissionGrant<'a> {
    pub package_id: &'a str,
    pub manifest_json: &'a str,
    pub permissions_json: &'a str,
    pub content_hash: &'a str,
    pub trust: &'a str,
    pub granted_at: &'a str,
}

/// Local installation provenance. Consent remains in
/// `package_permission_grants`; this record identifies the exact installed
/// artifact and its recovery/update policy.
#[derive(Queryable, Selectable, Debug, Serialize, Deserialize, Clone)]
#[diesel(table_name = crate::db::schema::installed_package_releases)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct InstalledPackageRelease {
    pub package_id: String,
    pub installed_version: String,
    pub source: String,
    pub release_id: Option<String>,
    pub artifact_sha256: String,
    pub marketplace_key_id: Option<String>,
    pub installed_at: String,
    pub updated_at: String,
    pub auto_update: bool,
    pub lifecycle_state: String,
    pub last_known_good_version: Option<String>,
    pub last_known_good_release_id: Option<String>,
    pub last_known_good_artifact_sha256: Option<String>,
    pub last_known_good_marketplace_key_id: Option<String>,
    pub installed_content_sha256: Option<String>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = crate::db::schema::installed_package_releases)]
pub struct NewInstalledPackageRelease<'a> {
    pub package_id: &'a str,
    pub installed_version: &'a str,
    pub source: &'a str,
    pub release_id: Option<&'a str>,
    pub artifact_sha256: &'a str,
    pub marketplace_key_id: Option<&'a str>,
    pub installed_at: &'a str,
    pub updated_at: &'a str,
    pub auto_update: bool,
    pub lifecycle_state: &'a str,
    pub last_known_good_version: Option<&'a str>,
    pub last_known_good_release_id: Option<&'a str>,
    pub last_known_good_artifact_sha256: Option<&'a str>,
    pub last_known_good_marketplace_key_id: Option<&'a str>,
    pub installed_content_sha256: Option<&'a str>,
}

// Focus Window action models
#[derive(Queryable, Selectable, Debug, Serialize, Deserialize)]
#[diesel(table_name = crate::db::schema::focus_windows)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct FocusWindow {
    pub id: i32,
    pub name: String,
    pub window_title: String,
    pub process_name: String,
    pub match_mode: String,
    pub created_at: String,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = crate::db::schema::focus_windows)]
pub struct NewFocusWindow<'a> {
    pub name: &'a str,
    pub window_title: &'a str,
    pub process_name: &'a str,
    pub match_mode: &'a str,
}

#[derive(AsChangeset)]
#[diesel(table_name = crate::db::schema::focus_windows)]
pub struct UpdateFocusWindow<'a> {
    pub name: Option<&'a str>,
    pub window_title: Option<&'a str>,
    pub process_name: Option<&'a str>,
    pub match_mode: Option<&'a str>,
}

// Programs (dedicated programs table)
#[derive(Queryable, Selectable, Debug, Serialize, Deserialize)]
#[diesel(table_name = crate::db::schema::programs)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Program {
    pub id: i32,
    pub name: String,
    pub executable: String,
    pub args: Option<String>,
    pub working_dir: Option<String>,
    pub env_vars: Option<String>,
    pub timeout_ms: Option<i32>,
    pub show_terminal: bool,
    pub program_type: String, // Stored as string, converted to enum at boundaries
    pub description: Option<String>,
    pub custom_icon: Option<String>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = crate::db::schema::programs)]
pub struct NewProgram<'a> {
    pub name: &'a str,
    pub executable: &'a str,
    pub args: Option<&'a str>,
    pub working_dir: Option<&'a str>,
    pub env_vars: Option<&'a str>,
    pub timeout_ms: Option<i32>,
    pub show_terminal: bool,
    pub program_type: &'a str,
    pub description: Option<&'a str>,
    pub custom_icon: Option<&'a str>,
}

#[derive(AsChangeset)]
#[diesel(table_name = crate::db::schema::programs)]
pub struct UpdateProgram<'a> {
    pub name: Option<&'a str>,
    pub executable: Option<&'a str>,
    pub args: Option<Option<&'a str>>,
    pub working_dir: Option<Option<&'a str>>,
    pub env_vars: Option<Option<&'a str>>,
    pub timeout_ms: Option<Option<i32>>,
    pub show_terminal: Option<bool>,
    pub program_type: Option<&'a str>,
    pub description: Option<Option<&'a str>>,
    pub custom_icon: Option<Option<&'a str>>,
}

// Enhanced Program with computed fields for UI
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProgramWithMeta {
    // Original database fields
    pub id: i32,
    pub name: String,
    pub executable: String,
    pub args: Option<String>,
    pub working_dir: Option<String>,
    pub env_vars: Option<String>,
    pub timeout_ms: Option<i32>,
    pub show_terminal: bool,
    pub program_type: String, // Stored as string, converted to enum at boundaries
    pub description: Option<String>,
    pub custom_icon: Option<String>,

    // Computed fields
    pub icon_data: Option<String>, // Base64 icon data or null
    pub category: String,          // For UI grouping
}

impl From<Program> for ProgramWithMeta {
    fn from(program: Program) -> Self {
        // Use the stored program_type instead of runtime detection
        let category = match program.program_type.as_str() {
            "executable" => "Applications",
            "script" => "Scripts",
            "command" => "System Commands",
            "shell" => "Shell Commands",
            _ => "Other",
        };

        ProgramWithMeta {
            id: program.id,
            name: program.name,
            executable: program.executable,
            args: program.args,
            working_dir: program.working_dir,
            env_vars: program.env_vars,
            timeout_ms: program.timeout_ms,
            show_terminal: program.show_terminal,
            program_type: program.program_type,
            description: program.description,
            custom_icon: program.custom_icon,
            icon_data: None, // Will be populated separately if needed
            category: category.to_string(),
        }
    }
}

// #[derive(Queryable, Selectable, Debug, Serialize, Deserialize)]
// #[diesel(table_name = crate::db::schema::system_controls)]
// #[diesel(check_for_backend(diesel::sqlite::Sqlite))]
// pub struct SystemControl {
//     pub id: i32,
//     pub name: String,
//     pub action: String,
//     pub params: String,
// }

// #[derive(Insertable, Debug)]
// #[diesel(table_name = crate::db::schema::system_controls)]
// pub struct NewSystemControl<'a> {
//     pub name: &'a str,
//     pub action: &'a str,
//     pub params: &'a str,
// }

// #[derive(AsChangeset)]
// #[diesel(table_name = crate::db::schema::system_controls)]
// pub struct UpdateSystemControl<'a> {
//     pub name: Option<&'a str>, // Optional update to the macro_chain
//     pub action: Option<&'a str>,
//     pub params: Option<&'a str>,
// }

// Flat DeckTheme for Diesel operations
#[derive(Debug, Queryable, Selectable, Serialize, AsChangeset, Deserialize, Insertable)]
#[diesel(table_name = crate::db::schema::themes)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct DeckTheme {
    pub id: i32,
    pub name: String,
    pub background_css: String,
    pub rotary_button_knob_css: String,
    pub rotary_button_pointer_css: String,
    pub click_button_button_css: String,
    pub click_button_icon_css: String,
    pub click_button_selected_css: String,
    pub rotary_button_ready_css: String,
}

impl DeckTheme {
    pub fn get(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "name": self.name,
            "backgroundCss": self.background_css,
            "components": {
                "rotaryButton": {
                    "knobCss": self.rotary_button_knob_css,
                    "pointerCss": self.rotary_button_pointer_css,
                    "readyCss": self.rotary_button_ready_css
                },
                "clickButton": {
                    "buttonCss": self.click_button_button_css,
                    "iconCss": self.click_button_icon_css,
                    "selectedCss": self.click_button_selected_css
                }
            }
        })
    }
}

// New struct for deserializing nested JSON input
#[derive(Debug, Deserialize)]
pub struct ThemeInput {
    pub id: i32,
    pub name: String,
    #[serde(rename = "backgroundCss")]
    pub background_css: String,
    pub components: ComponentsInput,
}

#[derive(Debug, Deserialize)]
pub struct ComponentsInput {
    #[serde(rename = "rotaryButton")]
    pub rotary_button: RotaryButtonInput,
    #[serde(rename = "clickButton")]
    pub click_button: ClickButtonInput,
}

#[derive(Debug, Deserialize)]
pub struct RotaryButtonInput {
    #[serde(rename = "knobCss")]
    pub knob_css: String,
    #[serde(rename = "pointerCss")]
    pub pointer_css: String,
    #[serde(rename = "readyCss")]
    pub ready_css: String,
}

#[derive(Debug, Deserialize)]
pub struct ClickButtonInput {
    #[serde(rename = "buttonCss")]
    pub button_css: String,
    #[serde(rename = "iconCss")]
    pub icon_css: String,
    #[serde(rename = "selectedCss")]
    pub selected_css: String,
}

impl From<ThemeInput> for DeckTheme {
    fn from(input: ThemeInput) -> Self {
        DeckTheme {
            id: input.id,
            name: input.name,
            background_css: input.background_css,
            rotary_button_knob_css: input.components.rotary_button.knob_css,
            rotary_button_pointer_css: input.components.rotary_button.pointer_css,
            rotary_button_ready_css: input.components.rotary_button.ready_css,
            click_button_button_css: input.components.click_button.button_css,
            click_button_icon_css: input.components.click_button.icon_css,
            click_button_selected_css: input.components.click_button.selected_css,
        }
    }
}

#[derive(Insertable)]
#[diesel(table_name = crate::db::schema::themes)]
pub struct NewDeckTheme<'a> {
    pub name: &'a str,
    pub background_css: &'a str,
    pub rotary_button_knob_css: &'a str,
    pub rotary_button_pointer_css: &'a str,
    pub click_button_button_css: &'a str,
    pub click_button_icon_css: &'a str,
    pub click_button_selected_css: &'a str,
    pub rotary_button_ready_css: &'a str,
}

#[derive(AsChangeset)]
#[diesel(table_name = crate::db::schema::themes)]
pub struct UpdateDeckTheme<'a> {
    pub name: Option<&'a str>,
    pub background_css: Option<&'a str>,
    pub rotary_button_knob_css: Option<&'a str>,
    pub rotary_button_pointer_css: Option<&'a str>,
    pub click_button_button_css: Option<&'a str>,
    pub click_button_icon_css: Option<&'a str>,
    pub click_button_selected_css: Option<&'a str>,
    pub rotary_button_ready_css: Option<&'a str>,
}

// Flow models for database integration
#[derive(Debug, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = crate::db::schema::flows)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Flow {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub data: String, // JSON serialized flow definition
    pub created_at: String,
    pub updated_at: String,
    pub created_by: Option<String>,
    pub tags: Option<String>, // JSON serialized tags array
    pub version: String,
}

#[derive(Insertable, Serialize, Deserialize)]
#[diesel(table_name = crate::db::schema::flows)]
pub struct NewFlow<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub description: Option<&'a str>,
    pub data: &'a str,
    pub created_at: &'a str,
    pub updated_at: &'a str,
    pub created_by: Option<&'a str>,
    pub tags: Option<&'a str>,
    pub version: &'a str,
}

#[derive(AsChangeset, Serialize, Deserialize)]
#[diesel(table_name = crate::db::schema::flows)]
pub struct UpdateFlow<'a> {
    pub name: Option<&'a str>,
    pub description: Option<&'a str>,
    pub data: Option<&'a str>,
    pub updated_at: &'a str,
    pub tags: Option<&'a str>,
    pub version: Option<&'a str>,
}

// Flow summary for UI display
#[derive(Debug, Serialize, Deserialize)]
pub struct FlowSummary {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub tags: Vec<String>,
    pub version: String,
}

impl Flow {
    pub fn to_summary(&self) -> FlowSummary {
        let tags = self
            .tags
            .as_ref()
            .and_then(|t| serde_json::from_str::<Vec<String>>(t).ok())
            .unwrap_or_default();

        FlowSummary {
            id: self.id.clone(),
            name: self.name.clone(),
            description: self.description.clone(),
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
            tags,
            version: self.version.clone(),
        }
    }
}

fn serialize_theme<S>(theme: &Option<DeckTheme>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match theme {
        Some(theme) => serializer.serialize_some(&theme.get()),
        None => serializer.serialize_none(),
    }
}

// ============================================================================
// WEBHOOK MODELS
// ============================================================================

/// Webhook configuration stored in database
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = crate::db::schema::webhooks)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Webhook {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub service: String,
    pub endpoint_url: String,
    pub inbound_secret: Option<String>, // External service secret (Twitch, GitHub) for verifying webhooks FROM them
    pub action: Option<String>,         // JSON serialized UniversalAction
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
    pub subscription_id: Option<String>, // External service subscription ID (e.g., Twitch EventSub)
    // Phase 1 features (n8n parity)
    pub test_url: Option<String>,        // Test webhook URL
    pub production_url: Option<String>,  // Production webhook URL
    pub is_test_active: bool,            // Is test mode currently active
    pub test_expires_at: Option<String>, // When test mode expires (120s timeout)
    pub response_mode: String, // ResponseMode: onReceived, lastNode, responseNode, streaming
    pub allowed_methods: String, // JSON array: ["GET", "POST"]
}

/// New webhook for insertion
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = crate::db::schema::webhooks)]
pub struct NewWebhook<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub description: Option<&'a str>,
    pub service: &'a str,
    pub endpoint_url: &'a str,
    pub inbound_secret: Option<&'a str>,
    pub action: Option<&'a str>,
    pub enabled: bool,
    pub created_at: &'a str,
    pub updated_at: &'a str,
    pub subscription_id: Option<&'a str>,
    // Phase 1 fields
    pub test_url: Option<&'a str>,
    pub production_url: Option<&'a str>,
    pub is_test_active: bool,
    pub test_expires_at: Option<&'a str>,
    pub response_mode: &'a str,
    pub allowed_methods: &'a str,
}

/// Update webhook fields
#[derive(Debug, Clone, AsChangeset)]
#[diesel(table_name = crate::db::schema::webhooks)]
pub struct UpdateWebhook<'a> {
    pub name: Option<&'a str>,
    pub description: Option<&'a str>,
    pub service: Option<&'a str>,
    pub inbound_secret: Option<&'a str>,
    pub action: Option<&'a str>,
    pub enabled: Option<bool>,
    pub updated_at: &'a str,
    pub subscription_id: Option<&'a str>,
    // Phase 1 fields
    pub test_url: Option<&'a str>,
    pub production_url: Option<&'a str>,
    pub is_test_active: Option<bool>,
    pub test_expires_at: Option<&'a str>,
    pub response_mode: Option<&'a str>,
    pub allowed_methods: Option<&'a str>,
}

/// Webhook event log entry
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = crate::db::schema::webhook_events)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct WebhookEvent {
    pub id: String,
    pub webhook_id: String,
    pub event_type: String,      // "received" or "sent"
    pub status: String,          // "success" or "error"
    pub payload: Option<String>, // JSON payload
    pub response_code: Option<i32>,
    pub response_body: Option<String>,
    pub duration_ms: Option<i32>,
    pub error_message: Option<String>,
    pub created_at: String,
}

/// New webhook event for insertion
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = crate::db::schema::webhook_events)]
pub struct NewWebhookEvent<'a> {
    pub id: &'a str,
    pub webhook_id: &'a str,
    pub event_type: &'a str,
    pub status: &'a str,
    pub payload: Option<&'a str>,
    pub response_code: Option<i32>,
    pub response_body: Option<&'a str>,
    pub duration_ms: Option<i32>,
    pub error_message: Option<&'a str>,
    pub created_at: &'a str,
}

// ============================================================================
// STREAM SESSION MODELS
// ============================================================================

/// Stream session stored in database
#[derive(Debug, Clone, Queryable, Selectable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = crate::db::schema::stream_sessions)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct StreamSessionDb {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub platforms: String, // JSON serialized Vec<PlatformStream>
    pub commands: String,  // JSON serialized Vec<SessionCommand>
    pub settings: String,  // JSON serialized SessionSettings
    pub status: String,    // SessionStatus as string
    pub created_at: String,
    pub updated_at: String,
    pub current_run_started_at: Option<String>,
    pub paused_at: Option<String>,
    pub duration: Option<i32>,
    pub message_count: Option<i32>,
    pub command_executions: Option<i32>,
    pub runs: Option<String>, // JSON serialized Vec<StreamRun>
    pub total_runs: Option<i32>,
    pub last_used_at: Option<String>,
    pub total_duration: Option<i32>,
    pub total_messages: Option<i32>,
    pub total_commands: Option<i32>,
}

/// New stream session for insertion
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = crate::db::schema::stream_sessions)]
pub struct NewStreamSession<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub description: Option<&'a str>,
    pub platforms: &'a str,
    pub commands: &'a str,
    pub settings: &'a str,
    pub status: &'a str,
    pub created_at: &'a str,
    pub updated_at: &'a str,
    pub current_run_started_at: Option<&'a str>,
    pub paused_at: Option<&'a str>,
    pub duration: Option<i32>,
    pub message_count: Option<i32>,
    pub command_executions: Option<i32>,
    pub runs: Option<&'a str>,
    pub total_runs: Option<i32>,
    pub last_used_at: Option<&'a str>,
    pub total_duration: Option<i32>,
    pub total_messages: Option<i32>,
    pub total_commands: Option<i32>,
}

/// Update stream session fields
#[derive(Debug, Clone, AsChangeset)]
#[diesel(table_name = crate::db::schema::stream_sessions)]
pub struct UpdateStreamSession<'a> {
    pub name: Option<&'a str>,
    pub description: Option<&'a str>,
    pub platforms: Option<&'a str>,
    pub commands: Option<&'a str>,
    pub settings: Option<&'a str>,
    pub status: Option<&'a str>,
    pub updated_at: &'a str,
    pub current_run_started_at: Option<&'a str>,
    pub paused_at: Option<&'a str>,
    pub duration: Option<i32>,
    pub message_count: Option<i32>,
    pub command_executions: Option<i32>,
    pub runs: Option<&'a str>,
    pub total_runs: Option<i32>,
    pub last_used_at: Option<&'a str>,
    pub total_duration: Option<i32>,
    pub total_messages: Option<i32>,
    pub total_commands: Option<i32>,
}
// ============================================================================
// Chat Messages (Streaming)
// ============================================================================

#[derive(Queryable, Selectable, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = crate::db::schema::chat_messages)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct ChatMessageDb {
    pub id: String,
    pub account_id: i32,
    pub session_id: Option<String>,
    pub session_run_index: Option<i32>,
    pub platform: String,
    pub user_id: String,
    pub user_name: String,
    pub user_display_name: String,
    pub message: String,
    pub timestamp: String,
    pub badges: Option<String>, // JSON
    pub is_moderator: i32,      // SQLite boolean
    pub is_subscriber: i32,
    pub is_vip: i32,
    pub platform_color: Option<String>,
    pub emotes: Option<String>,   // JSON
    pub mentions: Option<String>, // JSON
    pub created_at: String,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = crate::db::schema::chat_messages)]
pub struct NewChatMessage<'a> {
    pub id: &'a str,
    pub account_id: i32,
    pub session_id: Option<&'a str>,
    pub session_run_index: Option<i32>,
    pub platform: &'a str,
    pub user_id: &'a str,
    pub user_name: &'a str,
    pub user_display_name: &'a str,
    pub message: &'a str,
    pub timestamp: &'a str,
    pub badges: Option<&'a str>, // JSON string
    pub is_moderator: i32,
    pub is_subscriber: i32,
    pub is_vip: i32,
    pub platform_color: Option<&'a str>,
    pub emotes: Option<&'a str>,   // JSON string
    pub mentions: Option<&'a str>, // JSON string
}
