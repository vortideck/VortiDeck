/// HTTP Request features module - inspired by n8n's architecture
/// Provides authentication, retry, pagination, and response formatting
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;

use super::error::{ApiError, ApiResult};
use super::request_builder::RequestBuilder;
use super::response::ApiResponse;

// ============================================================================
// CONFIGURATION STRUCTURES
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRequestConfig {
    pub method: String,
    pub url: String,

    #[serde(default)]
    pub authentication: AuthType,

    #[serde(default)]
    pub headers: HashMap<String, String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Value>,

    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,

    #[serde(default)]
    pub response_format: ResponseFormat,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<RetryConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub pagination: Option<PaginationConfig>,
}

fn default_timeout() -> u64 {
    30000 // 30 seconds
}

// ============================================================================
// AUTHENTICATION TYPES
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AuthType {
    None,

    #[serde(rename = "bearer")]
    BearerToken {
        token: String,
    },

    #[serde(rename = "basic")]
    BasicAuth {
        username: String,
        password: String,
    },

    #[serde(rename = "api_key")]
    ApiKey {
        header_name: String,
        api_key: String,
    },

    #[serde(rename = "custom_headers")]
    CustomHeaders {
        headers: HashMap<String, String>,
    },
}

impl Default for AuthType {
    fn default() -> Self {
        AuthType::None
    }
}

// ============================================================================
// RESPONSE FORMAT OPTIONS
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ResponseFormat {
    /// Returns full response object with status, headers, body, timing
    FullResponse,

    /// Returns only the response body (parsed as JSON if possible)
    #[default]
    BodyOnly,

    /// Returns { status, body }
    StatusAndBody,
}

// ============================================================================
// RETRY CONFIGURATION
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// HTTP status codes that should trigger a retry
    #[serde(default = "default_retry_status_codes")]
    pub retry_on_status: Vec<u16>,

    /// Initial backoff delay in milliseconds (doubles each retry)
    #[serde(default = "default_backoff_ms")]
    pub backoff_ms: u64,
}

fn default_max_retries() -> u32 {
    3
}
fn default_retry_status_codes() -> Vec<u16> {
    vec![429, 500, 502, 503, 504]
}
fn default_backoff_ms() -> u64 {
    100
}

// ============================================================================
// PAGINATION CONFIGURATION
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationConfig {
    pub mode: PaginationMode,

    /// Maximum number of requests to make
    #[serde(default = "default_max_requests")]
    pub max_requests: u32,

    /// Delay between requests in milliseconds
    #[serde(default = "default_interval_ms")]
    pub interval_ms: u64,
}

