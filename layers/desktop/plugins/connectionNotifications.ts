/**
 * Listen for connection error events from Tauri backend
 * and show notifications using the existing notification system
 */

import { listen } from '@tauri-apps/api/event'
import { useMainStore } from '@/stores/Main'

interface ConnectionErrorPayload {
  type: 'authentication_failed' | 'signature_verification_failed' | 'unsigned_message'
  client_id: string
  message: string
  device_type?: string
}

export default defineNuxtPlugin(() => {
  const mainStore = useMainStore()

  // Listen for connection_error events from Rust backend
  if (process.client) {
    listen<ConnectionErrorPayload>('connection_error', (event) => {
      const error = event.payload
      console.warn('🔔 Connection error:', error)

      // Map error types to user-friendly messages
      let title = 'Connection Error'
      let message = error.message
      let color = 'red'

      switch (error.type) {
        case 'authentication_failed':
          title = 'Authentication Failed'
          message = `Mobile device authentication failed. They may need to scan a new QR code. (${error.client_id.substring(0, 16)}...)`
          break
        case 'signature_verification_failed':
          title = 'Security Check Failed'
          message = `Message signature verification failed. Mobile needs to re-pair. (${error.client_id.substring(0, 16)}...)`
          break
        case 'unsigned_message':
          title = 'Unsigned Message'
          message = `Received unsigned message from E2EE client. (${error.client_id.substring(0, 16)}...)`
          break
      }

      // Add notification to store
      mainStore.addNotification({
        id: 0, // Will be auto-assigned
        title,
        message,
        buttonText: 'Dismiss',
        buttonFunction: () => {}, // Close button already handled by component
        autoClose: true,
        duration: 10000, // 10 seconds
        type: 'error',
        color,
      })
    })

    listen<{ button_id?: number | string; error: string; is_plan_limit?: boolean }>(
      'action_execution_failed',
      (event) => {
        const payload = event.payload
        console.warn('🔔 Action execution failure:', payload)

        mainStore.addNotification({
          title: payload.is_plan_limit ? 'Plan Limit Reached' : 'Action Failed',
          message: payload.error,
          buttonText: 'Dismiss',
          buttonFunction: () => {},
          autoClose: true,
          duration: payload.is_plan_limit ? 8000 : 6000,
          type: payload.is_plan_limit ? 'warning' : 'error',
          color: payload.is_plan_limit ? 'amber' : 'red',
        })
      }
    )

    console.log('✅ Connection notifications listener started')
  }
})
