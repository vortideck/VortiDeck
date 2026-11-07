use diesel::prelude::*;
use diesel::sql_types::Text;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};

use vortideck_core::db::db::DbPool;
use vortideck_core::db::models::{
    ActionInvocationRecord, NewSavedActionBinding, SavedActionBinding,
};
use vortideck_core::db::schema::{
    buttons, deck_item_bindings, flows, macros, saved_actions, stream_sessions, webhooks,
};
use vortideck_core::integrations::{ActionPreflight, UniversalIntegrationManager};
use vortideck_core::services::execution::{
    ActionInvocationService, ActionReference, ButtonExecutionMode, ExecutionContext, UniversalAction,
    InvocationResult, RetryConfig, TriggerSource,
};

#[derive(Debug)]
struct BindingInput {
    name: String,
    action_id: String,
    parameters: HashMap<String, Value>,
    connection_selector: Option<String>,
    presentation: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedActionPreflight {
    pub stable_id: String,
    pub name: String,
    pub preflight: ActionPreflight,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedActionDependencies {
    pub stable_id: String,
    pub deck_buttons: i64,
    pub deck_widget_bindings: i64,
    pub flows: i64,
    pub macros: i64,
    pub webhooks: i64,
    pub stream_commands: i64,
    pub context_menu_items: i64,
}

impl SavedActionDependencies {
    fn total(&self) -> i64 {
        self.deck_buttons
            + self.deck_widget_bindings
            + self.flows
            + self.macros
            + self.webhooks
            + self.stream_commands
            + self.context_menu_items
    }
}

fn action_dependencies(
    connection: &mut SqliteConnection,
    stable_id: &str,
) -> SavedActionDependencies {
    let pattern = format!("%{}%", stable_id);
    SavedActionDependencies {
        stable_id: stable_id.to_string(),
        deck_buttons: buttons::table
            .filter(buttons::action.like(&pattern))
            .count()
            .get_result(connection)
            .unwrap_or(0),
        deck_widget_bindings: deck_item_bindings::table
            .filter(deck_item_bindings::action_json.like(&pattern))
            .count()
            .get_result(connection)
            .unwrap_or(0),
        flows: flows::table
            .filter(flows::data.like(&pattern))
            .count()
            .get_result(connection)
            .unwrap_or(0),
        macros: macros::table
            .filter(macros::macro_chain.like(&pattern))
            .count()
            .get_result(connection)
            .unwrap_or(0),
        webhooks: webhooks::table
            .filter(webhooks::action.like(&pattern))
            .count()
            .get_result(connection)
            .unwrap_or(0),
        stream_commands: stream_sessions::table
            .filter(stream_sessions::commands.like(&pattern))
            .count()
            .get_result(connection)
            .unwrap_or(0),
        context_menu_items: crate::commands::context_menu_commands::count_action_references(
            "SavedActionId",
            stable_id,
        ) as i64,
    }
}

fn parse_binding_input(action_data: &Value) -> Result<BindingInput, String> {
    let object = action_data
        .as_object()
        .ok_or_else(|| "Action binding must be a JSON object".to_string())?;
    let string_field = |names: &[&str]| {
        names
            .iter()
            .find_map(|name| object.get(*name).and_then(Value::as_str))
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    };

    let name = string_field(&["displayName", "display_name", "name"])
        .ok_or_else(|| "Action binding is missing displayName".to_string())?;
    let action_id = string_field(&["actionName", "action_name", "actionId", "action"])
        .ok_or_else(|| "Action binding is missing actionName".to_string())?;
    let parameters = match object.get("parameters").or_else(|| object.get("params")) {
        None => HashMap::new(),
        Some(Value::Object(values)) => values
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
        Some(_) => return Err("Action binding parameters must be an object".to_string()),
    };
    let connection_selector = object
        .get("connectionSelector")
        .filter(|value| !value.is_null())
        .map(serde_json::to_string)
        .transpose()
        .map_err(|error| error.to_string())?;
    let presentation = serde_json::to_string(
        object
            .get("presentation")
            .unwrap_or(&Value::Object(Default::default())),
    )
    .map_err(|error| error.to_string())?;

    Ok(BindingInput {
        name,
        action_id,
        parameters,
        connection_selector,
        presentation,
    })
}

fn manager(app_handle: &AppHandle) -> Result<Arc<UniversalIntegrationManager>, String> {
    app_handle
        .try_state::<Arc<UniversalIntegrationManager>>()
        .map(|manager| manager.inner().clone())
        .ok_or_else(|| "Integration contract registry is unavailable".to_string())
}

/// Canonical entry point for manually/API-invoked saved actions, macros, programs, and flows.
#[tauri::command(rename_all = "camelCase")]
pub async fn invoke_action_reference(
    invocation_service: State<'_, Arc<ActionInvocationService>>,
    reference: ActionReference,
    value: Option<f64>,
    variables: Option<HashMap<String, Value>>,
    inputs: Option<Value>,
) -> Result<InvocationResult, String> {
    let context = ExecutionContext {
        trigger_source: TriggerSource::Manual {
            user_id: "desktop".to_string(),
        },
        user_id: None,
        session_id: uuid::Uuid::new_v4().to_string(),
        variables: variables.unwrap_or_default(),
        timeout_ms: Some(30_000),
        retry_config: RetryConfig {
            max_retries: 0,
            retry_delay_ms: 0,
            exponential_backoff: false,
        },
        execution_mode: ButtonExecutionMode::RunOnce,
        execution_config: inputs.map(|inputs| serde_json::json!({ "inputs": inputs })),
    };

    invocation_service
        .invoke_reference(&reference, value, None, &context)
        .await
}

#[tauri::command(rename_all = "camelCase")]
pub async fn get_action_input_contract(
    invocation_service: State<'_, Arc<ActionInvocationService>>,
    reference: ActionReference,
) -> Result<vortideck_core::services::execution::input_contract::InputContractSnapshot, String> {
    invocation_service.get_input_contract(&reference).await
}

#[tauri::command(rename_all = "camelCase")]
pub async fn save_action_input_contract(
    invocation_service: State<'_, Arc<ActionInvocationService>>,
    reference: ActionReference,
    expected_revision: i64,
    contract: Option<vortideck_core::services::execution::input_contract::ActionInputContract>,
) -> Result<vortideck_core::services::execution::input_contract::InputContractSnapshot, String> {
    invocation_service.save_input_contract(&reference, expected_revision, contract).await
}

/// Inspect one canonical invocation, regardless of which surface started it.
#[tauri::command(rename_all = "camelCase")]
pub async fn get_action_invocation(
    invocation_service: State<'_, Arc<ActionInvocationService>>,
    invocation_id: String,
) -> Result<Option<ActionInvocationRecord>, String> {
    invocation_service.get_invocation(&invocation_id).await
}

/// Inspect recent canonical invocations. Filters use normalized persisted names
/// such as `flow`, `button`, `applied`, `failed`, or `uncertain`.
#[tauri::command(rename_all = "camelCase")]
pub async fn list_action_invocations(
    invocation_service: State<'_, Arc<ActionInvocationService>>,
    limit: Option<i64>,
    source: Option<String>,
    status: Option<String>,
    workflow_run_id: Option<String>,
    node_id: Option<String>,
) -> Result<Vec<ActionInvocationRecord>, String> {
    invocation_service
        .list_invocations(
            limit.unwrap_or(100).clamp(1, 500),
            source.as_deref(),
            status.as_deref(),
            workflow_run_id.as_deref(),
            node_id.as_deref(),
        )
        .await
}

#[tauri::command(rename_all = "camelCase")]
pub async fn save_integration_action_to_db(
    app_handle: AppHandle,
    db_pool: State<'_, DbPool>,
    integration_id: String,
    action_data: Value,
) -> Result<SavedActionBinding, String> {
    let input = parse_binding_input(&action_data)?;
    let capability = manager(&app_handle)?
        .resolve_capability(&integration_id, &input.action_id, &input.parameters)
        .await
        .map_err(|error| error.to_string())?;

    let stable_id = uuid::Uuid::new_v4().to_string();
    let params = serde_json::to_string(&input.parameters).map_err(|error| error.to_string())?;
    let mut connection = db_pool.get().map_err(|error| error.to_string())?;
    diesel::insert_into(saved_actions::table)
        .values(NewSavedActionBinding {
            stable_id: &stable_id,
            integration_id: &integration_id,
            name: &input.name,
            action: &input.action_id,
            params: &params,
            capability_version: capability.version as i32,
            connection_selector: input.connection_selector.as_deref(),
            presentation: &input.presentation,
        })
        .execute(&mut connection)
        .map_err(|error| error.to_string())?;

    saved_actions::table
        .filter(saved_actions::stable_id.eq(stable_id))
        .select(SavedActionBinding::as_select())
        .first(&mut connection)
        .map_err(|error| error.to_string())
}

#[tauri::command(rename_all = "camelCase")]
pub async fn update_integration_action_in_db(
    app_handle: AppHandle,
    db_pool: State<'_, DbPool>,
    integration_id: String,
    action_data: Value,
    action_id: i32,
) -> Result<SavedActionBinding, String> {
    let input = parse_binding_input(&action_data)?;
    let capability = manager(&app_handle)?
        .resolve_capability(&integration_id, &input.action_id, &input.parameters)
        .await
        .map_err(|error| error.to_string())?;

    let params = serde_json::to_string(&input.parameters).map_err(|error| error.to_string())?;
    let mut connection = db_pool.get().map_err(|error| error.to_string())?;
    connection.immediate_transaction::<SavedActionBinding, anyhow::Error, _>(|connection| {
    let stable_id = saved_actions::table.filter(saved_actions::id.eq(action_id)).filter(saved_actions::integration_id.eq(&integration_id))
        .select(saved_actions::stable_id).first::<String>(connection)?;
    let reference = ActionReference { kind: vortideck_core::services::execution::ActionReferenceKind::SavedAction, id: stable_id };
    let template = UniversalAction::Integration { integration_id: integration_id.clone(), action_id: input.action_id.clone(), parameters: input.parameters.clone() };
    vortideck_core::db::database::validate_saved_input_contract(connection, &reference, &template).map_err(anyhow::Error::msg)?;
    diesel::update(
        saved_actions::table
            .filter(saved_actions::id.eq(action_id))
            .filter(saved_actions::integration_id.eq(&integration_id)),
    )
    .set((
        saved_actions::name.eq(&input.name),
        saved_actions::action.eq(&input.action_id),
        saved_actions::params.eq(params),
        saved_actions::capability_version.eq(capability.version as i32),
        saved_actions::connection_selector.eq(input.connection_selector),
        saved_actions::presentation.eq(input.presentation),
    ))
    .execute(connection)?;

    saved_actions::table
        .filter(saved_actions::id.eq(action_id))
        .filter(saved_actions::integration_id.eq(integration_id))
        .select(SavedActionBinding::as_select())
        .first(connection).map_err(anyhow::Error::from)
    }).map_err(|error| error.to_string())
}

#[tauri::command(rename_all = "camelCase")]
pub async fn get_integration_action_by_id(
    db_pool: State<'_, DbPool>,
    integration_id: String,
    action_id: i32,
) -> Result<SavedActionBinding, String> {
    let mut connection = db_pool.get().map_err(|error| error.to_string())?;
    saved_actions::table
        .filter(saved_actions::id.eq(action_id))
        .filter(saved_actions::integration_id.eq(integration_id))
        .select(SavedActionBinding::as_select())
        .first(&mut connection)
        .map_err(|error| error.to_string())
}

#[tauri::command(rename_all = "camelCase")]
pub async fn get_saved_integration_action_by_stable_id(
    db_pool: State<'_, DbPool>,
    stable_id: String,
) -> Result<SavedActionBinding, String> {
    let mut connection = db_pool.get().map_err(|error| error.to_string())?;
    saved_actions::table
        .filter(saved_actions::stable_id.eq(stable_id))
        .select(SavedActionBinding::as_select())
        .first(&mut connection)
        .map_err(|error| error.to_string())
}

#[tauri::command(rename_all = "camelCase")]
pub async fn get_saved_integration_actions(
    db_pool: State<'_, DbPool>,
    integration_id: String,
) -> Result<Vec<SavedActionBinding>, String> {
    let mut connection = db_pool.get().map_err(|error| error.to_string())?;
    saved_actions::table
        .filter(saved_actions::integration_id.eq(integration_id))
        .order(saved_actions::name.asc())
        .select(SavedActionBinding::as_select())
        .load(&mut connection)
        .map_err(|error| error.to_string())
}

#[tauri::command(rename_all = "camelCase")]
pub async fn get_all_saved_integration_actions(
    db_pool: State<'_, DbPool>,
) -> Result<HashMap<String, Vec<SavedActionBinding>>, String> {
    let mut connection = db_pool.get().map_err(|error| error.to_string())?;
    let bindings = saved_actions::table
        .order((
            saved_actions::integration_id.asc(),
            saved_actions::name.asc(),
        ))
        .select(SavedActionBinding::as_select())
        .load::<SavedActionBinding>(&mut connection)
        .map_err(|error| error.to_string())?;
    let mut grouped = HashMap::new();
    for binding in bindings {
        grouped
            .entry(binding.integration_id.clone())
            .or_insert_with(Vec::new)
            .push(binding);
    }
    Ok(grouped)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn preflight_saved_integration_actions(
    app_handle: AppHandle,
    db_pool: State<'_, DbPool>,
    integration_id: String,
) -> Result<Vec<SavedActionPreflight>, String> {
    let bindings = get_saved_integration_actions(db_pool, integration_id).await?;
    let manager = manager(&app_handle)?;
    let mut results = Vec::with_capacity(bindings.len());
    for binding in bindings {
        let parameters =
            serde_json::from_str::<HashMap<String, Value>>(&binding.params).map_err(|error| {
                format!(
                    "Binding '{}' has invalid parameters: {}",
                    binding.name, error
                )
            })?;
        results.push(SavedActionPreflight {
            stable_id: binding.stable_id,
            name: binding.name,
            preflight: manager
                .preflight_action_versioned(
                    &binding.integration_id,
                    &binding.action,
                    &parameters,
                    Some(binding.capability_version.max(0) as u32),
                )
                .await,
        });
    }
    Ok(results)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn preflight_saved_integration_action(
    app_handle: AppHandle,
    db_pool: State<'_, DbPool>,
    stable_id: String,
) -> Result<SavedActionPreflight, String> {
    let mut connection = db_pool.get().map_err(|error| error.to_string())?;
    let binding = saved_actions::table
        .filter(saved_actions::stable_id.eq(&stable_id))
        .select(SavedActionBinding::as_select())
        .first::<SavedActionBinding>(&mut connection)
        .map_err(|error| error.to_string())?;
    let parameters =
        serde_json::from_str::<HashMap<String, Value>>(&binding.params).map_err(|error| {
            format!(
                "Binding '{}' has invalid parameters: {}",
                binding.name, error
            )
        })?;
    let preflight = manager(&app_handle)?
        .preflight_action_versioned(
            &binding.integration_id,
            &binding.action,
            &parameters,
            Some(binding.capability_version.max(0) as u32),
        )
        .await;

    Ok(SavedActionPreflight {
        stable_id: binding.stable_id,
        name: binding.name,
        preflight,
    })
}

#[tauri::command(rename_all = "camelCase")]
pub async fn get_saved_action_dependencies(
    db_pool: State<'_, DbPool>,
    stable_id: String,
) -> Result<SavedActionDependencies, String> {
    let mut connection = db_pool.get().map_err(|error| error.to_string())?;
    saved_actions::table
        .filter(saved_actions::stable_id.eq(&stable_id))
        .select(saved_actions::stable_id)
        .first::<String>(&mut connection)
        .map_err(|error| error.to_string())?;
    Ok(action_dependencies(&mut connection, &stable_id))
}

/// Rebind in place: the stable identity remains unchanged, so every consumer
/// immediately sees the provider's new validated contract.
#[tauri::command(rename_all = "camelCase")]
pub async fn rebind_saved_integration_action(
    app_handle: AppHandle,
    db_pool: State<'_, DbPool>,
    stable_id: String,
    integration_id: String,
    action_data: Value,
) -> Result<SavedActionBinding, String> {
    let input = parse_binding_input(&action_data)?;
    let capability = manager(&app_handle)?
        .resolve_capability(&integration_id, &input.action_id, &input.parameters)
        .await
        .map_err(|error| error.to_string())?;
    let params = serde_json::to_string(&input.parameters).map_err(|error| error.to_string())?;
    let mut connection = db_pool.get().map_err(|error| error.to_string())?;
    connection.immediate_transaction::<SavedActionBinding, anyhow::Error, _>(|connection| {
    let reference = ActionReference { kind: vortideck_core::services::execution::ActionReferenceKind::SavedAction, id: stable_id.clone() };
    let template = UniversalAction::Integration { integration_id: integration_id.clone(), action_id: input.action_id.clone(), parameters: input.parameters.clone() };
    vortideck_core::db::database::validate_saved_input_contract(connection, &reference, &template).map_err(anyhow::Error::msg)?;
    let changed =
        diesel::update(saved_actions::table.filter(saved_actions::stable_id.eq(&stable_id)))
            .set((
                saved_actions::integration_id.eq(&integration_id),
                saved_actions::name.eq(&input.name),
                saved_actions::action.eq(&input.action_id),
                saved_actions::params.eq(params),
                saved_actions::capability_version.eq(capability.version as i32),
                saved_actions::connection_selector.eq(input.connection_selector),
                saved_actions::presentation.eq(input.presentation),
            ))
            .execute(connection)?;
    if changed == 0 {
        anyhow::bail!("Saved action '{}' was not found", stable_id);
    }
    saved_actions::table
        .filter(saved_actions::stable_id.eq(stable_id))
        .select(SavedActionBinding::as_select())
        .first(connection).map_err(anyhow::Error::from)
    }).map_err(|error| error.to_string())
}

/// Atomically replace references in every database-backed action surface.
/// Context-menu configuration is file-backed and is updated immediately after
/// the database transaction; its changed count is included in the result.
#[tauri::command(rename_all = "camelCase")]
pub async fn replace_saved_action_references(
    db_pool: State<'_, DbPool>,
    old_stable_id: String,
    new_stable_id: String,
) -> Result<SavedActionDependencies, String> {
    if old_stable_id == new_stable_id {
        return Err("Replacement action must be different".to_string());
    }
    let mut connection = db_pool.get().map_err(|error| error.to_string())?;
    let existing = saved_actions::table
        .filter(saved_actions::stable_id.eq_any([&old_stable_id, &new_stable_id]))
        .select(saved_actions::stable_id)
        .load::<String>(&mut connection)
        .map_err(|error| error.to_string())?;
    if !existing.iter().any(|id| id == &old_stable_id)
        || !existing.iter().any(|id| id == &new_stable_id)
    {
        return Err("Both the source and replacement saved actions must exist".to_string());
    }
    let before = action_dependencies(&mut connection, &old_stable_id);
    let pattern = format!("%{}%", old_stable_id);
    connection
        .transaction::<_, diesel::result::Error, _>(|connection| {
            for (table, column) in [
                ("buttons", "action"),
                ("deck_item_bindings", "action_json"),
                ("flows", "data"),
                ("macros", "macro_chain"),
                ("webhooks", "action"),
                ("stream_sessions", "commands"),
            ] {
                diesel::sql_query(format!(
                    "UPDATE {table} SET {column} = replace({column}, ?, ?) WHERE {column} LIKE ?"
                ))
                .bind::<Text, _>(&old_stable_id)
                .bind::<Text, _>(&new_stable_id)
                .bind::<Text, _>(&pattern)
                .execute(connection)?;
            }
            Ok(())
        })
        .map_err(|error| error.to_string())?;
    let context_count = crate::commands::context_menu_commands::replace_action_references(
        "SavedActionId",
        &old_stable_id,
        &new_stable_id,
    )? as i64;
    Ok(SavedActionDependencies {
        context_menu_items: context_count,
        ..before
    })
}

#[tauri::command(rename_all = "camelCase")]
pub async fn disable_flows_using_saved_action(
    db_pool: State<'_, DbPool>,
    stable_id: String,
) -> Result<usize, String> {
    let mut connection = db_pool.get().map_err(|error| error.to_string())?;
    let pattern = format!("%{}%", stable_id);
    let records = flows::table
        .filter(flows::data.like(pattern))
        .select((flows::id, flows::data))
        .load::<(String, String)>(&mut connection)
        .map_err(|error| error.to_string())?;
    connection
        .transaction::<_, diesel::result::Error, _>(|connection| {
            for (id, raw) in &records {
                let mut document: Value = serde_json::from_str(raw)
                    .map_err(|error| diesel::result::Error::SerializationError(Box::new(error)))?;
                if let Some(object) = document.as_object_mut() {
                    object.insert("isActive".to_string(), Value::Bool(false));
                }
                let updated = serde_json::to_string(&document)
                    .map_err(|error| diesel::result::Error::SerializationError(Box::new(error)))?;
                diesel::update(flows::table.filter(flows::id.eq(id)))
                    .set(flows::data.eq(updated))
                    .execute(connection)?;
            }
            Ok(())
        })
        .map_err(|error| error.to_string())?;
    Ok(records.len())
}

#[tauri::command(rename_all = "camelCase")]
pub async fn remove_integration_action_from_db(
    db_pool: State<'_, DbPool>,
    integration_id: String,
    action_id: i32,
) -> Result<(), String> {
    let mut connection = db_pool.get().map_err(|error| error.to_string())?;
    let binding = saved_actions::table
        .filter(saved_actions::id.eq(action_id))
        .filter(saved_actions::integration_id.eq(&integration_id))
        .select(SavedActionBinding::as_select())
        .first::<SavedActionBinding>(&mut connection)
        .map_err(|error| error.to_string())?;
    let dependencies = action_dependencies(&mut connection, &binding.stable_id);
    if dependencies.total() > 0 {
        return Err(format!(
            "Cannot delete '{}': it has {} active reference(s). Replace or disable those references first.",
            binding.name,
            dependencies.total()
        ));
    }
    let deleted = diesel::delete(
        saved_actions::table
            .filter(saved_actions::id.eq(action_id))
            .filter(saved_actions::integration_id.eq(integration_id)),
    )
    .execute(&mut connection)
    .map_err(|error| error.to_string())?;
    if deleted == 0 {
        return Err(format!(
            "Saved action binding '{}' was not found",
            action_id
        ));
    }
    Ok(())
}
