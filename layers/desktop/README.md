# VortiDeck Desktop Layer

This Nuxt layer contains desktop-specific UI components, pages, and functionality for the VortiDeck application. It provides the visual interface for the AI flow builder, control deck, and desktop-specific features.

## 🎯 Features

- **Flow Builder Interface**: Visual drag-and-drop flow creation with AI nodes
- **AI Model Selection**: Per-node AI model configuration UI
- **Tool Integration**: Visual tool library and connection system
- **Real-time Monitoring**: Live flow execution status and feedback
- **Desktop-Optimized**: Tailored for desktop screen sizes and interactions

## 🏗️ Structure

```
layers/desktop/
├── components/           # Desktop-specific Vue components
├── pages/               # Desktop application pages
│   ├── deck/           # Control deck and flow builder pages
│   ├── settings/       # Application settings
│   └── dashboard/      # Main dashboard
├── stores/             # Pinia stores for desktop state
├── composables/        # Desktop-specific composables
├── assets/             # Desktop assets and styles
└── nuxt.config.ts      # Layer configuration
```

## 🎨 Key Components

### Flow Builder (`pages/deck/new/flow.vue`)
The main visual flow builder interface featuring:
- **Node Palette**: Categorized library of available nodes and tools
- **Canvas**: Infinite canvas with zoom, pan, and grid snapping
- **Connection System**: Visual connections between nodes with type safety
- **AI Model Selector**: Per-node AI model selection modal
- **Execution Controls**: Start, stop, and monitor flow execution

### Node Components
- **AI Nodes**: Visual representation of AI nodes with behavior indicators
- **Tool Nodes**: System and MCP tool nodes with parameter inputs
- **Trigger Nodes**: Flow starting points (webhook, button, timer, etc.)
- **Control Nodes**: Flow control logic (branch, delay, etc.)

### UI Features
- **Responsive Design**: Optimized for desktop screen sizes
- **Dark Theme**: Modern dark UI with gradient accents
- **Smooth Animations**: Fluid transitions and hover effects
- **Keyboard Shortcuts**: Efficient keyboard navigation
- **Context Menus**: Right-click actions for nodes and connections

## 🛠️ Technology Stack

- **Framework**: Vue.js 3 with Composition API
- **Styling**: Tailwind CSS with custom components
- **Templates**: Pug for clean, readable templates
- **Icons**: Iconify with multiple icon sets
- **State Management**: Pinia stores
- **Build System**: Vite with Nuxt layer support

## 📦 Setup

This layer is automatically included when developing the desktop application:

```bash
# Install dependencies
pnpm install

# Start development (from desktop app)
cd apps/tauri-windows
pnpm dev
```

## 🎮 Usage

### Extending the Layer

To use this layer in other applications:

```typescript
// nuxt.config.ts
export default defineNuxtConfig({
  extends: ['@vortideck/desktop-layer']
})
```

### Adding New Components

```vue
<!-- components/MyDesktopComponent.vue -->
<template lang="pug">
  div(class="desktop-component")
    h2 My Desktop Component
    // Component content
</template>

<script setup lang="ts">
// Component logic
</script>

<style scoped>
.desktop-component {
  /* Desktop-specific styles */
}
</style>
```

### Creating New Pages

```vue
<!-- pages/my-page.vue -->
<template lang="pug">
  div(class="page-container")
    h1 My Desktop Page
    // Page content
</template>

<script setup lang="ts">
// Page logic with desktop-specific features
</script>
```

## 🎨 Styling Guidelines

### Color Scheme
- **Primary**: Blue gradients (#3b82f6 to #1d4ed8)
- **Secondary**: Purple accents (#8b5cf6 to #7c3aed)
- **Background**: Dark grays (#1a1a1a, #2d3748)
- **Text**: White with opacity variations

### Component Patterns
- **Cards**: Rounded corners with subtle borders and shadows
- **Buttons**: Gradient backgrounds with hover effects
- **Inputs**: Dark backgrounds with focus states
- **Modals**: Backdrop blur with centered content

### Responsive Breakpoints
- **Desktop**: 1024px and above (primary target)
- **Large Desktop**: 1440px and above (enhanced layouts)
- **Ultra-wide**: 1920px and above (multi-panel layouts)

## 🔧 Configuration

### Layer Configuration
```typescript
// nuxt.config.ts
export default defineNuxtConfig({
  // Desktop-specific configuration
  ssr: false,  // Desktop apps don't need SSR
  
  css: [
    '~/assets/css/desktop.css'
  ],
  
  modules: [
    '@pinia/nuxt',
    '@nuxtjs/tailwindcss'
  ]
})
```

### Tailwind Configuration
```javascript
// tailwind.config.js
module.exports = {
  theme: {
    extend: {
      screens: {
        'desktop': '1024px',
        'large-desktop': '1440px',
        'ultra-wide': '1920px'
      }
    }
  }
}
```

## 🧪 Testing

### Component Testing
```bash
# Run component tests
pnpm test:components

# Run with coverage
pnpm test:coverage
```

### E2E Testing
```bash
# Run end-to-end tests
pnpm test:e2e

# Run specific test suite
pnpm test:e2e --spec="flow-builder"
```

## 📚 Documentation

- [Flow Builder Guide](../../docs/user-guide/flow-builder.md)
- Component Library
- Styling Guide
- Desktop Patterns

## 🤝 Contributing

### Development Workflow
1. Create feature branch
2. Develop in the desktop layer
3. Test with the desktop application
4. Update documentation
5. Submit pull request

### Component Guidelines
- Use Composition API with `<script setup>`
- Follow Vue 3 best practices
- Include TypeScript types
- Add comprehensive tests
- Document props and events

### Styling Guidelines
- Use Tailwind utility classes
- Follow the established color scheme
- Ensure desktop optimization
- Test across different screen sizes

## 📄 License

This layer is part of VortiDeck and is licensed under the MIT License.
