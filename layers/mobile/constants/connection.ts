/**
 * Connection-related constants for mobile WebSocket communication
 */

// Tauri Store Keys
export const TAURI_STORE_KEYS = {
  TOKEN: 'token',  // Legacy global token (for backward compatibility)
  TOKEN_FOR_DESKTOP: (desktopId: string) => `token_${desktopId}`,  // Per-desktop tokens
  DESKTOP_ID: 'desktop_id',
  LAST_PAIRED_DESKTOP_ID: 'last_paired_desktop_id',
  LAST_ACTIVE_DESKTOP_ID: 'last_active_desktop_id',  // Last desktop user was connected to
  PAIRED_DESKTOPS: 'paired_desktops',  // Array of all paired desktops
  MOBILE_CLIENT_ID: 'mobile_client_id',
  E2EE_SHARED_SECRET: (desktopId: string) => `e2ee_shared_secret_${desktopId}`,
} as const

// Connection Modes
export const CONNECTION_MODES = {
  DIRECT: 'direct',
  RELAY: 'relay',
} as const

export type ConnectionMode = typeof CONNECTION_MODES[keyof typeof CONNECTION_MODES]

// Connection Statuses
export const CONNECTION_STATUSES = {
  CONNECTED: 'connected',
  CONNECTING: 'connecting',
  DISCONNECTED: 'disconnected',
  PAIRED: 'paired',
  ERROR: 'error',
} as const

export type ConnectionStatus = typeof CONNECTION_STATUSES[keyof typeof CONNECTION_STATUSES]

// WebSocket Message Types (Relay)
export const RELAY_MESSAGE_TYPES = {
  CONNECTION_ESTABLISHED: 'connection_established',
  RELAY_REGISTERED: 'relay_registered',
  AVAILABLE_DESKTOPS: 'available_desktops',
  RELAY_FROM_DESKTOP: 'relay_from_desktop',
  RELAY_MESSAGE: 'relay_message',  // Desktop broadcasts via relay
  CONNECTED_TO_DESKTOP: 'connected_to_desktop',
  DISCONNECTED_FROM_DESKTOP: 'disconnected_from_desktop',
  DESKTOP_DISCONNECTED: 'desktop_disconnected',
  DESKTOP_CONNECTED: 'desktop_connected',  // Desktop reconnected to relay
  ERROR: 'error',
} as const

// WebSocket Message Actions (Direct)
export const MESSAGE_ACTIONS = {
  RELAY_MESSAGE: 'relay_message',
  NEW_TOKEN: 'new_token',
  E2EE_HANDSHAKE_REJECTED: 'e2ee_handshake_rejected',
  E2EE_HANDSHAKE_COMPLETE: 'e2ee_handshake_complete',
  ACTIVE_DECK: 'active_deck',
  DECK_UPDATED: 'deck_updated',
  ALL_DECKS: 'all_decks',
  SERVER_INFO: 'server_info',
  ACTION_RUNTIME_READINESS: 'action_runtime_readiness',
  WORKFLOW_APPROVAL_REQUESTED: 'workflow_approval_requested',
  WORKFLOW_APPROVALS_SNAPSHOT: 'workflow_approvals_snapshot',
  WORKFLOW_APPROVAL_RESOLVED: 'workflow_approval_resolved',
  WORKFLOW_APPROVAL_DECISION_ACK: 'workflow_approval_decision_ack',
  VOLUME_CHANGED: 'volume_changed',
  COMPONENT: 'component',
  DECK_WIDGET_SNAPSHOT: 'deck_widget_snapshot',
  DECK_WIDGET_DELTA: 'deck_widget_delta',
  DECK_WIDGET_STATE: 'deck_widget_state',
  DECK_WIDGET_INTENT_ACK: 'deck_widget_intent_ack',
  DECK_WIDGET_DEFINITIONS: 'deck_widget_definitions',
  DECK_WIDGET_OBJECTS: 'deck_widget_objects',
  DECK_WIDGET_MANIFEST: 'deck_widget_manifest',
} as const

// Connection Settings
export const CONNECTION_SETTINGS = {
  RECONNECT_INTERVAL: 3000, // 3 seconds
  WEBSOCKET_TIMEOUT: 10000, // 10 seconds
} as const

// URL Detection Patterns
export const URL_PATTERNS = {
  RELAY_INDICATORS: ['/relay/', 'wss://'] as const,
} as const
