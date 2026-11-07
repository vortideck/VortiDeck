/**
 * TypeScript PDK Generator
 *
 * Generates type-safe TypeScript wrappers for Extism plugins
 */
use super::{HostFunction, PdkGenerator};

pub struct TypeScriptGenerator;

impl PdkGenerator for TypeScriptGenerator {
    fn generate(&self, functions: &[HostFunction]) -> String {
        let mut output = String::new();

        // Header
        output.push_str("/**\n");
        output.push_str(" * VortiDeck Plugin Development Kit (PDK)\n");
        output.push_str(" * \n");
        output.push_str(" * AUTO-GENERATED from Rust host functions\n");
        output.push_str(" * DO NOT EDIT MANUALLY\n");
        output.push_str(" * \n");
        output
            .push_str(" * Regenerate with `pnpm generate:plugin-sdk` from the repository root.\n");
        output.push_str(" */\n\n");

        // Memory declaration
        output.push_str("// Memory is a global provided by Extism runtime\n");
        output.push_str("declare const Memory: any;\n\n");

        // Host declaration with function types
        output.push_str("// Host is provided by Extism runtime\n");
        output.push_str("declare const Host: {\n");
        output.push_str("  getFunctions(): {\n");
        for func in functions {
            let signature = if func.params.is_empty() {
                format!("    {}(): number\n", func.name)
            } else {
                format!("    {}(ptr: number): number\n", func.name)
            };
            output.push_str(&signature);
        }
        output.push_str("  }\n");
        output.push_str("}\n\n");

        // Shared Type Interfaces
        output.push_str(&generate_shared_types());

        // Use Host.getFunctions() pattern to avoid Wizer initialization issues
        output.push_str("const hostFunctions = Host.getFunctions();\n\n");

        // Generate wrapper functions with error handling
        output.push_str("// ===== HOST FUNCTION WRAPPERS =====\n");
        output.push_str("// Type-safe wrappers around raw host functions\n\n");

        for func in functions {
            output.push_str(&generate_function_wrapper(func));
        }

        // Add FlowParams helper class
        output.push_str(&generate_flow_params_helper());

        // Add helper utilities
        output.push_str(&generate_helper_utilities());

        output
    }

    fn file_extension(&self) -> &str {
        "ts"
    }

    fn language_name(&self) -> &str {
        "TypeScript"
    }

    fn generate_type_declarations(&self, functions: &[HostFunction]) -> Option<String> {
        let mut output = String::new();

        // Header comment
        output.push_str("/**\n");
        output.push_str(" * Extism Plugin Type Definitions\n");
        output.push_str(" *\n");
        output.push_str(" * AUTO-GENERATED from Rust host functions\n");
        output.push_str(" * DO NOT EDIT MANUALLY\n");
        output.push_str(" *\n");
        output.push_str(" * Note: Use Extism integer types (I32, I64) for compatibility with extism-js compiler\n");
        output.push_str(" */\n\n");

        // Plugin exports module (static - always the same)
        output.push_str("declare module 'main' {\n");
        output.push_str("  export function execute(): I32;\n");
        output.push_str("  export function get_metadata(): I32;\n");
        output.push_str("  export function register(): I32;\n");
        output.push_str("}\n\n");

        // Host functions module (dynamic - generated from Rust)
        output.push_str("declare module 'extism:host' {\n");
        output.push_str("  interface user {\n");
        for func in functions {
            // Functions with parameters take a ptr, functions without parameters take nothing
            let signature = if func.params.is_empty() {
                format!("    {}(): I64;\n", func.name)
            } else {
                format!("    {}(ptr: I64): I64;\n", func.name)
            };
            output.push_str(&signature);
        }
        output.push_str("  }\n");
        output.push_str("}\n");

        Some(output)
    }

    fn generate_sdk_wrapper(&self, functions: &[HostFunction]) -> Option<String> {
        let mut output = String::new();

        // Header
        output.push_str("/**\n");
        output.push_str(" * VortiDeck Plugin SDK - Clean API\n");
        output.push_str(" *\n");
        output.push_str(" * AUTO-GENERATED from Rust host functions\n");
        output.push_str(" * DO NOT EDIT MANUALLY\n");
        output.push_str(" *\n");
        output.push_str(
            " * This file provides a developer-friendly wrapper around the auto-generated PDK.\n",
        );
        output.push_str(" */\n\n");

        // Import from generated PDK
        output.push_str("import {\n");
        for func in functions {
            output.push_str(&format!("  {},\n", func.name));
        }
        output.push_str("} from './pdk.generated'\n\n");

        // Generate clean function exports
        for func in functions {
            let clean_name = to_camel_case(&func.name.trim_start_matches("vortideck_"));

            // JSDoc
            output.push_str("/**\n");
            if !func.description.is_empty() {
                output.push_str(&format!(
                    " * {}\n",
                    func.description.lines().next().unwrap_or(&func.name)
                ));
            } else {
                output.push_str(&format!(" * {}\n", clean_name));
            }

            // Add parameter docs
            for param in &func.params {
                let ts_type = param.to_language_type("typescript");
                output.push_str(&format!(" * @param {} - {}\n", param.name, ts_type));
            }

            let return_type_ts = rust_to_typescript(&func.return_type);
            output.push_str(&format!(" * @returns {}\n", return_type_ts));
            output.push_str(" */\n");

            output.push_str(&format!("export const {} = {}\n\n", clean_name, func.name));
        }

        // Default export namespace
        output.push_str("/**\n");
        output.push_str(" * Default export - namespace object for clean API\n");
        output.push_str(" *\n");
        output.push_str(" * @example\n");
        output.push_str(" * ```typescript\n");
        output.push_str(" * import vortideck from '@vortideck/plugin-sdk'\n");
        output.push_str(" *\n");
        if !functions.is_empty() {
            let first_func = &functions[0];
            let clean_name = to_camel_case(&first_func.name.trim_start_matches("vortideck_"));
            output.push_str(&format!(" * vortideck.{}()\n", clean_name));
        }
        output.push_str(" * ```\n");
        output.push_str(" */\n");
        output.push_str("const vortideck = {\n");
        for func in functions {
            let clean_name = to_camel_case(&func.name.trim_start_matches("vortideck_"));
            output.push_str(&format!("  {},\n", clean_name));
        }
        output.push_str("}\n\n");
        output.push_str("export default vortideck\n");

        Some(output)
    }
}

