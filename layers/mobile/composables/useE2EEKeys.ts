import { ref } from 'vue';
import type { E2EEKeys } from '~/utils/crypto';
import { generateE2EEKeys, deriveSharedSecret, decodePublicKey } from '~/utils/crypto';
import { load } from '@tauri-apps/plugin-store';

/**
 * Global E2EE keys storage for the current session
 */
const e2eeKeys = ref<E2EEKeys | null>(null);

/**
 * Tauri store for persisting E2EE shared secrets
 */
let storeInstance: Awaited<ReturnType<typeof load>> | null = null;
const getStore = async () => {
  if (!storeInstance) {
    storeInstance = await load('store.json', { autoSave: false });
  }
  return storeInstance;
};

/**
 * Composable for managing E2EE keys and shared secret
 */
export function useE2EEKeys() {
  /**
   * Initialize E2EE keys for pairing
   * Call this when scanning QR code
   */
  const initializeKeys = () => {
    e2eeKeys.value = generateE2EEKeys();
    console.log('🔐 Generated mobile E2EE keys');
  };

  /**
   * Complete the ECDH key exchange with desktop's public key
   * Call this after receiving desktop_public_key in token response
   * @param desktopPublicKeyB64 - Base64-encoded desktop public key from QR/token response
   * @param desktopId - Optional desktop ID to save shared secret for reconnection
   */
  const completeKeyExchange = async (desktopPublicKeyB64: string, desktopId?: string) => {
    if (!e2eeKeys.value) {
      throw new Error('E2EE keys not initialized. Call initializeKeys() first.');
    }

    const desktopPublicKey = decodePublicKey(desktopPublicKeyB64);
    e2eeKeys.value = deriveSharedSecret(e2eeKeys.value, desktopPublicKey);

    // 🔐 CRITICAL: Save shared secret (NOT public key) for reconnection
    // This way we don't need to do handshake again - just load and use!
    if (desktopId && e2eeKeys.value.sharedSecret) {
      try {
        const store = await getStore();
        // Convert Uint8Array to base64 for storage
        const sharedSecretB64 = btoa(String.fromCharCode(...e2eeKeys.value.sharedSecret));
        await store.set(`e2ee_shared_secret_${desktopId}`, sharedSecretB64);
        await store.save();
        console.log(`💾 Saved E2EE shared secret to Tauri store for desktop ${desktopId}`);
      } catch (error) {
        console.error('❌ Failed to save shared secret:', error);
      }
    }

    console.log('✅ E2EE shared secret derived');
  };

  /**
   * Load saved shared secret for a desktop (for reconnection without handshake)
   * @param desktopId - Desktop ID to load shared secret for
   * @returns true if loaded successfully, false if not found
   */
  const loadSharedSecret = async (desktopId: string): Promise<boolean> => {
    try {
      const store = await getStore();
      const sharedSecretB64 = await store.get(`e2ee_shared_secret_${desktopId}`);
      if (!sharedSecretB64 || typeof sharedSecretB64 !== 'string') {
        console.log(`⚠️ No saved shared secret in Tauri store for desktop ${desktopId}`);
        return false;
      }

      // Convert base64 back to Uint8Array
      const sharedSecretStr = atob(sharedSecretB64);
      const sharedSecret = new Uint8Array(sharedSecretStr.length);
      for (let i = 0; i < sharedSecretStr.length; i++) {
        sharedSecret[i] = sharedSecretStr.charCodeAt(i);
      }

      // Initialize keys object with the loaded shared secret
      // 🔐 IMPORTANT: Set mobile keys to empty arrays (not dummy data)
      // This ensures getMobilePublicKey() returns null, preventing desktop key regeneration
      e2eeKeys.value = {
        mobilePrivate: new Uint8Array(0), // Empty = not generated
        mobilePublic: new Uint8Array(0),  // Empty = don't send to desktop
        sharedSecret: sharedSecret
      };

      console.log(`✅ Loaded E2EE shared secret from Tauri store for desktop ${desktopId}`);
      return true;
    } catch (error) {
      console.error('❌ Failed to load shared secret:', error);
      return false;
    }
  };

  /**
   * Get shared secret (for HMAC signing)
   * @returns Shared secret or null if not yet derived
   */
  const getSharedSecret = (): Uint8Array | null => {
    return e2eeKeys.value?.sharedSecret || null;
  };

  /**
   * Check if E2EE is active (shared secret exists)
   */
  const isE2EEActive = (): boolean => {
    return !!e2eeKeys.value?.sharedSecret;
  };

  /**
   * Get mobile public key (for sending in handshake)
   * @returns Mobile public key or null if not yet generated
   */
  const getMobilePublicKey = (): Uint8Array | null => {
    const publicKey = e2eeKeys.value?.mobilePublic;
    // Return null if empty or not generated (length 0 or undefined)
    if (!publicKey || publicKey.length === 0) {
      return null;
    }
    return publicKey;
  };

  /**
   * Clear keys (on disconnect)
   */
  const clearKeys = () => {
    e2eeKeys.value = null;
    console.log('🗑️ Cleared E2EE keys');
  };

  return {
    e2eeKeys,
    initializeKeys,
    completeKeyExchange,
    loadSharedSecret,
    getSharedSecret,
    isE2EEActive,
    getMobilePublicKey,
    clearKeys,
  };
}
