use crate::db::Database;
use enigo::Enigo;
use serde_json::{json, Value};
/// Shared Command Handler Module
///
/// This module provides a unified command execution interface that can be used by:
/// 1. Direct WebSocket connections (local WiFi)
/// 2. Relay connections (internet-based)
///
/// The handler processes commands from mobile clients and executes them with proper
/// validation, E2EE verification, and execution mode handling.
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::broadcast;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::Message;
use vortideck_core::core::PlatformContext;
use vortideck_core::db::{Action, DeckWidgetDocument};
use vortideck_core::integrations::builtin::obs::ObsIntegration;
use vortideck_core::services::flow::commands::FlowExecutorState;
use vortideck_core::services::flow::executor::{
    ApprovalActor, ApprovalDecision, ApprovalDecisionRequest,
};

/// Parameters needed for command execution
#[derive(Clone)]
pub struct CommandExecutionContext {
    pub json: Value,
    pub obs_integration: Arc<ObsIntegration>,
    pub db: Database,
    pub tx: broadcast::Sender<Message>,
    pub enigo: Arc<Mutex<Enigo>>,
    pub app_handle: AppHandle,
}

/// Resolve an approval from an already-authenticated remote transport. Direct
/// and relay WebSockets call this same adapter after their HMAC/replay checks;
/// the durable executor remains the sole decision authority.
pub async fn handle_workflow_approval_decision(
    app_handle: &AppHandle,
    json: &Value,
    actor_id: &str,
    channel: &str,
) -> Result<Value, String> {
    let wake_id = json
        .get("wakeId")
        .and_then(Value::as_str)
        .ok_or("workflow_approval_decision requires wakeId")?;
    let decision = match json.get("decision").and_then(Value::as_str) {
        Some("approved") => ApprovalDecision::Approved,
        Some("denied") => ApprovalDecision::Denied,
        _ => return Err("Remote approval decisions must be approved or denied".to_string()),
    };
    let comment = json
        .get("comment")
        .and_then(Value::as_str)
        .map(str::to_string);
    let state = app_handle
        .try_state::<FlowExecutorState>()
        .ok_or("Flow executor state is unavailable")?;
    let guard = state.executor.lock().await;
    let executor = guard.as_ref().ok_or("Flow executor is not initialized")?;
    let outcome = executor
        .resolve_approval_wake(
            wake_id,
            ApprovalDecisionRequest {
                decision,
                actor: ApprovalActor {
                    id: actor_id.to_string(),
                    display_name: None,
                    channel: channel.to_string(),
                },
                comment,
            },
        )
        .await?;
    serde_json::to_value(outcome).map_err(|error| error.to_string())
}

pub async fn workflow_approvals_snapshot(
    app_handle: &AppHandle,
    actor_id: &str,
    channel: &str,
) -> Result<Value, String> {
    let state = app_handle
        .try_state::<FlowExecutorState>()
        .ok_or("Flow executor state is unavailable")?;
    let guard = state.executor.lock().await;
    let executor = guard.as_ref().ok_or("Flow executor is not initialized")?;
    let actor = ApprovalActor {
        id: actor_id.to_string(),
        display_name: None,
        channel: channel.to_string(),
    };
    let approvals: Vec<_> = executor
        .list_open_workflow_wakes()
        .await?
        .into_iter()
        .filter(|wake| {
            wake.kind == vortideck_core::services::flow::executor::WorkflowWakeKind::Approval
        })
        .filter(|wake| {
            vortideck_core::services::flow::executor::FlowExecutor::approval_request_details(wake)
                .is_ok_and(|request| {
                    request
                        .requested_channels
                        .iter()
                        .any(|requested| requested == "mobile")
                        && request.decision_policy.permits(&actor)
                })
        })
        .collect();
    Ok(json!({
        "action": "workflow_approvals_snapshot",
        "protocolVersion": 1,
        "approvals": approvals,
    }))
}

async fn active_widget_document(db: &Database) -> Option<DeckWidgetDocument> {
    let active_deck = db.get_active_deck().await?;
    if let Some(document) = crate::widget_document_cache::get(active_deck.deck_id).await {
        return Some(document);
    }

    let document = db
        .get_deck_widget_document(active_deck.deck_id)
        .await
        .ok()?;
    crate::widget_document_cache::store(document.clone()).await;
    Some(document)
}

fn parse_widget_cache_inventory(
    message: &Value,
) -> Option<(i32, HashSet<String>, HashSet<String>)> {
    if message.get("protocolVersion").and_then(Value::as_u64) != Some(2)
        || !message
            .get("schemaVersions")
            .and_then(Value::as_array)
            .is_some_and(|versions| versions.iter().any(|version| version.as_u64() == Some(1)))
    {
        return None;
    }
    let deck_id = i32::try_from(message.get("deckId")?.as_i64()?).ok()?;
    let parse_hashes = |field: &str| -> Option<HashSet<String>> {
        let values = message.get(field)?.as_array()?;
        if values.len() > 256 {
            return None;
        }
        values
            .iter()
            .map(|value| value.as_str())
            .map(|value| {
                value
                    .filter(|hash| {
                        hash.len() == 64
                            && hash.chars().all(|character| {
                                character.is_ascii_hexdigit() && !character.is_ascii_uppercase()
                            })
                    })
                    .map(str::to_string)
            })
            .collect()
    };
    Some((
        deck_id,
        parse_hashes("definitionHashes")?,
        parse_hashes("assetHashes")?,
    ))
}

pub async fn handle_widget_cache_inventory(ctx: CommandExecutionContext) {
    let Some((deck_id, definition_hashes, asset_hashes)) = parse_widget_cache_inventory(&ctx.json)
    else {
        return;
    };
    let Some(objects) = crate::action_execution::build_deck_widget_objects(
        &ctx.db,
        deck_id,
        &definition_hashes,
        &asset_hashes,
    )
    .await
    else {
        return;
    };
    let Some(manifest) =
        crate::action_execution::build_deck_widget_manifest(&ctx.db, deck_id).await
    else {
        return;
    };
    let Some(state_manager) = ctx
        .app_handle
        .try_state::<Arc<vortideck_core::core::StateManager>>()
    else {
        return;
    };
    crate::broadcast_helper::broadcast_to_all_clients(
        &ctx.app_handle,
        state_manager.inner(),
        manifest,
    )
    .await;
    crate::broadcast_helper::broadcast_to_all_clients(
        &ctx.app_handle,
        state_manager.inner(),
        objects,
    )
    .await;
}

