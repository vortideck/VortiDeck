import { x25519 } from '@noble/curves/ed25519';
import { chacha20poly1305 } from '@noble/ciphers/chacha';
import { hmac } from '@noble/hashes/hmac';
import { sha256 } from '@noble/hashes/sha256';
import { randomBytes } from '@noble/hashes/utils';
import { v4 as uuidv4 } from 'uuid';
import canonicalize from 'canonicalize';

export const RELAY_PROTOCOL_VERSION = 2;
const RELAY_ENVELOPE_TTL_MS = 30_000;
const RELAY_CLOCK_SKEW_MS = 5 * 60_000;

/**
 * E2EE Keys for secure communication with desktop
 */
export interface E2EEKeys {
  mobilePrivate: Uint8Array;     // 32 bytes - KEEP SECRET
  mobilePublic: Uint8Array;      // 32 bytes - send to desktop
  desktopPublic?: Uint8Array;    // 32 bytes - from QR code
  sharedSecret?: Uint8Array;     // 32 bytes - derived from ECDH
}

/**
 * Generate new mobile E2EE keys for pairing
 */
export function generateE2EEKeys(): E2EEKeys {
  const mobilePrivate = x25519.utils.randomPrivateKey();
  const mobilePublic = x25519.getPublicKey(mobilePrivate);

  return {
    mobilePrivate,
    mobilePublic,
  };
}

/**
 * Derive shared secret from desktop's public key using ECDH
 * @param keys - Mobile E2EE keys
 * @param desktopPublicKey - Desktop's public key (32 bytes)
 */
export function deriveSharedSecret(
  keys: E2EEKeys,
  desktopPublicKey: Uint8Array
): E2EEKeys {
  if (desktopPublicKey.length !== 32) {
    throw new Error('Desktop public key must be 32 bytes');
  }

  const sharedSecret = x25519.getSharedSecret(keys.mobilePrivate, desktopPublicKey);

  return {
    ...keys,
    desktopPublic: desktopPublicKey,
    sharedSecret,
  };
}

/**
 * Create HMAC-SHA256 signature for a message
 * @param sharedSecret - 32-byte shared secret
 * @param message - Message to sign (as string)
 * @returns Base64-encoded signature
 */
export function createHmacSignature(sharedSecret: Uint8Array, message: string): string {
  const messageBytes = new TextEncoder().encode(message);
  const signature = hmac(sha256, sharedSecret, messageBytes);
  return btoa(String.fromCharCode(...Array.from(signature)));
}

/**
 * Verify HMAC-SHA256 signature
 * @param sharedSecret - 32-byte shared secret
 * @param message - Original message (as string)
 * @param signatureB64 - Base64-encoded signature
 * @returns true if valid, false otherwise
 */
export function verifyHmacSignature(
  sharedSecret: Uint8Array,
  message: string,
  signatureB64: string
): boolean {
  try {
    const expectedSig = createHmacSignature(sharedSecret, message);
    return expectedSig === signatureB64;
  } catch {
    return false;
  }
}

/**
 * Sign a message with HMAC and add nonce for replay attack prevention
 * @param sharedSecret - 32-byte shared secret
 * @param message - Message object to sign
 * @returns Message with signature and nonce added
 */
export function signMessage(sharedSecret: Uint8Array, message: any): any {
  const nonce = uuidv4();
  const sent_at = Date.now();
  const messageWithNonce = {
    ...message,
    protocol_version: RELAY_PROTOCOL_VERSION,
    message_id: uuidv4(),
    sent_at,
    expires_at: sent_at + RELAY_ENVELOPE_TTL_MS,
    nonce,
  };
  // ⭐ Use RFC 8785 canonical JSON for deterministic serialization
  const messageStr = canonicalize(messageWithNonce)!;
  const signature = createHmacSignature(sharedSecret, messageStr);

  return {
    ...messageWithNonce,
    signature,
  };
}

/**
 * Verify a signed message (checks HMAC signature)
 * @param sharedSecret - 32-byte shared secret
 * @param message - Signed message object (with signature and nonce)
 * @returns true if valid, false otherwise
 */
export function verifyMessage(sharedSecret: Uint8Array, message: any): boolean {
  if (!message.signature || !message.nonce) {
    return false;
  }

  const { signature, nonce, ...messageWithoutSig } = message;
  const messageWithNonce = { ...messageWithoutSig, nonce };
  // ⭐ Use RFC 8785 canonical JSON for deterministic serialization
  const messageStr = canonicalize(messageWithNonce)!;

  return verifyHmacSignature(sharedSecret, messageStr, signature);
}

