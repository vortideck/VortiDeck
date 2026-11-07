/**
 * Element-Level CSS Variable Bridge
 *
 * Maps element-scoped CSS variables (--el-*) to theme defaults.
 * customHTML uses var(--el-*) instead of var(--theme-*) directly,
 * so the Properties panel can override them per-element.
 *
 * Canvas: --el-* vars are set on the wrapper div, cascade into v-html children.
 * OBS:    generateElementHTML resolves --el-* → concrete values before sending.
 */

/** Each --el-* variable and its default (a theme variable reference). */
export const ELEMENT_VAR_DEFAULTS: Record<string, string> = {
  '--el-bg': 'var(--theme-card-bg)',
  '--el-bg-alt': 'var(--theme-surface)',
  '--el-text': 'var(--theme-text)',
  '--el-text-muted': 'var(--theme-muted)',
  '--el-accent': 'var(--theme-accent)',
  '--el-primary': 'var(--theme-primary)',
  '--el-secondary': 'var(--theme-secondary)',
  '--el-font': 'var(--theme-font-primary)',
  '--el-font-alt': 'var(--theme-font-secondary)',
  '--el-radius': 'var(--theme-border-radius)',
  '--el-radius-sm': 'var(--theme-border-radius-sm)',
  '--el-border': 'var(--theme-border-thin)',
  '--el-border-heavy': 'var(--theme-border)',
  '--el-shadow': 'var(--theme-shadow)',
  '--el-shadow-sm': 'var(--theme-shadow-sm)',
}

/**
 * Maps Properties-panel style keys → --el-* variable names.
 * When the user sets a concrete value (e.g. backgroundColor = '#ff0000'),
 * the corresponding --el-* var is overridden on the wrapper.
 */
export const STYLE_TO_EL_VAR: Record<string, string> = {
  backgroundColor: '--el-bg',
  color: '--el-text',
  fontFamily: '--el-font',
  borderRadius: '--el-radius',
}

/**
 * Build the --el-* CSS custom properties for an element's wrapper div.
 * Starts from defaults, overrides with any concrete values from element.style.
 */
export function buildElementVars(elementStyle?: Record<string, any>): Record<string, string> {
  const vars: Record<string, string> = { ...ELEMENT_VAR_DEFAULTS }

  if (!elementStyle) return vars

  // Override from concrete style values (non-var values set via Properties panel)
  for (const [styleProp, elVar] of Object.entries(STYLE_TO_EL_VAR)) {
    const val = elementStyle[styleProp]
    if (val && typeof val === 'string' && !val.startsWith('var(')) {
      vars[elVar] = val
    }
  }

  // Pick up any --el-* vars set directly on the element style
  for (const [key, val] of Object.entries(elementStyle)) {
    if (key.startsWith('--el-') && val) {
      vars[key] = String(val)
    }
  }

  return vars
}