/// Handle a "command" action from a mobile client
///
/// This function:
/// 1. Parses the command string into an Action enum
/// 2. Handles OBS actions via the OBS integration
/// 3. Validates button execution against plan limits
/// 4. Executes the action via the universal execution engine
///
/// # Arguments
/// * `ctx` - Command execution context with all necessary state
pub async fn handle_command_action(ctx: CommandExecutionContext) {
    let json = &ctx.json;

    if let Some(command_str) = json["command"].as_str() {
        // We expect the "command" to be a valid Action enum serialized as JSON
        match serde_json::from_str::<Action>(command_str) {
            Ok(action) => {
                // Get the value if it exists
                let value = json.get("value").and_then(|v| v.as_f64());
                let button_id = json.get("buttonId").and_then(|v| v.as_u64());
                println!("button_id: {}", button_id.unwrap_or_default());

                // Guard: If no buttonId was provided, attempt to resolve by matching action string in active deck
                if button_id.is_none() && json.get("widgetInstanceId").is_none() {
                    if let Some(active_deck) =
                        ctx.db.get_active_deck_with_buttons(false, None).await
                    {
                        let mut buttons = active_deck.buttons;
                        buttons.sort_by_key(|b| b.position);
                        let matches: Vec<(usize, &vortideck_core::db::models::Button)> = buttons
                            .iter()
                            .enumerate()
                            .filter(|(_, b)| b.action.as_deref() == Some(command_str))
                            .collect();
                        if matches.len() == 1 {
                            let (idx, matched) = matches[0];
                            use vortideck_core::features::jwt_features::get_jwt_provider;
                            let provider = get_jwt_provider();
                            let plan = provider
                                .get_user_plan()
                                .unwrap_or_else(|| "guest".to_string());
                            let effective_limit =
                                provider.get_effective_feature_limit("ui_buttons") as usize;
                            println!(
                                "\u{1F4CA} Button exec guard (resolved): plan={}, ui_buttons_limit={}, index={}, button_id={}",
                                plan, effective_limit, idx, matched.id
                            );
                            if idx >= effective_limit {
                                println!(
                                    "\u{1F512} Blocking execution of resolved button {} (index {}) due to plan limit (allowed: {})",
                                    matched.id, idx, effective_limit
                                );
                                return; // Early return instead of continue
                            } else {
                                println!(
                                    "\u{2705} Allowed execution for resolved button {} (index {}) within limit {}",
                                    matched.id, idx, effective_limit
                                );
                            }
                        } else {
                            println!(
                                "\u{26A0}\u{FE0F} Could not uniquely resolve button from command string (matches={}); skipping positional guard",
                                matches.len()
                            );
                        }
                    }
                }

                // 🚀 UNIVERSAL ARCHITECTURE: Use universal execution (with error handling)
                let platform_context = match ctx.app_handle.try_state::<Arc<PlatformContext>>() {
                    Some(context) => context.inner().clone(),
                    None => {
                        println!("❌ PlatformContext not available - cannot execute action");
                        return;
                    }
                };

                // 📊 Track button press
                crate::analytics::capture(
                    &ctx.app_handle,
                    crate::analytics::AnalyticsEvent::new("button_pressed")
                        .prop(
                            "action_type",
                            format!("{:?}", action).chars().take(50).collect::<String>(),
                        )
                        .prop("button_id", button_id.unwrap_or(0) as u64)
                        .prop(
                            "widget_instance_id",
                            json.get("widgetInstanceId")
                                .and_then(Value::as_str)
                                .unwrap_or("legacy")
                                .to_string(),
                        ),
                );

                // Call execute_action_universal from main (since this is part of bin crate when used from main.rs)
                // This will be resolved at compile time since main.rs includes this module
                super::execute_action_universal(
                    action,
                    value,
                    button_id.map(|id| id.to_string()),
                    Arc::new(ctx.db.clone()),
                    ctx.tx.clone(),
                    json.get("widgetExecutionMode")
                        .and_then(Value::as_str)
                        .map(|mode| crate::action_execution::WidgetExecutionContext {
                            instance_id: json
                                .get("widgetInstanceId")
                                .and_then(Value::as_str)
                                .unwrap_or("unknown-widget")
                                .to_string(),
                            deck_id: json
                                .get("widgetDeckId")
                                .and_then(Value::as_i64)
                                .map(|value| value.to_string())
                                .unwrap_or_else(|| "unknown".to_string()),
                            mode: mode.to_string(),
                            config: json.get("widgetExecutionConfig").cloned(),
                            requested_values: json
                                .get("widgetRequestedValues")
                                .cloned()
                                .unwrap_or_else(|| json!({})),
                            commit_requested_on_success: json
                                .get("widgetCommitRequestedOnSuccess")
                                .and_then(Value::as_bool)
                                .unwrap_or(false),
                        }),
                )
                .await;
            }
            Err(e) => {
                println!("Invalid command format: {} - Error: {}", command_str, e);
                log::error!("Invalid command format: {} - Error: {}", command_str, e);
            }
        }
    } else {
        println!("No command found in message");
        log::error!("No command found in message");
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct WidgetRepeatKey {
    deck_id: i32,
    instance_id: String,
    port_id: String,
    client_id: String,
    connection_session_id: String,
    interaction_session_id: String,
}

#[derive(Clone, Debug)]
pub struct WidgetIntentSource {
    pub client_id: String,
    pub connection_session_id: String,
}

fn widget_repeat_tasks() -> &'static Mutex<HashMap<WidgetRepeatKey, JoinHandle<()>>> {
    static TASKS: OnceLock<Mutex<HashMap<WidgetRepeatKey, JoinHandle<()>>>> = OnceLock::new();
    TASKS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(super) async fn cancel_all_widget_repeats() {
    let mut tasks = widget_repeat_tasks().lock().await;
    for (_, task) in tasks.drain() {
        task.abort();
    }
}

pub(super) async fn cancel_removed_widget_repeats(deck_id: i32, removed: &[String]) {
    let mut tasks = widget_repeat_tasks().lock().await;
    let keys: Vec<_> = tasks
        .keys()
        .filter(|key| deck_id == key.deck_id && removed.contains(&key.instance_id))
        .cloned()
        .collect();
    for key in keys {
        if let Some(task) = tasks.remove(&key) {
            task.abort();
        }
    }
    drop(tasks);
    reset_widget_runtime_items(deck_id, removed).await;
}

pub(super) async fn reset_widget_runtime_items(deck_id: i32, instance_ids: &[String]) {
    if instance_ids.is_empty() {
        return;
    }
    let ids = instance_ids
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let preview_prefixes = instance_ids
        .iter()
        .map(|instance_id| format!("{deck_id}:{instance_id}:"))
        .collect::<Vec<_>>();
    widget_runtime_records().lock().await.retain(|key, _| {
        let Some((record_deck, instance_id)) = key.split_once(':') else {
            return true;
        };
        record_deck.parse::<i32>().ok() != Some(deck_id) || !ids.contains(instance_id)
    });
    widget_public_state_paths().lock().await.retain(|key, _| {
        let Some((record_deck, instance_id)) = key.split_once(':') else {
            return true;
        };
        record_deck.parse::<i32>().ok() != Some(deck_id) || !ids.contains(instance_id)
    });
    widget_preview_times().lock().await.retain(|key, _| {
        !preview_prefixes
            .iter()
            .any(|prefix| key.starts_with(prefix))
    });
    widget_intent_sequences()
        .lock()
        .await
        .entries
        .retain(|key, _| key.deck_id != deck_id || !ids.contains(key.instance_id.as_str()));
}

pub(super) async fn cancel_widget_repeats_for_source(source: &WidgetIntentSource) {
    let mut tasks = widget_repeat_tasks().lock().await;
    let keys: Vec<_> = tasks
        .keys()
        .filter(|key| {
            key.client_id == source.client_id
                && key.connection_session_id == source.connection_session_id
        })
        .cloned()
        .collect();
    for key in keys {
        if let Some(task) = tasks.remove(&key) {
            task.abort();
        }
    }
}

pub(super) async fn cancel_widget_repeats_for_client(client_id: &str) {
    let mut tasks = widget_repeat_tasks().lock().await;
    let keys: Vec<_> = tasks
        .keys()
        .filter(|key| key.client_id == client_id)
        .cloned()
        .collect();
    for key in keys {
        if let Some(task) = tasks.remove(&key) {
            task.abort();
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct WidgetSequenceKey {
    deck_id: i32,
    document_revision: i32,
    instance_id: String,
    port_id: String,
    input_source: String,
    client_id: String,
    connection_session_id: String,
    interaction_session_id: String,
}

#[derive(Default)]
struct WidgetSequenceTracker {
    entries: HashMap<WidgetSequenceKey, (u64, Instant)>,
}

impl WidgetSequenceTracker {
    const MAX_ENTRIES: usize = 4_096;
    const TTL: Duration = Duration::from_secs(30 * 60);

    fn accept(&mut self, key: WidgetSequenceKey, sequence: u64, now: Instant) -> bool {
        self.entries
            .retain(|_, (_, seen_at)| now.duration_since(*seen_at) <= Self::TTL);
        if self
            .entries
            .get(&key)
            .is_some_and(|(previous, _)| sequence <= *previous)
        {
            return false;
        }
        if self.entries.len() >= Self::MAX_ENTRIES && !self.entries.contains_key(&key) {
            if let Some(oldest) = self
                .entries
                .iter()
                .min_by_key(|(_, (_, seen_at))| *seen_at)
                .map(|(key, _)| key.clone())
            {
                self.entries.remove(&oldest);
            }
        }
        self.entries.insert(key, (sequence, now));
        true
    }
}

fn widget_intent_sequences() -> &'static Mutex<WidgetSequenceTracker> {
    static SEQUENCES: OnceLock<Mutex<WidgetSequenceTracker>> = OnceLock::new();
    SEQUENCES.get_or_init(|| Mutex::new(WidgetSequenceTracker::default()))
}

fn widget_preview_times() -> &'static Mutex<HashMap<String, Instant>> {
    static TIMES: OnceLock<Mutex<HashMap<String, Instant>>> = OnceLock::new();
    TIMES.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(Clone)]
struct WidgetRuntimeRecord {
    revision: u64,
    status: String,
    values: serde_json::Map<String, Value>,
    provider_states: serde_json::Map<String, Value>,
    request: Option<Value>,
    error: Option<Value>,
}

#[derive(Clone)]
struct WidgetProviderRecord {
    revision: u64,
    availability: &'static str,
    observed_at: u64,
    values: Value,
}

fn widget_runtime_records() -> &'static Mutex<HashMap<String, WidgetRuntimeRecord>> {
    static RECORDS: OnceLock<Mutex<HashMap<String, WidgetRuntimeRecord>>> = OnceLock::new();
    RECORDS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn widget_provider_records() -> &'static Mutex<HashMap<String, WidgetProviderRecord>> {
    static RECORDS: OnceLock<Mutex<HashMap<String, WidgetProviderRecord>>> = OnceLock::new();
    RECORDS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn unix_time_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn widget_public_state_paths() -> &'static Mutex<HashMap<String, Option<Vec<String>>>> {
    static PATHS: OnceLock<Mutex<HashMap<String, Option<Vec<String>>>>> = OnceLock::new();
    PATHS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn project_widget_values(source: &Value, paths: &[String]) -> Value {
    let mut target = json!({});
    for path in paths {
        let segments = path.split('.').collect::<Vec<_>>();
        let mut current = source;
        let mut found = true;
        for segment in &segments {
            let Some(next) = current.get(*segment) else {
                found = false;
                break;
            };
            current = next;
        }
        if !found {
            continue;
        }
        let mut cursor = &mut target;
        for (index, segment) in segments.iter().enumerate() {
            if index + 1 == segments.len() {
                cursor[*segment] = current.clone();
            } else {
                if !cursor.get(*segment).is_some_and(Value::is_object) {
                    cursor[*segment] = json!({});
                }
                cursor = &mut cursor[*segment];
            }
        }
    }
    target
}

async fn public_widget_values(
    app_handle: &AppHandle,
    deck_id: i32,
    instance_id: &str,
    values: &Value,
) -> Value {
    let key = format!("{}:{}", deck_id, instance_id);
    if let Some(paths) = widget_public_state_paths().lock().await.get(&key).cloned() {
        return paths.map_or_else(
            || values.clone(),
            |paths| project_widget_values(values, &paths),
        );
    }
    let Some(pool) = app_handle.try_state::<vortideck_core::db::db::DbPool>() else {
        return json!({});
    };
    let db = Database::new(pool.inner());
    let Ok(document) = db.get_deck_widget_document(deck_id).await else {
        return json!({});
    };
    let Some(item) = document
        .items
        .iter()
        .find(|item| item.item.id == instance_id)
    else {
        return json!({});
    };
    let paths = if item.item.definition_id.starts_with("vortideck.") {
        None
    } else {
        let mut resolved = Vec::new();
        if let (Some(package_id), Some(core)) = (
            item.item.package_id.as_deref(),
            app_handle.try_state::<vortideck_core::VortiDeckCore>(),
        ) {
            if let Some(manifest) = core.plugin_manager().package_manifest(package_id).await {
                for contribution in manifest.contributions {
                    if let vortideck_core::plugins::Contribution::Widget {
                        deck_surface: Some(surface),
                        ..
                    } = contribution
                    {
                        if surface.definition_id == item.item.definition_id {
                            resolved = surface.public_state_paths;
                            break;
                        }
                    }
                }
            }
        }
        Some(resolved)
    };
    // Built-in definitions have a stable public contract and can be cached.
    // Package allowlists are re-read so grant/update/re-consent changes fail
    // closed immediately instead of retaining a stale broader projection.
    if paths.is_none() {
        widget_public_state_paths().lock().await.insert(key, None);
    }
    paths.map_or_else(
        || values.clone(),
        |paths| project_widget_values(values, &paths),
    )
}

async fn public_widget_provider_states(
    app_handle: &AppHandle,
    deck_id: i32,
    instance_id: &str,
    provider_states: &Value,
) -> Value {
    let mut projected = serde_json::Map::new();
    for (topic, provider) in provider_states.as_object().into_iter().flatten() {
        let mut provider = provider.clone();
        provider["values"] = public_widget_values(
            app_handle,
            deck_id,
            instance_id,
            provider.get("values").unwrap_or(&Value::Null),
        )
        .await;
        projected.insert(topic.clone(), provider);
    }
    Value::Object(projected)
}

async fn widget_subscription_topics(
    app_handle: &AppHandle,
    item: &vortideck_core::db::DeckItemDocument,
) -> Vec<String> {
    let builtin = match item.item.definition_id.as_str() {
        "vortideck.rotary" | "vortideck.fader" => vec!["system.volume".to_string()],
        "vortideck.media-controls" => vec!["media.playback".to_string()],
        "vortideck.telemetry" => vec!["telemetry".to_string()],
        _ => Vec::new(),
    };
    if !builtin.is_empty() {
        return builtin;
    }
    let (Some(package_id), Some(core)) = (
        item.item.package_id.as_deref(),
        app_handle.try_state::<vortideck_core::VortiDeckCore>(),
    ) else {
        return Vec::new();
    };
    let Some(manifest) = core.plugin_manager().package_manifest(package_id).await else {
        return Vec::new();
    };
    manifest
        .contributions
        .into_iter()
        .find_map(|contribution| match contribution {
            vortideck_core::plugins::Contribution::Widget {
                deck_surface: Some(surface),
                ..
            } if surface.definition_id == item.item.definition_id => {
                Some(surface.state_subscriptions)
            }
            _ => None,
        })
        .unwrap_or_default()
}

pub(super) async fn publish_widget_runtime_state(
    app_handle: &AppHandle,
    fallback_tx: &broadcast::Sender<Message>,
    deck_id: i32,
    instance_id: &str,
    status: &str,
    values: Value,
    error: Option<Value>,
) {
    let key = format!("{}:{}", deck_id, instance_id);
    let patch = {
        let mut records = widget_runtime_records().lock().await;
        let record = records.entry(key).or_insert_with(|| WidgetRuntimeRecord {
            revision: 0,
            status: "disconnected".to_string(),
            values: serde_json::Map::new(),
            provider_states: serde_json::Map::new(),
            request: None,
            error: None,
        });
        let base_revision = record.revision;
        record.revision += 1;
        record.status = status.to_string();
        if let Some(object) = values.as_object() {
            record.values.extend(object.clone());
        }
        if status != "executing" {
            record.request = None;
        }
        record.error = error.clone();
        json!({
            "protocolVersion": 1,
            "instanceId": instance_id,
            "baseRevision": base_revision,
            "revision": record.revision,
            "status": status,
            "values": values,
            "providerStates": record.provider_states,
            "request": record.request,
            "error": error,
        })
    };
    let message = json!({
        "action": "deck_widget_state",
        "protocolVersion": 1,
        "deckId": deck_id,
        "patches": [patch],
    });
    let _ = app_handle.emit("deck_widget_state", message.clone());
    let public_values = public_widget_values(app_handle, deck_id, instance_id, &values).await;
    let public_provider_states =
        public_widget_provider_states(app_handle, deck_id, instance_id, &patch["providerStates"])
            .await;
    let public_patch = json!({
        "protocolVersion": 1,
        "instanceId": instance_id,
        "baseRevision": patch["baseRevision"],
        "revision": patch["revision"],
        "status": status,
        "values": public_values,
        "providerStates": public_provider_states,
        "request": patch["request"],
        "error": error.as_ref().map(|value| json!({
            "code": value.get("code").and_then(Value::as_str).unwrap_or("execution_failed"),
            "message": "Widget action failed",
        })),
    });
    let public_message = json!({
        "action": "deck_widget_state",
        "protocolVersion": 1,
        "deckId": deck_id,
        "patches": [public_patch],
    });
    if let Some(state_manager) = app_handle.try_state::<Arc<vortideck_core::core::StateManager>>() {
        crate::broadcast_helper::broadcast_to_all_clients(
            app_handle,
            state_manager.inner(),
            public_message,
        )
        .await;
    } else {
        let _ = fallback_tx.send(Message::Text(public_message.to_string().into()));
    }
}

pub(super) async fn publish_widget_requested_state(
    app_handle: &AppHandle,
    fallback_tx: &broadcast::Sender<Message>,
    deck_id: i32,
    instance_id: &str,
    values: Value,
) -> String {
    let request_id = uuid::Uuid::new_v4().to_string();
    let public_values = public_widget_values(app_handle, deck_id, instance_id, &values).await;
    let key = format!("{}:{}", deck_id, instance_id);
    let patch = {
        let mut records = widget_runtime_records().lock().await;
        let record = records.entry(key).or_insert_with(|| WidgetRuntimeRecord {
            revision: 0,
            status: "disconnected".to_string(),
            values: serde_json::Map::new(),
            provider_states: serde_json::Map::new(),
            request: None,
            error: None,
        });
        let base_revision = record.revision;
        record.revision += 1;
        record.status = "executing".to_string();
        record.error = None;
        record.request = Some(json!({
            "requestId": request_id.clone(),
            "requestedAt": unix_time_millis(),
            "values": values,
        }));
        json!({
            "protocolVersion": 1,
            "instanceId": instance_id,
            "baseRevision": base_revision,
            "revision": record.revision,
            "status": record.status,
            "values": {},
            "providerStates": record.provider_states,
            "request": record.request,
            "error": null,
        })
    };
    let message = json!({
        "action": "deck_widget_state",
        "protocolVersion": 1,
        "deckId": deck_id,
        "patches": [patch],
    });
    let _ = app_handle.emit("deck_widget_state", message.clone());
    let public_provider_states =
        public_widget_provider_states(app_handle, deck_id, instance_id, &patch["providerStates"])
            .await;
    let public_patch = json!({
        "protocolVersion": 1,
        "instanceId": instance_id,
        "baseRevision": patch["baseRevision"],
        "revision": patch["revision"],
        "status": "executing",
        "values": {},
        "providerStates": public_provider_states,
        "request": {
            "requestId": request_id,
            "requestedAt": patch["request"]["requestedAt"],
            "values": public_values,
        },
        "error": null,
    });
    let public_message = json!({
        "action": "deck_widget_state",
        "protocolVersion": 1,
        "deckId": deck_id,
        "patches": [public_patch],
    });
    if let Some(state_manager) = app_handle.try_state::<Arc<vortideck_core::core::StateManager>>() {
        crate::broadcast_helper::broadcast_to_all_clients(
            app_handle,
            state_manager.inner(),
            public_message,
        )
        .await;
    } else {
        let _ = fallback_tx.send(Message::Text(public_message.to_string().into()));
    }
    patch["request"]["requestId"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

pub(super) async fn widget_runtime_snapshots(deck_id: i32, instance_ids: &[String]) -> Vec<Value> {
    let records = widget_runtime_records().lock().await;
    instance_ids
        .iter()
        .filter_map(|instance_id| {
            records
                .get(&format!("{}:{}", deck_id, instance_id))
                .map(|record| {
                    json!({
                        "protocolVersion": 1,
                        "instanceId": instance_id,
                        "revision": record.revision,
                        "status": record.status,
                        "values": record.values,
                        "providerStates": record.provider_states,
                        "request": record.request,
                        "error": record.error,
                    })
                })
        })
        .collect()
}

pub(super) async fn publish_widget_topic_for_active_deck(
    app_handle: &AppHandle,
    topic: &str,
    values: Value,
) -> bool {
    publish_widget_topic_availability_for_active_deck(app_handle, topic, values, true).await
}

pub(super) async fn publish_widget_topic_availability_for_active_deck(
    app_handle: &AppHandle,
    topic: &str,
    values: Value,
    available: bool,
) -> bool {
    let observation = {
        let mut providers = widget_provider_records().lock().await;
        let revision = providers
            .get(topic)
            .map_or(1, |record| record.revision.saturating_add(1));
        let record = WidgetProviderRecord {
            revision,
            availability: if available {
                "available"
            } else {
                "unavailable"
            },
            observed_at: unix_time_millis(),
            values,
        };
        providers.insert(topic.to_string(), record.clone());
        record
    };
    let Some(pool) = app_handle.try_state::<vortideck_core::db::db::DbPool>() else {
        return false;
    };
    let db = Database::new(pool.inner());
    let Some(document) = active_widget_document(&db).await else {
        return false;
    };
    let tx = app_handle
        .try_state::<Arc<vortideck_core::core::StateManager>>()
        .and_then(|manager| manager.get_server_sender())
        .unwrap_or_else(|| broadcast::channel::<Message>(16).0);
    let mut published = false;
    for item in &document.items {
        if !item.item.enabled {
            continue;
        }
        if widget_subscription_topics(app_handle, item)
            .await
            .iter()
            .any(|subscription| subscription == topic)
        {
            published = true;
            publish_widget_provider_observation(
                app_handle,
                &tx,
                document.deck_id,
                &item.item.id,
                topic,
                &observation,
            )
            .await;
        }
    }
    published
}

async fn publish_widget_provider_observation(
    app_handle: &AppHandle,
    fallback_tx: &broadcast::Sender<Message>,
    deck_id: i32,
    instance_id: &str,
    topic: &str,
    observation: &WidgetProviderRecord,
) {
    let key = format!("{}:{}", deck_id, instance_id);
    let provider_state = json!({
        "revision": observation.revision,
        "availability": observation.availability,
        "observedAt": observation.observed_at,
        "values": observation.values,
    });
    let patch = {
        let mut records = widget_runtime_records().lock().await;
        let record = records.entry(key).or_insert_with(|| WidgetRuntimeRecord {
            revision: 0,
            status: "disconnected".to_string(),
            values: serde_json::Map::new(),
            provider_states: serde_json::Map::new(),
            request: None,
            error: None,
        });
        let base_revision = record.revision;
        record.revision += 1;
        record.status = if observation.availability == "available" {
            "ready".to_string()
        } else {
            "disconnected".to_string()
        };
        if observation.availability == "available" {
            if let Some(values) = observation.values.as_object() {
                record.values.extend(values.clone());
            }
        }
        record
            .provider_states
            .insert(topic.to_string(), provider_state.clone());
        record.request = None;
        record.error = None;
        json!({
            "protocolVersion": 1,
            "instanceId": instance_id,
            "baseRevision": base_revision,
            "revision": record.revision,
            "status": record.status,
            "values": if observation.availability == "available" { observation.values.clone() } else { json!({}) },
            "providerStates": { (topic): provider_state },
            "request": null,
            "error": null,
        })
    };
    let message = json!({
        "action": "deck_widget_state",
        "protocolVersion": 1,
        "deckId": deck_id,
        "patches": [patch.clone()],
    });
    let _ = app_handle.emit("deck_widget_state", message);

    let public_values =
        public_widget_values(app_handle, deck_id, instance_id, &observation.values).await;
    let public_provider_state = json!({
        "revision": observation.revision,
        "availability": observation.availability,
        "observedAt": observation.observed_at,
        "values": public_values.clone(),
    });
    let public_patch = json!({
        "protocolVersion": 1,
        "instanceId": instance_id,
        "baseRevision": patch["baseRevision"],
        "revision": patch["revision"],
        "status": patch["status"],
        "values": if observation.availability == "available" { public_values } else { json!({}) },
        "providerStates": { (topic): public_provider_state },
        "request": null,
        "error": null,
    });
    let public_message = json!({
        "action": "deck_widget_state",
        "protocolVersion": 1,
        "deckId": deck_id,
        "patches": [public_patch],
    });
    if let Some(state_manager) = app_handle.try_state::<Arc<vortideck_core::core::StateManager>>() {
        crate::broadcast_helper::broadcast_to_all_clients(
            app_handle,
            state_manager.inner(),
            public_message,
        )
        .await;
    } else {
        let _ = fallback_tx.send(Message::Text(public_message.to_string().into()));
    }
}

fn action_command(action: &Value) -> String {
    match action {
        Value::String(value) => value.clone(),
        value => value.to_string(),
    }
}

async fn send_widget_intent_ack(
    app_handle: &AppHandle,
    tx: &broadcast::Sender<Message>,
    deck_id: i32,
    instance_id: &str,
    port_id: &str,
    sequence: u64,
    status: &str,
    correlation_id: Option<&str>,
    interaction_session_id: &str,
) {
    let acknowledgement = json!({
        "action": "deck_widget_intent_ack",
        "protocolVersion": 1,
        "deckId": deck_id,
        "instanceId": instance_id,
        "portId": port_id,
        "sequence": sequence,
        "correlationId": correlation_id,
        "interactionSessionId": interaction_session_id,
        "status": status,
    });
    let _ = app_handle.emit("deck_widget_intent_ack", acknowledgement.clone());
    if let Some(state_manager) = app_handle.try_state::<Arc<vortideck_core::core::StateManager>>() {
        crate::broadcast_helper::broadcast_to_all_clients(
            app_handle,
            state_manager.inner(),
            acknowledgement,
        )
        .await;
    } else {
        let _ = tx.send(Message::Text(acknowledgement.to_string().into()));
    }
}

async fn reject_widget_intent(
    ctx: &CommandExecutionContext,
    deck_id: i32,
    instance_id: &str,
    port_id: &str,
    sequence: u64,
    code: &str,
    message: &str,
) {
    let acknowledgement = json!({
        "action": "deck_widget_intent_ack",
        "protocolVersion": 1,
        "deckId": deck_id,
        "instanceId": instance_id,
        "portId": port_id,
        "sequence": sequence,
        "correlationId": ctx.json.get("correlationId").and_then(Value::as_str),
        "interactionSessionId": ctx.json.pointer("/intent/interactionSessionId").and_then(Value::as_str),
        "status": "rejected",
        "code": code,
        "message": message,
    });
    let _ = ctx
        .app_handle
        .emit("deck_widget_intent_ack", acknowledgement.clone());
    if let Some(state_manager) = ctx
        .app_handle
        .try_state::<Arc<vortideck_core::core::StateManager>>()
    {
        crate::broadcast_helper::broadcast_to_all_clients(
            &ctx.app_handle,
            state_manager.inner(),
            acknowledgement,
        )
        .await;
    } else {
        let _ = ctx
            .tx
            .send(Message::Text(acknowledgement.to_string().into()));
    }
}

/// Resolve a normalized widget intent to a persisted host binding before
/// delegating execution. The client controls only instance/port/value; it can
/// never substitute an action payload or bypass a disabled widget.
pub async fn handle_widget_intent_action(
    mut ctx: CommandExecutionContext,
    source: WidgetIntentSource,
) {
    let deck_id = match ctx.json.get("deckId").and_then(Value::as_i64) {
        Some(value) => value as i32,
        None => {
            log::warn!("Rejected widget intent without deckId");
            return;
        }
    };
    let document_revision = ctx
        .json
        .get("documentRevision")
        .and_then(Value::as_i64)
        .unwrap_or(-1) as i32;
    let intent = match ctx.json.get("intent") {
        Some(value) => value.clone(),
        None => return,
    };
    let instance_id = match intent.get("instanceId").and_then(Value::as_str) {
        Some(value) => value.to_string(),
        None => return,
    };
    let port_id = match intent.get("portId").and_then(Value::as_str) {
        Some(value) => value.to_string(),
        None => return,
    };
    let phase = intent
        .get("phase")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let input_source = intent
        .get("inputSource")
        .and_then(Value::as_str)
        .unwrap_or("remote")
        .to_string();
    let interaction_session_id = match intent
        .get("interactionSessionId")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty() && value.len() <= 128)
    {
        Some(value) => value.to_string(),
        None => {
            log::warn!("Rejected widget intent without a valid interaction session");
            return;
        }
    };
    let sequence = match intent.get("sequence").and_then(Value::as_u64) {
        Some(value) => value,
        None => {
            log::warn!("Rejected widget intent without a sequence");
            return;
        }
    };
    if !matches!(
        phase.as_str(),
        "start" | "preview" | "commit" | "end" | "cancel"
    ) {
        reject_widget_intent(
            &ctx,
            deck_id,
            &instance_id,
            &port_id,
            sequence,
            "invalid_phase",
            "The widget intent phase is not supported.",
        )
        .await;
        return;
    }

    // The transport may retry, but the same input source may never replay or
    // reorder an already handled intent for the current document revision.
    let sequence_key = WidgetSequenceKey {
        deck_id,
        document_revision,
        instance_id: instance_id.clone(),
        port_id: port_id.clone(),
        input_source: input_source.clone(),
        client_id: source.client_id.clone(),
        connection_session_id: source.connection_session_id.clone(),
        interaction_session_id: interaction_session_id.clone(),
    };
    let replayed = !widget_intent_sequences().lock().await.accept(
        sequence_key.clone(),
        sequence,
        Instant::now(),
    );
    if replayed {
        log::warn!("Rejected replayed widget intent {:?}", sequence_key);
        reject_widget_intent(
            &ctx,
            deck_id,
            &instance_id,
            &port_id,
            sequence,
            "replayed_sequence",
            "This widget intent was already handled.",
        )
        .await;
        return;
    }

    // Release is fail-safe: a document edit may advance the revision between
    // pointer-down and pointer-up, but must never leave a repeat task running.
    if matches!(phase.as_str(), "end" | "cancel") {
        let repeat_key = WidgetRepeatKey {
            deck_id,
            instance_id: instance_id.clone(),
            port_id: port_id.clone(),
            client_id: source.client_id.clone(),
            connection_session_id: source.connection_session_id.clone(),
            interaction_session_id: interaction_session_id.clone(),
        };
        if let Some(task) = widget_repeat_tasks().lock().await.remove(&repeat_key) {
            task.abort();
            send_widget_intent_ack(
                &ctx.app_handle,
                &ctx.tx,
                deck_id,
                &instance_id,
                &port_id,
                sequence,
                "stopped",
                ctx.json.get("correlationId").and_then(Value::as_str),
                &interaction_session_id,
            )
            .await;
            return;
        }
    }

    let document = match ctx.db.get_deck_widget_document(deck_id).await {
        Ok(value) => value,
        Err(error) => {
            log::warn!("Failed to resolve widget intent: {}", error);
            reject_widget_intent(
                &ctx,
                deck_id,
                &instance_id,
                &port_id,
                sequence,
                "document_unavailable",
                "The deck widget document could not be loaded.",
            )
            .await;
            return;
        }
    };
    if document.revision != document_revision {
        log::warn!(
            "Rejected stale widget intent for deck {}: client revision {}, current {}",
            deck_id,
            document_revision,
            document.revision
        );
        reject_widget_intent(
            &ctx,
            deck_id,
            &instance_id,
            &port_id,
            sequence,
            "stale_revision",
            "The deck changed; refresh its widget snapshot and retry.",
        )
        .await;
        return;
    }
    let item = match document
        .items
        .iter()
        .find(|candidate| candidate.item.id == instance_id && candidate.item.enabled)
    {
        Some(value) => value,
        None => {
            reject_widget_intent(
                &ctx,
                deck_id,
                &instance_id,
                &port_id,
                sequence,
                "widget_unavailable",
                "The widget is missing or disabled.",
            )
            .await;
            return;
        }
    };

    // First-class items must not bypass the same plan entitlement enforced for
    // legacy buttons. Order by the resolved mobile layout, then stable ID.
    let layout_key_pref = intent
        .get("layoutKey")
        .and_then(Value::as_str)
        .or_else(|| ctx.json.get("layoutKey").and_then(Value::as_str));

    let mut entitled_order: Vec<_> = document
        .items
        .iter()
        .filter(|item| item.item.enabled)
        .collect();
    entitled_order.sort_by_key(|document| {
        let placement = layout_key_pref
            .and_then(|pref| {
                document
                    .layouts
                    .iter()
                    .find(|layout| layout.layout_key == pref)
            })
            .or_else(|| {
                document
                    .layouts
                    .iter()
                    .find(|layout| layout.layout_key == "landscape")
            })
            .or_else(|| {
                document
                    .layouts
                    .iter()
                    .find(|layout| layout.layout_key == "portrait")
            })
            .or_else(|| {
                document
                    .layouts
                    .iter()
                    .find(|layout| layout.layout_key == "default")
            })
            .or_else(|| document.layouts.first());
        (
            placement.map(|layout| layout.y).unwrap_or(i32::MAX),
            placement.map(|layout| layout.x).unwrap_or(i32::MAX),
            document.item.id.clone(),
        )
    });
    if let Some(index) = entitled_order
        .iter()
        .position(|document| document.item.id == instance_id)
    {
        use vortideck_core::features::jwt_features::get_jwt_provider;
        let limit = get_jwt_provider().get_effective_feature_limit("ui_buttons") as usize;
        if index >= limit {
            log::warn!(
                "Rejected widget '{}' at entitlement index {} (limit {})",
                instance_id,
                index,
                limit
            );
            reject_widget_intent(
                &ctx,
                deck_id,
                &instance_id,
                &port_id,
                sequence,
                "plan_limit",
                "This widget is outside the current plan limit.",
            )
            .await;
            return;
        }
    }
    if let Some(package_id) = item.item.package_id.as_deref() {
        if !vortideck_core::plugins::integrity::is_known_builtin(package_id) {
            match ctx
                .db
                .to_core_database()
                .get_package_permission_grant(package_id)
                .await
            {
                Ok(Some(grant)) => {
                    let plugin_manager =
                        match ctx.app_handle.try_state::<vortideck_core::VortiDeckCore>() {
                            Some(core) => core.plugin_manager(),
                            None => {
                                log::warn!(
                                    "Could not validate package '{}': core unavailable",
                                    package_id
                                );
                                return;
                            }
                        };
                    let manifest = match plugin_manager.package_manifest(package_id).await {
                        Some(value) => value,
                        None => {
                            log::warn!(
                                "Rejected widget intent for missing package '{}'",
                                package_id
                            );
                            return;
                        }
                    };
                    let current_hash =
                        match vortideck_core::plugins::manifest_consent_hash(&manifest) {
                            Ok(value) => value,
                            Err(error) => {
                                log::warn!("Could not hash package '{}': {}", package_id, error);
                                return;
                            }
                        };
                    let current_permissions =
                        vortideck_core::plugins::compile_manifest_permissions(&manifest);
                    let current_permissions_json =
                        serde_json::to_string(&current_permissions).unwrap_or_default();
                    if current_hash != grant.content_hash
                        || current_permissions_json != grant.permissions_json
                    {
                        log::warn!(
                            "Rejected widget intent pending re-consent for '{}'",
                            package_id
                        );
                        return;
                    }
                }
                Ok(None) => {
                    log::warn!(
                        "Rejected widget intent for revoked package '{}'",
                        package_id
                    );
                    return;
                }
                Err(error) => {
                    log::warn!(
                        "Could not validate widget package grant '{}': {}",
                        package_id,
                        error
                    );
                    return;
                }
            }
        }
    }
    let binding = match item
        .bindings
        .iter()
        .find(|candidate| candidate.port_id == port_id)
    {
        Some(value) => value,
        None => {
            reject_widget_intent(
                &ctx,
                deck_id,
                &instance_id,
                &port_id,
                sequence,
                "binding_missing",
                "No host action is bound to this widget port.",
            )
            .await;
            return;
        }
    };
    let stored_action: Value = match serde_json::from_str(&binding.action_json) {
        Ok(value) => value,
        Err(error) => {
            log::warn!(
                "Invalid persisted widget action '{}': {}",
                binding.id,
                error
            );
            reject_widget_intent(
                &ctx,
                deck_id,
                &instance_id,
                &port_id,
                sequence,
                "invalid_action",
                "The persisted widget action is invalid.",
            )
            .await;
            return;
        }
    };
    let policy: Value = match serde_json::from_str(&binding.policy_json) {
        Ok(value) => value,
        Err(error) => {
            log::warn!(
                "Invalid persisted widget policy '{}': {}",
                binding.id,
                error
            );
            reject_widget_intent(
                &ctx,
                deck_id,
                &instance_id,
                &port_id,
                sequence,
                "invalid_policy",
                "The persisted widget execution policy is invalid.",
            )
            .await;
            return;
        }
    };
    let policy_type = policy
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("run_once");
    let mut value = intent.get("value").cloned().unwrap_or(Value::Null);
    if let (Some(input), Some(transform_json)) = (value.as_f64(), binding.transform_json.as_ref()) {
        if let Ok(transform) = serde_json::from_str::<Value>(transform_json) {
            let input_min = transform
                .get("inputMin")
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            let input_max = transform
                .get("inputMax")
                .and_then(Value::as_f64)
                .unwrap_or(1.0);
            let output_min = transform
                .get("outputMin")
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            let output_max = transform
                .get("outputMax")
                .and_then(Value::as_f64)
                .unwrap_or(1.0);
            if input_max != input_min {
                let normalized = ((input - input_min) / (input_max - input_min)).clamp(0.0, 1.0);
                let curved =
                    if transform.get("curve").and_then(Value::as_str) == Some("logarithmic") {
                        (1.0 + 9.0 * normalized).log10()
                    } else {
                        normalized
                    };
                value = json!(output_min + curved * (output_max - output_min));
            }
        }
    }

    if policy_type == "repeat" {
        let repeat_key = WidgetRepeatKey {
            deck_id,
            instance_id: instance_id.clone(),
            port_id: port_id.clone(),
            client_id: source.client_id.clone(),
            connection_session_id: source.connection_session_id.clone(),
            interaction_session_id: interaction_session_id.clone(),
        };
        if matches!(phase.as_str(), "end" | "cancel") {
            if let Some(task) = widget_repeat_tasks().lock().await.remove(&repeat_key) {
                task.abort();
            }
            send_widget_intent_ack(
                &ctx.app_handle,
                &ctx.tx,
                deck_id,
                &instance_id,
                &port_id,
                sequence,
                "stopped",
                ctx.json.get("correlationId").and_then(Value::as_str),
                &interaction_session_id,
            )
            .await;
            return;
        }
        if phase != "start" {
            return;
        }

        let hold_ms = policy
            .get("holdMs")
            .and_then(Value::as_u64)
            .unwrap_or(300)
            .min(5_000);
        let interval_ms = policy
            .get("intervalMs")
            .and_then(Value::as_u64)
            .unwrap_or(100)
            .clamp(25, 60_000);
        let max_hold_ms = policy
            .get("maxHoldMs")
            .and_then(Value::as_u64)
            .unwrap_or(300_000)
            .clamp(interval_ms, 3_600_000);
        let command = action_command(&stored_action);
        let mut repeat_ctx = ctx.clone();
        repeat_ctx.json = json!({
            "action": "command",
            "command": command,
            "value": value,
            "buttonId": item.item.legacy_button_id,
            "widgetInstanceId": instance_id,
            "widgetBindingId": binding.id,
            "widgetDeckId": deck_id,
            "widgetExecutionMode": "repeat_while_pressing",
            "widgetExecutionConfig": policy,
        });

        if let Some(previous) = widget_repeat_tasks().lock().await.remove(&repeat_key) {
            previous.abort();
        }
        // Execute immediately on initial press so a standard click executes even if released before hold_ms
        handle_command_action(repeat_ctx.clone()).await;

        let task = tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(hold_ms)).await;
            let deadline = Instant::now() + Duration::from_millis(max_hold_ms);
            loop {
                if Instant::now() >= deadline {
                    break;
                }
                handle_command_action(repeat_ctx.clone()).await;
                tokio::time::sleep(tokio::time::Duration::from_millis(interval_ms)).await;
            }
        });
        widget_repeat_tasks().lock().await.insert(repeat_key, task);
        send_widget_intent_ack(
            &ctx.app_handle,
            &ctx.tx,
            deck_id,
            &instance_id,
            &port_id,
            sequence,
            "scheduled",
            ctx.json.get("correlationId").and_then(Value::as_str),
            &interaction_session_id,
        )
        .await;
        return;
    }

    if policy_type == "continuous" {
        if !matches!(phase.as_str(), "preview" | "commit") {
            return;
        }
        if phase == "preview" {
            let throttle_ms = policy
                .get("throttleMs")
                .and_then(Value::as_u64)
                .unwrap_or(50)
                .min(5_000);
            let preview_key = format!("{}:{}:{}", deck_id, instance_id, port_id);
            let now = Instant::now();
            let mut times = widget_preview_times().lock().await;
            if times.get(&preview_key).is_some_and(|previous| {
                now.duration_since(*previous) < Duration::from_millis(throttle_ms)
            }) {
                return;
            }
            times.insert(preview_key, now);
        }
    } else if phase != "commit" {
        return;
    }

    let mut execution_mode = match policy_type {
        "toggle" => "switch",
        "queue" => "queue",
        _ => "run_once",
    };
    let mut selected_action = stored_action;
    let mut next_toggle_state = None;
    let state_topics = widget_subscription_topics(&ctx.app_handle, item).await;
    let provider_observed = !state_topics.is_empty();

    if policy_type == "toggle" {
        let persist_state = policy
            .get("persistState")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let persisted_state = if persist_state {
            ctx.db
                .to_core_database()
                .get_deck_item_persistent_state(&instance_id)
                .await
                .ok()
                .flatten()
                .and_then(|state| state.get("enabled").and_then(Value::as_bool))
        } else {
            None
        };
        let observed_state = widget_runtime_records()
            .lock()
            .await
            .get(&format!("{}:{}", deck_id, instance_id))
            .and_then(|record| record.values.get("enabled"))
            .and_then(Value::as_bool);
        let next = !observed_state.or(persisted_state).unwrap_or(false);
        next_toggle_state = Some(next);
    }

    // A toggle can define explicit actions for both states. In that form the
    // host owns the state transition and executes exactly the selected branch.
    if policy_type == "toggle"
        && (policy.get("onAction").is_some() || policy.get("offAction").is_some())
    {
        let next_state = next_toggle_state.unwrap_or(false);
        let branch = if next_state { "onAction" } else { "offAction" };
        selected_action = match policy.get(branch) {
            Some(action) if !action.is_null() => action.clone(),
            _ => {
                let next = next_toggle_state.unwrap_or(false);
                if policy
                    .get("persistState")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    if let Err(error) = ctx
                        .db
                        .to_core_database()
                        .save_deck_item_persistent_state(&instance_id, &json!({ "enabled": next }))
                        .await
                    {
                        log::warn!(
                            "Failed to persist toggle state for '{}': {}",
                            instance_id,
                            error
                        );
                    }
                }
                publish_widget_runtime_state(
                    &ctx.app_handle,
                    &ctx.tx,
                    deck_id,
                    &instance_id,
                    "ready",
                    json!({ "enabled": next }),
                    None,
                )
                .await;
                send_widget_intent_ack(
                    &ctx.app_handle,
                    &ctx.tx,
                    deck_id,
                    &instance_id,
                    &port_id,
                    sequence,
                    "completed",
                    ctx.json.get("correlationId").and_then(Value::as_str),
                    &interaction_session_id,
                )
                .await;
                return;
            }
        };
        execution_mode = "run_once";
    }

    let command = action_command(&selected_action);
    if serde_json::from_str::<Action>(&command).is_err() {
        reject_widget_intent(
            &ctx,
            deck_id,
            &instance_id,
            &port_id,
            sequence,
            "invalid_action",
            "The saved action cannot be executed.",
        )
        .await;
        return;
    }
    if ctx.app_handle.try_state::<Arc<PlatformContext>>().is_none() {
        reject_widget_intent(
            &ctx,
            deck_id,
            &instance_id,
            &port_id,
            sequence,
            "runtime_unavailable",
            "The action runtime is unavailable.",
        )
        .await;
        return;
    }
    let requested_values = if let Some(enabled) = next_toggle_state {
        json!({ "enabled": enabled })
    } else if policy_type == "continuous" && !value.is_null() {
        json!({ "value": value })
    } else {
        json!({})
    };
    publish_widget_requested_state(
        &ctx.app_handle,
        &ctx.tx,
        deck_id,
        &instance_id,
        requested_values.clone(),
    )
    .await;
    let acknowledgement_correlation_id = ctx
        .json
        .get("correlationId")
        .and_then(Value::as_str)
        .map(str::to_string);
    ctx.json = json!({
        "action": "command",
        "command": command,
        "value": value,
        "buttonId": item.item.legacy_button_id,
        "widgetInstanceId": instance_id,
        "widgetBindingId": binding.id,
        "widgetDeckId": deck_id,
        "widgetExecutionMode": execution_mode,
        "widgetExecutionConfig": policy,
    });
    let acknowledgement_tx = ctx.tx.clone();
    let acknowledgement_app = ctx.app_handle.clone();
    ctx.json["widgetRequestedValues"] = requested_values;
    ctx.json["widgetCommitRequestedOnSuccess"] = json!(!provider_observed);
    handle_command_action(ctx).await;
    send_widget_intent_ack(
        &acknowledgement_app,
        &acknowledgement_tx,
        deck_id,
        &instance_id,
        &port_id,
        sequence,
        "accepted",
        acknowledgement_correlation_id.as_deref(),
        &interaction_session_id,
    )
    .await;
}

