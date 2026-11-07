import {
      ExecutionParams,
      ExecutionResult,
      PluginMetadata,
      FlowParams,
  } from './pdk'

import {
      vortideck_get_context,
      vortideck_log,
      vortideck_register_node,
  } from './pdk'

// Helper functions to work with Extism host functions
function logMessage(message: string): void {
  try {
    // The generated host function expects ArrayBufferLike, so create from string
    const encoder = new TextEncoder();
    const buffer = encoder.encode(message);
    vortideck_log(buffer.buffer);
  } catch (error) {
    // Fallback if logging fails
    console.log(message);
  }
}

function registerNode(metadata: any): boolean {
  try {
    // Convert metadata to JSON and pass as ArrayBufferLike
    const jsonString = JSON.stringify(metadata);
    const encoder = new TextEncoder();
    const buffer = encoder.encode(jsonString);
    const result = vortideck_register_node(buffer.buffer);
    return true; // Assume success if no error thrown
  } catch (error) {
    console.log("Failed to register node:", error);
    return false;
  }
}

/**
 * Execute the plugin with given parameters.
 * Main plugin logic implementation.
 *
 * @param {ExecutionParams} input - Raw flow parameters with metadata
 * @returns {ExecutionResult} 
 */
export function executeImpl(input: ExecutionParams): ExecutionResult {
  try {
    // Log the execution start
    logMessage("Text Processor Plugin: Starting execution");
    
    // 🔧 NEW: Use FlowParams helper to handle flow format directly
    const flowParams = new FlowParams(input);
    
    // Extract input text using the helper
    const inputText = flowParams.getInput("");
    
    // Extract configuration options using type-safe helpers
    const options = {
      uppercase: flowParams.getBoolean('uppercase', false),
      lowercase: flowParams.getBoolean('lowercase', false),
      reverse: flowParams.getBoolean('reverse', false),
      removeSpaces: flowParams.getBoolean('removeSpaces', false),
      addPrefix: flowParams.getString('addPrefix', ''),
      addSuffix: flowParams.getString('addSuffix', ''),
    };
    
    // Log the extracted parameters for debugging
    logMessage(`Text Processor Plugin: Input="${inputText}", Options=${JSON.stringify(options)}`);
    
    let processedText = inputText;
    
    // Apply text transformations based on configuration
    if (options.uppercase) {
      processedText = processedText.toUpperCase();
    }
    
    if (options.lowercase) {
      processedText = processedText.toLowerCase();
    }
    
    if (options.reverse) {
      processedText = processedText.split('').reverse().join('');
    }
    
    if (options.removeSpaces) {
      processedText = processedText.replace(/\s/g, '');
    }
    
    if (options.addPrefix) {
      processedText = options.addPrefix + processedText;
    }
    
    if (options.addSuffix) {
      processedText = processedText + options.addSuffix;
    }
    
    // Log the result
    logMessage(`Text Processor Plugin: Processed "${inputText}" -> "${processedText}"`);
    
    return {
      success: true,
      output: processedText,
      outputs: [processedText] // Single output for this plugin
    };
    
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    logMessage(`Text Processor Plugin: Error - ${errorMessage}`);
    
    return {
      success: false,
      error: errorMessage,
      error_code: "EXECUTION_ERROR"
    };
  }
}

let manifestData: PluginMetadata | null = null;

/**
 * Get embedded manifest data
 * This will be replaced at build time with the actual manifest data
 */
function loadManifest(): PluginMetadata {
  // manifestData will be embedded at build time by esbuild plugin
  if (!manifestData) {
    throw new Error("Manifest data not embedded properly");
  }
  
  return manifestData;
}

/**
 * Get plugin metadata for VortiDeck flow builder.
 * Now loads from manifest.json instead of hardcoding.
 *
 * @param {string} input - Empty string parameter
 * @returns {PluginMetadata} 
 */
export function get_metadataImpl(input: string): PluginMetadata {
  return loadManifest();
}

/**
 * Register the plugin with VortiDeck system.
 * Called when the plugin is first loaded.
 *
 * @param {string} input - Empty string parameter
 * @returns {string} Registration confirmation message
 */
export function registerImpl(input: string): string {
  try {
    // Log plugin registration
    logMessage("Text Processor Plugin: Registering with VortiDeck...");
    
    // Get the plugin metadata
    const metadata = get_metadataImpl("");
    
    // Register the node with VortiDeck
    const registrationResult = registerNode(metadata);
    
    if (registrationResult) {
      logMessage("Text Processor Plugin: Successfully registered!");
      return "Text Processor Plugin registered successfully";
    } else {
      logMessage("Text Processor Plugin: Registration failed");
      return "Text Processor Plugin registration failed";
    }
    
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    logMessage(`Text Processor Plugin: Registration error - ${errorMessage}`);
    return `Text Processor Plugin registration error: ${errorMessage}`;
  }
}



