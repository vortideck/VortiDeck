const hostFunctions = Host.getFunctions()


function isNull(v: any): boolean {
  return v === undefined || v === null
}

function cast(caster: (v: any) => any, v: any): any {
  if (isNull(v)) return v
  return caster(v)
}

function castArray(caster: (v: any) => any) {
  return (v?: Array<any>) => {
    if (isNull(v)) return v
    caster = cast.bind(null, caster) // bind to null-preserving logic in `cast`
    return v!.map(caster)
  }
}

function castMap(caster: (v: any) => any) {
  return (v?: any) => {
    if (isNull(v)) return v

    caster = cast.bind(null, caster) // bind to null-preserving logic in `cast`
    const newMap: any = {}
    for (const k in v) {
      newMap[k] = caster(v![k])
    }
    return newMap
  }
}

function dateToJson(v?: Date): string | undefined | null {
  if (v === undefined || v === null) return v
  return v.toISOString()
}
function dateFromJson(v?: string): Date | undefined | null {
  if (v === undefined || v === null) return v
  return new Date(v)
}

function bufferToJson(v?: ArrayBuffer): string | undefined | null {
  if (v === undefined || v === null) return v
  return Host.arrayBufferToBase64(v)
}
function bufferFromJson(v?: string): ArrayBuffer | undefined | null {
  if (v === undefined || v === null) return v
  return Host.base64ToArrayBuffer(v)
}


  

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
   * Number of output ports for the node
   */
  outputs_count: number;
  
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
  




/**
 * Get the current VortiDeck execution context.
 * Provides access to flow execution state.
 *
 * @returns {ArrayBufferLike} Memory pointer to context data
 */
export function vortideck_get_context() :ArrayBufferLike {
  const ptr = hostFunctions.vortideck_get_context()

      return Memory.find(ptr).readBytes();
  
}

/**
 * Log a message to VortiDeck console for debugging.
 *
 * @param {ArrayBufferLike} input - Memory pointer to message string
 * @returns {ArrayBufferLike} Memory pointer to result
 */
export function vortideck_log(input: ArrayBufferLike) :ArrayBufferLike {
    const mem = Memory.fromBuffer(input)
  
  const ptr = hostFunctions.vortideck_log(mem.offset)

      return Memory.find(ptr).readBytes();
  
}

/**
 * Register a flow node with VortiDeck system.
 * Used during plugin initialization.
 *
 * @param {ArrayBufferLike} input - Memory pointer to JSON metadata
 * @returns {ArrayBufferLike} Memory pointer to result
 */
export function vortideck_register_node(input: ArrayBufferLike) :ArrayBufferLike {
    const mem = Memory.fromBuffer(input)
  
  const ptr = hostFunctions.vortideck_register_node(mem.offset)

      return Memory.find(ptr).readBytes();
  
}

/**
 * Flow Parameter Helper - automatically extracts values from VortiDeck flow format
 * This eliminates the need for parameter transformation in the plugin provider
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

