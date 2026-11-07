import { describe, expect, it } from 'vitest'
import { widgetSettingsBridgeScript, widgetConfigMessage } from './widgetSettingsBridge'

describe('widget settings bridge', () => {
  it('replaces configuration without resetting runtime data, rejects other senders and stale revisions', () => {
    const listeners: Record<string, (event: any) => void> = {}
    const events: CustomEvent[] = []
    const parent = {}
    const frame: any = {
      parent, vortideckData: { current: 42 },
      addEventListener: (name: string, handler: (event: any) => void) => { listeners[name] = handler },
      dispatchEvent: (event: CustomEvent) => events.push(event),
    }
    new Function('window', widgetSettingsBridgeScript)(frame)
    const deliver = (config: Record<string, unknown>, revision: number, source = parent) => listeners.message!({ source, data: widgetConfigMessage(config, revision) })
    deliver({ goal: 100, color: 'red' }, 1)
    deliver({ goal: 200 }, 2)
    deliver({ goal: 10 }, 1)
    deliver({ goal: 999 }, 3, {})
    deliver({ goal: 200 }, 2)
    expect(frame.vortideckConfig).toEqual({ goal: 200 })
    expect(frame.vortideckData.current).toBe(42)
    expect(events).toHaveLength(2)
    expect(events[1]?.type).toBe('vortideck-config-update')
  })
})
