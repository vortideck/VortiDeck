import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { ref } from 'vue'
import { useOBSIntegration } from './useOBSIntegration'

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  listen: vi.fn(),
  listeners: new Map<string, (event: { payload: unknown }) => void>(),
}))

vi.mock('@tauri-apps/api/core', () => ({ invoke: mocks.invoke }))
vi.mock('@tauri-apps/api/event', () => ({ listen: mocks.listen }))

describe('OBS canvas Apply correlation', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    mocks.invoke.mockReset()
    mocks.listen.mockReset()
    mocks.listeners.clear()
    mocks.listen.mockImplementation(async (name: string, callback: (event: { payload: unknown }) => void) => {
      mocks.listeners.set(name, callback)
      return () => mocks.listeners.delete(name)
    })
    mocks.invoke.mockImplementation(async (command: string) => command === 'get_integration_status'
      ? { status: 'Connected' }
      : undefined)
    vi.stubGlobal('crypto', { randomUUID: () => 'apply-1' })
  })

  afterEach(() => { vi.clearAllTimers(); vi.useRealTimers() })

  it('initializes listeners once and resolves only the matching confirmed readback', async () => {
    const integration = useOBSIntegration()
    await Promise.all([integration.initialize(), integration.initialize()])
    expect(mocks.listen).toHaveBeenCalledTimes(3)

    const applying = integration.applyCanvasSize(2560, 1440)
    await Promise.resolve()
    expect(mocks.invoke).toHaveBeenCalledWith('apply_obs_canvas_size', {
      width: 2560,
      height: 1440,
      requestId: 'apply-1',
    })
    mocks.listeners.get('obs-canvas-apply-result')!({ payload: {
      type: 'OBS_CANVAS_APPLY_RESULT', request_id: 'stale', status: 'applied', width: 1920, height: 1080,
    } })
    mocks.listeners.get('obs-canvas-apply-result')!({ payload: {
      type: 'OBS_CANVAS_APPLY_RESULT', request_id: 'apply-1', status: 'applied', width: 2560, height: 1440,
      managed_source_found: true, managed_source_matches: true,
    } })
    await expect(applying).resolves.toMatchObject({ request_id: 'apply-1', width: 2560, height: 1440 })
    expect(integration.obsSyncStatus.value).toBe('synced')

    mocks.listeners.get('obs-canvas-size-changed')!({ payload: {
      width: 2560, height: 1440, synced_at: Date.now(), managed_source_found: true, managed_source_matches: true,
    } })
    expect(integration.obsSyncStatus.value).toBe('synced')

    mocks.listeners.get('obs-canvas-size-changed')!({ payload: {
      width: 1920, height: 1080, synced_at: Date.now(),
    } })
    expect(integration.obsSyncStatus.value).toBe('observed')
  })

  it('propagates an active-output rejection without pretending the canvas is synced', async () => {
    const integration = useOBSIntegration()
    await integration.initialize()
    const applying = integration.applyCanvasSize(1080, 1920)
    await Promise.resolve()
    mocks.listeners.get('obs-canvas-apply-result')!({ payload: {
      type: 'OBS_CANVAS_APPLY_RESULT', request_id: 'apply-1', status: 'rejected',
      error_code: 'output_active', error: 'Stop active outputs first',
    } })
    await expect(applying).rejects.toThrow('Stop active outputs first')
    expect(integration.obsSyncStatus.value).toBe('error')
  })
  it('settles command transport failures and permits a later retry', async () => {
    const integration = useOBSIntegration()
    mocks.invoke.mockRejectedValueOnce(new Error('Disconnected'))
    await expect(integration.applyCanvasSize(1920, 1080)).rejects.toThrow('Disconnected')
    expect(integration.obsSyncStatus.value).toBe('error')
    integration.cleanup()
  })

  it('does not claim agreement when the managed browser source is missing', async () => {
    const integration = useOBSIntegration(() => ({ width: 1920, height: 1080 }))
    await integration.initialize()
    mocks.listeners.get('obs-canvas-size-changed')!({ payload: {
      width: 1920, height: 1080, synced_at: Date.now(), managed_source_found: false, managed_source_matches: false,
    } })
    expect(integration.obsSyncStatus.value).toBe('observed')
    integration.cleanup()
  })

  it('clears the query timeout when readback arrives before invoke resolves', async () => {
    const integration = useOBSIntegration()
    await integration.initialize()
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === 'request_obs_canvas_size') {
        mocks.listeners.get('obs-canvas-size-changed')!({ payload: {
          width: 1920, height: 1080, managed_source_found: true, managed_source_matches: true,
        } })
      }
    })
    await integration.syncWithObs()
    expect(integration.obsSyncStatus.value).toBe('synced')
    expect(vi.getTimerCount()).toBe(0)
    integration.cleanup()
  })

  it('updates source-only confirmation and active-output state without changing dimensions', async () => {
    const desired = ref({ width: 1920, height: 1080 })
    const integration = useOBSIntegration(() => desired.value)
    await integration.initialize()
    const observe = mocks.listeners.get('obs-canvas-size-changed')!
    observe({ payload: { width: 1920, height: 1080, streaming: true, recording: true, managed_source_found: true, managed_source_matches: true } })
    expect(integration.obsSyncStatus.value).toBe('synced')
    expect(integration.syncStatusText.value).toBe('Canvas and OBS source sizes match')
    expect(integration.isStreaming.value).toBe(true)
    observe({ payload: { width: 1920, height: 1080, managed_source_found: false, managed_source_matches: false } })
    expect(integration.obsSyncStatus.value).toBe('observed')
    observe({ payload: { width: 1920, height: 1080, managed_source_found: true, managed_source_matches: true } })
    desired.value = { width: 1080, height: 1920 }
    expect(integration.obsSyncStatus.value).toBe('observed')
    integration.cleanup()
  })

  it('rejects mismatching readback even when OBS reports applied', async () => {
    const integration = useOBSIntegration()
    await integration.initialize()
    const applying = integration.applyCanvasSize(1080, 1920)
    mocks.listeners.get('obs-canvas-apply-result')!({ payload: {
      request_id: 'apply-1', status: 'applied', width: 1920, height: 1080,
      managed_source_found: true, managed_source_matches: true,
    } })
    await expect(applying).rejects.toThrow('readback does not match')
    expect(integration.obsSyncStatus.value).toBe('error')
    integration.cleanup()
  })

  it('rejects pending Apply on disconnect and ignores late source observations', async () => {
    const integration = useOBSIntegration()
    await integration.initialize()
    const applying = integration.applyCanvasSize(1080, 1920)
    mocks.listeners.get('obs_plugin_status')!({ payload: false })
    await expect(applying).rejects.toThrow('OBS disconnected')
    mocks.listeners.get('obs-canvas-size-changed')!({ payload: {
      width: 1080, height: 1920, managed_source_found: true, managed_source_matches: true,
    } })
    expect(integration.obsSyncStatus.value).toBe('disconnected')
    expect(integration.canvasSize.value).toBeNull()
    integration.cleanup()
  })

})
