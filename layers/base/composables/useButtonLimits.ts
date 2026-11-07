import { ref, computed, onMounted } from 'vue'
import { usePlanLimits } from './useLimits'
import { useMainStore } from '@base/stores/Main'
import { vibrate } from '@tauri-apps/plugin-haptics'

export const useButtonLimits = (isMobile = false) => {
  const planLimits = usePlanLimits(isMobile)
  const mainStore = useMainStore()
  
  // Button-specific state with sensible guest defaults so overlays show immediately
  const effectiveButtonLimit = ref<number>(3)
  const isLoading = ref(false)

  // Get button limit specifically
  const fetchButtonLimit = async () => {
    isLoading.value = true
    try {
      const limit = await planLimits.getFeatureLimit('ui_buttons')
      effectiveButtonLimit.value = limit || 3
    } catch (error) {
      console.error('Failed to fetch button limit:', error)
      effectiveButtonLimit.value = 3 // Fallback to guest limit
    } finally {
      isLoading.value = false
    }
  }

  // Handle locked button click
  const handleLockedButtonClick = async (position: number) => {
    console.log('🔒 Locked button clicked, position:', position, 'limit:', effectiveButtonLimit.value)
    
    if (isMobile) {
      // Mobile-specific behavior: vibration
      try {
        await vibrate(100)
      } catch (error) {
        console.warn('Vibration failed:', error)
      }
    } else {
      const limit = await planLimits.getFeatureLimit('ui_buttons')
      // Desktop-specific behavior: show notification
      const message = planLimits.isLoggedIn.value 
        ? `Button ${position} exceeds your plan limit. Upgrade to VortiDeck Pro for more buttons! You are currently limited to ${limit} buttons per deck`
        : `Button ${position} is locked. Sign in or upgrade to VortiDeck Pro to unlock more buttons! You are currently limited to ${limit} buttons per deck`
      
      mainStore.addNotification({
        title: 'Upgrade Required',
        message,
        type: 'warning',
        duration: 10000,
        autoClose: true

      })
    }

    // Trigger flash animation by adding/removing class
    setTimeout(() => {
      const overlay = document.querySelector(`.lock-overlay-${position}`)
      if (overlay) {
        overlay.classList.add('flash-animation')
        setTimeout(() => {
          overlay.classList.remove('flash-animation')
        }, 400)
      }
    }, 0)
  }

  // Check if a button at given index is locked
  const isButtonLocked = (index: number) => {
    return index >= effectiveButtonLimit.value
  }

  // Initialize on mount
  onMounted(() => {
    fetchButtonLimit()
  })

  return {
    effectiveButtonLimit: computed(() => effectiveButtonLimit.value),
    isLoading: computed(() => isLoading.value),
    isLoggedIn: planLimits.isLoggedIn,
    handleLockedButtonClick,
    isButtonLocked,
    refreshButtonLimit: fetchButtonLimit
  }
}
