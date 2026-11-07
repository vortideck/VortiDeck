/**
 * Connection utility helpers for WebSocket management
 */

import { URL_PATTERNS, CONNECTION_MODES, type ConnectionMode } from '../constants/connection'
import type { useConnectionStore } from '@/stores/Connection'
import type { ActiveConnectionTransport } from '@/stores/Connection'
import { usePortableWidgetAssets } from '@base/composables/usePortableWidgetAssets'

/**
 * Detect if URL is relay mode based on URL patterns
 */
export function detectConnectionMode(url: string | null | undefined): ConnectionMode {
  if (!url) return CONNECTION_MODES.DIRECT

  const isRelay = URL_PATTERNS.RELAY_INDICATORS.some(pattern => url.includes(pattern))
  return isRelay ? CONNECTION_MODES.RELAY : CONNECTION_MODES.DIRECT
}

/**
 * Update connection information in store atomically
 */
export function updateConnectionInfo(
  connectionStore: ReturnType<typeof useConnectionStore>,
  options: {
    wsUrl?: string
    connectionMode?: ConnectionMode
    activeTransport?: ActiveConnectionTransport | null
    activeDesktopId?: string | null
  }
) {
  if (options.wsUrl !== undefined) {
    connectionStore.setWsUrl(options.wsUrl)
  }

  if (options.connectionMode !== undefined) {
    connectionStore.setConnectionMode(options.connectionMode)
    if (options.connectionMode === CONNECTION_MODES.RELAY && options.activeTransport === undefined) {
      connectionStore.setActiveConnectionTransport('relay')
      connectionStore.setActiveConnectionLatencyMs(null)
    }
  }

  if (options.activeTransport !== undefined) {
    connectionStore.setActiveConnectionTransport(options.activeTransport)
  }

  if (options.activeDesktopId !== undefined) {
    connectionStore.setActiveDesktopId(options.activeDesktopId)
  }
}

/**
 * Check if HMAC signature verification is required
 */
export function shouldVerifySignature(
  sharedSecret: Uint8Array | null,
  message: { action?: string; encrypted_payload?: string; signature?: string; nonce?: string },
  relayEnvelopeVerified = false,
): boolean {
  // Relay payloads are authenticated by the outer envelope before unwrapping;
  // requiring a second signature on the inner message rejects valid delivery.
  if (relayEnvelopeVerified) return false
  if (!sharedSecret) return false
  // The first token response is already authenticated by ChaCha20-Poly1305 and
  // establishes the persisted session. Every subsequent direct message must
  // carry HMAC + nonce; verifyMessage fails closed when either is absent.
  if (message.action === 'new_token' && message.encrypted_payload) return false
  return true
}

/**
 * Reset E2EE pairing - clears all saved pairing data
 * Use this when desktop doesn't recognize the mobile device anymore
 */
export async function resetPairing(desktopId?: string) {
  try {
    const { load } = await import('@tauri-apps/plugin-store')
    const { TAURI_STORE_KEYS } = await import('../constants/connection')

    console.log('🔄 Resetting pairing data...')

    const store = await load('settings.json', { autoSave: false })

    // Clear E2EE shared secret for this desktop
    if (desktopId) {
      await store.delete(`e2ee_shared_secret_${desktopId}`)
      const { purgePortableDefinitionCache } = await import('./portableWidgetCache')
      await purgePortableDefinitionCache(desktopId)
      usePortableWidgetAssets().clear()
      console.log(`🗑️ Cleared E2EE shared secret for desktop: ${desktopId}`)
    }

    // Clear last paired desktop ID
    await store.delete(TAURI_STORE_KEYS.LAST_PAIRED_DESKTOP_ID)
    console.log('🗑️ Cleared last paired desktop ID')

    // Clear mobile client ID (will generate new one on next connection)
    await store.delete(TAURI_STORE_KEYS.MOBILE_CLIENT_ID)
    console.log('🗑️ Cleared mobile client ID')

    // Save changes
    await store.save()
    console.log('✅ Pairing data cleared successfully')

    return true
  } catch (error) {
    console.error('❌ Failed to reset pairing:', error)
    return false
  }
}
