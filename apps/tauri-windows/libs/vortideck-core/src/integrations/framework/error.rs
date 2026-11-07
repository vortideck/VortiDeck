use thiserror::Error;

#[derive(Debug, Error)]
pub enum IntegrationFrameworkError {
    #[error("Integration framework error: {0}")]
    Framework(String),
    #[error("Integration validation error: {0}")]
    Validation(String),
    #[error("Integration lifecycle error: {0}")]
    Lifecycle(String),
}

pub type IntegrationFrameworkResult<T> = Result<T, IntegrationFrameworkError>; 