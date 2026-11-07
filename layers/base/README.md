# VortiDeck Base Layer

This Nuxt layer contains shared components, utilities, and functionality used across all VortiDeck applications (desktop, mobile, and web). It provides the foundation for consistent UI and behavior across platforms.

## 🎯 Purpose

- **Shared Components**: Reusable UI components for all platforms
- **Common Utilities**: Helper functions and composables
- **Base Styling**: Core design system and theme
- **Type Definitions**: Shared TypeScript interfaces and types
- **API Clients**: Common API interaction logic

## 🏗️ Structure

```
layers/base/
├── components/          # Shared Vue components
│   ├── ui/             # Basic UI components (buttons, inputs, etc.)
│   ├── forms/          # Form components and validation
│   └── layout/         # Layout components
├── composables/        # Shared composables and utilities
├── types/              # TypeScript type definitions
├── utils/              # Utility functions
├── stores/             # Shared Pinia stores
├── assets/             # Shared assets and base styles
└── nuxt.config.ts      # Base layer configuration
```

## 🧩 Key Components

### UI Components
- **VButton**: Consistent button styling across platforms
- **VInput**: Standardized input fields with validation
- **VModal**: Reusable modal component
- **VCard**: Card layout component
- **VIcon**: Icon wrapper with consistent sizing

### Form Components
- **VForm**: Form wrapper with validation
- **VFieldset**: Grouped form fields
- **VSelect**: Dropdown selection component
- **VCheckbox**: Checkbox with label
- **VRadio**: Radio button component

### Layout Components
- **VContainer**: Responsive container
- **VGrid**: Grid layout system
- **VStack**: Vertical stack layout
- **VCluster**: Horizontal cluster layout

## 🛠️ Shared Utilities

### Composables
```typescript
// useApi - API interaction
const { get, post, put, delete } = useApi()

// useValidation - Form validation
const { validate, errors } = useValidation(schema)

// useTheme - Theme management
const { theme, toggleTheme } = useTheme()

// useStorage - Local storage wrapper
const { get, set, remove } = useStorage()
```

### Type Definitions
```typescript
// Core VortiDeck types
interface FlowNode {
  id: string
  type: string
  position: { x: number; y: number }
  data: Record<string, any>
}

interface CanvasConnection {
  id: string
  source: string
  target: string
  sourceHandle?: string
  targetHandle?: string
}

interface AiModel {
  id: string
  name: string
  provider: string
  capabilities: string[]
}
```

### Utility Functions
```typescript
// String utilities
export const slugify = (text: string): string => { /* ... */ }
export const capitalize = (text: string): string => { /* ... */ }

// Date utilities
export const formatDate = (date: Date, format: string): string => { /* ... */ }
export const timeAgo = (date: Date): string => { /* ... */ }

// Validation utilities
export const isEmail = (email: string): boolean => { /* ... */ }
export const isUrl = (url: string): boolean => { /* ... */ }
```

## 🎨 Design System

### Color Palette
```scss
// Primary colors
$primary-50: #eff6ff;
$primary-500: #3b82f6;
$primary-900: #1e3a8a;

// Secondary colors
$secondary-50: #faf5ff;
$secondary-500: #8b5cf6;
$secondary-900: #581c87;

// Neutral colors
$gray-50: #f9fafb;
$gray-500: #6b7280;
$gray-900: #111827;
```

### Typography Scale
```scss
// Font sizes
$text-xs: 0.75rem;    // 12px
$text-sm: 0.875rem;   // 14px
$text-base: 1rem;     // 16px
$text-lg: 1.125rem;   // 18px
$text-xl: 1.25rem;    // 20px

// Font weights
$font-normal: 400;
$font-medium: 500;
$font-semibold: 600;
$font-bold: 700;
```

### Spacing System
```scss
// Spacing scale (based on 0.25rem = 4px)
$space-1: 0.25rem;    // 4px
$space-2: 0.5rem;     // 8px
$space-4: 1rem;       // 16px
$space-8: 2rem;       // 32px
$space-16: 4rem;      // 64px
```

