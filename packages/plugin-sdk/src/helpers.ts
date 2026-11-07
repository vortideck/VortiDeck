/**
 * VortiDeck Plugin SDK - Helper Functions
 *
 * These helpers eliminate boilerplate and provide the best DX
 */

// Host is provided by Extism runtime
declare const Host: {
  inputString(): string
  outputString(s: string): void
}

import type { PluginMetadata, ExecutionResult } from './pdk.generated'
import {
  vortideck_emit_progress,
  vortideck_emit_event,
  vortideck_storage_get,
  vortideck_storage_set
} from './pdk.generated'

/**
 * Current VortiDeck Plugin API schema version
 *
 * This tracks the plugin API/SDK interface version, not your plugin version.
 * Breaking changes increment major version (e.g., 1.0 -> 2.0)
 */
export const PLUGIN_SCHEMA_VERSION = '1.0'

/**
 * Plugin permission types matching Rust PluginPermission enum
 */
export type PluginPermission =
  | 'dom'
  | 'css'
  | 'network'
  | 'filesystem'
  | 'executecommands'
  | 'accesscamera'
  | 'accessmicrophone'
  | 'flows'
  | 'storage'

/**
 * Execution context provided to node execute functions
 */
export interface NodeContext {
  /** Emit progress update (0.0 to 1.0) */
  emitProgress(progress: number, message?: string): void

  /** Emit custom event */
  emitEvent(eventType: string, data: any): void

  /**
   * Get plugin-scoped storage value (synchronous)
   * Note: Extism does not support async/await in plugins due to WASM limitations
   */
  getStorage(key: string): string

  /**
   * Set plugin-scoped storage value (synchronous)
   * Note: Extism does not support async/await in plugins due to WASM limitations
   */
  setStorage(key: string, value: string): void

  /** Current execution context */
  flowId: string
  nodeId: string
  executionId: string
}

/**
 * Single node definition
 */
export interface NodeDefinition {
  /** Node metadata */
  metadata: PluginMetadata

  /** Execute function - main node logic (sync or async) */
  execute: (input: any, context: NodeContext) => ExecutionResult | any | Promise<ExecutionResult | any>

  /** Optional custom register logic */
  onRegister?: () => void
}

/**
 * Plugin implementation interface (supports multiple nodes)
 * Note: Plugin metadata (id, name, version, author, permissions) comes from manifest.json
 * Plugins only define node implementations here
 */
export interface PluginImplementation {
  /** Map of node definitions by node ID */
  nodes: Record<string, NodeDefinition>

  /** Optional plugin-level initialization */
  onPluginRegister?: () => void
}

/**
 * Create Extism plugin exports from your implementation
 *
 * This handles ALL the Host boilerplate automatically!
 *
 * @example
 * ```typescript
 * // main.ts - Multi-node plugin with async support
 * import { createPlugin, defineNode, FlowParams } from '@vortideck/plugin-sdk'
 *
 * export const plugin = createPlugin({
 *   id: "text-processor",
 *   name: "Text Processor",
 *   version: "1.0.0",
 *   nodes: {
 *     uppercase: defineNode({
 *       metadata: {
 *         id: "text-processor:uppercase",
 *         name: "To Uppercase",
 *         description: "Convert text to uppercase",
 *         category: "text",
 *         type_color: "#4A90E2",
 *         outputs_count: 1
 *       },
 *       async execute(input, context) {
 *         context.emitProgress(0.3, "Processing...")
 *         const params = new FlowParams(input)
 *         const text = params.getInput()
 *
 *         context.emitProgress(0.7, "Almost done...")
 *         const result = text.toUpperCase()
 *
 *         context.emitProgress(1.0, "Complete!")
 *         return { success: true, output: result }
 *       }
 *     }),
 *     lowercase: defineNode({
 *       metadata: {
 *         id: "text-processor:lowercase",
 *         name: "To Lowercase",
 *         description: "Convert text to lowercase",
 *         category: "text",
 *         type_color: "#4A90E2",
 *         outputs_count: 1
 *       },
 *       execute(input, context) {
 *         const params = new FlowParams(input)
 *         return { success: true, output: params.getInput().toLowerCase() }
 *       }
 *     })
 *   }
 * })
 *
 * // Export all node functions for Extism
 * export const { execute, register, get_metadata } = plugin
 * ```
 */
