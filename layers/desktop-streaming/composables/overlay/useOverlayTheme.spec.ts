import { beforeEach, describe, expect, it } from 'vitest'
import { availableThemes } from './themeRegistry'
import { useOverlayTheme } from './useOverlayTheme'

describe('overlay theme state', () => {
  beforeEach(() => {
    document.body.innerHTML = '<div class="canvas-container"></div>'
    useOverlayTheme().applyTheme(availableThemes[0])
  })

  it('uses a catalog identity for the default theme', () => {
    const { currentTheme } = useOverlayTheme()
    expect(availableThemes.some(theme => theme.id === currentTheme.value.id)).toBe(true)
  })

  it('applies a selected catalog theme to shared state and the canvas', () => {
    const selected = availableThemes.find(theme => theme.id === 'retro')
    expect(selected).toBeDefined()

    const firstConsumer = useOverlayTheme()
    const secondConsumer = useOverlayTheme()
    firstConsumer.applyTheme(selected!)

    expect(secondConsumer.currentTheme.value.id).toBe('retro')
    expect(document.querySelector<HTMLElement>('.canvas-container')?.style
      .getPropertyValue('--theme-primary')).toBe(selected!.colors[0])
  })
})
