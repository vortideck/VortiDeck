//! Warp HTTP adapter for webhook handler
//! Converts between Warp types and platform-agnostic webhook types

use std::sync::Arc;
use vortideck_core::services::webhook::{
    handler_types::{HttpMethod, WebhookRequest, WebhookResponse},
    WebhookHandler,
};
use warp::Filter;

/// Largest webhook body accepted (1 MiB). Webhooks are reachable from the LAN.
const MAX_WEBHOOK_BODY_BYTES: u64 = 1024 * 1024;

/// Extract the request body only when its declared size is within
/// `MAX_WEBHOOK_BODY_BYTES`. Bodiless requests (no Content-Length and no
/// Transfer-Encoding, e.g. GET) are still accepted; streamed bodies without a
/// Content-Length are rejected (411) so the size is enforced before reading.
fn bounded_body(
) -> impl Filter<Extract = (warp::hyper::body::Bytes,), Error = warp::Rejection> + Clone {
    let declared_within_limit = warp::body::content_length_limit(MAX_WEBHOOK_BODY_BYTES);
    let bodiless = warp::header::optional::<String>("content-length")
        .and(warp::header::optional::<String>("transfer-encoding"))
        .and_then(|length: Option<String>, encoding: Option<String>| async move {
            if length.is_none() && encoding.is_none() {
                Ok(())
            } else {
                Err(warp::reject())
            }
        })
        .untuple_one();
    declared_within_limit
        .or(bodiless)
        .unify()
        .and(warp::body::bytes())
}

/// Create Warp routes for webhook handling
/// Returns filters that can be combined with other routes
pub fn create_webhook_routes(
    handler: Arc<WebhookHandler>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    // POST /webhook/:id - Production webhook endpoint
    let post_route = handle_webhook_request(handler.clone());

    // POST /webhook-test/:id - Test mode webhook endpoint
    let test_route = handle_webhook_test(handler.clone());

    post_route.or(test_route)
}

/// POST /webhook/:id - Handle production webhook request
fn handle_webhook_request(
    handler: Arc<WebhookHandler>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("webhook" / String)
        .and(warp::method())
        .and(warp::header::headers_cloned())
        .and(bounded_body())
        .and(warp::addr::remote())
        .and_then(
            move |webhook_id: String,
                  method: warp::http::Method,
                  headers: warp::http::HeaderMap,
                  body: warp::hyper::body::Bytes,
                  remote: Option<std::net::SocketAddr>| {
                let handler = handler.clone();
                async move {
                    // Convert Warp types to platform-agnostic types
                    let request =
                        convert_to_webhook_request(webhook_id, method, headers, body, remote);

                    // Call platform-agnostic handler
                    let response = handler.handle_request(request).await;

                    // Convert response back to Warp reply
                    Ok::<_, warp::Rejection>(convert_to_warp_reply(response))
                }
            },
        )
}

/// POST /webhook-test/:id - Handle test mode webhook request
fn handle_webhook_test(
    handler: Arc<WebhookHandler>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("webhook-test" / String)
        .and(warp::method())
        .and(warp::header::headers_cloned())
        .and(bounded_body())
        .and(warp::addr::remote())
        .and_then(
            move |webhook_id: String,
                  method: warp::http::Method,
                  headers: warp::http::HeaderMap,
                  body: warp::hyper::body::Bytes,
                  remote: Option<std::net::SocketAddr>| {
                let handler = handler.clone();
                async move {
                    // Convert Warp types to platform-agnostic types
                    let request =
                        convert_to_webhook_request(webhook_id, method, headers, body, remote);

                    // Call test mode handler
                    let response = handler.handle_test_request(request).await;

                    // Convert response back to Warp reply
                    Ok::<_, warp::Rejection>(convert_to_warp_reply(response))
                }
            },
        )
}

/// Convert Warp request types to platform-agnostic WebhookRequest
fn convert_to_webhook_request(
    webhook_id: String,
    method: warp::http::Method,
    headers: warp::http::HeaderMap,
    body: warp::hyper::body::Bytes,
    remote: Option<std::net::SocketAddr>,
) -> WebhookRequest {
    // Convert HTTP method
    let http_method = match method {
        warp::http::Method::GET => HttpMethod::GET,
        warp::http::Method::POST => HttpMethod::POST,
        warp::http::Method::PUT => HttpMethod::PUT,
        warp::http::Method::PATCH => HttpMethod::PATCH,
        warp::http::Method::DELETE => HttpMethod::DELETE,
        warp::http::Method::HEAD => HttpMethod::HEAD,
        _ => HttpMethod::POST, // Default to POST for unknown methods
    };

    // Convert headers to HashMap (lowercase keys for consistent lookup)
    let mut header_map = std::collections::HashMap::new();
    for (name, value) in headers.iter() {
        if let Ok(value_str) = value.to_str() {
            header_map.insert(name.as_str().to_lowercase(), value_str.to_string());
        }
    }

    // Extract remote IP
    let remote_ip = remote.map(|addr| addr.ip().to_string());

    WebhookRequest {
        webhook_id,
        method: http_method,
        headers: header_map,
        body: body.to_vec(),
        remote_ip,
    }
}

/// Convert platform-agnostic WebhookResponse to Warp reply
fn convert_to_warp_reply(response: WebhookResponse) -> warp::reply::Response {
    use warp::Reply;

    let status_code = warp::http::StatusCode::from_u16(response.status_code)
        .unwrap_or(warp::http::StatusCode::INTERNAL_SERVER_ERROR);

    // Create JSON response
    let json_reply = warp::reply::json(&response.body);
    let mut http_response = json_reply.into_response();

    // Set status code
    *http_response.status_mut() = status_code;

    // Set content-type to application/json (already set by warp::reply::json, but being explicit)
    http_response.headers_mut().insert(
        warp::http::header::CONTENT_TYPE,
        warp::http::header::HeaderValue::from_static("application/json"),
    );

    // Add custom headers
    for (key, value) in response.headers {
        if let Ok(header_name) = warp::http::header::HeaderName::from_bytes(key.as_bytes()) {
            if let Ok(header_value) = warp::http::header::HeaderValue::from_str(&value) {
                http_response
                    .headers_mut()
                    .insert(header_name, header_value);
            }
        }
    }

    http_response
}

#[cfg(test)]
mod tests {
    use super::create_webhook_routes;
    use diesel::r2d2::{self, ConnectionManager};
    use diesel::SqliteConnection;
    use std::sync::Arc;
    use vortideck_core::services::webhook::{
        WebhookHandler, WebhookService, WebhookStorage, WebhookTestSessionAuthority,
    };

    fn handler() -> Arc<WebhookHandler> {
        let manager = ConnectionManager::<SqliteConnection>::new(":memory:");
        let pool = r2d2::Pool::builder()
            .max_size(1)
            .build(manager)
            .expect("test pool");
        let service = Arc::new(WebhookService::new(Arc::new(WebhookStorage::new(pool))));
        Arc::new(WebhookHandler::new(
            service,
            Arc::new(WebhookTestSessionAuthority::new()),
            None,
            None,
        ))
    }

    #[tokio::test]
    async fn public_http_surface_has_no_management_or_info_routes() {
        let routes = create_webhook_routes(handler());

        let list = warp::test::request()
            .method("GET")
            .path("/webhooks")
            .reply(&routes)
            .await;
        assert_eq!(list.status(), 404);

        let info = warp::test::request()
            .method("GET")
            .path("/webhook/secret-id")
            .reply(&routes)
            .await;
        assert_eq!(info.status(), 404);
    }
}
