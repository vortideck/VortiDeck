use crate::services::api::{
    execute_http_request, AuthType, HttpRequestConfig, PaginationConfig, PaginationMode,
    ResponseFormat, RetryConfig,
};
/// HTTP Request nodes for making API calls
/// Based on n8n's HTTP Request node architecture
use crate::services::flow::traits::{FlowNode, FlowNodeProvider, NodeOutput};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;

/// HTTP Request Node - Make HTTP/HTTPS requests to any API
pub struct HttpRequestNode;

#[async_trait]
impl FlowNode for HttpRequestNode {
    fn id(&self) -> String {
        "http_request".to_string()
    }

    fn name(&self) -> String {
        "HTTP Request".to_string()
    }

    fn description(&self) -> String {
        "Make HTTP/HTTPS requests with authentication, retry, and pagination support".to_string()
    }

    fn category(&self) -> String {
        "ACTION".to_string()
    }

    fn category_display_name(&self) -> Option<String> {
        Some("Actions".to_string())
    }

    fn subcategory(&self) -> Option<String> {
        Some("Network".to_string())
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "method": {
                    "type": "string",
                    "enum": ["GET", "POST", "PUT", "DELETE", "PATCH"],
                    "default": "GET",
                    "description": "HTTP method to use"
                },
                "url": {
                    "type": "string",
                    "description": "The URL to make the request to (must start with http:// or https://)",
                    "placeholder": "https://api.example.com/endpoint"
                },
                "authentication": {
                    "type": "object",
                    "description": "Authentication configuration",
                    "properties": {
                        "type": {
                            "type": "string",
                            "enum": ["none", "bearer", "basic", "api_key", "custom_headers"],
                            "default": "none",
                            "description": "Authentication type"
                        },
                        "token": {
                            "type": "string",
                            "description": "Bearer token (for type: bearer)"
                        },
                        "username": {
                            "type": "string",
                            "description": "Username (for type: basic)"
                        },
                        "password": {
                            "type": "string",
                            "description": "Password (for type: basic)"
                        },
                        "header_name": {
                            "type": "string",
                            "description": "Header name (for type: api_key)",
                            "default": "X-API-Key"
                        },
                        "api_key": {
                            "type": "string",
                            "description": "API key value (for type: api_key)"
                        },
                        "headers": {
                            "type": "object",
                            "description": "Custom headers (for type: custom_headers)"
                        }
                    }
                },
                "headers": {
                    "type": "object",
                    "description": "Additional HTTP headers to send",
                    "default": {}
                },
                "body": {
                    "type": ["object", "string", "null"],
                    "description": "Request body (for POST/PUT/PATCH)"
                },
                "timeout_ms": {
                    "type": "number",
                    "default": 30000,
                    "description": "Request timeout in milliseconds"
                },
                "response_format": {
                    "type": "string",
                    "enum": ["body_only", "full_response", "status_and_body"],
                    "default": "body_only",
                    "description": "How to format the response"
                },
                "retry": {
                    "type": "object",
                    "description": "Retry configuration",
                    "properties": {
                        "enabled": {
                            "type": "boolean",
                            "default": false,
                            "description": "Enable automatic retry on failure"
                        },
                        "max_retries": {
                            "type": "number",
                            "default": 3,
                            "description": "Maximum number of retry attempts"
                        },
                        "retry_on_status": {
                            "type": "array",
                            "items": { "type": "number" },
                            "default": [429, 500, 502, 503, 504],
                            "description": "HTTP status codes that trigger retry"
                        },
                        "backoff_ms": {
                            "type": "number",
                            "default": 100,
                            "description": "Initial backoff delay in ms (doubles each retry)"
                        }
                    }
                },
                "pagination": {
                    "type": "object",
                    "description": "Pagination configuration",
                    "properties": {
                        "enabled": {
                            "type": "boolean",
                            "default": false,
                            "description": "Enable automatic pagination"
                        },
                        "mode": {
                            "type": "string",
                            "enum": ["update_parameter", "follow_next_url"],
                            "default": "update_parameter",
                            "description": "Pagination strategy"
                        },
                        "param_name": {
                            "type": "string",
                            "default": "page",
                            "description": "Parameter name to increment (for update_parameter mode)"
                        },
                        "start_value": {
                            "type": "number",
                            "default": 1,
                            "description": "Starting page number (for update_parameter mode)"
                        },
                        "increment": {
                            "type": "number",
                            "default": 1,
                            "description": "Amount to increment each page (for update_parameter mode)"
                        },
                        "next_url_path": {
                            "type": "string",
                            "default": "data.next_page",
                            "description": "JSON path to next URL (for follow_next_url mode)"
                        },
                        "max_requests": {
                            "type": "number",
                            "default": 10,
                            "description": "Maximum number of pages to fetch"
                        },
                        "interval_ms": {
                            "type": "number",
                            "default": 1000,
                            "description": "Delay between requests in ms"
                        }
                    }
                }
            },
            "required": ["method", "url"]
        })
    }

    fn type_color(&self) -> String {
        "#10b981".to_string() // Green
    }

    fn outputs_schema(&self) -> Vec<NodeOutput> {
        vec![
            NodeOutput {
                id: "response".to_string(),
                name: "Response".to_string(),
                output_type: "any".to_string(),
                description: Some(
                    "HTTP response (format depends on response_format setting)".to_string(),
                ),
                ..Default::default()
            },
            NodeOutput {
                id: "success".to_string(),
                name: "Success".to_string(),
                output_type: "boolean".to_string(),
                description: Some("Whether the request was successful".to_string()),
                ..Default::default()
            },
        ]
    }

    async fn execute(&self, params: Value) -> Result<Value, String> {
        // Parse parameters
        let method = params["method"]
            .as_str()
            .ok_or("Missing method")?
            .to_string();
        let url = params["url"].as_str().ok_or("Missing URL")?.to_string();

        // Validate URL
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err("HTTP URL must start with http:// or https://".into());
        }

        // Parse authentication
        let authentication = parse_authentication(&params)?;

        // Parse headers
        let headers = parse_headers(&params)?;

        // Parse body
        let body = params.get("body").cloned();

        // Parse timeout
        let timeout_ms = params["timeout_ms"].as_u64().unwrap_or(30000);

        // Parse response format
        let response_format = parse_response_format(&params)?;

        // Parse retry config
        let retry = parse_retry_config(&params)?;

        // Parse pagination config
        let pagination = parse_pagination_config(&params)?;

        // Build HTTP request config
        let config = HttpRequestConfig {
            method,
            url,
            authentication,
            headers,
            body,
            timeout_ms,
            response_format,
            retry,
            pagination,
        };

        // Execute HTTP request
        log::info!("🌐 HTTP Request: {}", config.method);

        match execute_http_request(config).await {
            Ok(response) => {
                log::info!("✅ HTTP Request completed successfully");
                Ok(json!({
                    "response": response,
                    "success": true
                }))
            }
            Err(e) => {
                log::error!("❌ HTTP Request failed: {}", e);
                Err(format!("HTTP request failed: {}", e))
            }
        }
    }

    async fn execute_with_context(
        &self,
        mut params: Value,
        ctx: &mut crate::services::flow::traits::ExecutionContext<'_>,
    ) -> Result<Value, String> {
        let method = params
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or("GET")
            .to_ascii_uppercase();
        if matches!(method.as_str(), "POST" | "PUT" | "PATCH" | "DELETE") {
            let object = params
                .as_object_mut()
                .ok_or("HTTP request parameters must be an object")?;
            let headers = object.entry("headers").or_insert_with(|| json!({}));
            let headers = headers
                .as_object_mut()
                .ok_or("HTTP request headers must be an object")?;
            if !headers
                .keys()
                .any(|key| key.eq_ignore_ascii_case("Idempotency-Key"))
            {
                headers.insert(
                    "Idempotency-Key".to_string(),
                    Value::String(format!("flow-http:{}:{}", ctx.flow_id, ctx.node_id)),
                );
            }
        }
        self.execute(params).await
    }
}

