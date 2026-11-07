import { useCookie, useRuntimeConfig } from 'nuxt/app'

export const useSharedFetchInterceptor = async <T = any>(
  url: string,
  options?: any
) => {
  const isAuthenticatedCookie = useCookie("isAuthenticated");
  const config = useRuntimeConfig();

  // Strip API_URL prefix so requests go through Nitro proxy (avoids CORS)
  const apiUrl = config.public.API_URL as string;
  const toProxyUrl = (fullUrl: string) =>
    (apiUrl && fullUrl.startsWith(apiUrl)) ? fullUrl.replace(apiUrl, '') : fullUrl;
  const proxyUrl = toProxyUrl(url);

  const verifyAndRefreshToken = async () => {
    const isAuthenticated = isAuthenticatedCookie.value === "true";

    if (isAuthenticated) {
      try {
        const verifyResponse = await $fetch(
          `/api/v1/auth/token/verify`,
          {
            method: "POST",
            credentials: "include",
            headers: { "Content-Type": "application/json" },
          }
        );

        if (
          verifyResponse === false ||
          (verifyResponse as any)?.valid === false
        ) {
          try {
            const refreshResponse = await $fetch(
              `/api/v1/auth/token/refresh`,
              {
                method: "POST",
                credentials: "include",
                headers: {
                  "Content-Type": "application/json",
                  "X-Client-Type": "web",
                },
              }
            );

            if (!refreshResponse) {
              isAuthenticatedCookie.value = "false";
              await $fetch("/api/auth/logout", { method: "POST" });
              return false;
            }

            isAuthenticatedCookie.value = "true";
          } catch (refreshError: any) {
            // Check if this is a network error or auth failure
            const status = refreshError.response?.status || 0;
            const isNetworkError = !refreshError.response || status >= 500;

            if (!isNetworkError) {
              // Only logout on actual auth failures (401/403/400)
              isAuthenticatedCookie.value = "false";
              await $fetch("/api/auth/logout", { method: "POST" });
            }
            return false;
          }
        }
      } catch {
        try {
          const refreshResponse = await $fetch(
            `/api/v1/auth/token/refresh`,
            {
              method: "POST",
              credentials: "include",
              headers: {
                "Content-Type": "application/json",
                "X-Client-Type": "web",
              },
            }
          );

          if (!refreshResponse) {
            isAuthenticatedCookie.value = "false";
            await $fetch("/api/auth/logout", { method: "POST" });
            return false;
          }
        } catch (refreshError: any) {
          // Check if this is a network error or auth failure
          const status = refreshError.response?.status || 0;
          const isNetworkError = !refreshError.response || status >= 500;

          if (!isNetworkError) {
            // Only logout on actual auth failures (401/403/400)
            isAuthenticatedCookie.value = "false";
            await $fetch("/api/auth/logout", { method: "POST" });
          }
          return false;
        }
      }
    }
    return true;
  };

  const execute = async () => {
    const ok = await verifyAndRefreshToken();
    if (!ok) {
      return { data: null as T | null, status: 401, error: "Auth required" };
    }

    const fetchOptions = {
      ...options,
      credentials: "include" as RequestCredentials,
    };

    try {
      const response = await $fetch<T>(proxyUrl, fetchOptions);
      return { data: response, status: 200, error: null };
    } catch (error: any) {
      if (error.response?.status === 401) {
        try {
          const refreshResponse = await $fetch(
            `/api/v1/auth/token/refresh`,
            {
              method: "POST",
              credentials: "include",
              headers: {
                "Content-Type": "application/json",
                "X-Client-Type": "web",
              },
            }
          );

          if (refreshResponse) {
            try {
              const retryResponse = await $fetch<T>(proxyUrl, fetchOptions);
              return { data: retryResponse, status: 200, error: null };
            } catch (retryError: any) {
              return {
                data: retryError.response?._data ?? null,
                status: retryError.response?.status ?? 0,
                error:
                  retryError.response?._data?.error ||
                  retryError.response?._data?.message ||
                  "Request failed",
              };
            }
          }
        } catch (refreshError: any) {
          // Check if this is a network error or auth failure
          const status = refreshError.response?.status || 0;
          const isNetworkError = !refreshError.response || status >= 500;

          if (isNetworkError) {
            // Network/server error - DON'T logout, preserve cookies
            console.warn('Network/server error during refresh - preserving session');
            return {
              data: null,
              status: 503,
              error: "Connection error. Please try again later.",
            };
          }

          // Auth failure (401/403/400) - NOW logout
          console.log('Auth failure - clearing session');
          isAuthenticatedCookie.value = "false";
          await $fetch("/api/auth/logout", { method: "POST" });
          return {
            data: null,
            status: 401,
            error: "Authentication failed, please log in again.",
          };
        }
      }

      const status = error.response?.status ?? 0;
      const data = error.response?._data ?? null;
      const message = data?.error || data?.message || "Request failed";
      return { data, status, error: message };
    }
  };

  return execute();
};
