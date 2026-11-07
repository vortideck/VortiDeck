//! Strict saved-request parsing shared by persistence, preview and invocation.

use super::http_request::{AuthType, HttpRequestConfig, ResponseFormat};
use serde_json::Value;
use std::collections::HashMap;

/// The saved-action and flow-node boundaries share the same header contract.
pub fn parse_header_values(value: Option<&Value>) -> Result<HashMap<String, String>, String> {
    let headers: HashMap<String, String> = match value.filter(|value| !value.is_null()) {
        Some(value) => serde_json::from_value(value.clone())
            .map_err(|_| "Headers must be a JSON object with string values")?,
        None => HashMap::new(),
    };
    for (name, value) in &headers {
        reqwest::header::HeaderName::from_bytes(name.as_bytes())
            .map_err(|_| "Invalid HTTP header name")?;
        reqwest::header::HeaderValue::from_str(value).map_err(|_| "Invalid HTTP header value")?;
    }
    Ok(headers)
}

pub fn parse_saved_request(
    method: &str,
    url: &str,
    headers: Option<&str>,
    body: Option<&str>,
    timeout_ms: Option<i32>,
) -> Result<HttpRequestConfig, String> {
    reqwest::Method::from_bytes(method.as_bytes()).map_err(|_| "Invalid HTTP method")?;
    let target = reqwest::Url::parse(url).map_err(|_| "Invalid HTTP URL")?;
    if !matches!(target.scheme(), "http" | "https") || target.host_str().is_none() {
        return Err("HTTP requests require an http:// or https:// URL".into());
    }
    let header_value: Option<Value> = match headers.filter(|value| !value.trim().is_empty()) {
        Some(value) => Some(
            serde_json::from_str(value)
                .map_err(|_| "Headers must be a JSON object with string values")?,
        ),
        None => None,
    };
    if header_value.as_ref().is_some_and(Value::is_null) {
        return Err("Headers must be a JSON object with string values".into());
    }
    let headers = parse_header_values(header_value.as_ref())?;
    let body: Option<Value> = match body.filter(|value| !value.trim().is_empty()) {
        Some(value) => {
            Some(serde_json::from_str(value).map_err(|_| "Request body must contain valid JSON")?)
        }
        None => None,
    };
    let timeout_ms = timeout_ms.unwrap_or(30_000);
    if timeout_ms <= 0 {
        return Err("HTTP timeout must be positive".into());
    }
    Ok(HttpRequestConfig {
        method: method.to_ascii_uppercase(),
        url: url.to_string(),
        authentication: AuthType::None,
        headers,
        body,
        timeout_ms: timeout_ms as u64,
        response_format: ResponseFormat::FullResponse,
        retry: None,
        pagination: None,
    })
}

pub fn validate_status_codes(codes: Option<&[u16]>) -> Result<(), String> {
    if codes.is_some_and(|codes| {
        codes.is_empty() || codes.iter().any(|code| !(100..=599).contains(code))
    }) {
        return Err(
            "Accepted HTTP statuses must be a nonempty list of codes between 100 and 599".into(),
        );
    }
    Ok(())
}

pub fn accepts_status(codes: Option<&[u16]>, status: u16) -> bool {
    codes.map_or_else(
        || (200..300).contains(&status),
        |codes| codes.contains(&status),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_configuration_is_rejected_without_silent_defaults() {
        let parse = |headers, body| {
            parse_saved_request("POST", "http://127.0.0.1:1234/", headers, body, None)
        };
        assert!(parse(Some("bad"), None).is_err());
        assert!(parse(Some(r#"{"X-Test":12}"#), None).is_err());
        assert!(parse(None, Some("{bad")).is_err());
        assert!(parse(Some(r#"{"X-Test":"a\r\nb"}"#), None).is_err());
        assert!(parse_saved_request("GET", "file:///private", None, None, None).is_err());
        assert!(parse_saved_request("GET", "http://localhost/", None, None, Some(-1)).is_err());
    }
    #[test]
    fn literal_json_and_optional_empty_body_are_preserved() {
        let config = parse_saved_request(
            "POST",
            "http://localhost/control",
            Some(r#"{"Content-Type":"application/json"}"#),
            Some(r#"{"text":"hi\"there","count":2,"active":false}"#),
            Some(1234),
        )
        .unwrap();
        assert_eq!(config.timeout_ms, 1234);
        assert_eq!(config.body.unwrap()["count"], 2);
        assert!(
            parse_saved_request("GET", "https://example.com/", Some(" "), Some(" "), None)
                .unwrap()
                .body
                .is_none()
        );
    }
    #[tokio::test]
    async fn saved_action_status_and_response_privacy_use_the_real_http_boundary() {
        use crate::services::execution::{
            ButtonExecutionMode, ExecutionContext, ExecutionEngine, RetryConfig, TriggerSource,
            UniversalAction, UniversalExecutionEngine,
        };
        use std::sync::Arc;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let engine = UniversalExecutionEngine::new(Arc::new(
            crate::core::state_manager::StateManager::new(),
        ));
        let context = ExecutionContext {
            trigger_source: TriggerSource::Manual {
                user_id: "http-fixture".into(),
            },
            user_id: None,
            session_id: "http-fixture".into(),
            variables: HashMap::new(),
            timeout_ms: Some(2000),
            retry_config: RetryConfig {
                max_retries: 0,
                retry_delay_ms: 0,
                exponential_backoff: false,
            },
            execution_mode: ButtonExecutionMode::RunOnce,
            execution_config: None,
        };
        for (status, explicit_acceptance) in [
            (200, false),
            (204, false),
            (400, false),
            (401, false),
            (429, false),
            (500, false),
            (404, true),
        ] {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let url = format!(
                "http://{}/fixture?private=query",
                listener.local_addr().unwrap()
            );
            let server = tokio::spawn(async move {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut bytes = [0; 4096];
                let count = socket.read(&mut bytes).await.unwrap();
                assert!(String::from_utf8_lossy(&bytes[..count]).contains("seeded-secret"));
                let body = if status == 204 {
                    String::new()
                } else {
                    r#"{"echo":"Bearer seeded-secret","message":"fixture"}"#.into()
                };
                socket.write_all(format!("HTTP/1.1 {} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", status, body.len(), body).as_bytes()).await.unwrap();
            });
            let action = UniversalAction::HttpRequest {
                name: "Fixture".into(),
                method: "POST".into(),
                url,
                headers: HashMap::from([("Authorization".into(), "Bearer seeded-secret".into())]),
                body: Some(serde_json::json!({"literal":"unchanged"})),
                timeout_ms: 2000,
                accepted_status_codes: explicit_acceptance.then_some(vec![status]),
            };
            let result = engine.execute_action(&action, &context).await;
            server.await.unwrap();
            assert_eq!(
                result.success,
                (200..300).contains(&status) || explicit_acceptance,
                "status {}",
                status
            );
            assert_eq!(result.outputs["status"], status);
            assert!(!serde_json::to_string(&result.outputs)
                .unwrap()
                .contains("seeded-secret"));
            assert!(!serde_json::to_string(&result.logs)
                .unwrap()
                .contains("private=query"));
            if status != 204 {
                assert_eq!(result.outputs["response"]["message"], "fixture");
            }
            assert_eq!(result.retry_count, 0);
        }
    }
}