fn default_max_requests() -> u32 {
    10
}
fn default_interval_ms() -> u64 {
    1000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PaginationMode {
    /// Increment a parameter in the URL (e.g., page=1, page=2...)
    #[serde(rename = "update_parameter")]
    UpdateParameter {
        param_name: String,
        start_value: i64,
        increment: i64,
    },

    /// Follow a "next" URL from the response body
    #[serde(rename = "follow_next_url")]
    FollowNextUrl {
        /// JSON path to the next URL (e.g., "data.next_page" or "links.next")
        next_url_path: String,
    },
}

// ============================================================================
// MAIN EXECUTION FUNCTION
// ============================================================================

/// Execute HTTP request with all n8n-inspired features
pub async fn execute_http_request(mut config: HttpRequestConfig) -> ApiResult<Value> {
    let original_headers = config.headers.clone();
    config.headers = super::credentials::resolve_headers(config.headers)
        .await
        .map_err(ApiError::HttpError)?;
    config.authentication = super::credentials::resolve_authentication(config.authentication)
        .await
        .map_err(ApiError::HttpError)?;
    let mut secrets: Vec<String> = config
        .headers
        .iter()
        .filter(|(name, _)| {
            super::credentials::requires_protection(name)
                || original_headers
                    .get(*name)
                    .is_some_and(|value| super::credentials::is_credential_reference(value))
        })
        .flat_map(|(_, value)| {
            let mut values = vec![value.clone()];
            if let Some(token) = value
                .strip_prefix("Bearer ")
                .or_else(|| value.strip_prefix("Basic "))
            {
                values.push(token.to_string());
            }
            values
        })
        .collect();
    match &config.authentication {
        AuthType::BearerToken { token } => secrets.push(token.clone()),
        AuthType::BasicAuth { username, password } => {
            secrets.push(username.clone());
            secrets.push(password.clone());
            secrets.push(
                base64::engine::general_purpose::STANDARD
                    .encode(format!("{}:{}", username, password)),
            );
        }
        AuthType::ApiKey { api_key, .. } => secrets.push(api_key.clone()),
        AuthType::CustomHeaders { headers } => secrets.extend(headers.values().cloned()),
        AuthType::None => {}
    }
    let protect_credentials =
        !secrets.is_empty() || !matches!(config.authentication, AuthType::None);
    // Handle pagination if configured
    let mut result = if let Some(pagination) = config.pagination.clone() {
        execute_with_pagination(config, pagination, protect_credentials).await?
    } else {
        execute_single_request(&config, protect_credentials).await?
    };
    super::credentials::redact_secrets(&mut result, &secrets);
    Ok(result)
}

// ============================================================================
// SINGLE REQUEST EXECUTION
// ============================================================================

async fn execute_single_request(
    config: &HttpRequestConfig,
    protect_credentials: bool,
) -> ApiResult<Value> {
    // Handle retry if configured
    if let Some(retry_cfg) = &config.retry {
        return execute_with_retry(
            config,
            retry_cfg,
            &config.response_format,
            protect_credentials,
        )
        .await;
    }

    // Execute without retry
    let request = build_request(config, protect_credentials)?;
    let response = request.send().await?;
    Ok(format_response(response, &config.response_format))
}

// ============================================================================
// REQUEST BUILDER
// ============================================================================

fn build_request(
    config: &HttpRequestConfig,
    protect_credentials: bool,
) -> ApiResult<RequestBuilder> {
    // 1. Create base request with method
    let method = reqwest::Method::from_bytes(config.method.to_ascii_uppercase().as_bytes())
        .map_err(|_| ApiError::InvalidMethod(config.method.clone()))?;
    let mut request =
        RequestBuilder::new(method, &config.url).protect_credentials(protect_credentials);

    // 2. Apply authentication
    request = apply_authentication(request, &config.authentication)?;

    // 3. Add custom headers
    for (key, value) in &config.headers {
        request = request.header(key, value);
    }

    // 4. Add body if provided
    if let Some(body) = &config.body {
        request = request.json(body.clone());
    }

    // 5. Set timeout
    request = request.timeout_ms(config.timeout_ms);

    Ok(request)
}

fn apply_authentication(mut request: RequestBuilder, auth: &AuthType) -> ApiResult<RequestBuilder> {
    match auth {
        AuthType::None => Ok(request),

        AuthType::BearerToken { token } => Ok(request.bearer_token(token)),

        AuthType::BasicAuth { username, password } => {
            let encoded = base64::engine::general_purpose::STANDARD
                .encode(format!("{}:{}", username, password));
            Ok(request.header("Authorization", format!("Basic {}", encoded)))
        }

        AuthType::ApiKey {
            header_name,
            api_key,
        } => Ok(request.header(header_name, api_key)),

        AuthType::CustomHeaders { headers } => {
            for (key, value) in headers {
                request = request.header(key, value);
            }
            Ok(request)
        }
    }
}

// ============================================================================
// RETRY LOGIC
// ============================================================================

async fn execute_with_retry(
    config: &HttpRequestConfig,
    retry_cfg: &RetryConfig,
    response_format: &ResponseFormat,
    protect_credentials: bool,
) -> ApiResult<Value> {
    let mut last_error = None;

    for attempt in 0..=retry_cfg.max_retries {
        // Rebuild request each time (RequestBuilder isn't Clone)
        let request = build_request(config, protect_credentials)?;

        match request.send().await {
            Ok(response) => {
                // Check if status code requires retry
                if retry_cfg.retry_on_status.contains(&response.status)
                    && attempt < retry_cfg.max_retries
                {
                    let delay = Duration::from_millis(retry_cfg.backoff_ms * (2_u64.pow(attempt)));
                    log::warn!(
                        "HTTP request returned status {}, retrying in {}ms (attempt {}/{})",
                        response.status,
                        delay.as_millis(),
                        attempt + 1,
                        retry_cfg.max_retries
                    );
                    tokio::time::sleep(delay).await;
                    continue;
                }

                return Ok(format_response(response, response_format));
            }
            Err(e) => {
                last_error = Some(e);
                if attempt < retry_cfg.max_retries {
                    let delay = Duration::from_millis(retry_cfg.backoff_ms * (2_u64.pow(attempt)));
                    log::warn!(
                        "HTTP request failed, retrying in {}ms (attempt {}/{})",
                        delay.as_millis(),
                        attempt + 1,
                        retry_cfg.max_retries
                    );
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }

    Err(last_error.unwrap())
}

// ============================================================================
// PAGINATION LOGIC
// ============================================================================

async fn execute_with_pagination(
    config: HttpRequestConfig,
    pagination: PaginationConfig,
    protect_credentials: bool,
) -> ApiResult<Value> {
    let mut all_results = Vec::new();
    let mut current_url = config.url.clone();

    for page_num in 0..pagination.max_requests {
        // Build request for this page
        let mut page_config = config.clone();
        page_config.url = current_url.clone();
        page_config.pagination = None; // Prevent infinite recursion

        // Update URL for parameter-based pagination
        if let PaginationMode::UpdateParameter {
            param_name,
            start_value,
            increment,
        } = &pagination.mode
        {
            let page_value = start_value + (increment * page_num as i64);
            page_config.url =
                update_url_parameter(&page_config.url, param_name, &page_value.to_string());
        }

        // Execute request
        if protect_credentials {
            let original = reqwest::Url::parse(&config.url)
                .map_err(|_| ApiError::HttpError("Invalid request URL".into()))?;
            let page = reqwest::Url::parse(&page_config.url)
                .map_err(|_| ApiError::HttpError("Invalid pagination URL".into()))?;
            if original.origin() != page.origin() {
                return Err(ApiError::HttpError(
                    "Credential-bearing pagination cannot follow another origin".into(),
                ));
            }
        }
        let request = build_request(&page_config, protect_credentials)?;
        let response = request.send().await?;

        // Parse response body
        let body: Value =
            serde_json::from_str(&response.body).unwrap_or_else(|_| json!(response.body));

        all_results.push(body.clone());

        // Check stop conditions
        if is_empty_response(&body) {
            log::info!(
                "Pagination stopped: empty response at page {}",
                page_num + 1
            );
            break;
        }

        // Handle next URL pagination
        if let PaginationMode::FollowNextUrl { next_url_path } = &pagination.mode {
            if let Some(next_url) = extract_json_path(&body, next_url_path) {
                if let Some(url_str) = next_url.as_str() {
                    if url_str.is_empty() {
                        log::info!("Pagination stopped: no next URL at page {}", page_num + 1);
                        break;
                    }
                    current_url = url_str.to_string();
                } else {
                    break;
                }
            } else {
                log::info!(
                    "Pagination stopped: next URL not found at page {}",
                    page_num + 1
                );
                break;
            }
        }

        // Rate limiting between requests
        if page_num < pagination.max_requests - 1 {
            tokio::time::sleep(Duration::from_millis(pagination.interval_ms)).await;
        }
    }

    Ok(json!({
        "items": all_results,
        "total_pages": all_results.len(),
        "pagination_complete": true
    }))
}

// ============================================================================
// RESPONSE FORMATTING
// ============================================================================

fn format_response(response: ApiResponse, format: &ResponseFormat) -> Value {
    match format {
        ResponseFormat::FullResponse => json!({
            "status": response.status,
            "headers": response.headers,
            "body": parse_body(&response.body),
            "elapsed_ms": response.elapsed_ms,
        }),

        ResponseFormat::BodyOnly => parse_body(&response.body),

        ResponseFormat::StatusAndBody => json!({
            "status": response.status,
            "body": parse_body(&response.body),
        }),
    }
}

fn parse_body(body: &str) -> Value {
    // Try to parse as JSON, fallback to string
    serde_json::from_str(body).unwrap_or_else(|_| json!(body))
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn is_empty_response(body: &Value) -> bool {
    match body {
        Value::Array(arr) => arr.is_empty(),
        Value::Object(obj) => obj.is_empty(),
        Value::String(s) => s.is_empty(),
        Value::Null => true,
        _ => false,
    }
}

fn extract_json_path<'a>(data: &'a Value, path: &str) -> Option<&'a Value> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = data;

    for part in parts {
        current = current.get(part)?;
    }

    Some(current)
}

fn update_url_parameter(url: &str, param: &str, value: &str) -> String {
    let Ok(mut parsed) = reqwest::Url::parse(url) else {
        return url.to_string();
    };
    let pairs: Vec<(String, String)> = parsed
        .query_pairs()
        .filter(|(name, _)| name != param)
        .map(|(name, value)| (name.into_owned(), value.into_owned()))
        .collect();
    parsed.set_query(None);
    parsed
        .query_pairs_mut()
        .extend_pairs(pairs)
        .append_pair(param, value);
    parsed.into()
}

#[cfg(test)]
mod credential_transport_tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    async fn serve_redirect(
        destination: String,
        same_origin: bool,
        pagination: bool,
    ) -> (String, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let destination = if same_origin {
            format!("{origin}/final")
        } else {
            destination
        };
        let task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0; 4096];
            let _ = stream.read(&mut request).await.unwrap();
            let response = if pagination {
                let body = json!({"next":destination,"items":[1]}).to_string();
                format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())
            } else {
                format!("HTTP/1.1 302 Found\r\nLocation: {destination}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            };
            stream.write_all(response.as_bytes()).await.unwrap();
            if same_origin {
                let (mut stream, _) = listener.accept().await.unwrap();
                let count = stream.read(&mut request).await.unwrap();
                assert!(String::from_utf8_lossy(&request[..count]).contains("seed-private"));
                stream
                    .write_all(
                        b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
                    )
                    .await
                    .unwrap();
            }
        });
        (origin, task)
    }

    fn config(url: String, protected: bool) -> HttpRequestConfig {
        HttpRequestConfig {
            method: "GET".into(),
            url,
            authentication: if protected {
                AuthType::CustomHeaders {
                    headers: HashMap::from([("X-Private-Service".into(), "seed-private".into())]),
                }
            } else {
                AuthType::None
            },
            headers: HashMap::new(),
            body: None,
            timeout_ms: 2000,
            response_format: ResponseFormat::FullResponse,
            retry: None,
            pagination: None,
        }
    }

    #[tokio::test]
    async fn custom_credentials_never_reach_another_origin_by_redirect_or_pagination() {
        let destination = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/capture", destination.local_addr().unwrap());
        for pagination in [false, true] {
            let (origin, task) = serve_redirect(url.clone(), false, pagination).await;
            let mut request = config(origin, true);
            if pagination {
                request.pagination = Some(PaginationConfig {
                    mode: PaginationMode::FollowNextUrl {
                        next_url_path: "next".into(),
                    },
                    max_requests: 2,
                    interval_ms: 0,
                });
            }
            assert!(execute_http_request(request).await.is_err());
            task.await.unwrap();
            assert!(
                tokio::time::timeout(Duration::from_millis(100), destination.accept())
                    .await
                    .is_err()
            );
        }
    }

    #[tokio::test]
    async fn same_origin_credentials_and_public_redirects_remain_supported() {
        let (origin, task) = serve_redirect(String::new(), true, false).await;
        assert_eq!(
            execute_http_request(config(origin, true)).await.unwrap()["status"],
            200
        );
        task.await.unwrap();
        let destination = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", destination.local_addr().unwrap());
        let destination_task = tokio::spawn(async move {
            let (mut stream, _) = destination.accept().await.unwrap();
            let mut buffer = [0; 4096];
            let _ = stream.read(&mut buffer).await.unwrap();
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}")
                .await
                .unwrap();
        });
        let (origin, task) = serve_redirect(url, false, false).await;
        assert_eq!(
            execute_http_request(config(origin, false)).await.unwrap()["status"],
            200
        );
        task.await.unwrap();
        destination_task.await.unwrap();
    }

    #[test]
    fn pagination_replaces_existing_parameter_and_encodes_values() {
        let result = update_url_parameter(
            "https://example.test/items?page=1&filter=a%20b#section",
            "page",
            "2 & 3",
        );
        let parsed = reqwest::Url::parse(&result).unwrap();
        assert_eq!(
            parsed
                .query_pairs()
                .filter(|(key, _)| key == "page")
                .collect::<Vec<_>>(),
            vec![("page".into(), "2 & 3".into())]
        );
        assert_eq!(parsed.fragment(), Some("section"));
    }
}