/// Convert snake_case to camelCase
fn to_camel_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;

    for c in s.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }

    result
}

fn generate_function_wrapper(func: &HostFunction) -> String {
    let mut output = String::new();

    // Multi-line JSDoc comment
    output.push_str("/**\n");
    if !func.description.is_empty() {
        for line in func.description.lines() {
            output.push_str(&format!(" * {}\n", line.trim()));
        }
    } else {
        output.push_str(&format!(" * {}\n", func.name));
    }

    // Add parameter docs
    for param in &func.params {
        let ts_type = param.to_language_type("typescript");
        output.push_str(&format!(
            " * @param {{{}}} {} - Parameter {}\n",
            ts_type, param.name, param.name
        ));
    }

    let return_type_ts = rust_to_typescript(&func.return_type);
    output.push_str(&format!(
        " * @returns {{{}}} Result from host function\n",
        return_type_ts
    ));
    output.push_str(" */\n");

    // Function signature
    let params: Vec<String> = func
        .params
        .iter()
        .map(|p| format!("{}: {}", p.name, p.to_language_type("typescript")))
        .collect();

    output.push_str(&format!(
        "export function {}({}): {} {{\n",
        func.name,
        params.join(", "),
        return_type_ts
    ));

    // Add try-catch wrapper
    output.push_str("  try {\n");

    // Function body
    if func.params.is_empty() {
        // No parameters - call without arguments
        output.push_str(&format!("    const ptr = hostFunctions.{}();\n", func.name));
    } else {
        // Serialize parameters to JSON
        output.push_str("    const params = {\n");
        for param in &func.params {
            output.push_str(&format!("      {},\n", param.name));
        }
        output.push_str("    };\n");
        output.push_str("    const paramsJson = JSON.stringify(params);\n");
        output.push_str("    const paramsMemory = Memory.fromString(paramsJson);\n");
        output.push_str(&format!(
            "    const resultPtr = hostFunctions.{}(paramsMemory.offset);\n",
            func.name
        ));
        output.push_str("    const ptr = resultPtr;\n");
    }

    // Deserialize result
    if return_type_ts != "void" {
        output.push_str("    const resultMemory = Memory.find(ptr);\n");
        output.push_str("    const resultJson = resultMemory.readString();\n");
        output.push_str("    return JSON.parse(resultJson);\n");
    }

    // Error handling
    output.push_str("  } catch (error) {\n");
    output.push_str(
        "    const errorMessage = error instanceof Error ? error.message : String(error);\n",
    );
    output.push_str(&format!(
        "    throw new Error(`VortiDeck PDK Error [{}]: ${{errorMessage}}`);\n",
        func.name
    ));
    output.push_str("  }\n");

    output.push_str("}\n\n");

    output
}

fn rust_to_typescript(rust_type: &str) -> String {
    match rust_type {
        "String" | "&str" => "string".to_string(),
        "bool" => "boolean".to_string(),
        "i32" | "i64" | "u32" | "u64" | "f32" | "f64" | "usize" => "number".to_string(),
        "()" => "void".to_string(),
        s if s.starts_with("Vec<") => {
            let inner = s.trim_start_matches("Vec<").trim_end_matches('>');
            format!("{}[]", rust_to_typescript(inner))
        }
        s if s.starts_with("Option<") => {
            let inner = s.trim_start_matches("Option<").trim_end_matches('>');
            format!("{} | null", rust_to_typescript(inner))
        }
        _ => "any".to_string(),
    }
}

fn generate_shared_types() -> String {
    r#"// ===== SHARED TYPE INTERFACES =====

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

"#
    .to_string()
}

fn generate_flow_params_helper() -> String {
    r#"// ===== FLOW PARAMETER HELPER =====

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

"#
    .to_string()
}

fn generate_helper_utilities() -> String {
    r#"// ===== HELPER UTILITIES =====

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

"#
    .to_string()
}
