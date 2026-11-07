use chrono::Utc;
use diesel::prelude::*;
use tauri::{AppHandle, Emitter, State};

use vortideck_core::db::db::DbPool;
use vortideck_core::db::models::{HttpRequest, NewHttpRequest};
use vortideck_core::db::schema::http_requests;
use vortideck_core::services::api::execute_http_request;
use vortideck_core::services::api::saved_request::parse_saved_request;

/// Save a new HTTP request template to database
#[tauri::command(rename_all = "camelCase")]
pub async fn save_http_request(
    name: String,
    method: String,
    url: String,
    headers: Option<String>,
    body: Option<String>,
    timeout_ms: Option<i32>,
    accepted_status_codes: Option<Vec<u16>>,
    secret_headers: Option<Vec<String>>,
    db_pool: State<'_, DbPool>,
    app_handle: AppHandle,
) -> Result<(String, u16), String> {
    let config = parse_saved_request(&method, &url, headers.as_deref(), body.as_deref(), timeout_ms)?;
    vortideck_core::services::api::saved_request::validate_status_codes(accepted_status_codes.as_deref())?;
    let status_policy = accepted_status_codes.map(|codes| serde_json::to_string(&codes)).transpose().map_err(|error| error.to_string())?;
    let pool = db_pool.inner().clone();
    let saved_name = name.clone();
    tokio::task::spawn_blocking(move || {
        let protected = vortideck_core::services::api::credentials::protect_headers(
            config.headers, &secret_headers.unwrap_or_default())?;
        let saved = (|| {
            let headers_json = serde_json::to_string(&protected.headers).map_err(|_| "Invalid HTTP headers".to_string())?;
            let conn = &mut pool.get().map_err(|_| "HTTP storage unavailable".to_string())?;
            let new_request = NewHttpRequest {
                name: &saved_name, method: &method, url: &url,
                headers: Some(&headers_json), body: body.as_deref(), timeout_ms,
                created_at: &Utc::now().to_rfc3339(),
                accepted_status_codes: status_policy.as_deref(),
            };
            diesel::insert_into(http_requests::table).values(&new_request).execute(conn)
                .map_err(|_| "Failed to save HTTP request".to_string())?;
            Ok::<_, String>(())
        })();
        if saved.is_err() { protected.rollback(); }
        saved
    }).await.map_err(|_| "HTTP save interrupted".to_string())??;

    // Emit event to frontend that HTTP request was created
    let _ = app_handle.emit("http_request_created", ());

    Ok((format!("HTTP request '{}' saved successfully", name), 200))
}

/// Get all HTTP requests from database
#[tauri::command]
pub async fn list_http_requests(db_pool: State<'_, DbPool>) -> Result<Vec<HttpRequest>, String> {
    vortideck_core::db::Database::new(db_pool.inner()).list_http_requests().await
}

/// Get a specific HTTP request by ID
#[tauri::command(rename_all = "camelCase")]
pub async fn get_http_request_by_id(
    request_id: i32,
    db_pool: State<'_, DbPool>,
) -> Result<HttpRequest, String> {
    vortideck_core::db::Database::new(db_pool.inner()).get_http_request_by_id(request_id).await
}

