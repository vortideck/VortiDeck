import { load } from '@tauri-apps/plugin-store'
import { ref } from 'vue'
import { TAURI_STORE_KEYS } from '~/constants/connection'
import { usePortableWidgetAssets } from '@base/composables/usePortableWidgetAssets'

export interface PairedDesktop {
  desktop_id: string
  desktop_name: string
  last_connected: string
  has_token: boolean
  has_e2ee_secret: boolean
  connection_mode: 'relay' | 'direct'
  ws_url?: string  // For direct connections
}

/**
 * Global paired desktops storage
 */
const pairedDesktops = ref<PairedDesktop[]>([])

/**
 * In-memory hash map for O(1) lookups by desktop_id
 */
const pairedDesktopsMap = ref<Map<string, PairedDesktop>>(new Map())

/**
 * Tauri store instance
 */
let storeInstance: Awaited<ReturnType<typeof load>> | null = null
const getStore = async () => {
  if (!storeInstance) {
    storeInstance = await load('store.json', { autoSave: false })
  }
  return storeInstance
}

/**
 * Composable for managing paired desktops registry with O(1) hash map lookups
 */
export function usePairedDesktops() {
  /**
   * Load all paired desktops from Tauri store
   */
  const loadPairedDesktops = async (): Promise<PairedDesktop[]> => {
    try {
      const store = await getStore()
      const desktops = await store.get(TAURI_STORE_KEYS.PAIRED_DESKTOPS)

      if (Array.isArray(desktops)) {
        pairedDesktops.value = desktops

        // Build in-memory hash map for O(1) lookups
        pairedDesktopsMap.value.clear()
        desktops.forEach(desktop => {
          pairedDesktopsMap.value.set(desktop.desktop_id, desktop)
        })

        console.log(`📱 Loaded ${desktops.length} paired desktops`)
        return desktops
      }

      console.log('📱 No paired desktops found')
      return []
    } catch (error) {
      console.error('❌ Failed to load paired desktops:', error)
      return []
    }
  }

  /**
   * Get desktop info by ID - O(1) hash map lookup
   */
  const getDesktopById = (desktopId: string): PairedDesktop | undefined => {
    return pairedDesktopsMap.value.get(desktopId)
  }

  /**
   * Add or update a paired desktop
   * Updates both array and hash map for consistency
   */
  const updatePairedDesktop = async (
    desktopId: string,
    updates: Partial<PairedDesktop>
  ): Promise<void> => {
    try {
      const store = await getStore()

      // Load existing desktops
      let desktops = await store.get(TAURI_STORE_KEYS.PAIRED_DESKTOPS)
      if (!Array.isArray(desktops)) {
        desktops = []
      }

      // Find existing or create new
      const existingIndex = desktops.findIndex(d => d.desktop_id === desktopId)
      let updatedDesktop: PairedDesktop

      if (existingIndex >= 0) {
        // Update existing
        updatedDesktop = {
          ...desktops[existingIndex],
          ...updates,
          desktop_id: desktopId  // Ensure ID doesn't change
        }
        desktops[existingIndex] = updatedDesktop
        console.log(`📝 Updated paired desktop: ${desktopId}`)
      } else {
        // Add new
        updatedDesktop = {
          desktop_id: desktopId,
          desktop_name: updates.desktop_name || `Desktop ${desktopId.substring(0, 8)}`,
          last_connected: new Date().toISOString(),
          has_token: false,
          has_e2ee_secret: false,
          connection_mode: 'relay',
          ...updates
        }
        desktops.push(updatedDesktop)
        console.log(`➕ Added new paired desktop: ${desktopId}`)
      }

      // Save to store
      await store.set(TAURI_STORE_KEYS.PAIRED_DESKTOPS, desktops)
      await store.save()

      // Update reactive refs
      pairedDesktops.value = desktops
      pairedDesktopsMap.value.set(desktopId, updatedDesktop)
    } catch (error) {
      console.error('❌ Failed to update paired desktop:', error)
    }
  }

  /**
   * Remove a paired desktop and clean up its data
   */
  const removePairedDesktop = async (desktopId: string): Promise<void> => {
    try {
      const store = await getStore()

      // Remove from list
      let desktops = await store.get(TAURI_STORE_KEYS.PAIRED_DESKTOPS)
      if (Array.isArray(desktops)) {
        desktops = desktops.filter(d => d.desktop_id !== desktopId)
        await store.set(TAURI_STORE_KEYS.PAIRED_DESKTOPS, desktops)
      }

      // Clean up associated data
      await store.delete(TAURI_STORE_KEYS.TOKEN_FOR_DESKTOP(desktopId))
      await store.delete(TAURI_STORE_KEYS.E2EE_SHARED_SECRET(desktopId))

      await store.save()
      const { purgePortableDefinitionCache } = await import('../utils/portableWidgetCache')
      await purgePortableDefinitionCache(desktopId)
      usePortableWidgetAssets().clear()

      // Update reactive refs
      pairedDesktops.value = desktops
      pairedDesktopsMap.value.delete(desktopId)

      console.log(`🗑️ Removed paired desktop and cleaned up data: ${desktopId}`)
    } catch (error) {
      console.error('❌ Failed to remove paired desktop:', error)
    }
  }

  /**
   * Check if desktop has valid credentials (token + E2EE secret)
   */
  const hasValidCredentials = async (desktopId: string): Promise<boolean> => {
    try {
      const store = await getStore()

      // Check for token
      const token = await store.get(TAURI_STORE_KEYS.TOKEN_FOR_DESKTOP(desktopId))
      const hasToken = !!token

      // Check for E2EE secret
      const secret = await store.get(TAURI_STORE_KEYS.E2EE_SHARED_SECRET(desktopId))
      const hasSecret = !!secret

      console.log(`🔐 Credentials check for ${desktopId}:`, { hasToken, hasSecret })
      return hasToken && hasSecret
    } catch (error) {
      console.error('❌ Failed to check credentials:', error)
      return false
    }
  }

  /**
   * Get count of paired desktops
   */
  const getPairedDesktopCount = (): number => {
    return pairedDesktops.value.length
  }

  /**
   * Check if a desktop is already paired
   */
  const isDesktopPaired = (desktopId: string): boolean => {
    return pairedDesktopsMap.value.has(desktopId)
  }

  return {
    // Reactive state
    pairedDesktops,           // Array for UI (v-for)
    pairedDesktopsMap,        // Map for O(1) lookups

    // Methods
    loadPairedDesktops,       // Load from store
    getDesktopById,           // O(1) lookup by ID
    updatePairedDesktop,      // Add/update desktop
    removePairedDesktop,      // Remove desktop
    hasValidCredentials,      // Check credentials
    getPairedDesktopCount,    // Get count
    isDesktopPaired,          // Check if paired
  }
}
