use super::interface::{PluginAction, PluginError, PluginMetadata};
use async_trait::async_trait;

/// Plugin trait for async operations
#[async_trait]
pub trait AsyncPlugin: Send + Sync {
    async fn initialize(&self) -> Result<(), PluginError>;
    async fn execute(&self, action: PluginAction) -> Result<(), PluginError>;
    async fn shutdown(&self) -> Result<(), PluginError>;
    fn metadata(&self) -> PluginMetadata;
}
