import { describe, expect, it } from 'vitest'
import { getComponentSchema } from './componentSchema'

function goal(rehearsal = false) {
  const frame = document.createElement('iframe')
  document.body.appendChild(frame)
  const window = frame.contentWindow! as Window & typeof globalThis
  const template = getComponentSchema('goal')!.template
  window.document.body.innerHTML = template
  const script = window.document.querySelector('script')!
  const globals = window as unknown as Record<string, unknown>
  globals.vortideckRehearsal = rehearsal
  globals.vortideckData = { current: 10, goal: 100, goalType: 'followers', valueSource: 'observed', platform: 'twitch', accountId: '7' }
  Object.defineProperty(window.document, 'currentScript', { configurable: true, value: script })
  window.eval(script.textContent!)
  return {
    window,
    text: () => window.document.querySelector('.goal-value')!.textContent,
    send: (type: string, detail: unknown) => window.dispatchEvent(new window.CustomEvent(type, { detail })),
    close: () => frame.remove(),
  }
}

describe('observed goal values', () => {
  it('hydrates one account, ignores live-event increments, and preserves values across settings changes and iframe replacement', () => {
    const first = goal()
    const snapshot = [{ metric: 'followers', platform: 'twitch', accountId: 7, total: 5 }, { metric: 'followers', platform: 'twitch', accountId: 8, total: 80 }]
    try {
      first.send('vortideck-stream-values', snapshot)
      expect(first.text()).toBe('15 / 100')
      first.send('vortideck-stream-event', { type: 'follow' })
      first.send('vortideck-stream-event', { type: 'follow' })
      expect(first.text()).toBe('15 / 100')
      first.send('vortideck-data-update', { label: 'New title', goal: 200 })
      expect(first.text()).toBe('15 / 200')
      const replaced = goal()
      try { replaced.send('vortideck-stream-values', snapshot); expect(replaced.text()).toBe('15 / 100') }
      finally { replaced.close() }
    } finally { first.close() }
  })

  it('keeps rehearsal increments independent of real counts', () => {
    const live = goal(); const preview = goal(true)
    try {
      preview.send('vortideck-stream-event', { type: 'follow' })
      expect(preview.text()).toBe('11 / 100')
      expect(live.text()).toBe('10 / 100')
    } finally { live.close(); preview.close() }
  })
})