#[cfg(test)]
mod public_projection_tests {
    use super::*;

    fn sequence_key(
        client: &str,
        connection: &str,
        interaction: &str,
        revision: i32,
    ) -> WidgetSequenceKey {
        WidgetSequenceKey {
            deck_id: 1,
            document_revision: revision,
            instance_id: "widget".to_string(),
            port_id: "primary".to_string(),
            input_source: "remote".to_string(),
            client_id: client.to_string(),
            connection_session_id: connection.to_string(),
            interaction_session_id: interaction.to_string(),
        }
    }

    #[test]
    fn sequence_identity_separates_clients_reconnects_and_remounts() {
        let now = Instant::now();
        let mut tracker = WidgetSequenceTracker::default();
        assert!(tracker.accept(sequence_key("a", "connection-1", "mount-1", 4), 1, now));
        assert!(!tracker.accept(sequence_key("a", "connection-1", "mount-1", 4), 1, now));
        assert!(!tracker.accept(sequence_key("a", "connection-1", "mount-1", 4), 0, now));
        assert!(tracker.accept(sequence_key("b", "connection-1", "mount-1", 4), 1, now));
        assert!(tracker.accept(sequence_key("a", "connection-2", "mount-1", 4), 1, now));
        assert!(tracker.accept(sequence_key("a", "connection-1", "mount-2", 4), 1, now));
        assert!(tracker.accept(sequence_key("a", "connection-1", "mount-1", 5), 1, now));
    }