const acceptedRelayNonces = new Set<string>();
const MAX_ACCEPTED_RELAY_NONCES = 4096;

/** Build the versioned envelope used only by the remote relay transport. */
export function createRelayPayloadEnvelope(
  sharedSecret: Uint8Array,
  payload: Record<string, unknown>,
) {
  const nonce = uuidv4();
  const sent_at = Date.now();
  const unsignedEnvelope = {
    protocol_version: RELAY_PROTOCOL_VERSION,
    message_id: uuidv4(),
    sent_at,
    expires_at: sent_at + RELAY_ENVELOPE_TTL_MS,
    payload,
    nonce,
  };
  const serialized = canonicalize(unsignedEnvelope);
  if (!serialized) throw new Error('Unable to canonicalize relay envelope');

  return {
    ...unsignedEnvelope,
    signature: createHmacSignature(sharedSecret, serialized),
  };
}

/** Verify a relay payload envelope and reject nonce replay. */
export function verifyRelayPayloadEnvelope(
  sharedSecret: Uint8Array,
  message: any,
  usedNonces: Set<string> = acceptedRelayNonces,
): boolean {
  if (
    message?.protocol_version !== RELAY_PROTOCOL_VERSION
    || typeof message?.message_id !== 'string'
    || typeof message?.sent_at !== 'number'
    || typeof message?.expires_at !== 'number'
    || !message?.payload
    || !message.signature
    || !message.nonce
  ) return false;
  const now = Date.now();
  if (
    message.expires_at <= message.sent_at
    || message.expires_at - message.sent_at > 60_000
    || message.sent_at > now + RELAY_CLOCK_SKEW_MS
    || message.expires_at < now - RELAY_CLOCK_SKEW_MS
  ) return false;
  if (usedNonces.has(message.nonce)) return false;

  const serialized = canonicalize({
    protocol_version: message.protocol_version,
    message_id: message.message_id,
    sent_at: message.sent_at,
    expires_at: message.expires_at,
    payload: message.payload,
    nonce: message.nonce,
  });
  if (!serialized || !verifyHmacSignature(sharedSecret, serialized, message.signature)) return false;

  if (usedNonces.size >= MAX_ACCEPTED_RELAY_NONCES) {
    const oldest = usedNonces.values().next().value;
    if (oldest) usedNonces.delete(oldest);
  }
  usedNonces.add(message.nonce);
  return true;
}

/**
 * Encrypt data using ChaCha20-Poly1305 (for future use with sensitive data)
 * @param sharedSecret - 32-byte shared secret
 * @param plaintext - Data to encrypt
 * @returns Base64-encoded ciphertext (nonce + encrypted data)
 */
export function encrypt(sharedSecret: Uint8Array, plaintext: string): string {
  const nonce = randomBytes(12); // ChaCha20-Poly1305 uses 12-byte nonce (matches desktop)
  const plaintextBytes = new TextEncoder().encode(plaintext);

  const cipher = chacha20poly1305(sharedSecret, nonce);
  const ciphertext = cipher.encrypt(plaintextBytes);

  // Prepend nonce to ciphertext
  const result = new Uint8Array(nonce.length + ciphertext.length);
  result.set(nonce);
  result.set(ciphertext, nonce.length);

  return btoa(String.fromCharCode(...Array.from(result)));
}

/**
 * Decrypt data using ChaCha20-Poly1305
 * @param sharedSecret - 32-byte shared secret
 * @param ciphertextB64 - Base64-encoded ciphertext (nonce + encrypted data)
 * @returns Decrypted plaintext
 */
export function decrypt(sharedSecret: Uint8Array, ciphertextB64: string): string {
  const ciphertextWithNonce = Uint8Array.from(atob(ciphertextB64), c => c.charCodeAt(0));

  if (ciphertextWithNonce.length < 12) {
    throw new Error('Ciphertext too short');
  }

  const nonce = ciphertextWithNonce.slice(0, 12);
  const ciphertext = ciphertextWithNonce.slice(12);

  const cipher = chacha20poly1305(sharedSecret, nonce);
  const plaintext = cipher.decrypt(ciphertext);

  return new TextDecoder().decode(plaintext);
}

/**
 * Base64 decode a public key
 */
export function decodePublicKey(base64: string): Uint8Array {
  return Uint8Array.from(atob(base64), c => c.charCodeAt(0));
}

/**
 * Base64 encode a public key
 */
export function encodePublicKey(publicKey: Uint8Array): string {
  return btoa(String.fromCharCode(...Array.from(publicKey)));
}
