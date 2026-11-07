import { fetch as tauriFetch, Response } from '@tauri-apps/plugin-http'
import { useSecureTokens } from './useSecureTokens'
import { useAuthStore } from '../stores/Auth'

export interface HttpOptions {
  method?: 'GET' | 'POST' | 'PUT' | 'DELETE' | 'PATCH'
  headers?: Record<string, string>
  body?: any
  requiresAuth?: boolean
}

/**
 * Mobile HTTP client with automatic token refresh
 * Uses Tauri HTTP plugin to avoid CORS issues
 * Gets access token from Pinia (instant), refresh token from keystore (biometric)
 */
export const useHttpClient = () => {
  const config = useRuntimeConfig()
  const authStore = useAuthStore()
  const { getRefreshToken, storeTokens } = useSecureTokens()

  /**
   * Make HTTP request with automatic token refresh
   * @param url - Full URL or path (will prepend API_URL if path starts with /)
   * @param options - Request options
   */
  const request = async <T = any>(
    url: string,
    options: HttpOptions = {}
  ): Promise<{ data: T | null; status: number; error: string | null }> => {
    const {
      method = 'GET',
      headers = {},
      body,
      requiresAuth = true
    } = options

    // Build full URL
    const fullUrl = url.startsWith('http')
      ? url
      : `${config.public.API_URL}/${url}`

    // Prepare headers (skip Content-Type for FormData — browser/Tauri sets boundary automatically)
    const isFormData = body instanceof FormData
    const requestHeaders: Record<string, string> = {
      ...(!isFormData && { 'Content-Type': 'application/json' }),
      'x-client-Type': 'native', // Match desktop app - server uses this to identify native clients
      ...headers
    }

    // Add authorization header if required
    if (requiresAuth) {
      // Get access token from Pinia (instant, no biometric prompt)
      const accessToken = authStore.getAccessToken
      if (!accessToken) {
        return {
          data: null,
          status: 401,
          error: 'Not authenticated'
        }
      }
      requestHeaders['Authorization'] = `Token ${accessToken}`
    }

    try {
      // Make request
      const response = await tauriFetch(fullUrl, {
        method,
        headers: requestHeaders,
        body: body ? (isFormData ? body : JSON.stringify(body)) : undefined
      })

      // Parse response
      const responseText = await response.text()
      let responseData: any = null

      try {
        responseData = responseText ? JSON.parse(responseText) : null
      } catch {
        responseData = responseText
      }

      // Handle 401 - token expired, try to refresh
      if (response.status === 401 && requiresAuth) {
        console.log('🔄 Token expired, attempting refresh...')

        try {
          const refreshToken = await getRefreshToken()
          if (!refreshToken) {
            throw new Error('No refresh token available')
          }

          // Request new access token
          const refreshResponse = await tauriFetch(
            `${config.public.API_URL}/api/v1/auth/token/refresh`,
            {
              method: 'POST',
              headers: {
                'Content-Type': 'application/json',
                'x-client-Type': 'native'
              },
              body: JSON.stringify({
                refresh: refreshToken
              })
            }
          )

          if (!refreshResponse.ok) {
            throw new Error('Token refresh failed')
          }

          const refreshData = await refreshResponse.json()
          // Backend returns access_token and refresh_token (with underscores)
          const newAccessToken = refreshData.access_token || refreshData.access
          const newRefreshToken = refreshData.refresh_token || refreshData.refresh

          if (!newAccessToken) {
            throw new Error('No access token in refresh response')
          }

          // Store new tokens in keystore
          if (newRefreshToken) {
            await storeTokens(newAccessToken, newRefreshToken)
          }

          // Update access token in Pinia (instant access for future requests)
          authStore.setAccessToken(newAccessToken)

          console.log('✅ Token refreshed - updated keystore + Pinia')

          // Retry original request with new token
          const retryResponse = await tauriFetch(fullUrl, {
            method,
            headers: {
              ...requestHeaders,
              'Authorization': `Token ${newAccessToken}`
            },
            body: body ? (isFormData ? body : JSON.stringify(body)) : undefined
          })

          const retryText = await retryResponse.text()
          let retryData: any = null

          try {
            retryData = retryText ? JSON.parse(retryText) : null
          } catch {
            retryData = retryText
          }

          return {
            data: retryData,
            status: retryResponse.status,
            error: retryResponse.ok ? null : retryData?.message || 'Request failed'
          }
        } catch (refreshError) {
          console.error('❌ Token refresh failed:', refreshError)
          return {
            data: null,
            status: 401,
            error: 'Authentication failed. Please log in again.'
          }
        }
      }

      // Return response
      return {
        data: responseData,
        status: response.status,
        error: response.ok ? null : responseData?.message || 'Request failed'
      }
    } catch (error: any) {
      console.error('❌ HTTP request failed:', error)
      return {
        data: null,
        status: 0,
        error: error.message || 'Network error'
      }
    }
  }

  /**
   * GET request
   */
  const get = <T = any>(url: string, options?: Omit<HttpOptions, 'method' | 'body'>) =>
    request<T>(url, { ...options, method: 'GET' })

  /**
   * POST request
   */
  const post = <T = any>(url: string, body?: any, options?: Omit<HttpOptions, 'method' | 'body'>) =>
    request<T>(url, { ...options, method: 'POST', body })

  /**
   * PUT request
   */
  const put = <T = any>(url: string, body?: any, options?: Omit<HttpOptions, 'method' | 'body'>) =>
    request<T>(url, { ...options, method: 'PUT', body })

  /**
   * DELETE request
   */
  const del = <T = any>(url: string, options?: Omit<HttpOptions, 'method' | 'body'>) =>
    request<T>(url, { ...options, method: 'DELETE' })

  /**
   * PATCH request
   */
  const patch = <T = any>(url: string, body?: any, options?: Omit<HttpOptions, 'method' | 'body'>) =>
    request<T>(url, { ...options, method: 'PATCH', body })

  return {
    request,
    get,
    post,
    put,
    delete: del,
    patch
  }
}