    #[test]
    fn sequence_identity_is_bounded_and_repeat_release_ignores_revision() {
        let now = Instant::now();
        let mut tracker = WidgetSequenceTracker::default();
        for index in 0..WidgetSequenceTracker::MAX_ENTRIES + 10 {
            assert!(tracker.accept(
                sequence_key(&format!("client-{index}"), "connection", "mount", 1),
                1,
                now,
            ));
        }
        assert_eq!(tracker.entries.len(), WidgetSequenceTracker::MAX_ENTRIES);

        let start = WidgetRepeatKey {
            deck_id: 1,
            instance_id: "widget".to_string(),
            port_id: "primary".to_string(),
            client_id: "client".to_string(),
            connection_session_id: "connection".to_string(),
            interaction_session_id: "mount".to_string(),
        };
        let release_after_revision_change = start.clone();
        assert_eq!(start, release_after_revision_change);
    }

    #[tokio::test]
    async fn disconnect_cancels_only_the_owning_clients_repeat_tasks() {
        cancel_all_widget_repeats().await;
        let key = |client: &str| WidgetRepeatKey {
            deck_id: 1,
            instance_id: "widget".to_string(),
            port_id: "primary".to_string(),
            client_id: client.to_string(),
            connection_session_id: "connection".to_string(),
            interaction_session_id: "mount".to_string(),
        };
        let task = || tokio::spawn(std::future::pending::<()>());
        widget_repeat_tasks()
            .lock()
            .await
            .insert(key("client-a"), task());
        widget_repeat_tasks()
            .lock()
            .await
            .insert(key("client-b"), task());

        cancel_widget_repeats_for_source(&WidgetIntentSource {
            client_id: "client-a".to_string(),
            connection_session_id: "connection".to_string(),
        })
        .await;

        let tasks = widget_repeat_tasks().lock().await;
        assert!(!tasks.contains_key(&key("client-a")));
        assert!(tasks.contains_key(&key("client-b")));
        drop(tasks);
        cancel_all_widget_repeats().await;
    }

