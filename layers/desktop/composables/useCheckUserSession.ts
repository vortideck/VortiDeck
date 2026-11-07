import { useAuthStore } from "@base/stores/Auth";
import { useMainStore } from "@base/stores/Main";
import { useDesktopUtils } from "@desktop/stores/desktopUtils";
import { invoke } from "@tauri-apps/api/core";
import { reportAuthFailure, safeAuthDiagnostic } from "@base/utils/authDiagnostics";


export default defineNuxtPlugin(async () => {
    const config = useRuntimeConfig();
    const auth = useAuthStore();
    const mainStore = useMainStore();
    const desktopUtils = useDesktopUtils();

    async function refreshAccessToken(tokenExisted: boolean) {
        try {
            const refreshed = await invoke("refresh_access_token");
            if (!refreshed) {
                // Refresh returned no token - but we don't know why yet
                console.warn("Token refresh returned no token - backend may be down");

                // DON'T clear tokens yet - they might still be valid
                mainStore.addNotification({
                    title: "Connection Issue",
                    message: "Unable to verify your session. Retrying...",
                    type: "warning",
                    autoClose: true,
                });
                return false;
            }
            auth.setAccessToken(refreshed as string);
            return true;
        } catch (error: unknown) {
            reportAuthFailure("startup access-token refresh", error);

            // Check if this is a network error or auth failure
            const diagnostic = safeAuthDiagnostic(error);
            const isNetworkError = diagnostic.networkError;

            if (isNetworkError) {
                // Network error - DON'T clear tokens from keyring
                console.warn("Network error during token refresh - tokens preserved");
                mainStore.addNotification({
                    title: "Connection Error",
                    message: "Cannot reach server. Please check your connection.",
                    type: "warning",
                    autoClose: true,
                });
                return false;
            }

            // Check if error indicates tokens were already cleared by Rust backend
            const isTokenNotFound = diagnostic.tokenNotFound;

            if (isTokenNotFound) {
                // Tokens already cleared by Rust backend (due to 401/403 from refresh endpoint)
                console.log("Tokens already cleared by backend - auth failure detected");
                auth.logout();
                mainStore.addNotification({
                    title: tokenExisted ? "You have been logged out" : "You are not logged in",
                    message: tokenExisted ? "Your session has expired" : "",
                    type: "logout",
                    buttonText: "Login",
                    buttonFunction: () => {
                        desktopUtils.setUserModal(true);
                        mainStore.removeAllNotificationsOfType("logout");
                    },
                    autoClose: true,
                });
                return false;
            }

            // Other errors - unclear what happened, don't clear tokens yet
            console.warn("Unknown error during token refresh - preserving tokens");
            mainStore.addNotification({
                title: "Session Check Failed",
                message: "Unable to verify your session. Please try again.",
                type: "warning",
                autoClose: true,
            });
            return false;
        }
    }

    async function verifyToken(token: string) {
        try {
            const response = await fetch(`${config.public.API_URL}/api/v1/auth/token/verify`, {
                method: "POST",
                headers: {
                    "Content-Type": "application/json"

                },
                body: JSON.stringify({ token })
            });
            return response.status === 200;
        } catch (error) {
            reportAuthFailure("startup access-token verification", error);
            return false;
        }
    }

    async function checkAndRefreshToken() {
        if (!auth.getAccessToken?.token) {
            await refreshAccessToken(false);
            return;
        }


        const isValid = await verifyToken(auth.getAccessToken.token);
        if (!isValid) {
            await refreshAccessToken(false);
        }

    }

    // Initial check
    await checkAndRefreshToken();

    // Set up periodic checks every 5 minutes
    // const intervalId = setInterval(checkAndRefreshToken, auth.getAccessToken?.expire * 1000);

    // // Clean up interval on plugin unload
    // return () => {
    //     clearInterval(intervalId);
    // };
});