// ============================================================================
// HELPER FUNCTIONS FOR PARSING PARAMETERS
// ============================================================================

fn parse_authentication(params: &Value) -> Result<AuthType, String> {
    let auth = &params["authentication"];

    if auth.is_null() {
        return Ok(AuthType::None);
    }
    if !auth.is_object() {
        return Err("HTTP authentication must be an object".into());
    }
    let auth_type = match auth.get("type") {
        None => "none",
        Some(value) => value
            .as_str()
            .ok_or("HTTP authentication type must be a string")?,
    };

    match auth_type {
        "none" => Ok(AuthType::None),

        "bearer" => {
            let token = auth["token"]
                .as_str()
                .ok_or("Missing token for bearer authentication")?
                .to_string();
            Ok(AuthType::BearerToken { token })
        }

        "basic" => {
            let username = auth["username"]
                .as_str()
                .ok_or("Missing username for basic authentication")?
                .to_string();
            let password = auth["password"]
                .as_str()
                .ok_or("Missing password for basic authentication")?
                .to_string();
            Ok(AuthType::BasicAuth { username, password })
        }

        "api_key" => {
            let header_name = auth["header_name"]
                .as_str()
                .unwrap_or("X-API-Key")
                .to_string();
            let api_key = auth["api_key"]
                .as_str()
                .ok_or("Missing api_key for api_key authentication")?
                .to_string();
            Ok(AuthType::ApiKey {
                header_name,
                api_key,
            })
        }

        "custom_headers" => {
            let headers_obj = auth["headers"]
                .as_object()
                .ok_or("Missing headers for custom_headers authentication")?;

            let headers = crate::services::api::saved_request::parse_header_values(Some(
                &Value::Object(headers_obj.clone()),
            ))?;

            Ok(AuthType::CustomHeaders { headers })
        }

        _ => Err(format!("Unknown authentication type: {}", auth_type)),
    }
}