    #[tokio::test]
    async fn removing_or_rebinding_a_widget_clears_its_runtime_ownership() {
        let deck_id = 91_237;
        let removed = "removed:widget".to_string();
        let retained = "retained-widget".to_string();
        let record = || WidgetRuntimeRecord {
            revision: 1,
            status: "ready".to_string(),
            values: serde_json::Map::new(),
            provider_states: serde_json::Map::new(),
            request: None,
            error: None,
        };
        widget_runtime_records()
            .lock()
            .await
            .insert(format!("{deck_id}:{removed}"), record());
        widget_runtime_records()
            .lock()
            .await
            .insert(format!("{deck_id}:{retained}"), record());
        widget_public_state_paths()
            .lock()
            .await
            .insert(format!("{deck_id}:{removed}"), None);
        widget_preview_times()
            .lock()
            .await
            .insert(format!("{deck_id}:{removed}:value"), Instant::now());
        widget_intent_sequences().lock().await.entries.insert(
            WidgetSequenceKey {
                deck_id,
                document_revision: 1,
                instance_id: removed.clone(),
                port_id: "value".to_string(),
                input_source: "remote".to_string(),
                client_id: "client".to_string(),
                connection_session_id: "connection".to_string(),
                interaction_session_id: "interaction".to_string(),
            },
            (1, Instant::now()),
        );

        reset_widget_runtime_items(deck_id, std::slice::from_ref(&removed)).await;

        let records = widget_runtime_records().lock().await;
        assert!(!records.contains_key(&format!("{deck_id}:{removed}")));
        assert!(records.contains_key(&format!("{deck_id}:{retained}")));
        drop(records);
        assert!(!widget_public_state_paths()
            .lock()
            .await
            .contains_key(&format!("{deck_id}:{removed}")));
        assert!(!widget_preview_times()
            .lock()
            .await
            .contains_key(&format!("{deck_id}:{removed}:value")));
        assert!(widget_intent_sequences()
            .lock()
            .await
            .entries
            .keys()
            .all(|key| key.deck_id != deck_id || key.instance_id != removed));
        widget_runtime_records()
            .lock()
            .await
            .remove(&format!("{deck_id}:{retained}"));
    }

