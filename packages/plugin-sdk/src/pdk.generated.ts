/**
 * VortiDeck Plugin Development Kit (PDK)
 * 
 * AUTO-GENERATED from Rust host functions
 * DO NOT EDIT MANUALLY
 * 
 * Regenerate with `pnpm generate:plugin-sdk` from the repository root.
 */

// Memory is a global provided by Extism runtime
declare const Memory: any;

// Host is provided by Extism runtime
declare const Host: {
  getFunctions(): {
    vortideck_log(ptr: number): number
    vortideck_register_node(ptr: number): number
    vortideck_get_context(): number
    vortideck_emit_progress(ptr: number): number
    vortideck_emit_event(ptr: number): number
    vortideck_storage_get(ptr: number): number
    vortideck_storage_set(ptr: number): number
    vortideck_read_file(ptr: number): number
    vortideck_write_file(ptr: number): number
    vortideck_http_request(ptr: number): number
  }
}

// ===== SHARED TYPE INTERFACES =====

/**
 * Parameters passed to the plugin during execution
 */
export interface ExecutionParams {
  // Raw flow parameters with metadata
  [key: string]: any;
}

/**
 * Result returned by the plugin after execution
 */
export interface ExecutionResult {
  /**
   * Whether the execution was successful
   */
  success: boolean;

  /**
   * The main output data
   */
  output?: string;

  /**
   * Array of outputs for multiple output ports
   */
  outputs?: Array<string>;

  /**
   * Error message if execution failed
   */
  error?: string;

  /**
   * Specific error code for programmatic handling
   */
  error_code?: string;
}

/**
 * Plugin metadata for VortiDeck flow system
 *
 * IMPORTANT: This interface MUST stay in sync with PluginInfo struct in:
 * apps/tauri-windows/libs/vortideck-core/src/plugins/metadata.rs
 *
 * TODO: Auto-generate this from Rust struct using serde reflection
 */
export interface PluginMetadata {
  /**
   * Unique plugin identifier
   */
  id: string;

  /**
   * Display name of the plugin
   */
  name: string;

  /**
   * What this plugin does
   */
  description: string;

  /**
   * Plugin category
   */
  category: string;

  /**
   * Hex color for the plugin node
   */
  type_color: string;

  /**
   * JSON Schema defining the plugin's output ports (optional)
   * Same format as MCP tools - defines named outputs
   * Example: { properties: { datetime: { type: "string" }, timezone: { type: "string" } } }
   */
  output_schema?: any;

  /**
   * Plugin version
   */
  version?: string;

  /**
   * Plugin author/creator
   */
  author?: string;

  /**
   * Custom display name for the category (optional)
   */
  category_display_name?: string;
  /** Optional second-level group shown inside the category. */
  subcategory?: string;

  /**
   * JSON schema defining the plugin's input parameters
   */
  parameters_schema?: any;

  /**
   * Tags for plugin discovery
   */
  tags?: Array<string>;
}

const hostFunctions = Host.getFunctions();

// ===== HOST FUNCTION WRAPPERS =====
// Type-safe wrappers around raw host functions

/**
 * vortideck_log
 * @param {string} message - Parameter message
 * @returns {string} Result from host function
 */
export function vortideck_log(message: string): string {
  try {
    const params = {
      message,
    };
    const paramsJson = JSON.stringify(params);
    const paramsMemory = Memory.fromString(paramsJson);
    const resultPtr = hostFunctions.vortideck_log(paramsMemory.offset);
    const ptr = resultPtr;
    const resultMemory = Memory.find(ptr);
    const resultJson = resultMemory.readString();
    return JSON.parse(resultJson);
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    throw new Error(`VortiDeck PDK Error [vortideck_log]: ${errorMessage}`);
  }
}

/**
 * vortideck_register_node
 * @param {string} metadata - Parameter metadata
 * @returns {boolean} Result from host function
 */
export function vortideck_register_node(metadata: string): boolean {
  try {
    const params = {
      metadata,
    };
    const paramsJson = JSON.stringify(params);
    const paramsMemory = Memory.fromString(paramsJson);
    const resultPtr = hostFunctions.vortideck_register_node(paramsMemory.offset);
    const ptr = resultPtr;
    const resultMemory = Memory.find(ptr);
    const resultJson = resultMemory.readString();
    return JSON.parse(resultJson);
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    throw new Error(`VortiDeck PDK Error [vortideck_register_node]: ${errorMessage}`);
  }
}

/**
 * vortideck_get_context
 * @returns {string} Result from host function
 */
