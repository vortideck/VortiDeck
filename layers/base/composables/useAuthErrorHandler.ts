/**
 * Unified Auth Error Handler
 *
 * Centralizes error detection, token clearing, and user notifications
 * for both web and desktop (Tauri) environments.
 */

import { useAuthStore } from "@base/stores/Auth";
import { useMainStore } from "@base/stores/Main";

export type AuthErrorType = 'network' | 'auth_failure' | 'token_not_found' | 'unknown';

export interface AuthErrorResult {
  type: AuthErrorType;
  shouldClearTokens: boolean;
  shouldLogout: boolean;
  userMessage: string;
  notificationType: 'warning' | 'error' | 'logout';
}

/**
 * Detects if error is a network/connection error (web-based errors)
 */
export function isWebNetworkError(error: any): boolean {
  const status = error?.response?.status || 0;
  return !error?.response || status >= 500;
}

/**
 * Detects if error is a network/connection error (Tauri-based errors)
 */
export function isTauriNetworkError(error: any): boolean {
  const errorMessage = error?.toString() || '';
  return errorMessage.includes('fetch') ||
         errorMessage.includes('network') ||
         errorMessage.includes('Failed to send request') ||
         errorMessage.includes('ECONNREFUSED') ||
         errorMessage.includes('ENOTFOUND');
}

/**
 * Detects if tokens were already cleared by backend
 */
export function isTokenNotFoundError(error: any): boolean {
  const errorMessage = error?.toString() || '';
  return errorMessage.includes('Token not found');
}

/**
 * Classifies an auth error and determines appropriate action
 */
export function classifyAuthErrorByEnv(error: any, environment: 'web' | 'tauri' = 'web'): AuthErrorResult {
  // Network errors - preserve tokens
  const isNetworkError = environment === 'web'
    ? isWebNetworkError(error)
    : isTauriNetworkError(error);

  if (isNetworkError) {
    return {
      type: 'network',
      shouldClearTokens: false,
      shouldLogout: false,
      userMessage: 'Cannot reach server. Please check your connection.',
      notificationType: 'warning',
    };
  }

  // Token not found - already cleared by backend
  if (isTokenNotFoundError(error)) {
    return {
      type: 'token_not_found',
      shouldClearTokens: false, // Already cleared by Rust backend
      shouldLogout: true, // Just clear frontend store
      userMessage: 'Your session has expired',
      notificationType: 'logout',
    };
  }

  // Auth failures (401/403/400) - clear tokens
  const status = error?.response?.status || 0;
  if (status === 401 || status === 403 || status === 400) {
    return {
      type: 'auth_failure',
      shouldClearTokens: true,
      shouldLogout: true,
      userMessage: 'Your session has expired',
      notificationType: 'logout',
    };
  }

  // Unknown errors - preserve tokens to be safe
  return {
    type: 'unknown',
    shouldClearTokens: false,
    shouldLogout: false,
    userMessage: 'Unable to verify your session. Please try again.',
    notificationType: 'warning',
  };
}

/**
 * Handles logout for web environment (cookies)
 */
export async function handleWebLogout() {
  const isAuthenticatedCookie = useCookie('isAuthenticated');
  isAuthenticatedCookie.value = 'false';
  await $fetch('/api/auth/logout', { method: 'POST' }).catch(() => {
    // Ignore logout errors, user is being logged out anyway
  });
}

/**
 * Handles logout for Tauri environment (keyring + store)
 */
export async function handleTauriLogout() {
  const { invoke } = await import('@tauri-apps/api/core');
  const auth = useAuthStore();

  await invoke('logout_user').catch(() => {
    // Ignore errors, clearing store anyway
  });
  auth.logout();
}

/**
 * Shows appropriate notification based on error classification
 */
export function showAuthErrorNotification(
  result: AuthErrorResult,
  options?: {
    tokenExisted?: boolean;
    onLoginClick?: () => void;
  }
) {
  const mainStore = useMainStore();

  if (result.type === 'network') {
    mainStore.addNotification({
      title: 'Connection Error',
      message: result.userMessage,
      type: 'warning',
      autoClose: true,
    });
    return;
  }

  if (result.shouldLogout) {
    const title = options?.tokenExisted
      ? 'You have been logged out'
      : 'You are not logged in';

    mainStore.addNotification({
      title,
      message: result.userMessage,
      type: 'logout',
      buttonText: 'Login',
      buttonFunction: options?.onLoginClick || (() => {}),
      autoClose: true,
    });
    return;
  }

  // Unknown/other errors
  mainStore.addNotification({
    title: 'Session Check Failed',
    message: result.userMessage,
    type: result.notificationType,
    autoClose: true,
  });
}

/**
 * Complete auth error handler - one function to rule them all
 *
 * @example
 * // Tauri environment
 * const handled = await handleAuthError(error, 'tauri', {
 *   tokenExisted: true,
 *   onLoginClick: () => desktopUtils.setUserModal(true)
 * });
 * if (handled.shouldStopExecution) return false;
 *
 * @example
 * // Web environment
 * const handled = await handleAuthError(error, 'web');
 * if (handled.shouldStopExecution) return { data: null, status: 503 };
 */
export async function handleAuthError(
  error: any,
  environment: 'web' | 'tauri' = 'web',
  options?: {
    tokenExisted?: boolean;
    onLoginClick?: () => void;
  }
): Promise<AuthErrorResult & { shouldStopExecution: boolean }> {
  const result = classifyAuthErrorByEnv(error, environment);

  // Show notification
  showAuthErrorNotification(result, options);

  // Handle logout if needed
  if (result.shouldClearTokens) {
    if (environment === 'web') {
      await handleWebLogout();
    } else {
      await handleTauriLogout();
    }
  } else if (result.shouldLogout) {
    // Token not found - just clear frontend store
    if (environment === 'tauri') {
      const auth = useAuthStore();
      auth.logout();
    } else {
      await handleWebLogout();
    }
  }

  return {
    ...result,
    shouldStopExecution: true, // Always stop execution after handling error
  };
}
