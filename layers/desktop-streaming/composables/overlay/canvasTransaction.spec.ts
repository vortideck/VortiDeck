import { describe, expect, it, vi } from 'vitest'
import { commitCanvas } from './canvasTransaction'

describe('saved canvas intent', () => {
  const from = { width: 1920, height: 1080 }
  const to = { width: 1080, height: 1920 }
  const elements = [{ x: 0, y: 0, width: 1920, height: 1080 }]
  it('saves portrait geometry before committing editor state, without OBS', async () => {
    const commitEditor = vi.fn()
    const persist = vi.fn(async () => { expect(commitEditor).not.toHaveBeenCalled(); return true })
    await commitCanvas({ from, to, elements, persist, commitEditor })
    expect(commitEditor).toHaveBeenCalledWith([{ x: 0, y: 656.25, width: 1080, height: 607.5 }], to)
    expect(elements[0]?.width).toBe(1920)
  })
  it('leaves editor untouched when native persistence fails', async () => {
    const commitEditor = vi.fn()
    await expect(commitCanvas({ from, to, elements, persist: async () => false, commitEditor })).rejects.toThrow('save')
    expect(commitEditor).not.toHaveBeenCalled()
  })
})
