import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ref } from 'vue'
import { useOverlayV3Sync } from './useOverlayV3Sync'

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }))
vi.mock('@tauri-apps/api/core', () => ({ invoke }))
vi.mock('./useOverlayTheme', () => ({ useOverlayTheme: () => ({ resolveVar: (value: string) => value, cssVars: ref({}) }) }))
vi.mock('./componentSchema', () => ({ getComponentSchema: () => undefined, buildComponentVars: () => ({}) }))

describe('overlay widget settings sync', () => {
  beforeEach(() => { invoke.mockReset() })
  const element = () => ({ id: 'goal-1', widgetId: 'goals', contributionId: 'follower-goal', trust: 'community',
    customHTML: '<strong>Goal</strong><script>window.counter = 42;</script>', width: 360, height: 112,
    config: { goal: 100 }, configRevision: 1, capabilities: { subscribe: [], actions: [], network: 'none' },
  })

  it('settings edits do not change iframe HTML', async () => {
    invoke.mockResolvedValue([{ id: 'goals', is_granted: true }])
    const sync = useOverlayV3Sync()
    await sync.refreshWidgetGrants()
    const widget = element()
    const before = sync.generateElementHTML(widget)
    expect(before).toContain('UPDATE_CONFIG')
    expect(before).toContain('vortideck-config-update')
    widget.config = { goal: 200 }
    widget.configRevision = 2
    expect(sync.generateElementHTML(widget)).toBe(before)
  })

  it('preserves hidden instances and recovers a newer host config before syncing cached projects', async () => {
    invoke.mockImplementation(async (command: string) => {
      if (command === 'list_packages') return [{ id: 'goals', is_granted: true }]
      if (command === 'get_overlay_widget_settings') return { packageId: 'goals', contributionId: 'follower-goal', config: { goal: 500 }, configRevision: 4 }
      return null
    })
    const widget = { ...element(), visible: false }
    expect(await useOverlayV3Sync().syncElementsToBackend([widget], { width: 1920, height: 1080 })).toBe(true)
    const args = invoke.mock.calls.find(call => call[0] === 'batch_sync_overlay')?.[1]
    const batch = args.elements
    expect(args.canvas).toEqual({ width: 1920, height: 1080 })
    expect(batch).toHaveLength(1)
    expect(batch[0]).toMatchObject({ visible: false, contributionId: 'follower-goal', config: { goal: 500 }, configRevision: 4 })
    expect(widget.config).toEqual({ goal: 500 })
  })

  it('single-add carries the same package, contribution and settings metadata', async () => {
    invoke.mockImplementation(async (command: string) => command === 'list_packages'
      ? [{ id: 'goals', is_granted: true }]
      : command === 'get_overlay_widget_settings'
        ? { config: { goal: 100, title: 'Followers' }, configRevision: 0 } : null)
    const widget = element()
    await useOverlayV3Sync().syncElementAdd(widget)
    const add = invoke.mock.calls.find(call => call[0] === 'add_overlay_element_v3')?.[1]
    expect(add.metadata).toMatchObject({ packageId: 'goals', contributionId: 'follower-goal', config: { goal: 100 } })
    expect(widget.config).toEqual({ goal: 100, title: 'Followers' })
  })

  it('refreshes consent before saving HTML after an editor-open grant or re-grant', async () => {
    let granted = false
    invoke.mockImplementation(async (command: string) => {
      if (command === 'list_packages') return [{ id: 'goals', is_granted: granted }]
      if (command === 'get_overlay_widget_settings') return { config: { goal: 100 }, configRevision: 0 }
      return null
    })
    const sync = useOverlayV3Sync()
    const widget = element()
    await sync.refreshWidgetGrants()
    expect(sync.generateElementHTML(widget)).toContain('Widget access revoked')
    granted = true
    await sync.syncElementAdd(widget)
    const add = invoke.mock.calls.find(call => call[0] === 'add_overlay_element_v3')?.[1]
    expect(add.html).not.toContain('Widget access revoked')
    expect(add.html).toContain('window.counter = 42')
    granted = false
    await sync.syncElementsToBackend([widget])
    expect(sync.generateElementHTML(widget)).toContain('Widget access revoked')
    granted = true
    await sync.syncElementsToBackend([widget])
    expect(sync.generateElementHTML(widget)).not.toContain('Widget access revoked')
  })

  it('does not overwrite saved output with a revoked placeholder when grant lookup fails', async () => {
    invoke.mockRejectedValue(new Error('permission lookup unavailable'))
    expect(await useOverlayV3Sync().syncElementsToBackend([element()])).toBe(false)
    expect(invoke.mock.calls.some(call => call[0] === 'batch_sync_overlay')).toBe(false)
  })
})
