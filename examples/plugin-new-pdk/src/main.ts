/**
 * Example Plugin using Auto-Generated PDK
 *
 * This demonstrates the clean, type-safe API from our macro-generated PDK.
 * Compare with plugin-xtp which uses manual ArrayBufferLike approach.
 */

import {
  vortideck_log,
  vortideck_register_node,
  vortideck_get_context,
} from './pdk.generated'

/**
 * Execute the plugin with given parameters
 */
export function executeImpl(input: any): any {
  try {
    // ✅ Clean API! Just pass a string, no ArrayBufferLike encoding!
    vortideck_log("Demo Plugin: Starting execution");

    // Get context (auto-parsed JSON)
    const context = vortideck_get_context();
    vortideck_log(`Context: ${context}`);

    // Extract input text
    const inputText = input?.input?.value || input?.text?.value || "";

    // Simple text transformation
    const processedText = inputText.toUpperCase();

    vortideck_log(`Processed: "${inputText}" -> "${processedText}"`);

    return {
      success: true,
      output: processedText,
      outputs: [processedText]
    };

  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    vortideck_log(`Error: ${errorMessage}`);

    return {
      success: false,
      error: errorMessage,
      error_code: "EXECUTION_ERROR"
    };
  }
}

// Manifest embedding - replaced at build time by esbuild
let manifestData: PluginMetadata | null = null;

interface PluginMetadata {
  id: string;
  name: string;
  description: string;
  version: string;
  author: string;
  category: string;
  type_color: string;
  outputs_count: number;
  parameters_schema?: any;
}

function loadManifest(): PluginMetadata {
  if (!manifestData) {
    throw new Error("Manifest data not embedded properly");
  }
  return manifestData;
}

/**
 * Get plugin metadata
 */
export function get_metadataImpl(_input: string): PluginMetadata {
  return loadManifest();
}

/**
 * Register the plugin
 */
export function registerImpl(_input: string): string {
  try {
    vortideck_log("Demo Plugin: Registering...");

    const metadata = get_metadataImpl("");

    // ✅ Clean API! Pass JSON string directly, no buffer encoding!
    const result = vortideck_register_node(JSON.stringify(metadata));

    if (result) {
      vortideck_log("Demo Plugin: Registered successfully!");
      return "Demo Plugin registered";
    } else {
      vortideck_log("Demo Plugin: Registration failed");
      return "Registration failed";
    }

  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    vortideck_log(`Registration error: ${errorMessage}`);
    return `Registration error: ${errorMessage}`;
  }
}