export function vortideck_get_context(): string {
  try {
    const ptr = hostFunctions.vortideck_get_context();
    const resultMemory = Memory.find(ptr);
    const resultJson = resultMemory.readString();
    return JSON.parse(resultJson);
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    throw new Error(`VortiDeck PDK Error [vortideck_get_context]: ${errorMessage}`);
  }
}

/**
 * vortideck_emit_progress
 * @param {string} progress_json - Parameter progress_json
 * @returns {boolean} Result from host function
 */
export function vortideck_emit_progress(progress_json: string): boolean {
  try {
    const params = {
      progress_json,
    };
    const paramsJson = JSON.stringify(params);
    const paramsMemory = Memory.fromString(paramsJson);
    const resultPtr = hostFunctions.vortideck_emit_progress(paramsMemory.offset);
    const ptr = resultPtr;
    const resultMemory = Memory.find(ptr);
    const resultJson = resultMemory.readString();
    return JSON.parse(resultJson);
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    throw new Error(`VortiDeck PDK Error [vortideck_emit_progress]: ${errorMessage}`);
  }
}

/**
 * vortideck_emit_event
 * @param {string} event_json - Parameter event_json
 * @returns {boolean} Result from host function
 */
export function vortideck_emit_event(event_json: string): boolean {
  try {
    const params = {
      event_json,
    };
    const paramsJson = JSON.stringify(params);
    const paramsMemory = Memory.fromString(paramsJson);
    const resultPtr = hostFunctions.vortideck_emit_event(paramsMemory.offset);
    const ptr = resultPtr;
    const resultMemory = Memory.find(ptr);
    const resultJson = resultMemory.readString();
    return JSON.parse(resultJson);
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    throw new Error(`VortiDeck PDK Error [vortideck_emit_event]: ${errorMessage}`);
  }
}

/**
 * vortideck_storage_get
 * @param {string} key - Parameter key
 * @returns {string} Result from host function
 */
export function vortideck_storage_get(key: string): string {
  try {
    const params = {
      key,
    };
    const paramsJson = JSON.stringify(params);
    const paramsMemory = Memory.fromString(paramsJson);
    const resultPtr = hostFunctions.vortideck_storage_get(paramsMemory.offset);
    const ptr = resultPtr;
    const resultMemory = Memory.find(ptr);
    const resultJson = resultMemory.readString();
    return JSON.parse(resultJson);
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    throw new Error(`VortiDeck PDK Error [vortideck_storage_get]: ${errorMessage}`);
  }
}

/**
 * vortideck_storage_set
 * @param {string} key - Parameter key
 * @param {string} value - Parameter value
 * @returns {boolean} Result from host function
 */
export function vortideck_storage_set(key: string, value: string): boolean {
  try {
    const params = {
      key,
      value,
    };
    const paramsJson = JSON.stringify(params);
    const paramsMemory = Memory.fromString(paramsJson);
    const resultPtr = hostFunctions.vortideck_storage_set(paramsMemory.offset);
    const ptr = resultPtr;
    const resultMemory = Memory.find(ptr);
    const resultJson = resultMemory.readString();
    return JSON.parse(resultJson);
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    throw new Error(`VortiDeck PDK Error [vortideck_storage_set]: ${errorMessage}`);
  }
}

/**
 * vortideck_read_file
 * @param {string} path - Parameter path
 * @returns {string} Result from host function
 */
export function vortideck_read_file(path: string): string {
  try {
    const params = {
      path,
    };
    const paramsJson = JSON.stringify(params);
    const paramsMemory = Memory.fromString(paramsJson);
    const resultPtr = hostFunctions.vortideck_read_file(paramsMemory.offset);
    const ptr = resultPtr;
    const resultMemory = Memory.find(ptr);
    const resultJson = resultMemory.readString();
    return JSON.parse(resultJson);
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    throw new Error(`VortiDeck PDK Error [vortideck_read_file]: ${errorMessage}`);
  }
}

/**
 * vortideck_write_file
 * @param {string} path - Parameter path
 * @param {string} content - Parameter content
 * @returns {boolean} Result from host function
 */
export function vortideck_write_file(path: string, content: string): boolean {
  try {
    const params = {
      path,
      content,
    };
    const paramsJson = JSON.stringify(params);
    const paramsMemory = Memory.fromString(paramsJson);
    const resultPtr = hostFunctions.vortideck_write_file(paramsMemory.offset);
    const ptr = resultPtr;
    const resultMemory = Memory.find(ptr);
    const resultJson = resultMemory.readString();
    return JSON.parse(resultJson);
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    throw new Error(`VortiDeck PDK Error [vortideck_write_file]: ${errorMessage}`);
  }
}

