import { checkStatus, BiometryType, setData, getData, hasData, removeData } from "@choochmeque/tauri-plugin-biometry-api"
import type { Status } from "@choochmeque/tauri-plugin-biometry-api"

const DOMAIN = 'com.vortideck.tokens'
const TOKEN_NAME = 'vortideck_tokens'

export const useSecureTokens = () => {

  const isBiometricAvailable = async (): Promise<boolean> => {
    try {
      const status: Status = await checkStatus()
      return status.biometryType !== BiometryType.None
    } catch (error) {
      console.error('Failed to check biometric status:', error)
      return false
    }
  }

  const getBiometricStatus = async (): Promise<Status | null> => {
    try {
      return await checkStatus()
    } catch (error) {
      console.error('Failed to get biometric status:', error)
      return null
    }
  }

  const storeTokens = async (accessToken: string, refreshToken: string): Promise<void> => {
    const tokens = JSON.stringify({
      access_token: accessToken,
      refresh_token: refreshToken,
      stored_at: Date.now()
    })

    const hasBiometrics = await isBiometricAvailable()

    if (!hasBiometrics) {
      console.warn('[Storage] No biometrics enrolled - using localStorage fallback')
      localStorage.setItem('vortideck_tokens_fallback', tokens)
      console.log('[Fallback] Tokens stored in localStorage')
      return
    }

    try {
      await setData({
        domain: DOMAIN,
        name: TOKEN_NAME,
        data: tokens
      })
      console.log('[Biometry] Tokens stored successfully with biometric protection!')
      localStorage.removeItem('vortideck_tokens_fallback')
    } catch (error) {
      console.error('[Biometry] Failed to store tokens:', error)

      const errorStr = String(error)
      if (errorStr.includes('cancel') || errorStr.includes('Cancel') || errorStr.includes('biometric') || errorStr.includes('authentication')) {
        console.warn('[Biometry] Biometric error - falling back to localStorage')
        localStorage.setItem('vortideck_tokens_fallback', tokens)
        console.log('[Fallback] Tokens stored in localStorage')
        return
      }

      throw new Error('Failed to store tokens securely')
    }
  }

  const getTokens = async (): Promise<{
    access_token: string
    refresh_token: string
    stored_at: number
  } | null> => {
    try {
      console.log('[Biometry] Attempting to retrieve tokens...')
      const exists = await hasData({ domain: DOMAIN, name: TOKEN_NAME })

      if (exists) {
        const response = await getData({
          domain: DOMAIN,
          name: TOKEN_NAME,
          reason: 'Access your VortiDeck credentials'
        })

        if (response.data) {
          const tokens = JSON.parse(response.data)
          console.log('[Biometry] Tokens retrieved successfully')
          return tokens
        }
      }

      console.log('[Biometry] No tokens found in secure storage')
    } catch (error) {
      console.error('[Biometry] Failed to retrieve tokens:', error)
      console.log('[Biometry] Falling back to localStorage...')
    }

    try {
      const fallbackTokens = localStorage.getItem('vortideck_tokens_fallback')
      if (fallbackTokens) {
        const tokens = JSON.parse(fallbackTokens)
        console.log('[Fallback] Tokens retrieved from localStorage')
        return tokens
      }
    } catch (error) {
      console.error('[Fallback] Failed to retrieve from localStorage:', error)
    }

    console.log('[Storage] No tokens found in any storage')
    return null
  }

  const getAccessToken = async (): Promise<string | null> => {
    const tokens = await getTokens()
    return tokens?.access_token || null
  }

  const getRefreshToken = async (): Promise<string | null> => {
    const tokens = await getTokens()
    return tokens?.refresh_token || null
  }

  const updateAccessToken = async (newAccessToken: string): Promise<void> => {
    try {
      const tokens = await getTokens()
      if (!tokens) {
        throw new Error('No existing tokens found')
      }
      await storeTokens(newAccessToken, tokens.refresh_token)
      console.log('Access token updated')
    } catch (error) {
      console.error('Failed to update access token:', error)
      throw error
    }
  }

  const clearTokens = async (): Promise<void> => {
    let biometryCleared = false
    let fallbackCleared = false

    try {
      console.log('[Biometry] Clearing tokens...')
      await removeData({ domain: DOMAIN, name: TOKEN_NAME })
      console.log('[Biometry] Tokens cleared')
      biometryCleared = true
    } catch (error) {
      console.error('[Biometry] Failed to clear:', error)
    }

    try {
      localStorage.removeItem('vortideck_tokens_fallback')
      console.log('[Fallback] Tokens cleared from localStorage')
      fallbackCleared = true
    } catch (error) {
      console.error('[Fallback] Failed to clear localStorage:', error)
    }

    if (!biometryCleared && !fallbackCleared) {
      throw new Error('Failed to clear tokens from any storage')
    }

    console.log('[Storage] All tokens cleared')
  }

  const hasTokens = async (): Promise<boolean> => {
    try {
      const tokens = await getTokens()
      return tokens !== null
    } catch (error) {
      return false
    }
  }

  return {
    isBiometricAvailable,
    getBiometricStatus,
    storeTokens,
    getTokens,
    getAccessToken,
    getRefreshToken,
    updateAccessToken,
    clearTokens,
    hasTokens
  }
}
