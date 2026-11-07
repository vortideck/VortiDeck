import { openUrl } from '@tauri-apps/plugin-opener'
import { onOpenUrl } from '@tauri-apps/plugin-deep-link'
import { useHttpClient } from './useHttpClient'
import { useSecureTokens } from './useSecureTokens'
import { useAuthStore } from '../stores/Auth'

/**
 * Google OAuth Login for Mobile
 * Handles OAuth flow with deep link callbacks
 */
export const useOAuthLogin = () => {
  const config = useRuntimeConfig()
  const { storeTokens } = useSecureTokens()
  const authStore = useAuthStore() // Auto-imported from mobile/stores/Auth.ts

  /**
   * Start Google OAuth login flow
   * Reuses base layer composable (same as desktop)
   *
   * Flow:
   * 1. Open Google OAuth URL directly (via base layer composable)
   * 2. User authenticates
   * 3. Google redirects to backend callback
   * 4. Backend sends deep link with code
   * 5. App receives code via deep link
   * 6. App exchanges code for tokens via backend
   */
  const loginWithGoogle = async () => {
    try {
      console.log('🔐 Starting Google OAuth flow')
      // Reuse base layer composable (same as desktop)
      await useLoginWithGoogle()
      console.log('✅ OAuth browser opened successfully')
    } catch (error) {
      console.error('❌ Failed to open OAuth browser:', error)
      throw error
    }
  }

  /**
   * Start Discord OAuth login flow
   */
  const loginWithDiscord = async () => {
    try {
      const apiUrl = config.public.API_URL
      const clientId = config.public.DISCORD_CLIENT_ID

      if (!clientId) {
        console.error('❌ DISCORD_CLIENT_ID not configured!')
        throw new Error('Discord OAuth not configured. Please contact support.')
      }

      // Ensure API_URL has protocol
      const fullApiUrl = apiUrl.startsWith('http') ? apiUrl : `https://${apiUrl}`

      // Build Discord OAuth URL
      const params = new URLSearchParams({
        client_id: clientId,
        redirect_uri: `${fullApiUrl}/api/v1/auth/discord/callback`,
        response_type: 'code',
        scope: 'identify email',
      })
      const oauthUrl = `https://discord.com/oauth2/authorize?${params.toString()}`

      console.log('🔐 Starting Discord OAuth flow')
      console.log('🌐 OAuth URL:', oauthUrl)

      // Open system browser for OAuth
      await openUrl(oauthUrl)

      console.log('✅ OAuth browser opened successfully')
    } catch (error) {
      console.error('❌ Failed to open OAuth browser:', error)
      throw error
    }
  }

  /**
   * Generate random string for PKCE code_verifier
   */
  const generateCodeVerifier = (): string => {
    const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~'
    let result = ''
    const randomValues = new Uint8Array(128)
    crypto.getRandomValues(randomValues)
    randomValues.forEach((value) => {
      result += chars[value % chars.length]
    })
    return result
  }

  /**
   * Generate SHA-256 hash and base64url encode for PKCE code_challenge
   */
  const generateCodeChallenge = async (codeVerifier: string): Promise<string> => {
    const encoder = new TextEncoder()
    const data = encoder.encode(codeVerifier)
    const hash = await crypto.subtle.digest('SHA-256', data)
    const base64 = btoa(String.fromCharCode(...new Uint8Array(hash)))
    return base64
      .replace(/\+/g, '-')
      .replace(/\//g, '_')
      .replace(/=/g, '')
  }

  /**
   * Start Kick OAuth login flow (requires PKCE)
   */
  const loginWithKick = async () => {
    try {
      const apiUrl = config.public.API_URL
      const clientId = config.public.KICK_CLIENT_ID

      if (!clientId) {
        console.error('❌ KICK_CLIENT_ID not configured!')
        throw new Error('Kick OAuth not configured. Please contact support.')
      }

      // Ensure API_URL has protocol
      const fullApiUrl = apiUrl.startsWith('http') ? apiUrl : `https://${apiUrl}`

      // Generate PKCE parameters (Kick requires PKCE)
      const codeVerifier = generateCodeVerifier()
      const codeChallenge = await generateCodeChallenge(codeVerifier)

      // Store code_verifier for token exchange (backend needs this)
      // Using sessionStorage temporarily - ideally should be more secure
      if (typeof sessionStorage !== 'undefined') {
        sessionStorage.setItem('kick_code_verifier', codeVerifier)
      }

      // Generate random state for CSRF protection
      const state = Math.random().toString(36).substring(2, 15)

      // Build Kick OAuth URL with PKCE
      const params = new URLSearchParams({
        client_id: clientId,
        redirect_uri: `${fullApiUrl}/api/v1/auth/kick/callback`,
        response_type: 'code',
        scope: 'user:read channel:read',
        code_challenge: codeChallenge,
        code_challenge_method: 'S256',
        state,
      })
      const oauthUrl = `https://id.kick.com/oauth/authorize?${params.toString()}`

      console.log('🔐 Starting Kick OAuth flow with PKCE')
      console.log('🌐 OAuth URL:', oauthUrl)

      // Open system browser for OAuth
      await openUrl(oauthUrl)

      console.log('✅ OAuth browser opened successfully')
    } catch (error) {
      console.error('❌ Failed to open OAuth browser:', error)
      throw error
    }
  }

  /**
   * Set up deep link listener for OAuth callbacks
   * Call this on app initialization
   *
   * @param onSuccess - Callback when login succeeds
   * @param onError - Callback when login fails
   */
  const setupOAuthListener = (
    onSuccess?: () => void,
    onError?: (error: string) => void,
    onProgress?: (message: string) => void
  ) => {
    // Listen for deep link OAuth callbacks
    const unlisten = onOpenUrl(async (urls) => {
      console.log('📥 Deep link received:', urls)
      console.log('🔔 Firing onProgress callback:', !!onProgress)
      onProgress?.('Processing OAuth callback...')

      // Process each URL sequentially (not forEach which doesn't await)
      for (const url of urls) {
        // Check if it's an OAuth callback
        if (url.startsWith('vortideck://oauth/callback')) {
          try {
            // Parse URL parameters
            const urlObj = new URL(url)
            const code = urlObj.searchParams.get('code')
            const loginId = urlObj.searchParams.get('login_id')
            const provider = urlObj.searchParams.get('provider')
            const error = urlObj.searchParams.get('error')

            if (error) {
              console.error('❌ OAuth error:', error)
              onError?.(error)
              return
            }

            if (!code || !loginId || !provider) {
              console.error('❌ Missing required parameters in OAuth callback:', { code: !!code, loginId: !!loginId, provider })
              onError?.('Missing required parameters in callback')
              return
            }

            console.log('🔐 Exchanging OAuth code for tokens...')
            console.log('📋 Code:', code.substring(0, 20) + '...')
            console.log('🆔 Login ID:', loginId)
            console.log('🏢 Provider:', provider)

            // Determine the correct code exchange endpoint based on provider
            const exchangeEndpoint = `api/v1/auth/${provider}-code-exchange`

            // Build request body based on provider
            // Google uses { code, login_id }
            // Discord uses { code }
            // Kick uses { code, code_verifier } (PKCE requirement)
            let requestBody: any
            if (provider === 'google') {
              requestBody = { code, login_id: loginId }
            } else if (provider === 'kick') {
              // Kick requires code_verifier for PKCE
              const codeVerifier = typeof sessionStorage !== 'undefined'
                ? sessionStorage.getItem('kick_code_verifier')
                : null
              requestBody = { code, code_verifier: codeVerifier }
              console.log('📤 Request body: { code, code_verifier }')
            } else {
              // Discord
              requestBody = { code }
              console.log('📤 Request body: { code }')
            }

            // Exchange code for tokens via backend
            console.log('🚀 Making POST request to:', exchangeEndpoint)
            onProgress?.('Exchanging code for tokens...')

            const { post } = useHttpClient()

            const startTime = Date.now()
            const { data, status, error: exchangeError } = await post(
              exchangeEndpoint,
              requestBody,
              { requiresAuth: false }
            )
            const duration = Date.now() - startTime

            console.log(`📊 Code exchange response (took ${duration}ms):`, {
              status,
              hasData: !!data,
              dataKeys: data ? Object.keys(data) : [],
              error: exchangeError,
              data
            })

            if (status !== 200 || !data) {
              console.error('❌ Code exchange failed:', {
                status,
                error: exchangeError,
                data
              })
              onError?.(exchangeError || 'Failed to exchange code for tokens')
              return
            }

            // Backend returns 'access' and 'refresh' tokens
            const accessToken = data.access || data.access_token
            const refreshToken = data.refresh || data.refresh_token

            if (!accessToken || !refreshToken) {
              console.error('❌ Missing tokens in exchange response:', data)
              onError?.('Invalid response from server')
              return
            }

            // Store tokens securely (biometric prompt or localStorage fallback)
            console.log('🔐 Storing tokens securely...')
            onProgress?.('Securing tokens...')

            try {
              await storeTokens(accessToken, refreshToken)
              console.log('✅ Tokens stored securely')
              onProgress?.('Login successful! Redirecting...')
            } catch (storeError) {
              console.error('❌ Failed to store tokens:', storeError)
              onError?.('Failed to store tokens securely. Please try again.')
              return
            }

            // Also store access token in Pinia for fast access (like desktop app)
            authStore.setAccessToken(accessToken)

            // Identify user in PostHog (links anonymous device ID to real user)
            try {
              const { $posthog } = useNuxtApp()
              const oauthUser = getUserDataFromJwt(accessToken)
              if (oauthUser?.id && $posthog) {
                $posthog.identify(String(oauthUser.id), {
                  email: oauthUser.email,
                  username: oauthUser.username,
                })
              }
            } catch { /* PostHog may not be available on all platforms */ }

            // Remember which provider was used (JWT doesn't include this)
            if (typeof localStorage !== 'undefined') {
              localStorage.setItem('vortideck_login_provider', provider)
            }

            console.log('✅ OAuth login successful - tokens in keystore + Pinia')

            // Wait 1 second to show success message before navigating
            await new Promise(resolve => setTimeout(resolve, 1000))

            onSuccess?.()
          } catch (err) {
            console.error('❌ Failed to process OAuth callback:', err)
            onError?.(String(err))
          }
        }
      }
    })

    // Return cleanup function
    return unlisten
  }

  /**
   * Login with email and password
   * @param email - User email
   * @param password - User password
   */
  const loginWithEmail = async (email: string, password: string) => {
    const { post } = useHttpClient()
    const authStore = useAuthStore()

    try {
      const { data, status, error } = await post(
        'api/v1/auth/login',
        { email, password },
        { requiresAuth: false }
      )

      if (status !== 200 || !data) {
        throw new Error(error || 'Login failed')
      }

      // Server returns 'access' and 'refresh' (not 'access_token' and 'refresh_token')
      // This matches the desktop app's expectation
      const { access, refresh } = data

      if (!access || !refresh) {
        console.error('Missing tokens in response:', data)
        throw new Error('Invalid response from server')
      }

      await storeTokens(access, refresh)

      // Also store access token in Pinia for fast access (like desktop app)
      authStore.setAccessToken(access)

      // Remember login method (JWT doesn't include provider)
      if (typeof localStorage !== 'undefined') {
        localStorage.setItem('vortideck_login_provider', 'email')
      }

      console.log('✅ Email login successful - tokens in keystore + Pinia')
      return true
    } catch (error) {
      console.error('❌ Email login failed:', error)
      throw error
    }
  }

  /**
   * Register new account with email and password
   * @param email - User email
   * @param password - User password
   * @param name - User name (optional)
   */
  const registerWithEmail = async (email: string, password: string, name?: string) => {
    const { post } = useHttpClient()

    try {
      const { data, status, error } = await post(
        'api/v1/auth/register',
        { email, password, name },
        { requiresAuth: false }
      )

      if (status !== 200 || !data) {
        throw new Error(error || 'Registration failed')
      }

      // Server returns 'access' and 'refresh' (not 'access_token' and 'refresh_token')
      const { access, refresh } = data

      if (!access || !refresh) {
        console.error('❌ Missing tokens in response:', data)
        throw new Error('Invalid response from server')
      }

      // Store tokens securely in keystore (will prompt for biometric)
      await storeTokens(access, refresh)

      // Also store access token in Pinia for fast access (like desktop app)
      authStore.setAccessToken(access)

      // Remember login method (JWT doesn't include provider)
      if (typeof localStorage !== 'undefined') {
        localStorage.setItem('vortideck_login_provider', 'email')
      }

      console.log('✅ Registration successful - tokens in keystore + Pinia')
      return true
    } catch (error) {
      console.error('❌ Registration failed:', error)
      throw error
    }
  }

  /**
   * Try to load access token from keystore into Pinia if not already loaded
   * Call this once when checking auth state (e.g., on app start or page load)
   * Returns true if token is available (either already in Pinia or loaded from keystore)
   */
  const tryLoadToken = async (): Promise<boolean> => {
    // If already in memory, no need to load
    if (authStore.getAccessToken) {
      return true
    }

    try {
      const { getTokens } = useSecureTokens()
      const tokens = await getTokens()

      if (tokens && tokens.access_token) {
        authStore.setAccessToken(tokens.access_token)
        console.log('✅ Token loaded from keystore to Pinia')
        return true
      }
    } catch (error) {
      console.log('ℹ️ No stored token or user cancelled biometric')
    }

    return false
  }

  /**
   * Get user account info from JWT token
   * Decodes access token from Pinia memory (no biometric prompt)
   */
  const getUserInfo = async () => {
    try {
      // Get access token from Pinia memory (instant, no biometric)
      const accessToken = authStore.getAccessToken

      if (!accessToken) {
        throw new Error('No access token available - user not logged in')
      }

      console.log('🔍 Access token from Pinia (first 50 chars):', accessToken.substring(0, 50) + '...')

      // Decode JWT payload (second part of the token)
      const tokenParts = accessToken.split('.')
      if (tokenParts.length !== 3) {
        throw new Error('Invalid JWT token format')
      }

      // Decode base64 payload using browser-compatible atob()
      const payloadBase64 = tokenParts[1]
      const payloadJson = atob(payloadBase64)
      const payload = JSON.parse(payloadJson)

      console.log('🔍 JWT payload from Pinia token:', payload)
      console.log('🔍 JWT payload keys:', Object.keys(payload))

      // Check if we have user data in payload.user (web/desktop format)
      if (payload.user) {
        console.log('✅ Found user data in payload.user')
        return payload.user
      }

      // If no payload.user, this might be a refresh token (only has user_id)
      console.error('❌ Token in Pinia appears to be REFRESH token (only has user_id, no user object)')
      console.error('❌ This means access and refresh tokens were swapped!')
      throw new Error('Invalid token - appears to be refresh token instead of access token')
    } catch (error) {
      console.error('❌ Failed to get user info:', error)
      return null
    }
  }

  /**
   * Logout user - clears tokens from both Pinia and keystore
   */
  const logout = async (): Promise<void> => {
    const { clearTokens } = useSecureTokens()

    // Clear from Pinia (instant)
    authStore.clearAccessToken()

    // Clear from keystore (may prompt for biometric)
    await clearTokens()

    console.log('✅ Logged out - tokens cleared from Pinia + keystore')
  }

  return {
    loginWithGoogle,
    loginWithDiscord,
    loginWithKick,
    loginWithEmail,
    registerWithEmail,
    setupOAuthListener,
    tryLoadToken,
    getUserInfo,
    logout
  }
}
