/**
 * OAuth Deep Link Listener Plugin
 * Sets up global listener for OAuth callback deep links
 */
import { useOAuthLogin } from '~/composables/useOAuthLogin'

// Global state for OAuth progress (accessible from login page)
const oauthProgress = ref<string>('')
const oauthError = ref<string>('')

export default defineNuxtPlugin(() => {
  const { setupOAuthListener } = useOAuthLogin()

  // Set up OAuth deep link listener
  const unlisten = setupOAuthListener(
    // Success callback
    () => {
      console.log('✅ OAuth login successful - navigating to deck')
      oauthProgress.value = ''
      oauthError.value = ''
      navigateTo('/deck')
    },
    // Error callback
    (error) => {
      console.error('❌ OAuth login failed:', error)
      oauthProgress.value = ''
      oauthError.value = error
      // Stay on login page to show error
    },
    // Progress callback
    (message) => {
      console.log('🔄 OAuth progress:', message)
      oauthProgress.value = message
      oauthError.value = ''
    }
  )

  // Provide state to the app
  return {
    provide: {
      oauthProgress,
      oauthError
    }
  }

  // Cleanup on app unmount (if needed)
  // Note: This may not be called on mobile apps, but good practice
  if (import.meta.client) {
    window.addEventListener('beforeunload', () => {
      unlisten?.then(fn => fn())
    })
  }
})
