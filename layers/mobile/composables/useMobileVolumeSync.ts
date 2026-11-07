import { ref, watch } from 'vue'
import { useDeckStore } from '@/stores/Deck'

export const useMobileVolumeSync = () => {
  const deckStore = useDeckStore()
  const isVolumeUpdating = ref(false)
  
  /**
   * Watch for system volume changes and update rotary components accordingly
   * Optimized for smooth animations with direct updates
   */
  const setupVolumeWatcher = (buttonId: string, updateRotaryPosition: (value: number) => void) => {
    return watch(
      () => deckStore.getSystemVolume,
      (newVolumeData) => {
        // Prevent infinite loops - don't update if we're currently dragging
        if (isVolumeUpdating.value) return
        
        // Only update if this is a system volume change (not from this button)
        if (newVolumeData.isButtonChange && newVolumeData.buttonId === buttonId) {
          return // Skip update - this button triggered the change
        }
        
        console.log(`📱 Mobile rotary ${buttonId} updating to volume: ${newVolumeData.value}%`)
        
        // Use requestAnimationFrame for smooth updates
        requestAnimationFrame(() => {
          updateRotaryPosition(newVolumeData.value)
        })
      },
      { immediate: true, flush: 'sync' } // Sync flush for immediate updates
    )
  }
  
  /**
   * Send volume change command to desktop via WebSocket
   */
  const sendVolumeChange = async (value: number, buttonId: string) => {
    isVolumeUpdating.value = true

    try {
      // Send volume command via WebSocket
      const volumeCommand = {
        action: "command",
        command: JSON.stringify({
          type: "SystemControl",
          action: `system.volume.set:${value}`
        }),
        value: value,
        buttonId: buttonId
      }

      // This will be handled by the existing WebSocket connection
      const { socket, sendSignedMessage } = await import('@/composables/useConnectWs')
      if (socket.value?.readyState === WebSocket.OPEN) {
        await sendSignedMessage(volumeCommand)
        console.log(`📱 Mobile sent volume command (E2EE signed): ${value}% from button ${buttonId}`)
      }
    } catch (error) {
      console.error('Failed to send volume command:', error)
    } finally {
      // Reset flag after a short delay to allow for system response
      setTimeout(() => {
        isVolumeUpdating.value = false
      }, 500)
    }
  }
  
  return {
    setupVolumeWatcher,
    sendVolumeChange,
    isVolumeUpdating
  }
} 