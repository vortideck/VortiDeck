import { useConnectionStore } from '@/stores/Connection'
import { useMobileRelay } from './useMobileRelay'
import { useOAuthLogin } from './useOAuthLogin'
import { watch } from 'vue'
import { navigateTo } from '#app'

/**
 * Get available relay desktops via WebSocket
 *
 * Correct Flow:
 * 1. Exchange the account JWT for a one-time WebSocket ticket
 * 2. Connect to /ws/relay/mobile/?ticket=...
 * 3. Server automatically sends available_desktops message
 * 4. Connection store receives and stores the list
 * 5. Reuse the same connection when selecting a desktop
 */

// Store the relay connection globally to reuse it
let relayConnection: any = null

export const useRelayDesktops = () => {
  const connectionStore = useConnectionStore()
  const { connectToRelay } = useMobileRelay()
  const { logout } = useOAuthLogin()

  /**
   * Connect to relay server and wait for desktop list
   * The desktop list is received via WebSocket and stored in connection store
   *
   * @returns Promise that resolves with the desktop list and connection
   */
  const getRelayDesktops = async () => {
    try {
      // Clear previous desktop list
      connectionStore.setAvailableDesktops([])

      console.log('📋 Connecting to relay to fetch desktops...')

      // Connect to relay - server will send available_desktops message
      // Store the connection for reuse
      relayConnection = await connectToRelay()

      console.log('🔌 WebSocket connected, waiting for available_desktops message...')

      // Wait for the available_desktops message with a timeout
      const desktops = await waitForDesktops(5000)

      console.log(`✅ Received ${desktops.length} desktops`)
      return { desktops, connection: relayConnection }
    } catch (error) {
      console.error('❌ Failed to get relay desktops:', error)
      relayConnection = null

      // If tokens are expired, automatically logout
      if (error instanceof Error && error.message === 'TOKENS_EXPIRED') {
        console.error('🔐 Tokens expired - logging out automatically')
        await logout()
        navigateTo('/login')
        throw new Error('Session expired. Please login again.')
      }

      throw error
    }
  }

  /**
   * Select a desktop using the existing relay connection
   * @param desktopId - The desktop ID to connect to
   */
  const selectDesktop = async (desktopId: string) => {
    if (!relayConnection) {
      throw new Error('No active relay connection')
    }

    try {
      console.log('🖥️ Selecting desktop:', desktopId)
      const desktop = connectionStore.getAvailableDesktops.find(
        (candidate: any) => candidate.desktop_id === desktopId,
      )
      if (desktop) {
        await relayConnection.connectToPreferredDesktop(desktop)
      } else {
        await relayConnection.connectToDesktop(desktopId)
      }
      console.log('✅ Desktop selected successfully')
    } catch (error) {
      console.error('❌ Failed to select desktop:', error)
      throw error
    }
  }

  /**
   * Wait for desktops to be populated in the store
   * @param timeout - Maximum time to wait in milliseconds
   */
  const waitForDesktops = (timeout: number): Promise<any[]> => {
    return new Promise((resolve) => {
      // Check if desktops already available
      const currentDesktops = connectionStore.getAvailableDesktops
      if (currentDesktops && currentDesktops.length > 0) {
        resolve(currentDesktops)
        return
      }

      let timeoutId: ReturnType<typeof setTimeout>

      // Watch for changes in available desktops
      const stopWatch = watch(
        () => connectionStore.getAvailableDesktops,
        (newDesktops) => {
          if (newDesktops && newDesktops.length > 0) {
            clearTimeout(timeoutId)
            stopWatch()
            resolve(newDesktops)
          }
        },
        { immediate: true }
      )

      // Timeout after specified time
      timeoutId = setTimeout(() => {
        stopWatch()
        // Return empty array instead of rejecting - might be legitimately no desktops
        console.log('⏱️ Timeout waiting for desktops - returning empty list')
        resolve(connectionStore.getAvailableDesktops || [])
      }, timeout)
    })
  }

  return {
    getRelayDesktops,
    selectDesktop
  }
}
