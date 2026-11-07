// middleware/auth.global.js
import { useAuthStore } from "@base/stores/Auth";
import { useDesktopUtils } from "@desktop/stores/desktopUtils";

export default defineNuxtRouteMiddleware(async (to, from) => {
    const authStore = useAuthStore();
    const desktopUtils = useDesktopUtils();

    if(!authStore.getAccessToken?.token) {
        desktopUtils.setUserModal(true);
        return navigateTo('/')
    }
        


});
