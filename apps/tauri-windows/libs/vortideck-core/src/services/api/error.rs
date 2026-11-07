use thiserror::Error;

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("HTTP error: {0}")]
    HttpError(String),

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Request timeout")]
    Timeout,

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Invalid method: {0}")]
    InvalidMethod(String),
}