export function createPlugin(impl: PluginImplementation): {
  execute(): number
  register(): number
  get_metadata(): number
} {
  // Store which node is currently executing
  let currentNodeId: string | null = null

  // Create execution context
  function createContext(flowId: string, nodeId: string, executionId: string): NodeContext {
    return {
      emitProgress(progress: number, message?: string) {
        vortideck_emit_progress(JSON.stringify({ progress, message, nodeId, executionId }))
      },
      emitEvent(eventType: string, data: any) {
        vortideck_emit_event(JSON.stringify({ eventType, data, nodeId, executionId }))
      },
      getStorage(key: string): string {
        return vortideck_storage_get(key)
      },
      setStorage(key: string, value: string): void {
        vortideck_storage_set(key, value)
      },
      flowId,
      nodeId,
      executionId
    }
  }

  return {
    /**
     * Execute function export for Extism
     * Handles routing to the correct node and async execution
     */
    execute(): number {
      try {
        const input = JSON.parse(Host.inputString())

        // Extract node ID from input
        // Flow execution: input.__node_id
        // Web extension: input.params?.node
        const nodeId = input.__node_id || input.params?.node || currentNodeId
        if (!nodeId) {
          throw new Error("No node ID specified for execution")
        }

        const node = impl.nodes[nodeId]
        if (!node) {
          throw new Error(`Node '${nodeId}' not found in plugin`)
        }

        // Detect calling context
        const isWebExtension = !!input.params?.node

        // Create execution context
        const context = createContext(
          input.__flow_id || "web-extension",
          nodeId,
          input.__execution_id || "web-extension"
        )

        // Prepare input for node's execute function
        // Web extension: pass input.inputs (the actual data)
        // Flow: pass full input object (contains __node_id, params, etc.)
        const nodeInput = isWebExtension ? input.inputs : input

        // Execute node (handle both sync and async)
        const result = node.execute(nodeInput, context)

        // Check if result is a promise (async function)
        if (result && typeof result.then === 'function') {
          // For async, we need to block and wait
          // This is a limitation of Extism's synchronous FFI
          throw new Error("Async execution not yet supported - use synchronous execute functions for now")
        }

        Host.outputString(JSON.stringify(result))
        return 0
      } catch (error) {
        const errorResult = {
          success: false,
          error: error instanceof Error ? error.message : String(error),
          error_code: 'EXECUTION_ERROR'
        }
        Host.outputString(JSON.stringify(errorResult))
        return 1
      }
    },


    /**
     * Register function export for Extism
     * Registers all nodes in this plugin
     */
    register(): number {
      try {
        // Call plugin-level initialization
        if (impl.onPluginRegister) {
          impl.onPluginRegister()
        }

        // Register each node
        for (const [nodeId, node] of Object.entries(impl.nodes)) {
          currentNodeId = nodeId

          if (node.onRegister) {
            node.onRegister()
          }
        }

        currentNodeId = null
        Host.outputString(`Plugin registered with ${Object.keys(impl.nodes).length} nodes`)
        return 0
      } catch (error) {
        Host.outputString(`Registration error: ${error}`)
        return 1
      }
    },

    /**
     * Get metadata function export for Extism
     * Returns metadata for all nodes in this plugin (multi-node format)
     */
    get_metadata(): number {
      try {
        // Build nodes array with internal node IDs
        const nodes = Object.entries(impl.nodes).map(([internalNodeId, node]) => ({
          ...node.metadata,
          _node_id: internalNodeId // Internal routing ID
        }))

        // Return new multi-node format: { plugin: {...}, nodes: [...] }
        // Plugin-level metadata comes from manifest.json, so we only return nodes here
        const response = {
          plugin: {
            // These will be overridden by manifest.json values
            id: "plugin-id-from-manifest",
            name: "Plugin Name from Manifest",
            version: "1.0.0",
            author: "Author from Manifest"
          },
          nodes: nodes
        }

        Host.outputString(JSON.stringify(response))
        return 0
      } catch (error) {
        Host.outputString(`Metadata error: ${error}`)
        return 1
      }
    }
  }
}

/**
 * Helper to define a single node
 * Makes the API cleaner and provides type safety
 */
export function defineNode(definition: NodeDefinition): NodeDefinition {
  return definition
}