## 📦 Setup

This layer is automatically included in all VortiDeck applications:

```bash
# Install dependencies
pnpm install

# The base layer is extended by other layers automatically
```

## 🎮 Usage

### Extending the Base Layer

```typescript
// nuxt.config.ts (in other layers or apps)
export default defineNuxtConfig({
  extends: ['@vortideck/base-layer']
})
```

### Using Shared Components

```vue
<template lang="pug">
  VContainer
    VCard
      VStack(gap="4")
        h2 My Component
        VButton(@click="handleClick") Click Me
        VInput(v-model="inputValue" placeholder="Enter text")
</template>

<script setup lang="ts">
import type { FlowNode } from '@vortideck/base-layer/types'

const inputValue = ref('')
const handleClick = () => {
  console.log('Button clicked!')
}
</script>
```

### Using Composables

```vue
<script setup lang="ts">
const { get, post } = useApi()
const { theme, toggleTheme } = useTheme()

// Fetch data
const { data: flows } = await get('/api/flows')

// Toggle theme
const handleThemeToggle = () => {
  toggleTheme()
}
</script>
```

## 🔧 Configuration

### Base Layer Config
```typescript
// nuxt.config.ts
export default defineNuxtConfig({
  // Base configuration shared across all apps
  css: [
    '~/assets/css/base.css'
  ],
  
  modules: [
    '@pinia/nuxt',
    '@nuxtjs/tailwindcss',
    '@vueuse/nuxt'
  ],
  
  tailwindcss: {
    config: {
      theme: {
        extend: {
          colors: {
            primary: {
              50: '#eff6ff',
              500: '#3b82f6',
              900: '#1e3a8a'
            }
          }
        }
      }
    }
  }
})
```

### Environment Variables
```bash
# API configuration
VORTIDECK_API_URL=https://api.vortideck.com
VORTIDECK_WS_URL=wss://ws.vortideck.com

# Feature flags
VORTIDECK_ENABLE_ANALYTICS=true
VORTIDECK_ENABLE_TELEMETRY=false
```

## 🧪 Testing

### Component Testing
```bash
# Run component tests
pnpm test:components

# Test specific component
pnpm test:components VButton
```

### Utility Testing
```bash
# Run utility tests
pnpm test:utils

# Test with coverage
pnpm test:coverage
```

## 📚 Documentation

### Component Documentation
Each component includes comprehensive documentation:

```vue
<!-- VButton.vue -->
<template>
  <!-- Component template -->
</template>

<script setup lang="ts">
/**
 * VButton - Consistent button component
 * 
 * @param variant - Button style variant
 * @param size - Button size
 * @param disabled - Whether button is disabled
 * @param loading - Whether button shows loading state
 */
interface Props {
  variant?: 'primary' | 'secondary' | 'outline' | 'ghost'
  size?: 'sm' | 'md' | 'lg'
  disabled?: boolean
  loading?: boolean
}
</script>
```

### API Documentation
- Component API
- Composables API
- Utilities API
- Type Definitions

## 🤝 Contributing

### Development Guidelines
1. **Components**: Follow Vue 3 Composition API patterns
2. **Styling**: Use Tailwind utility classes with design tokens
3. **Types**: Include comprehensive TypeScript types
4. **Testing**: Add unit tests for all utilities and components
5. **Documentation**: Document all public APIs

### Adding New Components
```bash
# Create component with tests and documentation
pnpm generate:component MyComponent

# This creates:
# - components/MyComponent.vue
# - components/__tests__/MyComponent.test.ts
# - docs/components/MyComponent.md
```

### Code Standards
- **ESLint**: Follow the project ESLint configuration
- **Prettier**: Use Prettier for code formatting
- **TypeScript**: Strict type checking enabled
- **Testing**: Minimum 80% code coverage

## 🔄 Versioning

The base layer follows semantic versioning:
- **Major**: Breaking changes to public APIs
- **Minor**: New features, backward compatible
- **Patch**: Bug fixes and improvements

## 📄 License

This layer is part of VortiDeck and is licensed under the MIT License.
