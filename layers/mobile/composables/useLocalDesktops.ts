import { useConnectWs } from '@/composables/useConnectWs'
import { useConnectionStore } from '@/stores/Connection'
import { useRouter } from 'vue-router'
import { useE2EEKeys } from '@/composables/useE2EEKeys'

export const useLocalDesktops = () => {
  const connectionStore = useConnectionStore()
  const router = useRouter()

  /**
   * Discover local desktops via mDNS
   * @returns Array of discovered desktop services
   */
  const getLocalDesktops = async () => {
    try {
      console.log('🔍 Discovering local desktops via mDNS...')
      // Get discoverService from useConnectWs instance
      const { discoverService } = await useConnectWs()
      const services = await discoverService()

      if (!services || services.length === 0) {
        console.warn('⚠️ No local desktops found')
        return []
      }

      console.log(`✅ Found ${services.length} local desktop(s)`)

      // Transform service info into desktop objects
      const desktops = services.map((service: any, index: number) => ({
        // Use real desktop_id from mDNS TXT record if available, otherwise fallback to IP-based ID
        desktop_id: service.desktop_id || `local_${service.ip}_${service.port}`,
        desktop_name: service.instance_name || service.hostname || `Desktop ${index + 1}`,
        online: service.ip !== 'Unknown', // Boolean flag for DesktopPicker component
        last_seen: new Date().toISOString(),
        // Metadata for display (hostname already trimmed in Rust)
        metadata: {
          os: service.os || 'Unknown OS',
          hostname: service.hostname || 'Unknown Host'
        },
        // Additional local-specific fields
        session_id: service.hostname || 'unknown',
        ip: service.ip,
        port: service.port,
        websocket_url: service.websocket_url,
        type: 'local',
        has_desktop_id: !!service.desktop_id, // Flag to indicate if desktop_id is available
        // Store the full service object for reference
        _service: service
      }))

      return desktops
    } catch (error) {
      console.error('❌ Failed to discover local desktops:', error)
      throw error
    }
  }

  /**
   * Connect to a local desktop with saved E2EE credentials
   * @param desktop Desktop object from getLocalDesktops()
   * NOTE: Navigation is handled by the caller (index.vue)
   */
  const connectToLocalDesktop = async (desktop: any) => {
    try {
      console.log(`🔌 Connecting to local desktop: ${desktop.desktop_name} (${desktop.ip})`)
      connectionStore.setStatus('connecting')

      // Get E2EE keys composable
      const { loadSharedSecret } = useE2EEKeys()

      // Load saved E2EE secret for this desktop if available
      let hasE2EESecret = false
      if (desktop.desktop_id && desktop.has_desktop_id) {
        console.log(`🔐 Loading saved E2EE secret for desktop ${desktop.desktop_id}`)
        hasE2EESecret = await loadSharedSecret(desktop.desktop_id)

        if (hasE2EESecret) {
          console.log('✅ E2EE secret loaded successfully')
        } else {
          console.log('⚠️ No saved E2EE secret found')
        }
      }

      // Connect to WebSocket
      // ⚠️ Don't pass desktop_id if it's a fallback ID - let desktop send real ID during handshake
      const realDesktopId = desktop.has_desktop_id ? desktop.desktop_id : undefined

      const { connectWebSocket } = await useConnectWs(
        desktop.websocket_url,  // ws://<desktop-ip>:9001/ws
        undefined,  // No pairing key - using saved secret
        undefined,  // No pairing token
        undefined,  // No desktop public key - already have shared secret
        realDesktopId  // Only pass desktop ID if it's real (not fallback)
      )

      console.log(`🔌 Establishing WebSocket connection to ${desktop.websocket_url}`)
      await connectWebSocket()
      connectionStore.setActiveConnectionTransport('lan')
      connectionStore.setActiveConnectionLatencyMs(null)
      connectionStore.setRelayFallbackAvailable(false)

      console.log('✅ Connected to local desktop successfully')
      // Navigation handled by caller

    } catch (error) {
      console.error('❌ Failed to connect to local desktop:', error)
      connectionStore.setStatus('disconnected')
      throw error
    }
  }

  return {
    getLocalDesktops,
    connectToLocalDesktop
  }
}
