//! Integration system for VortiDeck
//!
//! This module provides the core integration system, allowing for both built-in
//! and plugin-based integrations with external services.

pub mod builtin;
pub mod contracts;
pub mod framework;
pub mod interface;
pub mod traits;
pub mod universal_manager;

pub use contracts::{
    ActionContract, ActionPreflight, ActionPreflightIssue, ActionPreflightStatus,
    ActionReceiptSemantics, ActionRecoverySemantics, CapabilityDefinition, CapabilityKind,
    CapabilityOption, CapabilityOutput, CapabilityParameter, CapabilityPresentation,
    CapabilityWidget, ContractValueType, DynamicOptionSource,
};
pub use interface::{
    EventCallback, IntegrationAction, IntegrationActionExecution, IntegrationActionResult,
    IntegrationActionStatus, IntegrationId, IntegrationMetadata, IntegrationRequirements,
    IntegrationResult,
};

pub use traits::{AppIntegration, IntegrationError};

pub use framework::{
    IntegrationDiscovery, IntegrationFrameworkError, IntegrationFrameworkResult,
    IntegrationLifecycleManager, IntegrationManager, IntegrationRegistry, IntegrationValidator,
};

pub use universal_manager::{
    ConnectionStatus, EventEmitter, IntegrationStatus, IntegrationStatusMonitor,
    UniversalIntegrationManager,
};
