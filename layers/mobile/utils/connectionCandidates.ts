export type ConnectionTransport = 'lan' | 'tailscale'

export interface ConnectionCandidate {
  transport: ConnectionTransport
  host: string
  port: number
  path: '/ws'
  secure: boolean
  priority: number
}

export interface CandidateProbeSocket {
  onopen: (() => void) | null
  onerror: (() => void) | null
  close: () => void
}

export interface CandidateProbeOptions {
  timeoutMs?: number
  staggerMs?: number
  createSocket?: (url: string) => CandidateProbeSocket
}

export interface ReachableConnectionCandidate {
  candidate: ConnectionCandidate
  latencyMs: number
}

const HOSTNAME_PATTERN = /^(?=.{1,253}$)(?:[a-z\d](?:[a-z\d-]{0,61}[a-z\d])?\.)*[a-z\d](?:[a-z\d-]{0,61}[a-z\d])?$/i
const IPV6_PATTERN = /^[0-9a-f:]+$/i

function isValidHost(host: unknown): host is string {
  return typeof host === 'string'
    && host.length > 0
    && host.length <= 253
    && (HOSTNAME_PATTERN.test(host) || (host.includes(':') && IPV6_PATTERN.test(host)))
}

export function normalizeConnectionCandidates(value: unknown): ConnectionCandidate[] {
  if (!Array.isArray(value)) return []
  const seen = new Set<string>()
  return value
    .slice(0, 8)
    .filter((candidate): candidate is Record<string, unknown> => !!candidate && typeof candidate === 'object')
    .filter((candidate) => candidate.transport === 'lan' || candidate.transport === 'tailscale')
    .filter((candidate) => isValidHost(candidate.host))
    .filter((candidate) => Number.isInteger(candidate.port) && Number(candidate.port) > 0 && Number(candidate.port) <= 65535)
    .filter((candidate) => candidate.path === '/ws' && typeof candidate.secure === 'boolean')
    .filter((candidate) => Number.isInteger(candidate.priority) && Number(candidate.priority) >= 0 && Number(candidate.priority) <= 1000)
    .map((candidate) => ({
      transport: candidate.transport as ConnectionTransport,
      host: candidate.host as string,
      port: candidate.port as number,
      path: '/ws' as const,
      secure: candidate.secure as boolean,
      priority: candidate.priority as number,
    }))
    .sort((left, right) => left.priority - right.priority)
    .filter((candidate) => {
      const identity = `${candidate.transport}|${candidate.host.toLowerCase()}|${candidate.port}|${candidate.secure}`
      if (seen.has(identity)) return false
      seen.add(identity)
      return true
    })
}

export function connectionCandidateUrl(candidate: ConnectionCandidate): string {
  const scheme = candidate.secure ? 'wss' : 'ws'
  const host = candidate.host.includes(':') && !candidate.host.startsWith('[')
    ? `[${candidate.host}]`
    : candidate.host
  return `${scheme}://${host}:${candidate.port}${candidate.path}`
}

/**
 * Try advertised private-network endpoints in priority groups. Lower-priority
 * groups start shortly after the preferred group instead of waiting for a full
 * timeout (the same principle as Happy Eyeballs). The real connection still
 * performs normal VortiDeck authentication immediately afterwards.
 */
export function findReachableConnectionCandidate(
  candidates: ConnectionCandidate[],
  options: CandidateProbeOptions = {},
): Promise<ReachableConnectionCandidate | null> {
  if (!candidates.length) return Promise.resolve(null)

  const timeoutMs = options.timeoutMs ?? 1800
  const staggerMs = options.staggerMs ?? 120
  const createSocket: (url: string) => CandidateProbeSocket = options.createSocket
    ?? ((url: string) => new WebSocket(url) as unknown as CandidateProbeSocket)
  const orderedCandidates = [...candidates].sort((left, right) => left.priority - right.priority)
  const priorityGroups = [...new Set(orderedCandidates.map(candidate => candidate.priority))]
  const delayFor = (candidate: ConnectionCandidate) => (
    priorityGroups.indexOf(candidate.priority) * staggerMs
  )

  return new Promise((resolve) => {
    const sockets = new Map<CandidateProbeSocket, ConnectionCandidate>()
    const launchTimers: ReturnType<typeof setTimeout>[] = []
    const failedSockets = new Set<CandidateProbeSocket>()
    const launchedAt = new Map<CandidateProbeSocket, number>()
    let settled = false
    let remaining = orderedCandidates.length

    const finish = (reachable: ReachableConnectionCandidate | null) => {
      if (settled) return
      settled = true
      clearTimeout(timer)
      for (const launchTimer of launchTimers) clearTimeout(launchTimer)
      for (const socket of sockets.keys()) socket.close()
      resolve(reachable)
    }

    const failed = (socket?: CandidateProbeSocket) => {
      if (socket) {
        if (failedSockets.has(socket)) return
        failedSockets.add(socket)
      }
      remaining -= 1
      if (remaining === 0) finish(null)
    }

    const timer = setTimeout(() => finish(null), timeoutMs)
    const launch = (candidate: ConnectionCandidate) => {
      if (settled) return
      try {
        const socket = createSocket(connectionCandidateUrl(candidate))
        sockets.set(socket, candidate)
        launchedAt.set(socket, Date.now())
        socket.onopen = () => finish({
          candidate,
          latencyMs: Math.max(0, Date.now() - (launchedAt.get(socket) ?? Date.now())),
        })
        socket.onerror = () => failed(socket)
      } catch {
        failed()
      }
    }

    for (const candidate of orderedCandidates) {
      const delay = delayFor(candidate)
      if (delay === 0) launch(candidate)
      else launchTimers.push(setTimeout(() => launch(candidate), delay))
    }
  })
}
