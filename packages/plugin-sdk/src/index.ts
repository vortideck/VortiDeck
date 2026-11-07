/**
 * @vortideck/plugin-sdk
 *
 * Stable public entry point. Host wrappers are generated in api.ts and
 * pdk.generated.ts; this file is maintained with the public compatibility
 * contract.
 */

// Export the clean API (recommended)
export { default, log, registerNode, getContext, emitProgress, emitEvent, storageGet, storageSet, readFile, writeFile, httpRequest } from './api'

// Re-export all types and utilities from generated PDK
export type { PluginMetadata, ExecutionResult, ExecutionParams } from './pdk.generated'
export {
  // Utility functions
  safeJsonParse,
  success,
  failure,
  // Flow parameter helper
  FlowParams,
} from './pdk.generated'

// Export helper functions for plugin creation
export { PLUGIN_SCHEMA_VERSION, createPlugin, defineNode } from './helpers'
export type {
  PluginPermission,
  NodeContext,
  NodeDefinition,
  PluginImplementation,
} from './helpers'
