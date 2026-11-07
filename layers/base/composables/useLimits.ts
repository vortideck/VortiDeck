import { ref, computed, onMounted } from 'vue'
import { useMainStore } from '@base/stores/Main'
import { useConnectionStore } from '@base/stores/Connection'
import { invoke } from '@tauri-apps/api/core'

interface LimitsConfig {
  ui_buttons?: number
  saved_decks?: number
  plugin_count?: number
  macro_recording?: number
  premium_themes?: number
  custom_overlay?: number
  overlay_elements?: number
  saved_overlays?: number
  [key: string]: number | undefined
}

export const usePlanLimits = (isMobile = false) => {
  // Cached limits from backend
  const limits = ref<LimitsConfig>({})
  const isLoading = ref(false)
  const isLoggedIn = ref(false)

  const mainStore = useMainStore()
  const connectionStore = isMobile ? useConnectionStore() : null

  // Fetch a specific feature limit dynamically
  const getFeatureLimit = async (featureId: string): Promise<number> => {
    // Return cached if available locally
    if (limits.value[featureId] !== undefined) {
      return limits.value[featureId]!
    }

    // Mobile: Check connection store cache FIRST (populated from server_info)
    if (isMobile && connectionStore) {
      const cachedLimits = connectionStore.getFeatureLimits
      if (cachedLimits && cachedLimits[featureId] !== undefined) {
        console.log(`✅ Using cached limit from connection store: ${featureId} = ${cachedLimits[featureId]}`)
        limits.value[featureId] = cachedLimits[featureId]
        return cachedLimits[featureId]
      }
    }

    isLoading.value = true
    try {
      if (!isMobile) {
        // Desktop: use Tauri invoke
        const response = await invoke<{ feature: string, limit: number, is_logged_in: boolean }>('get_feature_limit', {
          featureId
        })
        limits.value[featureId] = response.limit
        isLoggedIn.value = response.is_logged_in
        return response.limit
      }

      // Mobile fallback: If not in cache, return guest default
      // Limits should always be in cache from server_info
      console.warn(`⚠️ Feature limit not in cache for ${featureId}, using guest default (3)`)
      return 3
    } catch (error) {
      console.error(`Failed to fetch ${featureId} limit:`, error)
      return 3 // Guest default
    } finally {
      isLoading.value = false
    }
  }

  // Check if a feature is available based on limits
  const isFeatureAvailable = async (featureId: string, currentCount = 0): Promise<boolean> => {
    const limit = await getFeatureLimit(featureId)
    return currentCount < limit
  }

  // Clear cached limits (useful for logout/login)
  const clearCache = () => {
    limits.value = {}
    // Also clear from connection store if mobile
    if (isMobile && connectionStore) {
      connectionStore.setFeatureLimits({})
    }
  }

  return {
    limits: computed(() => limits.value),
    isLoading: computed(() => isLoading.value),
    isLoggedIn: computed(() => isLoggedIn.value),
    getFeatureLimit,
    isFeatureAvailable,
    clearCache
  }
}
