# 📚 VortiDeck Plugin API Reference

Complete reference for VortiDeck plugin development APIs.

## 📋 Table of Contents

1. [Core Interfaces](#core-interfaces)
2. [FlowParams Class](#flowparams-class)
3. [Plugin Functions](#plugin-functions)
4. [Host Functions](#host-functions)
5. [Schema Definitions](#schema-definitions)
6. [Error Handling](#error-handling)
7. [Type Definitions](#type-definitions)

## 🔧 Core Interfaces

### ExecutionParams

```typescript
interface ExecutionParams {
  [key: string]: any;
}
```

Raw flow parameters with metadata. Use with FlowParams helper for type-safe access.

**Example:**
```typescript
// Flow format
{
  "input": {"value": "Hello", "type": "string", "attached": false},
  "count": {"value": 5, "type": "number", "attached": false}
}

// Direct format (backward compatibility)
{
  "input": "Hello",
  "count": 5
}
```

### ExecutionResult

```typescript
interface ExecutionResult {
  success: boolean;
  output?: string;
  outputs?: string[];
  error?: string;
  error_code?: string;
}
```

**Properties:**
- `success` - Whether execution completed successfully
- `output` - Primary output (for single-output plugins)
- `outputs` - Array of outputs (for multi-output plugins)
- `error` - Error message if execution failed
- `error_code` - Specific error code for programmatic handling

**Example:**
```typescript
// Success case
{
  success: true,
  output: "HELLO WORLD",
  outputs: ["HELLO WORLD"]
}

// Error case
{
  success: false,
  error: "Input text is required",
  error_code: "MISSING_INPUT"
}
```

### PluginMetadata

```typescript
interface PluginMetadata {
  id: string;
  name: string;
  description: string;
  category: string;
  type_color: string;
  outputs_count: number;
  version?: string;
  author?: string;
  category_display_name?: string;
  parameters_schema?: any;
  tags?: string[];
}
```

**Required Properties:**
- `id` - Unique plugin identifier (auto-prefixed with `plugin.`)
- `name` - Display name for the plugin
- `description` - What the plugin does
- `category` - Plugin category (e.g., "TEXT_PROCESSING")
- `type_color` - Hex color for the node (e.g., "#4CAF50")
- `outputs_count` - Number of output ports

**Optional Properties:**
- `version` - Plugin version string
- `author` - Plugin creator
- `category_display_name` - Human-readable category name
- `parameters_schema` - JSON schema for parameters
- `tags` - Keywords for search/discovery

## 🛠️ FlowParams Class

The FlowParams class provides type-safe parameter extraction from VortiDeck flow format.

### Constructor

```typescript
new FlowParams(params: any)
```

**Parameters:**
- `params` - Raw ExecutionParams object

**Example:**
```typescript
const flowParams = new FlowParams(input);
```

### String Methods

#### getString()

```typescript
getString(key: string, defaultValue: string = ""): string
```

Extract string value with default fallback.

**Parameters:**
- `key` - Parameter name
- `defaultValue` - Default value if parameter missing or empty

**Example:**
```typescript
const text = flowParams.getString('inputText', 'Hello');
const prefix = flowParams.getString('prefix', '>>> ');
```

#### getInput()

```typescript
getInput(defaultValue: string = ""): string
```

Auto-detect main input from common field names (`input`, `text`, `inputText`).

**Example:**
```typescript
const mainInput = flowParams.getInput('Default text');
```

### Boolean Methods

#### getBoolean()

```typescript
getBoolean(key: string, defaultValue: boolean = false): boolean
```

Extract boolean value with type conversion.

**Parameters:**
- `key` - Parameter name
- `defaultValue` - Default value if parameter missing

**Type Conversion:**
- `true`, `"true"` → `true`
- `false`, `"false"` → `false`
- Other values → Boolean conversion

**Example:**
```typescript
const isEnabled = flowParams.getBoolean('enabled', false);
const shouldProcess = flowParams.getBoolean('process', true);
```

### Number Methods

#### getNumber()

```typescript
getNumber(key: string, defaultValue: number = 0): number
```

Extract number value with parsing and validation.

**Parameters:**
- `key` - Parameter name
- `defaultValue` - Default value if parameter missing or invalid

**Example:**
```typescript
const count = flowParams.getNumber('repeatCount', 1);
const threshold = flowParams.getNumber('threshold', 0.5);
```

### Connection Methods

#### isAttached()

```typescript
isAttached(key: string): boolean
```

Check if parameter is connected to another node's output.

**Example:**
```typescript
if (flowParams.isAttached('input')) {
  // Handle connected input
  const data = flowParams.getString('input', '');
} else {
  // Handle manual input
  const data = flowParams.getInput('Enter text...');
}
```

### Utility Methods

#### getKeys()

```typescript
getKeys(): string[]
```

Get all available parameter names.

**Example:**
```typescript
const allParams = flowParams.getKeys();
console.log('Available parameters:', allParams);
```

#### getRaw()

```typescript
getRaw(key: string): any
```

Get raw parameter object (useful for debugging).

**Example:**
```typescript
const rawParam = flowParams.getRaw('complexParam');
console.log('Raw parameter:', rawParam);
```

#### toSimpleObject()

```typescript
toSimpleObject(): Record<string, any>
```

Convert all parameters to simple key-value format.

**Example:**
```typescript
const simpleParams = flowParams.toSimpleObject();
console.log('Simplified parameters:', simpleParams);
```

## 🔌 Plugin Functions

### Required Functions

#### executeImpl()

```typescript
export function executeImpl(input: ExecutionParams): ExecutionResult
```

Main plugin execution function. Called when the plugin runs in a flow.

**Parameters:**
- `input` - Raw flow parameters

**Returns:**
- `ExecutionResult` - Execution results

**Example:**
```typescript
export function executeImpl(input: ExecutionParams): ExecutionResult {
  try {
    const flowParams = new FlowParams(input);
    const text = flowParams.getInput('');
    
    // Your logic here
    const result = processText(text);
    
    return {
      success: true,
      output: result,
      outputs: [result]
    };
  } catch (error) {
    return {
      success: false,
      error: error instanceof Error ? error.message : String(error),
      error_code: "EXECUTION_ERROR"
    };
  }
}
```

#### get_metadataImpl()

```typescript
export function get_metadataImpl(input: string): PluginMetadata
```

Returns plugin metadata for VortiDeck registration.

**Parameters:**
- `input` - Empty string (unused)

**Returns:**
- `PluginMetadata` - Plugin information

**Example:**
```typescript
export function get_metadataImpl(input: string): PluginMetadata {
  return {
    id: "my-plugin",
    name: "My Plugin",
    description: "Does amazing things",
    category: "UTILITIES",
    type_color: "#2196F3",
    outputs_count: 1,
    parameters_schema: {
      type: "object",
      properties: {
        input: {
          type: "string",
          title: "Input",
          default: ""
        }
      }
    }
  };
}
```

### Optional Functions

#### registerImpl()

```typescript
export function registerImpl(input: string): string
```

Called during plugin initialization. Use for setup logic.

**Example:**
```typescript
export function registerImpl(input: string): string {
  // Initialization logic here
  return "Plugin registered successfully";
}
```

## 🏠 Host Functions

VortiDeck provides host functions for plugin-to-host communication.

### vortideck_log()

```typescript
function vortideck_log(input: ArrayBufferLike): ArrayBufferLike
```

Log messages to VortiDeck console for debugging.

**Example:**
```typescript
function logMessage(message: string): void {
  try {
    const encoder = new TextEncoder();
    const buffer = encoder.encode(message);
    vortideck_log(buffer);
  } catch (error) {
    console.log(message); // Fallback
  }
}

// Usage
logMessage("Plugin executing...");
```

### vortideck_register_node()

```typescript
function vortideck_register_node(input: ArrayBufferLike): ArrayBufferLike
```

Register plugin as a flow node (usually called automatically).

### vortideck_get_context()

```typescript
function vortideck_get_context(): ArrayBufferLike
```

Get VortiDeck execution context information.

## 📝 Schema Definitions

### Parameter Schema Format

Uses JSON Schema for parameter definitions:

```typescript
parameters_schema: {
  type: "object",
  properties: {
    // String parameter
    textInput: {
      type: "string",
      title: "Text Input",
      description: "Enter your text here",
      default: "Hello World",
      minLength: 1,
      maxLength: 1000
    },
    
    // Boolean parameter
    enableFeature: {
      type: "boolean", 
      title: "Enable Feature",
      description: "Turn this feature on/off",
      default: false
    },
    
    // Number parameter
    count: {
      type: "number",
      title: "Count",
      description: "Number of repetitions",
      default: 1,
      minimum: 1,
      maximum: 100
    },
    
    // Enum parameter
    mode: {
      type: "string",
      title: "Processing Mode",
      description: "How to process the data",
      enum: ["simple", "advanced", "custom"],
      default: "simple"
    },
    
    // Object parameter
    settings: {
      type: "object",
      title: "Advanced Settings",
      properties: {
        timeout: {
          type: "number",
          title: "Timeout (ms)",
          default: 5000
        },
        retries: {
          type: "number", 
          title: "Retry Count",
          default: 3
        }
      }
    }
  },
  required: ["textInput"] // Required parameters
}
```

### Common Schema Properties

| Property | Type | Description |
|----------|------|-------------|
| `type` | string | Parameter type: "string", "number", "boolean", "object" |
| `title` | string | Display name in UI |
| `description` | string | Help text for users |
| `default` | any | Default value |
| `enum` | array | Valid values for dropdown |
| `minimum` | number | Minimum value (numbers) |
| `maximum` | number | Maximum value (numbers) |
| `minLength` | number | Minimum length (strings) |
| `maxLength` | number | Maximum length (strings) |
| `required` | array | Required parameter names |

## ⚠️ Error Handling

### Standard Error Codes

| Code | Description | Usage |
|------|-------------|-------|
| `MISSING_INPUT` | Required input missing | Parameter validation |
| `INVALID_RANGE` | Value outside valid range | Number validation |
| `TYPE_ERROR` | Type conversion failed | Data processing |
| `EXECUTION_ERROR` | General execution failure | Catch-all errors |
| `TIMEOUT_ERROR` | Operation timed out | Async operations |
| `NETWORK_ERROR` | Network request failed | API calls |

### Error Response Format

```typescript
{
  success: false,
  error: "Human-readable error message",
  error_code: "MACHINE_READABLE_CODE"
}
```

### Best Practices

```typescript
export function executeImpl(input: ExecutionParams): ExecutionResult {
  try {
    const flowParams = new FlowParams(input);
    
    // Validate inputs
    const text = flowParams.getInput('');
    if (!text.trim()) {
      return {
        success: false,
        error: "Input text cannot be empty",
        error_code: "MISSING_INPUT"
      };
    }
    
    // Validate ranges
    const count = flowParams.getNumber('count', 1);
    if (count < 1 || count > 1000) {
      return {
        success: false,
        error: "Count must be between 1 and 1000",
        error_code: "INVALID_RANGE"
      };
    }
    
    // Process with error handling
    const result = processText(text, count);
    
    return {
      success: true,
      output: result,
      outputs: [result]
    };
    
  } catch (error) {
    // Provide helpful error information
    const errorMessage = error instanceof Error ? error.message : String(error);
    
    return {
      success: false,
      error: `Processing failed: ${errorMessage}`,
      error_code: getErrorCode(error)
    };
  }
}

function getErrorCode(error: any): string {
  if (error instanceof TypeError) return "TYPE_ERROR";
  if (error instanceof RangeError) return "INVALID_RANGE";
  if (error.name === 'TimeoutError') return "TIMEOUT_ERROR";
  return "EXECUTION_ERROR";
}
```

## 🏷️ Type Definitions

### Common Types

```typescript
// Parameter extraction types
type ParameterValue = string | number | boolean | object;
type ParameterKey = string;
type DefaultValue<T> = T;

// Flow format types
interface FlowParameter {
  value: any;
  type: string;
  attached: boolean;
}

// Plugin categories
type PluginCategory = 
  | "TEXT_PROCESSING"
  | "DATA_CONVERSION" 
  | "UTILITIES"
  | "INTEGRATIONS"
  | "MATH"
  | "VALIDATION"
  | "CUSTOM";

// Output port types
type OutputType = "string" | "number" | "boolean" | "object" | "any";
```

### Memory Types (Extism)

```typescript
// Extism memory types
type I32 = number;
type I64 = number;
type F32 = number;
type F64 = number;

// Memory buffer types
interface Memory {
  fromString(s: string): { offset: I64 };
  find(offset: I64): { readString(): string };
  fromBuffer(buffer: ArrayBufferLike): { offset: I64 };
}
```

---

## 📖 Usage Examples

### Simple Text Plugin

```typescript
import { FlowParams, ExecutionParams, ExecutionResult } from './pdk';

export function executeImpl(input: ExecutionParams): ExecutionResult {
  const flowParams = new FlowParams(input);
  const text = flowParams.getInput('');
  const operation = flowParams.getString('operation', 'uppercase');
  
  let result = text;
  switch (operation) {
    case 'uppercase':
      result = text.toUpperCase();
      break;
    case 'lowercase':
      result = text.toLowerCase();
      break;
    case 'reverse':
      result = text.split('').reverse().join('');
      break;
  }
  
  return { success: true, output: result, outputs: [result] };
}
```

### Multi-Output Plugin

```typescript
export function executeImpl(input: ExecutionParams): ExecutionResult {
  const flowParams = new FlowParams(input);
  const text = flowParams.getInput('');
  
  const upper = text.toUpperCase();
  const lower = text.toLowerCase();
  const count = text.length.toString();
  
  return {
    success: true,
    output: upper, // Primary output
    outputs: [upper, lower, count] // All outputs
  };
}
```

This completes the comprehensive API reference for VortiDeck plugin development! 