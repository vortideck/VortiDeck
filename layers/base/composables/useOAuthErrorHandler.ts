/**
 * Unified OAuth Error Handler
 *
 * Handles standardized OAuth error responses from backend:
 * {
 *   "error": {
 *     "error_code": "oauth_refresh_failed",
 *     "error_type": "authentication",
 *     "message": "Please reconnect your Twitch account",
 *     "action_required": true,
 *     "details": "..."
 *   }
 * }
 */
import { reportAuthFailure } from "../utils/authDiagnostics";
import { useMainStore } from "../stores/Main";

export interface OAuthError {
  error_code: string;
  error_type: string;
  message: string;
  action_required: boolean;
  details?: string;
}

export type OAuthErrorCode =
  | 'oauth_refresh_failed'      // Refresh token invalid/expired - user must reconnect
  | 'token_expired_no_refresh'  // Access token expired, no refresh token available
  | 'token_not_found'           // No OAuth token in database - account not connected
  | 'validation_error'          // Missing account or invalid parameters
  | 'unsupported_platform'      // Platform not supported
  | 'send_failed'               // API call failed (not OAuth-related)
  | 'unknown_error'             // Unexpected system error
  | 'expiration_unknown';       // Token exists but no expiration data

/**
 * Checks if an account needs reconnection based on token_status
 */
export function needsReconnection(tokenStatus: {
  action_required?: boolean;
  error?: { action_required?: boolean };
} | null | undefined): boolean {
  return tokenStatus?.action_required === true ||
         tokenStatus?.error?.action_required === true;
}

/**
 * Gets user-friendly message for OAuth error
 */
export function getOAuthErrorMessage(error: OAuthError, platform?: string): string {
  const platformName = platform ? platform.charAt(0).toUpperCase() + platform.slice(1) : 'your account';

  switch (error.error_code) {
    case 'oauth_refresh_failed':
      return error.message || `Please reconnect ${platformName}`;

    case 'token_expired_no_refresh':
      return error.message || `${platformName} token expired. Please reconnect.`;

    case 'token_not_found':
      return error.message || `${platformName} not connected`;

    case 'expiration_unknown':
      return error.message || `Unable to verify ${platformName} token expiration`;

    case 'send_failed':
      return error.message || `Failed to send message to ${platformName}`;

    case 'validation_error':
      return error.message || 'Invalid request parameters';

    case 'unsupported_platform':
      return error.message || `Platform ${platform} is not supported`;

    case 'unknown_error':
    default:
      return error.message || 'An unexpected error occurred';
  }
}

/**
 * Determines if error requires user action (reconnection)
 */
export function requiresUserAction(error: OAuthError): boolean {
  return error.action_required || ['oauth_refresh_failed', 'token_expired_no_refresh', 'token_not_found'].includes(error.error_code);
}

/**
 * Gets reconnection URL for a platform
 */
export function getReconnectUrl(platform: string): string {
  const config = useRuntimeConfig();
  return `${config.public.API_URL}/api/v1/auth/${platform}-web/`;
}

/**
 * Handle OAuth error and show appropriate notification
 *
 * @example
 * // Handle error from /connected endpoint
 * account.token_status.error && handleOAuthError(account.token_status.error, account.provider)
 *
 * @example
 * // Handle error from WebSocket
 * data.errors.forEach(err => handleOAuthError(err, err.platform))
 */
export function handleOAuthError(
  error: OAuthError,
  platform?: string,
  options?: {
    onReconnect?: (platform: string) => void;
    showNotification?: boolean;
  }
): void {
  const showNotification = options?.showNotification !== false;

  // Get user-friendly message
  const message = getOAuthErrorMessage(error, platform);

  if (!showNotification) {
    reportAuthFailure(`OAuth status for ${platform || "account"}`, error, console, "warn");
    return;
  }

  const mainStore = useMainStore();

  // Determine notification type based on error severity
  if (requiresUserAction(error)) {
    // Critical errors requiring reconnection
    mainStore.addNotification({
      title: `${platform ? platform.charAt(0).toUpperCase() + platform.slice(1) : 'Account'} Reconnection Required`,
      message,
      type: 'warning',
      buttonText: 'Reconnect',
      buttonFunction: () => {
        if (options?.onReconnect && platform) {
          options.onReconnect(platform);
        }
      },
      autoClose: false, // Don't auto-close, user needs to take action
    });
  } else if (error.error_code === 'expiration_unknown') {
    // Warning only
    mainStore.addNotification({
      title: 'Token Expiration Unknown',
      message,
      type: 'warning',
      autoClose: true,
    });
  } else {
    // Temporary/other errors
    mainStore.addNotification({
      title: 'Error',
      message,
      type: 'error',
      autoClose: true,
    });
  }
}

/**
 * Process connected accounts response and handle any OAuth errors
 *
 * @example
 * const response = await fetch('/api/v1/auth/social/connected')
 * const data = await response.json()
 * processConnectedAccountsErrors(data, (platform) => connectApp(platform))
 */
export function processConnectedAccountsErrors(
  response: {
    connected_accounts: Array<{
      provider: string;
      token_status?: { error?: OAuthError };
    }>;
  },
  onReconnect?: (platform: string) => void
): void {
  if (!response?.connected_accounts) return;

  response.connected_accounts.forEach(account => {
    if (account.token_status?.error) {
      handleOAuthError(account.token_status.error, account.provider, {
        onReconnect,
        showNotification: false // Don't show notifications, UI already shows warnings
      });
    }
  });
}

/**
 * Process WebSocket chat send errors
 *
 * @example
 * socket.on('send_chat_result', (data) => {
 *   if (!data.success) {
 *     processWebSocketErrors(data.errors, (platform) => connectApp(platform))
 *   }
 * })
 */
export function processWebSocketErrors(
  errors: Array<OAuthError & { platform?: string }>,
  onReconnect?: (platform: string) => void
): void {
  errors.forEach(error => {
    handleOAuthError(error, error.platform, {
      onReconnect,
      showNotification: true // Show notifications for WebSocket errors
    });
  });
}
