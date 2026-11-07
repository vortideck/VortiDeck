# ⚡ VortiDeck Plugin Quick Start

Get your first plugin running in 5 minutes!

## 🎯 Goal

Create a simple "Text Reverser" plugin that reverses input text.

## 📋 Prerequisites

- Node.js 18+ installed
- Text editor or IDE

## 🚀 Step 1: Setup

```bash
# Clone the plugin template
cp -r examples/plugin-xtp examples/text-reverser
cd examples/text-reverser

# Install dependencies
npm install
```

## ✏️ Step 2: Edit Plugin Logic

Open `src/main.ts` and replace the content:

```typescript
import {
  ExecutionParams,
  ExecutionResult,
  PluginMetadata,
  FlowParams,
} from './pdk';

/**
 * Execute the text reverser plugin
 */
export function executeImpl(input: ExecutionParams): ExecutionResult {
  try {
    // Extract parameters using FlowParams helper
    const flowParams = new FlowParams(input);
    const inputText = flowParams.getInput("");
    
    // Validate input
    if (!inputText) {
      return {
        success: false,
        error: "Please provide text to reverse",
        error_code: "MISSING_INPUT"
      };
    }
    
    // Reverse the text
    const reversedText = inputText.split('').reverse().join('');
    
    return {
      success: true,
      output: reversedText,
      outputs: [reversedText]
    };
    
  } catch (error) {
    return {
      success: false,
      error: error instanceof Error ? error.message : String(error),
      error_code: "EXECUTION_ERROR"
    };
  }
}

/**
 * Plugin metadata
 */
export function get_metadataImpl(input: string): PluginMetadata {
  return {
    id: "text-reverser",
    name: "Text Reverser",
    description: "Reverses the order of characters in text",
    category: "TEXT_PROCESSING",
    category_display_name: "Text Tools",
    type_color: "#FF5722",
    outputs_count: 1,
    version: "1.0.0",
    author: "You",
    tags: ["text", "reverse", "utility"],
    parameters_schema: {
      type: "object",
      properties: {
        input: {
          type: "string",
          title: "Input Text",
          description: "Text to reverse",
          default: "Hello World"
        }
      }
    }
  };
}

/**
 * Register the plugin
 */
export function registerImpl(input: string): string {
  return "Text Reverser plugin registered successfully";
}
```

## 🔨 Step 3: Build

```bash
npm run build
```

You should see:
```
✓ Build completed successfully
✓ Generated dist/plugin.wasm
```

## 🧪 Step 4: Test in VortiDeck

1. **Copy the plugin:**
   ```bash
   # Copy to your VortiDeck plugins directory
   cp dist/plugin.wasm /path/to/vortideck/plugins/text-reverser.wasm
   ```

2. **Open VortiDeck:**
   - The hot reloading system will detect your plugin automatically
   - Look for "Text Reverser" in the "Text Tools" category

3. **Test the plugin:**
   - Drag "Text Reverser" into your flow
   - Set input text to "Hello World"
   - Run the flow
   - Output should be "dlroW olleH"

## 🎉 Success!

You've created your first VortiDeck plugin! 

## 🔍 What's Next?

### Add More Features

```typescript
export function executeImpl(input: ExecutionParams): ExecutionResult {
  const flowParams = new FlowParams(input);
  const inputText = flowParams.getInput("");
  
  // Add options
  const preserveSpaces = flowParams.getBoolean('preserveSpaces', true);
  const reverseWords = flowParams.getBoolean('reverseWords', false);
  
  let result = inputText;
  
  if (reverseWords) {
    // Reverse word order instead of characters
    result = inputText.split(' ').reverse().join(' ');
  } else {
    // Reverse characters
    if (preserveSpaces) {
      // Keep spaces in place, reverse only letters
      const chars = inputText.split('');
      const letters = chars.filter(c => c !== ' ').reverse();
      let letterIndex = 0;
      result = chars.map(c => c === ' ' ? ' ' : letters[letterIndex++]).join('');
    } else {
      result = inputText.split('').reverse().join('');
    }
  }
  
  return {
    success: true,
    output: result,
    outputs: [result]
  };
}
```

Update the schema:
```typescript
parameters_schema: {
  type: "object",
  properties: {
    input: {
      type: "string",
      title: "Input Text",
      description: "Text to reverse",
      default: "Hello World"
    },
    preserveSpaces: {
      type: "boolean",
      title: "Preserve Spaces",
      description: "Keep spaces in original positions",
      default: true
    },
    reverseWords: {
      type: "boolean", 
      title: "Reverse Words",
      description: "Reverse word order instead of characters",
      default: false
    }
  }
}
```

### More Plugin Ideas

1. **Text Case Converter:** uppercase, lowercase, title case
2. **Word Counter:** count words, characters, sentences
3. **Text Formatter:** remove extra spaces, format paragraphs
4. **String Utilities:** substring, replace, split/join
5. **Data Validator:** email, URL, phone number validation

## 📚 Learn More

- [Full Development Guide](./PLUGIN_DEVELOPMENT_GUIDE.md)
- [Plugin API Reference](./PLUGIN_API_REFERENCE.md)

## 🆘 Need Help?

**Plugin not building?**
```bash
# Check TypeScript errors
npx tsc --noEmit

# Check dependencies
npm install
```

**Plugin not appearing?**
- Ensure `dist/plugin.wasm` exists
- Check VortiDeck console for errors
- Verify hot reloading is enabled

**Runtime errors?**
- Add debug logging to your plugin
- Check browser developer console
- Validate your parameter schema

Happy coding! 🎉 