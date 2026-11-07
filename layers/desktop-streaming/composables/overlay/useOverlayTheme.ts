import { ref, computed } from 'vue'

/**
 * Overlay Theme Composable — single source of truth for theme state.
 *
 * Owns:
 *  - current theme selection + CSS variable map
 *  - applying vars to the canvas DOM
 *  - providing resolved values for OBS sync
 *
 * A default theme is always loaded — var() in element styles never resolve to nothing.
 *
 * Consumers:
 *  - index.vue calls applyTheme / patchVars
 *  - useOverlayV3Sync calls resolveVar to inline values in OBS HTML
 */

export interface ThemeDef {
  id: string
  name: string
  description?: string
  style: string
  colors: string[]
  isPro?: boolean
  fonts?: { primary?: string; secondary?: string }
  borders?: {
    type?: string
    radius?: string
    glow?: boolean
    images?: { frame?: string; corner?: string; pattern?: string; edge?: string }
  }
  spacing?: { base?: string; large?: string }
  animations?: { speed?: string }
  [key: string]: unknown
}

// ── Default theme (always loaded) ───────────────────────────────

const DEFAULT_THEME: ThemeDef = {
  id: 'cyberpunk',
  name: 'Cyberpunk 2077',
  style: 'Neon Futuristic',
  colors: ['#ff00ff', '#00ffff', '#ff1493', '#0a0a0a', '#ffffff', '#1a1a1a'],
}

// ── CSS var generation ──────────────────────────────────────────

function buildCssVars(theme: ThemeDef): Record<string, string> {
  const c = theme.colors
  return {
    // Core palette
    '--theme-primary': c[0],
    '--theme-secondary': c[1],
    '--theme-accent': c[2] || c[1],
    '--theme-bg': c[3] || '#000000',
    '--theme-text': c[4] || '#ffffff',
    '--theme-surface': c[5] || c[3] || '#111111',
    '--theme-muted': c[6] || '#666666',

    // Typography (single quotes for font names — double quotes break style="..." in OBS HTML)
    '--theme-font-primary':
      theme.fonts?.primary ||
      (theme.style === 'Pixel Art' ? "'Courier New', monospace" : "'Inter', sans-serif"),
    '--theme-font-secondary': theme.fonts?.secondary || "'Roboto', sans-serif",

    // Borders & Radius
    '--theme-border': `2px solid ${c[0]}`,
    '--theme-border-thin': `1px solid ${c[0]}`,
    '--theme-border-radius':
      theme.borders?.radius ||
      (theme.style === 'Minimal' ? '8px' : theme.style === 'Pixel Art' ? '0px' : '12px'),
    '--theme-border-radius-sm': theme.borders?.radius
      ? `calc(${theme.borders.radius} / 2)` : '4px',

    // Spacing (used by --c-padding etc.)
    '--theme-spacing-base': theme.spacing?.base || '8px',
    '--theme-spacing-lg': theme.spacing?.large || '16px',

    // Shadows & Effects
    '--theme-shadow': theme.borders?.glow
      ? `0 0 20px ${c[0]}` : '0 4px 12px rgba(0,0,0,0.3)',
    '--theme-shadow-sm': theme.borders?.glow
      ? `0 0 10px ${c[0]}` : '0 2px 4px rgba(0,0,0,0.15)',
    '--theme-glow': theme.borders?.glow
      ? `drop-shadow(0 0 8px ${c[0]})` : 'none',

    // Transitions
    '--theme-transition': theme.animations?.speed === 'instant' ? '0ms' : '250ms',

    // Component helpers
    '--theme-card-bg': c[5] || c[3] || '#111111',
    '--theme-overlay-bg': `${c[3] || '#000000'}CC`,
  }
}

// ── Shared singleton state (persists across composable calls) ───

const _defaultVars = buildCssVars(DEFAULT_THEME)
const currentTheme = ref<ThemeDef & { cssVars: Record<string, string> }>({
  ...DEFAULT_THEME,
  cssVars: _defaultVars,
})

/**
 * Main composable — call from any component / page.
 */
export const useOverlayTheme = () => {
  /** Resolved CSS var map (always up-to-date). */
  const cssVars = computed(() => currentTheme.value.cssVars)

  /** Apply a full theme (from ThemeBrowserModal). */
  const applyTheme = (theme: ThemeDef) => {
    const vars = buildCssVars(theme)
    currentTheme.value = { ...theme, cssVars: vars }
    applyVarsToCanvas(vars)
  }

  /** Patch individual vars (from ThemeSettingsModal color/radius tweaks). */
  const patchVars = (overrides: Record<string, string>) => {
    const merged = { ...currentTheme.value.cssVars, ...overrides }
    currentTheme.value = { ...currentTheme.value, cssVars: merged }
    applyVarsToCanvas(merged)
  }

  /** Apply current vars to the canvas DOM (call on mount). */
  const mountTheme = () => {
    applyVarsToCanvas(currentTheme.value.cssVars)
  }

  /** Resolve var() references to actual values for OBS HTML. */
  const resolveVar = (value: string): string => {
    if (!value || typeof value !== 'string') return value
    const vars = cssVars.value
    return value.replace(/var\(([^)]+)\)/g, (_match, expr) => {
      const [varName, ...rest] = expr.split(',')
      return vars[varName.trim()] || rest.join(',').trim() || expr
    })
  }

  return { currentTheme, cssVars, applyTheme, patchVars, mountTheme, resolveVar }
}

// ── Internal helpers ────────────────────────────────────────────

function applyVarsToCanvas(vars: Record<string, string>) {
  if (typeof document === 'undefined') return
  const canvas = document.querySelector('.canvas-container') as HTMLElement | null
  if (!canvas) return
  for (const [key, value] of Object.entries(vars)) {
    canvas.style.setProperty(key, value)
  }
}
