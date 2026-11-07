/**
 * Theme Registry — catalog of all available overlay themes.
 *
 * Each theme defines colors, fonts, borders, and animation preferences.
 * useOverlayTheme.buildCssVars() converts these into CSS variables at runtime.
 *
 * To add a new theme: add one entry here. No other changes needed.
 */

import type { ThemeDef } from './useOverlayTheme'

export interface ThemeCategory {
  id: string
  name: string
  icon: string
  count: number
}

export const themeCategories: ThemeCategory[] = [
  { id: 'gaming', name: 'Gaming', icon: 'heroicons:puzzle-piece', count: 8 },
  { id: 'professional', name: 'Professional', icon: 'heroicons:briefcase', count: 5 },
  { id: 'creative', name: 'Creative', icon: 'heroicons:paint-brush', count: 6 },
  { id: 'minimal', name: 'Minimal', icon: 'heroicons:minus-circle', count: 4 },
  { id: 'branded', name: 'Branded', icon: 'heroicons:tag', count: 3 },
]

export const availableThemes: (ThemeDef & { category: string })[] = [
  // ── Gaming ────────────────────────────────────────────────────

  {
    id: 'cyberpunk',
    category: 'gaming',
    name: 'Cyberpunk 2077',
    description: 'Neon futuristic style with glowing effects',
    colors: ['#ff00ff', '#00ffff', '#ff1493', '#0a0a0a', '#ffffff', '#1a1a1a'],
    style: 'Neon Futuristic',
    isPro: false,
    fonts: {
      primary: '"Orbitron", "Courier New", monospace',
      secondary: '"Rajdhani", sans-serif',
    },
    spacing: { base: '8px', large: '16px' },
    animations: { speed: 'fast' },
    borders: {
      radius: '0px',
      glow: true,
      type: 'image',
      images: {
        frame: 'https://i.imgur.com/cyber-frame.png',
        corner: 'https://i.imgur.com/cyber-corner.png',
        edge: 'https://i.imgur.com/cyber-edge.png',
        pattern: 'https://i.imgur.com/circuit-pattern.png',
      },
    },
  },
  {
    id: 'retro',
    category: 'gaming',
    name: 'Retro Arcade',
    description: '8-bit pixel art style with blocky elements',
    colors: ['#ff6b35', '#f7931e', '#ffd700', '#2d1b69', '#ffffff', '#1a1a2e'],
    style: 'Pixel Art',
    isPro: false,
    fonts: {
      primary: '"Press Start 2P", "Courier New", monospace',
      secondary: '"VT323", monospace',
    },
    spacing: { base: '4px', large: '8px' },
    animations: { speed: 'instant' },
    borders: {
      radius: '0px',
      glow: false,
      type: 'image',
      images: {
        frame: 'https://i.imgur.com/pixel-frame.png',
        corner: 'https://i.imgur.com/pixel-corner.png',
        pattern: 'https://i.imgur.com/retro-pattern.png',
      },
    },
  },
  {
    id: 'esports',
    category: 'gaming',
    name: 'Esports Pro',
    description: 'Clean competitive gaming style',
    colors: ['#0066cc', '#3399ff', '#ff3333', '#111111', '#ffffff', '#1e1e1e'],
    style: 'Professional',
    isPro: true,
    fonts: {
      primary: '"Roboto Condensed", sans-serif',
      secondary: '"Oswald", sans-serif',
    },
    spacing: { base: '6px', large: '12px' },
    animations: { speed: 'smooth' },
    borders: { radius: '4px', glow: false },
  },

  // ── Professional ──────────────────────────────────────────────

  {
    id: 'clean',
    category: 'professional',
    name: 'Clean Minimal',
    description: 'Simple and professional design',
    colors: ['#3b82f6', '#1e40af', '#60a5fa', '#f8fafc', '#1e293b', '#f1f5f9'],
    style: 'Minimal',
    isPro: false,
    fonts: {
      primary: '"Inter", "Segoe UI", sans-serif',
      secondary: '"Roboto", sans-serif',
    },
    spacing: { base: '12px', large: '24px' },
    animations: { speed: 'smooth' },
    borders: { radius: '8px', glow: false },
  },
  {
    id: 'corporate',
    category: 'professional',
    name: 'Corporate',
    description: 'Business presentation style',
    colors: ['#2c3e50', '#3498db', '#ffffff', '#ecf0f1', '#2c3e50', '#bdc3c7'],
    style: 'Corporate',
    isPro: true,
    fonts: {
      primary: '"Montserrat", "Arial", sans-serif',
      secondary: '"Source Sans Pro", sans-serif',
    },
    spacing: { base: '16px', large: '32px' },
    animations: { speed: 'smooth' },
    borders: { radius: '6px', glow: false },
  },

  // ── Creative ──────────────────────────────────────────────────

  {
    id: 'neon',
    category: 'creative',
    name: 'Neon Dreams',
    description: 'Bright neon colors with energy',
    colors: ['#ff0080', '#8000ff', '#00ff80', '#000000', '#ffffff', '#0d0d0d'],
    style: 'Neon',
    isPro: false,
    fonts: {
      primary: '"Audiowide", "Arial Black", sans-serif',
      secondary: '"Electrolize", monospace',
    },
    spacing: { base: '10px', large: '20px' },
    animations: { speed: 'fast' },
    borders: {
      radius: '0px',
      glow: true,
      type: 'image',
      images: {
        frame: 'https://i.imgur.com/neon-frame.png',
        pattern: 'https://i.imgur.com/energy-pattern.png',
      },
    },
  },
  {
    id: 'pastel',
    category: 'creative',
    name: 'Soft Pastels',
    description: 'Gentle pastel colors for creative content',
    colors: ['#ffc0cb', '#e6e6fa', '#f0e68c', '#f5f5dc', '#2c2c2c', '#f8f8ff'],
    style: 'Pastel',
    isPro: false,
    fonts: {
      primary: '"Comfortaa", "Arial Rounded", sans-serif',
      secondary: '"Nunito", sans-serif',
    },
    spacing: { base: '14px', large: '28px' },
    animations: { speed: 'slow' },
    borders: { radius: '16px', glow: false },
  },

  // ── Minimal ───────────────────────────────────────────────────

  {
    id: 'monochrome',
    category: 'minimal',
    name: 'Monochrome',
    description: 'Black and white minimalist design',
    colors: ['#000000', '#ffffff', '#808080', '#f5f5f5', '#000000', '#e5e5e5'],
    style: 'Minimal',
    isPro: false,
    fonts: {
      primary: '"Helvetica Neue", "Arial", sans-serif',
      secondary: '"Source Sans Pro", sans-serif',
    },
    spacing: { base: '8px', large: '16px' },
    animations: { speed: 'instant' },
    borders: { radius: '2px', glow: false },
  },
  {
    id: 'zen',
    category: 'minimal',
    name: 'Zen Mode',
    description: 'Ultra minimal with maximum focus',
    colors: ['#4a5568', '#718096', '#e2e8f0', '#ffffff', '#2d3748', '#f7fafc'],
    style: 'Zen',
    isPro: true,
    fonts: {
      primary: '"SF Pro Display", "Inter", sans-serif',
      secondary: '"SF Pro Text", "Roboto", sans-serif',
    },
    spacing: { base: '20px', large: '40px' },
    animations: { speed: 'slow' },
    borders: { radius: '12px', glow: false },
  },
]

/** Get themes filtered by category */
export const getThemesByCategory = (categoryId: string) =>
  availableThemes.filter(t => t.category === categoryId)

/** Find a theme by ID */
export const getThemeById = (id: string) =>
  availableThemes.find(t => t.id === id)
