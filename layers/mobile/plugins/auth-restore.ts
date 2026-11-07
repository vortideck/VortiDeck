/**
 * Auth Restore Plugin
 *
 * Runs ONCE on app startup to load tokens into Pinia.
 * Non-blocking: the app renders immediately, token restore happens in background.
 * No biometric prompt on launch — tokens are restored silently.
 */

import { useAuthStore } from '../stores/Auth'

export default defineNuxtPlugin({
  name: 'auth-restore',
  setup() {
    const authStore = useAuthStore()
    const { getTokens } = useSecureTokens()

    // Restore tokens in background — don't block app rendering
    if (import.meta.client) {
      restoreTokens(authStore, getTokens)
    }
  }
})

async function restoreTokens(
  authStore: ReturnType<typeof useAuthStore>,
  getTokens: ReturnType<typeof useSecureTokens>['getTokens']
) {
  try {
    const tokens = await getTokens()

    if (tokens && tokens.access_token) {
      authStore.setAccessToken(tokens.access_token)
      console.log('[Auth Restore] Session restored')
    } else {
      console.log('[Auth Restore] No stored tokens')
    }
  } catch (error) {
    console.log('[Auth Restore] Could not restore session:', error)
  }
}