fn parse_headers(params: &Value) -> Result<HashMap<String, String>, String> {
    crate::services::api::saved_request::parse_header_values(params.get("headers"))
}

fn parse_response_format(params: &Value) -> Result<ResponseFormat, String> {
    let format_str = params["response_format"].as_str().unwrap_or("body_only");

    match format_str {
        "body_only" => Ok(ResponseFormat::BodyOnly),
        "full_response" => Ok(ResponseFormat::FullResponse),
        "status_and_body" => Ok(ResponseFormat::StatusAndBody),
        _ => Ok(ResponseFormat::BodyOnly), // Default
    }
}

fn parse_retry_config(params: &Value) -> Result<Option<RetryConfig>, String> {
    let retry = &params["retry"];

    if !retry["enabled"].as_bool().unwrap_or(false) {
        return Ok(None);
    }

    let max_retries = retry["max_retries"].as_u64().unwrap_or(3) as u32;
    let backoff_ms = retry["backoff_ms"].as_u64().unwrap_or(100);

    let retry_on_status = if let Some(status_array) = retry["retry_on_status"].as_array() {
        status_array
            .iter()
            .filter_map(|v| v.as_u64())
            .map(|n| n as u16)
            .collect()
    } else {
        vec![429, 500, 502, 503, 504]
    };

    Ok(Some(RetryConfig {
        max_retries,
        retry_on_status,
        backoff_ms,
    }))
}

fn parse_pagination_config(params: &Value) -> Result<Option<PaginationConfig>, String> {
    let pagination = &params["pagination"];

    if !pagination["enabled"].as_bool().unwrap_or(false) {
        return Ok(None);
    }

    let mode_str = pagination["mode"].as_str().unwrap_or("update_parameter");
    let max_requests = pagination["max_requests"].as_u64().unwrap_or(10) as u32;
    let interval_ms = pagination["interval_ms"].as_u64().unwrap_or(1000);

    let mode = match mode_str {
        "update_parameter" => {
            let param_name = pagination["param_name"]
                .as_str()
                .unwrap_or("page")
                .to_string();
            let start_value = pagination["start_value"].as_i64().unwrap_or(1);
            let increment = pagination["increment"].as_i64().unwrap_or(1);

            PaginationMode::UpdateParameter {
                param_name,
                start_value,
                increment,
            }
        }
        "follow_next_url" => {
            let next_url_path = pagination["next_url_path"]
                .as_str()
                .unwrap_or("data.next_page")
                .to_string();

            PaginationMode::FollowNextUrl { next_url_path }
        }
        _ => {
            return Err(format!("Unknown pagination mode: {}", mode_str));
        }
    };

    Ok(Some(PaginationConfig {
        mode,
        max_requests,
        interval_ms,
    }))
}

// ============================================================================
// NODE PROVIDER
// ============================================================================

pub struct HttpNodesProvider;

impl FlowNodeProvider for HttpNodesProvider {
    fn get_flow_nodes(&self) -> Vec<Box<dyn FlowNode>> {
        vec![Box::new(HttpRequestNode)]
    }

    fn provider_name(&self) -> &'static str {
        "HttpNodesProvider"
    }
}

#[cfg(test)]
mod boundary_tests {
    use super::*;

    #[tokio::test]
    async fn malformed_flow_headers_and_authentication_fail_before_transport() {
        for extra in [
            json!({"headers": {"X-Private": 42}}),
            json!({"headers": "not-an-object"}),
            json!({"headers": {"X-Test": "a\r\nb"}}),
            json!({"authentication": "bearer"}),
            json!({"authentication": {"type": 42}}),
            json!({"authentication": {"type": "custom_headers", "headers": {"X-Private": false}}}),
        ] {
            // An unreachable address makes accidental transport observable.
            let mut params =
                json!({"method": "POST", "url": "http://127.0.0.1:1/", "timeout_ms": 10});
            params
                .as_object_mut()
                .unwrap()
                .extend(extra.as_object().unwrap().clone());
            let error = HttpRequestNode.execute(params).await.unwrap_err();
            assert!(
                error.contains("header")
                    || error.contains("Header")
                    || error.contains("authentication"),
                "{error}"
            );
            assert!(!error.contains("42"));
        }
        assert!(parse_headers(&json!({})).unwrap().is_empty());
        assert_eq!(
            parse_headers(&json!({"headers":{"X-Test":"literal"}})).unwrap()["X-Test"],
            "literal"
        );
    }
}
