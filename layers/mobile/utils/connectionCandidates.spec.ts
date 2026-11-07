import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  connectionCandidateUrl,
  findReachableConnectionCandidate,
  normalizeConnectionCandidates,
  type CandidateProbeSocket,
} from './connectionCandidates'

afterEach(() => {
  vi.useRealTimers()
})

describe('connection candidate contract', () => {
  it('sorts, validates, and deduplicates candidates', () => {
    const result = normalizeConnectionCandidates([
      { transport: 'tailscale', host: 'studio.example.ts.net', port: 9001, path: '/ws', secure: false, priority: 30 },
      { transport: 'lan', host: '192.168.1.8', port: 9012, path: '/ws', secure: false, priority: 10 },
      { transport: 'lan', host: '192.168.1.8', port: 9012, path: '/ws', secure: false, priority: 50 },
      { transport: 'tailscale', host: 'bad host', port: 9001, path: '/ws', secure: false, priority: 20 },
      { transport: 'internet', host: 'example.com', port: 443, path: '/ws', secure: true, priority: 1 },
    ])

    expect(result.map(candidate => candidate.transport)).toEqual(['lan', 'tailscale'])
    expect(connectionCandidateUrl(result[0]!)).toBe('ws://192.168.1.8:9012/ws')
  })

  it('brackets IPv6 addresses in WebSocket URLs', () => {
    const [candidate] = normalizeConnectionCandidates([
      { transport: 'tailscale', host: 'fd7a:115c:a1e0::1', port: 9001, path: '/ws', secure: false, priority: 20 },
    ])

    expect(connectionCandidateUrl(candidate!)).toBe('ws://[fd7a:115c:a1e0::1]:9001/ws')
  })

  it('staggers priority groups and returns the first reachable candidate', async () => {
    vi.useFakeTimers()
    const candidates = normalizeConnectionCandidates([
      { transport: 'tailscale', host: 'studio.example.ts.net', port: 9001, path: '/ws', secure: false, priority: 30 },
      { transport: 'tailscale', host: '100.64.0.8', port: 9001, path: '/ws', secure: false, priority: 20 },
      { transport: 'lan', host: '192.168.1.8', port: 9001, path: '/ws', secure: false, priority: 10 },
    ])
    const launched: Array<{ url: string; socket: CandidateProbeSocket }> = []
    const result = findReachableConnectionCandidate(candidates, {
      staggerMs: 100,
      createSocket: (url) => {
        const socket: CandidateProbeSocket = { onopen: null, onerror: null, close: vi.fn() }
        launched.push({ url, socket })
        return socket
      },
    })

    expect(launched.map(item => item.url)).toEqual(['ws://192.168.1.8:9001/ws'])
    await vi.advanceTimersByTimeAsync(100)
    expect(launched.map(item => item.url)).toEqual([
      'ws://192.168.1.8:9001/ws',
      'ws://100.64.0.8:9001/ws',
    ])
    launched[1]!.socket.onopen?.()

    await expect(result).resolves.toEqual({ candidate: candidates[1], latencyMs: 0 })
    expect(launched).toHaveLength(2)
  })

  it('returns null after the global probe timeout', async () => {
    vi.useFakeTimers()
    const candidates = normalizeConnectionCandidates([
      { transport: 'lan', host: '192.168.1.8', port: 9001, path: '/ws', secure: false, priority: 10 },
      { transport: 'tailscale', host: '100.64.0.8', port: 9001, path: '/ws', secure: false, priority: 20 },
    ])
    const result = findReachableConnectionCandidate(candidates, {
      timeoutMs: 500,
      createSocket: () => ({ onopen: null, onerror: null, close: vi.fn() }),
    })

    await vi.advanceTimersByTimeAsync(500)
    await expect(result).resolves.toBeNull()
  })
})
