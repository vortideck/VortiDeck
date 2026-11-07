pub mod client;
pub mod credentials;
pub mod error;
pub mod http_request;
pub mod presets;
pub mod request_builder;
pub mod response;
pub mod saved_request;

// Re-export main types
pub use client::{HttpClient, GLOBAL_HTTP_CLIENT};
pub use error::{ApiError, ApiResult};
pub use http_request::{
    execute_http_request, AuthType, HttpRequestConfig, PaginationConfig, PaginationMode,
    ResponseFormat, RetryConfig,
};
pub use presets::{send_event, send_webhook, send_webhook_async};
pub use request_builder::{delete, get, patch, post, put, RequestBuilder};
pub use response::ApiResponse;
