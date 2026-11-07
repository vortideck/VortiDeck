import { useAuthStore } from "@base/stores/Auth";
import { useMainStore } from "@base/stores/Main";
import { useDesktopUtils } from "@desktop/stores/desktopUtils";
import { invoke } from "@tauri-apps/api/core";
import { fetch } from "@tauri-apps/plugin-http";
import { reportAuthFailure, safeAuthDiagnostic } from "@base/utils/authDiagnostics";

export const useFetchInterceptor = async (url: string, options: RequestInit) => {
    const config = useRuntimeConfig();
    const authStore = useAuthStore();
    const mainStore = useMainStore();
    const desktopUtils = useDesktopUtils();

    // Verify current token if it exists
    if (authStore.getAccessToken?.token) {
        try {
            console.debug("[Auth] Verifying access token");
            const response = await fetch(`${config.public.API_URL}/api/v1/auth/token/verify`, {
                method: "POST",
                headers: {
                    "Content-Type": "application/json"

                },
                body: JSON.stringify({ token: authStore.getAccessToken.token })
            });

            if (response.status !== 200) {
                // Token is invalid, try to refresh
                try {
                    const refreshed = await invoke("refresh_access_token");
                    if (!refreshed) {
                        // Refresh failed - but we need to check WHY before clearing tokens
                        console.warn("Token refresh returned no token");

                        // Don't clear tokens yet - they might still be valid, backend might be down
                        mainStore.addNotification({
                            title: "Connection Issue",
                            message: "Unable to verify your session. Retrying...",
                            type: "warning",
                            autoClose: true,
                        });
                        return new Response(null, { status: 401 });
                    }
                    console.info("[Auth] Access token refreshed");
                    authStore.setAccessToken(refreshed as string);
                } catch (error: unknown) {
                    reportAuthFailure("access-token refresh", error);

                    // Check if this is a network error or auth failure
                    const isNetworkError = safeAuthDiagnostic(error).networkError;

                    if (isNetworkError) {
                        // Network error - DON'T clear tokens from keyring
                        console.warn("Network error during token refresh - tokens preserved");
                        mainStore.addNotification({
                            title: "Connection Error",
                            message: "Cannot reach server. Please check your connection.",
                            type: "warning",
                            autoClose: true,
                        });
                        return new Response(null, { status: 503 }); // Service unavailable
                    }

                    // Auth failure (401/403) - NOW we clear tokens and logout
                    console.log("Authentication failed - clearing tokens and logging out");
                    await invoke("logout_user"); // This clears tokens from keyring
                    authStore.logout();
                    mainStore.addNotification({
                        title: "You have been logged out",
                        message: "Your session has expired",
                        type: "logout",
                        buttonText: "Login",
                        buttonFunction: () => {
                            desktopUtils.setUserModal(true);
                            mainStore.removeAllNotificationsOfType("logout");
                        },
                        autoClose: true,
                    });
                    return new Response(null, { status: 401 });
                }
            }
        } catch (error: unknown) {
            reportAuthFailure("access-token verification", error);

            // Network error during verification - DON'T clear tokens
            const isNetworkError = safeAuthDiagnostic(error).networkError;

            if (isNetworkError) {
                console.warn("Network error during token verification - tokens preserved");
                mainStore.addNotification({
                    title: "Connection Error",
                    message: "Cannot reach server. Please check your connection.",
                    type: "warning",
                    autoClose: true,
                });
                return new Response(null, { status: 503 });
            }

            // For other errors, try to refresh before giving up
            try {
                const refreshed = await invoke("refresh_access_token");
                if (refreshed) {
                    authStore.setAccessToken(refreshed as string);
                }
            } catch (refreshError) {
                reportAuthFailure("recovery refresh after verification", refreshError);
            }

            return new Response(null, { status: 401 });
        }
    }

    // Make the original request with the current/refreshed token
    const response = await fetch(url, {
        ...options,
        headers: {
            ...options.headers,
            Authorization: authStore.getAccessToken?.token ? `Token ${authStore.getAccessToken.token}` : '',
        },
        
    });


    return response;
}