/// Update an existing HTTP request in database
#[tauri::command(rename_all = "camelCase")]
pub async fn update_http_request(
    request_id: i32,
    name: String,
    method: String,
    url: String,
    headers: Option<String>,
    body: Option<String>,
    timeout_ms: Option<i32>,
    accepted_status_codes: Option<Vec<u16>>,
    secret_headers: Option<Vec<String>>,
    db_pool: State<'_, DbPool>,
    app_handle: AppHandle,
) -> Result<(String, u16), String> {
    use diesel::prelude::*;

    let config = parse_saved_request(&method, &url, headers.as_deref(), body.as_deref(), timeout_ms)?;
    vortideck_core::services::api::saved_request::validate_status_codes(accepted_status_codes.as_deref())?;
    let status_policy = accepted_status_codes.map(|codes| serde_json::to_string(&codes)).transpose().map_err(|error| error.to_string())?;
    let pool = db_pool.inner().clone();
    let saved_name = name.clone();
    tokio::task::spawn_blocking(move || {
        let protected = vortideck_core::services::api::credentials::protect_headers(
            config.headers, &secret_headers.unwrap_or_default())?;
        let saved = (|| {
            let headers_json = serde_json::to_string(&protected.headers).map_err(|_| "Invalid HTTP headers".to_string())?;
            let conn = &mut pool.get().map_err(|_| "HTTP storage unavailable".to_string())?;
            conn.immediate_transaction::<(), anyhow::Error, _>(|conn| {
            let candidate = HttpRequest { id: request_id, name: saved_name.clone(), method: method.clone(), url: url.clone(),
                headers: Some(headers_json.clone()), body: body.clone(), timeout_ms, accepted_status_codes: status_policy.clone(), created_at: String::new() };
            let template = vortideck_core::services::execution::action_converter::saved_http_request_template(&candidate).map_err(anyhow::Error::msg)?;
            let reference = vortideck_core::services::execution::ActionReference { kind: vortideck_core::services::execution::ActionReferenceKind::HttpRequest, id: request_id.to_string() };
            vortideck_core::db::database::validate_saved_input_contract(conn, &reference, &template).map_err(anyhow::Error::msg)?;
            let count = diesel::update(http_requests::table.find(request_id)).set((
                http_requests::name.eq(&saved_name), http_requests::method.eq(&method),
                http_requests::url.eq(&url), http_requests::headers.eq(Some(&headers_json)),
                http_requests::body.eq(body.as_deref()), http_requests::timeout_ms.eq(timeout_ms),
                http_requests::accepted_status_codes.eq(status_policy.as_deref()),
            )).execute(conn).map_err(|_| anyhow::anyhow!("Failed to update HTTP request"))?;
            anyhow::ensure!(count == 1, "HTTP request no longer exists");
            Ok(())
            }).map_err(|error| error.to_string())
        })();
        if saved.is_err() { protected.rollback(); }
        saved
    }).await.map_err(|_| "HTTP save interrupted".to_string())??;

    // Emit event to frontend that HTTP request was updated
    let _ = app_handle.emit("http_request_updated", ());

    Ok((format!("HTTP request '{}' updated successfully", name), 200))
}

/// Delete an HTTP request from database
#[tauri::command(rename_all = "camelCase")]
pub async fn delete_http_request(
    app_handle: AppHandle,
    request_id: i32,
    db_pool: State<'_, DbPool>,
) -> Result<(String, u16), String> {
    let conn = &mut db_pool.get().map_err(|e| e.to_string())?;

    diesel::delete(http_requests::table.find(request_id))
        .execute(conn)
        .map_err(|e| format!("Failed to delete HTTP request: {}", e))?;

    // Clean up context menu items referencing this HTTP request
    if crate::commands::context_menu_commands::clear_orphaned_items(
        "HttpRequestId",
        &request_id.to_string(),
    ) {
        let _ = app_handle.emit("context-menu-updated", ());
    }

    Ok(("HTTP request deleted successfully".to_string(), 200))
}

/// Execute an HTTP request by ID (for UI testing/preview)
///
/// NOTE: When executed via button press, the Universal Execution Engine is used instead.
/// This command is primarily for UI preview/testing before assigning to a button.
#[tauri::command(rename_all = "camelCase")]
pub async fn execute_http_request_by_id(
    request_id: i32,
    db_pool: State<'_, DbPool>,
) -> Result<serde_json::Value, String> {
    let request = vortideck_core::db::Database::new(db_pool.inner()).get_http_request_by_id(request_id).await?;

    let config = parse_saved_request(&request.method, &request.url, request.headers.as_deref(), request.body.as_deref(), request.timeout_ms)?;

    // 3. Execute using existing API service
    let response = execute_http_request(config)
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    // 4. Return response (it's already a JSON Value)
    Ok(response)
}

/// Test execute an HTTP request without saving (for UI testing)
#[tauri::command(rename_all = "camelCase")]
pub async fn test_http_request(
    method: String,
    url: String,
    headers: Option<String>,
    body: Option<String>,
    timeout_ms: Option<i32>,
    secret_headers: Option<Vec<String>>,
) -> Result<serde_json::Value, String> {
    let mut config = parse_saved_request(&method, &url, headers.as_deref(), body.as_deref(), timeout_ms)?;
    let secret_names = secret_headers.unwrap_or_default();
    let private_headers = config.headers.iter().filter(|(name, _)| secret_names.iter().any(|secret| secret.eq_ignore_ascii_case(name)))
        .map(|(name, value)| (name.clone(), value.clone())).collect::<std::collections::HashMap<_,_>>();
    if !private_headers.is_empty() {
        config.authentication = vortideck_core::services::api::http_request::AuthType::CustomHeaders { headers: private_headers };
    }

    let response = execute_http_request(config)
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    Ok(response)
}
