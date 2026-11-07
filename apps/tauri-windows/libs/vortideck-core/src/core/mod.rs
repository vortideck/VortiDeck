//! Core module for VortiDeck
//!
//! This module contains the core functionality and state management for VortiDeck.

pub mod abstractions;
pub mod performance;
pub mod state;
pub mod state_manager;

pub use abstractions::*;
pub use performance::*;
pub use state::*;
pub use state_manager::{
    AuthState,
    ConnectionCandidate,
    ConnectionStats,
    MessageSender,
    ModuleState,
    ObsIntegrationState,
    ObsListItem,
    ObsParameter,
    // EventEmitter, // Removed - doesn't exist in state_manager
    // OBS Integration types (migrated from shared_state.rs)
    ObsPluginAction,
    PeerInfo,
    PeerStateData,
    PluginPermissionResult,
    PluginPermissions,
    PluginSandboxMode,
    PluginSecurityLevel,
    PublicState,
    SecurityEvent,
    SecurityState,
    SerializablePeerInfo,
    SerializableServerInfo,
    ServerInfo,
    ServerState,
    StateManager,
    UIState,
    WindowInfo,
};
