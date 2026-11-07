/**
 * Auth Error Classifier
 *
 * Distinguishes between transient failures (network, backend down)
 * and permanent auth failures (invalid/expired tokens).
 *
 * Only clear tokens/logout on PERMANENT auth failures.
 */

export enum ErrorType {
  /** Network error - backend unreachable, DNS failure, timeout */
  NETWORK_ERROR = 'NETWORK_ERROR',

  /** Server error - 5xx, backend is having issues but tokens are still valid */
  SERVER_ERROR = 'SERVER_ERROR',

  /** Permanent auth failure - token is invalid/expired/revoked, should logout */
  AUTH_FAILURE = 'AUTH_FAILURE',

  /** Other errors - 4xx (except 401/403), validation errors, etc */
  CLIENT_ERROR = 'CLIENT_ERROR',
}

export interface ClassifiedError {
  type: ErrorType
  shouldLogout: boolean
  shouldClearTokens: boolean
  userMessage: string
  originalError: any
}

/**
 * Classifies an error to determine if tokens should be cleared
 */
export function classifyAuthError(error: any, context: 'verify' | 'refresh' | 'request'): ClassifiedError {
  // Network errors (no response received)
  if (!error.response && (error.message?.includes('fetch') || error.code === 'ECONNREFUSED' || error.code === 'ENOTFOUND')) {
    return {
      type: ErrorType.NETWORK_ERROR,
      shouldLogout: false,
      shouldClearTokens: false,
      userMessage: 'Connection error. Please check your internet connection and try again.',
      originalError: error,
    }
  }

  const status = error.response?.status || error.status || 0

  // Server errors (5xx) - backend is down but tokens are valid
  if (status >= 500) {
    return {
      type: ErrorType.SERVER_ERROR,
      shouldLogout: false,
      shouldClearTokens: false,
      userMessage: 'Server is temporarily unavailable. Please try again later.',
      originalError: error,
    }
  }

  // Auth failures - only clear tokens on specific cases
  if (status === 401 || status === 403) {
    // For refresh endpoint specifically: 401/403 means refresh token is invalid
    if (context === 'refresh') {
      return {
        type: ErrorType.AUTH_FAILURE,
        shouldLogout: true,
        shouldClearTokens: true,
        userMessage: 'Your session has expired. Please log in again.',
        originalError: error,
      }
    }

    // For verify endpoint: 401 just means we should try refresh
    if (context === 'verify') {
      return {
        type: ErrorType.CLIENT_ERROR,
        shouldLogout: false,
        shouldClearTokens: false,
        userMessage: 'Token verification failed. Attempting to refresh...',
        originalError: error,
      }
    }

    // For regular requests: 401 triggers refresh attempt
    return {
      type: ErrorType.CLIENT_ERROR,
      shouldLogout: false,
      shouldClearTokens: false,
      userMessage: 'Authentication required. Attempting to refresh session...',
      originalError: error,
    }
  }

  // 400 from refresh endpoint - malformed token, should clear
  if (status === 400 && context === 'refresh') {
    return {
      type: ErrorType.AUTH_FAILURE,
      shouldLogout: true,
      shouldClearTokens: true,
      userMessage: 'Invalid authentication token. Please log in again.',
      originalError: error,
    }
  }

  // Other client errors (validation, not found, etc)
  return {
    type: ErrorType.CLIENT_ERROR,
    shouldLogout: false,
    shouldClearTokens: false,
    userMessage: error.response?._data?.error || error.response?._data?.message || 'Request failed',
    originalError: error,
  }
}
