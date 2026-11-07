use super::*;
use crate::integrations::builtin::obs::{ObsIntegration, ObsState};
use crate::integrations::interface::{IntegrationMetadata, IntegrationRequirements};
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::test]
async fn test_registry_initialization() {
    let registry = IntegrationRegistry::new();
    let metadata = registry.list_integrations().await;
    assert!(metadata.is_empty());
}
#[tokio::test]
async fn test_register_native_integration() {
    let registry = Arc::new(IntegrationRegistry::new());
    let integration = Arc::new(ObsIntegration::new());
    let provider = IntegrationProvider::Native(integration);

    registry
        .register_native_integration("obs".to_string(), provider)
        .await
        .unwrap();

    let metadata = registry.list_integrations().await;
    assert!(metadata.contains_key("obs"));

    let obs_metadata = metadata.get("obs").unwrap();
    assert_eq!(obs_metadata.name, "OBS Studio");
    assert_eq!(obs_metadata.version, "1.0.0");
}

#[tokio::test]
async fn test_register_plugin_integration() {
    let registry = Arc::new(IntegrationRegistry::new());
    let metadata = IntegrationMetadata {
        name: "Test Plugin".to_string(),
        version: "1.0.0".to_string(),
        description: "A test plugin".to_string(),
    };
    let requirements = IntegrationRequirements {
        premium_features: vec![],
        context: "Test context".to_string(),
    };

    let provider = IntegrationProvider::Plugin {
        metadata: metadata.clone(),
        requirements: requirements.clone(),
    };

    registry
        .register_plugin_integration("test_plugin".to_string(), provider)
        .await
        .unwrap();

    let integrations = registry.list_integrations().await;
    assert!(integrations.contains_key("test_plugin"));

    let plugin_metadata = integrations.get("test_plugin").unwrap();
    assert_eq!(plugin_metadata.name, metadata.name);
    assert_eq!(plugin_metadata.version, metadata.version);
}

#[tokio::test]
async fn test_unregister_integration() {
    let registry = Arc::new(IntegrationRegistry::new());
    let integration = Arc::new(ObsIntegration::new());
    let provider = IntegrationProvider::Native(integration);

    registry
        .register_native_integration("obs".to_string(), provider)
        .await
        .unwrap();

    let metadata = registry.list_integrations().await;
    assert!(metadata.contains_key("obs"));

    registry
        .unregister_integration("obs".to_string())
        .await
        .unwrap();

    let metadata = registry.list_integrations().await;
    assert!(!metadata.contains_key("obs"));
}

#[tokio::test]
async fn test_get_integration() {
    let registry = Arc::new(IntegrationRegistry::new());
    let integration = Arc::new(ObsIntegration::new());
    let provider = IntegrationProvider::Native(integration);

    registry
        .register_native_integration("obs".to_string(), provider)
        .await
        .unwrap();

    let provider = registry.get_integration("obs").await;
    assert!(provider.is_some());

    match provider.unwrap() {
        IntegrationProvider::Native(integration) => {
            assert_eq!(integration.metadata().name, "OBS Studio");
        }
        _ => panic!("Expected native integration"),
    }
}
