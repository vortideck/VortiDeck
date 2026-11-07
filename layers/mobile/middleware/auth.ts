/**
 * Authentication Middleware
 * Protects routes that require login
 *
 * IMPORTANT: Only checks if token exists in Pinia (instant, no biometric)
 * Token is loaded once on app startup by auth-restore plugin
 */
import { useAuthStore } from '../stores/Auth'

export default defineNuxtRouteMiddleware((to) => {
  // Skip auth check for login page
  if (to.path === '/login') {
    return
  }

  const authStore = useAuthStore()

  console.log('🔍 [Auth Middleware] Checking auth for route:', to.path)
  console.log('🔍 [Auth Middleware] isLoggedIn =', authStore.isLoggedIn)
  console.log('🔍 [Auth Middleware] accessToken =', authStore.getAccessToken ? 'EXISTS' : 'NULL')

  // Check if token exists in Pinia (instant, no biometric prompt)
  if (!authStore.isLoggedIn) {
    console.log('⚠️ [Auth Middleware] Not authenticated - redirecting to login')
    return navigateTo('/login')
  }

  console.log('✅ [Auth Middleware] Authenticated - allowing navigation')
})
