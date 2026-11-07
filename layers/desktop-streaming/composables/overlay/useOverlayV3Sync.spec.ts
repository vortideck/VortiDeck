import { describe, expect, it, vi } from 'vitest'

import { componentRegistry } from './componentSchema'
import {
  cssVarDeclarations,
  elementNeedsSandbox,
  isSafeCssVar,
  isSafeElementId,
  safeObjectFit,
  useOverlayV3Sync,
} from './useOverlayV3Sync'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))

const BREAKOUT = 'red}</style><img src=x onerror="__TAURI_INTERNALS__.invoke(1)">'

describe('CSS custom-property sanitizer', () => {
  it('accepts ordinary theme and component values', () => {
    expect(isSafeCssVar('--theme-primary', '#ff1493')).toBe(true)
    expect(isSafeCssVar('--c_bg', 'linear-gradient(135deg, var(--theme-accent), #fff)')).toBe(true)
    expect(isSafeCssVar('--c-font', '"Inter", sans-serif')).toBe(true)
    expect(isSafeCssVar('--c-size', 12)).toBe(true)
  })

  it('rejects values and names that can leave the declaration', () => {
    for (const value of [BREAKOUT, 'red; color: blue', 'red /* x', '\\3c', 'a{', 'a}', '<b>', 'a>b']) {
      expect(isSafeCssVar('--x', value), value).toBe(false)
    }
    for (const name of ['color', '--', '--x}body{', '--x:y', '--x y', '-x']) {
      expect(isSafeCssVar(name, 'red'), name).toBe(false)
    }
  })

  it('drops unsafe pairs when serializing declarations', () => {
    const css = cssVarDeclarations({ '--ok': '#123', '--bad': BREAKOUT, '--x}</style>': 'red', '--font': '"Inter"' })
    expect(css).toBe("--ok: #123; --font: 'Inter'")
    expect(cssVarDeclarations({ '--a': '1', '--b': '2' }, ';', ':')).toBe('--a:1;--b:2')
  })

  it('validates element ids and image object-fit', () => {
    expect(isSafeElementId('element-1_A')).toBe(true)
    expect(isSafeElementId('x{}</style>')).toBe(false)
    expect(isSafeElementId('')).toBe(false)
    expect(safeObjectFit('contain')).toBe('contain')
    expect(safeObjectFit('cover;"><script>')).toBe('cover')
    expect(safeObjectFit(undefined)).toBe('cover')
  })
})

describe('host v-html sandbox decision', () => {
  const schema = componentRegistry.goal
  const scriptless = Object.values(componentRegistry).find(s => s.template && !s.template.includes('<script'))

  it('isolates community and verified widgets regardless of markup', () => {
    expect(elementNeedsSandbox({ trust: 'community', name: 'x' })).toBe(true)
    expect(elementNeedsSandbox({ trust: 'verified', src: 'https://a/b.png' })).toBe(true)
  })

  it('isolates element-supplied markup even without a script tag', () => {
    const payload = '<img src=x onerror="__TAURI_INTERNALS__.invoke(\'x\')">'
    expect(elementNeedsSandbox({ trust: 'local', customHTML: payload })).toBe(true)
    expect(elementNeedsSandbox({ customHTML: payload })).toBe(true)
    expect(elementNeedsSandbox({ trust: 'builtin', innerHTML: payload })).toBe(true)
    expect(elementNeedsSandbox({ trust: 'builtin', content: payload })).toBe(true)
    expect(elementNeedsSandbox({ type: 'modular', template: payload, bindings: {} }, scriptless)).toBe(true)
  })

  it('isolates first-party templates on non-builtin elements', () => {
    expect(scriptless).toBeDefined()
    expect(elementNeedsSandbox({ trust: 'local', schemaId: scriptless!.id }, scriptless)).toBe(true)
  })

  it('keeps builtin first-party templates and plain elements in the host', () => {
    expect(elementNeedsSandbox({ schemaId: scriptless!.id, customHTML: scriptless!.template }, scriptless)).toBe(false)
    expect(elementNeedsSandbox({ trust: 'builtin', content: 'Welcome to My Stream!' })).toBe(false)
    expect(elementNeedsSandbox({ trust: 'local', src: 'https://a/b.png', name: 'x' })).toBe(false)
    expect(elementNeedsSandbox({ trust: 'builtin', schemaId: 'goal' }, schema)).toBe(schema!.template!.includes('<script'))
  })
})

describe('published element HTML', () => {
  it('never emits CSS values or ids that close the style element', () => {
    const { generateElementHTML } = useOverlayV3Sync()
    const html = generateElementHTML({
      id: 'el-1',
      type: 'custom',
      style: { '--c-bg': BREAKOUT, '--c-text': '#fff' },
      content: 'hello',
    })
    expect(html).toContain('<style>#el-1{--c-text:#fff}</style>')
    expect(html).not.toContain('onerror')

    const badId = generateElementHTML({ id: 'x{}</style><b>', type: 'custom', style: { '--c-text': '#fff' } })
    expect(badId).not.toContain('<style>#')
  })

  it('escapes image fallback attributes and validates object-fit', () => {
    const { generateElementHTML } = useOverlayV3Sync()
    const html = generateElementHTML({ id: 'img-1', type: 'image', src: 'x" onerror="alert(1)', name: '<b>', objectFit: 'cover;x' })
    expect(html).not.toContain('onerror="alert(1)')
    expect(html).not.toContain('onerror=&quot;alert(1)')
    expect(html).toContain('object-fit: cover;')
  })
})