/**
 * vortideck_http_request
 * @param {string} options_json - Parameter options_json
 * @returns {string} Result from host function
 */
export function vortideck_http_request(options_json: string): string {
  try {
    const params = {
      options_json,
    };
    const paramsJson = JSON.stringify(params);
    const paramsMemory = Memory.fromString(paramsJson);
    const resultPtr = hostFunctions.vortideck_http_request(paramsMemory.offset);
    const ptr = resultPtr;
    const resultMemory = Memory.find(ptr);
    const resultJson = resultMemory.readString();
    return JSON.parse(resultJson);
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    throw new Error(`VortiDeck PDK Error [vortideck_http_request]: ${errorMessage}`);
  }
}

// ===== FLOW PARAMETER HELPER =====

/**
 * Flow Parameter Helper - automatically extracts values from VortiDeck flow format
 * This eliminates the need for parameter transformation in the plugin provider
 *
 * @example
 * ```typescript
 * const params = new FlowParams(input);
 * const text = params.getString('text', 'default');
 * const count = params.getNumber('count', 10);
 * const enabled = params.getBoolean('enabled', false);
 * ```
 */
export class FlowParams {
  private params: any;

  constructor(params: any) {
    this.params = params || {};
  }

  /**
   * Get a string value from flow parameters
   * Handles both flow format {value: "text", type: "string", attached: false} and direct values
   */
  getString(key: string, defaultValue: string = ""): string {
    const param = this.params[key];
    if (!param) return defaultValue;

    // Handle flow format with metadata
    if (typeof param === 'object' && param.value !== undefined) {
      return String(param.value || defaultValue);
    }

    // Handle direct value
    return String(param || defaultValue);
  }

  /**
   * Get a boolean value from flow parameters
   */
  getBoolean(key: string, defaultValue: boolean = false): boolean {
    const param = this.params[key];
    if (!param) return defaultValue;

    // Handle flow format with metadata
    if (typeof param === 'object' && param.value !== undefined) {
      const value = param.value;
      if (typeof value === 'boolean') return value;
      if (typeof value === 'string') return value.toLowerCase() === 'true';
      return Boolean(value);
    }

    // Handle direct value
    if (typeof param === 'boolean') return param;
    if (typeof param === 'string') return param.toLowerCase() === 'true';
    return Boolean(param);
  }

  /**
   * Get a number value from flow parameters
   */
  getNumber(key: string, defaultValue: number = 0): number {
    const param = this.params[key];
    if (!param) return defaultValue;

    // Handle flow format with metadata
    if (typeof param === 'object' && param.value !== undefined) {
      const value = param.value;
      const num = Number(value);
      return isNaN(num) ? defaultValue : num;
    }

    // Handle direct value
    const num = Number(param);
    return isNaN(num) ? defaultValue : num;
  }

  /**
   * Get the main input text - looks for 'input', 'text', 'inputText' in that order
   */
  getInput(defaultValue: string = ""): string {
    return this.getString('input') ||
           this.getString('text') ||
           this.getString('inputText') ||
           defaultValue;
  }

  /**
   * Check if a parameter is attached (connected to another node's output)
   */
  isAttached(key: string): boolean {
    const param = this.params[key];
    if (!param) return false;

    // Handle flow format with metadata
    if (typeof param === 'object' && param.attached !== undefined) {
      return Boolean(param.attached);
    }

    return false;
  }

  /**
   * Get all parameter keys
   */
  getKeys(): string[] {
    return Object.keys(this.params);
  }

  /**
   * Get raw parameter object for a key (useful for debugging)
   */
  getRaw(key: string): any {
    return this.params[key];
  }

  /**
   * Get all parameters as a simple key-value object (values extracted from flow format)
   */
  toSimpleObject(): Record<string, any> {
    const result: Record<string, any> = {};
    for (const key of this.getKeys()) {
      const param = this.params[key];
      if (typeof param === 'object' && param.value !== undefined) {
        result[key] = param.value;
      } else {
        result[key] = param;
      }
    }
    return result;
  }
}

// ===== HELPER UTILITIES =====

/**
 * Create a successful execution result
 */
export function success(output: string, outputs?: Array<string>): ExecutionResult {
  return {
    success: true,
    output,
    outputs: outputs || [output],
  };
}

/**
 * Create a failed execution result
 */
export function failure(error: string, error_code?: string): ExecutionResult {
  return {
    success: false,
    error,
    error_code: error_code || 'EXECUTION_ERROR',
  };
}

/**
 * Safe JSON parse that returns default value on error
 */
export function safeJsonParse<T>(json: string, defaultValue: T): T {
  try {
    return JSON.parse(json);
  } catch {
    return defaultValue;
  }
}

