import { reactive } from 'vue'
import { describe, expect, it } from 'vitest'
import { copyWidgetCapabilities } from './packageManifest'

describe('package manifest helpers', () => {
  it('copies reactive widget capabilities into detached plain data', () => {
    const source = reactive({
      subscribe: ['follow'],
      actions: ['triggerFlow:welcome'],
      ui: true,
      network: { whitelist: ['https://events.example'] },
    })

    const copied = copyWidgetCapabilities(source)

    expect(copied).toEqual({
      subscribe: ['follow'],
      actions: ['triggerFlow:welcome'],
      ui: true,
      network: { whitelist: ['https://events.example'] },
    })
    expect(copied.subscribe).not.toBe(source.subscribe)
    expect(copied.actions).not.toBe(source.actions)
    expect(copied.network).not.toBe(source.network)
    expect(() => structuredClone(copied)).not.toThrow()
  })
})
