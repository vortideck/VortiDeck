/**
 * Mobile Authentication Store
 * Stores access token in memory (like desktop app's Pinia store)
 * Refresh token stays in secure keystore
 */

export const useAuthStore = defineStore('mobileAuthStore', {
  state: () => ({
    accessToken: null as string | null,
  }),

  getters: {
    /**
     * Get current access token from memory
     */
    getAccessToken(state): string | null {
      return state.accessToken
    },

    /**
     * Check if user is logged in (instant, no biometric prompt)
     */
    isLoggedIn(state): boolean {
      return state.accessToken !== null
    }
  },

  actions: {
    /**
     * Set access token in memory
     * Called after login or token refresh
     */
    setAccessToken(token: string) {
      this.accessToken = token
      console.log('✅ Access token stored in Pinia (memory)')
    },

    /**
     * Clear access token (logout)
     */
    clearAccessToken() {
      this.accessToken = null
      console.log('✅ Access token cleared from Pinia')
    }
  }
})
