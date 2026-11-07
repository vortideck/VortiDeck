import { useConnectWs } from '../composables/useConnectWs'
import { ref, type Ref } from 'vue'

type Device = {
  client_id: string;
  device_type: string;
  device: string;
};

export default defineNuxtPlugin({
  name: 'socket',

  async setup() {
    const { socket, isWebSocketConnected, connectWebSocket, sendSignedMessage } = await useConnectWs()
    const paired: Ref<Device[] | null> = ref([])

    // Initialize WebSocket connection asynchronously (non-blocking)
    // Don't await here so the app can continue loading even if connection fails
    setTimeout(async () => {
      try {
        await connectWebSocket()
        console.log("✅ WebSocket connection initiated successfully")
      } catch (error) {
        console.warn("⚠️ WebSocket connection failed during app startup:", error)
        // App continues to work even if initial connection fails
        // Reconnection logic will handle retries
      }
    }, 100) // Small delay to let the app finish initialization first

    return {
      provide: {
        socket,
        isWebSocketConnected,
        sendSignedMessage,
        paired,
      },
    }
  }
})
