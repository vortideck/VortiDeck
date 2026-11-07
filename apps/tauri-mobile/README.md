# VortiDeck Mobile Application

The mobile version of VortiDeck built with Tauri Mobile, providing AI flow builder capabilities on iOS and Android devices.

## 🎯 Features

- **Mobile Flow Builder**: Touch-optimized interface for creating AI workflows
- **Gesture Controls**: Intuitive touch gestures for deck interaction
- **Offline Capability**: Local flow execution without internet dependency
- **Cross-Platform**: Native performance on both iOS and Android
- **Mobile-Optimized UI**: Responsive design tailored for mobile screens

## 🏗️ Architecture

```
apps/tauri-mobile/
├── src-tauri/          # Tauri mobile backend (Rust)
├── pages/              # Mobile application pages
├── components/         # Mobile-specific components
├── composables/        # Mobile utility functions
└── nuxt.config.ts      # Mobile app configuration
```

## 📱 Development Status

This mobile application is currently in development planning phase. It will provide:

- Core VortiDeck functionality optimized for mobile
- Touch-first user interface
- Native mobile device integration
- Offline-first architecture
- Mobile-specific AI behaviors

## 🔧 Setup

Make sure to install dependencies:

```bash
# pnpm
pnpm install
```

## Development Server

Start the development server on `http://localhost:3000`:

```bash
pnpm dev
```

## Mobile Development

For mobile development with Tauri:

```bash
# iOS development
pnpm tauri ios dev

# Android development  
pnpm tauri android dev
```

## Production

Build the application for production:

```bash
pnpm build
```

Build for mobile platforms:

```bash
# iOS build
pnpm tauri ios build

# Android build
pnpm tauri android build
```

## 🚀 Future Enhancements

- Haptic feedback for button interactions
- Mobile device sensors integration
- Camera integration for QR code scanning
- Voice control capabilities
- Mobile-specific themes and layouts

Check out the [Tauri Mobile documentation](https://tauri.app/v1/guides/building/mobile) for more information.
