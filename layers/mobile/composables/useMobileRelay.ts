import { useAuthStore } from '../stores/Auth'
import { useConnectWs } from './useConnectWs'
import { useSecureTokens } from './useSecureTokens'
import { fetch as tauriFetch } from '@tauri-apps/plugin-http'

/**
 * Mobile Relay Connection with JWT Authentication
 *
 * Correct Flow:
 * 1. Exchange the account JWT for a single-use WebSocket ticket
 * 2. Connect to /ws/relay/mobile/?ticket=...
 * 3. Receive available_desktops list from server
 * 4. Send connect_to_desktop with selected desktop_id
 * 5. Send and receive signed relay envelopes
 */
export const useMobileRelay = () => {
  const config = useRuntimeConfig()
  const authStore = useAuthStore()
  const { getRefreshToken, storeTokens } = useSecureTokens()

  /**
   * Refresh the access token if needed
   * @returns Fresh access token
   */
  const refreshAccessToken = async (): Promise<string> => {
    try {
      console.log('🔄 Refreshing access token...')

      const refreshToken = await getRefreshToken()
      if (!refreshToken) {
        throw new Error('No refresh token available')
      }

      console.log('🔍 Calling refresh endpoint:', `${config.public.API_URL}/api/v1/auth/token/refresh`)

      // Request new access token
      const refreshResponse = await tauriFetch(
        `${config.public.API_URL}/api/v1/auth/token/refresh`,
        {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            'x-client-Type': 'native'  // Match desktop app - identifies native client
          },
          body: JSON.stringify({
            refresh: refreshToken
          })
        }
      )

      console.log('🔍 Refresh response status:', refreshResponse.status)

      if (!refreshResponse.ok) {
        console.error(`❌ Token refresh failed with status: ${refreshResponse.status}`)

        // 401/403 means refresh token is invalid/expired - need to logout
        if (refreshResponse.status === 401 || refreshResponse.status === 403) {
          throw new Error('REFRESH_TOKEN_EXPIRED')
        }

        throw new Error(`Token refresh failed with status ${refreshResponse.status}`)
      }

      const refreshData = await refreshResponse.json()
      console.log('🔍 Response keys:', Object.keys(refreshData))

      // Backend returns access_token and refresh_token (with underscores)
      const newAccessToken = refreshData.access_token || refreshData.access
      const newRefreshToken = refreshData.refresh_token || refreshData.refresh

      console.log('🔍 New access token exists:', !!newAccessToken)
      console.log('🔍 New refresh token exists:', !!newRefreshToken)

      if (!newAccessToken) {
        console.error('❌ No access token in refresh response')
        throw new Error('No access token in refresh response')
      }

      // Store new tokens
      if (newRefreshToken) {
        await storeTokens(newAccessToken, newRefreshToken)
      }

      // Update Pinia
      authStore.setAccessToken(newAccessToken)

      console.log('✅ Token refreshed successfully')
      return newAccessToken
    } catch (error) {
      console.error('❌ Token refresh failed:', error)
      if (error instanceof Error && error.message === 'REFRESH_TOKEN_EXPIRED') {
        throw error
      }
      throw new Error('TOKEN_REFRESH_FAILED')
    }
  }

  /**
   * Connect to backend relay server and specific desktop
   * This is the unified relay connection method
   *
   * @param desktopId - Optional desktop ID to connect to directly
   * @returns WebSocket connection
   */
  const connectToRelay = async (desktopId?: string) => {
    try {
      // Try to refresh token proactively to avoid 403 on WebSocket
      const accessToken = authStore.getAccessToken

      if (!accessToken) {
        console.error('❌ Not logged in - cannot connect to relay')
        throw new Error('USER_NOT_LOGGED_IN')
      }

      // Proactively refresh the token to ensure it's valid
      try {
        console.log('🔄 Refreshing token before relay connection...')
        await refreshAccessToken()
        console.log('✅ Using refreshed token for relay connection')
      } catch (error) {
        console.error('❌ Token refresh failed:', error)

        // If refresh token is expired, automatic logout is required
        if (error instanceof Error && error.message === 'REFRESH_TOKEN_EXPIRED') {
          console.error('🔐 Refresh token expired - automatic logout required')
          throw new Error('TOKENS_EXPIRED')
        }

        // For other errors, still throw to prevent connection with potentially invalid token
        throw error
      }

      const wsUrl = String(config.public.WS_URL || '').replace(/\/$/, '')
      if (!/^wss?:\/\//.test(wsUrl)) {
        throw new Error('RELAY_WS_URL_NOT_CONFIGURED')
      }

      const relayEndpoint = `${wsUrl}/ws/relay/mobile/`
      const requestRelayUrl = async () => {
        let currentAccessToken = authStore.getAccessToken
        if (!currentAccessToken) throw new Error('USER_NOT_LOGGED_IN')

        const requestTicket = (token: string) => tauriFetch(
          `${String(config.public.API_URL).replace(/\/$/, '')}/api/v1/auth/ws-ticket`,
          {
            method: 'POST',
            headers: {
              'Authorization': `Token ${token}`,
              'Content-Type': 'application/json',
              'x-client-Type': 'native',
            },
            body: JSON.stringify({ audience: 'relay' }),
          },
        )

        // Every connection attempt needs a fresh single-use ticket. Refresh
        // the account token once if it expired while the socket was alive.
        let ticketResponse = await requestTicket(currentAccessToken)
        if (ticketResponse.status === 401 || ticketResponse.status === 403) {
          currentAccessToken = await refreshAccessToken()
          ticketResponse = await requestTicket(currentAccessToken)
        }
        if (!ticketResponse.ok) {
          throw new Error(`RELAY_TICKET_FAILED_${ticketResponse.status}`)
        }
        const ticketData = await ticketResponse.json()
        if (typeof ticketData.ticket !== 'string' || !ticketData.ticket) {
          throw new Error('RELAY_TICKET_INVALID')
        }
        return `${relayEndpoint}?ticket=${encodeURIComponent(ticketData.ticket)}`
      }

      console.log('📡 Connecting to backend relay server...')

      // 🔐 Check if we have a saved desktop public key for E2EE
      let desktopPublicKey: string | undefined
      if (desktopId) {
        const storageKey = `desktop_public_key_${desktopId}`
        desktopPublicKey = localStorage.getItem(storageKey) || undefined
        if (desktopPublicKey) {
          console.log(`🔐 Found saved desktop public key for ${desktopId} - E2EE will be enabled`)
        } else {
          console.log(`⚠️ No saved desktop public key for ${desktopId} - E2EE handshake required`)
        }
      }

      // 🔒 REUSE PLUGIN SOCKET: Don't create new connection, update existing one
      const connection = await useConnectWs(
        relayEndpoint,
        undefined,
        undefined,
        desktopPublicKey,
        desktopId,
        requestRelayUrl,
      )

      // If this is the first call, connect
      if (!connection.socket.value) {
        console.log('🆕 First connection - establishing WebSocket')
        await connection.connectWebSocket()
      }

      // If desktop ID provided, connect to it
      if (desktopId) {
        await connection.connectToDesktop(desktopId)
      }

      return connection
    } catch (error) {
      console.error('❌ Failed to connect to relay:', error)
      throw error
    }
  }

  return {
    connectToRelay
  }
}