    #[test]
    fn runtime_projection_keeps_only_explicit_nested_paths() {
        let values = json!({
            "track": { "title": "Song", "privateUrl": "secret" },
            "playing": true,
            "accessToken": "never-send",
        });
        let projected =
            project_widget_values(&values, &["track.title".to_string(), "playing".to_string()]);
        assert_eq!(
            projected,
            json!({ "track": { "title": "Song" }, "playing": true })
        );
        assert!(!projected.to_string().contains("secret"));
        assert!(!projected.to_string().contains("never-send"));
    }

    #[test]
    fn cache_inventory_is_versioned_bounded_and_hash_only() {
        let hash = "a".repeat(64);
        let parsed = parse_widget_cache_inventory(&json!({
            "protocolVersion": 2,
            "schemaVersions": [1],
            "deckId": 7,
            "definitionHashes": [hash],
            "assetHashes": [],
        }))
        .expect("valid inventory");
        assert_eq!(parsed.0, 7);
        assert_eq!(parsed.1.len(), 1);
        assert!(parse_widget_cache_inventory(&json!({
            "protocolVersion": 2,
            "schemaVersions": [1],
            "deckId": 7,
            "definitionHashes": ["not-a-hash"],
            "assetHashes": [],
        }))
        .is_none());
    }
}
