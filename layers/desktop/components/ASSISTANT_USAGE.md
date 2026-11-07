# AI Assistant Component Usage Guide

The AI Assistant is a context-aware AI helper that opens as a separate Tauri window and can help fill forms, suggest commands, and provide tips.

## Basic Usage

### Simple Button

```vue
<template>
  <Assistant
    preset="commandInput"
    buttonClass="bg-blue-500 hover:bg-blue-600 text-white"
    showText
  />
</template>
```

### With Input Field

```vue
<template>
  <div class="flex gap-2">
    <input ref="commandInput" v-model="command" placeholder="Enter command..." />
    <Assistant
      :targetInput="commandInput"
      preset="commandInput"
      @fill="handleFill"
    />
  </div>
</template>

<script setup>
const commandInput = ref();
const command = ref('');

const handleFill = (value) => {
  command.value = value;
  console.log('AI filled:', value);
};
</script>
```

### Custom Context

```vue
<template>
  <Assistant
    context="executable_path"
    contextInfo="Program Executable"
    :targetInput="executableInput"
    :formData="{ programType: 'command', workingDir: '/home/user' }"
  />
</template>
```

## Available Presets

- `commandInput` - For command line inputs
- `executablePath` - For executable file paths
- `programArgs` - For program arguments
- `scriptEditor` - For script content
- `macroRecording` - For macro actions
- `workingDirectory` - For directory paths
- `environmentVariables` - For env vars

## Advanced Usage with Composable

```vue
<template>
  <div>
    <input ref="myInput" v-model="value" />
    <button @click="assistant.open()">Ask AI</button>
  </div>
</template>

<script setup>
import { useAIAssistant } from '@/composables/useAIAssistant';

const myInput = ref();
const value = ref('');

const assistant = useAIAssistant({
  context: 'custom_context',
  contextInfo: 'My Custom Input',
  targetInput: myInput,
  formData: computed(() => ({
    relatedField1: someValue.value,
    relatedField2: anotherValue.value,
  })),
  onFill: (filledValue) => {
    value.value = filledValue;
    // Custom logic after fill
  },
});
</script>
```

## Props

| Prop | Type | Description |
|------|------|-------------|
| `preset` | string | Use a predefined configuration |
| `context` | string | Context identifier for AI |
| `contextInfo` | string | Human-readable context label |
| `targetInput` | HTMLElement | Input element to fill |
| `formData` | object | Additional form context |
| `buttonText` | string | Button label text |
| `buttonClass` | string | Custom button classes |
| `showText` | boolean | Show button text |
| `title` | string | Button tooltip |

## Events

- `@fill` - Emitted when AI fills a value
- `@opened` - Emitted when assistant window opens

## Features

- ✨ **Context-Aware**: Understands what you're trying to do
- 🎯 **Smart Suggestions**: Provides relevant command/path suggestions
- 🔄 **Auto-Fill**: Can fill input fields directly
- 💡 **Tips & Tricks**: Shows helpful tips for each context
- 🪟 **Separate Window**: Opens as a Tauri window, always on top
- 🎨 **Custom Styling**: Fully customizable appearance

## Backend Requirements

Make sure to:
1. Register assistant commands in `main.rs`
2. Initialize assistant context store
3. Add route for `/assistant-window` page
4. Have MCP enabled and connected for AI responses

## Example: Program Creation Form

```vue
<template>
  <div class="space-y-4">
    <!-- Program Path -->
    <div class="flex gap-2">
      <input
        ref="pathInput"
        v-model="programPath"
        placeholder="Program path..."
        class="flex-1"
      />
      <Assistant
        preset="executablePath"
        :targetInput="pathInput"
        buttonClass="bg-blue-500 hover:bg-blue-600 text-white"
      />
    </div>

    <!-- Arguments -->
    <div class="flex gap-2">
      <textarea
        ref="argsInput"
        v-model="programArgs"
        placeholder="Arguments..."
        class="flex-1"
      />
      <Assistant
        preset="programArgs"
        :targetInput="argsInput"
        :formData="{ programPath }"
      />
    </div>

    <!-- Working Directory -->
    <div class="flex gap-2">
      <input
        ref="dirInput"
        v-model="workingDir"
        placeholder="Working directory..."
        class="flex-1"
      />
      <Assistant
        preset="workingDirectory"
        :targetInput="dirInput"
      />
    </div>
  </div>
</template>
```
